//! Hyperliquid WebSocket client ([subscriptions](https://hyperliquid.gitbook.io/hyperliquid-docs/for-developers/api/websocket/subscriptions)).
//!
//! Connect with [`WsClient::connect`] or [`WsClient::connect_with_base_url`], then
//! [`WsClient::subscribe`] / [`WsClient::unsubscribe`]. Read frames with [`WsClient::next_message`].
//!
//! **Reconnect:** The API may disconnect without notice. Callers should reconnect and resubscribe;
//! snapshot acks on resubscribe include data missed while disconnected.

mod message;
pub mod responses;
mod subscription;

pub use message::WsMessage;
pub use responses::{
    AllMids, Candle, Notification, OpenOrders, PerpsAssetCtx, Price, SpotAssetCtx, TwapState,
    TwapStates, UserBalance, WebData3, WsActiveAssetCtx, WsActiveAssetData, WsAllDexsAssetCtxs,
    WsAllDexsClearinghouseState, WsBasicOrder, WsBbo, WsBook, WsFill, WsLevel, WsLiquidation,
    WsNonUserCancel, WsOrder, WsSpotState, WsTrade, WsUserEvent, WsUserFunding, WsUserFundings,
    WsUserNonFundingLedgerUpdate, WsUserNonFundingLedgerUpdates, WsUserTwapHistory,
    WsUserTwapSliceFills,
};
pub use subscription::Subscription;

use futures_util::{SinkExt, StreamExt};
use message::WsRequest;
use tokio::net::TcpStream;
use tokio_tungstenite::{
    connect_async, tungstenite::protocol::Message, MaybeTlsStream, WebSocketStream,
};

use crate::error::Error;
use crate::prelude::Result;
use crate::types::BaseUrl;

type HlWsStream = WebSocketStream<MaybeTlsStream<TcpStream>>;

/// WebSocket client for Hyperliquid streaming feeds.
pub struct WsClient {
    write: futures_util::stream::SplitSink<HlWsStream, Message>,
    read: futures_util::stream::SplitStream<HlWsStream>,
}

impl WsClient {
    /// Connect to `wss://.../ws` (or `ws://` for local dev).
    pub async fn connect(url: &str) -> Result<Self> {
        let (ws, _) = connect_async(url)
            .await
            .map_err(|e| Error::WsConnect(e.to_string()))?;
        let (write, read) = ws.split();
        Ok(Self { write, read })
    }

    /// Connect using [`BaseUrl::ws_url`].
    pub async fn connect_with_base_url(base: &BaseUrl) -> Result<Self> {
        Self::connect(base.ws_url().as_str()).await
    }

    /// Subscribe to a feed.
    pub async fn subscribe(&mut self, sub: Subscription) -> Result<()> {
        let json = serde_json::to_string(&WsRequest::subscribe(sub))
            .map_err(|e| Error::SerializationFailure(e.to_string()))?;
        self.write
            .send(Message::Text(json.into()))
            .await
            .map_err(|e| Error::WsSend(e.to_string()))?;
        Ok(())
    }

    /// Unsubscribe from a feed (subscription object must match the original subscribe).
    pub async fn unsubscribe(&mut self, sub: Subscription) -> Result<()> {
        let json = serde_json::to_string(&WsRequest::unsubscribe(sub))
            .map_err(|e| Error::SerializationFailure(e.to_string()))?;
        self.write
            .send(Message::Text(json.into()))
            .await
            .map_err(|e| Error::WsSend(e.to_string()))?;
        Ok(())
    }

    /// Send keepalive ping (server responds with `channel: "pong"`).
    pub async fn ping(&mut self) -> Result<()> {
        let json = serde_json::to_string(&WsRequest::ping())
            .map_err(|e| Error::SerializationFailure(e.to_string()))?;
        self.write
            .send(Message::Text(json.into()))
            .await
            .map_err(|e| Error::WsSend(e.to_string()))?;
        Ok(())
    }

    /// Receive the next JSON message as [`WsMessage`].
    ///
    /// Skips the plain-text `"Websocket connection established."` handshake line.
    /// Handles WebSocket ping frames by replying with pong.
    pub async fn next_message(&mut self) -> Option<Result<WsMessage>> {
        while let Some(item) = self.read.next().await {
            match item {
                Ok(Message::Text(t)) => {
                    if t == "Websocket connection established." {
                        continue;
                    }
                    return Some(
                        serde_json::from_str::<WsMessage>(&t)
                            .map_err(|e| Error::JsonParse(e.to_string())),
                    );
                }
                Ok(Message::Ping(payload)) => {
                    let _ = self.write.send(Message::Pong(payload)).await;
                    continue;
                }
                Ok(Message::Close(_)) => return None,
                Ok(Message::Pong(_)) | Ok(Message::Frame(_)) => continue,
                Ok(Message::Binary(b)) => {
                    let Ok(s) = String::from_utf8(b.to_vec()) else {
                        return Some(Err(Error::JsonParse(
                            "binary frame is not valid UTF-8".to_string(),
                        )));
                    };
                    return Some(
                        serde_json::from_str::<WsMessage>(&s)
                            .map_err(|e| Error::JsonParse(e.to_string())),
                    );
                }
                Err(e) => return Some(Err(Error::WsReceive(e.to_string()))),
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn subscription_all_mids_json() {
        let s = Subscription::AllMids { dex: None };
        let v = serde_json::to_value(&s).unwrap();
        assert_eq!(v["type"], "allMids");
    }

    #[test]
    fn subscription_l2book_optional_fields_omitted() {
        let s = Subscription::L2Book {
            coin: "ETH".to_string(),
            n_sig_figs: None,
            mantissa: None,
        };
        let json = serde_json::to_string(&s).unwrap();
        assert!(!json.contains("nSigFigs"));
    }
}
