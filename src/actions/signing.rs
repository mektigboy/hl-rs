use alloy::{
    dyn_abi::Eip712Domain,
    primitives::{keccak256, Address, B256},
    sol,
    sol_types::{eip712_domain, SolStruct},
};
use serde::Serialize;

use crate::{Error, SigningChain};

/// Metadata passed during action signing
#[derive(Debug, Clone)]
pub struct SigningMeta<'a> {
    pub nonce: u64,
    pub vault_address: Option<Address>,
    pub expires_after: Option<u64>,
    pub signing_chain: &'a SigningChain,
}

// EIP-712 Agent struct for L1 action signing
sol! {
    #[derive(Debug)]
    struct Agent {
        string source;
        bytes32 connectionId;
    }
}

fn agent_domain() -> Eip712Domain {
    eip712_domain! {
        name: "Exchange",
        version: "1",
        chain_id: 1337,
        verifying_contract: Address::ZERO,
    }
}

pub(crate) fn agent_signing_hash(connection_id: B256, source: &str) -> B256 {
    let agent = Agent {
        source: source.to_string(),
        connectionId: connection_id,
    };

    let domain_hash = agent_domain().hash_struct();
    let struct_hash = agent.eip712_hash_struct();

    let mut digest = [0u8; 66];
    digest[0] = 0x19;
    digest[1] = 0x01;
    digest[2..34].copy_from_slice(&domain_hash[..]);
    digest[34..66].copy_from_slice(&struct_hash[..]);

    keccak256(digest)
}

pub(crate) fn compute_l1_hash<T: Serialize>(
    action: &T,
    nonce: u64,
    vault_address: Option<Address>,
    expires_after: Option<u64>,
) -> Result<B256, Error> {
    let mut bytes = rmp_serde::to_vec_named(action).map_err(|e| Error::RmpParse(e.to_string()))?;

    bytes.extend(nonce.to_be_bytes());

    match vault_address {
        Some(addr) => {
            bytes.push(1);
            bytes.extend(addr.as_slice());
        }
        None => bytes.push(0),
    }

    if let Some(expires) = expires_after {
        bytes.push(0);
        bytes.extend(expires.to_be_bytes());
    }

    Ok(keccak256(&bytes))
}

#[cfg(test)]
mod tests {
    use alloy::signers::{local::PrivateKeySigner, SignerSync};
    use rust_decimal_macros::dec;

    use super::SigningMeta;
    use crate::actions::{Action, UsdSend};
    use crate::SigningChain;

    #[test]
    fn test_usd_send_signing_hardcoded() {
        let wallet: PrivateKeySigner =
            "e908f86dbb4d55ac876378565aafeabc187f6690f046459397b17d9b9a19688e"
                .parse()
                .unwrap();
        let destination = "0x0d1d9635d0640821d15e323ac8adadfa9c111414"
            .parse()
            .unwrap();
        let action = UsdSend {
            destination,
            amount: dec!(1.0),
            nonce: Some(1700000000000),
        };
        let signing_chain = SigningChain::Testnet;
        let meta = SigningMeta {
            nonce: 0,
            vault_address: None,
            expires_after: None,
            signing_chain: &signing_chain,
        };

        let signing_hash = action.signing_hash(&meta).unwrap();
        let signature = wallet.sign_hash_sync(&signing_hash).unwrap();
        let actual_sig = signature.to_string();

        let expected_sig = "0xd4ad4602ed7f14be3960934f4e2ed205f0f24539fc467a606999719a0f13c384521699107322321581048d35fed5d3272eb913377a42940299922c9a854fad281b";
        assert_eq!(actual_sig, expected_sig);
        assert_eq!(
            format!("{:#x}", signature.r()).to_lowercase(),
            "0xd4ad4602ed7f14be3960934f4e2ed205f0f24539fc467a606999719a0f13c384"
        );
        assert_eq!(
            format!("{:#x}", signature.s()).to_lowercase(),
            "0x521699107322321581048d35fed5d3272eb913377a42940299922c9a854fad28"
        );
        let v = if signature.v() { 28 } else { 27 };
        assert_eq!(v, 27);
    }

    #[test]
    fn test_user_set_abstraction_signing_hardcoded() {
        use alloy::signers::{local::PrivateKeySigner, SignerSync};

        use crate::actions::{AbstractionMode, UserSetAbstraction};

        let wallet: PrivateKeySigner =
            "e908f86dbb4d55ac876378565aafeabc187f6690f046459397b17d9b9a19688e"
                .parse()
                .unwrap();
        let signing_chain = SigningChain::Mainnet;
        let prepared = crate::actions::PreparedAction::new(
            UserSetAbstraction {
                user: wallet.address(),
                abstraction: AbstractionMode::Disabled,
                nonce: Some(1700000000000),
            },
            &signing_chain,
            None,
            None,
        )
        .unwrap();
        let signed = prepared.sign(&wallet).unwrap();

        let expected_sig = "0x5e9d2fcbc9f4cecf997dd7ad72dabf096c2c772aacdedfe4fe558948bc504989026be4dfd8bab24982eca3cac8fb5a1cc2f5bbcf95af94f59da079653a86c7141c";
        assert_eq!(signed.signature.to_string(), expected_sig);
    }
}
