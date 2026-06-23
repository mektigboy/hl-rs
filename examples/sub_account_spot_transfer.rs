//! Transfer USDC between the main account and a sub-account on spot (testnet).
//!
//! `SubAccountSpotTransfer` moves tokens between your main spot balance and a
//! sub-account address — not directly between two sub-accounts.
//!
//! # Environment
//! - `PRIVATE_KEY` - hex private key for the main account (required)
//!
//! # Optional
//! - `SUB_ACCOUNT` - sub-account address (default: address below)
//! - `AMOUNT` - USDC amount (default: `10`)
//! - `WITHDRAW` - set to `1` to withdraw from sub-account instead of deposit

use std::str::FromStr;

use alloy::primitives::{address, Address};
use alloy::signers::local::PrivateKeySigner;
use hl_rs::{BaseUrl, ExchangeClient, SubAccountSpotTransfer};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;

const SUB_ACCOUNT: Address = address!("0xaa3b7392052d62928cc87701e3ca6fb6630bb6e2");

#[tokio::main]
async fn main() {
    dotenv::dotenv().ok();

    let amount = dec!(10);

    let action = SubAccountSpotTransfer::deposit(SUB_ACCOUNT, "USDC", amount);

    let private_key = std::env::var("PRIVATE_KEY").expect("PRIVATE_KEY must be set");
    let wallet = PrivateKeySigner::from_str(&private_key).expect("invalid PRIVATE_KEY");
    println!("wallet: {}", wallet.address());
    println!("depositing {amount} USDC to sub-account {SUB_ACCOUNT}");

    let client = ExchangeClient::new(BaseUrl::Mainnet).with_signer(wallet);
    let result = client
        .send_action(action)
        .await
        .expect("sub-account spot transfer failed");

    println!("sub-account spot transfer result: {result:?}");
}
