use alloy::primitives::{Address, B256};

use crate::{
    Error, Result,
    eip712::Eip712,
    exchange::{Action, ActionKind, ExchangeClient, SigningData},
    utils::next_nonce,
};

pub trait BuildAction {
    fn build(self, client: &ExchangeClient) -> Result<Action>;
}

impl BuildAction for ActionKind {
    fn build(self, client: &ExchangeClient) -> Result<Action> {
        let timestamp = next_nonce();
        let vault_address = client.vault_address();

        let is_l1_action = self.is_l1_action();

        if is_l1_action {
            self.build_l1_action(client, timestamp, vault_address)
        } else {
            self.build_typed_data_action(client, timestamp, vault_address)
        }
    }
}

impl ActionKind {
    fn is_l1_action(&self) -> bool {
        matches!(
            self,
            ActionKind::Order(_)
                | ActionKind::Cancel(_)
                | ActionKind::CancelByCloid(_)
                | ActionKind::BatchModify(_)
                | ActionKind::UpdateLeverage(_)
                | ActionKind::UpdateIsolatedMargin(_)
                | ActionKind::SpotUser(_)
                | ActionKind::VaultTransfer(_)
                | ActionKind::SetReferrer(_)
                | ActionKind::EvmUserModify(_)
                | ActionKind::ScheduleCancel(_)
                | ActionKind::ClaimRewards(_)
        )
    }

    fn build_l1_action(
        self,
        client: &ExchangeClient,
        timestamp: u64,
        vault_address: Option<Address>,
    ) -> Result<Action> {
        let connection_id = self.hash(timestamp, vault_address)?;
        let action_json =
            serde_json::to_value(&self).map_err(|e| Error::JsonParse(e.to_string()))?;

        Ok(Action {
            action: action_json,
            nonce: timestamp,
            vault_address,
            signing_data: SigningData::L1 {
                connection_id,
                is_mainnet: client.is_mainnet(),
            },
            http_client: client.http_client().clone(),
        })
    }

    fn build_typed_data_action(
        self,
        client: &ExchangeClient,
        timestamp: u64,
        vault_address: Option<Address>,
    ) -> Result<Action> {
        let hash = self.extract_eip712_hash()?;
        let action_json =
            serde_json::to_value(&self).map_err(|e| Error::JsonParse(e.to_string()))?;

        Ok(Action {
            action: action_json,
            nonce: timestamp,
            vault_address,
            signing_data: SigningData::TypedData { hash },
            http_client: client.http_client().clone(),
        })
    }

    fn extract_eip712_hash(&self) -> Result<B256> {
        match self {
            ActionKind::UsdSend(usd_send) => Ok(usd_send.eip712_signing_hash()),
            ActionKind::Withdraw3(withdraw) => Ok(withdraw.eip712_signing_hash()),
            ActionKind::SpotSend(spot_send) => Ok(spot_send.eip712_signing_hash()),
            ActionKind::SendAsset(send_asset) => Ok(send_asset.eip712_signing_hash()),
            ActionKind::ApproveAgent(approve_agent) => Ok(approve_agent.eip712_signing_hash()),
            ActionKind::ApproveBuilderFee(approve_builder_fee) => {
                Ok(approve_builder_fee.eip712_signing_hash())
            }
            _ => Err(Error::GenericParse(
                "Action type not supported for typed data signing".to_string(),
            )),
        }
    }
}
