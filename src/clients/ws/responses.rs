//! Typed WebSocket `data` payloads for Hyperliquid streaming feeds.
//!
//! Shapes follow the TypeScript definitions in
//! <https://hyperliquid.gitbook.io/hyperliquid-docs/for-developers/api/websocket/subscriptions>.
//!
//! ## Prices vs other numerics
//!
//! Per [WebSocket subscriptions — data type definitions](https://hyperliquid.gitbook.io/hyperliquid-docs/for-developers/api/websocket/subscriptions):
//! - **Prices** (mid, trade px, book px, OHLC, mark/mid/prev/oracle, limit px, liquidation mark px) use [`Price`].
//! - **Sizes / notionals / fees / balances** still use [`Decimal`] where the API uses string decimals.
//! - Other API **numbers** deserialize as [`Decimal`] (string or JSON number); WS types do not use `f64`.

use std::collections::HashMap;

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Mid / last / mark / oracle / limit / OHLC **price** (per HL WS docs).
pub type Price = Decimal;

// ---------------------------------------------------------------------------
// Common / shared
// ---------------------------------------------------------------------------

mod serde_decimal {
    use super::*;
    use serde::de::Error;

    pub fn de<'de, D>(deserializer: D) -> Result<Decimal, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let v = Value::deserialize(deserializer)?;
        match v {
            Value::String(s) => s.parse::<Decimal>().map_err(D::Error::custom),
            Value::Number(n) => n
                .to_string()
                .parse::<Decimal>()
                .map_err(D::Error::custom),
            _ => Err(D::Error::custom("expected string or number decimal")),
        }
    }

    pub fn de_opt<'de, D>(deserializer: D) -> Result<Option<Decimal>, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let v = Option::<Value>::deserialize(deserializer)?;
        match v {
            None | Some(Value::Null) => Ok(None),
            Some(Value::String(s)) => Ok(Some(s.parse::<Decimal>().map_err(D::Error::custom)?)),
            Some(Value::Number(n)) => Ok(Some(
                n.to_string()
                    .parse::<Decimal>()
                    .map_err(D::Error::custom)?,
            )),
            Some(_) => Err(D::Error::custom("expected string or number decimal")),
        }
    }

    pub fn de_pair<'de, D>(deserializer: D) -> Result<[Decimal; 2], D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let v = Value::deserialize(deserializer)?;
        let Value::Array(arr) = v else {
            return Err(D::Error::custom("expected array[2] of decimals"));
        };
        if arr.len() != 2 {
            return Err(D::Error::custom("expected array[2] of decimals"));
        }
        let a: Decimal = serde_json::from_value(arr[0].clone()).map_err(D::Error::custom)?;
        let b: Decimal = serde_json::from_value(arr[1].clone()).map_err(D::Error::custom)?;
        Ok([a, b])
    }

    /// `AllMids.mids`: coin → mid price (`Record<string, string>` in WS docs).
    pub fn de_mids_prices<'de, D>(deserializer: D) -> Result<HashMap<String, Price>, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let m = HashMap::<String, Value>::deserialize(deserializer)?;
        let mut out = HashMap::with_capacity(m.len());
        for (k, v) in m {
            let d = match v {
                Value::String(s) => s.parse::<Price>().map_err(D::Error::custom)?,
                Value::Number(n) => n
                    .to_string()
                    .parse::<Price>()
                    .map_err(D::Error::custom)?,
                _ => return Err(D::Error::custom("mid price must be string or number")),
            };
            out.insert(k, d);
        }
        Ok(out)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AllMids {
    #[serde(deserialize_with = "serde_decimal::de_mids_prices")]
    pub mids: HashMap<String, Price>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Notification {
    pub notification: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WsTrade {
    pub coin: String,
    pub side: String,
    #[serde(deserialize_with = "serde_decimal::de")]
    pub px: Price,
    #[serde(deserialize_with = "serde_decimal::de")]
    pub sz: Decimal,
    pub hash: String,
    pub time: u64,
    pub tid: u64,
    pub users: [String; 2],
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WsLevel {
    #[serde(deserialize_with = "serde_decimal::de")]
    pub px: Price,
    #[serde(deserialize_with = "serde_decimal::de")]
    pub sz: Decimal,
    pub n: u64,
}

/// L2 book snapshot / update (`l2Book` channel).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WsBook {
    pub coin: String,
    pub levels: [Vec<WsLevel>; 2],
    pub time: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WsBbo {
    pub coin: String,
    pub time: u64,
    pub bbo: [Option<WsLevel>; 2],
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Candle {
    #[serde(rename = "t")]
    pub open_ms: u64,
    #[serde(rename = "T")]
    pub close_ms: u64,
    #[serde(rename = "s")]
    pub coin: String,
    #[serde(rename = "i")]
    pub interval: String,
    #[serde(rename = "o")]
    #[serde(deserialize_with = "serde_decimal::de")]
    pub open: Price,
    #[serde(rename = "c")]
    #[serde(deserialize_with = "serde_decimal::de")]
    pub close: Price,
    #[serde(rename = "h")]
    #[serde(deserialize_with = "serde_decimal::de")]
    pub high: Price,
    #[serde(rename = "l")]
    #[serde(deserialize_with = "serde_decimal::de")]
    pub low: Price,
    #[serde(rename = "v")]
    #[serde(deserialize_with = "serde_decimal::de")]
    pub volume: Decimal,
    #[serde(rename = "n")]
    pub num_trades: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WsBasicOrder {
    pub coin: String,
    pub side: String,
    #[serde(deserialize_with = "serde_decimal::de")]
    pub limit_px: Price,
    #[serde(deserialize_with = "serde_decimal::de")]
    pub sz: Decimal,
    pub oid: u64,
    pub timestamp: u64,
    #[serde(deserialize_with = "serde_decimal::de")]
    pub orig_sz: Decimal,
    pub cloid: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WsOrder {
    pub order: WsBasicOrder,
    pub status: String,
    pub status_timestamp: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FillLiquidation {
    pub liquidated_user: Option<String>,
    #[serde(deserialize_with = "serde_decimal::de")]
    pub mark_px: Price,
    pub method: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WsFill {
    pub coin: String,
    #[serde(deserialize_with = "serde_decimal::de")]
    pub px: Price,
    #[serde(deserialize_with = "serde_decimal::de")]
    pub sz: Decimal,
    pub side: String,
    pub time: u64,
    #[serde(deserialize_with = "serde_decimal::de")]
    pub start_position: Decimal,
    pub dir: String,
    #[serde(deserialize_with = "serde_decimal::de")]
    pub closed_pnl: Decimal,
    pub hash: String,
    pub oid: u64,
    pub crossed: bool,
    #[serde(deserialize_with = "serde_decimal::de")]
    pub fee: Decimal,
    pub tid: u64,
    pub fee_token: String,
    #[serde(default)]
    pub cloid: Option<String>,
    #[serde(default)]
    pub liquidation: Option<FillLiquidation>,
    #[serde(default)]
    #[serde(deserialize_with = "serde_decimal::de_opt")]
    pub builder_fee: Option<Decimal>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WsUserFunding {
    pub time: u64,
    pub coin: String,
    #[serde(deserialize_with = "serde_decimal::de")]
    pub usdc: Decimal,
    #[serde(deserialize_with = "serde_decimal::de")]
    pub szi: Decimal,
    #[serde(deserialize_with = "serde_decimal::de")]
    pub funding_rate: Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WsLiquidation {
    pub lid: u64,
    pub liquidator: String,
    pub liquidated_user: String,
    pub liquidated_ntl_pos: String,
    pub liquidated_account_value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WsNonUserCancel {
    pub coin: String,
    pub oid: u64,
}

/// User events that are not order updates (`userEvents` channel).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum WsUserEvent {
    Fills { fills: Vec<WsFill> },
    Funding { funding: WsUserFunding },
    Liquidation { liquidation: WsLiquidation },
    NonUserCancel {
        #[serde(rename = "nonUserCancel")]
        non_user_cancel: Vec<WsNonUserCancel>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WsUserFills {
    #[serde(default)]
    pub is_snapshot: Option<bool>,
    pub user: String,
    pub fills: Vec<WsFill>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WsUserFundings {
    #[serde(default)]
    pub is_snapshot: Option<bool>,
    pub user: String,
    /// Funding payments (field name per Hyperliquid WS payloads).
    #[serde(default)]
    pub fundings: Vec<WsUserFunding>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WsUserNonFundingLedgerUpdates {
    #[serde(default)]
    pub is_snapshot: Option<bool>,
    pub user: String,
    pub updates: Vec<WsUserNonFundingLedgerUpdate>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WsUserNonFundingLedgerUpdate {
    pub time: u64,
    pub hash: String,
    pub delta: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SharedAssetCtx {
    /// Day notional **volume** (not a price; `dayNtlVlm` in WS docs).
    #[serde(deserialize_with = "serde_decimal::de")]
    pub day_ntl_vlm: Decimal,
    /// Previous day **price**.
    #[serde(deserialize_with = "serde_decimal::de")]
    pub prev_day_px: Price,
    /// Mark **price**.
    #[serde(deserialize_with = "serde_decimal::de")]
    pub mark_px: Price,
    #[serde(default)]
    #[serde(deserialize_with = "serde_decimal::de_opt")]
    pub mid_px: Option<Price>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PerpsAssetCtx {
    #[serde(flatten)]
    pub shared: SharedAssetCtx,
    /// Hourly funding (not a spot/perp mid price).
    #[serde(deserialize_with = "serde_decimal::de")]
    pub funding: Decimal,
    /// Open interest (not a price).
    #[serde(deserialize_with = "serde_decimal::de")]
    pub open_interest: Decimal,
    /// Oracle **price**.
    #[serde(deserialize_with = "serde_decimal::de")]
    pub oracle_px: Price,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SpotAssetCtx {
    #[serde(flatten)]
    pub shared: SharedAssetCtx,
    /// Token supply (not a price).
    #[serde(deserialize_with = "serde_decimal::de")]
    pub circulating_supply: Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WsActivePerpAssetCtx {
    pub coin: String,
    pub ctx: PerpsAssetCtx,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WsActiveSpotAssetCtx {
    pub coin: String,
    pub ctx: SpotAssetCtx,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum WsActiveAssetCtx {
    Perp(WsActivePerpAssetCtx),
    Spot(WsActiveSpotAssetCtx),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WsActiveAssetData {
    pub user: String,
    pub coin: String,
    pub leverage: Value,
    #[serde(deserialize_with = "serde_decimal::de_pair")]
    pub max_trade_szs: [Decimal; 2],
    #[serde(deserialize_with = "serde_decimal::de_pair")]
    pub available_to_trade: [Decimal; 2],
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WsTwapSliceFill {
    pub fill: WsFill,
    pub twap_id: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WsUserTwapSliceFills {
    #[serde(default)]
    pub is_snapshot: Option<bool>,
    pub user: String,
    pub twap_slice_fills: Vec<WsTwapSliceFill>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TwapState {
    pub coin: String,
    pub user: String,
    pub side: String,
    #[serde(deserialize_with = "serde_decimal::de")]
    pub sz: Decimal,
    #[serde(deserialize_with = "serde_decimal::de")]
    pub executed_sz: Decimal,
    #[serde(deserialize_with = "serde_decimal::de")]
    pub executed_ntl: Decimal,
    pub minutes: u32,
    pub reduce_only: bool,
    pub randomize: bool,
    pub timestamp: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TwapStatusInfo {
    pub status: String,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WsTwapHistory {
    pub state: TwapState,
    pub status: TwapStatusInfo,
    pub time: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WsUserTwapHistory {
    #[serde(default)]
    pub is_snapshot: Option<bool>,
    pub user: String,
    pub history: Vec<WsTwapHistory>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserBalance {
    pub coin: String,
    pub token: u64,
    pub hold: String,
    pub total: String,
    pub entry_ntl: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SpotState {
    pub balances: Vec<UserBalance>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WsSpotState {
    pub user: String,
    pub spot_state: SpotState,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WsAllDexsClearinghouseState {
    pub user: String,
    pub clearinghouse_states: HashMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WsAllDexsAssetCtxs {
    pub ctxs: HashMap<String, Vec<PerpsAssetCtx>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TwapStates {
    pub dex: String,
    pub user: String,
    pub states: Vec<(u64, TwapState)>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OpenOrders {
    pub dex: String,
    pub user: String,
    pub orders: Vec<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WebData3 {
    pub user_state: Value,
    pub perp_dex_states: Vec<Value>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ws_trade_roundtrip() {
        let j = serde_json::json!({
            "coin": "SOL",
            "side": "B",
            "px": "100",
            "sz": "1",
            "hash": "0xabc",
            "time": 1,
            "tid": 2,
            "users": ["0xa", "0xb"]
        });
        let t: WsTrade = serde_json::from_value(j).unwrap();
        assert_eq!(t.coin, "SOL");
        assert_eq!(t.px, Decimal::from(100u64));
        assert_eq!(t.sz, Decimal::from(1u64));
    }

    #[test]
    fn ws_user_event_fills_variant() {
        let j = serde_json::json!({ "fills": [] });
        let e: WsUserEvent = serde_json::from_value(j).unwrap();
        assert!(matches!(e, WsUserEvent::Fills { .. }));
    }
}
