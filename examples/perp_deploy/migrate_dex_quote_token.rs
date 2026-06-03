use std::str::FromStr;

use alloy::signers::local::PrivateKeySigner;
use hl_rs::{BaseUrl, ExchangeClient, MigrateDexQuoteToken, PerpDexSchema};

#[tokio::main]
async fn main() {
    dotenv::dotenv().unwrap();

    let url = BaseUrl::Testnet;

    // Note: this action is intended only for migrating USDH DEXes.
    let action = MigrateDexQuoteToken::new(
        "slob",
        "slob",
        PerpDexSchema::new("New Slob Dex", "1452"),
    );

    let private_key = std::env::var("PRIVATE_KEY").unwrap();
    let wallet = PrivateKeySigner::from_str(&private_key).unwrap();
    println!("wallet: {}", wallet.address());

    let client = ExchangeClient::new(url).with_signer(wallet);
    let result = client.send_action(action).await.unwrap();

    println!("Migrate dex quote token result: {:?}", result);
}
