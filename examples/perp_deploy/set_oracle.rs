use std::str::FromStr;

use alloy::signers::local::PrivateKeySigner;
use hl_rs::{BaseUrl, ExchangeClient, SetOracle, TESTNET_API_URL};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AssetCtx {
    oracle_px: Option<String>,
    mark_px: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DexAsset {
    name: String,
    #[serde(default)]
    is_delisted: bool,
}

#[derive(Debug, Deserialize)]
struct MetaResponse {
    universe: Vec<DexAsset>,
}

#[tokio::main]
async fn main() {
    dotenv::dotenv().unwrap();

    let url = BaseUrl::Testnet;
    let dex_name = "blob";

    // Build a valid request for live assets on dddd using:
    // { "type": "metaAndAssetCtxs", "dex": "dddd" }.
    let http = reqwest::Client::new();
    let response: (MetaResponse, Vec<AssetCtx>) = http
        .post(TESTNET_API_URL.to_string() + "/info")
        .json(&serde_json::json!({
            "type": "metaAndAssetCtxs",
            "dex": dex_name,
        }))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();

    let (meta, asset_ctxs) = response;
    assert_eq!(
        meta.universe.len(),
        asset_ctxs.len(),
        "meta/universe length mismatch for slob",
    );

    let mut oracle_pxs = Vec::new();
    let mut mark_prices = Vec::new();

    for (asset, ctx) in meta.universe.iter().zip(asset_ctxs.iter()) {
        //if asset.is_delisted {
        //    continue;
        //}

        // oraclePxs should carry the external oracle value where available.
        let oracle_px = ctx.oracle_px.clone().unwrap_or_else(|| ctx.mark_px.clone());
        oracle_pxs.push((asset.name.clone(), oracle_px));
        mark_prices.push((asset.name.clone(), ctx.mark_px.clone()));
    }

    assert!(!oracle_pxs.is_empty(), "No live (non-delisted) assets found for dddd");

    let action = SetOracle {
        dex: dex_name.to_string(),
        oracle_pxs,
        // Use one mark-price source list; backend combines it with local mark.
        mark_pxs: vec![mark_prices.clone()],
        // Keep this aligned with all listed assets.
        external_perp_pxs: mark_prices,
        nonce: None,
    };

    let private_key = std::env::var("PRIVATE_KEY").unwrap();
    let wallet = PrivateKeySigner::from_str(&private_key).unwrap();
    println!("wallet: {}", wallet.address());

    let client = ExchangeClient::new(url).with_signer(wallet);

    let result = client.send_action(action).await.unwrap();

    println!("Set oracle result: {:?}", result);
}
