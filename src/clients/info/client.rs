use alloy::primitives::Address;
use serde::Deserialize;

use crate::{
    error::ApiError,
    http::HttpClient,
    info::{
        client_builder::InfoClientBuilder,
        types::{
            InfoRequest, MetaAndAssetCtxsResponse, UserRateLimit, UserRoleResponse,
            UserStateResponse, UserToMultiSigSignersResponse,
        },
    },
    prelude::{Error, Result},
    types::{Meta, PerpDeployAuctionStatus, PerpDex, PerpDexStatus, SpotMeta, UserStakingSummary},
    BaseUrl,
};

#[derive(Debug, Clone)]
pub struct InfoClient {
    pub(crate) http_client: HttpClient,
}

impl InfoClient {
    pub fn builder(base_url: BaseUrl) -> InfoClientBuilder {
        InfoClientBuilder::new(base_url)
    }

    pub async fn send_request<T: for<'a> Deserialize<'a>>(
        &self,
        info_request: InfoRequest,
    ) -> Result<T> {
        let return_data = self.http_client.post("/info", &info_request).await?;
        serde_json::from_str(&return_data).map_err(|e| Error::JsonParse(e.to_string()))
    }

    pub async fn meta(&self) -> Result<Meta> {
        self.send_request(InfoRequest::Meta).await
    }

    /// Retrieve perpetual metadata and asset contexts.
    ///
    /// Pass `dex` to query a HIP-3 dex; omit it for the default clearinghouse.
    ///
    /// See [Retrieve perpetuals metadata (universe and margin tables)](https://hyperliquid.gitbook.io/hyperliquid-docs/for-developers/api/info-endpoint/perpetuals#retrieve-perpetuals-metadata-universe-and-margin-tables).
    pub async fn meta_and_asset_ctxs(
        &self,
        dex: Option<&str>,
    ) -> Result<MetaAndAssetCtxsResponse> {
        self.send_request(InfoRequest::MetaAndAssetCtxs {
            dex: dex.map(str::to_string),
        })
        .await
    }

    pub async fn spot_meta(&self) -> Result<SpotMeta> {
        self.send_request(InfoRequest::SpotMeta).await
    }

    pub async fn user_staking_summary(&self, user: &Address) -> Result<UserStakingSummary> {
        self.send_request(InfoRequest::UserStakingSummary {
            user: user.to_owned(),
        })
        .await
    }

    pub async fn perp_dexs(&self) -> Result<Vec<PerpDex>> {
        use serde_json::Value;

        let response: Vec<Value> = self.send_request(InfoRequest::PerpDexs).await?;

        if response.is_empty() {
            return Ok(vec![]);
        }

        // First element is error (null = no error)
        if !response[0].is_null() {
            let error = response[0]
                .as_str()
                .ok_or_else(|| Error::GenericParse("Expected string error".to_string()))?
                .to_string();
            return Err(Error::Api(ApiError::Other { message: error }));
        }

        let mut dexs = Vec::with_capacity(response.len().saturating_sub(1));
        for (i, value) in response[1..].iter().enumerate() {
            let mut dex: PerpDex = serde_json::from_value(value.clone())
                .map_err(|e| Error::JsonParse(e.to_string()))?;
            dex.id = (i as u32) + 1;
            dexs.push(dex);
        }

        Ok(dexs)
    }

    pub async fn perp_dex_status(&self, dex_name: &str) -> Result<PerpDexStatus> {
        self.send_request(InfoRequest::PerpDexStatus {
            dex: dex_name.to_string(),
        })
        .await
    }

    pub async fn perp_deploy_auction_status(&self) -> Result<PerpDeployAuctionStatus> {
        self.send_request(InfoRequest::PerpDeployAuctionStatus)
            .await
    }

    pub async fn user_role(&self, user: &Address) -> Result<UserRoleResponse> {
        self.send_request(InfoRequest::UserRole {
            user: user.to_owned(),
        })
        .await
    }

    /// Returns multisig signer configuration for a user, if the user is multisig-enabled.
    ///
    /// The API may return `null` when the user is not a multisig user, which maps to `None`.
    pub async fn user_to_multisig_signers(
        &self,
        user: &Address,
    ) -> Result<Option<UserToMultiSigSignersResponse>> {
        self.send_request(InfoRequest::UserToMultiSigSigners {
            user: user.to_owned(),
        })
        .await
    }

    /// Query a user's API rate limit state.
    ///
    /// See [Query user rate limits](https://hyperliquid.gitbook.io/hyperliquid-docs/for-developers/api/info-endpoint#query-user-rate-limits).
    pub async fn user_rate_limit(&self, user: &Address) -> Result<UserRateLimit> {
        self.send_request(InfoRequest::UserRateLimit {
            user: user.to_owned(),
        })
        .await
    }

    /// Query a user's perp clearinghouse state, including `withdrawable` USDC and margin summaries.
    ///
    /// See [`clearinghouseState`](https://hyperliquid.gitbook.io/hyperliquid-docs/for-developers/api/info-endpoint/perpetuals#retrieve-users-perpetuals-account-summary).
    pub async fn user_state(&self, user: &Address) -> Result<UserStateResponse> {
        self.send_request(InfoRequest::UserState {
            user: user.to_owned(),
        })
        .await
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use super::*;

    #[tokio::test]
    async fn test_meta() {
        let info_client = InfoClient::builder(BaseUrl::Testnet).build().unwrap();
        let meta = info_client.meta().await.unwrap();
        println!("{:?}", meta);
    }

    #[tokio::test]
    async fn test_spot_meta() {
        let info_client = InfoClient::builder(BaseUrl::Testnet).build().unwrap();
        let spot_meta = info_client.spot_meta().await.unwrap();
        println!("{:?}", spot_meta);
    }

    #[tokio::test]
    async fn test_user_staking_summary() {
        let info_client = InfoClient::builder(BaseUrl::Testnet).build().unwrap();
        let user_staking_summary = info_client
            .user_staking_summary(
                &Address::from_str("0x1234567890123456789012345678901234567890").unwrap(),
            )
            .await
            .unwrap();
        println!("{:?}", user_staking_summary);
    }

    #[tokio::test]
    async fn test_perp_dexs() {
        let info_client = InfoClient::builder(BaseUrl::Testnet).build().unwrap();
        let perp_dexs = info_client.perp_dexs().await.unwrap();
        println!("{:?}", perp_dexs);

        for (i, dex) in perp_dexs.iter().enumerate() {
            assert_eq!(dex.id, (i as u32) + 1, "dex ids must be sequential starting at 1");
        }
    }

    #[tokio::test]
    async fn test_perp_dex_status() {
        let info_client = InfoClient::builder(BaseUrl::Testnet).build().unwrap();
        let perp_dex_status = info_client.perp_dex_status("slob").await.unwrap();
        println!("{:?}", perp_dex_status);
    }

    #[tokio::test]
    async fn test_perp_deploy_auction_status() {
        let info_client = InfoClient::builder(BaseUrl::Testnet).build().unwrap();
        let perp_deploy_auction_status = info_client.perp_deploy_auction_status().await.unwrap();
        println!("{:?}", perp_deploy_auction_status);
    }

    #[tokio::test]
    async fn test_user_to_multisig_signers() {
        let info_client = InfoClient::builder(BaseUrl::Testnet).build().unwrap();
        let user_to_multisig_signers = info_client
            .user_to_multisig_signers(
                &Address::from_str("0x2C8b738ED0735943CAB99BDBc5dC299813c08E91").unwrap(),
            )
            .await
            .unwrap();
        println!("{:?}", user_to_multisig_signers);
    }

    #[tokio::test]
    async fn test_meta_and_asset_ctxs() {
        let info_client = InfoClient::builder(BaseUrl::Testnet).build().unwrap();
        let response = info_client.meta_and_asset_ctxs(None).await.unwrap();
        assert_eq!(
            response.meta.universe.len(),
            response.asset_ctxs.len(),
            "meta universe and asset ctx lengths must match",
        );
        println!("{:?}", response.meta.universe.first());
        println!("{:?}", response.asset_ctxs.first());
    }

    #[tokio::test]
    async fn test_user_rate_limit() {
        let info_client = InfoClient::builder(BaseUrl::Testnet).build().unwrap();
        let rate_limit = info_client
            .user_rate_limit(
                &Address::from_str("0x1234567890123456789012345678901234567890").unwrap(),
            )
            .await
            .unwrap();
        println!("{:?}", rate_limit);
    }
}
