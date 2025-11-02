use std::collections::HashMap;

use alloy::signers::local::PrivateKeySigner;
use hl_rs::{
    ExchangeClient,
    exchange::{ActionKind, builder::BuildAction, requests::UsdSend},
};

#[tokio::main]
async fn main() {
    env_logger::init();

    // Key was randomly generated for testing and shouldn't be used with any real funds
    let wallet: PrivateKeySigner =
        "e908f86dbb4d55ac876378565aafeabc187f6690f046459397b17d9b9a19688e"
            .parse()
            .unwrap();

    let exchange_client = ExchangeClient::new(None, None, HashMap::new()).unwrap();

    let usd_send = UsdSend {
        signature_chain_id: 421614,
        hyperliquid_chain: "Testnet".to_string(),
        destination: "0x1234567890123456789012345678901234567890".to_string(),
        amount: "100".to_string(),
        time: 1690393044548,
    };

    let action = ActionKind::UsdSend(usd_send)
        .build(&exchange_client)
        .expect("Failed to build action");

    let signed = action.sign(&wallet).expect("Failed to sign action");

    println!("Signed action: {:?}", signed.signature.to_string());

    // let res = signed.send().await.expect("Failed to send action");

    // // let res = exchange_client
    // //     .usdc_transfer(amount, destination, None)
    // //     .await
    // //     .unwrap();
    // info!("Usdc transfer result: {res:?}");
}
