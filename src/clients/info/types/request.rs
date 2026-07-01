use alloy::primitives::Address;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(tag = "type")]
#[serde(rename_all = "camelCase")]
pub enum InfoRequest {
    #[serde(rename = "clearinghouseState")]
    UserState {
        user: Address,
    },
    #[serde(rename = "batchClearinghouseStates")]
    UserStates {
        users: Vec<Address>,
    },
    #[serde(rename = "spotClearinghouseState")]
    UserTokenBalances {
        user: Address,
    },
    UserFees {
        user: Address,
    },
    #[serde(rename = "delegatorSummary")]
    UserStakingSummary {
        user: Address,
    },

    OpenOrders {
        user: Address,
    },
    OrderStatus {
        user: Address,
        oid: u64,
    },
    Meta,
    MetaAndAssetCtxs {
        #[serde(skip_serializing_if = "Option::is_none")]
        dex: Option<String>,
    },
    SpotMeta,
    SpotMetaAndAssetCtxs,
    AllMids,
    UserFills {
        user: Address,
    },
    #[serde(rename_all = "camelCase")]
    FundingHistory {
        coin: String,
        start_time: u64,
        end_time: Option<u64>,
    },
    #[serde(rename_all = "camelCase")]
    UserFunding {
        user: Address,
        start_time: u64,
        end_time: Option<u64>,
    },
    L2Book {
        coin: String,
    },
    RecentTrades {
        coin: String,
    },
    #[serde(rename_all = "camelCase")]
    CandleSnapshot {
        req: CandleSnapshotRequest,
    },
    Referral {
        user: Address,
    },
    HistoricalOrders {
        user: Address,
    },
    ActiveAssetData {
        user: Address,
        coin: String,
    },
    PerpDexs,
    PerpDexStatus {
        dex: String,
    },
    PerpDeployAuctionStatus,
    UserRole {
        user: Address,
    },
    UserToMultiSigSigners {
        user: Address,
    },
    #[serde(rename = "userRateLimit")]
    UserRateLimit {
        user: Address,
    },
}

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CandleSnapshotRequest {
    coin: String,
    interval: String,
    start_time: u64,
    end_time: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn meta_and_asset_ctxs_request_omits_null_dex() {
        let req = InfoRequest::MetaAndAssetCtxs { dex: None };
        let json = serde_json::to_string(&req).unwrap();
        assert_eq!(json, r#"{"type":"metaAndAssetCtxs"}"#);
    }
}
