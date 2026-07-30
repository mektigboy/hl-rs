//! Approve a builder fee cap, then place an order that attributes volume to that builder.
//!
//! Hyperliquid expects `maxFeeRate` in the EIP-712 payload as a string; this SDK stores it
//! as a [`rust_decimal::Decimal`] (see [`hl_rs::ApproveBuilderFee`]). The order `builder.f`
//! field is the fee in **tenths of a basis point** (see Hyperliquid exchange docs): `f = 10`
//! charges **1 bp** of notional to the builder. Keep `f` within the approved cap.
//!
//! # Environment
//!
//! - `PRIVATE_KEY` — hex private key for the trading wallet (required).
//! - `BUILDER_ADDRESS` — builder address to approve and attach to the order (required).
//!
//! # Optional
//!
//! - `ASSET_INDEX` — asset id (default: `11338` spot on testnet, same as `place_order` example).
//! - `BUILDER_F` — builder fee in tenths of a basis point (default: `10` → 1 bp). Must stay
//!   under the approved `MAX_FEE_RATE` cap.
//! - `MAX_FEE_RATE` — decimal string for `ApproveBuilderFee::max_fee_rate` (default `0.001`).

use std::str::FromStr;

use alloy::primitives::{address, Address};
use alloy::signers::local::PrivateKeySigner;
use hl_rs::{
    ApproveBuilderFee, BaseUrl, BatchOrder, BuilderInfo, ExchangeClient, LimitOrderType, OrderType,
    OrderWire, Tif,
};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;

#[tokio::main]
async fn main() {
    dotenv::dotenv().ok();

    let private_key = std::env::var("PRIVATE_KEY").expect("PRIVATE_KEY must be set");
    let wallet = PrivateKeySigner::from_str(&private_key).expect("invalid PRIVATE_KEY");
    let builder = address!("0x0ef4B52b87ddcB520009cCb57cfe83B8c36b3955");

    let asset: u32 = std::env::var("ASSET_INDEX")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(10_000 + 1338);

    let builder_f: u32 = 10;
    let max_fee_rate = dec!(10);

    let url = BaseUrl::Testnet;
    let wallet_addr = wallet.address();
    let client = ExchangeClient::new(url).with_signer(wallet);

    println!("wallet: {wallet_addr}");
    println!("builder: {builder:#x}");

    let approve = ApproveBuilderFee::new(max_fee_rate, builder);
    let approve_resp = client
        .send_action(approve)
        .await
        .expect("approveBuilderFee failed");
    println!("approveBuilderFee: {approve_resp:?}");

    // Hyperliquid has no separate “market” wire type: use an IOC limit with a price far
    // through the book so it takes liquidity immediately (here, a small buy capped at 10).
    let order = OrderWire {
        asset,
        is_buy: true,
        limit_px: dec!(10),
        size: dec!(1),
        reduce_only: false,
        order_type: OrderType::Limit(LimitOrderType { tif: Tif::Ioc }),
        client_order_id: None,
    };

    let builder_info = BuilderInfo {
        b: builder.to_string().to_lowercase(),
        f: builder_f,
    };

    let batch = BatchOrder::new(vec![order]).with_builder(builder_info);
    let order_resp = client
        .send_action(batch)
        .await
        .expect("order with builder failed");
    println!("order: {order_resp:?}");
}
