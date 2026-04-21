#![cfg_attr(not(feature = "std"), no_std)]

// =========================================================================
// Existing modules
// =========================================================================
pub mod access_control;
pub mod constants;
pub mod crypto;
pub mod errors;
pub mod randomness;

pub use access_control::*;
pub use crypto::*;
pub mod i18n;
pub mod monitoring;

// =========================================================================
// New domain-specific modules (Issue #101)
// =========================================================================
pub mod bridge;
pub mod compliance;
pub mod dex;
pub mod fee;
pub mod oracle;
pub mod property;

// =========================================================================
// Re-exports for backward compatibility
// =========================================================================

// Original re-exports
pub use errors::*;
pub use i18n::*;
pub use monitoring::*;

// Re-export all new module contents at the crate root so that
// existing `use propchain_traits::*` continues to resolve every type.
pub use bridge::*;
pub use dex::*;
pub use oracle::*;
pub use property::*;

// Re-export compliance and fee module contents (types are defined in those modules)
pub use compliance::*;
pub use fee::*;

#[cfg(not(feature = "std"))]
use scale_info::prelude::vec::Vec;

/// AccountId type alias for convenience
pub type AccountId = ink::primitives::AccountId;

/// Advanced escrow trait with multi-signature and document custody
pub trait AdvancedEscrow {
    /// Error type for escrow operations
    type Error;

    /// Create an advanced escrow with multi-signature support
    #[allow(clippy::too_many_arguments)]
    fn create_escrow_advanced(
        &mut self,
        property_id: u64,
        amount: u128,
        buyer: AccountId,
        seller: AccountId,
        participants: Vec<AccountId>,
        required_signatures: u8,
        release_time_lock: Option<u64>,
    ) -> Result<u64, Self::Error>;

    /// Deposit funds to escrow
    fn deposit_funds(&mut self, escrow_id: u64) -> Result<(), Self::Error>;

    /// Release funds with multi-signature approval
    fn release_funds(&mut self, escrow_id: u64) -> Result<(), Self::Error>;

    /// Refund funds with multi-signature approval
    fn refund_funds(&mut self, escrow_id: u64) -> Result<(), Self::Error>;

    /// Upload document hash to escrow
    fn upload_document(
        &mut self,
        escrow_id: u64,
        document_hash: ink::primitives::Hash,
        document_type: String,
    ) -> Result<(), Self::Error>;

    /// Verify a document
    fn verify_document(
        &mut self,
        escrow_id: u64,
        document_hash: ink::primitives::Hash,
    ) -> Result<(), Self::Error>;

    /// Add a condition to the escrow
    fn add_condition(&mut self, escrow_id: u64, description: String) -> Result<u64, Self::Error>;

    /// Mark a condition as met
    fn mark_condition_met(&mut self, escrow_id: u64, condition_id: u64) -> Result<(), Self::Error>;

    /// Sign approval for release or refund
    fn sign_approval(
        &mut self,
        escrow_id: u64,
        approval_type: ApprovalType,
    ) -> Result<(), Self::Error>;

    /// Raise a dispute
    fn raise_dispute(&mut self, escrow_id: u64, reason: String) -> Result<(), Self::Error>;

    /// Resolve a dispute (admin only)
    fn resolve_dispute(&mut self, escrow_id: u64, resolution: String) -> Result<(), Self::Error>;

    /// Emergency override (admin only)
    fn emergency_override(
        &mut self,
        escrow_id: u64,
        release_to_seller: bool,
    ) -> Result<(), Self::Error>;
}

/// Approval type for multi-signature operations
#[derive(Debug, Clone, PartialEq, Eq, scale::Encode, scale::Decode)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub enum ApprovalType {
    Release,
    Refund,
    EmergencyOverride,
}

/// Chain ID type for cross-chain operations
pub type ChainId = u64;

/// Token ID type for property tokens
pub type TokenId = u64;

/// Cross-chain bridge trait for property tokens
pub trait PropertyTokenBridge {
    /// Error type for bridge operations
    type Error;

    /// Lock a token for bridging to another chain
    fn lock_token_for_bridge(
        &mut self,
        token_id: TokenId,
        destination_chain: ChainId,
        recipient: ink::primitives::AccountId,
    ) -> Result<(), Self::Error>;

    /// Mint a bridged token from another chain
    fn mint_bridged_token(
        &mut self,
        source_chain: ChainId,
        original_token_id: TokenId,
        recipient: ink::primitives::AccountId,
        metadata: PropertyMetadata,
    ) -> Result<TokenId, Self::Error>;

    /// Burn a bridged token when returning to original chain
    fn burn_bridged_token(
        &mut self,
        token_id: TokenId,
        destination_chain: ChainId,
        recipient: ink::primitives::AccountId,
    ) -> Result<(), Self::Error>;

    /// Unlock a token that was previously locked
    fn unlock_token(
        &mut self,
        token_id: TokenId,
        recipient: ink::primitives::AccountId,
    ) -> Result<(), Self::Error>;

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
    fn add_bridge_operator(
        &mut self,
        operator: ink::primitives::AccountId,
    ) -> Result<(), Self::Error>;

    /// Remove a bridge operator
    fn remove_bridge_operator(
        &mut self,
        operator: ink::primitives::AccountId,
    ) -> Result<(), Self::Error>;

    /// Check if an account is a bridge operator
    fn is_bridge_operator(&self, account: ink::primitives::AccountId) -> bool;

    /// Get all bridge operators
    fn get_bridge_operators(&self) -> Vec<ink::primitives::AccountId>;
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
        recipient: ink::primitives::AccountId,
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
    fn get_bridge_history(&self, account: ink::primitives::AccountId) -> Vec<BridgeTransaction>;
}

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
    pub sender: ink::primitives::AccountId,
    pub recipient: ink::primitives::AccountId,
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
    pub sender: ink::primitives::AccountId,
    pub recipient: ink::primitives::AccountId,
    pub required_signatures: u8,
    pub signatures: Vec<ink::primitives::AccountId>,
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
    pub bridge_contract_address: Option<ink::primitives::AccountId>,
    pub is_active: bool,
    pub gas_multiplier: u32,      // Gas cost multiplier for this chain
    pub confirmation_blocks: u32, // Blocks to wait for confirmation
    pub supported_tokens: Vec<TokenId>,
}

// =============================================================================
// Structured Logging (Issue #107)
// =============================================================================

/// Log severity levels for classifying contract events.
/// Used by off-chain tooling to filter and prioritize event streams.
#[derive(Debug, Clone, Copy, PartialEq, Eq, scale::Encode, scale::Decode)]
#[cfg_attr(
    feature = "std",
    derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
)]
pub enum LogLevel {
    /// Informational events: resource creation, normal state transitions
    Info,
    /// Warning events: unusual conditions that may need attention
    Warning,
    /// Error events: operation failures, rejected transactions
    Error,
    /// Critical events: security-related, admin changes, emergency actions
    Critical,
}

/// Event categories for structured log aggregation and filtering.
#[derive(Debug, Clone, Copy, PartialEq, Eq, scale::Encode, scale::Decode)]
#[cfg_attr(
    feature = "std",
    derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
)]
pub enum EventCategory {
    /// Resource creation: property registered, escrow created, token minted
    Lifecycle,
    /// State mutations: transfers, metadata updates, status changes
    StateChange,
    /// Permission changes: approvals granted or revoked
    Authorization,
    /// Value movements: escrow releases, refunds, fee payments
    Financial,
    /// System operations: pause, resume, upgrades, config changes
    Administrative,
    /// Regulatory and compliance: verification, audit logs, consent
    Audit,
}

// =============================================================================
// Security Audit Trail (Issue #82)
// =============================================================================

/// Security event severity for audit classification.
/// Determines the urgency and attention level for each audit record.
#[derive(Debug, Clone, Copy, PartialEq, Eq, scale::Encode, scale::Decode)]
#[cfg_attr(
    feature = "std",
    derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
)]
pub enum SecuritySeverity {
    /// Normal operations: property registered, metadata updated
    Low,
    /// Ownership/financial state changes: transfers, escrows
    Medium,
    /// Administrative changes: configuration, guardian updates
    High,
    /// Role changes, emergency pauses, admin transfers, access violations
    Critical,
}

/// Classification of security-relevant operations for the audit trail.
#[derive(Debug, Clone, Copy, PartialEq, Eq, scale::Encode, scale::Decode)]
#[cfg_attr(
    feature = "std",
    derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
)]
pub enum SecurityEventType {
    // --- Critical ---
    AdminChanged,
    RoleGranted,
    RoleRevoked,
    ContractPaused,
    ContractResumed,
    EmergencyAction,

    // --- High ---
    ConfigurationChanged,
    PauseGuardianUpdated,
    ComplianceRegistryChanged,
    OracleChanged,
    FeeManagerChanged,

    // --- Medium ---
    PropertyTransferred,
    EscrowCreated,
    EscrowReleased,
    EscrowRefunded,
    FractionalEnabled,
    ApprovalGranted,
    ApprovalCleared,

    // --- Low ---
    PropertyRegistered,
    MetadataUpdated,
    BatchOperation,
    BadgeIssued,
    BadgeRevoked,
    VerificationRequested,
    VerificationReviewed,
    AppealSubmitted,
    AppealResolved,

    // --- Security violations ---
    UnauthorizedAccess,
    ComplianceViolation,
    /// Cryptographic operations: hashing, signature verification, key rotation
    Cryptographic,
}
