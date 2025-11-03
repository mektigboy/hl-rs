use alloy::primitives::Address;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct RegisterAssetRequest {
    pub coin: String,
    pub sz_decimals: u32,
    pub oracle_px: String,
    pub margin_table_id: u32,
    pub only_isolated: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct PerpDexSchemaInput {
    pub full_name: String,
    pub collateral_token: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub oracle_updater: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct RegisterAsset {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_gas: Option<String>,
    pub asset_request: RegisterAssetRequest,
    pub dex: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub schema: Option<PerpDexSchemaInput>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SetOracle {
    pub dex: String,
    pub oracle_pxs: Vec<(String, String)>,
    pub mark_pxs: Vec<Vec<(String, String)>>,
    pub external_perp_pxs: Vec<(String, String)>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SetFundingMultipliers {
    pub multipliers: Vec<(String, String)>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct HaltTrading {
    pub coin: String,
    pub is_halted: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SetMarginTableIds {
    pub ids: Vec<(String, u32)>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SetFeeRecipient {
    pub dex: String,
    pub fee_recipient: Address,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SetOpenInterestCaps {
    pub caps: Vec<(String, String)>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct RawMarginTier {
    pub lower_bound: i64,
    pub max_leverage: u32,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct RawMarginTable {
    pub description: String,
    pub margin_tiers: Vec<RawMarginTier>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct InsertMarginTable {
    pub dex: String,
    pub margin_table: RawMarginTable,
}

/// Wrapper that serializes with type: "perpDeploy"
/// and one of the action fields (registerAsset, setOracle, etc.)
#[derive(Debug, Clone)]
pub enum PerpDeploy {
    RegisterAsset(RegisterAsset),
    SetOracle(SetOracle),
    SetFundingMultipliers(SetFundingMultipliers),
    HaltTrading(HaltTrading),
    SetMarginTableIds(SetMarginTableIds),
    SetFeeRecipient(SetFeeRecipient),
    SetOpenInterestCaps(SetOpenInterestCaps),
    InsertMarginTable(InsertMarginTable),
}

impl Serialize for PerpDeploy {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut state = serializer.serialize_struct("PerpDeploy", 2)?;
        state.serialize_field("type", "perpDeploy")?;
        match self {
            PerpDeploy::RegisterAsset(v) => state.serialize_field("registerAsset", v)?,
            PerpDeploy::SetOracle(v) => state.serialize_field("setOracle", v)?,
            PerpDeploy::SetFundingMultipliers(v) => {
                state.serialize_field("setFundingMultipliers", v)?;
            }
            PerpDeploy::HaltTrading(v) => state.serialize_field("haltTrading", v)?,
            PerpDeploy::SetMarginTableIds(v) => state.serialize_field("setMarginTableIds", v)?,
            PerpDeploy::SetFeeRecipient(v) => state.serialize_field("setFeeRecipient", v)?,
            PerpDeploy::SetOpenInterestCaps(v) => {
                state.serialize_field("setOpenInterestCaps", v)?;
            }
            PerpDeploy::InsertMarginTable(v) => state.serialize_field("insertMarginTable", v)?,
        }
        state.end()
    }
}

impl<'de> Deserialize<'de> for PerpDeploy {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de::{self, Visitor};
        use std::fmt;

        struct PerpDeployVisitor;

        impl<'de> Visitor<'de> for PerpDeployVisitor {
            type Value = PerpDeploy;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a perpDeploy action")
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: serde::de::MapAccess<'de>,
            {
                let mut type_val: Option<String> = None;
                let mut register_asset: Option<RegisterAsset> = None;
                let mut set_oracle: Option<SetOracle> = None;
                let mut set_funding_multipliers: Option<SetFundingMultipliers> = None;
                let mut halt_trading: Option<HaltTrading> = None;
                let mut set_margin_table_ids: Option<SetMarginTableIds> = None;
                let mut set_fee_recipient: Option<SetFeeRecipient> = None;
                let mut set_open_interest_caps: Option<SetOpenInterestCaps> = None;
                let mut insert_margin_table: Option<InsertMarginTable> = None;

                while let Some(key) = map.next_key::<String>()? {
                    match key.as_str() {
                        "type" => {
                            type_val = Some(map.next_value()?);
                        }
                        "registerAsset" => {
                            register_asset = Some(map.next_value()?);
                        }
                        "setOracle" => {
                            set_oracle = Some(map.next_value()?);
                        }
                        "setFundingMultipliers" => {
                            set_funding_multipliers = Some(map.next_value()?);
                        }
                        "haltTrading" => {
                            halt_trading = Some(map.next_value()?);
                        }
                        "setMarginTableIds" => {
                            set_margin_table_ids = Some(map.next_value()?);
                        }
                        "setFeeRecipient" => {
                            set_fee_recipient = Some(map.next_value()?);
                        }
                        "setOpenInterestCaps" => {
                            set_open_interest_caps = Some(map.next_value()?);
                        }
                        "insertMarginTable" => {
                            insert_margin_table = Some(map.next_value()?);
                        }
                        _ => {
                            let _ = map.next_value::<serde::de::IgnoredAny>()?;
                        }
                    }
                }

                if type_val.as_deref() != Some("perpDeploy") {
                    return Err(de::Error::invalid_value(
                        serde::de::Unexpected::Str(type_val.unwrap_or_default().as_str()),
                        &"perpDeploy",
                    ));
                }

                if let Some(v) = register_asset {
                    Ok(PerpDeploy::RegisterAsset(v))
                } else if let Some(v) = set_oracle {
                    Ok(PerpDeploy::SetOracle(v))
                } else if let Some(v) = set_funding_multipliers {
                    Ok(PerpDeploy::SetFundingMultipliers(v))
                } else if let Some(v) = halt_trading {
                    Ok(PerpDeploy::HaltTrading(v))
                } else if let Some(v) = set_margin_table_ids {
                    Ok(PerpDeploy::SetMarginTableIds(v))
                } else if let Some(v) = set_fee_recipient {
                    Ok(PerpDeploy::SetFeeRecipient(v))
                } else if let Some(v) = set_open_interest_caps {
                    Ok(PerpDeploy::SetOpenInterestCaps(v))
                } else if let Some(v) = insert_margin_table {
                    Ok(PerpDeploy::InsertMarginTable(v))
                } else {
                    Err(de::Error::missing_field(
                        "one of the perpDeploy action fields",
                    ))
                }
            }
        }

        deserializer.deserialize_map(PerpDeployVisitor)
    }
}
