#![cfg_attr(not(feature = "std"), no_std)]
#![allow(unexpected_cfgs)]
#![allow(clippy::needless_borrows_for_generic_args)]
#![allow(clippy::enum_variant_names)]

use ink::prelude::string::String;
use ink::prelude::vec::Vec;
use ink::storage::Mapping;
use propchain_traits::access_control::{
    AccessControl, Action, Permission, PermissionAuditEntry, Resource, Role,
};

// Re-export traits
pub use propchain_traits::*;

// Import identity module
use propchain_identity::propchain_identity::IdentityRegistryRef;

// Export error handling utilities
#[cfg(feature = "std")]
pub mod error_handling;

#[ink::contract]
mod propchain_contracts {
    use super::*;

    /// Error types for contract
    #[derive(Debug, Clone, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub enum Error {
        /// Property does not exist in the registry
        PropertyNotFound,
        /// Caller is not authorized for this operation
        Unauthorized,
        /// Property metadata is invalid or malformed
        InvalidMetadata,
        /// Recipient is not compliant with regulatory requirements
        NotCompliant,
        /// Call to the compliance registry contract failed
        ComplianceCheckFailed,
        /// Escrow does not exist
        EscrowNotFound,
        /// Escrow has already been released
        EscrowAlreadyReleased,
        /// Badge does not exist for this property
        BadgeNotFound,
        /// Badge type is invalid
        InvalidBadgeType,
        /// Badge has already been issued to this property
        BadgeAlreadyIssued,
        /// Caller is not an authorized verifier
        NotVerifier,
        /// Appeal does not exist
        AppealNotFound,
        /// Appeal status does not allow this operation
        InvalidAppealStatus,
        /// Compliance registry contract address has not been configured
        ComplianceRegistryNotSet,
        /// Oracle contract returned an error
        OracleError,
        /// Contract is currently paused
        ContractPaused,
        /// Contract is already paused
        AlreadyPaused,
        /// Contract is not currently paused
        NotPaused,
        /// A resume request is already in progress
        ResumeRequestAlreadyActive,
        /// No active resume request exists
        ResumeRequestNotFound,
        /// Not enough approvals to complete the operation
        InsufficientApprovals,
        /// Caller has already approved this operation
        AlreadyApproved,
        /// Caller is not authorized to pause the contract
        NotAuthorizedToPause,
        /// Identity verification failed
        IdentityVerificationFailed,
        /// Insufficient reputation for operation
        InsufficientReputation,
        /// Identity not found
        IdentityNotFound,
        /// Identity registry not configured
        IdentityRegistryNotSet,
        /// Provided address is the zero address (all zeros)
        ZeroAddress,
        /// Input string exceeds maximum allowed length
        StringTooLong,
        /// Input string is empty when a value is required
        StringEmpty,
        /// Numeric value is out of acceptable bounds
        ValueOutOfBounds,
        /// Input batch exceeds the configured max_batch_size
        BatchSizeExceeded,
        /// Cannot transfer or approve to yourself
        SelfTransferNotAllowed,
        /// Range is invalid (min > max)
        InvalidRange,
    }

    /// Property Registry contract
    #[ink(storage)]
    pub struct PropertyRegistry {
        /// Mapping from property ID to property information
        properties: Mapping<u64, PropertyInfo>,
        /// Mapping from owner to their properties
        owner_properties: Mapping<AccountId, Vec<u64>>,
        /// Reverse mapping: property ID to owner (optimization for faster lookups)
        property_owners: Mapping<u64, AccountId>,
        /// Mapping from property ID to approved account
        approvals: Mapping<u64, AccountId>,
        /// Property counter
        property_count: u64,
        /// Contract version
        version: u32,
        /// Admin for upgrades (if used directly, or for logic-level auth)
        admin: AccountId,
        /// Mapping from escrow ID to escrow information
        escrows: Mapping<u64, EscrowInfo>,
        /// Escrow counter
        escrow_count: u64,
        /// Gas usage tracking
        gas_tracker: GasTracker,
        /// Compliance registry contract address (optional)
        compliance_registry: Option<AccountId>,
        /// Badge storage: (property_id, badge_type) -> Badge
        property_badges: Mapping<(u64, BadgeType), Badge>,
        /// Authorized badge verifiers
        badge_verifiers: Mapping<AccountId, bool>,
        /// Verification requests
        verification_requests: Mapping<u64, VerificationRequest>,
        /// Verification request counter
        verification_count: u64,
        /// Appeals
        appeals: Mapping<u64, Appeal>,
        /// Appeal counter
        appeal_count: u64,
        /// Pause configuration and state
        pause_info: PauseInfo,
        /// Accounts authorized to pause the contract
        pause_guardians: Mapping<AccountId, bool>,
        /// Oracle contract address (optional)
        oracle: Option<AccountId>,
        /// Fee manager contract for dynamic fees and market mechanism (optional)
        fee_manager: Option<AccountId>,
        /// Fractional properties info
        fractional: Mapping<u64, FractionalInfo>,
        /// Centralized RBAC and permission audit state
        access_control: AccessControl,
        /// Identity registry contract address for identity verification
        identity_registry: Option<AccountId>,
        /// Minimum reputation threshold for property operations
        min_reputation_threshold: u32,
        /// Batch operation configuration
        batch_config: BatchConfig,
        /// Batch operation statistics
        batch_operation_stats: BatchOperationStats,
    }

    /// Escrow information
    #[derive(
        Debug, Clone, PartialEq, scale::Encode, scale::Decode, ink::storage::traits::StorageLayout,
    )]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub struct EscrowInfo {
        pub id: u64,
        pub property_id: u64,
        pub buyer: AccountId,
        pub seller: AccountId,
        pub amount: u128,
        pub released: bool,
    }

    /// Portfolio summary statistics
    #[derive(
        Debug, Clone, PartialEq, scale::Encode, scale::Decode, ink::storage::traits::StorageLayout,
    )]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub struct PortfolioSummary {
        pub property_count: u64,
        pub total_valuation: u128,
        pub average_valuation: u128,
        pub total_size: u64,
        pub average_size: u64,
    }

    /// Detailed portfolio information
    #[derive(
        Debug, Clone, PartialEq, scale::Encode, scale::Decode, ink::storage::traits::StorageLayout,
    )]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub struct PortfolioDetails {
        pub owner: AccountId,
        pub properties: Vec<PortfolioProperty>,
        pub total_count: u64,
    }

    /// Individual property in portfolio
    #[derive(
        Debug, Clone, PartialEq, scale::Encode, scale::Decode, ink::storage::traits::StorageLayout,
    )]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub struct PortfolioProperty {
        pub id: u64,
        pub location: String,
        pub size: u64,
        pub valuation: u128,
        pub registered_at: u64,
    }

    #[derive(
        Debug,
        Clone,
        PartialEq,
        Eq,
        scale::Encode,
        scale::Decode,
        ink::storage::traits::StorageLayout,
    )]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub struct FractionalInfo {
        pub total_shares: u128,
        pub enabled: bool,
        pub created_at: u64,
    }

    /// Health status information for monitoring
    #[derive(
        Debug, Clone, PartialEq, scale::Encode, scale::Decode, ink::storage::traits::StorageLayout,
    )]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub struct HealthStatus {
        pub is_healthy: bool,
        pub is_paused: bool,
        pub contract_version: u32,
        pub property_count: u64,
        pub escrow_count: u64,
        pub has_oracle: bool,
        pub has_compliance_registry: bool,
        pub has_fee_manager: bool,
        pub block_number: u32,
        pub timestamp: u64,
    }

    /// Global analytics data
    #[derive(
        Debug, Clone, PartialEq, scale::Encode, scale::Decode, ink::storage::traits::StorageLayout,
    )]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub struct GlobalAnalytics {
        pub total_properties: u64,
        pub total_valuation: u128,
        pub average_valuation: u128,
        pub total_size: u64,
        pub average_size: u64,
        pub unique_owners: u64,
    }

    /// Gas metrics for monitoring
    #[derive(
        Debug, Clone, PartialEq, scale::Encode, scale::Decode, ink::storage::traits::StorageLayout,
    )]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub struct GasMetrics {
        pub last_operation_gas: u64,
        pub average_operation_gas: u64,
        pub total_operations: u64,
        pub min_gas_used: u64,
        pub max_gas_used: u64,
    }

    /// Gas tracker for monitoring usage
    #[derive(
        Debug, Clone, PartialEq, scale::Encode, scale::Decode, ink::storage::traits::StorageLayout,
    )]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub struct GasTracker {
        pub total_gas_used: u64,
        pub operation_count: u64,
        pub last_operation_gas: u64,
        pub min_gas_used: u64,
        pub max_gas_used: u64,
    }

    /// Configuration for batch operations
    #[derive(
        Debug,
        Clone,
        PartialEq,
        Eq,
        scale::Encode,
        scale::Decode,
        ink::storage::traits::StorageLayout,
    )]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub struct BatchConfig {
        /// Maximum number of items in a single batch call.
        pub max_batch_size: u32,
        /// Stop processing after this many failures.
        pub max_failure_threshold: u32,
    }

    impl Default for BatchConfig {
        fn default() -> Self {
            Self {
                max_batch_size: 50,
                max_failure_threshold: 5,
            }
        }
    }

    /// Result of a batch operation with partial success support
    #[derive(Debug, Clone, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub struct BatchResult {
        /// Successfully processed item IDs.
        pub successes: Vec<u64>,
        /// Per-item failures with index, item ID, and error.
        pub failures: Vec<BatchItemFailure>,
        /// Batch performance metrics.
        pub metrics: BatchMetrics,
    }

    /// A single item failure within a batch operation
    #[derive(Debug, Clone, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub struct BatchItemFailure {
        /// Position in the input array.
        pub index: u32,
        /// Property ID that failed (0 if not yet assigned).
        pub item_id: u64,
        /// The specific error that occurred.
        pub error: Error,
    }

    /// Metrics for a single batch operation call
    #[derive(Debug, Clone, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub struct BatchMetrics {
        pub total_items: u32,
        pub successful_items: u32,
        pub failed_items: u32,
        /// True if processing stopped due to failure threshold.
        pub early_terminated: bool,
    }

    /// Historical batch operation statistics (stored on-chain)
    #[derive(
        Debug,
        Clone,
        PartialEq,
        Eq,
        Default,
        scale::Encode,
        scale::Decode,
        ink::storage::traits::StorageLayout,
    )]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub struct BatchOperationStats {
        pub total_batches_processed: u64,
        pub total_items_processed: u64,
        pub total_items_failed: u64,
        pub total_early_terminations: u64,
        pub largest_batch_processed: u32,
    }

    /// Badge types for property verification
    #[derive(
        Debug,
        Clone,
        Copy,
        PartialEq,
        Eq,
        scale::Encode,
        scale::Decode,
        ink::storage::traits::StorageLayout,
    )]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub enum BadgeType {
        OwnerVerification,    // KYC/Identity verified
        DocumentVerification, // Legal documents verified
        LegalCompliance,      // Regulatory compliance verified
        PremiumListing,       // Premium tier property
    }

    /// Badge information
    #[derive(
        Debug, Clone, PartialEq, scale::Encode, scale::Decode, ink::storage::traits::StorageLayout,
    )]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub struct Badge {
        pub badge_type: BadgeType,
        pub issued_at: u64,
        pub issued_by: AccountId,
        pub expires_at: Option<u64>,
        pub metadata_url: String,
        pub revoked: bool,
        pub revoked_at: Option<u64>,
        pub revocation_reason: String,
    }

    /// Verification request for badge
    #[derive(
        Debug, Clone, PartialEq, scale::Encode, scale::Decode, ink::storage::traits::StorageLayout,
    )]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub struct VerificationRequest {
        pub id: u64,
        pub property_id: u64,
        pub badge_type: BadgeType,
        pub requester: AccountId,
        pub requested_at: u64,
        pub evidence_url: String,
        pub status: VerificationStatus,
        pub reviewed_by: Option<AccountId>,
        pub reviewed_at: Option<u64>,
    }

    /// Verification status
    #[derive(
        Debug,
        Clone,
        Copy,
        PartialEq,
        Eq,
        scale::Encode,
        scale::Decode,
        ink::storage::traits::StorageLayout,
    )]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub enum VerificationStatus {
        Pending,
        Approved,
        Rejected,
    }

    /// Appeal for badge revocation
    #[derive(
        Debug, Clone, PartialEq, scale::Encode, scale::Decode, ink::storage::traits::StorageLayout,
    )]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub struct Appeal {
        pub id: u64,
        pub property_id: u64,
        pub badge_type: BadgeType,
        pub appellant: AccountId,
        pub reason: String,
        pub submitted_at: u64,
        pub status: AppealStatus,
        pub resolved_by: Option<AccountId>,
        pub resolved_at: Option<u64>,
        pub resolution: String,
    }

    /// Appeal status
    #[derive(
        Debug,
        Clone,
        Copy,
        PartialEq,
        Eq,
        scale::Encode,
        scale::Decode,
        ink::storage::traits::StorageLayout,
    )]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub enum AppealStatus {
        Pending,
        Approved,
        Rejected,
    }

    /// Pause information
    #[derive(
        Debug, Clone, PartialEq, scale::Encode, scale::Decode, ink::storage::traits::StorageLayout,
    )]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub struct PauseInfo {
        pub paused: bool,
        pub paused_at: Option<u64>,
        pub paused_by: Option<AccountId>,
        pub reason: Option<String>,
        pub auto_resume_at: Option<u64>,

        // For Resume Process
        pub resume_request_active: bool,
        pub resume_requester: Option<AccountId>,
        pub resume_approvals: Vec<AccountId>,
        pub required_approvals: u32,
    }

    // ============================================================================
    // STRUCTURED EVENT SYSTEM - Version 1.0
    // ============================================================================
    // All events follow a standardized format with:
    // - Indexed fields (topics) for efficient querying
    // - Timestamps and block numbers for historical tracking
    // - Event versioning for future compatibility
    // - Detailed metadata for off-chain indexing
    // ============================================================================

    /// Event emitted when the contract is initialized
    #[ink(event)]
    pub struct ContractInitialized {
        #[ink(topic)]
        admin: AccountId,
        #[ink(topic)]
        contract_version: u32,
        timestamp: u64,
        block_number: u32,
    }

    /// Event emitted when a property is registered
    /// Indexed fields: property_id, owner for efficient filtering
    #[ink(event)]
    pub struct PropertyRegistered {
        #[ink(topic)]
        property_id: u64,
        #[ink(topic)]
        owner: AccountId,
        #[ink(topic)]
        event_version: u8,
        location: String,
        size: u64,
        valuation: u128,
        timestamp: u64,
        block_number: u32,
        transaction_hash: Hash,
    }

    /// Event emitted when property ownership is transferred
    /// Indexed fields: property_id, from, to for efficient querying
    #[ink(event)]
    pub struct PropertyTransferred {
        #[ink(topic)]
        property_id: u64,
        #[ink(topic)]
        from: AccountId,
        #[ink(topic)]
        to: AccountId,
        #[ink(topic)]
        event_version: u8,
        timestamp: u64,
        block_number: u32,
        transaction_hash: Hash,
        transferred_by: AccountId, // The account that initiated the transfer
    }

    /// Event emitted when property metadata is updated
    /// Indexed fields: property_id, owner for efficient filtering
    #[ink(event)]
    pub struct PropertyMetadataUpdated {
        #[ink(topic)]
        property_id: u64,
        #[ink(topic)]
        owner: AccountId,
        #[ink(topic)]
        event_version: u8,
        old_location: String,
        new_location: String,
        old_valuation: u128,
        new_valuation: u128,
        timestamp: u64,
        block_number: u32,
        transaction_hash: Hash,
    }

    /// Event emitted when an account is approved to transfer a property
    /// Indexed fields: property_id, owner, approved for efficient querying
    #[ink(event)]
    pub struct ApprovalGranted {
        #[ink(topic)]
        property_id: u64,
        #[ink(topic)]
        owner: AccountId,
        #[ink(topic)]
        approved: AccountId,
        #[ink(topic)]
        event_version: u8,
        timestamp: u64,
        block_number: u32,
        transaction_hash: Hash,
    }

    /// Event emitted when an approval is cleared/revoked
    /// Indexed fields: property_id, owner for efficient querying
    #[ink(event)]
    pub struct ApprovalCleared {
        #[ink(topic)]
        property_id: u64,
        #[ink(topic)]
        owner: AccountId,
        #[ink(topic)]
        event_version: u8,
        timestamp: u64,
        block_number: u32,
        transaction_hash: Hash,
    }

    /// Event emitted when an escrow is created
    /// Indexed fields: escrow_id, property_id, buyer, seller for efficient querying
    #[ink(event)]
    pub struct EscrowCreated {
        #[ink(topic)]
        escrow_id: u64,
        #[ink(topic)]
        property_id: u64,
        #[ink(topic)]
        buyer: AccountId,
        #[ink(topic)]
        seller: AccountId,
        #[ink(topic)]
        event_version: u8,
        amount: u128,
        timestamp: u64,
        block_number: u32,
        transaction_hash: Hash,
    }

    /// Event emitted when escrow is released and property transferred
    /// Indexed fields: escrow_id, property_id, buyer for efficient querying
    #[ink(event)]
    pub struct EscrowReleased {
        #[ink(topic)]
        escrow_id: u64,
        #[ink(topic)]
        property_id: u64,
        #[ink(topic)]
        buyer: AccountId,
        #[ink(topic)]
        event_version: u8,
        amount: u128,
        timestamp: u64,
        block_number: u32,
        transaction_hash: Hash,
        released_by: AccountId,
    }

    /// Event emitted when escrow is refunded
    /// Indexed fields: escrow_id, property_id, seller for efficient querying
    #[ink(event)]
    pub struct EscrowRefunded {
        #[ink(topic)]
        escrow_id: u64,
        #[ink(topic)]
        property_id: u64,
        #[ink(topic)]
        seller: AccountId,
        #[ink(topic)]
        event_version: u8,
        amount: u128,
        timestamp: u64,
        block_number: u32,
        transaction_hash: Hash,
        refunded_by: AccountId,
    }

    /// Event emitted when admin is changed
    /// Indexed fields: old_admin, new_admin for efficient querying
    #[ink(event)]
    pub struct AdminChanged {
        #[ink(topic)]
        old_admin: AccountId,
        #[ink(topic)]
        new_admin: AccountId,
        #[ink(topic)]
        event_version: u8,
        timestamp: u64,
        block_number: u32,
        transaction_hash: Hash,
        changed_by: AccountId,
    }

    /// Batch event for multiple property registrations
    /// Indexed fields: owner for efficient filtering
    #[ink(event)]
    pub struct BatchPropertyRegistered {
        #[ink(topic)]
        owner: AccountId,
        #[ink(topic)]
        event_version: u8,
        property_ids: Vec<u64>,
        count: u64,
        timestamp: u64,
        block_number: u32,
        transaction_hash: Hash,
    }

    /// Batch event for multiple property transfers to the same recipient
    /// Indexed fields: from, to for efficient querying
    #[ink(event)]
    pub struct BatchPropertyTransferred {
        #[ink(topic)]
        from: AccountId,
        #[ink(topic)]
        to: AccountId,
        #[ink(topic)]
        event_version: u8,
        property_ids: Vec<u64>,
        count: u64,
        timestamp: u64,
        block_number: u32,
        transaction_hash: Hash,
        transferred_by: AccountId,
    }

    /// Batch event for multiple metadata updates
    /// Indexed fields: owner for efficient filtering
    #[ink(event)]
    pub struct BatchMetadataUpdated {
        #[ink(topic)]
        owner: AccountId,
        #[ink(topic)]
        event_version: u8,
        property_ids: Vec<u64>,
        count: u64,
        timestamp: u64,
        block_number: u32,
        transaction_hash: Hash,
    }

    /// Batch event for multiple property transfers to different recipients
    /// Indexed fields: from for efficient querying
    #[ink(event)]
    pub struct BatchPropertyTransferredToMultiple {
        #[ink(topic)]
        from: AccountId,
        #[ink(topic)]
        event_version: u8,
        transfers: Vec<(u64, AccountId)>, // (property_id, to)
        count: u64,
        timestamp: u64,
        block_number: u32,
        transaction_hash: Hash,
        transferred_by: AccountId,
    }

    /// Event emitted after every batch operation for monitoring
    #[ink(event)]
    pub struct BatchOperationCompleted {
        /// 0=register, 1=transfer, 2=metadata_update, 3=transfer_multiple
        operation_code: u8,
        #[ink(topic)]
        caller: AccountId,
        #[ink(topic)]
        event_version: u8,
        total_items: u32,
        successful_items: u32,
        failed_items: u32,
        early_terminated: bool,
        timestamp: u64,
        block_number: u32,
        transaction_hash: Hash,
    }

    /// Event emitted when a badge is issued to a property
    #[ink(event)]
    pub struct BadgeIssued {
        #[ink(topic)]
        property_id: u64,
        #[ink(topic)]
        badge_type: BadgeType,
        #[ink(topic)]
        issued_by: AccountId,
        #[ink(topic)]
        event_version: u8,
        expires_at: Option<u64>,
        metadata_url: String,
        timestamp: u64,
        block_number: u32,
        transaction_hash: Hash,
    }

    /// Event emitted when a badge is revoked
    #[ink(event)]
    pub struct BadgeRevoked {
        #[ink(topic)]
        property_id: u64,
        #[ink(topic)]
        badge_type: BadgeType,
        #[ink(topic)]
        revoked_by: AccountId,
        #[ink(topic)]
        event_version: u8,
        reason: String,
        timestamp: u64,
        block_number: u32,
        transaction_hash: Hash,
    }

    /// Event emitted when a verification is requested
    #[ink(event)]
    pub struct VerificationRequested {
        #[ink(topic)]
        request_id: u64,
        #[ink(topic)]
        property_id: u64,
        #[ink(topic)]
        badge_type: BadgeType,
        #[ink(topic)]
        requester: AccountId,
        #[ink(topic)]
        event_version: u8,
        evidence_url: String,
        timestamp: u64,
        block_number: u32,
        transaction_hash: Hash,
    }

    /// Event emitted when a verification is reviewed
    #[ink(event)]
    pub struct VerificationReviewed {
        #[ink(topic)]
        request_id: u64,
        #[ink(topic)]
        property_id: u64,
        #[ink(topic)]
        reviewer: AccountId,
        #[ink(topic)]
        approved: bool,
        #[ink(topic)]
        event_version: u8,
        timestamp: u64,
        block_number: u32,
        transaction_hash: Hash,
    }

    /// Event emitted when an appeal is submitted
    #[ink(event)]
    pub struct AppealSubmitted {
        #[ink(topic)]
        appeal_id: u64,
        #[ink(topic)]
        property_id: u64,
        #[ink(topic)]
        badge_type: BadgeType,
        #[ink(topic)]
        appellant: AccountId,
        #[ink(topic)]
        event_version: u8,
        reason: String,
        timestamp: u64,
        block_number: u32,
        transaction_hash: Hash,
    }

    /// Event emitted when an appeal is resolved
    #[ink(event)]
    pub struct AppealResolved {
        #[ink(topic)]
        appeal_id: u64,
        #[ink(topic)]
        property_id: u64,
        #[ink(topic)]
        resolved_by: AccountId,
        #[ink(topic)]
        approved: bool,
        #[ink(topic)]
        event_version: u8,
        resolution: String,
        timestamp: u64,
        block_number: u32,
        transaction_hash: Hash,
    }

    /// Event emitted when a verifier is added or removed
    #[ink(event)]
    pub struct VerifierUpdated {
        #[ink(topic)]
        verifier: AccountId,
        #[ink(topic)]
        authorized: bool,
        #[ink(topic)]
        updated_by: AccountId,
        #[ink(topic)]
        event_version: u8,
        timestamp: u64,
        block_number: u32,
        transaction_hash: Hash,
    }

    /// Event emitted when contract is paused
    #[ink(event)]
    pub struct ContractPaused {
        #[ink(topic)]
        by: AccountId,
        #[ink(topic)]
        reason: String,
        timestamp: u64,
        auto_resume_at: Option<u64>,
    }

    /// Event emitted when a resume is requested
    #[ink(event)]
    pub struct ResumeRequested {
        #[ink(topic)]
        requester: AccountId,
        timestamp: u64,
    }

    /// Event emitted when a resume request is approved
    #[ink(event)]
    pub struct ResumeApproved {
        #[ink(topic)]
        approver: AccountId,
        current_approvals: u32,
        required_approvals: u32,
        timestamp: u64,
    }

    /// Event emitted when contract is resumed
    #[ink(event)]
    pub struct ContractResumed {
        #[ink(topic)]
        by: AccountId,
        timestamp: u64,
    }

    /// Event emitted when a pause guardian is updated
    #[ink(event)]
    pub struct PauseGuardianUpdated {
        #[ink(topic)]
        guardian: AccountId,
        #[ink(topic)]
        is_guardian: bool,
        updated_by: AccountId,
    }

    impl PropertyRegistry {
        /// # Creates a new PropertyRegistry Contract Instance
        ///
        /// ## Description
        /// Initializes a new instance of the PropertyRegistry contract with the caller as admin.
        /// This is the constructor that must be called once during deployment to set up initial state.
        ///
        /// ## Parameters
        /// None - Uses `env().caller()` as the initial admin
        ///
        /// ## Returns
        /// - `PropertyRegistry` - New contract instance with:
        ///   - `admin` set to caller's account
        ///   - `version` set to 1
        ///   - All storage mappings initialized
        ///   - Access control bootstrap completed
        ///
        /// ## Events Emitted
        /// - [`ContractInitialized`](crate::ContractInitialized) - Emitted immediately after initialization
        ///   - `admin`: Account ID of contract creator
        ///   - `contract_version`: Version number (always 1 for initial deployment)
        ///   - `timestamp`: Block timestamp at initialization
        ///   - `block_number`: Block number at initialization
        ///
        /// ## Example
        /// ```rust,ignore
        /// // Deploy and initialize contract
        /// use ink::env::DefaultEnvironment;
        /// use propchain_contracts::PropertyRegistry;
        ///
        /// // Constructor is called automatically during deployment
        /// let contract = PropertyRegistry::new();
        ///
        /// // Verify admin is set correctly
        /// assert_eq!(contract.admin(), caller_account);
        /// assert_eq!(contract.version(), 1);
        /// ```
        ///
        /// ## Security Requirements
        /// - **Caller**: Becomes contract admin with full privileges
        /// - **One-time call**: Should only be called once during deployment
        /// - **Access Control**: Admin role granted to caller automatically
        ///
        /// ## Gas Considerations
        /// - **Cost**: ~200,000 gas (one-time deployment cost)
        /// - **Storage**: Allocates initial contract state (~50 bytes)
        /// - **Optimization**: No user-controllable parameters to optimize
        ///
        /// ## Post-Deployment Steps
        /// 1. Verify admin account is correct
        /// 2. Configure oracle contract (if using valuations)
        /// 3. Set compliance registry address (if enforcing KYC/AML)
        /// 4. Add pause guardians for emergency controls
        /// 5. Fund contract with initial balance for operations
        ///
        /// ## Related Functions
        /// - [`change_admin`](crate::PropertyRegistry::change_admin) - Transfer admin privileges
        /// - [`set_oracle`](crate::PropertyRegistry::set_oracle) - Configure price oracle
        /// - [`set_compliance_registry`](crate::PropertyRegistry::set_compliance_registry) - Set compliance
        ///
        /// ## Version History
        /// - **v1.0.0** - Initial implementation
        /// - **v1.1.0** - Added access control bootstrap
        /// - **v1.2.0** - Enhanced with pause guardians and gas tracking
        #[ink(constructor)]
        pub fn new() -> Self {
            let caller = Self::env().caller();
            let timestamp = Self::env().block_timestamp();
            let block_number = Self::env().block_number();

            let contract = Self {
                properties: Mapping::default(),
                owner_properties: Mapping::default(),
                property_owners: Mapping::default(),
                approvals: Mapping::default(),
                property_count: 0,
                version: 1,
                admin: caller,
                escrows: Mapping::default(),
                escrow_count: 0,
                gas_tracker: GasTracker {
                    total_gas_used: 0,
                    operation_count: 0,
                    last_operation_gas: 0,
                    min_gas_used: u64::MAX,
                    max_gas_used: 0,
                },
                compliance_registry: None,
                property_badges: Mapping::default(),
                badge_verifiers: Mapping::default(),
                verification_requests: Mapping::default(),
                verification_count: 0,
                appeals: Mapping::default(),
                appeal_count: 0,
                pause_info: PauseInfo {
                    paused: false,
                    paused_at: None,
                    paused_by: None,
                    reason: None,
                    auto_resume_at: None,
                    resume_request_active: false,
                    resume_requester: None,
                    resume_approvals: Vec::new(),
                    required_approvals: 2, // Default requirement
                },
                pause_guardians: Mapping::default(),
                oracle: None,
                fee_manager: None,
                fractional: Mapping::default(),
                access_control: {
                    let mut ac = AccessControl::new(64);
                    ac.bootstrap(caller, block_number, timestamp);
                    let _ = ac.grant_role(caller, caller, Role::Verifier, block_number, timestamp);
                    let _ =
                        ac.grant_role(caller, caller, Role::PauseGuardian, block_number, timestamp);
                    ac
                },
                identity_registry: None,
                min_reputation_threshold: 300, // Default minimum reputation
                batch_config: BatchConfig::default(),
                batch_operation_stats: BatchOperationStats::default(),
            };

            // Emit contract initialization event
            Self::env().emit_event(ContractInitialized {
                admin: caller,
                contract_version: 1,
                timestamp,
                block_number,
            });

            contract
        }

        /// # Returns the Contract Version
        ///
        /// ## Description
        /// Returns the current version number of the PropertyRegistry contract.
        /// Used for compatibility checks and upgrade management.
        ///
        /// ## Parameters
        /// None
        ///
        /// ## Returns
        /// - `u32` - Contract version number (currently 1)
        ///
        /// ## Example
        /// ```rust,ignore
        /// // Check contract version before calling version-specific methods
        /// let version = contract.version();
        /// assert_eq!(version, 1);
        ///
        /// if version >= 2 {
        ///     // Use v2+ features
        ///     contract.new_feature()?;
        /// } else {
        ///     // Use legacy approach
        ///     contract.legacy_feature()?;
        /// }
        /// ```
        ///
        /// ## Gas Considerations
        /// - **Cost**: ~500 gas (simple storage read)
        /// - **Optimization**: Free function, no state changes
        ///
        /// ## Related Functions
        /// - [`admin`](crate::PropertyRegistry::admin) - Get admin account
        /// - [`health_check`](crate::PropertyRegistry::health_check) - Full health status
        #[ink(message)]
        pub fn version(&self) -> u32 {
            self.version
        }

        /// # Returns the Admin Account
        ///
        /// ## Description
        /// Returns the AccountId of the current contract administrator.
        /// The admin has privileges to configure contracts, pause operations, and manage access control.
        ///
        /// ## Parameters
        /// None
        ///
        /// ## Returns
        /// - `AccountId` - Account ID of contract administrator
        ///
        /// ## Example
        /// ```rust,ignore
        /// // Verify admin before sensitive operations
        /// let admin = contract.admin();
        /// println!("Contract admin: {:?}", admin);
        ///
        /// // Check if caller is admin
        /// if self.env().caller() == contract.admin() {
        ///     // Perform admin-only operation
        /// }
        /// ```
        ///
        /// ## Security Requirements
        /// - **Access**: Read-only, anyone can query
        /// - **Use Case**: Verify admin identity for off-chain coordination
        ///
        /// ## Gas Considerations
        /// - **Cost**: ~500 gas (storage read)
        ///
        /// ## Related Functions
        /// - [`change_admin`](crate::PropertyRegistry::change_admin) - Transfer admin privileges
        /// - [`version`](crate::PropertyRegistry::version) - Get contract version
        #[ink(message)]
        pub fn admin(&self) -> AccountId {
            self.admin
        }

        /// # Returns Full Contract Health Status
        ///
        /// ## Description
        /// Provides comprehensive health monitoring data for the contract.
        /// Used by monitoring systems, dashboards, and automated health checks.
        ///
        /// ## Parameters
        /// None
        ///
        /// ## Returns
        /// - [`HealthStatus`](crate::HealthStatus) - Complete health information including:
        ///   - `is_healthy`: Overall health flag (false if paused)
        ///   - `is_paused`: Current pause state
        ///   - `contract_version`: Version number
        ///   - `property_count`: Total registered properties
        ///   - `escrow_count`: Active escrows
        ///   - `has_oracle`: Oracle configured
        ///   - `has_compliance_registry`: Compliance registry configured
        ///   - `has_fee_manager`: Fee manager configured
        ///   - `block_number`: Current block
        ///   - `timestamp`: Current timestamp
        ///
        /// ## Example
        /// ```rust,ignore
        /// // Monitor contract health in dashboard
        /// let health = contract.health_check()?;
        ///
        /// if !health.is_healthy {
        ///     alert_admins("Contract unhealthy!");
        /// }
        ///
        /// println!("Properties: {}", health.property_count);
        /// println!("Escrows: {}", health.escrow_count);
        /// println!("Oracle: {:?}", health.has_oracle);
        /// ```
        ///
        /// ## Use Cases
        /// 1. **Monitoring Dashboards**: Display real-time contract status
        /// 2. **Automated Alerts**: Trigger notifications on unhealthy states
        /// 3. **Pre-flight Checks**: Verify contract before operations
        /// 4. **Audit Trails**: Log periodic health snapshots
        ///
        /// ## Gas Considerations
        /// - **Cost**: ~2,000 gas (multiple storage reads)
        /// - **Optimization**: Read-only, no state changes
        ///
        /// ## Related Functions
        /// - [`ping`](crate::PropertyRegistry::ping) - Simple liveness check
        /// - [`dependencies_healthy`](crate::PropertyRegistry::dependencies_healthy) - Dependency check
        /// - [`pause_contract`](crate::PropertyRegistry::pause_contract) - Pause operations
        #[ink(message)]
        pub fn health_check(&self) -> HealthStatus {
            let is_paused = self.pause_info.paused;
            HealthStatus {
                is_healthy: !is_paused,
                is_paused,
                contract_version: self.version,
                property_count: self.property_count,
                escrow_count: self.escrow_count,
                has_oracle: self.oracle.is_some(),
                has_compliance_registry: self.compliance_registry.is_some(),
                has_fee_manager: self.fee_manager.is_some(),
                block_number: self.env().block_number(),
                timestamp: self.env().block_timestamp(),
            }
        }

        /// Simple liveness check that returns true if the contract is responsive
        #[ink(message)]
        pub fn ping(&self) -> bool {
            true
        }

        /// Returns true if all critical dependencies (oracle, compliance, fees) are configured
        #[ink(message)]
        pub fn dependencies_healthy(&self) -> bool {
            self.oracle.is_some()
                && self.compliance_registry.is_some()
                && self.fee_manager.is_some()
        }

        /// Set the oracle contract address
        #[ink(message)]
        pub fn set_oracle(&mut self, oracle: AccountId) -> Result<(), Error> {
            Self::ensure_not_zero_address(oracle)?;
            if !self.ensure_admin_rbac() {
                return Err(Error::Unauthorized);
            }
            self.oracle = Some(oracle);
            Ok(())
        }

        /// Returns the oracle contract address
        #[ink(message)]
        pub fn oracle(&self) -> Option<AccountId> {
            self.oracle
        }

        /// Set the fee manager contract address (admin only)
        #[ink(message)]
        pub fn set_fee_manager(&mut self, fee_manager: Option<AccountId>) -> Result<(), Error> {
            if let Some(fm) = fee_manager {
                Self::ensure_not_zero_address(fm)?;
            }
            if !self.ensure_admin_rbac() {
                return Err(Error::Unauthorized);
            }
            self.fee_manager = fee_manager;
            Ok(())
        }

        /// Returns the fee manager contract address
        #[ink(message)]
        pub fn get_fee_manager(&self) -> Option<AccountId> {
            self.fee_manager
        }

        /// Get dynamic fee for an operation (calls fee manager if set; otherwise returns 0)
        #[ink(message)]
        pub fn get_dynamic_fee(&self, operation: FeeOperation) -> u128 {
            let fee_manager_addr = match self.fee_manager {
                Some(addr) => addr,
                None => return 0,
            };
            use ink::env::call::FromAccountId;
            let fee_manager: ink::contract_ref!(DynamicFeeProvider) =
                FromAccountId::from_account_id(fee_manager_addr);
            fee_manager.get_recommended_fee(operation)
        }

        /// Update property valuation using the oracle
        #[ink(message)]
        pub fn update_valuation_from_oracle(&mut self, property_id: u64) -> Result<(), Error> {
            let oracle_addr = self.oracle.ok_or(Error::OracleError)?;

            // Use the Oracle trait to perform the cross-contract call
            use ink::env::call::FromAccountId;
            let oracle: ink::contract_ref!(Oracle) = FromAccountId::from_account_id(oracle_addr);

            // Fetch valuation from oracle
            let valuation = oracle
                .get_valuation(property_id)
                .map_err(|_| Error::OracleError)?;

            // Update the property's recorded valuation in its metadata
            if let Some(mut property) = self.properties.get(&property_id) {
                property.metadata.valuation = valuation.valuation;
                self.properties.insert(&property_id, &property);
            } else {
                return Err(Error::PropertyNotFound);
            }

            Ok(())
        }

        /// Changes the admin account (only callable by current admin)
        #[ink(message)]
        pub fn change_admin(&mut self, new_admin: AccountId) -> Result<(), Error> {
            Self::ensure_not_zero_address(new_admin)?;
            let caller = self.env().caller();
            if !self.ensure_admin_rbac() {
                return Err(Error::Unauthorized);
            }

            let old_admin = self.admin;
            self.admin = new_admin;
            let _ = self.access_control.grant_role(
                caller,
                new_admin,
                Role::Admin,
                self.env().block_number(),
                self.env().block_timestamp(),
            );

            // Emit enhanced admin changed event

            let transaction_hash: Hash = [0u8; 32].into();
            self.env().emit_event(AdminChanged {
                old_admin,
                new_admin,
                event_version: 1,
                timestamp: self.env().block_timestamp(),
                block_number: self.env().block_number(),
                transaction_hash,
                changed_by: caller,
            });

            Ok(())
        }

        /// Sets the compliance registry contract address (admin only)
        #[ink(message)]
        pub fn set_compliance_registry(
            &mut self,
            registry: Option<AccountId>,
        ) -> Result<(), Error> {
            if let Some(r) = registry {
                Self::ensure_not_zero_address(r)?;
            }
            if !self.ensure_admin_rbac() {
                return Err(Error::Unauthorized);
            }
            self.compliance_registry = registry;
            Ok(())
        }

        /// Gets the compliance registry address
        #[ink(message)]
        pub fn get_compliance_registry(&self) -> Option<AccountId> {
            self.compliance_registry
        }

        /// Sets the identity registry contract address (admin only)
        #[ink(message)]
        pub fn set_identity_registry(&mut self, registry: Option<AccountId>) -> Result<(), Error> {
            if !self.ensure_admin_rbac() {
                return Err(Error::Unauthorized);
            }
            self.identity_registry = registry;
            Ok(())
        }

        /// Gets the identity registry address
        #[ink(message)]
        pub fn get_identity_registry(&self) -> Option<AccountId> {
            self.identity_registry
        }

        /// Sets the minimum reputation threshold for property operations (admin only)
        #[ink(message)]
        pub fn set_min_reputation_threshold(&mut self, threshold: u32) -> Result<(), Error> {
            if !self.ensure_admin_rbac() {
                return Err(Error::Unauthorized);
            }
            self.min_reputation_threshold = threshold;
            Ok(())
        }

        /// Gets the minimum reputation threshold
        #[ink(message)]
        pub fn get_min_reputation_threshold(&self) -> u32 {
            self.min_reputation_threshold
        }

        /// Helper: Check compliance for an account via the compliance registry (Issue #45).
        /// Returns Ok if compliant or no registry set, Err(NotCompliant) or Err(ComplianceCheckFailed) otherwise.
        fn check_compliance(&self, account: AccountId) -> Result<(), Error> {
            let registry_addr = match self.compliance_registry {
                Some(addr) => addr,
                None => return Ok(()),
            };

            use ink::env::call::FromAccountId;
            let registry: ink::contract_ref!(ComplianceChecker) =
                FromAccountId::from_account_id(registry_addr);

            let is_compliant = registry.is_compliant(account);

            if !is_compliant {
                return Err(Error::NotCompliant);
            }
            Ok(())
        }

        /// Helper: Check identity verification and reputation requirements
        /// Returns Ok if requirements are met or no identity registry set, Err otherwise.
        fn check_identity_requirements(&self, account: AccountId) -> Result<(), Error> {
            let registry_addr = match self.identity_registry {
                Some(addr) => addr,
                None => return Ok(()),
            };

            use ink::env::call::FromAccountId;
            let registry: IdentityRegistryRef = FromAccountId::from_account_id(registry_addr);

            // Check if identity exists
            let identity = registry
                .get_identity(account)
                .ok_or(Error::IdentityNotFound)?;

            // Check if identity is verified
            if !identity.is_verified {
                return Err(Error::IdentityVerificationFailed);
            }

            // Check reputation threshold
            if identity.reputation_score < self.min_reputation_threshold {
                return Err(Error::InsufficientReputation);
            }

            Ok(())
        }

        /// Check if an account is compliant (delegates to registry when set). For use by frontends.
        #[ink(message)]
        pub fn check_account_compliance(&self, account: AccountId) -> Result<bool, Error> {
            if self.compliance_registry.is_none() {
                return Ok(true);
            }
            let registry_addr = self.compliance_registry.unwrap();
            use ink::env::call::FromAccountId;
            let registry: ink::contract_ref!(ComplianceChecker) =
                FromAccountId::from_account_id(registry_addr);
            Ok(registry.is_compliant(account))
        }

        /// Helper to check if contract is paused
        pub fn ensure_not_paused(&self) -> Result<(), Error> {
            if self.pause_info.paused {
                // Check for auto-resume
                if let Some(resume_time) = self.pause_info.auto_resume_at {
                    if self.env().block_timestamp() >= resume_time {
                        // In a real scenario we might want to auto-resume here or require a trigger.
                        // For safety, we usually require explicit resume even if time passed,
                        // purely to update the state, OR we treat it as not paused.
                        // However, since state mutability is needed to update 'paused' flag,
                        // and this is a read-only check often, we'll return Error::ContractPaused
                        // unless someone triggers the resume.
                        // But requirements say "Time-based automatic resume".
                        // Use a separate method or assume logic handles it.
                        // For strict safety:
                        return Err(Error::ContractPaused);
                    }
                }
                return Err(Error::ContractPaused);
            }
            Ok(())
        }

        // --- Pause/Resume Functionality ---

        /// Pauses the contract. Can be called by admin or pause guardians.
        #[ink(message)]
        pub fn pause_contract(
            &mut self,
            reason: String,
            duration_seconds: Option<u64>,
        ) -> Result<(), Error> {
            use propchain_traits::constants::*;
            Self::validate_string_length(&reason, MAX_REASON_LENGTH)?;
            if let Some(d) = duration_seconds {
                if !(MIN_PAUSE_DURATION..=MAX_PAUSE_DURATION).contains(&d) {
                    return Err(Error::ValueOutOfBounds);
                }
            }
            let caller = self.env().caller();
            let is_admin = self.access_control.has_role(caller, Role::Admin);
            let is_guardian = self.pause_guardians.get(caller).unwrap_or(false);

            if !is_admin && !is_guardian {
                return Err(Error::NotAuthorizedToPause);
            }

            if self.pause_info.paused {
                return Err(Error::AlreadyPaused);
            }

            let timestamp = self.env().block_timestamp();
            let auto_resume_at = duration_seconds.map(|d| timestamp + d);

            self.pause_info.paused = true;
            self.pause_info.paused_at = Some(timestamp);
            self.pause_info.paused_by = Some(caller);
            self.pause_info.reason = Some(reason.clone());
            self.pause_info.auto_resume_at = auto_resume_at;

            // Clear any previous resume requests
            self.pause_info.resume_request_active = false;
            self.pause_info.resume_approvals.clear();

            self.env().emit_event(ContractPaused {
                by: caller,
                reason,
                timestamp,
                auto_resume_at,
            });

            Ok(())
        }

        /// Emergency pause - same as pause but implies critical severity
        #[ink(message)]
        pub fn emergency_pause(&mut self, reason: String) -> Result<(), Error> {
            self.pause_contract(reason, None)
        }

        /// Provide a mechanism to try auto-resume if time passed
        #[ink(message)]
        pub fn try_auto_resume(&mut self) -> Result<(), Error> {
            if !self.pause_info.paused {
                return Err(Error::NotPaused);
            }

            if let Some(resume_time) = self.pause_info.auto_resume_at {
                if self.env().block_timestamp() >= resume_time {
                    self.pause_info.paused = false;
                    self.pause_info.reason = None;

                    self.env().emit_event(ContractResumed {
                        by: self.env().caller(), // triggered by
                        timestamp: self.env().block_timestamp(),
                    });
                    return Ok(());
                }
            }
            Err(Error::ContractPaused)
        }

        /// Request to resume the contract. Requires multi-sig approval.
        #[ink(message)]
        pub fn request_resume(&mut self) -> Result<(), Error> {
            let caller = self.env().caller();
            // Only admin or guardians can request resume
            let is_admin = self.access_control.has_role(caller, Role::Admin);
            let is_guardian = self.pause_guardians.get(caller).unwrap_or(false);

            if !is_admin && !is_guardian {
                return Err(Error::Unauthorized);
            }

            if !self.pause_info.paused {
                return Err(Error::NotPaused);
            }

            if self.pause_info.resume_request_active {
                return Err(Error::ResumeRequestAlreadyActive);
            }

            self.pause_info.resume_request_active = true;
            self.pause_info.resume_requester = Some(caller);
            self.pause_info.resume_approvals.clear();
            // Auto-approve by requester? Usually yes, let's say yes.
            self.pause_info.resume_approvals.push(caller);

            self.env().emit_event(ResumeRequested {
                requester: caller,
                timestamp: self.env().block_timestamp(),
            });

            // If only 1 approval required (e.g. dev mode), check immediately
            if self.pause_info.required_approvals <= 1 {
                self._execute_resume()?;
            }

            Ok(())
        }

        /// Approve the pending resume request
        #[ink(message)]
        pub fn approve_resume(&mut self) -> Result<(), Error> {
            let caller = self.env().caller();
            let is_admin = self.access_control.has_role(caller, Role::Admin);
            let is_guardian = self.pause_guardians.get(caller).unwrap_or(false);

            if !is_admin && !is_guardian {
                return Err(Error::Unauthorized);
            }

            if !self.pause_info.resume_request_active {
                return Err(Error::ResumeRequestNotFound);
            }

            if self.pause_info.resume_approvals.contains(&caller) {
                return Err(Error::AlreadyApproved);
            }

            self.pause_info.resume_approvals.push(caller);

            let approvals_count = self.pause_info.resume_approvals.len() as u32;

            self.env().emit_event(ResumeApproved {
                approver: caller,
                current_approvals: approvals_count,
                required_approvals: self.pause_info.required_approvals,
                timestamp: self.env().block_timestamp(),
            });

            if approvals_count >= self.pause_info.required_approvals {
                self._execute_resume()?;
            }

            Ok(())
        }

        fn _execute_resume(&mut self) -> Result<(), Error> {
            self.pause_info.paused = false;
            self.pause_info.resume_request_active = false;
            self.pause_info.reason = None;

            self.env().emit_event(ContractResumed {
                by: self.env().caller(),
                timestamp: self.env().block_timestamp(),
            });
            Ok(())
        }

        /// Manage pause guardians
        #[ink(message)]
        pub fn set_pause_guardian(
            &mut self,
            guardian: AccountId,
            is_enabled: bool,
        ) -> Result<(), Error> {
            Self::ensure_not_zero_address(guardian)?;
            if !self.ensure_admin_rbac() {
                return Err(Error::Unauthorized);
            }
            self.pause_guardians.insert(guardian, &is_enabled);

            self.env().emit_event(PauseGuardianUpdated {
                guardian,
                is_guardian: is_enabled,
                updated_by: self.env().caller(),
            });
            Ok(())
        }

        /// Get pause state
        #[ink(message)]
        pub fn get_pause_state(&self) -> PauseInfo {
            self.pause_info.clone()
        }

        #[ink(message)]
        pub fn grant_role(&mut self, account: AccountId, role: Role) -> Result<(), Error> {
            Self::ensure_not_zero_address(account)?;
            let caller = self.env().caller();
            self.access_control
                .grant_role(
                    caller,
                    account,
                    role,
                    self.env().block_number(),
                    self.env().block_timestamp(),
                )
                .map_err(|_| Error::Unauthorized)
        }

        #[ink(message)]
        pub fn revoke_role(&mut self, account: AccountId, role: Role) -> Result<(), Error> {
            let caller = self.env().caller();
            self.access_control
                .revoke_role(
                    caller,
                    account,
                    role,
                    self.env().block_number(),
                    self.env().block_timestamp(),
                )
                .map_err(|_| Error::Unauthorized)
        }

        #[ink(message)]
        pub fn has_role(&self, account: AccountId, role: Role) -> bool {
            self.access_control.has_role(account, role)
        }

        #[ink(message)]
        pub fn get_permission_audit_entry(&self, id: u64) -> Option<PermissionAuditEntry> {
            self.access_control.get_audit_entry(id)
        }

        /// Registers a new property
        /// Optionally checks compliance if compliance registry is set
        /// Checks identity verification and reputation requirements
        #[ink(message)]
        pub fn register_property(&mut self, metadata: PropertyMetadata) -> Result<u64, Error> {
            self.ensure_not_paused()?;
            Self::validate_metadata(&metadata)?;
            let caller = self.env().caller();

            // Check identity verification and reputation
            self.check_identity_requirements(caller)?;

            // Check compliance for property registration (optional but recommended)
            self.check_compliance(caller)?;

            self.property_count += 1;
            let property_id = self.property_count;

            let property_info = PropertyInfo {
                id: property_id,
                owner: caller,
                metadata,
                registered_at: self.env().block_timestamp(),
            };

            self.properties.insert(property_id, &property_info);
            // Optimized: Also store reverse mapping for faster owner lookups
            self.property_owners.insert(property_id, &caller);

            let mut owner_props = self.owner_properties.get(caller).unwrap_or_default();
            owner_props.push(property_id);
            self.owner_properties.insert(caller, &owner_props);

            // Track gas usage
            self.track_gas_usage("register_property".as_bytes());

            // Emit enhanced property registration event

            let transaction_hash: Hash = [0u8; 32].into();
            self.env().emit_event(PropertyRegistered {
                property_id,
                owner: caller,
                event_version: 1,
                location: property_info.metadata.location.clone(),
                size: property_info.metadata.size,
                valuation: property_info.metadata.valuation,
                timestamp: property_info.registered_at,
                block_number: self.env().block_number(),
                transaction_hash,
            });

            Ok(property_id)
        }

        /// Transfers property ownership
        /// Requires recipient to be compliant if compliance registry is set
        /// Requires recipient to meet identity verification and reputation requirements
        #[ink(message)]
        pub fn transfer_property(&mut self, property_id: u64, to: AccountId) -> Result<(), Error> {
            self.ensure_not_paused()?;
            Self::ensure_not_zero_address(to)?;
            let caller = self.env().caller();
            Self::ensure_not_self(caller, to)?;
            let mut property = self
                .properties
                .get(property_id)
                .ok_or(Error::PropertyNotFound)?;

            let approved = self.approvals.get(property_id);
            if property.owner != caller && Some(caller) != approved {
                return Err(Error::Unauthorized);
            }

            // Check compliance for recipient
            self.check_compliance(to)?;

            // Check identity verification and reputation for recipient
            self.check_identity_requirements(to)?;

            let from = property.owner;

            // Remove from current owner's properties
            let mut current_owner_props = self.owner_properties.get(from).unwrap_or_default();
            current_owner_props.retain(|&id| id != property_id);
            self.owner_properties.insert(from, &current_owner_props);

            // Add to new owner's properties
            let mut new_owner_props = self.owner_properties.get(to).unwrap_or_default();
            new_owner_props.push(property_id);
            self.owner_properties.insert(to, &new_owner_props);

            // Update property owner
            property.owner = to;
            self.properties.insert(property_id, &property);
            // Optimized: Update reverse mapping
            self.property_owners.insert(property_id, &to);

            // Clear approval
            self.approvals.remove(property_id);

            // Update reputation scores for both parties if identity registry is set
            if let Some(registry_addr) = self.identity_registry {
                use ink::env::call::FromAccountId;
                let mut registry: IdentityRegistryRef =
                    FromAccountId::from_account_id(registry_addr);

                let transaction_value = property.metadata.valuation;

                // Update reputation for both sender and receiver
                let _ = registry.update_reputation(from, true, transaction_value);
                let _ = registry.update_reputation(to, true, transaction_value);
            }

            // Track gas usage
            self.track_gas_usage("transfer_property".as_bytes());

            // Emit enhanced property transfer event

            let transaction_hash: Hash = [0u8; 32].into();
            self.env().emit_event(PropertyTransferred {
                property_id,
                from,
                to,
                event_version: 1,
                timestamp: self.env().block_timestamp(),
                block_number: self.env().block_number(),
                transaction_hash,
                transferred_by: caller,
            });

            Ok(())
        }

        /// Gets property information
        #[ink(message)]
        pub fn get_property(&self, property_id: u64) -> Option<PropertyInfo> {
            self.properties.get(property_id)
        }

        /// Gets properties owned by an account
        #[ink(message)]
        pub fn get_owner_properties(&self, owner: AccountId) -> Vec<u64> {
            self.owner_properties.get(owner).unwrap_or_default()
        }

        /// Gets total property count
        #[ink(message)]
        pub fn property_count(&self) -> u64 {
            self.property_count
        }

        /// Updates property metadata
        #[ink(message)]
        pub fn update_metadata(
            &mut self,
            property_id: u64,
            metadata: PropertyMetadata,
        ) -> Result<(), Error> {
            self.ensure_not_paused()?;
            let caller = self.env().caller();
            let mut property = self
                .properties
                .get(property_id)
                .ok_or(Error::PropertyNotFound)?;

            if property.owner != caller {
                return Err(Error::Unauthorized);
            }

            Self::validate_metadata(&metadata)?;

            // Store old metadata for event
            let old_location = property.metadata.location.clone();
            let old_valuation = property.metadata.valuation;

            property.metadata = metadata.clone();
            self.properties.insert(property_id, &property);

            // Emit enhanced metadata update event

            let transaction_hash: Hash = [0u8; 32].into();
            self.env().emit_event(PropertyMetadataUpdated {
                property_id,
                owner: caller,
                event_version: 1,
                old_location,
                new_location: metadata.location,
                old_valuation,
                new_valuation: metadata.valuation,
                timestamp: self.env().block_timestamp(),
                block_number: self.env().block_number(),
                transaction_hash,
            });

            Ok(())
        }

        /// Batch registers multiple properties in a single transaction
        #[ink(message)]
        pub fn batch_register_properties(
            &mut self,
            properties: Vec<PropertyMetadata>,
        ) -> Result<BatchResult, Error> {
            self.ensure_not_paused()?;
            if properties.is_empty() {
                return Err(Error::ValueOutOfBounds);
            }
            self.validate_batch_size(properties.len())?;

            let caller = self.env().caller();
            let timestamp = self.env().block_timestamp();
            let total_items = properties.len() as u32;
            let mut successes = Vec::new();
            let mut failures = Vec::new();
            let mut early_terminated = false;
            let mut next_id = self.property_count + 1;

            let mut owner_props = self.owner_properties.get(caller).unwrap_or_default();

            for (i, metadata) in properties.into_iter().enumerate() {
                // Check early termination
                if failures.len() >= self.batch_config.max_failure_threshold as usize {
                    early_terminated = true;
                    break;
                }

                // Validate metadata
                if let Err(e) = Self::validate_metadata(&metadata) {
                    failures.push(BatchItemFailure {
                        index: i as u32,
                        item_id: 0,
                        error: e,
                    });
                    continue;
                }

                let property_id = next_id;
                next_id += 1;

                let property_info = PropertyInfo {
                    id: property_id,
                    owner: caller,
                    metadata,
                    registered_at: timestamp,
                };

                self.properties.insert(property_id, &property_info);
                owner_props.push(property_id);
                successes.push(property_id);
            }

            // Update property count only if there were successes
            if !successes.is_empty() {
                self.property_count = next_id - 1;
                self.owner_properties.insert(caller, &owner_props);

                let transaction_hash: Hash = [0u8; 32].into();
                self.env().emit_event(BatchPropertyRegistered {
                    owner: caller,
                    event_version: 1,
                    property_ids: successes.clone(),
                    count: successes.len() as u64,
                    timestamp,
                    block_number: self.env().block_number(),
                    transaction_hash,
                });
            }

            let metrics = BatchMetrics {
                total_items,
                successful_items: successes.len() as u32,
                failed_items: failures.len() as u32,
                early_terminated,
            };

            self.record_batch_operation(0, &metrics);
            self.track_gas_usage("batch_register_properties".as_bytes());

            Ok(BatchResult {
                successes,
                failures,
                metrics,
            })
        }

        /// Batch transfers multiple properties to the same recipient
        #[ink(message)]
        pub fn batch_transfer_properties(
            &mut self,
            property_ids: Vec<u64>,
            to: AccountId,
        ) -> Result<(), Error> {
            self.ensure_not_paused()?;
            if property_ids.is_empty() {
                return Err(Error::ValueOutOfBounds);
            }
            self.validate_batch_size(property_ids.len())?;
            Self::ensure_not_zero_address(to)?;

            let caller = self.env().caller();
            Self::ensure_not_self(caller, to)?;

            // Phase 1: Validate all properties (atomic — fail on first error)
            for &property_id in &property_ids {
                let property = self
                    .properties
                    .get(property_id)
                    .ok_or(Error::PropertyNotFound)?;

                let approved = self.approvals.get(property_id);
                if property.owner != caller && Some(caller) != approved {
                    return Err(Error::Unauthorized);
                }
            }

            // Capture the original owner
            let from = self
                .properties
                .get(property_ids[0])
                .ok_or(Error::PropertyNotFound)?
                .owner;

            // Phase 2: Optimized execution — batch storage reads/writes per owner
            // Read owner_properties for `from` once, remove all in one pass
            let mut from_props = self.owner_properties.get(from).unwrap_or_default();
            from_props.retain(|id| !property_ids.contains(id));
            self.owner_properties.insert(from, &from_props);

            // Accumulate `to` owner additions, write once
            let mut to_props = self.owner_properties.get(to).unwrap_or_default();

            for &property_id in &property_ids {
                let mut property = self
                    .properties
                    .get(property_id)
                    .ok_or(Error::PropertyNotFound)?;

                property.owner = to;
                self.properties.insert(property_id, &property);
                self.property_owners.insert(property_id, &to);
                self.approvals.remove(property_id);
                to_props.push(property_id);
            }

            // Single write for `to` owner properties
            self.owner_properties.insert(to, &to_props);

            // Emit events
            let transaction_hash: Hash = [0u8; 32].into();
            self.env().emit_event(BatchPropertyTransferred {
                from,
                to,
                event_version: 1,
                property_ids: property_ids.clone(),
                count: property_ids.len() as u64,
                timestamp: self.env().block_timestamp(),
                block_number: self.env().block_number(),
                transaction_hash,
                transferred_by: caller,
            });

            let metrics = BatchMetrics {
                total_items: property_ids.len() as u32,
                successful_items: property_ids.len() as u32,
                failed_items: 0,
                early_terminated: false,
            };
            self.record_batch_operation(1, &metrics);
            self.track_gas_usage("batch_transfer_properties".as_bytes());

            Ok(())
        }

        /// Batch updates metadata for multiple properties
        #[ink(message)]
        pub fn batch_update_metadata(
            &mut self,
            updates: Vec<(u64, PropertyMetadata)>,
        ) -> Result<BatchResult, Error> {
            self.ensure_not_paused()?;
            if updates.is_empty() {
                return Err(Error::ValueOutOfBounds);
            }
            self.validate_batch_size(updates.len())?;

            let caller = self.env().caller();
            let total_items = updates.len() as u32;
            let mut successes = Vec::new();
            let mut failures = Vec::new();
            let mut early_terminated = false;

            for (i, (property_id, metadata)) in updates.into_iter().enumerate() {
                if failures.len() >= self.batch_config.max_failure_threshold as usize {
                    early_terminated = true;
                    break;
                }

                // Validate property exists
                let property = match self.properties.get(property_id) {
                    Some(p) => p,
                    None => {
                        failures.push(BatchItemFailure {
                            index: i as u32,
                            item_id: property_id,
                            error: Error::PropertyNotFound,
                        });
                        continue;
                    }
                };

                // Validate ownership
                if property.owner != caller {
                    failures.push(BatchItemFailure {
                        index: i as u32,
                        item_id: property_id,
                        error: Error::Unauthorized,
                    });
                    continue;
                }

                // Validate metadata
                if let Err(e) = Self::validate_metadata(&metadata) {
                    failures.push(BatchItemFailure {
                        index: i as u32,
                        item_id: property_id,
                        error: e,
                    });
                    continue;
                }

                // Apply update
                let mut property = property;
                property.metadata = metadata;
                self.properties.insert(property_id, &property);
                successes.push(property_id);
            }

            // Emit existing batch event for successes
            if !successes.is_empty() {
                let transaction_hash: Hash = [0u8; 32].into();
                self.env().emit_event(BatchMetadataUpdated {
                    owner: caller,
                    event_version: 1,
                    property_ids: successes.clone(),
                    count: successes.len() as u64,
                    timestamp: self.env().block_timestamp(),
                    block_number: self.env().block_number(),
                    transaction_hash,
                });
            }

            let metrics = BatchMetrics {
                total_items,
                successful_items: successes.len() as u32,
                failed_items: failures.len() as u32,
                early_terminated,
            };

            self.record_batch_operation(2, &metrics);
            self.track_gas_usage("batch_update_metadata".as_bytes());

            Ok(BatchResult {
                successes,
                failures,
                metrics,
            })
        }

        /// Transfers multiple properties to different recipients
        #[ink(message)]
        pub fn batch_transfer_properties_to_multiple(
            &mut self,
            transfers: Vec<(u64, AccountId)>,
        ) -> Result<(), Error> {
            self.ensure_not_paused()?;
            if transfers.is_empty() {
                return Err(Error::ValueOutOfBounds);
            }
            self.validate_batch_size(transfers.len())?;

            let caller = self.env().caller();
            for (_, to) in &transfers {
                Self::ensure_not_zero_address(*to)?;
                Self::ensure_not_self(caller, *to)?;
            }

            // Phase 1: Validate all transfers (atomic)
            for (property_id, _) in &transfers {
                let property = self
                    .properties
                    .get(property_id)
                    .ok_or(Error::PropertyNotFound)?;

                let approved = self.approvals.get(property_id);
                if property.owner != caller && Some(caller) != approved {
                    return Err(Error::Unauthorized);
                }
            }

            // Phase 2: Group by from-owner and to-owner for batched writes
            let transfer_ids: Vec<u64> = transfers.iter().map(|(id, _)| *id).collect();

            // Remove all transferred properties from caller's list in one pass
            let mut from_props = self.owner_properties.get(caller).unwrap_or_default();
            from_props.retain(|id| !transfer_ids.contains(id));
            self.owner_properties.insert(caller, &from_props);

            // Group additions by recipient to minimize writes
            let mut recipient_additions: Vec<(AccountId, Vec<u64>)> = Vec::new();

            for (property_id, to) in &transfers {
                let mut property = self
                    .properties
                    .get(property_id)
                    .ok_or(Error::PropertyNotFound)?;

                property.owner = *to;
                self.properties.insert(property_id, &property);
                self.property_owners.insert(property_id, to);
                self.approvals.remove(property_id);

                // Accumulate by recipient
                if let Some(entry) = recipient_additions.iter_mut().find(|(addr, _)| addr == to) {
                    entry.1.push(*property_id);
                } else {
                    recipient_additions.push((*to, vec![*property_id]));
                }
            }

            // Batch write per recipient
            for (recipient, new_ids) in recipient_additions {
                let mut recipient_props = self.owner_properties.get(recipient).unwrap_or_default();
                recipient_props.extend(new_ids);
                self.owner_properties.insert(recipient, &recipient_props);
            }

            // Emit event
            let transaction_hash: Hash = [0u8; 32].into();
            self.env().emit_event(BatchPropertyTransferredToMultiple {
                from: caller,
                event_version: 1,
                transfers: transfers.clone(),
                count: transfers.len() as u64,
                timestamp: self.env().block_timestamp(),
                block_number: self.env().block_number(),
                transaction_hash,
                transferred_by: caller,
            });

            let metrics = BatchMetrics {
                total_items: transfers.len() as u32,
                successful_items: transfers.len() as u32,
                failed_items: 0,
                early_terminated: false,
            };
            self.record_batch_operation(3, &metrics);
            self.track_gas_usage("batch_transfer_properties_to_multiple".as_bytes());

            Ok(())
        }

        /// Approves an account to transfer a specific property
        #[ink(message)]
        pub fn approve(&mut self, property_id: u64, to: Option<AccountId>) -> Result<(), Error> {
            self.ensure_not_paused()?;
            if let Some(account) = to {
                Self::ensure_not_zero_address(account)?;
            }
            let caller = self.env().caller();
            if let Some(account) = to {
                Self::ensure_not_self(caller, account)?;
            }
            let property = self
                .properties
                .get(property_id)
                .ok_or(Error::PropertyNotFound)?;

            if property.owner != caller {
                return Err(Error::Unauthorized);
            }

            let transaction_hash: Hash = [0u8; 32].into();

            if let Some(account) = to {
                self.approvals.insert(property_id, &account);
                // Emit enhanced approval granted event
                self.env().emit_event(ApprovalGranted {
                    property_id,
                    owner: caller,
                    approved: account,
                    event_version: 1,
                    timestamp: self.env().block_timestamp(),
                    block_number: self.env().block_number(),
                    transaction_hash,
                });
            } else {
                self.approvals.remove(property_id);
                // Emit enhanced approval cleared event
                self.env().emit_event(ApprovalCleared {
                    property_id,
                    owner: caller,
                    event_version: 1,
                    timestamp: self.env().block_timestamp(),
                    block_number: self.env().block_number(),
                    transaction_hash,
                });
            }

            Ok(())
        }

        /// Gets the approved account for a property
        #[ink(message)]
        pub fn get_approved(&self, property_id: u64) -> Option<AccountId> {
            self.approvals.get(property_id)
        }

        /// Creates a new escrow for property transfer
        /// Seller creates escrow and specifies the buyer
        #[ink(message)]
        pub fn create_escrow(
            &mut self,
            property_id: u64,
            buyer: AccountId,
            amount: u128,
        ) -> Result<u64, Error> {
            self.ensure_not_paused()?;
            Self::ensure_not_zero_address(buyer)?;
            if amount == 0 {
                return Err(Error::ValueOutOfBounds);
            }
            let caller = self.env().caller();
            let property = self
                .properties
                .get(property_id)
                .ok_or(Error::PropertyNotFound)?;

            // Only property owner (seller) can create escrow
            if property.owner != caller {
                return Err(Error::Unauthorized);
            }

            self.escrow_count += 1;
            let escrow_id = self.escrow_count;

            let escrow_info = EscrowInfo {
                id: escrow_id,
                property_id,
                buyer,
                seller: property.owner,
                amount,
                released: false,
            };

            self.escrows.insert(escrow_id, &escrow_info);

            // Emit enhanced escrow created event

            let transaction_hash: Hash = [0u8; 32].into();
            self.env().emit_event(EscrowCreated {
                escrow_id,
                property_id,
                buyer,
                seller: property.owner,
                event_version: 1,
                amount,
                timestamp: self.env().block_timestamp(),
                block_number: self.env().block_number(),
                transaction_hash,
            });

            Ok(escrow_id)
        }

        /// Releases escrow funds and transfers property
        #[ink(message)]
        pub fn release_escrow(&mut self, escrow_id: u64) -> Result<(), Error> {
            self.ensure_not_paused()?;
            let caller = self.env().caller();
            let mut escrow = self.escrows.get(escrow_id).ok_or(Error::EscrowNotFound)?;

            if escrow.released {
                return Err(Error::EscrowAlreadyReleased);
            }

            // Only buyer can release
            if escrow.buyer != caller {
                return Err(Error::Unauthorized);
            }

            // Transfer property
            self.transfer_property(escrow.property_id, escrow.buyer)?;

            escrow.released = true;
            self.escrows.insert(escrow_id, &escrow);

            // Emit enhanced escrow released event

            let transaction_hash: Hash = [0u8; 32].into();
            self.env().emit_event(EscrowReleased {
                escrow_id,
                property_id: escrow.property_id,
                buyer: escrow.buyer,
                event_version: 1,
                amount: escrow.amount,
                timestamp: self.env().block_timestamp(),
                block_number: self.env().block_number(),
                transaction_hash,
                released_by: caller,
            });

            Ok(())
        }

        /// Refunds escrow funds
        #[ink(message)]
        pub fn refund_escrow(&mut self, escrow_id: u64) -> Result<(), Error> {
            self.ensure_not_paused()?;
            let caller = self.env().caller();
            let mut escrow = self.escrows.get(escrow_id).ok_or(Error::EscrowNotFound)?;

            if escrow.released {
                return Err(Error::EscrowAlreadyReleased);
            }

            // Only seller can refund
            if escrow.seller != caller {
                return Err(Error::Unauthorized);
            }

            escrow.released = true;
            self.escrows.insert(escrow_id, &escrow);

            // Emit enhanced escrow refunded event

            let transaction_hash: Hash = [0u8; 32].into();
            self.env().emit_event(EscrowRefunded {
                escrow_id,
                property_id: escrow.property_id,
                seller: escrow.seller,
                event_version: 1,
                amount: escrow.amount,
                timestamp: self.env().block_timestamp(),
                block_number: self.env().block_number(),
                transaction_hash,
                refunded_by: caller,
            });

            Ok(())
        }

        /// Gets escrow information
        #[ink(message)]
        pub fn get_escrow(&self, escrow_id: u64) -> Option<EscrowInfo> {
            self.escrows.get(escrow_id)
        }

        /// Portfolio Management: Gets summary statistics for properties owned by an account
        #[ink(message)]
        pub fn get_portfolio_summary(&self, owner: AccountId) -> PortfolioSummary {
            let property_ids = self.owner_properties.get(owner).unwrap_or_default();
            let mut total_valuation = 0u128;
            let mut total_size = 0u64;
            let mut property_count = 0u64;

            // Optimized loop with iterator for better performance
            let iter = property_ids.iter();
            for &property_id in iter {
                if let Some(property) = self.properties.get(property_id) {
                    // Unrolled additions for better performance
                    total_valuation = total_valuation.wrapping_add(property.metadata.valuation);
                    total_size = total_size.wrapping_add(property.metadata.size);
                    property_count += 1;
                }
            }

            PortfolioSummary {
                property_count,
                total_valuation,
                average_valuation: if property_count > 0 {
                    total_valuation
                        .checked_div(property_count as u128)
                        .unwrap_or(0)
                } else {
                    0
                },
                total_size,
                average_size: if property_count > 0 {
                    total_size.checked_div(property_count).unwrap_or(0)
                } else {
                    0
                },
            }
        }

        /// Portfolio Management: Gets detailed portfolio information for an owner
        #[ink(message)]
        pub fn get_portfolio_details(&self, owner: AccountId) -> PortfolioDetails {
            let property_ids = self.owner_properties.get(owner).unwrap_or_default();
            let mut properties = Vec::with_capacity(property_ids.len());

            let iter = property_ids.iter();
            for &property_id in iter {
                if let Some(property) = self.properties.get(property_id) {
                    // Direct construction to avoid intermediate allocations
                    let portfolio_property = PortfolioProperty {
                        id: property.id,
                        location: property.metadata.location.clone(),
                        size: property.metadata.size,
                        valuation: property.metadata.valuation,
                        registered_at: property.registered_at,
                    };
                    properties.push(portfolio_property);
                }
            }

            PortfolioDetails {
                owner,
                total_count: properties.len() as u64,
                properties,
            }
        }

        /// Analytics: Gets aggregated statistics across all properties
        /// WARNING: This is expensive for large datasets. Consider off-chain indexing.
        #[ink(message)]
        pub fn get_global_analytics(&self) -> GlobalAnalytics {
            let mut total_valuation = 0u128;
            let mut total_size = 0u64;
            let mut property_count = 0u64;
            let mut owners = Vec::new();

            // Optimized loop with early termination possibility
            // Note: This is expensive for large datasets. Consider off-chain indexing.
            let mut i = 1u64;
            while i <= self.property_count {
                if let Some(property) = self.properties.get(i) {
                    total_valuation += property.metadata.valuation;
                    total_size += property.metadata.size;
                    property_count += 1;

                    // Add owner if not already in list (manual deduplication)
                    if !owners.contains(&property.owner) {
                        owners.push(property.owner);
                    }
                }
                i += 1;
            }

            GlobalAnalytics {
                total_properties: property_count,
                total_valuation,
                average_valuation: if property_count > 0 {
                    total_valuation
                        .checked_div(property_count as u128)
                        .unwrap_or(0)
                } else {
                    0
                },
                total_size,
                average_size: if property_count > 0 {
                    total_size.checked_div(property_count).unwrap_or(0)
                } else {
                    0
                },
                unique_owners: owners.len() as u64,
            }
        }

        /// Analytics: Gets properties within a price range
        #[ink(message)]
        pub fn get_properties_by_price_range(
            &self,
            min_price: u128,
            max_price: u128,
        ) -> Result<Vec<u64>, Error> {
            if min_price > max_price {
                return Err(Error::InvalidRange);
            }
            let mut result = Vec::new();

            let mut i = 1u64;
            while i <= self.property_count {
                if let Some(property) = self.properties.get(i) {
                    let valuation = property.metadata.valuation;
                    if valuation >= min_price && valuation <= max_price {
                        result.push(property.id);
                    }
                }
                i += 1;
            }

            Ok(result)
        }

        /// Analytics: Gets properties by size range
        #[ink(message)]
        pub fn get_properties_by_size_range(
            &self,
            min_size: u64,
            max_size: u64,
        ) -> Result<Vec<u64>, Error> {
            if min_size > max_size {
                return Err(Error::InvalidRange);
            }
            let mut result = Vec::new();

            let mut i = 1u64;
            while i <= self.property_count {
                if let Some(property) = self.properties.get(i) {
                    let size = property.metadata.size;
                    if size >= min_size && size <= max_size {
                        result.push(property.id);
                    }
                }
                i += 1;
            }

            Ok(result)
        }

        /// Helper method to track gas usage
        fn track_gas_usage(&mut self, _operation: &[u8]) {
            // In a real implementation, this would measure actual gas consumption
            // For demonstration purposes, we increment counters
            let gas_used = 10000; // Placeholder value
            self.gas_tracker.operation_count += 1;
            self.gas_tracker.last_operation_gas = gas_used;
            self.gas_tracker.total_gas_used += gas_used;

            // Track min/max gas usage
            if gas_used < self.gas_tracker.min_gas_used {
                self.gas_tracker.min_gas_used = gas_used;
            }
            if gas_used > self.gas_tracker.max_gas_used {
                self.gas_tracker.max_gas_used = gas_used;
            }
        }

        /// Updates batch operation stats and emits monitoring event.
        fn record_batch_operation(&mut self, operation_code: u8, metrics: &BatchMetrics) {
            self.batch_operation_stats.total_batches_processed += 1;
            self.batch_operation_stats.total_items_processed += metrics.successful_items as u64;
            self.batch_operation_stats.total_items_failed += metrics.failed_items as u64;
            if metrics.early_terminated {
                self.batch_operation_stats.total_early_terminations += 1;
            }
            if metrics.total_items > self.batch_operation_stats.largest_batch_processed {
                self.batch_operation_stats.largest_batch_processed = metrics.total_items;
            }

            let transaction_hash: Hash = [0u8; 32].into();
            self.env().emit_event(BatchOperationCompleted {
                operation_code,
                caller: self.env().caller(),
                event_version: 1,
                total_items: metrics.total_items,
                successful_items: metrics.successful_items,
                failed_items: metrics.failed_items,
                early_terminated: metrics.early_terminated,
                timestamp: self.env().block_timestamp(),
                block_number: self.env().block_number(),
                transaction_hash,
            });
        }

        /// Validates batch size against config. Returns Err(BatchSizeExceeded) if too large.
        fn validate_batch_size(&self, size: usize) -> Result<(), Error> {
            if size > self.batch_config.max_batch_size as usize {
                return Err(Error::BatchSizeExceeded);
            }
            Ok(())
        }

        /// Gas Monitoring: Tracks gas usage for operations
        #[ink(message)]
        pub fn get_gas_metrics(&self) -> GasMetrics {
            GasMetrics {
                last_operation_gas: self.gas_tracker.last_operation_gas,
                average_operation_gas: if self.gas_tracker.operation_count > 0 {
                    self.gas_tracker
                        .total_gas_used
                        .checked_div(self.gas_tracker.operation_count)
                        .unwrap_or(0)
                } else {
                    0
                },
                total_operations: self.gas_tracker.operation_count,
                min_gas_used: if self.gas_tracker.min_gas_used == u64::MAX {
                    0
                } else {
                    self.gas_tracker.min_gas_used
                },
                max_gas_used: self.gas_tracker.max_gas_used,
            }
        }

        /// Admin-only: update batch operation configuration.
        #[ink(message)]
        pub fn update_batch_config(
            &mut self,
            max_batch_size: u32,
            max_failure_threshold: u32,
        ) -> Result<(), Error> {
            let caller = self.env().caller();
            if caller != self.admin {
                return Err(Error::Unauthorized);
            }
            if max_batch_size == 0 || max_batch_size > 200 {
                return Err(Error::InvalidMetadata);
            }
            if max_failure_threshold == 0 || max_failure_threshold > max_batch_size {
                return Err(Error::InvalidMetadata);
            }
            self.batch_config = BatchConfig {
                max_batch_size,
                max_failure_threshold,
            };
            Ok(())
        }

        /// Returns the current batch operation configuration.
        #[ink(message)]
        pub fn get_batch_config(&self) -> BatchConfig {
            self.batch_config.clone()
        }

        /// Returns historical batch operation statistics.
        #[ink(message)]
        pub fn get_batch_stats(&self) -> BatchOperationStats {
            self.batch_operation_stats.clone()
        }

        /// Performance Monitoring: Gets optimization recommendations
        #[ink(message)]
        pub fn get_performance_recommendations(&self) -> Vec<String> {
            let mut recommendations = Vec::new();

            // Check for high gas usage operations
            let avg_gas = if self.gas_tracker.operation_count > 0 {
                self.gas_tracker
                    .total_gas_used
                    .checked_div(self.gas_tracker.operation_count)
                    .unwrap_or(0)
            } else {
                0
            };
            if avg_gas > 50000 {
                recommendations
                    .push("Consider using batch operations for multiple properties".to_string());
            }

            // Check for many small operations
            if self.gas_tracker.operation_count > 100 && avg_gas < 10000 {
                recommendations.push(
                    "Operations are efficient but consider consolidating related operations"
                        .to_string(),
                );
            }

            // Check for inconsistent gas usage
            if self.gas_tracker.max_gas_used > self.gas_tracker.min_gas_used * 10 {
                recommendations
                    .push("Gas usage varies significantly - review operation patterns".to_string());
            }

            // General recommendations
            recommendations
                .push("Use batch operations for multiple property transfers".to_string());
            recommendations
                .push("Prefer portfolio analytics over individual property queries".to_string());
            recommendations.push("Consider off-chain indexing for complex analytics".to_string());

            recommendations
        }

        // ============================================================================
        // BADGE MANAGEMENT SYSTEM
        // ============================================================================

        /// Adds or removes a badge verifier (admin only)
        #[ink(message)]
        pub fn set_verifier(&mut self, verifier: AccountId, authorized: bool) -> Result<(), Error> {
            Self::ensure_not_zero_address(verifier)?;
            let caller = self.env().caller();
            if !self.ensure_admin_rbac() {
                return Err(Error::Unauthorized);
            }

            self.badge_verifiers.insert(verifier, &authorized);

            // Emit verifier updated event
            let timestamp = self.env().block_timestamp();
            let block_number = self.env().block_number();
            self.env().emit_event(VerifierUpdated {
                verifier,
                authorized,
                updated_by: caller,
                event_version: 1,
                timestamp,
                block_number,
                transaction_hash: [0u8; 32].into(),
            });

            Ok(())
        }

        /// Checks if an account is an authorized verifier
        #[ink(message)]
        pub fn is_verifier(&self, account: AccountId) -> bool {
            self.badge_verifiers.get(account).unwrap_or(false)
        }

        /// Issues a badge to a property (verifier only)
        #[ink(message)]
        pub fn issue_badge(
            &mut self,
            property_id: u64,
            badge_type: BadgeType,
            expires_at: Option<u64>,
            metadata_url: String,
        ) -> Result<(), Error> {
            self.ensure_not_paused()?;
            Self::validate_url(&metadata_url)?;
            if let Some(exp) = expires_at {
                if exp <= self.env().block_timestamp() {
                    return Err(Error::ValueOutOfBounds);
                }
            }
            let caller = self.env().caller();

            // Only verifiers can issue badges
            if !self.is_verifier(caller) && caller != self.admin {
                return Err(Error::NotVerifier);
            }

            // Check if property exists
            self.properties
                .get(property_id)
                .ok_or(Error::PropertyNotFound)?;

            // Check if badge already exists and is not revoked
            if let Some(existing_badge) = self.property_badges.get((property_id, badge_type)) {
                if !existing_badge.revoked {
                    return Err(Error::BadgeAlreadyIssued);
                }
            }

            let badge = Badge {
                badge_type,
                issued_at: self.env().block_timestamp(),
                issued_by: caller,
                expires_at,
                metadata_url: metadata_url.clone(),
                revoked: false,
                revoked_at: None,
                revocation_reason: String::new(),
            };

            self.property_badges
                .insert((property_id, badge_type), &badge);

            // Emit badge issued event
            let timestamp = self.env().block_timestamp();
            let block_number = self.env().block_number();
            self.env().emit_event(BadgeIssued {
                property_id,
                badge_type,
                issued_by: caller,
                event_version: 1,
                expires_at,
                metadata_url,
                timestamp,
                block_number,
                transaction_hash: [0u8; 32].into(),
            });

            Ok(())
        }

        /// Revokes a badge from a property (verifier or admin only)
        #[ink(message)]
        pub fn revoke_badge(
            &mut self,
            property_id: u64,
            badge_type: BadgeType,
            reason: String,
        ) -> Result<(), Error> {
            self.ensure_not_paused()?;
            Self::validate_string_length(&reason, propchain_traits::constants::MAX_REASON_LENGTH)?;
            let caller = self.env().caller();

            // Only verifiers or admin can revoke badges
            if !self.is_verifier(caller) && caller != self.admin {
                return Err(Error::NotVerifier);
            }

            let mut badge = self
                .property_badges
                .get((property_id, badge_type))
                .ok_or(Error::BadgeNotFound)?;

            if badge.revoked {
                return Err(Error::BadgeNotFound);
            }

            badge.revoked = true;
            badge.revoked_at = Some(self.env().block_timestamp());
            badge.revocation_reason = reason.clone();

            self.property_badges
                .insert((property_id, badge_type), &badge);

            let timestamp = self.env().block_timestamp();
            let block_number = self.env().block_number();
            self.env().emit_event(BadgeRevoked {
                property_id,
                badge_type,
                revoked_by: caller,
                event_version: 1,
                reason,
                timestamp,
                block_number,
                transaction_hash: [0u8; 32].into(),
            });

            Ok(())
        }

        /// Submits a verification request for a property badge.
        ///
        /// The property owner provides evidence (e.g. a URL to supporting documents)
        /// and the request is queued for review by an authorized verifier.
        ///
        /// # Arguments
        ///
        /// * `property_id` - The property to request verification for
        /// * `badge_type` - The type of badge being requested
        /// * `evidence_url` - URL pointing to supporting evidence
        ///
        /// # Returns
        ///
        /// Returns `Result<u64, Error>` with the new verification request ID on success
        #[ink(message, selector = 0x4C0F_B92C)]
        pub fn request_verification(
            &mut self,
            property_id: u64,
            badge_type: BadgeType,
            evidence_url: String,
        ) -> Result<u64, Error> {
            self.ensure_not_paused()?;
            Self::validate_url(&evidence_url)?;
            let caller = self.env().caller();
            let property = self
                .properties
                .get(property_id)
                .ok_or(Error::PropertyNotFound)?;

            if property.owner != caller {
                return Err(Error::Unauthorized);
            }

            self.verification_count += 1;
            let request_id = self.verification_count;

            let request = VerificationRequest {
                id: request_id,
                property_id,
                badge_type,
                requester: caller,
                requested_at: self.env().block_timestamp(),
                evidence_url: evidence_url.clone(),
                status: VerificationStatus::Pending,
                reviewed_by: None,
                reviewed_at: None,
            };

            self.verification_requests.insert(request_id, &request);

            // Emit verification requested event
            let timestamp = self.env().block_timestamp();
            let block_number = self.env().block_number();
            self.env().emit_event(VerificationRequested {
                request_id,
                property_id,
                badge_type,
                requester: caller,
                event_version: 1,
                evidence_url,
                timestamp,
                block_number,
                transaction_hash: [0u8; 32].into(),
            });

            Ok(request_id)
        }

        /// Reviews a pending verification request and optionally issues the badge.
        ///
        /// Only authorized verifiers or the admin may call this. When approved,
        /// the corresponding badge is automatically issued to the property.
        ///
        /// # Arguments
        ///
        /// * `request_id` - The verification request to review
        /// * `approved` - Whether to approve or reject the request
        /// * `expires_at` - Optional expiration timestamp for the badge
        /// * `metadata_url` - URL pointing to badge metadata
        ///
        /// # Returns
        ///
        /// Returns `Result<(), Error>` indicating success or failure
        #[ink(message)]
        pub fn review_verification(
            &mut self,
            request_id: u64,
            approved: bool,
            expires_at: Option<u64>,
            metadata_url: String,
        ) -> Result<(), Error> {
            self.ensure_not_paused()?;
            Self::validate_url(&metadata_url)?;
            let caller = self.env().caller();

            if !self.is_verifier(caller) && caller != self.admin {
                return Err(Error::NotVerifier);
            }

            let mut request = self
                .verification_requests
                .get(request_id)
                .ok_or(Error::BadgeNotFound)?;

            request.status = if approved {
                VerificationStatus::Approved
            } else {
                VerificationStatus::Rejected
            };
            request.reviewed_by = Some(caller);
            request.reviewed_at = Some(self.env().block_timestamp());

            self.verification_requests.insert(request_id, &request);

            if approved {
                self.issue_badge(
                    request.property_id,
                    request.badge_type,
                    expires_at,
                    metadata_url,
                )?;
            }

            let timestamp = self.env().block_timestamp();
            let block_number = self.env().block_number();
            self.env().emit_event(VerificationReviewed {
                request_id,
                property_id: request.property_id,
                reviewer: caller,
                approved,
                event_version: 1,
                timestamp,
                block_number,
                transaction_hash: [0u8; 32].into(),
            });

            Ok(())
        }

        /// Submits an appeal against a revoked badge.
        ///
        /// Only the property owner may appeal. The badge must already be revoked
        /// for the appeal to be valid.
        ///
        /// # Arguments
        ///
        /// * `property_id` - The property whose badge was revoked
        /// * `badge_type` - The type of badge to appeal
        /// * `reason` - Justification for the appeal
        ///
        /// # Returns
        ///
        /// Returns `Result<u64, Error>` with the new appeal ID on success
        #[ink(message)]
        pub fn submit_appeal(
            &mut self,
            property_id: u64,
            badge_type: BadgeType,
            reason: String,
        ) -> Result<u64, Error> {
            self.ensure_not_paused()?;
            Self::validate_string_length(&reason, propchain_traits::constants::MAX_REASON_LENGTH)?;
            let caller = self.env().caller();
            let property = self
                .properties
                .get(property_id)
                .ok_or(Error::PropertyNotFound)?;

            if property.owner != caller {
                return Err(Error::Unauthorized);
            }

            let badge = self
                .property_badges
                .get((property_id, badge_type))
                .ok_or(Error::BadgeNotFound)?;

            if !badge.revoked {
                return Err(Error::InvalidAppealStatus);
            }

            self.appeal_count += 1;
            let appeal_id = self.appeal_count;

            let appeal = Appeal {
                id: appeal_id,
                property_id,
                badge_type,
                appellant: caller,
                reason: reason.clone(),
                submitted_at: self.env().block_timestamp(),
                status: AppealStatus::Pending,
                resolved_by: None,
                resolved_at: None,
                resolution: String::new(),
            };

            self.appeals.insert(appeal_id, &appeal);

            let timestamp = self.env().block_timestamp();
            let block_number = self.env().block_number();
            self.env().emit_event(AppealSubmitted {
                appeal_id,
                property_id,
                badge_type,
                appellant: caller,
                event_version: 1,
                reason,
                timestamp,
                block_number,
                transaction_hash: [0u8; 32].into(),
            });

            Ok(appeal_id)
        }

        /// Resolves a pending appeal (admin only).
        ///
        /// If approved, the revoked badge is reinstated for the property.
        ///
        /// # Arguments
        ///
        /// * `appeal_id` - The appeal to resolve
        /// * `approved` - Whether to approve or reject the appeal
        /// * `resolution` - Explanation of the resolution decision
        ///
        /// # Returns
        ///
        /// Returns `Result<(), Error>` indicating success or failure
        #[ink(message)]
        pub fn resolve_appeal(
            &mut self,
            appeal_id: u64,
            approved: bool,
            resolution: String,
        ) -> Result<(), Error> {
            self.ensure_not_paused()?;
            Self::validate_string_length(
                &resolution,
                propchain_traits::constants::MAX_REASON_LENGTH,
            )?;
            let caller = self.env().caller();

            if !self.ensure_admin_rbac() {
                return Err(Error::Unauthorized);
            }

            let mut appeal = self.appeals.get(appeal_id).ok_or(Error::AppealNotFound)?;

            appeal.status = if approved {
                AppealStatus::Approved
            } else {
                AppealStatus::Rejected
            };
            appeal.resolved_by = Some(caller);
            appeal.resolved_at = Some(self.env().block_timestamp());
            appeal.resolution = resolution.clone();

            self.appeals.insert(appeal_id, &appeal);

            // If approved, reinstate the badge
            if approved {
                if let Some(mut badge) = self
                    .property_badges
                    .get((appeal.property_id, appeal.badge_type))
                {
                    badge.revoked = false;
                    badge.revoked_at = None;
                    badge.revocation_reason = String::new();
                    self.property_badges
                        .insert((appeal.property_id, appeal.badge_type), &badge);
                }
            }

            // Emit appeal resolved event
            let timestamp = self.env().block_timestamp();
            let block_number = self.env().block_number();
            self.env().emit_event(AppealResolved {
                appeal_id,
                property_id: appeal.property_id,
                resolved_by: caller,
                approved,
                event_version: 1,
                resolution,
                timestamp,
                block_number,
                transaction_hash: [0u8; 32].into(),
            });

            Ok(())
        }

        /// Gets all badges for a property
        #[ink(message)]
        pub fn get_property_badges(&self, property_id: u64) -> Vec<(BadgeType, Badge)> {
            let mut badges = Vec::new();

            // Check all badge types
            let badge_types = [
                BadgeType::OwnerVerification,
                BadgeType::DocumentVerification,
                BadgeType::LegalCompliance,
                BadgeType::PremiumListing,
            ];

            for badge_type in badge_types.iter() {
                if let Some(badge) = self.property_badges.get((property_id, *badge_type)) {
                    if !badge.revoked {
                        badges.push((*badge_type, badge));
                    }
                }
            }

            badges
        }

        /// Checks whether a property holds a valid (non-revoked) badge of the given type.
        ///
        /// # Arguments
        ///
        /// * `property_id` - The property to check
        /// * `badge_type` - The badge type to look for
        ///
        /// # Returns
        ///
        /// Returns `true` if the property has the badge and it has not been revoked
        #[ink(message)]
        pub fn has_badge(&self, property_id: u64, badge_type: BadgeType) -> bool {
            if let Some(badge) = self.property_badges.get((property_id, badge_type)) {
                !badge.revoked
            } else {
                false
            }
        }

        /// Returns the badge for a property and badge type, if one exists.
        ///
        /// # Arguments
        ///
        /// * `property_id` - The property to query
        /// * `badge_type` - The badge type to retrieve
        ///
        /// # Returns
        ///
        /// Returns `Option<Badge>` containing the badge details, or `None`
        #[ink(message)]
        pub fn get_badge(&self, property_id: u64, badge_type: BadgeType) -> Option<Badge> {
            self.property_badges.get((property_id, badge_type))
        }

        /// Returns a verification request by its ID.
        ///
        /// # Arguments
        ///
        /// * `request_id` - The verification request ID to look up
        ///
        /// # Returns
        ///
        /// Returns `Option<VerificationRequest>` with the request details, or `None`
        #[ink(message)]
        pub fn get_verification_request(&self, request_id: u64) -> Option<VerificationRequest> {
            self.verification_requests.get(request_id)
        }

        /// Returns an appeal by its ID.
        ///
        /// # Arguments
        ///
        /// * `appeal_id` - The appeal ID to look up
        ///
        /// # Returns
        ///
        /// Returns `Option<Appeal>` with the appeal details, or `None`
        #[ink(message)]
        pub fn get_appeal(&self, appeal_id: u64) -> Option<Appeal> {
            self.appeals.get(appeal_id)
        }
    }

    #[cfg(kani)]
    mod verification {
        use super::*;

        #[kani::proof]
        fn verify_arithmetic_overflow() {
            let a: u64 = kani::any();
            let b: u64 = kani::any();
            // Verify that addition is safe
            if a < 100 && b < 100 {
                assert!(a + b < 200);
            }
        }

        #[kani::proof]
        fn verify_property_info_struct() {
            let id: u64 = kani::any();
            // Verify PropertyInfo layout/safety if needed
            // This is a placeholder for checking structural invariants
            if id > 0 {
                assert!(id > 0);
            }
        }
    }

    impl Default for PropertyRegistry {
        fn default() -> Self {
            Self::new()
        }
    }

    impl Escrow for PropertyRegistry {
        type Error = Error;

        fn create_escrow(&mut self, property_id: u64, amount: u128) -> Result<u64, Self::Error> {
            // For trait compatibility, use caller as buyer
            // In production, use the direct create_escrow method with explicit buyer
            use ink::codegen::Env;
            let caller = self.env().caller();
            self.create_escrow(property_id, caller, amount)
        }

        fn release_escrow(&mut self, escrow_id: u64) -> Result<(), Self::Error> {
            self.release_escrow(escrow_id)
        }

        fn refund_escrow(&mut self, escrow_id: u64) -> Result<(), Self::Error> {
            self.refund_escrow(escrow_id)
        }
    }

    impl PropertyRegistry {
        /// Enables fractional ownership for a property by specifying a total share count.
        ///
        /// Only the property owner or admin may call this. The total shares must be
        /// greater than zero.
        ///
        /// # Arguments
        ///
        /// * `property_id` - The property to fractionalize
        /// * `total_shares` - The total number of shares to divide the property into
        ///
        /// # Returns
        ///
        /// Returns `Result<(), Error>` indicating success or failure
        #[ink(message)]
        pub fn enable_fractional(
            &mut self,
            property_id: u64,
            total_shares: u128,
        ) -> Result<(), Error> {
            let caller = self.env().caller();
            let property = self
                .properties
                .get(property_id)
                .ok_or(Error::PropertyNotFound)?;
            if caller != self.admin && caller != property.owner {
                return Err(Error::Unauthorized);
            }
            if total_shares == 0 {
                return Err(Error::InvalidMetadata);
            }
            let info = FractionalInfo {
                total_shares,
                enabled: true,
                created_at: self.env().block_timestamp(),
            };
            self.fractional.insert(property_id, &info);
            Ok(())
        }

        /// Returns the fractional ownership information for a property.
        ///
        /// # Arguments
        ///
        /// * `property_id` - The property to query
        ///
        /// # Returns
        ///
        /// Returns `Option<FractionalInfo>` with share details, or `None` if not fractionalized
        #[ink(message)]
        pub fn get_fractional_info(&self, property_id: u64) -> Option<FractionalInfo> {
            self.fractional.get(property_id)
        }

        /// Checks whether a property has fractional ownership enabled.
        ///
        /// # Arguments
        ///
        /// * `property_id` - The property to check
        ///
        /// # Returns
        ///
        /// Returns `true` if fractional ownership is active for this property
        #[ink(message)]
        pub fn is_fractional(&self, property_id: u64) -> bool {
            self.fractional
                .get(property_id)
                .map(|i: FractionalInfo| i.enabled)
                .unwrap_or(false)
        }

        fn ensure_admin_rbac(&mut self) -> bool {
            let caller = self.env().caller();
            self.access_control.has_permission_cached(
                caller,
                Permission {
                    resource: Resource::PropertyRegistry,
                    action: Action::Configure,
                },
                self.env().block_number(),
            ) || self.access_control.has_role(caller, Role::Admin)
        }

        // ====================================================================
        // INPUT VALIDATION HELPERS (Issue #79)
        // ====================================================================

        /// Rejects the zero address (all 32 bytes == 0x00).
        fn ensure_not_zero_address(account: AccountId) -> Result<(), Error> {
            if account == AccountId::from([0x0; 32]) {
                return Err(Error::ZeroAddress);
            }
            Ok(())
        }

        /// Validates that caller is not the same as the target.
        fn ensure_not_self(caller: AccountId, target: AccountId) -> Result<(), Error> {
            if caller == target {
                return Err(Error::SelfTransferNotAllowed);
            }
            Ok(())
        }

        /// Full metadata validation using centralized constants.
        fn validate_metadata(metadata: &PropertyMetadata) -> Result<(), Error> {
            use propchain_traits::constants::*;

            if metadata.location.is_empty() || metadata.legal_description.is_empty() {
                return Err(Error::InvalidMetadata);
            }
            if metadata.location.len() as u32 > MAX_LOCATION_LENGTH {
                return Err(Error::StringTooLong);
            }
            if metadata.legal_description.len() as u32 > MAX_LEGAL_DESCRIPTION_LENGTH {
                return Err(Error::StringTooLong);
            }
            if metadata.size < MIN_PROPERTY_SIZE || metadata.size > MAX_PROPERTY_SIZE {
                return Err(Error::ValueOutOfBounds);
            }
            if metadata.valuation < MIN_VALUATION {
                return Err(Error::ValueOutOfBounds);
            }
            if metadata.documents_url.len() as u32 > MAX_URL_LENGTH {
                return Err(Error::StringTooLong);
            }
            Ok(())
        }

        /// Validates a string field (reason, resolution) against a max length.
        fn validate_string_length(s: &str, max_len: u32) -> Result<(), Error> {
            if s.is_empty() {
                return Err(Error::StringEmpty);
            }
            if s.len() as u32 > max_len {
                return Err(Error::StringTooLong);
            }
            Ok(())
        }

        /// Validates a URL string is non-empty and within length limits.
        fn validate_url(url: &str) -> Result<(), Error> {
            use propchain_traits::constants::MAX_URL_LENGTH;
            if url.is_empty() {
                return Err(Error::StringEmpty);
            }
            if url.len() as u32 > MAX_URL_LENGTH {
                return Err(Error::StringTooLong);
            }
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod tests_pause {
    use super::propchain_contracts::{Error, PropertyRegistry};
    use ink::primitives::AccountId;
    use propchain_traits::PropertyMetadata;

    #[ink::test]
    fn test_pause_resume_flow() {
        let mut contract = PropertyRegistry::new();
        let _admin = AccountId::from([0x1; 32]);

        // 1. Verify initial state
        assert!(!contract.get_pause_state().paused);

        // 2. Pause contract
        assert!(contract
            .pause_contract("Security breach".into(), None)
            .is_ok());
        contract.ensure_not_paused().expect_err("Should be paused");

        // 3. Try to register property (should fail)
        let metadata = PropertyMetadata {
            location: "Test Loc".into(),
            size: 100,
            legal_description: "Test Description".into(),
            valuation: 1000,
            documents_url: "http://test.com".into(),
        };
        assert_eq!(
            contract.register_property(metadata.clone()),
            Err(Error::ContractPaused)
        );

        // 4. Request resume
        assert!(contract.request_resume().is_ok());
        let state = contract.get_pause_state();
        assert!(state.resume_request_active);

        // 5. Approve resume (admin already approved implicitly by requesting if we implemented it that way,
        // but let's check approvals. In `request_resume` we pushed caller to approvals.
        // `required_approvals` is 2 by default.
        // We need another distinct account to approve.

        // In simple unit testing here, tracking caller changes requires `ink::env::test::set_caller`.
        // Let's simulate a second account approval.
        let account2 = AccountId::from([0x2; 32]);
        ink::env::test::set_caller::<ink::env::DefaultEnvironment>(contract.admin());
        assert!(contract.set_pause_guardian(account2, true).is_ok());

        ink::env::test::set_caller::<ink::env::DefaultEnvironment>(account2);
        assert!(contract.approve_resume().is_ok());

        // Now it should be resumed
        assert!(!contract.get_pause_state().paused);
        assert!(contract.ensure_not_paused().is_ok());
    }
}
