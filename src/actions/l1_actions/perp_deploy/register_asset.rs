use crate::actions::serialization::ser_lowercase;
use alloy::primitives::Address;
use hl_rs_derive::L1Action;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/// Register a new asset on a perp DEX.
///
/// This action registers a new perpetual asset on an existing or new DEX.
/// When creating a new DEX, the `schema` field must be provided via [`deploy_dex`](Self::deploy_dex).
///
/// # Examples
///
/// Register an asset on an existing DEX:
///
/// ```
/// use hl_rs::actions::{RegisterAsset, AssetRequest};
/// use rust_decimal_macros::dec;
///
/// let asset = AssetRequest {
///     coin: "BTC".to_string(),
///     sz_decimals: 4,
///     oracle_px: dec!(50_000.69),
///     margin_table_id: 20,
///     only_isolated: false,
/// };
/// let action = RegisterAsset::new("mydex", asset);
///
/// assert_eq!(action.dex, "mydex");
/// // Note: coin is prefixed with dex name
/// assert_eq!(action.asset_request.coin, "mydex:BTC");
/// ```
///
/// Register an asset on a new DEX (requires schema):
///
/// ```
/// use alloy::primitives::Address;
/// use hl_rs::actions::{RegisterAsset, AssetRequest, PerpDexSchema};
/// use rust_decimal_macros::dec;
///
/// let oracle_updater = Address::repeat_byte(0x42);
/// let schema = PerpDexSchema::new("My DEX", 1)  // USDC collateral
///     .with_oracle_updater(oracle_updater);
///
/// let asset = AssetRequest {
///     coin: "BTC".to_string(),
///     sz_decimals: 4,
///     oracle_px: dec!(50_000.69),
///     margin_table_id: 20,
///     only_isolated: false,
/// };
/// let action = RegisterAsset::new("newdex", asset)
///     .deploy_dex(schema);
///
/// assert!(action.schema.is_some());
/// ```
///
/// With max gas limit and isolated margin only:
///
/// ```
/// use hl_rs::actions::{RegisterAsset, AssetRequest};
/// use rust_decimal_macros::dec;
///
/// let asset = AssetRequest {
///     coin: "MEME".to_string(),
///     sz_decimals: 2,
///     oracle_px: dec!(0.001),
///     margin_table_id: 1,
///     only_isolated: true,
/// };
/// let action = RegisterAsset::new("mydex", asset).max_gas(1_000_000);
///
/// assert_eq!(action.max_gas, Some(1_000_000));
/// assert!(action.asset_request.only_isolated);
/// ```
#[derive(Serialize, Deserialize, Debug, Clone, L1Action)]
#[action(action_type = "perpDeploy", payload_key = "registerAsset")]
#[serde(rename_all = "camelCase")]
pub struct RegisterAsset {
    /// Maximum gas for the operation (optional).
    pub max_gas: Option<u64>,
    /// The asset registration request details.
    pub asset_request: AssetRequest,
    /// The DEX name to register the asset on.
    pub dex: String,
    /// Schema for new DEX creation (required when creating a new DEX).
    /// Note: This field is always serialized (even as null) to match the Hyperliquid API.
    pub schema: Option<PerpDexSchema>,
    #[serde(skip_serializing)]
    pub nonce: Option<u64>,
}

/// Asset registration request details.
///
/// Contains the parameters for registering a new perpetual asset.
///
/// # Examples
///
/// ```
/// use hl_rs::actions::AssetRequest;
/// use rust_decimal_macros::dec;
///
/// let asset = AssetRequest {
///     coin: "BTC".to_string(),
///     sz_decimals: 3,
///     oracle_px: dec!(420.69),
///     margin_table_id: 1,
///     only_isolated: true,
/// };
/// ```
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct AssetRequest {
    /// The coin/asset symbol (e.g., "BTC", "ETH").
    pub coin: String,
    /// Number of decimal places for size (position sizing precision).
    pub sz_decimals: u32,
    /// Initial oracle price.
    pub oracle_px: Decimal,
    /// ID of the margin table to use for this asset.
    pub margin_table_id: u32,
    /// Whether this asset only supports isolated margin (not cross margin).
    pub only_isolated: bool,
}

/// Schema for creating a new perp DEX.
///
/// Required when registering an asset on a new DEX (one that doesn't exist yet).
///
/// # Examples
///
/// Basic schema with USDC collateral:
///
/// ```
/// use hl_rs::actions::PerpDexSchema;
///
/// let schema = PerpDexSchema::new("My DEX", 1);  // 1 = USDC
/// assert_eq!(schema.full_name, "My DEX");
/// assert_eq!(schema.collateral_token, 1);
/// assert!(schema.oracle_updater.is_none());
/// ```
///
/// Schema with custom oracle updater:
///
/// ```
/// use alloy::primitives::Address;
/// use hl_rs::actions::PerpDexSchema;
///
/// let oracle = Address::repeat_byte(0x42);
/// let schema = PerpDexSchema::new("My DEX", 1)
///     .with_oracle_updater(oracle);
///
/// assert!(schema.oracle_updater.is_some());
/// ```
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct PerpDexSchema {
    /// Full display name of the DEX.
    pub full_name: String,
    /// Collateral token identifier (e.g., 1 for USDC).
    pub collateral_token: u32,
    /// Optional oracle updater address. If None, the deployer is the oracle updater.
    #[serde(serialize_with = "serialize_optional_address")]
    pub oracle_updater: Option<Address>,
}

fn serialize_optional_address<S>(addr: &Option<Address>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    match addr {
        Some(a) => ser_lowercase(a, serializer),
        None => serializer.serialize_none(),
    }
}

impl PerpDexSchema {
    /// Create a new DEX schema.
    ///
    /// # Arguments
    ///
    /// * `full_name` - Display name for the DEX
    /// * `collateral_token` - Collateral token identifier (e.g., 1 for USDC)
    pub fn new(full_name: impl Into<String>, collateral_token: u32) -> Self {
        Self {
            full_name: full_name.into(),
            collateral_token,
            oracle_updater: None,
        }
    }

    /// Set a custom oracle updater address.
    ///
    /// If not set, the deployer will be the oracle updater.
    pub fn with_oracle_updater(mut self, updater: Address) -> Self {
        self.oracle_updater = Some(updater);
        self
    }
}

impl RegisterAsset {
    /// Create a `RegisterAsset` action.
    ///
    /// # Arguments
    ///
    /// * `dex` - The name of the existing DEX
    /// * `asset_request` - The asset request details
    pub fn new(dex: impl Into<String>, asset_request: AssetRequest) -> Self {
        let dex = dex.into();

        let mut asset_request = asset_request;
        if !asset_request.coin.contains(":") {
            asset_request.coin = format!("{}:{}", dex, asset_request.coin);
        }

        Self {
            max_gas: None,
            asset_request,
            dex,
            schema: None,
            nonce: None,
        }
    }

    /// Create a `RegisterAsset` action for a new DEX.
    ///
    /// Use this when creating a new DEX along with its first asset.
    ///
    /// # Arguments
    ///
    /// * `dex` - The name for the new DEX
    /// * `schema` - The DEX schema configuration
    pub fn deploy_dex(mut self, schema: PerpDexSchema) -> Self {
        self.schema = Some(schema);
        self
    }

    /// Use a reserve ticker for the operation.
    /// This will use a reserve ticker even if the auction is still available
    pub fn use_reserve_ticker(self) -> Self {
        self.max_gas(0)
    }

    /// Set the maximum gas limit for the operation.
    /// Setting max_gas to `Some(0)` uses a reserve ticker.
    pub fn max_gas(mut self, gas: u64) -> Self {
        self.max_gas = Some(gas);
        self
    }
}
