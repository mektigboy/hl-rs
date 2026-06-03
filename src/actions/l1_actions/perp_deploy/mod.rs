mod set_oi_caps;
pub use set_oi_caps::SetOpenInterestCaps;

mod toggle_trading;
pub use toggle_trading::ToggleTrading;

mod set_oracle;
pub use set_oracle::SetOracle;

mod set_fee_recipient;
pub use set_fee_recipient::SetFeeRecipient;

mod set_funding_multipliers;
pub use set_funding_multipliers::SetFundingMultipliers;

mod set_funding_interest_rates;
pub use set_funding_interest_rates::SetFundingInterestRates;

mod set_margin_table_ids;
pub use set_margin_table_ids::SetMarginTableIds;

mod insert_margin_table;
pub use insert_margin_table::{InsertMarginTable, MarginTable, MarginTier};

mod set_sub_deployers;
pub use set_sub_deployers::{SetSubDeployers, SubDeployer, SubDeployerVariant};

mod set_growth_modes;
pub use set_growth_modes::SetGrowthModes;

mod set_margin_modes;
pub use set_margin_modes::{MarginMode, SetMarginModes};

mod set_perp_annotation;
pub use set_perp_annotation::{
    SetPerpAnnotation, SetPerpAnnotationBuildError, SetPerpAnnotationBuilder,
};

mod register_asset;
pub use register_asset::{AssetRequest, PerpDexSchema, RegisterAsset};

mod migrate_dex_quote_token;
pub use migrate_dex_quote_token::MigrateDexQuoteToken;

#[macro_export]
macro_rules! flatten_vec {
    ($struct_name:ident, $field:ident) => {
        impl ::serde::Serialize for $struct_name {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: ::serde::Serializer,
            {
                let mut data = self.$field.clone();
                data.sort_by(|a, b| a.cmp(b));
                data.serialize(serializer)
            }
        }

        impl<'de> ::serde::Deserialize<'de> for $struct_name {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: ::serde::Deserializer<'de>,
            {
                let vec_data = Vec::deserialize(deserializer)?;
                Ok($struct_name {
                    $field: vec_data,
                    nonce: None,
                })
            }
        }
    };
}
