use hl_rs_derive::L1Action;
use serde::{Deserialize, Serialize};

/// Claim accrued builder-code / referral rewards.
///
/// Serializes to `{"type":"claimRewards"}`; the `nonce` field is required by the
/// `L1Action` derive but is not part of the wire payload.
#[derive(Serialize, Deserialize, Debug, Clone, L1Action, Default)]
#[action(action_type = "claimRewards")]
pub struct ClaimRewards {
    #[serde(skip_serializing)]
    pub nonce: Option<u64>,
}

impl ClaimRewards {
    pub fn new() -> Self {
        Self::default()
    }
}
