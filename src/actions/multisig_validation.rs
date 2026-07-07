//! Validation of inbound Hyperliquid `multiSig` transactions.
//!
//! This is the inverse of the signing path in [`super::multisig`]. Given a
//! request in exactly the shape Hyperliquid's `/exchange` endpoint receives, we
//! recompute the hashes each party signed and recover their addresses, so a
//! leader (or an off-chain policy engine) can decide whether the transaction is
//! authorized *before* it is broadcast.
//!
//! # What is checked
//! 1. The outer action really is a `multiSig` action for the expected multisig user.
//! 2. Each inner signature recovers to a distinct **authorized** signer.
//! 3. The number of distinct authorized signers meets the `threshold`.
//! 4. The leader (`outerSigner`) is itself an authorized signer.
//! 5. The outer/leader signature recovers to `outerSigner`.
//!
//! # Signing scheme background
//! Every authorized signer signs an **inner** hash over
//! `[multiSigUser, outerSigner, action]`. For L1 actions (e.g. `perpDeploy`)
//! this is the msgpack `connectionId` wrapped in the `Agent` EIP-712 domain; for
//! user-signed actions it is an EIP-712 typed-data hash. The leader then signs an
//! **outer** envelope (`HyperliquidTransaction:SendMultiSig`) over the collected
//! signatures. See <https://hyperliquid.gitbook.io/hyperliquid-docs/hypercore/multi-sig>.
//!
//! Because `serde_json` preserves object key order in this crate, the inner
//! `action` value can be re-hashed straight from the wire without reconstructing
//! the concrete typed action — which is what makes validating an *unknown*
//! perp-deploy action possible.

use std::collections::HashSet;

use alloy::primitives::{Address, B256};
use alloy_signer::Signature;
use serde::Deserialize;

use crate::{Error, SigningChain};

use super::{
    agent_signing_hash, compute_l1_hash,
    multisig::multisig_outer_signing_hash_with_payload_action, serialization::WireValue,
    ActionKind,
};

/// A `multiSig` request in the exact JSON shape submitted to `/exchange`.
///
/// ```json
/// {
///   "action": {
///     "type": "multiSig",
///     "signatureChainId": "0x66eee",
///     "signatures": [{ "r": "0x..", "s": "0x..", "v": 27 }, ...],
///     "payload": { "multiSigUser": "0x..", "outerSigner": "0x..", "action": { .. } }
///   },
///   "nonce": 1700000000000,
///   "signature": { "r": "0x..", "s": "0x..", "v": 27 },
///   "vaultAddress": "0x..",   // optional
///   "expiresAfter": 1700000000000 // optional
/// }
/// ```
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MultiSigRequest {
    pub action: MultiSigRequestAction,
    pub nonce: u64,
    /// The leader's (outer) signature over the `SendMultiSig` envelope.
    #[serde(deserialize_with = "super::serialization::deserialize_sig")]
    pub signature: Signature,
    #[serde(default)]
    pub vault_address: Option<Address>,
    #[serde(default)]
    pub expires_after: Option<u64>,
}

/// The outer `multiSig` action object.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MultiSigRequestAction {
    #[serde(rename = "type")]
    pub action_type: String,
    pub signature_chain_id: String,
    /// Inner signatures collected from the authorized signers, in submission order.
    #[serde(deserialize_with = "deserialize_sig_vec")]
    pub signatures: Vec<Signature>,
    pub payload: MultiSigRequestPayload,
}

/// The `payload` wrapping the inner action and its multisig context.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MultiSigRequestPayload {
    pub multi_sig_user: Address,
    pub outer_signer: Address,
    /// The inner action, kept as a raw value so it can be re-hashed exactly as received.
    pub action: serde_json::Value,
}

fn deserialize_sig_vec<'de, D>(deserializer: D) -> Result<Vec<Signature>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    struct SigDe(Signature);
    impl<'de> Deserialize<'de> for SigDe {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: serde::Deserializer<'de>,
        {
            Ok(SigDe(super::serialization::deserialize_sig(deserializer)?))
        }
    }
    let sigs = Vec::<SigDe>::deserialize(deserializer)?;
    Ok(sigs.into_iter().map(|s| s.0).collect())
}

/// Successful validation report.
#[derive(Debug, Clone)]
pub struct MultisigValidation {
    /// The inner hash every authorized signer signed.
    pub inner_signing_hash: B256,
    /// The outer envelope hash the leader signed.
    pub outer_signing_hash: B256,
    /// The distinct authorized signers whose inner signatures were verified, in signature order.
    pub signers: Vec<Address>,
    /// The leader (`outerSigner`).
    pub leader: Address,
}

/// A reason a multisig transaction failed validation.
#[derive(Debug, Clone, thiserror::Error)]
pub enum MultisigValidationError {
    #[error("action.type is `{found}`, expected `multiSig`")]
    NotMultiSigAction { found: String },

    #[error("payload.multiSigUser {found} does not match expected multisig user {expected}")]
    MultiSigUserMismatch { expected: Address, found: Address },

    #[error("invalid threshold {threshold}: must be between 1 and the number of authorized signers ({authorized})")]
    InvalidThreshold { threshold: usize, authorized: usize },

    #[error("leader (outerSigner) {leader} is not an authorized signer")]
    LeaderNotAuthorized { leader: Address },

    #[error("leader (outerSigner) {leader} must not be the multisig user itself")]
    LeaderIsMultiSigUser { leader: Address },

    #[error(
        "inner action `{action_type}` is user-signed (EIP-712); validate it with \
         `validate_multisig_user_signed_action` using the typed action"
    )]
    UserSignedInnerNotSupported { action_type: String },

    #[error("no inner signatures present")]
    NoSignatures,

    #[error("inner signature #{index} could not be recovered: {reason}")]
    UnrecoverableSignature { index: usize, reason: String },

    #[error("inner signature #{index} recovered to {recovered}, which is not an authorized signer")]
    UnauthorizedSigner { index: usize, recovered: Address },

    #[error("authorized signer {recovered} signed more than once")]
    DuplicateSigner { recovered: Address },

    #[error("only {valid} distinct authorized signatures, but threshold is {threshold}")]
    ThresholdNotMet { valid: usize, threshold: usize },

    #[error("outer/leader signature could not be recovered: {reason}")]
    UnrecoverableOuterSignature { reason: String },

    #[error("outer/leader signature recovered to {recovered}, expected leader {expected}")]
    InvalidOuterSignature {
        recovered: Address,
        expected: Address,
    },

    #[error("failed to recompute a signing hash: {0}")]
    HashComputation(String),
}

impl From<Error> for MultisigValidationError {
    fn from(e: Error) -> Self {
        MultisigValidationError::HashComputation(e.to_string())
    }
}

impl MultiSigRequest {
    /// Parse a `multiSig` request from the JSON `/exchange` body.
    pub fn from_json(json: &str) -> Result<Self, Error> {
        serde_json::from_str(json).map_err(|e| Error::JsonParse(e.to_string()))
    }

    /// Best-effort classification of the inner action.
    ///
    /// This is the ergonomic entry point for "determine the action, then validate
    /// it appropriately": recognized actions (including `perpDeploy`/`spotDeploy`
    /// sum types) map to a concrete [`ActionKind`], and anything unrecognized
    /// becomes [`ActionKind::Unknown`] with the raw value preserved. It never
    /// affects validation, which hashes the raw wire value regardless.
    pub fn inner_action_kind(&self) -> Result<ActionKind, Error> {
        let obj = self
            .action
            .payload
            .action
            .as_object()
            .ok_or_else(|| Error::JsonParse("inner action must be a JSON object".to_string()))?;
        super::dispatch_action_kind::<serde_json::Error>(obj)
            .map_err(|e| Error::JsonParse(e.to_string()))
    }

    /// Validate this request against a multisig user, its authorized signers, and threshold.
    ///
    /// `signing_chain` selects mainnet vs. testnet, which is required because the
    /// inner L1 hash embeds the chain `source` and is not derivable from the wire
    /// `signatureChainId` alone.
    ///
    /// Only L1 inner actions (e.g. `perpDeploy`) are supported here; for
    /// user-signed inner actions use [`validate_multisig_user_signed_action`].
    pub fn validate(
        &self,
        multi_sig_user: Address,
        authorized_signers: &[Address],
        threshold: usize,
        signing_chain: &SigningChain,
    ) -> Result<MultisigValidation, MultisigValidationError> {
        if self.action.action_type != crate::actions::MultiSigAction::ACTION_TYPE {
            return Err(MultisigValidationError::NotMultiSigAction {
                found: self.action.action_type.clone(),
            });
        }

        let payload = &self.action.payload;
        if payload.multi_sig_user != multi_sig_user {
            return Err(MultisigValidationError::MultiSigUserMismatch {
                expected: multi_sig_user,
                found: payload.multi_sig_user,
            });
        }

        // Reject user-signed inner actions: their signers use an EIP-712 hash we
        // cannot reproduce without the concrete typed action. L1 action objects
        // never carry these envelope fields.
        if let Some(obj) = payload.action.as_object() {
            if obj.contains_key("signatureChainId") || obj.contains_key("hyperliquidChain") {
                let action_type = obj
                    .get("type")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown")
                    .to_string();
                return Err(MultisigValidationError::UserSignedInnerNotSupported { action_type });
            }
        }

        // Recompute the inner L1 hash straight from the wire action value.
        let inner_payload = (
            payload.multi_sig_user.to_string().to_lowercase(),
            payload.outer_signer.to_string().to_lowercase(),
            WireValue(payload.action.clone()),
        );
        let connection_id = compute_l1_hash(
            &inner_payload,
            self.nonce,
            self.vault_address,
            self.expires_after,
        )?;
        let inner_signing_hash = agent_signing_hash(connection_id, &signing_chain.get_source());

        // Recompute the outer envelope hash from the same wire payload.
        let outer_signing_hash = multisig_outer_signing_hash_with_payload_action(
            payload.action.clone(),
            payload.multi_sig_user,
            payload.outer_signer,
            self.action.signature_chain_id.clone(),
            self.action.signatures.clone(),
            signing_chain,
            self.nonce,
            self.vault_address,
            self.expires_after,
        )?;

        finish_validation(
            inner_signing_hash,
            outer_signing_hash,
            payload.outer_signer,
            multi_sig_user,
            &self.action.signatures,
            &self.signature,
            authorized_signers,
            threshold,
        )
    }
}

/// Shared checks over recomputed hashes: signer recovery, membership, threshold, leader.
#[allow(clippy::too_many_arguments)]
fn finish_validation(
    inner_signing_hash: B256,
    outer_signing_hash: B256,
    outer_signer: Address,
    multi_sig_user: Address,
    inner_signatures: &[Signature],
    outer_signature: &Signature,
    authorized_signers: &[Address],
    threshold: usize,
) -> Result<MultisigValidation, MultisigValidationError> {
    let authorized: HashSet<Address> = authorized_signers.iter().copied().collect();

    if threshold == 0 || threshold > authorized.len() {
        return Err(MultisigValidationError::InvalidThreshold {
            threshold,
            authorized: authorized.len(),
        });
    }

    if outer_signer == multi_sig_user {
        return Err(MultisigValidationError::LeaderIsMultiSigUser {
            leader: outer_signer,
        });
    }
    if !authorized.contains(&outer_signer) {
        return Err(MultisigValidationError::LeaderNotAuthorized {
            leader: outer_signer,
        });
    }

    if inner_signatures.is_empty() {
        return Err(MultisigValidationError::NoSignatures);
    }

    let mut seen: HashSet<Address> = HashSet::new();
    let mut signers: Vec<Address> = Vec::with_capacity(inner_signatures.len());
    for (index, sig) in inner_signatures.iter().enumerate() {
        let recovered = sig
            .recover_address_from_prehash(&inner_signing_hash)
            .map_err(|e| MultisigValidationError::UnrecoverableSignature {
                index,
                reason: e.to_string(),
            })?;
        if !authorized.contains(&recovered) {
            return Err(MultisigValidationError::UnauthorizedSigner { index, recovered });
        }
        if !seen.insert(recovered) {
            return Err(MultisigValidationError::DuplicateSigner { recovered });
        }
        signers.push(recovered);
    }

    if signers.len() < threshold {
        return Err(MultisigValidationError::ThresholdNotMet {
            valid: signers.len(),
            threshold,
        });
    }

    // Verify the leader actually signed the outer envelope.
    let recovered_leader = outer_signature
        .recover_address_from_prehash(&outer_signing_hash)
        .map_err(|e| MultisigValidationError::UnrecoverableOuterSignature {
            reason: e.to_string(),
        })?;
    if recovered_leader != outer_signer {
        return Err(MultisigValidationError::InvalidOuterSignature {
            recovered: recovered_leader,
            expected: outer_signer,
        });
    }

    Ok(MultisigValidation {
        inner_signing_hash,
        outer_signing_hash,
        signers,
        leader: outer_signer,
    })
}

/// Validate a multisig L1 action from the concrete typed action.
///
/// Use this when you already hold the typed inner action (rather than a raw wire
/// request). The action must carry the exact `nonce` used at signing time
/// (`with_nonce`); `vault_address`/`expires_after` must match the outer envelope.
#[allow(clippy::too_many_arguments)]
pub fn validate_multisig_l1_action<A: super::Action + serde::Serialize>(
    action: A,
    multi_sig_user: Address,
    outer_signer: Address,
    signing_chain: &SigningChain,
    vault_address: Option<Address>,
    expires_after: Option<u64>,
    inner_signatures: &[Signature],
    outer_signature: &Signature,
    authorized_signers: &[Address],
    threshold: usize,
) -> Result<MultisigValidation, MultisigValidationError> {
    if A::is_user_signed() {
        return Err(MultisigValidationError::UserSignedInnerNotSupported {
            action_type: A::ACTION_TYPE.to_string(),
        });
    }
    let (action, hashes) = super::multisig_inner_signing_hash(
        action,
        multi_sig_user,
        outer_signer,
        signing_chain,
        vault_address,
        expires_after,
    )?;
    let wrapped = super::build_multisig_action(
        &action,
        multi_sig_user,
        outer_signer,
        inner_signatures.to_vec(),
        signing_chain,
    )?;
    let outer_signing_hash = super::multisig_outer_signing_hash(
        &action,
        multi_sig_user,
        outer_signer,
        &wrapped,
        signing_chain,
        hashes.nonce,
        vault_address,
        expires_after,
    )?;

    finish_validation(
        hashes.inner_signing_hash,
        outer_signing_hash,
        outer_signer,
        multi_sig_user,
        inner_signatures,
        outer_signature,
        authorized_signers,
        threshold,
    )
}

/// Validate a multisig user-signed action (e.g. `SpotTransfer`) from the typed action.
///
/// The action must carry the exact `nonce` used at signing time (`with_nonce`).
#[allow(clippy::too_many_arguments)]
pub fn validate_multisig_user_signed_action<
    A: super::UserSignedAction + super::Action + serde::Serialize,
>(
    action: A,
    multi_sig_user: Address,
    outer_signer: Address,
    signing_chain: &SigningChain,
    vault_address: Option<Address>,
    expires_after: Option<u64>,
    inner_signatures: &[Signature],
    outer_signature: &Signature,
    authorized_signers: &[Address],
    threshold: usize,
) -> Result<MultisigValidation, MultisigValidationError> {
    let (action, hashes) = super::multisig_inner_user_signed_signing_hash(
        action,
        multi_sig_user,
        outer_signer,
        signing_chain,
    )?;
    let wrapped = super::build_multisig_action(
        &action,
        multi_sig_user,
        outer_signer,
        inner_signatures.to_vec(),
        signing_chain,
    )?;
    let outer_signing_hash = super::multisig_outer_signing_hash(
        &action,
        multi_sig_user,
        outer_signer,
        &wrapped,
        signing_chain,
        hashes.nonce,
        vault_address,
        expires_after,
    )?;

    finish_validation(
        hashes.inner_signing_hash,
        outer_signing_hash,
        outer_signer,
        multi_sig_user,
        inner_signatures,
        outer_signature,
        authorized_signers,
        threshold,
    )
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use alloy::primitives::Address;
    use alloy::signers::local::PrivateKeySigner;
    use alloy_signer::SignerSync;

    use super::*;
    use crate::actions::{
        assemble_signed_multisig_action, build_multisig_action, multisig_inner_signing_hash,
        multisig_outer_signing_hash, SetOpenInterestCaps,
    };
    use crate::SigningChain;

    const NONCE: u64 = 1_700_000_000_000;

    fn signer(hex: &str) -> PrivateKeySigner {
        PrivateKeySigner::from_str(hex).unwrap()
    }

    /// Build a fully-signed multisig `perpDeploy` request as JSON, exactly as it
    /// would reach `/exchange`, signed by `inner_signers` with `leader` as outer.
    fn build_request_json(
        signing_chain: &SigningChain,
        multi_sig_user: Address,
        leader: &PrivateKeySigner,
        inner_signers: &[&PrivateKeySigner],
    ) -> String {
        let action = SetOpenInterestCaps {
            caps: vec![("BTC".to_string(), 1_000_000), ("ETH".to_string(), 500_000)],
            nonce: Some(NONCE),
        };

        let (action, hashes) = multisig_inner_signing_hash(
            action,
            multi_sig_user,
            leader.address(),
            signing_chain,
            None,
            None,
        )
        .unwrap();

        let inner_signatures: Vec<Signature> = inner_signers
            .iter()
            .map(|s| s.sign_hash_sync(&hashes.inner_signing_hash).unwrap())
            .collect();

        let wrapped = build_multisig_action(
            &action,
            multi_sig_user,
            leader.address(),
            inner_signatures,
            signing_chain,
        )
        .unwrap();

        let outer_hash = multisig_outer_signing_hash(
            &action,
            multi_sig_user,
            leader.address(),
            &wrapped,
            signing_chain,
            hashes.nonce,
            None,
            None,
        )
        .unwrap();
        let outer_sig = leader.sign_hash_sync(&outer_hash).unwrap();

        let signed = assemble_signed_multisig_action(wrapped, hashes.nonce, outer_sig, None, None);
        serde_json::to_string(&signed).unwrap()
    }

    #[test]
    fn valid_two_of_two_perp_deploy_passes() {
        let chain = SigningChain::Testnet;
        let multi_sig_user = Address::repeat_byte(0xee);
        let s1 = signer("0x0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef");
        let s2 = signer("0x0223456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef");
        let json = build_request_json(&chain, multi_sig_user, &s1, &[&s1, &s2]);

        let request = MultiSigRequest::from_json(&json).unwrap();
        let authorized = vec![s1.address(), s2.address()];
        let report = request
            .validate(multi_sig_user, &authorized, 2, &chain)
            .unwrap();

        assert_eq!(report.signers, vec![s1.address(), s2.address()]);
        assert_eq!(report.leader, s1.address());
    }

    #[test]
    fn inner_action_kind_classifies_perp_deploy() {
        let chain = SigningChain::Testnet;
        let multi_sig_user = Address::repeat_byte(0xee);
        let s1 = signer("0x0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef");
        let json = build_request_json(&chain, multi_sig_user, &s1, &[&s1]);

        let request = MultiSigRequest::from_json(&json).unwrap();
        match request.inner_action_kind().unwrap() {
            ActionKind::SetOpenInterestCaps(a) => {
                assert_eq!(
                    a.caps,
                    vec![("BTC".to_string(), 1_000_000), ("ETH".to_string(), 500_000)]
                );
            }
            other => panic!("expected SetOpenInterestCaps, got {other:?}"),
        }
    }

    #[test]
    fn threshold_not_met_is_rejected() {
        let chain = SigningChain::Testnet;
        let multi_sig_user = Address::repeat_byte(0xee);
        let s1 = signer("0x0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef");
        let s2 = signer("0x0223456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef");
        // Only s1 signs, but threshold is 2.
        let json = build_request_json(&chain, multi_sig_user, &s1, &[&s1]);

        let request = MultiSigRequest::from_json(&json).unwrap();
        let authorized = vec![s1.address(), s2.address()];
        let err = request
            .validate(multi_sig_user, &authorized, 2, &chain)
            .unwrap_err();
        assert!(matches!(
            err,
            MultisigValidationError::ThresholdNotMet {
                valid: 1,
                threshold: 2
            }
        ));
    }

    #[test]
    fn unauthorized_signer_is_rejected() {
        let chain = SigningChain::Testnet;
        let multi_sig_user = Address::repeat_byte(0xee);
        let s1 = signer("0x0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef");
        let s2 = signer("0x0223456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef");
        let json = build_request_json(&chain, multi_sig_user, &s1, &[&s1, &s2]);

        let request = MultiSigRequest::from_json(&json).unwrap();
        // s2 is not in the authorized set.
        let authorized = vec![s1.address(), Address::repeat_byte(0xab)];
        let err = request
            .validate(multi_sig_user, &authorized, 1, &chain)
            .unwrap_err();
        assert!(matches!(
            err,
            MultisigValidationError::UnauthorizedSigner { index: 1, .. }
        ));
    }

    #[test]
    fn tampered_outer_signature_is_rejected() {
        let chain = SigningChain::Testnet;
        let multi_sig_user = Address::repeat_byte(0xee);
        let s1 = signer("0x0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef");
        let s2 = signer("0x0223456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef");
        let json = build_request_json(&chain, multi_sig_user, &s1, &[&s1, &s2]);

        // Replace the outer signature with s2's signature over a bogus hash.
        let mut request = MultiSigRequest::from_json(&json).unwrap();
        request.signature = s2.sign_hash_sync(&B256::repeat_byte(0x01)).unwrap();

        let authorized = vec![s1.address(), s2.address()];
        let err = request
            .validate(multi_sig_user, &authorized, 2, &chain)
            .unwrap_err();
        assert!(matches!(
            err,
            MultisigValidationError::InvalidOuterSignature { .. }
        ));
    }

    #[test]
    fn multisig_user_mismatch_is_rejected() {
        let chain = SigningChain::Testnet;
        let multi_sig_user = Address::repeat_byte(0xee);
        let s1 = signer("0x0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef");
        let json = build_request_json(&chain, multi_sig_user, &s1, &[&s1]);

        let request = MultiSigRequest::from_json(&json).unwrap();
        let authorized = vec![s1.address()];
        let err = request
            .validate(Address::repeat_byte(0x99), &authorized, 1, &chain)
            .unwrap_err();
        assert!(matches!(
            err,
            MultisigValidationError::MultiSigUserMismatch { .. }
        ));
    }

    #[test]
    fn typed_user_signed_round_trip_passes() {
        use crate::actions::multisig_inner_user_signed_signing_hash;
        use crate::SpotTransfer;
        use rust_decimal_macros::dec;

        let chain = SigningChain::Testnet;
        let multi_sig_user = Address::repeat_byte(0xee);
        let s1 = signer("0x0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef");
        let s2 = signer("0x0223456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef");
        let leader = &s1;

        let mut action = SpotTransfer::new(Address::repeat_byte(0xaa), "HYPE", dec!(1.0));
        action.nonce = Some(NONCE);

        let (action, hashes) = multisig_inner_user_signed_signing_hash(
            action,
            multi_sig_user,
            leader.address(),
            &chain,
        )
        .unwrap();
        let inner_sigs = vec![
            s1.sign_hash_sync(&hashes.inner_signing_hash).unwrap(),
            s2.sign_hash_sync(&hashes.inner_signing_hash).unwrap(),
        ];

        // Compute the outer hash the same way the client would, then sign it.
        let wrapped = build_multisig_action(
            &action,
            multi_sig_user,
            leader.address(),
            inner_sigs.clone(),
            &chain,
        )
        .unwrap();
        let outer_hash = multisig_outer_signing_hash(
            &action,
            multi_sig_user,
            leader.address(),
            &wrapped,
            &chain,
            hashes.nonce,
            None,
            None,
        )
        .unwrap();
        let outer_sig = leader.sign_hash_sync(&outer_hash).unwrap();

        let authorized = vec![s1.address(), s2.address()];
        let report = validate_multisig_user_signed_action(
            action,
            multi_sig_user,
            leader.address(),
            &chain,
            None,
            None,
            &inner_sigs,
            &outer_sig,
            &authorized,
            2,
        )
        .unwrap();
        assert_eq!(report.signers, vec![s1.address(), s2.address()]);
    }

    #[test]
    fn wire_validator_rejects_user_signed_inner() {
        // A user-signed inner action carries envelope fields the wire (L1) path
        // cannot hash; it must be steered to the typed API instead of silently
        // recovering wrong addresses.
        let chain = SigningChain::Testnet;
        let multi_sig_user = Address::repeat_byte(0xee);
        let s1 = signer("0x0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef");
        let json = build_request_json(&chain, multi_sig_user, &s1, &[&s1]);
        let mut request = MultiSigRequest::from_json(&json).unwrap();
        // Splice in a user-signed-looking inner action.
        request.action.payload.action = serde_json::json!({
            "type": "spotSend",
            "signatureChainId": "0x66eee",
            "hyperliquidChain": "Testnet",
            "destination": "0x000000000000000000000000000000000000dead",
            "token": "HYPE",
            "amount": "1",
            "time": NONCE,
        });
        let err = request
            .validate(multi_sig_user, &[s1.address()], 1, &chain)
            .unwrap_err();
        assert!(matches!(
            err,
            MultisigValidationError::UserSignedInnerNotSupported { .. }
        ));
    }

    #[test]
    fn wrong_chain_recovers_wrong_signers() {
        // Signed on testnet, validated as mainnet: the inner source differs, so
        // recovery yields addresses outside the authorized set.
        let multi_sig_user = Address::repeat_byte(0xee);
        let s1 = signer("0x0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef");
        let json = build_request_json(&SigningChain::Testnet, multi_sig_user, &s1, &[&s1]);

        let request = MultiSigRequest::from_json(&json).unwrap();
        let authorized = vec![s1.address()];
        let err = request
            .validate(multi_sig_user, &authorized, 1, &SigningChain::Mainnet)
            .unwrap_err();
        assert!(matches!(
            err,
            MultisigValidationError::UnauthorizedSigner { .. }
        ));
    }
}
