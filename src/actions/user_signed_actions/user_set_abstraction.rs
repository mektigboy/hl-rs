use alloy::primitives::Address;
use derive_builder::Builder;
use hl_rs_derive::UserSignedAction;
use serde::{Deserialize, Serialize};

use crate::{abi_value::AbiResult, actions::serialization::ser_lowercase, Error, ToAbiValue};

/// Account abstraction mode for [`UserSetAbstraction`].
///
/// See [account abstraction modes](https://hyperliquid.gitbook.io/hyperliquid-docs/trading/account-abstraction-modes).
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum AbstractionMode {
    DexAbstraction,
    UnifiedAccount,
    PortfolioMargin,
    Disabled,
}

impl AbstractionMode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::DexAbstraction => "dexAbstraction",
            Self::UnifiedAccount => "unifiedAccount",
            Self::PortfolioMargin => "portfolioMargin",
            Self::Disabled => "disabled",
        }
    }
}

impl ToAbiValue for AbstractionMode {
    fn to_abi_value(&self, abi_type: &alloy::dyn_abi::DynSolType) -> AbiResult {
        if abi_type != &alloy::dyn_abi::DynSolType::String {
            return Err(Error::AbiEncode {
                rust_type: "AbstractionMode",
                abi_type: format!("{:?}", abi_type),
            });
        }

        Ok(alloy::dyn_abi::DynSolValue::FixedBytes(
            alloy::primitives::keccak256(self.as_str().as_bytes()),
            32,
        ))
    }
}

/// Set account abstraction mode for a user (EIP-712, signed by the principal).
///
/// Like `agentSetAbstraction` but signed by the wallet owner instead of an agent.
#[derive(Debug, Clone, Serialize, Deserialize, Builder, UserSignedAction)]
#[action(
    types = "UserSetAbstraction(string hyperliquidChain,address user,string abstraction,uint64 nonce)"
)]
#[serde(rename_all = "camelCase")]
#[builder(setter(into))]
pub struct UserSetAbstraction {
    /// User address to modify (must match the signer).
    #[serde(serialize_with = "ser_lowercase")]
    pub user: Address,
    /// Abstraction mode to set.
    pub abstraction: AbstractionMode,
    #[builder(default)]
    pub nonce: Option<u64>,
}

impl UserSetAbstraction {
    pub fn builder() -> UserSetAbstractionBuilder {
        UserSetAbstractionBuilder::default()
    }

    pub fn new(user: Address, abstraction: AbstractionMode) -> Self {
        Self {
            user,
            abstraction,
            nonce: None,
        }
    }

    pub fn disable(user: Address) -> Self {
        Self::new(user, AbstractionMode::Disabled)
    }

    pub fn unified_account(user: Address) -> Self {
        Self::new(user, AbstractionMode::UnifiedAccount)
    }

    pub fn portfolio_margin(user: Address) -> Self {
        Self::new(user, AbstractionMode::PortfolioMargin)
    }
}
