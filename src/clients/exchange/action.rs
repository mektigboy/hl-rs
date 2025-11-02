use alloy::{
    primitives::{Address, B256, Signature},
    signers::{SignerSync, local::PrivateKeySigner},
};
use serde::{Deserialize, Serialize, Serializer, ser::SerializeStruct};
use serde_json::Value;

use crate::{Error, Result, http::HttpClient, utils::sign_l1_action};

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ExchangePayload {
    action: serde_json::Value,
    #[serde(serialize_with = "serialize_sig")]
    signature: Signature,
    nonce: u64,
    vault_address: Option<Address>,
}

fn serialize_sig<S>(sig: &Signature, s: S) -> std::result::Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let mut state = s.serialize_struct("Signature", 3)?;
    state.serialize_field("r", &sig.r())?;
    state.serialize_field("s", &sig.s())?;
    state.serialize_field("v", &(27 + sig.v() as u64))?;
    state.end()
}

/// Unsigned action ready to be signed.
///
/// Represents a fully prepared action that has been built with all
/// necessary metadata (timestamp, vault address, signing data) but has
/// not yet been signed.
///
/// # Example
/// ```no_run
/// let action = ActionKind::UsdSend(usd_send).build(&client)?;
/// let signed = action.sign(&wallet)?;
/// ```
pub struct Action {
    pub action: Value,
    pub nonce: u64,
    pub vault_address: Option<Address>,
    pub signing_data: SigningData,
    pub http_client: HttpClient,
}

/// Enum representing data needed for signing an action.
#[derive(Debug, Clone)]
pub enum SigningData {
    /// L1 actions require a connection_id and network type.
    L1 {
        connection_id: B256,
        is_mainnet: bool,
    },
    /// Typed data actions require only the EIP-712 hash.
    TypedData { hash: B256 },
}

/// Signed action ready to be sent to the Hyperliquid API.
///
/// This action has been fully prepared and signed, and can be sent
/// immediately to the exchange.
pub struct SignedAction {
    pub action: Value,
    pub nonce: u64,
    pub signature: Signature,
    pub vault_address: Option<Address>,
    pub http_client: HttpClient,
}

impl Action {
    /// Sign action with the provided wallet.
    pub fn sign(self, wallet: &PrivateKeySigner) -> Result<SignedAction> {
        let signature = match self.signing_data {
            SigningData::L1 {
                connection_id,
                is_mainnet,
            } => sign_l1_action(wallet, connection_id, is_mainnet)?,
            SigningData::TypedData { hash } => wallet
                .sign_hash_sync(&hash)
                .map_err(|e| Error::SignatureFailure(e.to_string()))?,
        };

        Ok(SignedAction {
            action: self.action,
            nonce: self.nonce,
            signature,
            vault_address: self.vault_address,
            http_client: self.http_client,
        })
    }

    /// Attach externally-provided signature to this action.
    /// Use this when signing is done outside the SDK (e.g., using Nitro Enclave).
    pub fn with_signature(self, signature: Signature) -> SignedAction {
        SignedAction {
            action: self.action,
            nonce: self.nonce,
            signature,
            vault_address: self.vault_address,
            http_client: self.http_client,
        }
    }

    /// Get signing data needed for external signing.
    pub fn signing_data(&self) -> &SigningData {
        &self.signing_data
    }
}

impl SignedAction {
    /// Send signed action to Hyperliquid API.
    pub async fn send(self) -> Result<crate::exchange::responses::ExchangeResponseStatus> {
        let exchange_payload = ExchangePayload {
            action: self.action,
            signature: self.signature,
            nonce: self.nonce,
            vault_address: self.vault_address,
        };

        let res = serde_json::to_string(&exchange_payload)
            .map_err(|e| Error::JsonParse(e.to_string()))?;

        let output = self.http_client.post("/exchange", res).await?;

        serde_json::from_str(&output).map_err(|e| Error::JsonParse(e.to_string()))
    }
}
