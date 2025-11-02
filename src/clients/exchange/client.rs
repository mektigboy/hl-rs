use std::collections::HashMap;

use alloy::primitives::Address;

use crate::{BaseUrl, http::HttpClient, prelude::Result};

#[derive(Debug, Clone)]
pub struct ExchangeClient {
    http_client: HttpClient,
    vault_address: Option<Address>,
    coin_to_asset: HashMap<String, u32>,
}

impl ExchangeClient {
    pub fn new(
        base_url: Option<BaseUrl>,
        vault_address: Option<Address>,
        coin_to_asset: HashMap<String, u32>,
    ) -> Result<Self> {
        let base_url = base_url.unwrap_or(BaseUrl::Mainnet);

        Ok(Self {
            http_client: HttpClient {
                client: reqwest::Client::default(),
                base_url: base_url.get_url(),
            },
            vault_address,
            coin_to_asset,
        })
    }

    pub(crate) fn vault_address(&self) -> Option<Address> {
        self.vault_address
    }

    pub(crate) fn is_mainnet(&self) -> bool {
        self.http_client.is_mainnet()
    }

    pub(crate) fn http_client(&self) -> &HttpClient {
        &self.http_client
    }

    pub fn coin_to_asset(&self) -> &HashMap<String, u32> {
        &self.coin_to_asset
    }
}
