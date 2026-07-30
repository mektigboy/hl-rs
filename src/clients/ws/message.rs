//! WebSocket message types for the Hyperliquid API.
//!
//! Incoming frames are parsed into [`WsMessage`] using channel names and payload shapes from
//! <https://hyperliquid.gitbook.io/hyperliquid-docs/for-developers/api/websocket/subscriptions>.

use serde::{Deserialize, Deserializer, Serialize};
use serde_json::Value;

use super::responses::*;
use super::Subscription;

/// Outgoing request (subscribe, unsubscribe, ping).
#[derive(Debug, Clone, Serialize)]
pub(crate) struct WsRequest {
    method: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    subscription: Option<Subscription>,
}

impl WsRequest {
    pub fn subscribe(sub: Subscription) -> Self {
        Self {
            method: "subscribe",
            subscription: Some(sub),
        }
    }

    pub fn unsubscribe(sub: Subscription) -> Self {
        Self {
            method: "unsubscribe",
            subscription: Some(sub),
        }
    }

    pub fn ping() -> Self {
        Self {
            method: "ping",
            subscription: None,
        }
    }
}

/// Incoming message from the server (typed by `channel`).
#[derive(Debug, Clone)]
pub enum WsMessage {
    /// Acknowledgement after subscribe; `data` echoes the subscription.
    SubscriptionResponse { data: Subscription },
    /// Response to [`super::WsClient::ping`].
    Pong,
    AllMids {
        data: AllMids,
        is_snapshot: Option<bool>,
    },
    Notification {
        data: Notification,
        is_snapshot: Option<bool>,
    },
    WebData3 {
        data: WebData3,
        is_snapshot: Option<bool>,
    },
    TwapStates {
        data: TwapStates,
        is_snapshot: Option<bool>,
    },
    /// Clearinghouse / margin state (same overall shape as the info `clearinghouseState` snapshot).
    ClearinghouseState {
        data: Value,
        is_snapshot: Option<bool>,
    },
    OpenOrders {
        data: OpenOrders,
        is_snapshot: Option<bool>,
    },
    Candle {
        data: Candle,
        is_snapshot: Option<bool>,
    },
    L2Book {
        data: WsBook,
        is_snapshot: Option<bool>,
    },
    Trades {
        data: Vec<WsTrade>,
        is_snapshot: Option<bool>,
    },
    OrderUpdates {
        data: Vec<WsOrder>,
        is_snapshot: Option<bool>,
    },
    UserEvents {
        data: WsUserEvent,
        is_snapshot: Option<bool>,
    },
    UserFills {
        data: WsUserFills,
        is_snapshot: Option<bool>,
    },
    UserFundings {
        data: WsUserFundings,
        is_snapshot: Option<bool>,
    },
    UserNonFundingLedgerUpdates {
        data: WsUserNonFundingLedgerUpdates,
        is_snapshot: Option<bool>,
    },
    ActiveAssetCtx {
        data: WsActiveAssetCtx,
        is_snapshot: Option<bool>,
    },
    ActiveAssetData {
        data: WsActiveAssetData,
        is_snapshot: Option<bool>,
    },
    UserTwapSliceFills {
        data: WsUserTwapSliceFills,
        is_snapshot: Option<bool>,
    },
    UserTwapHistory {
        data: WsUserTwapHistory,
        is_snapshot: Option<bool>,
    },
    Bbo {
        data: WsBbo,
        is_snapshot: Option<bool>,
    },
    SpotState {
        data: WsSpotState,
        is_snapshot: Option<bool>,
    },
    AllDexsClearinghouseState {
        data: WsAllDexsClearinghouseState,
        is_snapshot: Option<bool>,
    },
    AllDexsAssetCtxs {
        data: WsAllDexsAssetCtxs,
        is_snapshot: Option<bool>,
    },
    /// Any channel not mapped yet, or a payload that failed typed deserialization.
    Unknown {
        channel: String,
        data: Option<Value>,
        is_snapshot: Option<bool>,
    },
}

impl WsMessage {
    /// Hyperliquid `channel` field (wire name).
    pub fn channel(&self) -> &str {
        match self {
            WsMessage::SubscriptionResponse { .. } => "subscriptionResponse",
            WsMessage::Pong => "pong",
            WsMessage::AllMids { .. } => "allMids",
            WsMessage::Notification { .. } => "notification",
            WsMessage::WebData3 { .. } => "webData3",
            WsMessage::TwapStates { .. } => "twapStates",
            WsMessage::ClearinghouseState { .. } => "clearinghouseState",
            WsMessage::OpenOrders { .. } => "openOrders",
            WsMessage::Candle { .. } => "candle",
            WsMessage::L2Book { .. } => "l2Book",
            WsMessage::Trades { .. } => "trades",
            WsMessage::OrderUpdates { .. } => "orderUpdates",
            WsMessage::UserEvents { .. } => "userEvents",
            WsMessage::UserFills { .. } => "userFills",
            WsMessage::UserFundings { .. } => "userFundings",
            WsMessage::UserNonFundingLedgerUpdates { .. } => "userNonFundingLedgerUpdates",
            WsMessage::ActiveAssetCtx { .. } => "activeAssetCtx",
            WsMessage::ActiveAssetData { .. } => "activeAssetData",
            WsMessage::UserTwapSliceFills { .. } => "userTwapSliceFills",
            WsMessage::UserTwapHistory { .. } => "userTwapHistory",
            WsMessage::Bbo { .. } => "bbo",
            WsMessage::SpotState { .. } => "spotState",
            WsMessage::AllDexsClearinghouseState { .. } => "allDexsClearinghouseState",
            WsMessage::AllDexsAssetCtxs { .. } => "allDexsAssetCtxs",
            WsMessage::Unknown { channel, .. } => channel.as_str(),
        }
    }

    /// Snapshot / streaming flag from the outer message (`isSnapshot`), when present.
    pub fn is_snapshot(&self) -> Option<bool> {
        match self {
            WsMessage::SubscriptionResponse { .. } | WsMessage::Pong => None,
            WsMessage::AllMids { is_snapshot, .. }
            | WsMessage::Notification { is_snapshot, .. }
            | WsMessage::WebData3 { is_snapshot, .. }
            | WsMessage::TwapStates { is_snapshot, .. }
            | WsMessage::ClearinghouseState { is_snapshot, .. }
            | WsMessage::OpenOrders { is_snapshot, .. }
            | WsMessage::Candle { is_snapshot, .. }
            | WsMessage::L2Book { is_snapshot, .. }
            | WsMessage::Trades { is_snapshot, .. }
            | WsMessage::OrderUpdates { is_snapshot, .. }
            | WsMessage::UserEvents { is_snapshot, .. }
            | WsMessage::UserFills { is_snapshot, .. }
            | WsMessage::UserFundings { is_snapshot, .. }
            | WsMessage::UserNonFundingLedgerUpdates { is_snapshot, .. }
            | WsMessage::ActiveAssetCtx { is_snapshot, .. }
            | WsMessage::ActiveAssetData { is_snapshot, .. }
            | WsMessage::UserTwapSliceFills { is_snapshot, .. }
            | WsMessage::UserTwapHistory { is_snapshot, .. }
            | WsMessage::Bbo { is_snapshot, .. }
            | WsMessage::SpotState { is_snapshot, .. }
            | WsMessage::AllDexsClearinghouseState { is_snapshot, .. }
            | WsMessage::AllDexsAssetCtxs { is_snapshot, .. }
            | WsMessage::Unknown { is_snapshot, .. } => *is_snapshot,
        }
    }

    /// JSON `data` field as [`serde_json::Value`] (lossless round-trip for app code that still expects JSON).
    pub fn data_json(&self) -> Option<Value> {
        match self {
            WsMessage::SubscriptionResponse { data } => serde_json::to_value(data).ok(),
            WsMessage::Pong => None,
            WsMessage::AllMids { data, .. } => serde_json::to_value(data).ok(),
            WsMessage::Notification { data, .. } => serde_json::to_value(data).ok(),
            WsMessage::WebData3 { data, .. } => serde_json::to_value(data).ok(),
            WsMessage::TwapStates { data, .. } => serde_json::to_value(data).ok(),
            WsMessage::ClearinghouseState { data, .. } => Some(data.clone()),
            WsMessage::OpenOrders { data, .. } => serde_json::to_value(data).ok(),
            WsMessage::Candle { data, .. } => serde_json::to_value(data).ok(),
            WsMessage::L2Book { data, .. } => serde_json::to_value(data).ok(),
            WsMessage::Trades { data, .. } => serde_json::to_value(data).ok(),
            WsMessage::OrderUpdates { data, .. } => serde_json::to_value(data).ok(),
            WsMessage::UserEvents { data, .. } => serde_json::to_value(data).ok(),
            WsMessage::UserFills { data, .. } => serde_json::to_value(data).ok(),
            WsMessage::UserFundings { data, .. } => serde_json::to_value(data).ok(),
            WsMessage::UserNonFundingLedgerUpdates { data, .. } => serde_json::to_value(data).ok(),
            WsMessage::ActiveAssetCtx { data, .. } => serde_json::to_value(data).ok(),
            WsMessage::ActiveAssetData { data, .. } => serde_json::to_value(data).ok(),
            WsMessage::UserTwapSliceFills { data, .. } => serde_json::to_value(data).ok(),
            WsMessage::UserTwapHistory { data, .. } => serde_json::to_value(data).ok(),
            WsMessage::Bbo { data, .. } => serde_json::to_value(data).ok(),
            WsMessage::SpotState { data, .. } => serde_json::to_value(data).ok(),
            WsMessage::AllDexsClearinghouseState { data, .. } => serde_json::to_value(data).ok(),
            WsMessage::AllDexsAssetCtxs { data, .. } => serde_json::to_value(data).ok(),
            WsMessage::Unknown { data, .. } => data.clone(),
        }
    }
}

impl<'de> Deserialize<'de> for WsMessage {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let v = Value::deserialize(deserializer)?;
        parse_ws_message(v).map_err(serde::de::Error::custom)
    }
}

/// Serialize as the wire JSON object `{ channel, data?, isSnapshot? }` (for tests / logging).
impl Serialize for WsMessage {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut s = serializer.serialize_struct("WsMessage", 3)?;
        s.serialize_field("channel", self.channel())?;
        if let Some(data) = self.data_json() {
            s.serialize_field("data", &data)?;
        } else {
            s.skip_field("data")?;
        }
        if let Some(flag) = self.is_snapshot() {
            s.serialize_field("isSnapshot", &flag)?;
        } else {
            s.skip_field("isSnapshot")?;
        }
        s.end()
    }
}

fn parse_ws_message(v: Value) -> Result<WsMessage, String> {
    let channel = v
        .get("channel")
        .and_then(|c| c.as_str())
        .ok_or_else(|| "missing \"channel\"".to_string())?;
    let data = v.get("data").cloned();
    let outer_snapshot = v.get("isSnapshot").and_then(|x| x.as_bool());

    macro_rules! typed {
        ($variant:ident, $t:ty) => {{
            let d = data
                .clone()
                .ok_or_else(|| format!("missing \"data\" for {channel}"))?;
            let inner: $t = serde_json::from_value(d).map_err(|e| e.to_string())?;
            Ok(WsMessage::$variant {
                data: inner,
                is_snapshot: outer_snapshot,
            })
        }};
    }

    match channel {
        "subscriptionResponse" => {
            let d = data.ok_or_else(|| "missing \"data\" for subscriptionResponse".to_string())?;
            // Live API echoes `{ "method": "subscribe", "subscription": { "type": "...", ... } }`;
            // some fixtures use the inner object directly.
            let sub: Subscription = if let Some(inner) = d.get("subscription") {
                serde_json::from_value(inner.clone())
            } else {
                serde_json::from_value(d.clone())
            }
            .map_err(|e| e.to_string())?;
            Ok(WsMessage::SubscriptionResponse { data: sub })
        }
        "pong" => Ok(WsMessage::Pong),
        "allMids" => typed!(AllMids, AllMids),
        "notification" => typed!(Notification, Notification),
        "webData3" => typed!(WebData3, WebData3),
        "twapStates" => typed!(TwapStates, TwapStates),
        "clearinghouseState" => {
            let d = data.ok_or_else(|| "missing \"data\" for clearinghouseState".to_string())?;
            Ok(WsMessage::ClearinghouseState {
                data: d,
                is_snapshot: outer_snapshot,
            })
        }
        "openOrders" => typed!(OpenOrders, OpenOrders),
        "candle" => typed!(Candle, Candle),
        "l2Book" => typed!(L2Book, WsBook),
        "trades" => typed!(Trades, Vec<WsTrade>),
        "orderUpdates" => typed!(OrderUpdates, Vec<WsOrder>),
        "userEvents" => typed!(UserEvents, WsUserEvent),
        "userFills" => {
            let d = data
                .clone()
                .ok_or_else(|| format!("missing \"data\" for {channel}"))?;
            let mut inner: WsUserFills = serde_json::from_value(d).map_err(|e| e.to_string())?;
            if inner.is_snapshot.is_none() {
                inner.is_snapshot = outer_snapshot;
            }
            let merged = outer_snapshot.or(inner.is_snapshot);
            Ok(WsMessage::UserFills {
                data: inner,
                is_snapshot: merged,
            })
        }
        "userFundings" => {
            let d = data
                .clone()
                .ok_or_else(|| format!("missing \"data\" for {channel}"))?;
            let mut inner: WsUserFundings = serde_json::from_value(d).map_err(|e| e.to_string())?;
            if inner.is_snapshot.is_none() {
                inner.is_snapshot = outer_snapshot;
            }
            let merged = outer_snapshot.or(inner.is_snapshot);
            Ok(WsMessage::UserFundings {
                data: inner,
                is_snapshot: merged,
            })
        }
        "userNonFundingLedgerUpdates" => {
            let d = data
                .clone()
                .ok_or_else(|| format!("missing \"data\" for {channel}"))?;
            let mut inner: WsUserNonFundingLedgerUpdates =
                serde_json::from_value(d).map_err(|e| e.to_string())?;
            if inner.is_snapshot.is_none() {
                inner.is_snapshot = outer_snapshot;
            }
            let merged = outer_snapshot.or(inner.is_snapshot);
            Ok(WsMessage::UserNonFundingLedgerUpdates {
                data: inner,
                is_snapshot: merged,
            })
        }
        "activeAssetCtx" => typed!(ActiveAssetCtx, WsActiveAssetCtx),
        "activeAssetData" => typed!(ActiveAssetData, WsActiveAssetData),
        "userTwapSliceFills" => {
            let d = data
                .clone()
                .ok_or_else(|| format!("missing \"data\" for {channel}"))?;
            let mut inner: WsUserTwapSliceFills =
                serde_json::from_value(d).map_err(|e| e.to_string())?;
            if inner.is_snapshot.is_none() {
                inner.is_snapshot = outer_snapshot;
            }
            let merged = outer_snapshot.or(inner.is_snapshot);
            Ok(WsMessage::UserTwapSliceFills {
                data: inner,
                is_snapshot: merged,
            })
        }
        "userTwapHistory" => {
            let d = data
                .clone()
                .ok_or_else(|| format!("missing \"data\" for {channel}"))?;
            let mut inner: WsUserTwapHistory =
                serde_json::from_value(d).map_err(|e| e.to_string())?;
            if inner.is_snapshot.is_none() {
                inner.is_snapshot = outer_snapshot;
            }
            let merged = outer_snapshot.or(inner.is_snapshot);
            Ok(WsMessage::UserTwapHistory {
                data: inner,
                is_snapshot: merged,
            })
        }
        "bbo" => typed!(Bbo, WsBbo),
        "spotState" => typed!(SpotState, WsSpotState),
        "allDexsClearinghouseState" => {
            typed!(AllDexsClearinghouseState, WsAllDexsClearinghouseState)
        }
        "allDexsAssetCtxs" => typed!(AllDexsAssetCtxs, WsAllDexsAssetCtxs),
        _ => Ok(WsMessage::Unknown {
            channel: channel.to_string(),
            data,
            is_snapshot: outer_snapshot,
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trades_channel_roundtrip() {
        let v = serde_json::json!({
            "channel": "trades",
            "data": [{
                "coin": "SOL",
                "side": "B",
                "px": "1",
                "sz": "2",
                "hash": "h",
                "time": 3,
                "tid": 4,
                "users": ["0xa", "0xb"]
            }]
        });
        let m: WsMessage = serde_json::from_value(v).unwrap();
        assert!(matches!(m, WsMessage::Trades { .. }));
    }

    #[test]
    fn subscription_response() {
        let v = serde_json::json!({
            "channel": "subscriptionResponse",
            "data": { "type": "trades", "coin": "ETH" }
        });
        let m: WsMessage = serde_json::from_value(v).unwrap();
        match m {
            WsMessage::SubscriptionResponse { data } => {
                assert!(matches!(data, Subscription::Trades { .. }));
            }
            _ => panic!("expected subscription response"),
        }
    }

    #[test]
    fn subscription_response_nested_echo() {
        let v = serde_json::json!({
            "channel": "subscriptionResponse",
            "data": {
                "method": "subscribe",
                "subscription": { "type": "activeAssetCtx", "coin": "BTC" }
            }
        });
        let m: WsMessage = serde_json::from_value(v).unwrap();
        match m {
            WsMessage::SubscriptionResponse { data } => {
                assert!(matches!(
                    data,
                    Subscription::ActiveAssetCtx { ref coin } if coin == "BTC"
                ));
            }
            _ => panic!("expected subscription response"),
        }
    }
}
