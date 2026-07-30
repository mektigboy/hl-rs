//! Subscription types for the Hyperliquid WebSocket API.
//!
//! See <https://hyperliquid.gitbook.io/hyperliquid-docs/for-developers/api/websocket/subscriptions>

use serde::{Deserialize, Serialize};

/// Subscription request for a specific feed.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum Subscription {
    #[serde(rename = "allMids")]
    AllMids {
        #[serde(skip_serializing_if = "Option::is_none")]
        dex: Option<String>,
    },

    Notification {
        user: String,
    },

    #[serde(rename = "webData3")]
    WebData3 {
        user: String,
    },

    #[serde(rename = "twapStates")]
    TwapStates {
        user: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        dex: Option<String>,
    },

    #[serde(rename = "clearinghouseState")]
    ClearinghouseState {
        user: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        dex: Option<String>,
    },

    #[serde(rename = "openOrders")]
    OpenOrders {
        user: String,
        dex: Option<String>,
    },

    Candle {
        coin: String,
        interval: String,
    },

    #[serde(rename = "l2Book")]
    L2Book {
        coin: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        n_sig_figs: Option<i32>,
        #[serde(skip_serializing_if = "Option::is_none")]
        mantissa: Option<i32>,
    },

    Trades {
        coin: String,
    },

    #[serde(rename = "orderUpdates")]
    OrderUpdates {
        user: String,
    },

    #[serde(rename = "userEvents")]
    UserEvents {
        user: String,
    },

    #[serde(rename = "userFills")]
    UserFills {
        user: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        aggregate_by_time: Option<bool>,
    },

    #[serde(rename = "userFundings")]
    UserFundings {
        user: String,
    },

    #[serde(rename = "userNonFundingLedgerUpdates")]
    UserNonFundingLedgerUpdates {
        user: String,
    },

    #[serde(rename = "activeAssetCtx")]
    ActiveAssetCtx {
        coin: String,
    },

    #[serde(rename = "activeAssetData")]
    ActiveAssetData {
        user: String,
        coin: String,
    },

    #[serde(rename = "userTwapSliceFills")]
    UserTwapSliceFills {
        user: String,
    },

    #[serde(rename = "userTwapHistory")]
    UserTwapHistory {
        user: String,
    },

    Bbo {
        coin: String,
    },

    #[serde(rename = "spotState")]
    SpotState {
        user: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        is_portfolio_margin: Option<bool>,
    },

    #[serde(rename = "allDexsClearinghouseState")]
    AllDexsClearinghouseState {
        user: String,
    },

    #[serde(rename = "allDexsAssetCtxs")]
    AllDexsAssetCtxs,
}
