use std::str::FromStr;

use alloy::signers::local::PrivateKeySigner;
use hl_rs::{BaseUrl, DisableDex, ExchangeClient};

#[tokio::main]
async fn main() {
    dotenv::dotenv().unwrap();

    let url = BaseUrl::Testnet;
    let dex_name = "slob";

    // Sends:
    // { "type": "perpDeploy", "disableDex": "<dex name>" }
    let action = DisableDex::new(dex_name);

    let private_key = std::env::var("PRIVATE_KEY").unwrap();
    let wallet = PrivateKeySigner::from_str(&private_key).unwrap();
    println!("wallet: {}", wallet.address());

    let client = ExchangeClient::new(url).with_signer(wallet);
    let result = client.send_action(action).await.unwrap();

    println!("Disable dex result: {:?}", result);
}
