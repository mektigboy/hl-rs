use std::str::FromStr;

use alloy::signers::local::PrivateKeySigner;
use hl_rs::{BaseUrl, ExchangeClient, ToggleTrading};

#[tokio::main]
async fn main() {
    dotenv::dotenv().unwrap();

    let url = BaseUrl::Mainnet;
    let dex_name = "km";

    // Resume (enable) trading for an asset
    let action = ToggleTrading::halt(dex_name, "GLDMINE");

    // To halt (disable) trading instead, use:
    // let action = ToggleTrading::halt(dex_name, "TSLA");

    let private_key = std::env::var("PRIVATE_KEY").unwrap();
    let wallet = PrivateKeySigner::from_str(&private_key).unwrap();
    println!("wallet: {}", wallet.address());

    let client = ExchangeClient::new(url).with_signer(wallet);

    let result = client.send_action(action).await.unwrap();

    println!("Toggle trading result: {:?}", result);
}
