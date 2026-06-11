//! Multisig spot transfer on testnet.
//!
//! Sends 0.01 HYPE from a 1-of-1 multisig account. The sole authorized signer
//! is also the outer signer that submits the wrapped `multiSig` action.
//!
//! # Environment
//! - `PRIVATE_KEY` - hex private key for the authorized / outer signer (required)

use std::str::FromStr;

use alloy::primitives::{address, Address};
use alloy::signers::local::PrivateKeySigner;
use hl_rs::{BaseUrl, ExchangeClient, SpotTransfer};
use rust_decimal_macros::dec;

/// Multisig account that holds spot funds.
const MULTI_SIG_USER: Address = address!("0x97878e89E37D1C565Af9213a49416585fdd17Ca5");
/// Sole authorized signer (1-of-1). `PRIVATE_KEY` must control this address.
const AUTHORIZED_SIGNER: Address = address!("0xE71EA6211e4956712c3BB3f36F9d0050bcD48BA0");
const DESTINATION: Address = address!("0x73c4c9fB0113f19D4d015E4ebb06cEaabc2b3CEA");
const HYPE_TOKEN: &str = "HYPE:0x7317beb7cceed72ef0b346074cc8e7ab";
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

    let mut action = SpotTransfer::new(DESTINATION, HYPE_TOKEN, dec!(0.01));
    action.nonce = Some(ExchangeClient::current_timestamp_ms());

    let client = ExchangeClient::new(BASE_URL).with_signer(wallet);

    let inner_signature = client
        .sign_multisig_inner_user_signed_action(action.clone(), MULTI_SIG_USER, signer_address)
        .expect("authorized signer should sign inner multisig payload");

    let result = client
        .send_multisig_action(action, MULTI_SIG_USER, signer_address, vec![inner_signature])
        .await
        .expect("multisig spot transfer failed");

    println!("multisig spot transfer result: {result:?}");
}
