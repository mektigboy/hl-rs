use hl_rs_derive::L1Action;
use serde::{Deserialize, Serialize};

use super::PerpDexSchema;

/// Migrate a perp DEX to a new quote token/schema.
///
/// Note: This action is intended only for migrating USDH DEXes.
#[derive(Serialize, Deserialize, Debug, Clone, L1Action)]
#[action(action_type = "perpDeploy", payload_key = "migrateDexQuoteToken")]
#[serde(rename_all = "camelCase")]
pub struct MigrateDexQuoteToken {
    /// Existing DEX name.
    pub dex: String,
    /// New DEX name to migrate into.
    pub new_dex: String,
    /// New schema to apply for the migrated DEX.
    pub schema: PerpDexSchema,
    #[serde(skip_serializing)]
    pub nonce: Option<u64>,
}

impl MigrateDexQuoteToken {
    pub fn new(
        dex: impl Into<String>,
        new_dex: impl Into<String>,
        schema: PerpDexSchema,
    ) -> Self {
        Self {
            dex: dex.into(),
            new_dex: new_dex.into(),
            schema,
            nonce: None,
        }
    }
}
