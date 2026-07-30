//! `activeAssetCtx` coverage: JSON round-trip and optional live WebSocket receive.
//!
//! Live test (ignored by default): `cargo test -p hl-rs --features ws ws_active_asset_ctx -- --ignored --nocapture`
//!
//! Environment (live only):
//! - `HL_WS_URL` — WebSocket URL (default: [`hl_rs::MAINNET_WS_URL`])
//! - `HL_WS_ACTIVE_ASSET_CTX_COIN` — perp/spot coin id (default: `BTC`)

use std::time::Duration;

use hl_rs::{Subscription, WsActiveAssetCtx, WsClient, WsMessage, MAINNET_WS_URL};
use rust_decimal_macros::dec;

#[test]
fn ws_active_asset_ctx_decodes_km_mu() {
    // Representative payload based on Hyperliquid WS docs `activeAssetCtx`.
    // Coin includes the `km:` prefix (as used by the bot for some products).
    let v = serde_json::json!({
        "channel": "activeAssetCtx",
        "data": {
            "coin": "km:MU",
            "ctx": {
                "dayNtlVlm": "123456.78",
                "prevDayPx": "10.50",
                "markPx": "10.75",
                "midPx": "10.74",
                "funding": "0.0000123",
                "openInterest": "98765.4321",
                "oraclePx": "10.73"
            }
        },
        "isSnapshot": true
    });

    let m: WsMessage = serde_json::from_value(v).expect("parse WsMessage");

    match m {
        WsMessage::ActiveAssetCtx { data, is_snapshot } => {
            assert_eq!(is_snapshot, Some(true));
            match data {
                WsActiveAssetCtx::Perp(p) => {
                    assert_eq!(p.coin, "km:MU");
                    assert_eq!(p.ctx.funding, dec!(0.0000123));
                    assert_eq!(p.ctx.shared.day_ntl_vlm, dec!(123456.78));
                    assert_eq!(p.ctx.open_interest, dec!(98765.4321));
                    assert_eq!(p.ctx.shared.mark_px, dec!(10.75));
                    assert_eq!(p.ctx.oracle_px, dec!(10.73));
                    assert_eq!(p.ctx.shared.mid_px, Some(dec!(10.74)));
                }
                WsActiveAssetCtx::Spot(_) => panic!("expected Perp ctx"),
            }
        }
        other => panic!("expected ActiveAssetCtx, got {other:?}"),
    }
}

#[tokio::test]
#[ignore = "network: Hyperliquid WS; run with --ignored (optional: HL_WS_URL, HL_WS_ACTIVE_ASSET_CTX_COIN)"]
async fn ws_active_asset_ctx_live_receives_message() {
    let url = std::env::var("HL_WS_URL").unwrap_or_else(|_| MAINNET_WS_URL.to_string());
    let coin = std::env::var("HL_WS_ACTIVE_ASSET_CTX_COIN").unwrap_or_else(|_| "km:MU".to_string());

    let mut client = WsClient::connect(&url)
        .await
        .expect("connect to HL_WS_URL / mainnet");

    client
        .subscribe(Subscription::ActiveAssetCtx { coin: coin.clone() })
        .await
        .expect("subscribe activeAssetCtx");

    let data = tokio::time::timeout(Duration::from_secs(30), async {
        while let Some(res) = client.next_message().await {
            let m = res.expect("parse WsMessage");
            match m {
                WsMessage::SubscriptionResponse { .. } | WsMessage::Pong => continue,
                WsMessage::ActiveAssetCtx { data, .. } => return data,
                _ => continue,
            }
        }
        panic!("websocket closed before activeAssetCtx");
    })
    .await
    .expect("timed out waiting for activeAssetCtx");

    match data {
        WsActiveAssetCtx::Perp(p) => assert_eq!(p.coin, coin),
        WsActiveAssetCtx::Spot(s) => assert_eq!(s.coin, coin),
    }
}
