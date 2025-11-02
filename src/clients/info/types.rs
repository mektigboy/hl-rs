use serde::{Deserialize, Serialize};

use alloy::primitives::Address;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserStateResponse {
    pub asset_positions: Vec<AssetPosition>,
    pub cross_margin_summary: MarginSummary,
    pub margin_summary: MarginSummary,
    pub withdrawable: String,
}

#[derive(Debug, Deserialize)]
pub struct AssetPosition {
    pub position: PositionData,
    #[serde(rename = "type")]
    pub type_string: String,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct PositionData {
    pub coin: String,
    pub entry_px: Option<String>,
    pub leverage: Leverage,
    pub liquidation_px: Option<String>,
    pub margin_used: String,
    pub position_value: String,
    pub return_on_equity: String,
    pub szi: String,
    pub unrealized_pnl: String,
    pub max_leverage: u32,
    pub cum_funding: CumulativeFunding,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct CumulativeFunding {
    pub all_time: String,
    pub since_open: String,
    pub since_change: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MarginSummary {
    pub account_value: String,
    pub total_margin_used: String,
    pub total_ntl_pos: String,
    pub total_raw_usd: String,
}

#[derive(Debug, Deserialize)]
pub struct UserTokenBalanceResponse {
    pub balances: Vec<UserTokenBalance>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserTokenBalance {
    pub coin: String,
    pub hold: String,
    pub total: String,
    pub entry_ntl: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserFeesResponse {
    pub active_referral_discount: String,
    pub daily_user_vlm: Vec<DailyUserVlm>,
    pub fee_schedule: FeeSchedule,
    pub user_add_rate: String,
    pub user_cross_rate: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DailyUserVlm {
    pub date: String,
    pub exchange: String,
    pub user_add: String,
    pub user_cross: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FeeSchedule {
    pub add: String,
    pub cross: String,
    pub referral_discount: String,
    pub tiers: Tiers,
}

#[derive(Deserialize, Debug)]
pub struct Tiers {
    pub mm: Vec<Mm>,
    pub vip: Vec<Vip>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Mm {
    pub add: String,
    pub maker_fraction_cutoff: String,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Vip {
    pub add: String,
    pub cross: String,
    pub ntl_cutoff: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OpenOrdersResponse {
    pub coin: String,
    pub limit_px: String,
    pub oid: u64,
    pub side: String,
    pub sz: String,
    pub timestamp: u64,
    pub cloid: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserFillsResponse {
    pub closed_pnl: String,
    pub coin: String,
    pub crossed: bool,
    pub dir: String,
    pub hash: String,
    pub oid: u64,
    pub px: String,
    pub side: String,
    pub start_position: String,
    pub sz: String,
    pub time: u64,
    pub fee: String,
    pub tid: u64,
    pub fee_token: String,
    pub twap_id: Option<u64>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FundingHistoryResponse {
    pub coin: String,
    pub funding_rate: String,
    pub premium: String,
    pub time: u64,
}

#[derive(Debug, Deserialize)]
pub struct UserFundingResponse {
    pub time: u64,
    pub hash: String,
    pub delta: Delta,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Delta {
    #[serde(rename = "type")]
    pub type_string: String,
    pub coin: String,
    pub usdc: String,
    pub szi: String,
    pub funding_rate: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct L2SnapshotResponse {
    pub coin: String,
    pub levels: Vec<Vec<Level>>,
    pub time: u64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Level {
    pub n: u64,
    pub px: String,
    pub sz: String,
}

#[derive(serde::Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct RecentTradesResponse {
    pub coin: String,
    pub side: String,
    pub px: String,
    pub sz: String,
    pub time: u64,
    pub hash: String,
}

#[derive(serde::Deserialize, Debug)]
pub struct CandlesSnapshotResponse {
    #[serde(rename = "t")]
    pub time_open: u64,
    #[serde(rename = "T")]
    pub time_close: u64,
    #[serde(rename = "s")]
    pub coin: String,
    #[serde(rename = "i")]
    pub candle_interval: String,
    #[serde(rename = "o")]
    pub open: String,
    #[serde(rename = "c")]
    pub close: String,
    #[serde(rename = "h")]
    pub high: String,
    #[serde(rename = "l")]
    pub low: String,
    #[serde(rename = "v")]
    pub vlm: String,
    #[serde(rename = "n")]
    pub num_trades: u64,
}

#[derive(Deserialize, Debug)]
pub struct OrderStatusResponse {
    pub status: String,
    #[serde(default)]
    pub order: Option<OrderInfo>, // `None` if the order is not found
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OrderInfo {
    pub order: BasicOrderInfo,
    pub status: String,
    pub status_timestamp: u64,
}

#[derive(Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct BasicOrderInfo {
    pub coin: String,
    pub side: String,
    pub limit_px: String,
    pub sz: String,
    pub oid: u64,
    pub timestamp: u64,
    pub trigger_condition: String,
    pub is_trigger: bool,
    pub trigger_px: String,
    pub is_position_tpsl: bool,
    pub reduce_only: bool,
    pub order_type: String,
    pub orig_sz: String,
    pub tif: Option<String>,
    pub cloid: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReferralResponse {
    pub referred_by: Option<Referrer>,
    pub cum_vlm: String,
    pub unclaimed_rewards: String,
    pub claimed_rewards: String,
    pub referrer_state: ReferrerState,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Referrer {
    pub referrer: Address,
    pub code: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReferrerState {
    pub stage: String,
    pub data: ReferrerData,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReferrerData {
    pub required: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ActiveAssetDataResponse {
    pub user: Address,
    pub coin: String,
    pub leverage: Leverage,
    pub max_trade_szs: Vec<String>,
    pub available_to_trade: Vec<String>,
    pub mark_px: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Leverage {
    #[serde(rename = "type")]
    pub type_string: String,
    pub value: u32,
    pub raw_usd: Option<String>,
}
