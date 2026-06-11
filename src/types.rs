use std::collections::HashMap;

use alloy::primitives::B128;
use serde::Deserialize;

use crate::{MAINNET_API_URL, MAINNET_WS_URL, TESTNET_API_URL, TESTNET_WS_URL};

#[derive(Debug, Clone)]
pub enum BaseUrl {
    Testnet,
    Mainnet,
    Custom {
        url: String,
        signing_chain: SigningChain,
    },
}

#[derive(Debug, Clone)]
pub enum SigningChain {
    Mainnet,
    Testnet,
    /// Custom SigningChain for accepting core-style signatures without replay risks.
    #[cfg(feature = "custom-signing-chain")]
    Custom {
        source: String,
        hyperliquid_chain: String,
        signature_chain_id: u64,
    },
}

impl SigningChain {
    pub(crate) fn get_source(&self) -> String {
        match self {
            SigningChain::Testnet => "b".to_string(),
            SigningChain::Mainnet => "a".to_string(),
            #[cfg(feature = "custom-signing-chain")]
            SigningChain::Custom { source, .. } => source.to_owned(),
        }
    }

    pub(crate) fn get_hyperliquid_chain(&self) -> String {
        match self {
            SigningChain::Testnet => "Testnet".to_string(),
            SigningChain::Mainnet => "Mainnet".to_string(),
            #[cfg(feature = "custom-signing-chain")]
            SigningChain::Custom {
                hyperliquid_chain, ..
            } => hyperliquid_chain.to_owned(),
        }
    }
    pub(crate) fn get_signature_chain_id(&self) -> u64 {
        match self {
            SigningChain::Testnet => 421614,
            SigningChain::Mainnet => 42161,
            #[cfg(feature = "custom-signing-chain")]
            SigningChain::Custom {
                signature_chain_id, ..
            } => *signature_chain_id,
        }
    }
}
impl BaseUrl {
    /// Get the url for the base url.
    pub fn get_url(&self) -> String {
        match self {
            BaseUrl::Custom { url, .. } => url.to_owned(),
            BaseUrl::Mainnet => MAINNET_API_URL.to_string(),
            BaseUrl::Testnet => TESTNET_API_URL.to_string(),
        }
    }

    /// Get the WebSocket URL (https → wss, append /ws).
    pub fn ws_url(&self) -> String {
        match self {
            BaseUrl::Custom { url, .. } => {
                url.replace("https://", "wss://")
                    .replace("http://", "ws://")
                    .trim_end_matches('/')
                    .to_string()
                    + "/ws"
            }
            BaseUrl::Mainnet => MAINNET_WS_URL.to_string(),
            BaseUrl::Testnet => TESTNET_WS_URL.to_string(),
        }
    }

    /// Get the signing chain for the base url.
    pub fn get_signing_chain(&self) -> &SigningChain {
        match self {
            BaseUrl::Custom { signing_chain, .. } => signing_chain,
            BaseUrl::Mainnet => &SigningChain::Mainnet,
            BaseUrl::Testnet => &SigningChain::Testnet,
        }
    }

    /// Get the signature chain id for the base url.
    pub fn get_signature_chain_id(&self) -> u64 {
        self.get_signing_chain().get_signature_chain_id()
    }

    /// Get the source for signing l1 actions.
    pub fn get_source(&self) -> String {
        self.get_signing_chain().get_source()
    }

    /// Get the hyperliquid chain for signing user signed actions.
    pub fn get_hyperliquid_chain(&self) -> String {
        self.get_signing_chain().get_hyperliquid_chain()
    }
}

#[derive(Debug, Clone)]
pub struct CoinToAsset(HashMap<String, u32>);

impl CoinToAsset {
    pub fn new(mapping: HashMap<String, u32>) -> Self {
        Self(mapping)
    }

    pub fn as_map(&self) -> &HashMap<String, u32> {
        &self.0
    }

    pub fn into_map(self) -> HashMap<String, u32> {
        self.0
    }
}

#[derive(Deserialize, Debug, Clone)]
pub struct Meta {
    pub universe: Vec<AssetMeta>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct SpotMeta {
    pub universe: Vec<SpotAssetMeta>,
    pub tokens: Vec<TokenInfo>,
}

impl SpotMeta {
    pub fn add_pair_and_name_to_index_map(
        &self,
        mut coin_to_asset: HashMap<String, u32>,
    ) -> HashMap<String, u32> {
        let index_to_name: HashMap<usize, &str> = self
            .tokens
            .iter()
            .map(|info| (info.index, info.name.as_str()))
            .collect();

        for asset in self.universe.iter() {
            let spot_ind: u32 = 10000 + asset.index as u32;
            let name_to_ind = (asset.name.clone(), spot_ind);

            let Some(token_1_name) = index_to_name.get(&asset.tokens[0]) else {
                continue;
            };

            let Some(token_2_name) = index_to_name.get(&asset.tokens[1]) else {
                continue;
            };

            coin_to_asset.insert(format!("{token_1_name}/{token_2_name}"), spot_ind);
            coin_to_asset.insert(name_to_ind.0, name_to_ind.1);
        }

        coin_to_asset
    }
}

#[derive(Deserialize, Debug, Clone)]
#[serde(untagged)]
pub enum SpotMetaAndAssetCtxs {
    SpotMeta(SpotMeta),
    Context(Vec<SpotAssetContext>),
}

#[derive(Deserialize, Debug, Clone)]
#[serde(untagged)]
pub enum MetaAndAssetCtxs {
    Meta(Meta),
    Context(Vec<AssetContext>),
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SpotAssetContext {
    pub day_ntl_vlm: String,
    pub mark_px: String,
    pub mid_px: Option<String>,
    pub prev_day_px: String,
    pub circulating_supply: String,
    pub coin: String,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct AssetContext {
    pub day_ntl_vlm: String,
    pub funding: String,
    pub impact_pxs: Option<Vec<String>>,
    pub mark_px: String,
    pub mid_px: Option<String>,
    pub open_interest: String,
    pub oracle_px: String,
    pub premium: Option<String>,
    pub prev_day_px: String,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct AssetMeta {
    pub name: String,
    pub sz_decimals: u32,
    pub max_leverage: usize,
    #[serde(default)]
    pub only_isolated: Option<bool>,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SpotAssetMeta {
    pub tokens: [usize; 2],
    pub name: String,
    pub index: usize,
    pub is_canonical: bool,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct TokenInfo {
    pub name: String,
    pub sz_decimals: u8,
    pub wei_decimals: u8,
    pub index: usize,
    pub token_id: B128,
    pub is_canonical: bool,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct PerpDex {
    /// Protocol perp-dex index. The default dex is index 0 and is not returned.
    /// Deployed dexes start at 1.
    #[serde(skip_deserializing, default)]
    pub id: u32,
    pub name: String,
    pub full_name: String,
    pub deployer: String,
    pub deployer_fee_scale: String,
    pub fee_recipient: Option<String>,
    pub oracle_updater: Option<String>,
    pub asset_to_streaming_oi_cap: Vec<(String, String)>,
    pub last_deployer_fee_scale_change_time: String,
    // pub sub_deployers: Vec<Vec<String>>,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct PerpDexStatus {
    pub total_net_deposit: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserStakingSummary {
    pub delegated: String,
    pub undelegated: String,
    pub total_pending_withdrawal: String,
    pub n_pending_withdrawals: u64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PerpDeployAuctionStatus {
    pub start_time_seconds: u64,
    pub duration_seconds: u64,
    pub start_gas: String,
    pub current_gas: Option<String>,
    pub end_gas: Option<String>,
}
