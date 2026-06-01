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
// Cross-chain transaction status tracking (per-chain visibility)
// =========================================================================

/// Per-chain transaction status. Each leg of a cross-chain transfer
/// (source-chain lock + destination-chain mint/release) carries one of these.
#[derive(Debug, Clone, Copy, PartialEq, Eq, scale::Encode, scale::Decode)]
#[cfg_attr(
    feature = "std",
    derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
)]
pub enum ChainTxStatus {
    /// No activity has occurred on this chain for the request yet.
    NotStarted,
    /// The transaction has been broadcast to the chain but not yet included.
    Submitted,
    /// The transaction is included but is still awaiting confirmations.
    Confirming,
    /// The transaction is finalized on this chain.
    Confirmed,
    /// The transaction failed on this chain (reverted, dropped, or timed out).
    Failed,
}

/// Snapshot of the transaction state on a single chain at a given point.
#[derive(Debug, Clone, PartialEq, scale::Encode, scale::Decode)]
#[cfg_attr(
    feature = "std",
    derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
)]
pub struct ChainStatusUpdate {
    /// The chain this update applies to.
    pub chain_id: ChainId,
    /// Latest known status on that chain.
    pub status: ChainTxStatus,
    /// Hash of the chain-native transaction, once known.
    pub tx_hash: Option<ink::primitives::Hash>,
    /// Block number of the chain-native tx (relayer-supplied for foreign chains,
    /// `env().block_number()` for the local chain).
    pub block_number: u64,
    /// Timestamp when the update was recorded on the bridge contract.
    pub timestamp: u64,
    /// Number of confirmations observed (0 until inclusion).
    pub confirmations: u32,
    /// Optional human-readable reason in case of failure.
    pub error_message: Option<String>,
}

/// Aggregated status of a cross-chain transaction across all chains involved.
///
/// One record is created per bridge request and is updated as the request
/// progresses on each chain. `history` retains a chronological audit trail of
/// every update so off-chain indexers can replay the full lifecycle.
#[derive(Debug, Clone, PartialEq, scale::Encode, scale::Decode)]
#[cfg_attr(
    feature = "std",
    derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
)]
pub struct CrossChainTxStatus {
    pub request_id: u64,
    pub token_id: TokenId,
    pub source_chain: ChainId,
    pub destination_chain: ChainId,
    /// Latest status snapshot on the source chain.
    pub source_status: ChainStatusUpdate,
    /// Latest status snapshot on the destination chain.
    pub destination_status: ChainStatusUpdate,
    /// Aggregated overall status derived from both legs.
    pub overall_status: BridgeOperationStatus,
    /// Full chronological log of every per-chain update.
    pub history: Vec<ChainStatusUpdate>,
    /// Block timestamp of the most recent update.
    pub last_updated: u64,
}

// =========================================================================
// Emergency pause / circuit-breaker types (TASK 2)
// =========================================================================

/// Granular pause flags. Each operation class can be paused independently,
/// or `all_operations` can be set as a master kill-switch. This lets the
/// security team freeze e.g. only new requests while still allowing
/// in-flight settlements to complete.
#[derive(Debug, Clone, Copy, PartialEq, Eq, scale::Encode, scale::Decode)]
#[cfg_attr(
    feature = "std",
    derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
)]
pub struct PauseFlags {
    /// Master kill-switch — if true, every guarded operation is blocked.
    pub all_operations: bool,
    /// Block `initiate_bridge_multisig`.
    pub new_requests: bool,
    /// Block `sign_bridge_request` / signed variant.
    pub signing: bool,
    /// Block `execute_bridge`.
    pub execution: bool,
    /// Block cross-chain DEX trade registration / attachment / settlement.
    pub cross_chain_trades: bool,
}

impl PauseFlags {
    /// Convenience constructor: nothing paused.
    pub fn none() -> Self {
        Self {
            all_operations: false,
            new_requests: false,
            signing: false,
            execution: false,
            cross_chain_trades: false,
        }
    }

    /// Convenience constructor: master kill-switch on.
    pub fn all() -> Self {
        Self {
            all_operations: true,
            new_requests: true,
            signing: true,
            execution: true,
            cross_chain_trades: true,
        }
    }
}

/// Logical operation classes that can be individually paused.
#[derive(Debug, Clone, Copy, PartialEq, Eq, scale::Encode, scale::Decode)]
#[cfg_attr(
    feature = "std",
    derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
)]
pub enum BridgeOperation {
    NewRequest,
    Signing,
    Execution,
    CrossChainTrade,
}

/// Why an emergency pause / unpause was triggered. Used both for the
/// audit log and for the on-chain event so dashboards can categorize
/// incidents.
#[derive(Debug, Clone, PartialEq, Eq, scale::Encode, scale::Decode)]
#[cfg_attr(
    feature = "std",
    derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
)]
pub enum PauseReason {
    /// Manual action by the bridge admin.
    ManualAdmin,
    /// Manual action by a registered guardian.
    GuardianTrigger,
    /// Auto-pause: per-account request burst exceeded the threshold.
    SuspiciousFrequency,
    /// Auto-pause: chain volume in the rolling window exceeded the threshold.
    SuspiciousVolume,
    /// Auto-pause: too many failed/rejected signatures in the rolling window.
    FailedSignatureSurge,
    /// Free-form reason (carried in the audit `detail` field).
    Custom,
}

/// Single audit entry recording a pause or unpause action.
#[derive(Debug, Clone, PartialEq, scale::Encode, scale::Decode)]
#[cfg_attr(
    feature = "std",
    derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
)]
pub struct PauseAuditEntry {
    pub triggered_by: AccountId,
    /// `true` if this entry is a pause, `false` if it is an unpause.
    pub paused: bool,
    /// Snapshot of the resulting flag state right after this action.
    pub flags_after: PauseFlags,
    pub reason: PauseReason,
    /// Optional human-readable detail (incident reference, ticket, etc.).
    pub detail: Option<String>,
    pub block_number: u64,
    pub timestamp: u64,
}

/// Tunable thresholds that drive automatic pausing on suspicious activity.
/// All bounds are inclusive — hitting `>=` the configured value triggers.
#[derive(Debug, Clone, PartialEq, Eq, scale::Encode, scale::Decode)]
#[cfg_attr(
    feature = "std",
    derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
)]
pub struct SuspiciousActivityConfig {
    /// Master switch for the auto-pause feature.
    pub auto_pause_enabled: bool,
    /// Maximum bridge requests an account may submit in a single block
    /// before its activity is treated as a burst attack.
    pub max_requests_per_block_per_account: u32,
    /// Maximum aggregate cross-chain volume routed to one chain in a
    /// 1-hour rolling window before auto-pause.
    pub max_volume_per_hour_per_chain: u128,
    /// Maximum number of `approve = false` (rejection) signatures observed
    /// in a 1-hour rolling window before auto-pause.
    pub max_failed_signatures_per_hour: u32,
}

impl SuspiciousActivityConfig {
    /// Sensible defaults; admins should tune to their deployment profile.
    pub fn default_config() -> Self {
        Self {
            auto_pause_enabled: true,
            max_requests_per_block_per_account: 5,
            max_volume_per_hour_per_chain: 10_000_000_000_000_000_000,
            max_failed_signatures_per_hour: 10,
        }
    }
}

// =========================================================================
// Bridge Analytics Dashboard Types (Issue #208)
// =========================================================================

/// Aggregate bridge analytics returned by get_bridge_analytics.
#[derive(Debug, Clone, PartialEq, scale::Encode, scale::Decode)]
#[cfg_attr(
    feature = "std",
    derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
)]
pub struct BridgeAnalytics {
    pub total_requests: u64,
    pub total_transactions: u64,
    pub total_cross_chain_trades: u64,
    pub active_validators: u32,
    pub active_operators: u32,
    pub supported_chains: u32,
    pub guardian_count: u32,
}

/// Per-chain volume statistics returned by get_chain_volume_stats.
#[derive(Debug, Clone, PartialEq, scale::Encode, scale::Decode)]
#[cfg_attr(
    feature = "std",
    derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
)]
pub struct ChainVolumeStats {
    pub chain_id: ChainId,
    pub chain_name: String,
    pub is_active: bool,
    pub daily_volume: u128,
    pub hourly_volume: u128,
    pub daily_limit: u128,
}

/// Bridge health status summary returned by get_bridge_health_status.
#[derive(Debug, Clone, PartialEq, scale::Encode, scale::Decode)]
#[cfg_attr(
    feature = "std",
    derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
)]
pub struct BridgeHealthStatus {
    pub is_paused: bool,
    pub new_requests_paused: bool,
    pub signing_paused: bool,
    pub execution_paused: bool,
    pub cross_chain_trades_paused: bool,
    pub active_validator_count: u32,
    pub active_operator_count: u32,
    pub guardian_count: u32,
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
