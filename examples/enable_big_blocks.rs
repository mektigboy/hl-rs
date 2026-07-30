//! Enable big blocks for HyperEVM transactions on testnet.
//!
//! Sends an `evmUserModify` action with `usingBigBlocks: true`. Once enabled,
//! subsequent HyperEVM transactions from this wallet use large blocks (30M gas
//! limit, ~1 per minute) instead of small blocks (2M gas limit, ~1 per second).
//!
//! Useful for contract deployments and other high-gas EVM operations.
//!
//! # Environment
//! - `PRIVATE_KEY` - hex private key for the wallet (required)

use std::str::FromStr;

use alloy::signers::local::PrivateKeySigner;
use hl_rs::{BaseUrl, ExchangeClient, ToggleBigBlocks};

#[tokio::main]
async fn main() {
    dotenv::dotenv().ok();

    let private_key = std::env::var("PRIVATE_KEY").expect("PRIVATE_KEY must be set");
    let wallet = PrivateKeySigner::from_str(&private_key).expect("invalid PRIVATE_KEY");

    println!("wallet: {}", wallet.address());
    println!("enabling big blocks on testnet");

    let action = ToggleBigBlocks::enable();

    let client = ExchangeClient::new(BaseUrl::Testnet).with_signer(wallet);
    let result = client
        .send_action(action)
        .await
        .expect("enable big blocks failed");

    println!("enable big blocks result: {result:?}");
}
