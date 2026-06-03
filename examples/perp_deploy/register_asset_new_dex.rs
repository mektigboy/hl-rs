use std::str::FromStr;

use alloy::signers::local::PrivateKeySigner;
use hl_rs::{
    actions::{PerpDexSchema, RegisterAsset},
    AssetRequest, BaseUrl, ExchangeClient,
};
use rust_decimal_macros::dec;

#[tokio::main]
async fn main() {
    dotenv::dotenv().unwrap();

    let url = BaseUrl::Testnet;
    let dex_name = "bde";

    let asset_request = AssetRequest {
        coin: "TSLA".to_string(),
        sz_decimals: 3,
        oracle_px: dec!(0.1),
        margin_table_id: 12,
        only_isolated: false,
    };

    let schema = PerpDexSchema::new("Big Dex Energy", 1452); // USDH testnet

    let action = RegisterAsset::new(dex_name, asset_request).deploy_dex(schema);

    let private_key = std::env::var("PRIVATE_KEY").unwrap();
    let wallet = PrivateKeySigner::from_str(&private_key).unwrap();
    println!("wallet: {}", wallet.address());

    let client = ExchangeClient::new(url).with_signer(wallet);

    let register_asset_result = client.send_action(action).await.unwrap();

    println!("Register asset result: {:?}", register_asset_result);
}
