//! Submit and then cancel an L1 order via a 1-of-1 multisig, executing on a
//! subaccount (vault).
//!
//! Demonstrates the multisig + `vaultAddress` path: a multisig master places an
//! order on its subaccount, signed inner+outer by the sole authorized signer. The
//! vault address is folded into both the inner and outer action hashes and emitted
//! as `vaultAddress` in the request body.
//!
//! # Setup (testnet)
//! - `MULTI_SIG_USER` must be a converted 1-of-1 multisig whose only authorized user
//!   is `AUTHORIZED_SIGNER`.
//! - `VAULT` must be a subaccount of `MULTI_SIG_USER` and hold enough quote balance
//!   for the order below.
//! - `PRIVATE_KEY` must control `AUTHORIZED_SIGNER` (it is also the outer signer).
//!
//! To place a plain multisig order on the multisig account itself (no subaccount),
//! drop the `.with_vault_address(VAULT)` call.
//!
//! # Environment
//! - `PRIVATE_KEY` - hex private key for the authorized / outer signer (required)

use std::str::FromStr;

use alloy::primitives::{address, Address};
use alloy::signers::local::PrivateKeySigner;
use hl_rs::clients::exchange::responses::ExchangeDataStatus;
use hl_rs::{
    BaseUrl, BatchCancel, BatchOrder, ExchangeClient, LimitOrderType, OrderType, OrderWire, Tif,
};
use rust_decimal_macros::dec;
/// Multisig account (the multisig user / order owner).
const MULTI_SIG_USER: Address = address!("0x643e6e308c84CF718d89f03E4F6096cEB4FbE4a0");
/// Sole authorized signer (1-of-1), also the outer signer. `PRIVATE_KEY` controls it.
const AUTHORIZED_SIGNER: Address = address!("0x97878e89E37D1C565Af9213a49416585fdd17Ca5");
/// Subaccount of `MULTI_SIG_USER` to execute the order on.
const VAULT: Address = address!("0x4f819b040838b269adecdb7df34a8ec9997f27ba");
/// USDH/USDC spot on testnet (`10_000 + spot pair index`).
const ASSET: u32 = 10_000 + 1338;
const BASE_URL: BaseUrl = BaseUrl::Testnet;

#[tokio::main]
async fn main() {
    dotenv::dotenv().ok();

    let private_key = std::env::var("PRIVATE_KEY").expect("PRIVATE_KEY must be set");
    let wallet = PrivateKeySigner::from_str(&private_key).expect("invalid PRIVATE_KEY");
    let signer_address = wallet.address();
    assert_eq!(
        signer_address, AUTHORIZED_SIGNER,
        "PRIVATE_KEY must control the sole authorized signer {AUTHORIZED_SIGNER:?}"
    );

    // The client carries the vault: multisig signing folds it into both hashes and
    // the request body.
    let client = ExchangeClient::new(BASE_URL)
        .with_signer(wallet)
        .with_vault_address(VAULT);

    // --- Submit a resting buy limit order via multisig ---
    let order = OrderWire {
        asset: ASSET,
        is_buy: true,
        // Far below market so it rests (and can be cancelled) rather than filling.
        limit_px: dec!(0.5),
        size: dec!(20.0),
        reduce_only: false,
        order_type: OrderType::Limit(LimitOrderType { tif: Tif::Gtc }),
        client_order_id: None,
    };
    let mut place = BatchOrder::new(vec![order]);
    // Pin the nonce so the inner signature and the outer submission agree.
    place.nonce = Some(ExchangeClient::current_timestamp_ms());

    let inner_signature = client
        .sign_multisig_inner_action(place.clone(), MULTI_SIG_USER, signer_address)
        .expect("authorized signer should sign inner order payload");
    let place_result = client
        .send_multisig_action(place, MULTI_SIG_USER, signer_address, vec![inner_signature])
        .await
        .expect("multisig order failed");
    println!("multisig order result: {place_result:?}");

    // --- Find the resting order id ---
    let oid = place_result
        .order_data()
        .and_then(|data| {
            data.statuses.into_iter().find_map(|status| match status {
                ExchangeDataStatus::Resting(resting) => Some(resting.oid),
                _ => None,
            })
        })
        .expect("order did not rest; nothing to cancel");
    println!("resting oid: {oid}");

    // --- Cancel that order via multisig ---
    let mut cancel = BatchCancel::single(ASSET, oid);
    cancel.nonce = Some(ExchangeClient::current_timestamp_ms());

    let inner_signature = client
        .sign_multisig_inner_action(cancel.clone(), MULTI_SIG_USER, signer_address)
        .expect("authorized signer should sign inner cancel payload");
    let cancel_result = client
        .send_multisig_action(cancel, MULTI_SIG_USER, signer_address, vec![inner_signature])
        .await
        .expect("multisig cancel failed");
    println!("multisig cancel result: {cancel_result:?}");
}
