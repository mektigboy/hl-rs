use std::str::FromStr;

use alloy::{primitives::address, signers::local::PrivateKeySigner};
use hl_rs::{BaseUrl, ConvertToMultiSigUser, ExchangeClient};

#[tokio::main]
async fn main() {
    dotenv::dotenv().unwrap();

    let private_key = std::env::var("PRIVATE_KEY").unwrap();
    let wallet = PrivateKeySigner::from_str(&private_key).unwrap();
    let authorized_signer = address!("0xE71EA6211e4956712c3BB3f36F9d0050bcD48BA0");

    // 1 authorized signer, threshold 1
    let action = ConvertToMultiSigUser::new(vec![authorized_signer], 1);

    let client = ExchangeClient::new(BaseUrl::Testnet).with_signer(wallet);
    let result = client.send_action(action).await.unwrap();

    println!("ConvertToMultiSigUser result: {:?}", result);
}
