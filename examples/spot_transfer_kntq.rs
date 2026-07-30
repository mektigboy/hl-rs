//! Transfer spot kNTQ between Hyperliquid accounts.
//!
//! # Environment
//! - `PRIVATE_KEY` - hex private key for the sender wallet (required)
//! - `DESTINATION` - recipient wallet address (required)
//!
//! # Optional
//! - `TOKEN` - spot token symbol (default: `kNTQ`)
//! - `AMOUNT` - transfer amount as decimal string (default: `1`)

use std::str::FromStr;

use alloy::primitives::{address, Address};
use alloy::signers::local::PrivateKeySigner;
use hl_rs::{BaseUrl, ExchangeClient, SpotTransfer};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;

#[tokio::main]
async fn main() {
    dotenv::dotenv().ok();

    let chain = BaseUrl::Mainnet;
    let private_key = std::env::var("PRIVATE_KEY").expect("PRIVATE_KEY must be set");
    let destination = address!("0x696238e0ca31c94e24ca4cbe7921754e172e4d0f");

    let token = "KNTQ:0xbd31bd605c0a1b82c72aae3587f9061f".to_string(); //KNTQ on mainnet
    let amount: Decimal = dec!(469_200);

    let action = SpotTransfer::new(destination, token, amount);

    let wallet = PrivateKeySigner::from_str(&private_key).expect("invalid PRIVATE_KEY");
    let client = ExchangeClient::new(chain).with_signer(wallet);

    let result = client
        .send_action(action)
        .await
        .expect("spot transfer failed");
    println!("spot transfer result: {result:?}");
}
