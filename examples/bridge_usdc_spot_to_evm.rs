//! Bridge USDC from HyperCore spot to HyperEVM on testnet.
//!
//! Sends a `spotSend` to the USDC system address (`0x20…0000`). Hyperliquid
//! credits the same amount to your wallet on HyperEVM as native USDC
//! (`0x2B3370eE501B4a559b57D449569354196457D8Ab` on testnet).
//!
//! See [HyperCore ↔ HyperEVM transfers](https://hyperliquid.gitbook.io/hyperliquid-docs/for-developers/hyperevm/hypercore-less-than-greater-than-hyperevm-transfers).
//!
//! # Environment
//! - `PRIVATE_KEY` - hex private key for the sender wallet (required)
//!
//! # Optional
//! - `AMOUNT` - USDC amount as decimal string (default: `1`)

use std::str::FromStr;

use alloy::primitives::address;
use alloy::signers::local::PrivateKeySigner;
use hl_rs::{BaseUrl, ExchangeClient, SpotTransfer};
use rust_decimal::Decimal;

/// USDC system address (token index 0): `0x20` + zeros + index.
const USDC_SYSTEM_ADDRESS: alloy::primitives::Address =
    address!("0x2000000000000000000000000000000000000000");

#[tokio::main]
async fn main() {
    dotenv::dotenv().ok();

    let amount = rust_decimal_macros::dec!(10);

    let private_key = std::env::var("PRIVATE_KEY").expect("PRIVATE_KEY must be set");
    let wallet = PrivateKeySigner::from_str(&private_key).expect("invalid PRIVATE_KEY");

    println!("sender: {}", wallet.address());
    println!("bridging {amount} USDC from spot to HyperEVM (testnet)");

    let action = SpotTransfer::new(USDC_SYSTEM_ADDRESS, "USDC", amount);

    let client = ExchangeClient::new(BaseUrl::Testnet).with_signer(wallet);
    let result = client
        .send_action(action)
        .await
        .expect("bridge spot → EVM failed");

    println!("bridge result: {result:?}");
}
