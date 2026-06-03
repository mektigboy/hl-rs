// ============================================================================
// Client (Prepare, Sign, Send)
// ============================================================================

use alloy::{primitives::Address, signers::local::PrivateKeySigner};
use alloy_signer::{Signature, SignerSync};
use reqwest::Client;
use serde::Serialize;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::{
    actions::{
        agent_signing_hash, build_action_value, compute_l1_hash, multisig_outer_signing_hash_with_action,
        signature_chain_id_hex, Action, MultiSigAction, MultiSigPayload, SignedAction,
        SignedMultiSigAction,
    },
    clients::exchange::responses::{ExchangeResponse, ExchangeResponseStatusRaw},
    http::HttpClient,
    BaseUrl, Error, PreparedAction,
};

/// Client for preparing, signing, and sending exchange actions.
///
/// # Example
/// ```no_run
/// use std::str::FromStr;
///
/// use alloy::primitives::Address;
/// use alloy::signers::local::PrivateKeySigner;
/// use hl_rs::{BaseUrl, ExchangeClientV2, UsdSend};
/// use rust_decimal_macros::dec;
///
/// # async fn run() -> Result<(), hl_rs::Error> {
/// let wallet =
///     PrivateKeySigner::from_str("0x0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef")
///         .expect("valid private key hex");
/// let client = ExchangeClientV2::new(BaseUrl::Testnet).with_signer(wallet);
///
/// let action = UsdSend::new(Address::ZERO, dec!(1.0));
///
/// let _response = client.send_action(action).await?;
/// # Ok(())
/// # }
/// ```
///
/// # Example (perpDeploy setSubDeployers)
/// ```no_run
/// use std::str::FromStr;
///
/// use alloy::primitives::Address;
/// use alloy::signers::local::PrivateKeySigner;
/// use hl_rs::{BaseUrl, ExchangeClientV2, SetSubDeployers, SubDeployer, SubDeployerVariant};
///
/// # async fn run() -> Result<(), hl_rs::Error> {
/// let wallet =
///     PrivateKeySigner::from_str("0x0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef")
///         .expect("valid private key hex");
/// let client = ExchangeClientV2::new(BaseUrl::Testnet).with_signer(wallet);
///
/// let sub_deployer = SubDeployer::enable(Address::ZERO, SubDeployerVariant::SetOracle);
/// let action = SetSubDeployers::new("km", vec![sub_deployer]);
/// let _response = client.send_action(action).await?;
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone)]
pub struct ExchangeClient {
    base_url: BaseUrl,
    http_client: HttpClient,
    vault_address: Option<Address>,
    expires_after: Option<u64>,
    signer_private_key: Option<PrivateKeySigner>,
    nonce_counter: Arc<AtomicU64>,
}

impl ExchangeClient {
    pub fn current_timestamp_ms() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64
    }

    pub fn new(base_url: BaseUrl) -> Self {
        let http_client = HttpClient {
            client: Client::default(),
            base_url: base_url.get_url(),
        };

        Self {
            base_url,
            http_client,
            vault_address: None,
            expires_after: None,
            signer_private_key: None,
            nonce_counter: Arc::new(AtomicU64::new(Self::current_timestamp_ms())),
        }
    }

    pub fn with_vault_address(mut self, vault_address: Address) -> Self {
        self.vault_address = Some(vault_address);
        self
    }

    pub fn with_expires_after(mut self, expires_after: u64) -> Self {
        self.expires_after = Some(expires_after);
        self
    }
    pub fn with_signer(mut self, signer_private_key: PrivateKeySigner) -> Self {
        self.signer_private_key = Some(signer_private_key);
        self
    }

    pub fn prepare_action<A: Action>(&self, action: A) -> Result<PreparedAction<A>, Error> {
        PreparedAction::new(
            action,
            self.base_url.get_signing_chain(),
            self.vault_address,
            self.expires_after,
        )
    }

    pub fn sign_action<A: Action>(
        &self,
        action: A,
        wallet: &PrivateKeySigner,
    ) -> Result<SignedAction<A>, Error> {
        self.prepare_action(self.ensure_action_nonce(action))?.sign(wallet)
    }

    pub async fn send_signed_action<A: Action + Serialize>(
        &self,
        signed_action: SignedAction<A>,
    ) -> Result<ExchangeResponse, Error> {
        let output = self.http_client.post("/exchange", signed_action).await?;
        let raw: ExchangeResponseStatusRaw =
            serde_json::from_str(&output).map_err(|e| Error::JsonParse(e.to_string()))?;

        raw.into_result()
    }

    pub async fn send_action<A: Action + Serialize>(
        &self,
        action: A,
    ) -> Result<ExchangeResponse, Error> {
        let prepared = self.prepare_action(self.ensure_action_nonce(action))?;
        let signer = self.signer_private_key.as_ref().ok_or(Error::SignerNotSet)?;
        let signed = prepared.sign(signer)?;
        self.send_signed_action(signed).await
    }

    /// Ensure the action carries a nonce before signing. If the caller already
    /// set one, respect it; otherwise assign a strictly-monotonic nonce so
    /// rapid-fire actions (e.g. UpdateLeverage then an order) never collide.
    fn ensure_action_nonce<A: Action>(&self, action: A) -> A {
        if action.nonce().is_some() {
            action
        } else {
            action.with_nonce(self.next_nonce())
        }
    }

    /// Monotonic nonce: current ms, bumped by 1 if we'd otherwise repeat or go
    /// backwards within the same millisecond. Lock-free via a CAS loop.
    fn next_nonce(&self) -> u64 {
        let now = Self::current_timestamp_ms();
        loop {
            let last = self.nonce_counter.load(Ordering::Relaxed);
            let next = if now > last { now } else { last + 1 };
            if self
                .nonce_counter
                .compare_exchange(last, next, Ordering::SeqCst, Ordering::Relaxed)
                .is_ok()
            {
                return next;
            }
        }
    }

    /// Send a wrapped Hyperliquid `multiSig` action.
    ///
    /// Breaking change: this now requires caller-provided `inner_signatures`
    /// (already ordered and threshold-valid) and submits the protocol wrapper:
    /// `{type:"multiSig", signatureChainId, signatures, payload:{multiSigUser, outerSigner, action}}`.
    ///
    /// The outer signature is computed over `{payload, signatureChainId, signatures}`
    /// (without top-level `type`) using the standard nonce/expiration conventions.
    pub async fn send_multisig_action<A: Action + Serialize>(
        &self,
        action: A,
        multi_sig_user: Address,
        outer_signer: Address,
        inner_signatures: Vec<Signature>,
    ) -> Result<ExchangeResponse, Error> {
        let signed_multisig = self.build_signed_multisig_action(
            action,
            multi_sig_user,
            outer_signer,
            inner_signatures,
        )?;
        let output = self.http_client.post("/exchange", signed_multisig).await?;
        let raw: ExchangeResponseStatusRaw =
            serde_json::from_str(&output).map_err(|e| Error::JsonParse(e.to_string()))?;
        raw.into_result()
    }

    /// Sign the inner multisig payload `(multiSigUser, outerSigner, action)` with this
    /// client's configured signer.
    ///
    /// This is useful when the outer signer is also an authorized inner signer.
    pub fn sign_multisig_inner_action<A: Action + Serialize>(
        &self,
        action: A,
        multi_sig_user: Address,
        outer_signer: Address,
    ) -> Result<Signature, Error> {
        let nonce = action.nonce().unwrap_or_else(Self::current_timestamp_ms);
        let action = action.with_nonce(nonce);
        let signing_chain = self.base_url.get_signing_chain().clone();
        let inner_payload = (
            multi_sig_user.to_string().to_lowercase(),
            outer_signer.to_string().to_lowercase(),
            crate::actions::L1ActionWrapper { action: &action },
        );
        let connection_id = compute_l1_hash(&inner_payload, nonce, None, self.expires_after)?;
        let signing_hash = agent_signing_hash(connection_id, &signing_chain.get_source());

        let signer = self
            .signer_private_key
            .as_ref()
            .ok_or(Error::SignerNotSet)?;
        signer
            .sign_hash_sync(&signing_hash)
            .map_err(|e| Error::SignatureFailure(e.to_string()))
    }

    fn build_signed_multisig_action<A: Action + Serialize>(
        &self,
        action: A,
        multi_sig_user: Address,
        outer_signer: Address,
        inner_signatures: Vec<Signature>,
    ) -> Result<SignedMultiSigAction, Error> {
        let nonce = action.nonce().unwrap_or_else(Self::current_timestamp_ms);
        let action = action.with_nonce(nonce);
        let signing_chain = self.base_url.get_signing_chain().clone();
        let wrapped_action_value = build_action_value(&action, Some(&signing_chain))
            .map_err(Error::SerializationFailure)?;
        let payload = MultiSigPayload {
            multi_sig_user,
            outer_signer,
            action: wrapped_action_value,
        };
        let wrapped_action = MultiSigAction::new(
            signature_chain_id_hex(&signing_chain),
            inner_signatures,
            payload,
        );
        let signing_hash = multisig_outer_signing_hash_with_action(
            &action,
            multi_sig_user,
            outer_signer,
            wrapped_action.signature_chain_id.clone(),
            wrapped_action.signatures.clone(),
            &signing_chain,
            nonce,
            self.expires_after,
        )?;

        let signer = self
            .signer_private_key
            .as_ref()
            .ok_or(Error::SignerNotSet)?;
        let signature = signer
            .sign_hash_sync(&signing_hash)
            .map_err(|e| Error::SignatureFailure(e.to_string()))?;

        let signed = SignedMultiSigAction {
            action: wrapped_action,
            nonce,
            signature,
            expires_after: self.expires_after,
        };

        Ok(signed)
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use alloy::{primitives::U256, signers::local::PrivateKeySigner};
    use rust_decimal_macros::dec;

    use super::ExchangeClient;
    use crate::{actions::ToggleBigBlocks, BaseUrl, UsdSend};

    #[test]
    fn multisig_outer_signing_is_deterministic() {
        let signer = PrivateKeySigner::from_str(
            "0x0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
        )
        .unwrap();
        let outer_signer = signer.address();
        let client = ExchangeClient::new(BaseUrl::Testnet).with_signer(signer);
        let action = ToggleBigBlocks {
            using_big_blocks: true,
            nonce: Some(1_700_000_000_000),
        };
        let inner_signatures = vec![
            alloy_signer::Signature::new(U256::from(1), U256::from(2), false),
            alloy_signer::Signature::new(U256::from(3), U256::from(4), true),
        ];

        let signed_a = client
            .build_signed_multisig_action(
                action.clone(),
                alloy::primitives::Address::repeat_byte(0x11),
                outer_signer,
                inner_signatures.clone(),
            )
            .unwrap();
        let signed_b = client
            .build_signed_multisig_action(
                action,
                alloy::primitives::Address::repeat_byte(0x11),
                outer_signer,
                inner_signatures,
            )
            .unwrap();

        assert_eq!(signed_a.signature, signed_b.signature);
        assert_eq!(
            serde_json::to_value(&signed_a).unwrap(),
            serde_json::to_value(&signed_b).unwrap()
        );
    }

    #[test]
    fn normal_sign_action_serialization_unchanged() {
        let signer = PrivateKeySigner::from_str(
            "0x0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
        )
        .unwrap();
        let client = ExchangeClient::new(BaseUrl::Testnet);
        let action = UsdSend {
            destination: alloy::primitives::Address::repeat_byte(0x22),
            amount: dec!(1.5),
            nonce: Some(1_700_000_000_000),
        };

        let signed = client.sign_action(action, &signer).unwrap();
        let value = serde_json::to_value(signed).unwrap();
        let action_obj = value.get("action").unwrap();

        assert_eq!(
            action_obj.get("type").and_then(|v| v.as_str()),
            Some("usdSend")
        );
        assert_eq!(
            action_obj.get("signatureChainId").and_then(|v| v.as_str()),
            Some("0x66eee")
        );
        assert!(action_obj.get("time").is_some());
    }
}
