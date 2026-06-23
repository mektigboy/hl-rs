//! Multisig spot transfer using external-signer hash helpers (KMS/HSM/enclave flow).
//!
//! Mirrors launch-stack's two-step multisig signing without holding the key in-process
//! for the inner signature. This example still uses a local signer to simulate the
//! external signer signing the returned hashes.
//!
//! # Environment
//! - `PRIVATE_KEY` - hex private key for the authorized / outer signer (required)

use std::str::FromStr;

use alloy::primitives::{address, Address};
use alloy::signers::local::PrivateKeySigner;
use alloy_signer::SignerSync;
use hl_rs::{
    assemble_signed_multisig_action, build_multisig_action, multisig_inner_user_signed_signing_hash,
    multisig_outer_signing_hash, BaseUrl, ExchangeClient, SpotTransfer,
};
use rust_decimal_macros::dec;

const MULTI_SIG_USER: Address = address!("0x97878e89E37D1C565Af9213a49416585fdd17Ca5");
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

    let action = SpotTransfer::new(DESTINATION, HYPE_TOKEN, dec!(0.01));
    let signing_chain = BASE_URL.get_signing_chain();

    // Step 1: compute inner hash (external signer signs inner_signing_hash)
    let (action, inner_hashes) = multisig_inner_user_signed_signing_hash(
        action,
        MULTI_SIG_USER,
        signer_address,
        &signing_chain,
    )
    .expect("inner hash computation failed");
    let inner_signature = wallet
        .sign_hash_sync(&inner_hashes.inner_signing_hash)
        .expect("inner signature failed");

    // Step 2: build wrapped multiSig action
    let wrapped = build_multisig_action(
        &action,
        MULTI_SIG_USER,
        signer_address,
        vec![inner_signature],
        &signing_chain,
    )
    .expect("build multisig action failed");

    // Step 3: compute outer hash (external signer signs outer hash)
    let outer_hash = multisig_outer_signing_hash(
        &action,
        MULTI_SIG_USER,
        signer_address,
        &wrapped,
        &signing_chain,
        inner_hashes.nonce,
        None,
        None,
    )
    .expect("outer hash computation failed");
    let outer_signature = wallet
        .sign_hash_sync(&outer_hash)
        .expect("outer signature failed");

    // Step 4: assemble and submit
    let signed =
        assemble_signed_multisig_action(wrapped, inner_hashes.nonce, outer_signature, None, None);
    let client = ExchangeClient::new(BASE_URL);
    let result = client
        .send_signed_multisig_action(signed)
        .await
        .expect("multisig spot transfer failed");

    println!("multisig spot transfer (external signer flow) result: {result:?}");
}
