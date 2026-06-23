use alloy::primitives::{Address, U256};
use alloy_signer::Signature;
use serde::{
    de::DeserializeOwned,
    ser::{Error, SerializeMap, SerializeSeq, SerializeStruct},
    Deserialize, Deserializer, Serialize, Serializer,
};

use crate::{
    actions::{
        core::{SignedAction, SignedActionKind},
        traits::Action,
        ActionKind,
    },
    SigningChain,
};

/// Body hashed for L1 `connectionId` when `PAYLOAD_KEY == ACTION_TYPE` (Python `msgpack.packb(action)`).
/// The official Python SDK always includes a top-level `"type"` key; derive-only action structs omit it
/// and must not duplicate `type` in their own `Serialize` impl (see `BatchOrder`).
#[derive(Serialize)]
struct L1FlatSigningBody<'a, T: Serialize> {
    #[serde(rename = "type")]
    action_type: &'static str,
    #[serde(flatten)]
    body: &'a T,
}

/// Compute the L1 action hash (MessagePack + metadata)
pub(crate) struct L1ActionWrapper<'a, T: Action + Serialize> {
    pub action: &'a T,
}

impl<'a, T: Action + Serialize> Serialize for L1ActionWrapper<'a, T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        // For nested payloads, we serialize the action directly to avoid
        // serde_json::Value intermediary which produces different msgpack output
        if T::PAYLOAD_KEY != T::ACTION_TYPE {
            // Nested payload: {"type": "X", "payloadKey": {action fields}}
            let mut map = serializer.serialize_map(Some(2))?;
            map.serialize_entry("type", T::ACTION_TYPE)?;
            map.serialize_entry(T::PAYLOAD_KEY, self.action)?;
            return map.end();
        }

        // Flattened: match Python — `action` dict includes `"type"` then payload fields (camelCase).
        L1FlatSigningBody {
            action_type: T::ACTION_TYPE,
            body: self.action,
        }
        .serialize(serializer)
    }
}

struct SigSer<'a>(&'a Signature);

impl<'a> Serialize for SigSer<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serialize_sig(self.0, serializer)
    }
}

fn action_type_from_obj<E: serde::de::Error>(
    obj: &serde_json::Map<String, serde_json::Value>,
) -> Result<&str, E> {
    obj.get("type")
        .and_then(|v| v.as_str())
        .ok_or_else(|| E::custom("missing action type"))
}

fn extract_action_payload_for<T: Action, E: serde::de::Error>(
    obj: &serde_json::Map<String, serde_json::Value>,
) -> Result<serde_json::Value, E> {
    let payload = if T::PAYLOAD_KEY == T::ACTION_TYPE {
        let mut payload = obj.clone();
        payload.remove("type");
        if T::is_user_signed()
            && T::uses_time()
            && payload.contains_key("time")
            && !payload.contains_key("nonce")
        {
            if let Some(time_value) = payload.get("time").cloned() {
                payload.insert("nonce".to_string(), time_value);
            }
        }
        serde_json::Value::Object(payload)
    } else {
        let payload = obj
            .get(T::PAYLOAD_KEY)
            .ok_or_else(|| E::custom("missing action payload"))?;
        if T::is_user_signed() {
            let mut payload = payload
                .as_object()
                .cloned()
                .ok_or_else(|| E::custom("action payload must be object"))?;
            if T::uses_time() && payload.contains_key("time") && !payload.contains_key("nonce") {
                if let Some(time_value) = payload.get("time").cloned() {
                    payload.insert("nonce".to_string(), time_value);
                }
            }
            serde_json::Value::Object(payload)
        } else {
            payload.clone()
        }
    };
    Ok(payload)
}

/// Parse an action from a JSON object, extracting and normalizing the payload.
///
/// This is `pub(crate)` so the `dispatch_action_kind` macro-generated code in
/// `mod.rs` can call it.
pub(crate) fn parse_action_from_obj<T, E>(
    obj: &serde_json::Map<String, serde_json::Value>,
) -> Result<T, E>
where
    T: Action + DeserializeOwned,
    E: serde::de::Error,
{
    let payload = extract_action_payload_for::<T, E>(obj)?;
    serde_json::from_value(payload).map_err(E::custom)
}

fn deserialize_action_kind<'de, D>(deserializer: D) -> Result<ActionKind, D::Error>
where
    D: Deserializer<'de>,
{
    let value = serde_json::Value::deserialize(deserializer)?;
    let obj = value
        .as_object()
        .ok_or_else(|| serde::de::Error::custom("action must be object"))?;

    // Calls the macro-generated dispatcher in mod.rs
    super::dispatch_action_kind::<D::Error>(obj)
}

fn deserialize_action<'de, D, T>(deserializer: D) -> Result<T, D::Error>
where
    D: Deserializer<'de>,
    T: Action + DeserializeOwned,
{
    let value = serde_json::Value::deserialize(deserializer)?;
    let obj = value
        .as_object()
        .ok_or_else(|| serde::de::Error::custom("action must be object"))?;

    let action_type = action_type_from_obj::<D::Error>(obj)?;
    if action_type != T::ACTION_TYPE {
        return Err(serde::de::Error::custom(format!(
            "invalid action type: {}",
            action_type
        )));
    }

    let action_payload = extract_action_payload_for::<T, D::Error>(obj)?;
    serde_json::from_value(action_payload).map_err(serde::de::Error::custom)
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(bound(deserialize = "T: Action + DeserializeOwned"))]
struct SignedActionHelper<T: Action> {
    #[serde(deserialize_with = "deserialize_action")]
    action: T,
    nonce: u64,
    #[serde(deserialize_with = "deserialize_sig")]
    signature: Signature,
    vault_address: Option<Address>,
    expires_after: Option<u64>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SignedActionKindHelper {
    #[serde(deserialize_with = "deserialize_action_kind")]
    action: ActionKind,
    nonce: u64,
    #[serde(deserialize_with = "deserialize_sig")]
    signature: Signature,
    vault_address: Option<Address>,
    expires_after: Option<u64>,
}

impl<T: Action + Serialize> Serialize for SignedAction<T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let action_value =
            build_action_value(&self.action, self.signing_chain.as_ref()).map_err(Error::custom)?;
        let mut state = serializer.serialize_struct("SignedAction", 5)?;
        state.serialize_field("action", &action_value)?;
        state.serialize_field("nonce", &self.nonce)?;
        state.serialize_field("signature", &SigSer(&self.signature))?;
        if let Some(vault_address) = &self.vault_address {
            state.serialize_field("vaultAddress", vault_address)?;
        }
        if let Some(expires_after) = &self.expires_after {
            state.serialize_field("expiresAfter", expires_after)?;
        }
        state.end()
    }
}

impl<'de, T: Action + DeserializeOwned> Deserialize<'de> for SignedAction<T> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let helper = SignedActionHelper::deserialize(deserializer)?;
        Ok(SignedAction {
            action: helper.action,
            nonce: helper.nonce,
            signature: helper.signature,
            vault_address: helper.vault_address,
            expires_after: helper.expires_after,
            signing_chain: None,
        })
    }
}

impl<'de> Deserialize<'de> for SignedActionKind {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let helper = SignedActionKindHelper::deserialize(deserializer)?;
        Ok(SignedActionKind {
            action: helper.action,
            nonce: helper.nonce,
            signature: helper.signature,
            vault_address: helper.vault_address,
            expires_after: helper.expires_after,
            signing_chain: None,
        })
    }
}

pub(crate) fn serialize_sig<S>(sig: &Signature, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let mut state = serializer.serialize_struct("Signature", 3)?;
    // Match Python SDK `to_hex`: unpadded lowercase hex (msgpack hash is sensitive to this).
    state.serialize_field("r", &format!("0x{:x}", sig.r()))?;
    state.serialize_field("s", &format!("0x{:x}", sig.s()))?;
    state.serialize_field("v", &(27 + sig.v() as u64))?;
    state.end()
}

fn deserialize_sig<'de, D>(deserializer: D) -> Result<Signature, D::Error>
where
    D: Deserializer<'de>,
{
    let value = serde_json::Value::deserialize(deserializer)?;
    let obj = value
        .as_object()
        .ok_or_else(|| serde::de::Error::custom("signature must be object"))?;

    let r = obj
        .get("r")
        .and_then(|v| v.as_str())
        .ok_or_else(|| serde::de::Error::custom("missing signature.r"))?;
    let s = obj
        .get("s")
        .and_then(|v| v.as_str())
        .ok_or_else(|| serde::de::Error::custom("missing signature.s"))?;

    let v_value = obj
        .get("v")
        .ok_or_else(|| serde::de::Error::custom("missing signature.v"))?;
    let v = if let Some(v) = v_value.as_u64() {
        v
    } else if let Some(v) = v_value.as_str() {
        v.parse::<u64>()
            .map_err(|e| serde::de::Error::custom(e.to_string()))?
    } else {
        return Err(serde::de::Error::custom("invalid signature.v"));
    };

    let r = r.strip_prefix("0x").unwrap_or(r);
    let s = s.strip_prefix("0x").unwrap_or(s);

    let r = U256::from_str_radix(r, 16).map_err(|e| serde::de::Error::custom(e.to_string()))?;
    let s = U256::from_str_radix(s, 16).map_err(|e| serde::de::Error::custom(e.to_string()))?;
    let v = v
        .checked_sub(27)
        .ok_or_else(|| serde::de::Error::custom("invalid v value"))?;

    Ok(Signature::new(r, s, v != 0))
}

pub(crate) fn ser_lowercase<S>(address: &Address, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_str(&address.to_string().to_lowercase())
}

/// Msgpack key order follows insertion order. `serde_json::Value` serialization through
/// `rmp_serde` sorts object keys alphabetically, so wire payloads must use this wrapper.
#[derive(Debug, Clone)]
pub(crate) struct WireValue(pub serde_json::Value);

impl Serialize for WireValue {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serialize_wire_value(&self.0, serializer)
    }
}

fn serialize_wire_value<S>(value: &serde_json::Value, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    match value {
        serde_json::Value::Object(map) => {
            let mut state = serializer.serialize_map(Some(map.len()))?;
            for (key, value) in map {
                state.serialize_entry(key, &WireValue(value.clone()))?;
            }
            state.end()
        }
        serde_json::Value::Array(values) => {
            let mut seq = serializer.serialize_seq(Some(values.len()))?;
            for value in values {
                seq.serialize_element(&WireValue(value.clone()))?;
            }
            seq.end()
        }
        other => other.serialize(serializer),
    }
}

/// Multisig inner-action wire order (type and chain fields first).
/// Reference: hyperliquid-python-sdk `examples/multi_sig_usd_send.py` and issue #177.
fn multisig_inner_wire_field_order(action_type: &str) -> &'static [&'static str] {
    match action_type {
        "spotSend" => &[
            "type",
            "signatureChainId",
            "hyperliquidChain",
            "destination",
            "token",
            "amount",
            "time",
        ],
        "usdSend" | "withdraw3" => &[
            "type",
            "signatureChainId",
            "hyperliquidChain",
            "destination",
            "amount",
            "time",
        ],
        "sendAsset" => &[
            "type",
            "signatureChainId",
            "hyperliquidChain",
            "destination",
            "sourceDex",
            "destinationDex",
            "token",
            "amount",
            "fromSubAccount",
            "nonce",
        ],
        "usdClassTransfer" => &[
            "type",
            "signatureChainId",
            "hyperliquidChain",
            "amount",
            "toPerp",
            "nonce",
        ],
        "tokenDelegate" => &[
            "type",
            "signatureChainId",
            "hyperliquidChain",
            "validator",
            "wei",
            "isUndelegate",
            "nonce",
        ],
        "convertToMultiSigUser" => &[
            "type",
            "signatureChainId",
            "hyperliquidChain",
            "signers",
            "nonce",
        ],
        _ => &[],
    }
}

/// Python SDK insertion order for regular signed-action wire payloads.
fn user_signed_wire_field_order(action_type: &str) -> &'static [&'static str] {
    match action_type {
        "spotSend" => &[
            "destination",
            "amount",
            "token",
            "time",
            "type",
            "signatureChainId",
            "hyperliquidChain",
        ],
        "usdSend" | "withdraw3" => &[
            "destination",
            "amount",
            "time",
            "type",
            "signatureChainId",
            "hyperliquidChain",
        ],
        "tokenDelegate" => &[
            "validator",
            "wei",
            "isUndelegate",
            "nonce",
            "type",
            "signatureChainId",
            "hyperliquidChain",
        ],
        "usdClassTransfer" => &[
            "type",
            "amount",
            "toPerp",
            "nonce",
            "signatureChainId",
            "hyperliquidChain",
        ],
        "sendAsset" => &[
            "destination",
            "sourceDex",
            "destinationDex",
            "token",
            "amount",
            "fromSubAccount",
            "nonce",
            "type",
            "signatureChainId",
            "hyperliquidChain",
        ],
        "convertToMultiSigUser" => &[
            "type",
            "signers",
            "nonce",
            "signatureChainId",
            "hyperliquidChain",
        ],
        "userSetAbstraction" => &[
            "user",
            "abstraction",
            "nonce",
            "type",
            "signatureChainId",
            "hyperliquidChain",
        ],
        "userDexAbstraction" => &[
            "user",
            "enabled",
            "nonce",
            "type",
            "signatureChainId",
            "hyperliquidChain",
        ],
        _ => &[],
    }
}

fn build_user_signed_wire_object_with_order<T: Action + Serialize>(
    action: &T,
    signing_chain: &SigningChain,
    ordered_keys: &'static [&'static str],
) -> Result<serde_json::Map<String, serde_json::Value>, String> {
    let payload_value = serde_json::to_value(action).map_err(|e| e.to_string())?;
    let mut payload = payload_value
        .as_object()
        .cloned()
        .ok_or_else(|| "action payload must be object".to_string())?;

    if T::uses_time() && payload.contains_key("nonce") && !payload.contains_key("time") {
        if let Some(nonce_value) = payload.remove("nonce") {
            payload.insert("time".to_string(), nonce_value);
        }
    }

    let signature_chain_id = format!("0x{:x}", signing_chain.get_signature_chain_id());
    let hyperliquid_chain = signing_chain.get_hyperliquid_chain();
    let mut action_obj = serde_json::Map::new();
    if ordered_keys.is_empty() {
        for (key, value) in payload {
            action_obj.insert(key, value);
        }
        action_obj.insert(
            "type".to_string(),
            serde_json::Value::String(T::ACTION_TYPE.to_string()),
        );
        action_obj.insert(
            "signatureChainId".to_string(),
            serde_json::Value::String(signature_chain_id),
        );
        action_obj.insert(
            "hyperliquidChain".to_string(),
            serde_json::Value::String(hyperliquid_chain),
        );
        return Ok(action_obj);
    }

    for key in ordered_keys {
        match *key {
            "type" => {
                action_obj.insert(
                    "type".to_string(),
                    serde_json::Value::String(T::ACTION_TYPE.to_string()),
                );
            }
            "signatureChainId" => {
                action_obj.insert(
                    "signatureChainId".to_string(),
                    serde_json::Value::String(signature_chain_id.clone()),
                );
            }
            "hyperliquidChain" => {
                action_obj.insert(
                    "hyperliquidChain".to_string(),
                    serde_json::Value::String(hyperliquid_chain.clone()),
                );
            }
            field => {
                if let Some(value) = payload.get(field) {
                    action_obj.insert(field.to_string(), value.clone());
                }
            }
        }
    }

    Ok(action_obj)
}

fn build_user_signed_wire_object<T: Action + Serialize>(
    action: &T,
    signing_chain: &SigningChain,
) -> Result<serde_json::Map<String, serde_json::Value>, String> {
    build_user_signed_wire_object_with_order(
        action,
        signing_chain,
        user_signed_wire_field_order(T::ACTION_TYPE),
    )
}

/// Wire-format inner action for multisig payloads (distinct field order from regular actions).
pub(crate) fn build_multisig_inner_action_value<T: Action + Serialize>(
    action: &T,
    signing_chain: &SigningChain,
) -> Result<serde_json::Value, String> {
    Ok(serde_json::Value::Object(build_user_signed_wire_object_with_order(
        action,
        signing_chain,
        multisig_inner_wire_field_order(T::ACTION_TYPE),
    )?))
}

impl<T: Action + DeserializeOwned> SignedAction<T> {
    /// Deserialize from the exchange API format
    pub fn from_json(json: &str) -> Result<Self, crate::Error> {
        serde_json::from_str(json).map_err(|e| crate::Error::JsonParse(e.to_string()))
    }
}

pub(crate) fn build_action_value<T: Action + Serialize>(
    action: &T,
    signing_chain: Option<&SigningChain>,
) -> Result<serde_json::Value, String> {
    let payload_value = serde_json::to_value(action).map_err(|e| e.to_string())?;
    let mut payload = payload_value;

    if T::is_user_signed() {
        let signing_chain = signing_chain
            .ok_or_else(|| "signing_chain must be set for user-signed actions".to_string())?;
        if T::PAYLOAD_KEY == T::ACTION_TYPE {
            return Ok(serde_json::Value::Object(build_user_signed_wire_object(
                action,
                signing_chain,
            )?));
        }

        let obj = payload
            .as_object_mut()
            .ok_or_else(|| "action payload must be object".to_string())?;
        if T::uses_time() && obj.contains_key("nonce") && !obj.contains_key("time") {
            if let Some(nonce_value) = obj.remove("nonce") {
                obj.insert("time".to_string(), nonce_value);
            }
        }
        obj.insert(
            "signatureChainId".to_string(),
            serde_json::Value::String(format!("0x{:x}", signing_chain.get_signature_chain_id())),
        );
        obj.insert(
            "hyperliquidChain".to_string(),
            serde_json::Value::String(signing_chain.get_hyperliquid_chain()),
        );
    }

    let mut action_obj = serde_json::Map::new();
    action_obj.insert(
        "type".to_string(),
        serde_json::Value::String(T::ACTION_TYPE.to_string()),
    );

    if T::PAYLOAD_KEY == T::ACTION_TYPE {
        let payload = payload
            .as_object()
            .cloned()
            .ok_or_else(|| "action payload must be object".to_string())?;
        for (key, value) in payload {
            action_obj.insert(key, value);
        }
    } else {
        action_obj.insert(T::PAYLOAD_KEY.to_string(), payload);
    }

    Ok(serde_json::Value::Object(action_obj))
}

#[cfg(test)]
mod tests {
    use alloy::primitives::{Address, U256};
    use alloy_signer::Signature;
    use rust_decimal_macros::dec;
    use serde_json::json;

    use super::*;
    use crate::actions::{
        ActionKind, BatchCancel, BatchModify, CancelByCloid, CancelWire, LimitOrderType, OrderType,
        OrderWire, PreparedAction, SetOpenInterestCaps, SignedActionKind, Tif, ToggleBigBlocks,
        UsdSend,
    };
    use crate::SigningChain;

    /// Hyperliquid Python SDK (`bulk_modify_orders_new`) builds:
    /// `{ "type": "batchModify", "modifies": [ { "oid", "order" }, ... ] }`.
    /// A wrong `payload_key` double-nests `modifies` and the API returns HTTP 422.
    #[test]
    fn batch_modify_action_shape_matches_python_sdk() {
        let order = OrderWire {
            asset: 4,
            is_buy: true,
            limit_px: dec!(100.0),
            size: dec!(0.01),
            reduce_only: false,
            order_type: OrderType::Limit(LimitOrderType { tif: Tif::Gtc }),
            client_order_id: None,
        };
        let action = BatchModify::single(91_490_942, order);
        let v = build_action_value(&action, None).expect("build_action_value");
        let obj = v.as_object().expect("action object");
        assert_eq!(
            obj.get("type").and_then(|x| x.as_str()),
            Some("batchModify")
        );
        let modifies = obj.get("modifies").expect("top-level modifies");
        assert!(
            modifies.is_array(),
            "expected `modifies` to be a JSON array (flat shape); got {modifies:?}"
        );
        let arr = modifies.as_array().unwrap();
        assert_eq!(arr.len(), 1);
        assert_eq!(arr[0].get("oid"), Some(&json!(91_490_942u64)));
        assert!(arr[0].get("order").is_some());
    }

    #[test]
    fn batch_cancel_action_shape_matches_python_sdk() {
        let action = BatchCancel::new(vec![CancelWire {
            a: 110_000,
            o: 12_345,
        }]);
        let v = build_action_value(&action, None).expect("build_action_value");
        let obj = v.as_object().expect("action object");
        assert_eq!(obj.get("type").and_then(|x| x.as_str()), Some("cancel"));
        let cancels = obj.get("cancels").expect("top-level cancels");
        assert!(
            cancels.is_array(),
            "expected `cancels` to be a JSON array; got {cancels:?}"
        );
        let arr = cancels.as_array().unwrap();
        assert_eq!(arr.len(), 1);
        assert_eq!(arr[0].get("a"), Some(&json!(110_000u32)));
        assert_eq!(arr[0].get("o"), Some(&json!(12_345u64)));
    }

    #[test]
    fn cancel_by_cloid_action_shape_matches_python_sdk() {
        let action = CancelByCloid::single(110_000, "0x0123456789abcdef0123456789abcdef");
        let v = build_action_value(&action, None).expect("build_action_value");
        let obj = v.as_object().expect("action object");
        assert_eq!(
            obj.get("type").and_then(|x| x.as_str()),
            Some("cancelByCloid")
        );
        let cancels = obj.get("cancels").expect("top-level cancels");
        assert!(cancels.is_array(), "expected array; got {cancels:?}");
    }

    #[test]
    fn test_enable_big_blocks_serialization() {
        let action = ToggleBigBlocks::enable();
        let signing_chain = SigningChain::Mainnet;

        let prepared = PreparedAction::new(action, &signing_chain, None, None).unwrap();

        // Create a dummy signature for testing
        let sig = Signature::new(U256::from(1), U256::from(2), false);
        let signed = prepared.with_signature(sig);

        let json = serde_json::to_string_pretty(&signed).unwrap();
        println!("{}", json);

        // Verify structure
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert!(parsed.get("action").is_some());
        assert!(parsed.get("nonce").is_some());
        assert!(parsed.get("signature").is_some());

        let action_obj = parsed.get("action").unwrap();
        assert_eq!(action_obj.get("type").unwrap(), "evmUserModify");
        assert!(action_obj.get("usingBigBlocks").is_some());
    }

    #[test]
    fn test_usd_send_serialization() {
        let action = UsdSend::new(Address::ZERO, dec!(100.0));
        let signing_chain = SigningChain::Testnet;

        let prepared = PreparedAction::new(action, &signing_chain, None, None).unwrap();

        let sig = Signature::new(U256::from(1), U256::from(2), false);
        let signed = prepared.with_signature(sig);

        let json = serde_json::to_string_pretty(&signed).unwrap();
        println!("{}", json);

        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        let action_obj = parsed.get("action").unwrap();
        assert_eq!(action_obj.get("type").unwrap(), "usdSend");
        assert_eq!(action_obj.get("hyperliquidChain").unwrap(), "Testnet");
        assert_eq!(action_obj.get("signatureChainId").unwrap(), "0x66eee");
        assert!(action_obj.get("destination").is_some());
        assert!(action_obj.get("amount").is_some());
        assert!(action_obj.get("time").is_some());
    }

    #[test]
    fn test_perp_deploy_v2_set_open_interest_caps_serialization_matches_docs() {
        let action = SetOpenInterestCaps {
            caps: vec![("BTC".to_string(), 1_000_000), ("ETH".to_string(), 500_000)],
            nonce: None,
        };
        let signing_chain = SigningChain::Testnet;
        let prepared = PreparedAction::new(action, &signing_chain, None, None).unwrap();
        let sig = Signature::new(U256::from(3), U256::from(4), true);
        let signed = prepared.with_signature(sig);

        let parsed = serde_json::to_value(&signed).unwrap();
        let action_obj = parsed.get("action").unwrap();
        assert_eq!(
            action_obj,
            &json!({
                "type": "perpDeploy",
                "setOpenInterestCaps": [
                    ["BTC", 1_000_000],
                    ["ETH", 500_000]
                ]
            })
        );
    }

    #[test]
    fn test_extract_action_kind_preserves_values() {
        // Test UsdSend
        let dest = Address::repeat_byte(0x42);
        let amount = dec!(123.456);
        let nonce = 9876543210u64;
        let usd_send = UsdSend {
            destination: dest,
            amount,
            nonce: Some(nonce),
        };

        let signing_chain = SigningChain::Testnet;
        let prepared = PreparedAction::new(usd_send, &signing_chain, None, None).unwrap();
        let sig = Signature::new(U256::from(1), U256::from(2), false);
        let signed = prepared.with_signature(sig);

        // Extract ActionKind and verify values are preserved
        let kind = signed.extract_action_kind();
        match kind {
            ActionKind::UsdSend(extracted) => {
                assert_eq!(extracted.destination, dest);
                assert_eq!(extracted.amount, amount);
                assert_eq!(extracted.nonce, Some(nonce));
            }
            _ => panic!("Expected ActionKind::UsdSend"),
        }

        // Test EnableBigBlocks
        let enable_blocks = ToggleBigBlocks::enable();
        let prepared = PreparedAction::new(enable_blocks, &signing_chain, None, None).unwrap();
        let sig = Signature::new(U256::from(3), U256::from(4), true);
        let signed = prepared.with_signature(sig);

        let kind = signed.extract_action_kind();
        match kind {
            ActionKind::ToggleBigBlocks(extracted) => {
                assert!(extracted.using_big_blocks);
            }
            _ => panic!("Expected ActionKind::EnableBigBlocks"),
        }

        // Also test disable variant
        let disable_blocks = ToggleBigBlocks::disable();
        let prepared = PreparedAction::new(disable_blocks, &signing_chain, None, None).unwrap();
        let sig = Signature::new(U256::from(5), U256::from(6), false);
        let signed = prepared.with_signature(sig);

        let kind = signed.extract_action_kind();
        match kind {
            ActionKind::ToggleBigBlocks(extracted) => {
                assert!(!extracted.using_big_blocks);
            }
            _ => panic!("Expected ActionKind::EnableBigBlocks"),
        }
    }

    #[test]
    fn test_signed_action_serialization_roundtrip() {
        // Test UsdSend roundtrip
        let dest = Address::repeat_byte(0xAB);
        let amount = dec!(999.123);
        let nonce = 1234567890u64;
        let usd_send = UsdSend {
            destination: dest,
            amount,
            nonce: Some(nonce),
        };

        let signing_chain = SigningChain::Testnet;
        let prepared = PreparedAction::new(usd_send, &signing_chain, None, None).unwrap();
        let nonce = prepared.nonce;

        let r = U256::from_str_radix("abcd1234", 16).unwrap();
        let s = U256::from_str_radix("5678ef90", 16).unwrap();
        let sig = Signature::new(r, s, true);
        let signed = prepared.with_signature(sig);

        // Serialize to JSON
        let json = serde_json::to_string(&signed).unwrap();
        println!("Serialized UsdSend: {}", json);

        // Deserialize back
        let deserialized: SignedAction<UsdSend> = SignedAction::from_json(&json).unwrap();

        // Verify all fields match
        assert_eq!(deserialized.action.destination, dest);
        assert_eq!(deserialized.action.amount, amount);
        assert_eq!(deserialized.action.nonce, Some(nonce));
        assert_eq!(deserialized.nonce, nonce);
        assert_eq!(deserialized.signature.r(), signed.signature.r());
        assert_eq!(deserialized.signature.s(), signed.signature.s());
        assert_eq!(deserialized.signature.v(), signed.signature.v());
        assert_eq!(deserialized.vault_address, None);
        assert_eq!(deserialized.expires_after, None);

        // Test EnableBigBlocks roundtrip
        let enable_blocks = ToggleBigBlocks::enable();
        let prepared = PreparedAction::new(enable_blocks, &signing_chain, None, None).unwrap();
        let nonce = prepared.nonce;

        let sig = Signature::new(U256::from(111), U256::from(222), false);
        let signed = prepared.with_signature(sig);

        let json = serde_json::to_string(&signed).unwrap();
        println!("Serialized EnableBigBlocks: {}", json);

        let deserialized: SignedAction<ToggleBigBlocks> = SignedAction::from_json(&json).unwrap();

        assert!(deserialized.action.using_big_blocks);
        assert_eq!(deserialized.nonce, nonce);
        assert_eq!(deserialized.signature.r(), signed.signature.r());
        assert_eq!(deserialized.signature.s(), signed.signature.s());

        // Test with optional fields (vault_address)
        let vault = Address::repeat_byte(0x99);
        let usd_send = UsdSend {
            destination: dest,
            amount: dec!(50.0),
            nonce: Some(111222333),
        };
        let prepared = PreparedAction::new(usd_send, &signing_chain, Some(vault), None).unwrap();
        let sig = Signature::new(U256::from(333), U256::from(444), true);
        let signed = prepared.with_signature(sig);

        let json = serde_json::to_string(&signed).unwrap();
        println!("Serialized with vault: {}", json);

        let deserialized: SignedAction<UsdSend> = SignedAction::from_json(&json).unwrap();
        assert_eq!(deserialized.vault_address, Some(vault));
    }

    #[test]
    fn test_dec_ser() {
        assert_eq!("10000.5".to_string(), dec!(10_000.5).to_string());
    }

    /// Reference: `hyperliquid.utils.signing.action_hash` uses `msgpack.packb(action)` on the full dict
    /// including `"type"`. Generated via CPython `msgpack.packb` on the official SDK action shape.
    #[test]
    fn l1_flat_update_leverage_msgpack_matches_python_reference() {
        use crate::actions::l1_actions::UpdateLeverage;

        let action = UpdateLeverage::isolated(15, 3);
        let wrapper = super::L1ActionWrapper { action: &action };
        let got = rmp_serde::to_vec_named(&wrapper).unwrap();
        const EXPECTED: &[u8] = &[
            0x84, 0xa4, 0x74, 0x79, 0x70, 0x65, 0xae, 0x75, 0x70, 0x64, 0x61, 0x74, 0x65, 0x4c,
            0x65, 0x76, 0x65, 0x72, 0x61, 0x67, 0x65, 0xa5, 0x61, 0x73, 0x73, 0x65, 0x74, 0x0f,
            0xa7, 0x69, 0x73, 0x43, 0x72, 0x6f, 0x73, 0x73, 0xc2, 0xa8, 0x6c, 0x65, 0x76, 0x65,
            0x72, 0x61, 0x67, 0x65, 0x03,
        ];
        assert_eq!(got.as_slice(), EXPECTED);
    }

    /// Hardening: wider integers must pick the same minimal msgpack width as the
    /// Python SDK. asset=300 -> uint16 (cd 01 2c), leverage=200 -> uint8 (cc c8).
    /// The reference above only uses single-byte fixints, so this guards the
    /// uint8/uint16 paths where rmp-serde and Python could otherwise disagree.
    /// Generated via CPython `msgpack.packb` on the official SDK action shape.
    #[test]
    fn l1_flat_update_leverage_wide_ints_msgpack_matches_python_reference() {
        use crate::actions::l1_actions::UpdateLeverage;

        let action = UpdateLeverage::cross(300, 200);
        let wrapper = super::L1ActionWrapper { action: &action };
        let got = rmp_serde::to_vec_named(&wrapper).unwrap();
        const EXPECTED: &[u8] = &[
            0x84, 0xa4, 0x74, 0x79, 0x70, 0x65, 0xae, 0x75, 0x70, 0x64, 0x61, 0x74, 0x65, 0x4c, 0x65,
            0x76, 0x65, 0x72, 0x61, 0x67, 0x65, 0xa5, 0x61, 0x73, 0x73, 0x65, 0x74, 0xcd, 0x01, 0x2c,
            0xa7, 0x69, 0x73, 0x43, 0x72, 0x6f, 0x73, 0x73, 0xc3, 0xa8, 0x6c, 0x65, 0x76, 0x65, 0x72,
            0x61, 0x67, 0x65, 0xcc, 0xc8,
        ];
        assert_eq!(got.as_slice(), EXPECTED);
    }

    #[test]
    fn test_signed_action_kind_deserializes_user_signed_action() {
        let dest = Address::repeat_byte(0xAB);
        let amount = dec!(1.23);
        let signing_chain = SigningChain::Testnet;

        let prepared =
            PreparedAction::new(UsdSend::new(dest, amount), &signing_chain, None, None).unwrap();
        let nonce = prepared.nonce;

        let sig = Signature::new(U256::from(1), U256::from(2), true);
        let signed = prepared.with_signature(sig);

        let json = serde_json::to_string(&signed).unwrap();
        let deserialized = SignedActionKind::from_json(&json).unwrap();

        assert_eq!(deserialized.nonce, nonce);
        assert_eq!(deserialized.signature.r(), signed.signature.r());
        assert_eq!(deserialized.signature.s(), signed.signature.s());
        assert_eq!(deserialized.signature.v(), signed.signature.v());

        match deserialized.action {
            ActionKind::UsdSend(action) => {
                assert_eq!(action.destination, dest);
                assert_eq!(action.amount, amount);
                assert_eq!(action.nonce, Some(nonce));
            }
            other => panic!("expected ActionKind::UsdSend, got: {other:?}"),
        }
    }

    #[test]
    fn test_signed_action_kind_deserializes_perp_deploy_by_payload_key() {
        let action = SetOpenInterestCaps {
            caps: vec![("BTC".to_string(), 1_000_000), ("ETH".to_string(), 500_000)],
            nonce: None,
        };
        let signing_chain = SigningChain::Testnet;

        let prepared = PreparedAction::new(action, &signing_chain, None, None).unwrap();
        let sig = Signature::new(U256::from(3), U256::from(4), true);
        let signed = prepared.with_signature(sig);

        let json = serde_json::to_string(&signed).unwrap();
        let deserialized = SignedActionKind::from_json(&json).unwrap();

        match deserialized.action {
            ActionKind::SetOpenInterestCaps(action) => {
                assert_eq!(
                    action.caps,
                    vec![("BTC".to_string(), 1_000_000), ("ETH".to_string(), 500_000)]
                );
            }
            other => panic!("expected ActionKind::SetOpenInterestCaps, got: {other:?}"),
        }
    }

    #[test]
    fn test_signed_action_kind_unknown_preserves_raw_action() {
        let envelope = json!({
            "action": {
                "type": "someFutureAction",
                "foo": 1,
                "bar": { "baz": true }
            },
            "nonce": 123,
            "signature": {
                "r": "0x01",
                "s": "0x02",
                "v": 27
            }
        });

        let json = serde_json::to_string(&envelope).unwrap();
        let deserialized = SignedActionKind::from_json(&json).unwrap();

        match deserialized.action {
            ActionKind::Unknown { action_type, raw } => {
                assert_eq!(action_type, "someFutureAction");
                assert_eq!(raw, envelope.get("action").cloned().unwrap());
            }
            other => panic!("expected ActionKind::Unknown, got: {other:?}"),
        }
    }

    #[test]
    fn spot_send_regular_wire_order_matches_python_sdk() {
        use crate::SpotTransfer;
        use alloy::primitives::address;

        let mut action = SpotTransfer::new(
            address!("0x73c4c9fb0113f19d4d015e4ebb06ceaabc2b3cea"),
            "HYPE:0x7317beb7cceed72ef0b346074cc8e7ab",
            dec!(0.01),
        );
        action.nonce = Some(1_781_248_501_639);
        let v = build_action_value(&action, Some(&SigningChain::Testnet)).unwrap();
        let keys: Vec<_> = v.as_object().unwrap().keys().cloned().collect();
        assert_eq!(
            keys,
            vec![
                "destination".to_string(),
                "amount".to_string(),
                "token".to_string(),
                "time".to_string(),
                "type".to_string(),
                "signatureChainId".to_string(),
                "hyperliquidChain".to_string(),
            ]
        );
    }

    #[test]
    fn spot_send_multisig_inner_wire_order_matches_python_examples() {
        use crate::SpotTransfer;
        use alloy::primitives::address;

        let mut action = SpotTransfer::new(
            address!("0x73c4c9fb0113f19d4d015e4ebb06ceaabc2b3cea"),
            "HYPE:0x7317beb7cceed72ef0b346074cc8e7ab",
            dec!(0.01),
        );
        action.nonce = Some(1_781_248_501_639);
        let v = build_multisig_inner_action_value(&action, &SigningChain::Testnet).unwrap();
        let keys: Vec<_> = v.as_object().unwrap().keys().cloned().collect();
        assert_eq!(
            keys,
            vec![
                "type".to_string(),
                "signatureChainId".to_string(),
                "hyperliquidChain".to_string(),
                "destination".to_string(),
                "token".to_string(),
                "amount".to_string(),
                "time".to_string(),
            ]
        );
    }
}
