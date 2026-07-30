use alloy::primitives::Address;
use derive_builder::Builder;
use hl_rs_derive::UserSignedAction;
use serde::{Deserialize, Serialize};

/// Approve a builder fee rate for a specific builder address.
#[derive(Debug, Clone, Serialize, Deserialize, Builder, UserSignedAction)]
#[action(
    types = "ApproveBuilderFee(string hyperliquidChain,string maxFeeRate,address builder,uint64 nonce)"
)]
#[serde(rename_all = "camelCase")]
#[builder(setter(into))]
pub struct ApproveBuilderFee {
    /// Maximum fee rate as a percentage string (e.g. `"0.01%"` for 1 bp, `"0.1%"` for 10 bps).
    ///
    /// Hyperliquid requires the `%` suffix on the wire and in EIP-712 signing.
    pub max_fee_rate: String,
    /// Builder address to approve
    pub builder: Address,
    #[builder(default)]
    pub nonce: Option<u64>,
}

impl ApproveBuilderFee {
    pub fn builder() -> ApproveBuilderFeeBuilder {
        ApproveBuilderFeeBuilder::default()
    }

    /// Create a new approval with the given fee rate and builder address.
    ///
    /// `max_fee_rate` must be a percentage string with a `%` suffix, e.g. `"0.001%"`.
    pub fn new(max_fee_rate: impl Into<String>, builder: Address) -> Self {
        Self {
            max_fee_rate: max_fee_rate.into(),
            builder,
            nonce: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::actions::build_action_value;
    use crate::SigningChain;

    #[test]
    fn max_fee_rate_serializes_with_percent_suffix_on_wire() {
        let action = ApproveBuilderFee::new("0.001%", Address::repeat_byte(0xab));
        let wire = build_action_value(&action, Some(&SigningChain::Testnet)).unwrap();
        assert_eq!(
            wire.get("maxFeeRate").and_then(|v| v.as_str()),
            Some("0.001%")
        );
    }
}
