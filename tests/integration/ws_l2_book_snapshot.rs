//! `l2Book` and `bbo` coverage: the WebSocket stack passes through `isSnapshot` and payloads.
//!
//! Hyperliquid may tag messages with `isSnapshot` (see WS subscriptions docs). [`WsClient::next_message`]
//! does not filter these; they surface on [`WsMessage::L2Book`] and [`WsMessage::Bbo`].
//!
//! Live test (ignored by default), from the repo that vendors `hl-rs`:
//! `cargo test --manifest-path vendor/hl-rs/Cargo.toml --features ws --test ws_l2_book_snapshot -- --ignored --nocapture`
//!
//! Environment (live only):
//! - `HL_WS_URL` — WebSocket URL (default: [`hl_rs::MAINNET_WS_URL`])
//! - `HL_WS_L2_BOOK_COIN` — coin symbol for `l2Book` (default: `ETH`)
//! - `HL_WS_BBO_COIN` — coin symbol for `bbo` (default: `ETH`)

use std::time::Duration;

use hl_rs::{Subscription, WsClient, WsMessage, MAINNET_WS_URL};
use rust_decimal_macros::dec;

#[test]
fn l2_book_wire_passes_outer_is_snapshot_true() {
    let v = serde_json::json!({
        "channel": "l2Book",
        "data": {
            "coin": "ETH",
            "time": 1_712_764_800_000_u64,
            "levels": [
                [{ "px": "2999.5", "sz": "1.25", "n": 3 }],
                [{ "px": "3000.5", "sz": "2.5", "n": 5 }],
            ]
        },
        "isSnapshot": true
    });

    let m: WsMessage = serde_json::from_value(v).expect("parse WsMessage");

    match m {
        WsMessage::L2Book { data, is_snapshot } => {
            assert_eq!(
                is_snapshot,
                Some(true),
                "outer isSnapshot must not be dropped"
            );
            assert_eq!(data.coin, "ETH");
            assert_eq!(data.time, 1_712_764_800_000_u64);
            assert_eq!(data.levels[0].len(), 1);
            assert_eq!(data.levels[1].len(), 1);
            assert_eq!(data.levels[0][0].px, dec!(2999.5));
            assert_eq!(data.levels[0][0].sz, dec!(1.25));
            assert_eq!(data.levels[0][0].n, 3);
            assert_eq!(data.levels[1][0].px, dec!(3000.5));
            assert_eq!(data.levels[1][0].sz, dec!(2.5));
            assert_eq!(data.levels[1][0].n, 5);
        }
        other => panic!("expected L2Book, got {other:?}"),
    }
}

#[test]
fn l2_book_wire_passes_outer_is_snapshot_false() {
    let v = serde_json::json!({
        "channel": "l2Book",
        "data": {
            "coin": "BTC",
            "time": 2_u64,
            "levels": [[], []],
        },
        "isSnapshot": false
    });

    let m: WsMessage = serde_json::from_value(v).expect("parse WsMessage");

    match m {
        WsMessage::L2Book { is_snapshot, .. } => {
            assert_eq!(is_snapshot, Some(false));
        }
        other => panic!("expected L2Book, got {other:?}"),
    }
}

#[tokio::test]
#[ignore = "network: Hyperliquid WS; run with --ignored (optional: HL_WS_URL, HL_WS_L2_BOOK_COIN)"]
async fn l2_book_live_receives_book_and_preserves_is_snapshot() {
    let url = std::env::var("HL_WS_URL").unwrap_or_else(|_| MAINNET_WS_URL.to_string());
    let coin = std::env::var("HL_WS_L2_BOOK_COIN").unwrap_or_else(|_| "ETH".to_string());

    let mut client = WsClient::connect(&url)
        .await
        .expect("connect to HL_WS_URL / mainnet");

    client
        .subscribe(Subscription::L2Book {
            coin: coin.clone(),
            n_sig_figs: None,
            mantissa: None,
        })
        .await
        .expect("subscribe l2Book");

    let (data, is_snapshot) = tokio::time::timeout(Duration::from_secs(30), async {
        while let Some(res) = client.next_message().await {
            let m = res.expect("parse WsMessage");
            match m {
                WsMessage::SubscriptionResponse { .. } | WsMessage::Pong => continue,
                WsMessage::L2Book { data, is_snapshot } => return (data, is_snapshot),
                _ => continue,
            }
        }
        panic!("websocket closed before l2Book");
    })
    .await
    .expect("timed out waiting for l2Book");

    assert_eq!(data.coin, coin);
    assert!(
        !data.levels[0].is_empty() || !data.levels[1].is_empty(),
        "expected non-empty bid or ask side on live book"
    );
    println!("is_snapshot: {:?}", is_snapshot);
    // `next_message` uses the same JSON parse path as the wire-format tests above; the flag is
    // never stripped. For `l2Book`, Hyperliquid may omit `isSnapshot` or set it true on the first book.
    assert!(
        is_snapshot.is_none() || is_snapshot == Some(true),
        "unexpected is_snapshot={is_snapshot:?} on first l2Book after subscribe"
    );
}

#[test]
fn bbo_wire_passes_outer_is_snapshot_true() {
    let v = serde_json::json!({
        "channel": "bbo",
        "data": {
            "coin": "ETH",
            "time": 1_712_764_800_000_u64,
            "bbo": [
                { "px": "2999.5", "sz": "1.25", "n": 3 },
                { "px": "3000.5", "sz": "2.5", "n": 5 },
            ]
        },
        "isSnapshot": true
    });

    let m: WsMessage = serde_json::from_value(v).expect("parse WsMessage");

    match m {
        WsMessage::Bbo { data, is_snapshot } => {
            assert_eq!(
                is_snapshot,
                Some(true),
                "outer isSnapshot must not be dropped"
            );
            assert_eq!(data.coin, "ETH");
            assert_eq!(data.time, 1_712_764_800_000_u64);
            let bid = data.bbo[0].as_ref().expect("bid");
            let ask = data.bbo[1].as_ref().expect("ask");
            assert_eq!(bid.px, dec!(2999.5));
            assert_eq!(bid.sz, dec!(1.25));
            assert_eq!(bid.n, 3);
            assert_eq!(ask.px, dec!(3000.5));
            assert_eq!(ask.sz, dec!(2.5));
            assert_eq!(ask.n, 5);
        }
        other => panic!("expected Bbo, got {other:?}"),
    }
}

#[test]
fn bbo_wire_passes_outer_is_snapshot_false() {
    let v = serde_json::json!({
        "channel": "bbo",
        "data": {
            "coin": "BTC",
            "time": 2_u64,
            "bbo": [null, null],
        },
        "isSnapshot": false
    });

    let m: WsMessage = serde_json::from_value(v).expect("parse WsMessage");

    match m {
        WsMessage::Bbo { is_snapshot, .. } => {
            assert_eq!(is_snapshot, Some(false));
        }
        other => panic!("expected Bbo, got {other:?}"),
    }
}

#[tokio::test]
#[ignore = "network: Hyperliquid WS; run with --ignored (optional: HL_WS_URL, HL_WS_BBO_COIN)"]
async fn bbo_live_receives_bbo_and_preserves_is_snapshot() {
    let url = std::env::var("HL_WS_URL").unwrap_or_else(|_| MAINNET_WS_URL.to_string());
    let coin = std::env::var("HL_WS_BBO_COIN").unwrap_or_else(|_| "ETH".to_string());

    let mut client = WsClient::connect(&url)
        .await
        .expect("connect to HL_WS_URL / mainnet");

    client
        .subscribe(Subscription::Bbo { coin: coin.clone() })
        .await
        .expect("subscribe bbo");

    let (data, is_snapshot) = tokio::time::timeout(Duration::from_secs(30), async {
        while let Some(res) = client.next_message().await {
            let m = res.expect("parse WsMessage");
            match m {
                WsMessage::SubscriptionResponse { .. } | WsMessage::Pong => continue,
                WsMessage::Bbo { data, is_snapshot } => return (data, is_snapshot),
                _ => continue,
            }
        }
        panic!("websocket closed before bbo");
    })
    .await
    .expect("timed out waiting for bbo");

    assert_eq!(data.coin, coin);
    assert!(
        data.bbo[0].is_some() || data.bbo[1].is_some(),
        "expected non-null bid or ask on live bbo"
    );
    println!("is_snapshot: {:?}", is_snapshot);
    assert!(
        is_snapshot.is_none() || is_snapshot == Some(true),
        "unexpected is_snapshot={is_snapshot:?} on first bbo after subscribe"
    );
}
