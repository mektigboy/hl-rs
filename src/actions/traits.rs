use serde::Serialize;

use crate::{actions::SigningMeta, Error, SigningChain};
use alloy::{
    primitives::{keccak256, Address, B256},
    sol_types::eip712_domain,
};

use super::ActionKind;

// ============================================================================
// L1Action Trait
// ============================================================================

/// Trait for actions signed via L1 connection_id mechanism.
///
/// L1 actions are hashed using MessagePack serialization combined with
/// timestamp, vault address, and expiration metadata.
pub trait L1Action: Serialize + Clone + Send + Sync + 'static {
    /// Action type name for API serialization (e.g., "order", "updateLeverage")
    const ACTION_TYPE: &'static str;

    /// Key to serialize action payload to.
    const PAYLOAD_KEY: &'static str;

    /// Whether to exclude vault_address from the hash (default: false)
    /// Override to `true` for actions like PerpDeploy
    const EXCLUDE_VAULT_FROM_HASH: bool = false;
}

// ============================================================================
// UserSignedAction Trait
// ============================================================================

/// Trait for actions signed via EIP-712 typed data.
///
/// User-signed actions produce human-readable signature requests in wallets.
/// They embed their own timestamp/nonce and use the Hyperliquid typed data domain.
///
/// # Examples
/// - UsdSend, Withdraw3, SpotSend
/// - ApproveAgent, ApproveBuilderFee
pub trait UserSignedAction: Serialize + Send + Sync + 'static {
    /// Action type name for API serialization (e.g., "usdSend", "withdraw3")
    const ACTION_TYPE: &'static str;

    /// Compute the EIP-712 struct hash (type-specific)
    ///
    /// # Errors
    /// Returns an error if any field cannot be encoded as its expected ABI type.
    fn struct_hash(&self, chain: &SigningChain) -> Result<B256, Error>;

    /// Compute the full EIP-712 signing hash
    ///
    /// # Errors
    /// Returns an error if the struct hash computation fails.
    fn eip712_signing_hash(&self, chain: &SigningChain) -> Result<B256, Error> {
        Ok(Self::eip712_signing_hash_from_struct_hash(
            self.struct_hash(chain)?,
            chain,
        ))
    }

    /// Compute the EIP-712 struct hash for an inner multi-sig signature.
    ///
    /// # Errors
    /// Returns an error if any field cannot be encoded as its expected ABI type.
    fn multisig_struct_hash(
        &self,
        chain: &SigningChain,
        payload_multi_sig_user: Address,
        outer_signer: Address,
    ) -> Result<B256, Error>;

    /// Compute the full EIP-712 signing hash for an inner multi-sig signature.
    ///
    /// # Errors
    /// Returns an error if the struct hash computation fails.
    fn multisig_eip712_signing_hash(
        &self,
        chain: &SigningChain,
        payload_multi_sig_user: Address,
        outer_signer: Address,
    ) -> Result<B256, Error> {
        Ok(Self::eip712_signing_hash_from_struct_hash(
            self.multisig_struct_hash(chain, payload_multi_sig_user, outer_signer)?,
            chain,
        ))
    }

    fn eip712_signing_hash_from_struct_hash(struct_hash: B256, chain: &SigningChain) -> B256 {
        let domain = eip712_domain! {
            name: "HyperliquidSignTransaction",
            version: "1",
            chain_id: chain.get_signature_chain_id(),
            verifying_contract: Address::ZERO,
        };

        let domain_hash = domain.hash_struct();

        let mut digest = [0u8; 66];
        digest[0] = 0x19;
        digest[1] = 0x01;
        digest[2..34].copy_from_slice(&domain_hash[..]);
        digest[34..66].copy_from_slice(&struct_hash[..]);

        keccak256(digest)
    }
}

// ============================================================================
// Action Trait (Unified Interface)
// ============================================================================

/// Unified trait for all exchange actions.
///
/// This trait is implemented for action types via the provided macros.
/// Client code uses this trait for a uniform API.
pub trait Action: Serialize + Send + Sync {
    /// Action type name for 'type' tag serialization
    const ACTION_TYPE: &'static str;
    /// Key to serialize action payload to, if it differs from 'ACTION_TYPE'
    const PAYLOAD_KEY: &'static str;

    /// Whether the action is user-signed (EIP-712).
    fn is_user_signed() -> bool {
        false
    }

    /// Whether the user-signed payload uses `time` (vs `nonce`).
    fn uses_time() -> bool {
        false
    }

    /// Build the signing hash from action and metadata
    fn signing_hash(&self, meta: &SigningMeta) -> Result<B256, Error>;

    /// Get embedded nonce from the action (if present)
    fn nonce(&self) -> Option<u64>;

    /// Extract the action kind (clones the action into an ActionKind variant)
    fn extract_action_kind(&self) -> ActionKind;

    /// Attach an embedded nonce to the action
    fn with_nonce(self, nonce: u64) -> Self;
}
