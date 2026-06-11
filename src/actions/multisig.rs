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

use super::{compute_l1_hash, Action, L1ActionWrapper};

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

#[cfg(test)]
mod tests {
    use alloy::primitives::{Address, U256};
    use alloy_signer::Signature;
    use serde::Serialize;
    use serde_json::json;

    use super::{send_multisig_envelope_signing_hash, signature_chain_id_hex, MultiSigAction, MultiSigPayload};
    use crate::actions::compute_l1_hash;
    use crate::SigningChain;

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

    fn multisig_outer_signing_hash(
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

        let hash_a = multisig_outer_signing_hash(
            &signing_payload,
            &SigningChain::Testnet,
            1700000000000,
            None,
        )
        .unwrap();
        let hash_b = multisig_outer_signing_hash(
            &signing_payload,
            &SigningChain::Testnet,
            1700000000000,
            None,
        )
        .unwrap();
        assert_eq!(hash_a, hash_b);
    }
}
