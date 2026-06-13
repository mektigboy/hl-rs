use crate::actions::serialization::ser_lowercase;
use alloy::primitives::Address;
use hl_rs_derive::L1Action;
use serde::{Deserialize, Serialize};

/// Set sub-deployers with specific permissions for a perp DEX.
///
/// Sub-deployers can be granted permission to execute specific action types
/// on behalf of the main deployer. This allows delegation of certain DEX
/// management tasks to other addresses.
///
/// # Examples
///
/// Grant a single permission to a sub-deployer:
///
/// ```
/// use alloy::primitives::Address;
/// use hl_rs::actions::{SetSubDeployers, SubDeployer, SubDeployerVariant};
///
/// let user = Address::repeat_byte(0x42);
/// let sub_deployer = SubDeployer::enable(user, SubDeployerVariant::SetOracle);
///
/// let action = SetSubDeployers::new("mydex")
///     .with_sub_deployer(sub_deployer);
///
/// assert_eq!(action.sub_deployers.len(), 1);
/// assert!(action.sub_deployers[0].allowed);
/// ```
///
/// Grant multiple permissions using the builder pattern:
///
/// ```
/// use alloy::primitives::Address;
/// use hl_rs::actions::{SetSubDeployers, SubDeployerVariant};
///
/// let oracle_updater = Address::repeat_byte(0x11);
/// let fee_manager = Address::repeat_byte(0x22);
///
/// let action = SetSubDeployers::new("mydex")
///     .enable_permissions(oracle_updater, vec![
///         SubDeployerVariant::SetOracle,
///         SubDeployerVariant::SetFundingMultipliers,
///     ])
///     .enable_permissions(fee_manager, vec![
///         SubDeployerVariant::SetFeeRecipient,
///     ]);
///
/// assert_eq!(action.sub_deployers.len(), 3);
/// ```
///
/// Revoke a permission:
///
/// ```
/// use alloy::primitives::Address;
/// use hl_rs::actions::{SetSubDeployers, SubDeployerVariant};
///
/// let user = Address::repeat_byte(0x33);
///
/// let action = SetSubDeployers::new("mydex")
///     .disable_permissions(user, vec![SubDeployerVariant::SetOracle]);
///
/// assert!(!action.sub_deployers[0].allowed);
/// ```
#[derive(Serialize, Deserialize, Debug, Clone, L1Action)]
#[action(action_type = "perpDeploy", payload_key = "setSubDeployers")]
#[serde(rename_all = "camelCase")]
pub struct SetSubDeployers {
    /// The DEX name (lowercase).
    pub dex: String,
    /// List of sub-deployer configurations.
    pub sub_deployers: Vec<SubDeployer>,
    #[serde(skip_serializing)]
    pub nonce: Option<u64>,
}

/// A sub-deployer configuration specifying permissions for an address.
///
/// Each `SubDeployer` represents a permission grant or revocation for a
/// specific action variant.
///
/// # Examples
///
/// Enable a permission:
///
/// ```
/// use alloy::primitives::Address;
/// use hl_rs::actions::{SubDeployer, SubDeployerVariant};
///
/// let user = Address::repeat_byte(0x42);
/// let permission = SubDeployer::enable(user, SubDeployerVariant::SetOracle);
///
/// assert!(permission.allowed);
/// assert_eq!(permission.variant, SubDeployerVariant::SetOracle);
/// ```
///
/// Disable a permission:
///
/// ```
/// use alloy::primitives::Address;
/// use hl_rs::actions::{SubDeployer, SubDeployerVariant};
///
/// let user = Address::repeat_byte(0x42);
/// let permission = SubDeployer::disable(user, SubDeployerVariant::HaltTrading);
///
/// assert!(!permission.allowed);
/// ```
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SubDeployer {
    /// The action variant this sub-deployer is allowed to execute.
    pub variant: SubDeployerVariant,
    /// The address of the sub-deployer.
    #[serde(serialize_with = "ser_lowercase")]
    pub user: Address,
    /// Whether this permission is enabled (`true`) or revoked (`false`).
    pub allowed: bool,
}

/// The perp deploy action variants that can be delegated to sub-deployers.
///
/// Each variant corresponds to a specific perp deploy action that the main
/// deployer can delegate to sub-deployers.
///
/// # Examples
///
/// ```
/// use hl_rs::actions::SubDeployerVariant;
///
/// // Common variants for oracle management
/// let oracle_perms = vec![
///     SubDeployerVariant::SetOracle,
///     SubDeployerVariant::SetFundingMultipliers,
/// ];
///
/// // Risk management variants
/// let risk_perms = vec![
///     SubDeployerVariant::HaltTrading,
///     SubDeployerVariant::SetOpenInterestCaps,
/// ];
/// ```
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum SubDeployerVariant {
    /// Permission to register new assets on the DEX.
    RegisterAsset,
    /// Permission to update oracle prices.
    SetOracle,
    /// Permission to set funding rate multipliers.
    SetFundingMultipliers,
    /// Permission to set 8-hour funding interest rates.
    SetFundingInterestRates,
    /// Permission to halt/resume trading for assets.
    HaltTrading,
    /// Permission to set margin table IDs for assets.
    SetMarginTableIds,
    /// Permission to set the fee recipient address.
    SetFeeRecipient,
    /// Permission to set the fee scale for the DEX.
    SetFeeScale,
    /// Permission to set open interest caps.
    SetOpenInterestCaps,
    /// Permission to insert new margin tables.
    InsertMarginTable,
    /// Permission to set growth modes for assets.
    SetGrowthModes,
    /// Permission to set margin modes for assets.
    SetMarginModes,
    /// Permission to set the category and description for assets.
    SetPerpAnnotation,
    /// Permission to disable the DEX.
    DisableDex,
}

impl SetSubDeployers {
    /// Create a new `SetSubDeployers` action for the specified DEX.
    ///
    /// # Arguments
    ///
    /// * `dex` - Name of the perp DEX (will be stored as-is)
    ///
    /// # Example
    ///
    /// ```
    /// use hl_rs::actions::SetSubDeployers;
    ///
    /// let action = SetSubDeployers::new("mydex");
    /// assert_eq!(action.dex, "mydex");
    /// assert!(action.sub_deployers.is_empty());
    /// ```
    pub fn new(dex: impl Into<String>) -> Self {
        Self {
            dex: dex.into(),
            sub_deployers: vec![],
            nonce: None,
        }
    }

    /// Add a single sub-deployer configuration.
    ///
    /// # Arguments
    ///
    /// * `sub_deployer` - The sub-deployer configuration to add
    ///
    /// # Example
    ///
    /// ```
    /// use alloy::primitives::Address;
    /// use hl_rs::actions::{SetSubDeployers, SubDeployer, SubDeployerVariant};
    ///
    /// let user = Address::repeat_byte(0x42);
    /// let action = SetSubDeployers::new("mydex")
    ///     .with_sub_deployer(SubDeployer::enable(user, SubDeployerVariant::SetOracle));
    ///
    /// assert_eq!(action.sub_deployers.len(), 1);
    /// ```
    pub fn with_sub_deployer(mut self, sub_deployer: SubDeployer) -> Self {
        self.sub_deployers.push(sub_deployer);
        self
    }

    /// Add multiple sub-deployer configurations.
    ///
    /// # Arguments
    ///
    /// * `sub_deployers` - The sub-deployer configurations to add
    ///
    /// # Example
    ///
    /// ```
    /// use alloy::primitives::Address;
    /// use hl_rs::actions::{SetSubDeployers, SubDeployer, SubDeployerVariant};
    ///
    /// let user = Address::repeat_byte(0x42);
    /// let action = SetSubDeployers::new("mydex")
    ///     .with_sub_deployers(vec![
    ///         SubDeployer::enable(user, SubDeployerVariant::SetOracle),
    ///         SubDeployer::enable(user, SubDeployerVariant::HaltTrading),
    ///     ]);
    ///
    /// assert_eq!(action.sub_deployers.len(), 2);
    /// ```
    pub fn with_sub_deployers(mut self, sub_deployers: Vec<SubDeployer>) -> Self {
        self.sub_deployers.extend(sub_deployers);
        self
    }

    /// Enable multiple permissions for a user.
    ///
    /// This is a convenience method that creates `SubDeployer::enable` entries
    /// for each variant.
    ///
    /// # Arguments
    ///
    /// * `user` - The address to grant permissions to
    /// * `variants` - The action variants to enable
    ///
    /// # Example
    ///
    /// ```
    /// use alloy::primitives::Address;
    /// use hl_rs::actions::{SetSubDeployers, SubDeployerVariant};
    ///
    /// let user = Address::repeat_byte(0x42);
    /// let action = SetSubDeployers::new("mydex")
    ///     .enable_permissions(user, vec![
    ///         SubDeployerVariant::SetOracle,
    ///         SubDeployerVariant::SetFundingMultipliers,
    ///     ]);
    ///
    /// assert_eq!(action.sub_deployers.len(), 2);
    /// assert!(action.sub_deployers.iter().all(|s| s.allowed));
    /// ```
    pub fn enable_permissions(mut self, user: Address, variants: Vec<SubDeployerVariant>) -> Self {
        for variant in variants {
            self.sub_deployers.push(SubDeployer::enable(user, variant));
        }
        self
    }

    /// Disable (revoke) multiple permissions for a user.
    ///
    /// This is a convenience method that creates `SubDeployer::disable` entries
    /// for each variant.
    ///
    /// # Arguments
    ///
    /// * `user` - The address to revoke permissions from
    /// * `variants` - The action variants to disable
    ///
    /// # Example
    ///
    /// ```
    /// use alloy::primitives::Address;
    /// use hl_rs::actions::{SetSubDeployers, SubDeployerVariant};
    ///
    /// let user = Address::repeat_byte(0x42);
    /// let action = SetSubDeployers::new("mydex")
    ///     .disable_permissions(user, vec![SubDeployerVariant::SetOracle]);
    ///
    /// assert!(!action.sub_deployers[0].allowed);
    /// ```
    pub fn disable_permissions(mut self, user: Address, variants: Vec<SubDeployerVariant>) -> Self {
        for variant in variants {
            self.sub_deployers.push(SubDeployer::disable(user, variant));
        }
        self
    }
}

impl SubDeployer {
    /// Create a sub-deployer configuration that grants a permission.
    ///
    /// # Arguments
    ///
    /// * `user` - The address to grant the permission to
    /// * `variant` - The action variant to allow
    ///
    /// # Example
    ///
    /// ```
    /// use alloy::primitives::Address;
    /// use hl_rs::actions::{SubDeployer, SubDeployerVariant};
    ///
    /// let user = Address::repeat_byte(0x42);
    /// let permission = SubDeployer::enable(user, SubDeployerVariant::SetOracle);
    ///
    /// assert!(permission.allowed);
    /// assert_eq!(permission.user, user);
    /// ```
    pub fn enable(user: Address, variant: SubDeployerVariant) -> Self {
        Self {
            variant,
            user,
            allowed: true,
        }
    }

    /// Create a sub-deployer configuration that revokes a permission.
    ///
    /// # Arguments
    ///
    /// * `user` - The address to revoke the permission from
    /// * `variant` - The action variant to disallow
    ///
    /// # Example
    ///
    /// ```
    /// use alloy::primitives::Address;
    /// use hl_rs::actions::{SubDeployer, SubDeployerVariant};
    ///
    /// let user = Address::repeat_byte(0x42);
    /// let permission = SubDeployer::disable(user, SubDeployerVariant::HaltTrading);
    ///
    /// assert!(!permission.allowed);
    /// ```
    pub fn disable(user: Address, variant: SubDeployerVariant) -> Self {
        Self {
            variant,
            user,
            allowed: false,
        }
    }
}
