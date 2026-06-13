use alloy::{
    dyn_abi::DynSolValue,
    primitives::{keccak256, Address, B256, U256},
    sol_types::eip712_domain,
};
use alloy_signer::Signature;
use serde::{
    ser::{SerializeSeq, SerializeStruct},
    Serialize, Serializer,
};

use crate::{
    actions::serialization::{serialize_sig, WireValue},
    Error, SigningChain,
};

use super::{
    agent_signing_hash, build_action_value, build_multisig_inner_action_value,
    compute_l1_hash, core::current_timestamp_ms, traits::UserSignedAction, Action, L1ActionWrapper,
};

/// Wrapper payload used by Hyperliquid `multiSig` actions.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MultiSigPayload {
    #[serde(serialize_with = "crate::actions::serialization::ser_lowercase")]
    pub multi_sig_user: Address,
    #[serde(serialize_with = "crate::actions::serialization::ser_lowercase")]
    pub outer_signer: Address,
    pub action: serde_json::Value,
}

/// Outbound `action` object for multi-sig submission.
///
/// Field order is intentionally stable to preserve signing/msgpack parity with
/// markets-internal: `type`, `signatureChainId`, `signatures`, `payload`.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MultiSigAction {
    #[serde(rename = "type")]
    pub action_type: &'static str,
    pub signature_chain_id: String,
    #[serde(serialize_with = "serialize_signature_vec")]
    pub signatures: Vec<Signature>,
    pub payload: MultiSigPayload,
}

/// Canonical preimage object for outer multi-sig signing.
///
/// This excludes the top-level `type` tag by protocol definition.
/// Field order is intentionally stable: `payload`, `signatureChainId`,
/// `signatures`.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MultiSigSigningPayload {
    pub payload: MultiSigPayload,
    pub signature_chain_id: String,
    #[serde(serialize_with = "serialize_signature_vec")]
    pub signatures: Vec<Signature>,
}

impl MultiSigAction {
    pub const ACTION_TYPE: &'static str = "multiSig";

    pub fn new(
        signature_chain_id: String,
        signatures: Vec<Signature>,
        payload: MultiSigPayload,
    ) -> Self {
        Self {
            action_type: Self::ACTION_TYPE,
            signature_chain_id,
            signatures,
            payload,
        }
    }

    pub fn to_signing_payload(&self) -> MultiSigSigningPayload {
        MultiSigSigningPayload {
            payload: self.payload.clone(),
            signature_chain_id: self.signature_chain_id.clone(),
            signatures: self.signatures.clone(),
        }
    }
}

pub(crate) fn signature_chain_id_hex(signing_chain: &SigningChain) -> String {
    format!("0x{:x}", signing_chain.get_signature_chain_id())
}

fn send_multisig_envelope_signing_hash(
    multi_sig_action_hash: B256,
    signing_chain: &SigningChain,
    nonce: u64,
) -> B256 {
    let type_hash = keccak256(
        "HyperliquidTransaction:SendMultiSig(string hyperliquidChain,bytes32 multiSigActionHash,uint64 nonce)",
    );
    let values = vec![
        DynSolValue::FixedBytes(type_hash, 32),
        DynSolValue::FixedBytes(keccak256(signing_chain.get_hyperliquid_chain()), 32),
        DynSolValue::FixedBytes(multi_sig_action_hash, 32),
        DynSolValue::Uint(U256::from(nonce), 64),
    ];
    let struct_hash = keccak256(DynSolValue::Tuple(values).abi_encode());

    let domain = eip712_domain! {
        name: "HyperliquidSignTransaction",
        version: "1",
        chain_id: signing_chain.get_signature_chain_id(),
        verifying_contract: Address::ZERO,
    };
    let domain_hash = domain.hash_struct();

    let mut digest = [0u8; 66];
    digest[0] = 0x19;
    digest[1] = 0x01;
    digest[2..34].copy_from_slice(&domain_hash[..]);
    digest[34..66].copy_from_slice(&struct_hash[..]);
    keccak256(digest)
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct MultiSigPayloadForHash<'a, A: Action + Serialize> {
    #[serde(serialize_with = "crate::actions::serialization::ser_lowercase")]
    multi_sig_user: Address,
    #[serde(serialize_with = "crate::actions::serialization::ser_lowercase")]
    outer_signer: Address,
    action: L1ActionWrapper<'a, A>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct MultiSigSigningPayloadForHash<'a, A: Action + Serialize> {
    signature_chain_id: String,
    #[serde(serialize_with = "serialize_signature_vec")]
    signatures: Vec<Signature>,
    payload: MultiSigPayloadForHash<'a, A>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct MultiSigPayloadForHashValue {
    #[serde(serialize_with = "crate::actions::serialization::ser_lowercase")]
    multi_sig_user: Address,
    #[serde(serialize_with = "crate::actions::serialization::ser_lowercase")]
    outer_signer: Address,
    action: WireValue,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct MultiSigSigningPayloadForHashValue {
    signature_chain_id: String,
    #[serde(serialize_with = "serialize_signature_vec")]
    signatures: Vec<Signature>,
    payload: MultiSigPayloadForHashValue,
}

pub(crate) fn multisig_outer_signing_hash_with_action<A: Action + Serialize>(
    action: &A,
    multi_sig_user: Address,
    outer_signer: Address,
    signature_chain_id: String,
    signatures: Vec<Signature>,
    signing_chain: &SigningChain,
    nonce: u64,
    expires_after: Option<u64>,
) -> Result<B256, Error> {
    let signing_payload = MultiSigSigningPayloadForHash {
        signature_chain_id,
        signatures,
        payload: MultiSigPayloadForHash {
            multi_sig_user,
            outer_signer,
            action: L1ActionWrapper { action },
        },
    };
    let multi_sig_action_hash = compute_l1_hash(&signing_payload, nonce, None, expires_after)?;

    Ok(send_multisig_envelope_signing_hash(
        multi_sig_action_hash,
        signing_chain,
        nonce,
    ))
}

/// Outer multi-sig hash for user-signed inner actions using the wire-format payload.
pub(crate) fn multisig_outer_signing_hash_with_payload_action(
    payload_action: serde_json::Value,
    multi_sig_user: Address,
    outer_signer: Address,
    signature_chain_id: String,
    signatures: Vec<Signature>,
    signing_chain: &SigningChain,
    nonce: u64,
    expires_after: Option<u64>,
) -> Result<B256, Error> {
    let signing_payload = MultiSigSigningPayloadForHashValue {
        signature_chain_id,
        signatures,
        payload: MultiSigPayloadForHashValue {
            multi_sig_user,
            outer_signer,
            action: WireValue(payload_action.clone()),
        },
    };
    let multi_sig_action_hash = compute_l1_hash(&signing_payload, nonce, None, expires_after)?;

    Ok(send_multisig_envelope_signing_hash(
        multi_sig_action_hash,
        signing_chain,
        nonce,
    ))
}

fn serialize_signature_vec<S>(signatures: &[Signature], serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let mut seq = serializer.serialize_seq(Some(signatures.len()))?;
    for sig in signatures {
        seq.serialize_element(&SignatureWire(sig))?;
    }
    seq.end()
}

struct SignatureWire<'a>(&'a Signature);

impl<'a> Serialize for SignatureWire<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serialize_sig(self.0, serializer)
    }
}

/// Request envelope submitted to `/exchange` for multisig actions.
///
/// This mirrors `SignedAction` serialization behavior.
#[derive(Debug, Clone)]
pub struct SignedMultiSigAction {
    pub action: MultiSigAction,
    pub nonce: u64,
    pub signature: Signature,
    pub expires_after: Option<u64>,
}

impl Serialize for SignedMultiSigAction {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("SignedMultiSigAction", 4)?;
        state.serialize_field("action", &self.action)?;
        state.serialize_field("nonce", &self.nonce)?;
        state.serialize_field("signature", &SignatureWire(&self.signature))?;
        if let Some(expires_after) = self.expires_after {
            state.serialize_field("expiresAfter", &expires_after)?;
        }
        state.end()
    }
}

/// Hashes required to sign a multisig action with an external signer (e.g. KMS/enclave).
#[derive(Debug, Clone)]
pub struct MultisigSigningHashes {
    pub nonce: u64,
    pub inner_signing_hash: B256,
}

/// Compute the inner multisig signing hash for L1 actions `(multiSigUser, outerSigner, action)`.
pub fn multisig_inner_signing_hash<A: Action + Serialize>(
    action: A,
    multi_sig_user: Address,
    outer_signer: Address,
    signing_chain: &SigningChain,
    expires_after: Option<u64>,
) -> Result<(A, MultisigSigningHashes), Error> {
    if A::is_user_signed() {
        return Err(Error::GenericParse(
            "use multisig_inner_user_signed_signing_hash for user-signed actions".to_string(),
        ));
    }

    let nonce = action.nonce().unwrap_or_else(current_timestamp_ms);
    let action = action.with_nonce(nonce);
    let inner_payload = (
        multi_sig_user.to_string().to_lowercase(),
        outer_signer.to_string().to_lowercase(),
        L1ActionWrapper { action: &action },
    );
    let connection_id = compute_l1_hash(&inner_payload, nonce, None, expires_after)?;
    let inner_signing_hash = agent_signing_hash(connection_id, &signing_chain.get_source());
    Ok((
        action,
        MultisigSigningHashes {
            nonce,
            inner_signing_hash,
        },
    ))
}

/// Compute the inner multisig signing hash for user-signed actions (e.g. `SpotTransfer`).
pub fn multisig_inner_user_signed_signing_hash<A: UserSignedAction + Action + Serialize>(
    action: A,
    multi_sig_user: Address,
    outer_signer: Address,
    signing_chain: &SigningChain,
) -> Result<(A, MultisigSigningHashes), Error> {
    let nonce = action.nonce().unwrap_or_else(current_timestamp_ms);
    let action = action.with_nonce(nonce);
    let inner_signing_hash =
        action.multisig_eip712_signing_hash(signing_chain, multi_sig_user, outer_signer)?;
    Ok((
        action,
        MultisigSigningHashes {
            nonce,
            inner_signing_hash,
        },
    ))
}

/// Build the wrapped `multiSig` action object with caller-provided inner signatures.
pub fn build_multisig_action<A: Action + Serialize>(
    action: &A,
    multi_sig_user: Address,
    outer_signer: Address,
    inner_signatures: Vec<Signature>,
    signing_chain: &SigningChain,
) -> Result<MultiSigAction, Error> {
    let wrapped_action_value = if A::is_user_signed() {
        build_multisig_inner_action_value(action, signing_chain)
    } else {
        build_action_value(action, Some(signing_chain))
    }
    .map_err(Error::SerializationFailure)?;
    Ok(MultiSigAction::new(
        signature_chain_id_hex(signing_chain),
        inner_signatures,
        MultiSigPayload {
            multi_sig_user,
            outer_signer,
            action: wrapped_action_value,
        },
    ))
}

/// Compute the outer multisig envelope signing hash.
pub fn multisig_outer_signing_hash<A: Action + Serialize>(
    action: &A,
    multi_sig_user: Address,
    outer_signer: Address,
    wrapped_action: &MultiSigAction,
    signing_chain: &SigningChain,
    nonce: u64,
    expires_after: Option<u64>,
) -> Result<B256, Error> {
    if A::is_user_signed() {
        multisig_outer_signing_hash_with_payload_action(
            wrapped_action.payload.action.clone(),
            multi_sig_user,
            outer_signer,
            wrapped_action.signature_chain_id.clone(),
            wrapped_action.signatures.clone(),
            signing_chain,
            nonce,
            expires_after,
        )
    } else {
        multisig_outer_signing_hash_with_action(
            action,
            multi_sig_user,
            outer_signer,
            wrapped_action.signature_chain_id.clone(),
            wrapped_action.signatures.clone(),
            signing_chain,
            nonce,
            expires_after,
        )
    }
}

/// Assemble a signed multisig action ready for `/exchange` submission.
pub fn assemble_signed_multisig_action(
    wrapped_action: MultiSigAction,
    nonce: u64,
    outer_signature: Signature,
    expires_after: Option<u64>,
) -> SignedMultiSigAction {
    SignedMultiSigAction {
        action: wrapped_action,
        nonce,
        signature: outer_signature,
        expires_after,
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use alloy::primitives::{Address, U256};
    use alloy::signers::local::PrivateKeySigner;
    use alloy_signer::{Signature, SignerSync};
    use rust_decimal_macros::dec;
    use serde::Serialize;
    use serde_json::json;

    use super::{
        build_multisig_action, multisig_inner_signing_hash, multisig_inner_user_signed_signing_hash,
        multisig_outer_signing_hash, send_multisig_envelope_signing_hash, signature_chain_id_hex,
        MultiSigAction, MultiSigPayload,
    };
    use crate::actions::compute_l1_hash;
    use crate::actions::traits::UserSignedAction;
    use crate::{SpotTransfer, ToggleBigBlocks, SigningChain};

    fn test_signature(v: u64) -> Signature {
        Signature::new(U256::from(v), U256::from(v + 1), false)
    }

    #[derive(Serialize)]
    #[serde(rename_all = "camelCase")]
    struct MultiSigSigningHashPreimage {
        signature_chain_id: String,
        #[serde(serialize_with = "super::serialize_signature_vec")]
        signatures: Vec<Signature>,
        payload: MultiSigPayload,
    }

    fn multisig_outer_signing_hash_from_payload(
        signing_payload: &super::MultiSigSigningPayload,
        signing_chain: &SigningChain,
        nonce: u64,
        expires_after: Option<u64>,
    ) -> Result<alloy::primitives::B256, crate::Error> {
        let signing_preimage = MultiSigSigningHashPreimage {
            signature_chain_id: signing_payload.signature_chain_id.clone(),
            signatures: signing_payload.signatures.clone(),
            payload: signing_payload.payload.clone(),
        };
        let multi_sig_action_hash = compute_l1_hash(&signing_preimage, nonce, None, expires_after)?;
        Ok(send_multisig_envelope_signing_hash(
            multi_sig_action_hash,
            signing_chain,
            nonce,
        ))
    }

    #[test]
    fn multisig_action_json_shape_stable() {
        let payload = MultiSigPayload {
            multi_sig_user: Address::repeat_byte(0x11),
            outer_signer: Address::repeat_byte(0x22),
            action: json!({"type":"perpDeploy","setSubDeployers":{"dex":"km","subDeployers":[]}}),
        };
        let action = MultiSigAction::new(
            signature_chain_id_hex(&SigningChain::Testnet),
            vec![test_signature(1), test_signature(3)],
            payload,
        );

        let value = serde_json::to_value(action).unwrap();
        assert_eq!(value.get("type").and_then(|v| v.as_str()), Some("multiSig"));
        assert_eq!(
            value.get("signatureChainId").and_then(|v| v.as_str()),
            Some("0x66eee")
        );
        assert!(value.get("signatures").unwrap().is_array());
        let payload = value.get("payload").unwrap();
        assert_eq!(
            payload.get("multiSigUser").and_then(|v| v.as_str()),
            Some("0x1111111111111111111111111111111111111111")
        );
        assert_eq!(
            payload.get("outerSigner").and_then(|v| v.as_str()),
            Some("0x2222222222222222222222222222222222222222")
        );
    }

    #[test]
    fn multisig_outer_hash_is_deterministic() {
        let payload = MultiSigPayload {
            multi_sig_user: Address::repeat_byte(0x44),
            outer_signer: Address::repeat_byte(0x55),
            action: json!({"type":"noop"}),
        };
        let action = MultiSigAction::new(
            signature_chain_id_hex(&SigningChain::Testnet),
            vec![test_signature(7), test_signature(9)],
            payload,
        );
        let signing_payload = action.to_signing_payload();

        let hash_a = multisig_outer_signing_hash_from_payload(
            &signing_payload,
            &SigningChain::Testnet,
            1700000000000,
            None,
        )
        .unwrap();
        let hash_b = multisig_outer_signing_hash_from_payload(
            &signing_payload,
            &SigningChain::Testnet,
            1700000000000,
            None,
        )
        .unwrap();
        assert_eq!(hash_a, hash_b);
    }

    #[test]
    fn l1_inner_hash_is_deterministic() {
        let signing_chain = SigningChain::Testnet;
        let action = ToggleBigBlocks {
            using_big_blocks: true,
            nonce: Some(1_700_000_000_000),
        };
        let multi_sig_user = Address::repeat_byte(0x11);
        let outer_signer = Address::repeat_byte(0x22);

        let (_, hashes_a) =
            multisig_inner_signing_hash(action.clone(), multi_sig_user, outer_signer, &signing_chain, None)
                .unwrap();
        let (_, hashes_b) =
            multisig_inner_signing_hash(action, multi_sig_user, outer_signer, &signing_chain, None).unwrap();

        assert_eq!(hashes_a.inner_signing_hash, hashes_b.inner_signing_hash);
        assert_eq!(hashes_a.nonce, 1_700_000_000_000);
    }

    #[test]
    fn l1_inner_hash_rejects_user_signed_actions() {
        let action = SpotTransfer::new(Address::repeat_byte(0xaa), "HYPE", dec!(1.0));
        let err = multisig_inner_signing_hash(
            action,
            Address::repeat_byte(0x11),
            Address::repeat_byte(0x22),
            &SigningChain::Testnet,
            None,
        )
        .unwrap_err();
        assert!(err.to_string().contains("multisig_inner_user_signed_signing_hash"));
    }

    #[test]
    fn l1_outer_hash_matches_public_helper() {
        let signing_chain = SigningChain::Testnet;
        let action = ToggleBigBlocks {
            using_big_blocks: true,
            nonce: Some(1_700_000_000_000),
        };
        let multi_sig_user = Address::repeat_byte(0x11);
        let outer_signer = Address::repeat_byte(0x22);
        let inner_signatures = vec![
            Signature::new(U256::from(1), U256::from(2), false),
            Signature::new(U256::from(3), U256::from(4), true),
        ];

        let wrapped = build_multisig_action(
            &action,
            multi_sig_user,
            outer_signer,
            inner_signatures.clone(),
            &signing_chain,
        )
        .unwrap();

        let hash_a = multisig_outer_signing_hash(
            &action,
            multi_sig_user,
            outer_signer,
            &wrapped,
            &signing_chain,
            1_700_000_000_000,
            None,
        )
        .unwrap();
        let hash_b = multisig_outer_signing_hash(
            &action,
            multi_sig_user,
            outer_signer,
            &wrapped,
            &signing_chain,
            1_700_000_000_000,
            None,
        )
        .unwrap();
        assert_eq!(hash_a, hash_b);
    }

    #[test]
    fn user_signed_inner_hash_matches_local_signer() {
        let signing_chain = SigningChain::Testnet;
        let signer = PrivateKeySigner::from_str(
            "0x0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
        )
        .unwrap();
        let multi_sig_user = Address::repeat_byte(0x11);
        let outer_signer = signer.address();
        let action = SpotTransfer::new(Address::repeat_byte(0xaa), "HYPE", dec!(0.01));
        let (action, hashes) = multisig_inner_user_signed_signing_hash(
            action,
            multi_sig_user,
            outer_signer,
            &signing_chain,
        )
        .unwrap();

        let sig_from_hash = signer.sign_hash_sync(&hashes.inner_signing_hash).unwrap();
        let sig_direct = action
            .multisig_eip712_signing_hash(&signing_chain, multi_sig_user, outer_signer)
            .unwrap();
        let sig_from_direct = signer.sign_hash_sync(&sig_direct).unwrap();
        assert_eq!(sig_from_hash, sig_from_direct);
    }

    #[test]
    fn user_signed_outer_hash_is_deterministic() {
        let signing_chain = SigningChain::Testnet;
        let multi_sig_user = Address::repeat_byte(0x11);
        let outer_signer = Address::repeat_byte(0x22);
        let nonce = 1_700_000_000_000;
        let mut action = SpotTransfer::new(Address::repeat_byte(0xaa), "HYPE", dec!(0.01));
        action.nonce = Some(nonce);
        let inner_signatures = vec![Signature::new(U256::from(7), U256::from(9), false)];

        let wrapped = build_multisig_action(
            &action,
            multi_sig_user,
            outer_signer,
            inner_signatures,
            &signing_chain,
        )
        .unwrap();

        let hash_a = multisig_outer_signing_hash(
            &action,
            multi_sig_user,
            outer_signer,
            &wrapped,
            &signing_chain,
            nonce,
            None,
        )
        .unwrap();
        let hash_b = multisig_outer_signing_hash(
            &action,
            multi_sig_user,
            outer_signer,
            &wrapped,
            &signing_chain,
            nonce,
            None,
        )
        .unwrap();
        assert_eq!(hash_a, hash_b);
    }
}
