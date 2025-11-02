use alloy::primitives::{Address, B256, keccak256};
use serde::{Deserialize, Serialize};

use crate::{
    Error, Result,
    exchange::requests::{
        ApproveAgent, ApproveBuilderFee, BulkCancel, BulkCancelCloid, BulkModify, BulkOrder,
        ClaimRewards, EvmUserModify, ScheduleCancel, SendAsset, SetReferrer, SpotSend, SpotUser,
        UpdateIsolatedMargin, UpdateLeverage, UsdSend, VaultTransfer, Withdraw3,
    },
};

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "type")]
#[serde(rename_all = "camelCase")]
pub enum ActionKind {
    UsdSend(UsdSend),
    UpdateLeverage(UpdateLeverage),
    UpdateIsolatedMargin(UpdateIsolatedMargin),
    Order(BulkOrder),
    Cancel(BulkCancel),
    CancelByCloid(BulkCancelCloid),
    BatchModify(BulkModify),
    ApproveAgent(ApproveAgent),
    Withdraw3(Withdraw3),
    SpotUser(SpotUser),
    SendAsset(SendAsset),
    VaultTransfer(VaultTransfer),
    SpotSend(SpotSend),
    SetReferrer(SetReferrer),
    ApproveBuilderFee(ApproveBuilderFee),
    EvmUserModify(EvmUserModify),
    ScheduleCancel(ScheduleCancel),
    ClaimRewards(ClaimRewards),
}

impl ActionKind {
    pub fn hash(&self, timestamp: u64, vault_address: Option<Address>) -> Result<B256> {
        let mut bytes =
            rmp_serde::to_vec_named(self).map_err(|e| Error::RmpParse(e.to_string()))?;
        bytes.extend(timestamp.to_be_bytes());
        if let Some(vault_address) = vault_address {
            bytes.push(1);
            bytes.extend(vault_address);
        } else {
            bytes.push(0);
        }
        Ok(keccak256(bytes))
    }
}
