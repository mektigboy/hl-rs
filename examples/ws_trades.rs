//! Connect to Hyperliquid WebSocket, subscribe to public trades for a coin, print a few messages.
//!
//! Run (requires network):
//! ```text
//! cargo run --example ws_trades --features ws -- SOL
//! ```
//!
//! CI: use `#[ignore]` integration style; this example is for manual verification.

use hl_rs::{Subscription, WsClient, MAINNET_WS_URL};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let coin = std::env::args().nth(1).unwrap_or_else(|| "SOL".to_string());

    let mut client = WsClient::connect(MAINNET_WS_URL).await?;

    client
        .subscribe(Subscription::Trades { coin: coin.clone() })
        .await?;

    println!("subscribed to trades for {coin}; waiting for messages (Ctrl+C to stop)…");

    for _ in 0..8 {
        match client.next_message().await {
            Some(Ok(msg)) => println!("{msg:?}"),
            Some(Err(e)) => eprintln!("error: {e}"),
            None => break,
        }
    }

    client.unsubscribe(Subscription::Trades { coin }).await?;

    Ok(())
}
