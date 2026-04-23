//! Cross-chain bridge types and trait definitions.
//!
//! This module contains all bridge-related types, status enums, configuration
//! structures, and trait definitions for cross-chain property token bridging.

use crate::property::{ChainId, PropertyMetadata, TokenId};
use ink::prelude::string::String;
use ink::prelude::vec::Vec;
use ink::primitives::AccountId;

// =========================================================================
// Data Types
// =========================================================================

/// Bridge status information
#[derive(Debug, Clone, PartialEq, scale::Encode, scale::Decode)]
#[cfg_attr(
    feature = "std",
    derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
)]
pub struct BridgeStatus {
    pub is_locked: bool,
    pub source_chain: Option<ChainId>,
    pub destination_chain: Option<ChainId>,
    pub locked_at: Option<u64>,
    pub bridge_request_id: Option<u64>,
    pub status: BridgeOperationStatus,
}

/// Bridge operation status
#[derive(Debug, Clone, PartialEq, scale::Encode, scale::Decode)]
#[cfg_attr(
    feature = "std",
    derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
)]
pub enum BridgeOperationStatus {
    None,
    Pending,
    Locked,
    InTransit,
    Completed,
    Failed,
    Recovering,
    Expired,
}

/// Bridge monitoring information
#[derive(Debug, Clone, PartialEq, scale::Encode, scale::Decode)]
#[cfg_attr(
    feature = "std",
    derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
)]
pub struct BridgeMonitoringInfo {
    pub bridge_request_id: u64,
    pub token_id: TokenId,
    pub source_chain: ChainId,
    pub destination_chain: ChainId,
    pub status: BridgeOperationStatus,
    pub created_at: u64,
    pub expires_at: Option<u64>,
    pub signatures_collected: u8,
    pub signatures_required: u8,
    pub error_message: Option<String>,
}

/// Recovery action for failed bridges
#[derive(Debug, Clone, PartialEq, scale::Encode, scale::Decode)]
#[cfg_attr(
    feature = "std",
    derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
)]
pub enum RecoveryAction {
    UnlockToken,
    RefundGas,
    RetryBridge,
    CancelBridge,
}

/// Bridge transaction record
#[derive(Debug, Clone, PartialEq, scale::Encode, scale::Decode)]
#[cfg_attr(
    feature = "std",
    derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
)]
pub struct BridgeTransaction {
    pub transaction_id: u64,
    pub token_id: TokenId,
    pub source_chain: ChainId,
    pub destination_chain: ChainId,
    pub sender: AccountId,
    pub recipient: AccountId,
    pub transaction_hash: ink::primitives::Hash,
    pub timestamp: u64,
    pub gas_used: u64,
    pub status: BridgeOperationStatus,
    pub metadata: PropertyMetadata,
}

/// Multi-signature bridge request
#[derive(Debug, Clone, PartialEq, scale::Encode, scale::Decode)]
#[cfg_attr(
    feature = "std",
    derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
)]
pub struct MultisigBridgeRequest {
    pub request_id: u64,
    pub token_id: TokenId,
    pub source_chain: ChainId,
    pub destination_chain: ChainId,
    pub sender: AccountId,
    pub recipient: AccountId,
    pub required_signatures: u8,
    pub signatures: Vec<AccountId>,
    pub created_at: u64,
    pub expires_at: Option<u64>,
    pub status: BridgeOperationStatus,
    pub metadata: PropertyMetadata,
}

/// Bridge configuration
#[derive(Debug, Clone, PartialEq, scale::Encode, scale::Decode)]
#[cfg_attr(
    feature = "std",
    derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
)]
pub struct BridgeConfig {
    pub supported_chains: Vec<ChainId>,
    pub min_signatures_required: u8,
    pub max_signatures_required: u8,
    pub default_timeout_blocks: u64,
    pub gas_limit_per_bridge: u64,
    pub emergency_pause: bool,
    pub metadata_preservation: bool,
    pub rate_limit_enabled: bool,
    pub max_requests_per_day: u64,
    pub max_value_per_day: u128,
}

/// Chain-specific bridge information
#[derive(Debug, Clone, PartialEq, scale::Encode, scale::Decode)]
#[cfg_attr(
    feature = "std",
    derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
)]
pub struct ChainBridgeInfo {
    pub chain_id: ChainId,
    pub chain_name: String,
    pub bridge_contract_address: Option<AccountId>,
    pub is_active: bool,
    pub gas_multiplier: u32,      // Gas cost multiplier for this chain
    pub confirmation_blocks: u32, // Blocks to wait for confirmation
    pub supported_tokens: Vec<TokenId>,
    pub chain_daily_limit: u128, // Max volume allowed to be routed to this chain per day
}

/// Bridge fee quote for cross-chain operations
#[derive(Debug, Clone, PartialEq, scale::Encode, scale::Decode)]
#[cfg_attr(
    feature = "std",
    derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
)]
pub struct BridgeFeeQuote {
    pub destination_chain: ChainId,
    pub gas_estimate: u64,
    pub protocol_fee: u128,
    pub total_fee: u128,
}

// =========================================================================
// Trait Definitions
// =========================================================================

/// Cross-chain bridge trait for property tokens
pub trait PropertyTokenBridge {
    /// Error type for bridge operations
    type Error;

    /// Lock a token for bridging to another chain
    fn lock_token_for_bridge(
        &mut self,
        token_id: TokenId,
        destination_chain: ChainId,
        recipient: AccountId,
    ) -> Result<(), Self::Error>;

    /// Mint a bridged token from another chain
    fn mint_bridged_token(
        &mut self,
        source_chain: ChainId,
        original_token_id: TokenId,
        recipient: AccountId,
        metadata: PropertyMetadata,
    ) -> Result<TokenId, Self::Error>;

    /// Burn a bridged token when returning to original chain
    fn burn_bridged_token(
        &mut self,
        token_id: TokenId,
        destination_chain: ChainId,
        recipient: AccountId,
    ) -> Result<(), Self::Error>;

    /// Unlock a token that was previously locked
    fn unlock_token(&mut self, token_id: TokenId, recipient: AccountId) -> Result<(), Self::Error>;

    /// Get bridge status for a token
    fn get_bridge_status(&self, token_id: TokenId) -> Option<BridgeStatus>;

    /// Verify bridge transaction hash
    fn verify_bridge_transaction(
        &self,
        token_id: TokenId,
        transaction_hash: ink::primitives::Hash,
        source_chain: ChainId,
    ) -> bool;

    /// Add a bridge operator
    fn add_bridge_operator(&mut self, operator: AccountId) -> Result<(), Self::Error>;

    /// Remove a bridge operator
    fn remove_bridge_operator(&mut self, operator: AccountId) -> Result<(), Self::Error>;

    /// Check if an account is a bridge operator
    fn is_bridge_operator(&self, account: AccountId) -> bool;

    /// Get all bridge operators
    fn get_bridge_operators(&self) -> Vec<AccountId>;
}

/// Advanced bridge trait with multi-signature and monitoring
pub trait AdvancedBridge {
    /// Error type for advanced bridge operations
    type Error;

    /// Initiate bridge with multi-signature requirement
    fn initiate_bridge_multisig(
        &mut self,
        token_id: TokenId,
        destination_chain: ChainId,
        recipient: AccountId,
        required_signatures: u8,
        timeout_blocks: Option<u64>,
    ) -> Result<u64, Self::Error>; // Returns bridge request ID

    /// Sign a bridge request
    fn sign_bridge_request(
        &mut self,
        bridge_request_id: u64,
        approve: bool,
    ) -> Result<(), Self::Error>;

    /// Execute bridge after collecting required signatures
    fn execute_bridge(&mut self, bridge_request_id: u64) -> Result<(), Self::Error>;

    /// Monitor bridge status and handle errors
    fn monitor_bridge_status(&self, bridge_request_id: u64) -> Option<BridgeMonitoringInfo>;

    /// Recover from failed bridge operation
    fn recover_failed_bridge(
        &mut self,
        bridge_request_id: u64,
        recovery_action: RecoveryAction,
    ) -> Result<(), Self::Error>;

    /// Get gas estimation for bridge operation
    fn estimate_bridge_gas(
        &self,
        token_id: TokenId,
        destination_chain: ChainId,
    ) -> Result<u64, Self::Error>;

    /// Get bridge history for an account
    fn get_bridge_history(&self, account: AccountId) -> Vec<BridgeTransaction>;
}
