use hl_rs_derive::L1Action;
use serde::{Deserialize, Serialize};

/// Reserve additional request weight (rate-limit capacity).
///
/// Instead of trading to raise the address-based rate limit, this action reserves additional
/// actions directly. It is paid from the signer's **perp balance** at `0.0005 USDC` per unit of
/// `weight` (so `weight = floor(usdc / 0.0005)`).
///
/// See [Reserve additional actions](https://hyperliquid.gitbook.io/hyperliquid-docs/for-developers/api/exchange-endpoint#reserve-additional-actions).
///
/// # Example
/// ```no_run
/// use std::str::FromStr;
///
/// use alloy::signers::local::PrivateKeySigner;
/// use hl_rs::{BaseUrl, ExchangeClient, ReserveRequestWeight};
///
/// # async fn run() -> Result<(), hl_rs::Error> {
/// let wallet = PrivateKeySigner::from_str(
///     "0x0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
/// )
/// .expect("valid private key hex");
/// let client = ExchangeClient::new(BaseUrl::Testnet).with_signer(wallet);
///
/// let action = ReserveRequestWeight::new(1_000);
/// let _response = client.send_action(action).await?;
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, L1Action)]
#[action(action_type = "reserveRequestWeight")]
#[serde(rename_all = "camelCase")]
pub struct ReserveRequestWeight {
    /// Number of additional actions to reserve.
    pub weight: u64,
    #[serde(skip_serializing)]
    pub nonce: Option<u64>,
}

impl ReserveRequestWeight {
    pub fn new(weight: u64) -> Self {
        Self {
            weight,
            nonce: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::SigningChain;
    use crate::actions::{PreparedAction, SignedActionKind};
    use alloy::primitives::U256;
    use alloy_signer::Signature;
    use serde_json::json;

    #[test]
    fn action_shape_matches_docs() {
        let action = ReserveRequestWeight::new(1234);
        let prepared = PreparedAction::new(action, &SigningChain::Testnet, None, None).unwrap();
        let signed = prepared.with_signature(Signature::new(U256::from(1), U256::from(2), false));

        let v = serde_json::to_value(&signed).unwrap();
        assert_eq!(
            v.get("action").unwrap(),
            &json!({ "type": "reserveRequestWeight", "weight": 1234 })
        );
    }

    #[test]
    fn round_trips_through_signed_action_kind() {
        let action = ReserveRequestWeight::new(42);
        let prepared = PreparedAction::new(action, &SigningChain::Testnet, None, None).unwrap();
        let signed = prepared.with_signature(Signature::new(U256::from(3), U256::from(4), true));
        let json = serde_json::to_string(&signed).unwrap();

        match SignedActionKind::from_json(&json).unwrap().action {
            crate::actions::ActionKind::ReserveRequestWeight(a) => assert_eq!(a.weight, 42),
            other => panic!("expected ReserveRequestWeight, got {other:?}"),
        }
    }
}
