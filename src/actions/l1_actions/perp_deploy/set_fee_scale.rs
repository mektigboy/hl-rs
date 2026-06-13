use hl_rs_derive::L1Action;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/// Set the fee scale for a perp DEX.
///
/// Scale must be between 0.0 and 3.0. On mainnet, changes are rate limited to
/// one per 30 days. See [HIP-3 deployer actions](https://hyperliquid.gitbook.io/hyperliquid-docs/for-developers/api/hip-3-deployer-actions).
#[derive(Serialize, Deserialize, Debug, Clone, L1Action)]
#[action(action_type = "perpDeploy", payload_key = "setFeeScale")]
#[serde(rename_all = "camelCase")]
pub struct SetFeeScale {
    /// The DEX name (lowercase).
    pub dex: String,
    /// Fee scale between 0.0 and 3.0.
    pub scale: Decimal,
    #[serde(skip_serializing)]
    pub nonce: Option<u64>,
}

impl SetFeeScale {
    pub fn new(dex: impl Into<String>, scale: Decimal) -> Self {
        Self {
            dex: dex.into(),
            scale,
            nonce: None,
        }
    }
}
