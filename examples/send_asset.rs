use std::str::FromStr;

use alloy::primitives::{Address, address};
use alloy::signers::local::PrivateKeySigner;
use hl_rs::{BaseUrl, DexId, ExchangeClient, SendAsset};
use rust_decimal_macros::dec;

#[tokio::main]
async fn main() {
    dotenv::dotenv().unwrap();

    let destination = address!("0x73c4c9fB0113f19D4d015E4ebb06cEaabc2b3CEA");

    // Transfer USDC from spot to perp margin for the destination account.
    let action = SendAsset::new(
        destination,
        DexId::Hip3("name".to_owned()),
        DexId::Hip3("name".to_owned()),
        "USDC",
        dec!(69.0),
    );

    let wallet = PrivateKeySigner::from_str("0x..").unwrap();
    println!("wallet: {}", wallet.address());

    let client = ExchangeClient::new(BaseUrl::Testnet).with_signer(wallet);
    let result = client.send_action(action).await.unwrap();

    println!("Send asset result: {:?}", result);
}
