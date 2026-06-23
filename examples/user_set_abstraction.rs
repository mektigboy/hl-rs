//! Disable account abstraction for the signing wallet on testnet.
//!
//! Mirrors [user_abstraction.py](https://github.com/hyperliquid-dex/hyperliquid-python-sdk/blob/2fdb18f9517675ea03695a0962bd19eece9c83f0/examples/user_abstraction.py):
//! the principal wallet must sign `userSetAbstraction` for its own address.
//!
//! The account must be in `"default"` mode before some transitions succeed.
//!
//! # Environment
//! - `PRIVATE_KEY` - hex private key for the user wallet (required)
//!
//! # Optional
//! - `USER` - user address to modify (default: wallet address)
//! - `ABSTRACTION` - `disabled`, `unifiedAccount`, `portfolioMargin`, or `dexAbstraction`
//!   (default: `disabled`)

use std::str::FromStr;

use alloy::signers::local::PrivateKeySigner;
use hl_rs::{AbstractionMode, BaseUrl, ExchangeClient, UserSetAbstraction};

#[tokio::main]
async fn main() {
    dotenv::dotenv().ok();

    let private_key = std::env::var("PRIVATE_KEY").expect("PRIVATE_KEY must be set");
    let wallet = PrivateKeySigner::from_str(&private_key).expect("invalid PRIVATE_KEY");

    let action = UserSetAbstraction::new(wallet.address(), AbstractionMode::Disabled);

    println!("wallet: {}", wallet.address());
    println!("setting abstraction to `{}`", AbstractionMode::Disabled.as_str());

    let client = ExchangeClient::new(BaseUrl::Testnet).with_signer(wallet);
    let result = client
        .send_action(action)
        .await
        .expect("user set abstraction failed");

    println!("user set abstraction result: {result:?}");
}
