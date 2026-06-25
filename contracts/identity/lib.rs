#![cfg_attr(not(feature = "std"), no_std)]
#![allow(unexpected_cfgs)]
#![allow(clippy::needless_borrows_for_generic_args)]
#![allow(clippy::enum_variant_names)]
#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::arithmetic_side_effects)]
#![allow(clippy::cast_sign_loss)]
#![allow(clippy::unnecessary_lazy_evaluations)]
#![allow(clippy::unnecessary_cast)]

use ink::prelude::string::String;
use ink::prelude::vec::Vec;
use ink::storage::Mapping;
use propchain_traits::*;

/// Cross-chain identity and reputation system for trusted property transactions
#[ink::contract]
pub mod propchain_identity {
    use super::*;

    /// Identity verification errors
    #[derive(Debug, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub enum IdentityError {
        /// Identity does not exist
        IdentityNotFound,
        /// Caller is not authorized for this operation
        Unauthorized,
        /// Invalid cryptographic signature
        InvalidSignature,
        /// Identity verification failed
        VerificationFailed,
        /// Insufficient reputation score
        InsufficientReputation,
        /// Recovery process already in progress
        RecoveryInProgress,
        /// No recovery process active
        RecoveryNotActive,
        /// Invalid recovery parameters
        InvalidRecoveryParams,
        /// Identity already exists
        IdentityAlreadyExists,
        /// Invalid DID format
        InvalidDid,
        /// Social recovery threshold not met
        RecoveryThresholdNotMet,
        /// Privacy verification failed
        PrivacyVerificationFailed,
        /// Chain not supported for cross-chain operations
        UnsupportedChain,
        /// Cross-chain verification failed
        CrossChainVerificationFailed,
        /// Identity has been revoked
        IdentityRevoked,
        /// Requested resource not found
        NotFound,
        /// Operation blocked by active timelock
        TimelockActive,
        /// Resource already exists or operation already performed
        AlreadyExists,
    }

    /// Audit trail entry for identity operations
    #[derive(
        Debug, Clone, PartialEq, scale::Encode, scale::Decode, ink::storage::traits::StorageLayout,
    )]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub struct AuditEntry {
        pub entry_id: u64,
        pub account: AccountId,
        pub action: String,
        pub performed_by: AccountId,
        pub timestamp: u64,
        pub details: String,
    }

    /// Reason for identity revocation
    #[derive(
        Debug, Clone, Copy, PartialEq, Eq, scale::Encode, scale::Decode, ink::storage::traits::StorageLayout,
    )]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub enum RevocationReason {
        /// KYC/AML policy violation
        KycAmlRevoked,
        /// Fraudulent activity detected
        FraudDetected,
        /// Account compromised
        AccountCompromised,
        /// User request
        UserRequest,
        /// Other reason
        Other,
    }

    /// Revocation record for a revoked identity
    #[derive(
        Debug, Clone, PartialEq, scale::Encode, scale::Decode, ink::storage::traits::StorageLayout,
    )]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub struct RevocationRecord {
        pub account: AccountId,
        pub revoked_by: AccountId,
        pub reason: RevocationReason,
        pub revoked_at: u64,
    }

    /// GDPR data export structure containing all personal data for an account
    #[derive(
        Debug, Clone, PartialEq, scale::Encode, scale::Decode, ink::storage::traits::StorageLayout,
    )]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub struct PersonalDataExport {
        pub identity: Option<Identity>,
        pub reputation: Option<ReputationMetrics>,
        pub verification_history: Vec<VerificationHistoryEntry>,
        pub audit_entries: Vec<AuditEntry>,
        pub kyc_tier: Option<KycTier>,
    }

    /// Single verification history entry for data export
    #[derive(
        Debug, Clone, PartialEq, Eq, scale::Encode, scale::Decode, ink::storage::traits::StorageLayout,
    )]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub struct VerificationHistoryEntry {
        pub verifier: AccountId,
        pub verification_level: VerificationLevel,
        pub verified_at: u64,
    }

    /// Status of a GDPR data deletion request
    #[derive(
        Debug, Clone, PartialEq, Eq, scale::Encode, scale::Decode, ink::storage::traits::StorageLayout,
    )]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub enum DataDeletionStatus {
        None,
        Requested,
        Completed,
    }

    /// GDPR data deletion request with cooldown tracking
    #[derive(
        Debug, Clone, PartialEq, Eq, scale::Encode, scale::Decode, ink::storage::traits::StorageLayout,
    )]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub struct DataDeletionRequest {
        pub requested_at: u64,
        pub cooldown_ends_at: u64,
        pub status: DataDeletionStatus,
    }

    /// Decentralized Identifier (DID) document structure
    #[derive(
        Debug, Clone, PartialEq, scale::Encode, scale::Decode, ink::storage::traits::StorageLayout,
    )]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub struct DIDDocument {
        pub did: String,                      // Decentralized Identifier
        pub public_key: Vec<u8>,              // Public key for verification
        pub verification_method: String,      // Verification method (e.g., Ed25519)
        pub service_endpoint: Option<String>, // Service endpoint for identity verification
        pub created_at: u64,                  // Creation timestamp
        pub updated_at: u64,                  // Last update timestamp
        pub version: u32,                     // Document version
    }

    /// Identity information with cross-chain support
    #[derive(
        Debug, Clone, PartialEq, scale::Encode, scale::Decode, ink::storage::traits::StorageLayout,
    )]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub struct Identity {
        pub account_id: AccountId,
        pub did_document: DIDDocument,
        pub reputation_score: u32, // 0-1000 reputation score
        pub verification_level: VerificationLevel,
        pub kyc_tier: KycTier, // KYC tier level - Issue #282
        pub trust_score: u32,  // Trust score 0-100
        pub is_verified: bool,
        pub verified_at: Option<u64>,
        pub verification_expires: Option<u64>,
        pub social_recovery: SocialRecoveryConfig,
        pub privacy_settings: PrivacySettings,
        pub created_at: u64,
        pub last_activity: u64,
    }

    /// Verification levels for identity verification
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
    pub enum VerificationLevel {
        None,     // No verification
        Basic,    // Basic identity verification
        Standard, // Standard KYC verification
        Enhanced, // Enhanced due diligence
        Premium,  // Premium verification with multiple checks
    }

    /// Social recovery configuration
    #[derive(
        Debug, Clone, PartialEq, scale::Encode, scale::Decode, ink::storage::traits::StorageLayout,
    )]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub struct SocialRecoveryConfig {
        pub guardians: Vec<AccountId>, // Trusted guardians for recovery
        pub threshold: u8,             // Number of guardians required for recovery
        pub recovery_period: u64,      // Recovery period in blocks
        pub timelock_period: u64,      // Timelock period in blocks
        pub last_recovery_attempt: Option<u64>,
        pub is_recovery_active: bool,
        pub recovery_approvals: Vec<AccountId>,
        pub recovery_completion_timestamp: Option<u64>,
    }

    /// Privacy settings for identity verification
    #[derive(
        Debug, Clone, PartialEq, scale::Encode, scale::Decode, ink::storage::traits::StorageLayout,
    )]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub struct PrivacySettings {
        pub public_reputation: bool,           // Make reputation score public
        pub public_verification: bool,         // Make verification status public
        pub data_sharing_consent: bool,        // Consent for data sharing
        pub zero_knowledge_proof: bool,        // Use zero-knowledge proofs
        pub selective_disclosure: Vec<String>, // Fields to selectively disclose
    }

    /// Cross-chain verification information
    #[derive(
        Debug, Clone, PartialEq, scale::Encode, scale::Decode, ink::storage::traits::StorageLayout,
    )]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub struct CrossChainVerification {
        pub chain_id: ChainId,
        pub verified_at: u64,
        pub verification_hash: Hash,
        pub reputation_score: u32,
        pub is_active: bool,
    }

    /// Reputation metrics based on transaction history
    #[derive(
        Debug, Clone, PartialEq, scale::Encode, scale::Decode, ink::storage::traits::StorageLayout,
    )]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub struct ReputationMetrics {
        pub total_transactions: u64,
        pub successful_transactions: u64,
        pub failed_transactions: u64,
        pub dispute_count: u64,
        pub dispute_resolved_count: u64,
        pub average_transaction_value: u128,
        pub total_value_transacted: u128,
        pub last_updated: u64,
        pub reputation_score: u32,
    }

    /// Trust assessment for counterparties
    #[derive(
        Debug, Clone, PartialEq, scale::Encode, scale::Decode, ink::storage::traits::StorageLayout,
    )]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub struct TrustAssessment {
        pub target_account: AccountId,
        pub trust_score: u32, // 0-100 trust score
        pub verification_level: VerificationLevel,
        pub reputation_score: u32,
        pub shared_transactions: u64,
        pub positive_interactions: u64,
        pub negative_interactions: u64,
        pub risk_level: RiskLevel,
        pub assessment_date: u64,
        pub expires_at: u64,
    }

    /// KYC Tier structure for tiered verification - Issue #282
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
    pub enum KycTier {
        Tier0Unverified,  // No KYC, basic access only
        Tier1Basic,       // Basic identity verification
        Tier2Standard,    // Standard KYC with document verification
        Tier3Enhanced,    // Enhanced due diligence
        Tier4Premium,     // Premium verification with full background check
        Tier0_Unverified, // No KYC, basic access only
        Tier1_Basic,      // Basic identity verification
        Tier2_Standard,   // Standard KYC with document verification
        Tier3_Enhanced,   // Enhanced due diligence
        Tier4_Premium,    // Premium verification with full background check
    }

    /// KYC Tier privileges
    #[derive(
        Debug, Clone, PartialEq, scale::Encode, scale::Decode, ink::storage::traits::StorageLayout,
    )]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub struct KycTierPrivileges {
        pub tier: KycTier,
        pub max_transaction_value: u128,
        pub daily_transaction_limit: u64,
        pub can_trade: bool,
        pub can_withdraw: bool,
        pub requires_additional_verification: bool,
        pub description: [u8; 128],
    }

    /// Verification Provider - Issue #283
    #[derive(
        Debug, Clone, PartialEq, scale::Encode, scale::Decode, ink::storage::traits::StorageLayout,
    )]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub struct VerificationProvider {
        pub provider_id: AccountId,
        pub name: [u8; 64],
        pub provider_type: ProviderType,
        pub is_active: bool,
        pub verified_identities: u64,
        pub registered_at: u64,
        pub supported_tiers: Vec<KycTier>,
    }

    /// Provider type classification
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
    pub enum ProviderType {
        GovernmentId,          // Official government ID verification
        DocumentVerification,  // Passport, driver's license, etc.
        BiometricVerification, // Facial recognition, fingerprints
        FinancialVerification, // Bank account, credit check
        ThirdPartyKyc,         // Third-party KYC services
    }

    /// Verification request with provider
    #[derive(
        Debug, Clone, PartialEq, scale::Encode, scale::Decode, ink::storage::traits::StorageLayout,
    )]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub struct ProviderVerificationRequest {
        pub request_id: u64,
        pub applicant: AccountId,
        pub provider_id: AccountId,
        pub requested_tier: KycTier,
        pub evidence_hash: Option<Hash>,
        pub requested_at: u64,
        pub status: VerificationStatus,
        pub completed_at: Option<u64>,
        pub result_metadata: [u8; 128],
    }

    /// Risk level assessment
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
    pub enum RiskLevel {
        Low,      // Low risk, highly trusted
        Medium,   // Medium risk, some trust established
        High,     // High risk, limited trust
        Critical, // Critical risk, avoid transactions
    }

    /// Identity verification request
    #[derive(
        Debug, Clone, PartialEq, scale::Encode, scale::Decode, ink::storage::traits::StorageLayout,
    )]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub struct VerificationRequest {
        pub id: u64,
        pub requester: AccountId,
        pub verification_level: VerificationLevel,
        pub evidence_hash: Option<Hash>,
        pub requested_at: u64,
        pub status: VerificationStatus,
        pub reviewed_by: Option<AccountId>,
        pub reviewed_at: Option<u64>,
        pub comments: String,
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
        Expired,
        Revoked,
    }

    /// Main identity registry contract
    #[ink(storage)]
    pub struct IdentityRegistry {
        /// Mapping from account to identity
        identities: Mapping<AccountId, Identity>,
        /// Mapping from DID to account
        did_to_account: Mapping<String, AccountId>,
        /// Reputation metrics for accounts
        reputation_metrics: Mapping<AccountId, ReputationMetrics>,
        /// Trust assessments between accounts
        trust_assessments: Mapping<(AccountId, AccountId), TrustAssessment>,
        /// Verification requests
        verification_requests: Mapping<u64, VerificationRequest>,
        /// Verification request counter
        verification_count: u64,
        /// Cross-chain verifications
        cross_chain_verifications: Mapping<(AccountId, ChainId), CrossChainVerification>,
        /// Supported chains for cross-chain verification
        supported_chains: Vec<ChainId>,
        /// Admin account
        admin: AccountId,
        /// Authorized verifiers
        authorized_verifiers: Mapping<AccountId, bool>,
        /// Contract version
        version: u32,
        /// Privacy verification nonces
        privacy_nonces: Mapping<AccountId, u64>,
        /// Audit trail entries indexed by entry id
        audit_trail: Mapping<u64, AuditEntry>,
        /// Audit entry counter
        audit_count: u64,
        /// Per-account audit entry index list (stores entry ids)
        account_audit_index: Mapping<(AccountId, u64), u64>,
        /// Per-account audit entry count
        account_audit_count: Mapping<AccountId, u64>,
        /// Revocation records for revoked identities
        revocations: Mapping<AccountId, RevocationRecord>,
        /// Verification providers - Issue #283
        verification_providers: Mapping<AccountId, VerificationProvider>,
        /// Provider verification requests
        provider_verification_requests: Mapping<u64, ProviderVerificationRequest>,
        /// Provider request counter
        provider_request_count: u64,
        /// KYC tier privileges configuration
        kyc_tier_privileges: Mapping<KycTier, KycTierPrivileges>,
        /// User's current KYC tier
        user_kyc_tiers: Mapping<AccountId, KycTier>,
        /// Hash of the current privacy policy document
        privacy_policy_hash: [u8; 32],
        /// GDPR data deletion requests with cooldown
        data_deletion_requests: Mapping<AccountId, DataDeletionRequest>,
        /// Cooldown period in blocks before data can be deleted
        deletion_cooldown_blocks: u32,
    }

    /// Events
    #[ink(event)]
    pub struct IdentityCreated {
        #[ink(topic)]
        account: AccountId,
        #[ink(topic)]
        did: String,
        timestamp: u64,
    }

    #[ink(event)]
    pub struct IdentityVerified {
        #[ink(topic)]
        account: AccountId,
        #[ink(topic)]
        verification_level: VerificationLevel,
        #[ink(topic)]
        verified_by: AccountId,
        timestamp: u64,
    }

    #[ink(event)]
    pub struct ReputationUpdated {
        #[ink(topic)]
        account: AccountId,
        old_score: u32,
        new_score: u32,
        timestamp: u64,
    }

    #[ink(event)]
    pub struct TrustAssessmentCreated {
        #[ink(topic)]
        assessor: AccountId,
        #[ink(topic)]
        target: AccountId,
        trust_score: u32,
        risk_level: RiskLevel,
        timestamp: u64,
    }

    #[ink(event)]
    pub struct CrossChainVerified {
        #[ink(topic)]
        account: AccountId,
        #[ink(topic)]
        chain_id: ChainId,
        reputation_score: u32,
        timestamp: u64,
    }

    #[ink(event)]
    pub struct RecoveryInitiated {
        #[ink(topic)]
        account: AccountId,
        #[ink(topic)]
        initiator: AccountId,
        timestamp: u64,
    }

    #[ink(event)]
    pub struct RecoveryCompleted {
        #[ink(topic)]
        account: AccountId,
        #[ink(topic)]
        new_account: AccountId,
        timestamp: u64,
    }

    #[ink(event)]
    pub struct IdentityPorted {
        #[ink(topic)]
        old_account: AccountId,
        #[ink(topic)]
        new_account: AccountId,
        timestamp: u64,
    }

    #[ink(event)]
    pub struct IdentityRevoked {
        #[ink(topic)]
        account: AccountId,
        #[ink(topic)]
        revoked_by: AccountId,
        reason: String,
        timestamp: u64,
    }

    #[ink(event)]
    pub struct AuditEntryAdded {
        #[ink(topic)]
        account: AccountId,
        entry_id: u64,
        action: String,
        timestamp: u64,
    }

    /// Emitted when a KYC verification has expired
    #[ink(event)]
    pub struct KycExpired {
        #[ink(topic)]
        account: AccountId,
        expired_at: u64,
        timestamp: u64,
    }

    /// Emitted when KYC renewal is required (approaching expiry)
    #[ink(event)]
    pub struct KycRenewalRequired {
        #[ink(topic)]
        account: AccountId,
        expires_at: u64,
        timestamp: u64,
    }

    /// Emitted when a DID document is updated
    #[ink(event)]
    pub struct DIDUpdated {
        #[ink(topic)]
        account: AccountId,
        #[ink(topic)]
        did: String,
        version: u32,
        timestamp: u64,
    }

    /// Emitted when a ZK KYC proof is verified
    #[ink(event)]
    pub struct ZkKycVerified {
        #[ink(topic)]
        account: AccountId,
        proof_type: String,
        timestamp: u64,
    }

    /// Emitted when a GDPR data export is requested
    #[ink(event)]
    pub struct DataExportRequested {
        #[ink(topic)]
        account: AccountId,
        timestamp: u64,
    }

    /// Emitted when a GDPR data deletion is requested
    #[ink(event)]
    pub struct DataDeletionRequested {
        #[ink(topic)]
        account: AccountId,
        cooldown_ends_at: u64,
        timestamp: u64,
    }

    /// Emitted when GDPR data deletion is completed
    #[ink(event)]
    pub struct DataDeletionCompleted {
        #[ink(topic)]
        account: AccountId,
        timestamp: u64,
    }

    impl Default for IdentityRegistry {
        fn default() -> Self {
            Self {
                identities: Mapping::default(),
                did_to_account: Mapping::default(),
                reputation_metrics: Mapping::default(),
                trust_assessments: Mapping::default(),
                verification_requests: Mapping::default(),
                verification_count: 0,
                cross_chain_verifications: Mapping::default(),
                supported_chains: vec![1, 2, 3, 4, 5],
                admin: AccountId::from([0u8; 32]),
                authorized_verifiers: Mapping::default(),
                version: 0,
                privacy_nonces: Mapping::default(),
                audit_trail: Mapping::default(),
                audit_count: 0,
                account_audit_index: Mapping::default(),
                account_audit_count: Mapping::default(),
                revocations: Mapping::default(),
                verification_providers: Mapping::default(),
                provider_verification_requests: Mapping::default(),
                provider_request_count: 0,
                kyc_tier_privileges: Mapping::default(),
                user_kyc_tiers: Mapping::default(),
                privacy_policy_hash: [0u8; 32],
                data_deletion_requests: Mapping::default(),
                deletion_cooldown_blocks: 100,
            }
        }
    }

    impl IdentityRegistry {
        /// Creates a new IdentityRegistry contract
        #[ink(constructor)]
        pub fn new() -> Self {
            let caller = Self::env().caller();
            let mut registry = Self {
                identities: Mapping::default(),
                did_to_account: Mapping::default(),
                reputation_metrics: Mapping::default(),
                trust_assessments: Mapping::default(),
                verification_requests: Mapping::default(),
                verification_count: 0,
                cross_chain_verifications: Mapping::default(),
                supported_chains: vec![
                    1, // Ethereum
                    2, // Polkadot
                    3, // Avalanche
                    4, // BSC
                    5, // Polygon
                ],
                admin: caller,
                authorized_verifiers: Mapping::default(),
                version: 1,
                privacy_nonces: Mapping::default(),
                audit_trail: Mapping::default(),
                audit_count: 0,
                account_audit_index: Mapping::default(),
                account_audit_count: Mapping::default(),
                revocations: Mapping::default(),
                verification_providers: Mapping::default(),
                provider_verification_requests: Mapping::default(),
                provider_request_count: 0,
                kyc_tier_privileges: Mapping::default(),
                user_kyc_tiers: Mapping::default(),
                privacy_policy_hash: [0u8; 32],
                data_deletion_requests: Mapping::default(),
                deletion_cooldown_blocks: 100,
            };

            // Initialize default KYC tier privileges
            registry.initialize_kyc_tiers();

            registry
        }

        /// Revokes an identity
        #[ink(message)]
        pub fn revoke_identity(
            &mut self,
            account: AccountId,
            reason: RevocationReason,
        ) -> Result<(), IdentityError> {
            let caller = self.env().caller();

            // Check if caller is authorized
            if !self.authorized_verifiers.contains(&caller) && caller != self.admin {
                return Err(IdentityError::Unauthorized);
            }

            // Check if identity exists
            let mut identity = self.identities.get(&account).ok_or(IdentityError::IdentityNotFound)?;

            // Update identity status
            identity.is_verified = false;
            identity.verification_level = VerificationLevel::None;
            self.identities.insert(&account, &identity);

            // Create revocation record
            let revocation_record = RevocationRecord {
                account,
                revoked_by: caller,
                reason,
                revoked_at: self.env().block_timestamp(),
            };

            self.revocations.insert(&account, &revocation_record);

            // Emit event
            self.env().emit_event(IdentityRevoked {
                account,
                revoked_by: caller,
                reason: format!("{:?}", reason),
                timestamp: self.env().block_timestamp(),
            });

            Ok(())
        }

        /// Create a new identity with DID
        #[ink(message)]
        pub fn create_identity(
            &mut self,
            did: String,
            public_key: Vec<u8>,
            verification_method: String,
            service_endpoint: Option<String>,
            privacy_settings: PrivacySettings,
        ) -> Result<(), IdentityError> {
            let caller = self.env().caller();
            let timestamp = self.env().block_timestamp();

            // Check if identity already exists
            if self.identities.contains(&caller) {
                return Err(IdentityError::IdentityAlreadyExists);
            }

            // Validate DID format
            if !self.validate_did_format(&did) {
                return Err(IdentityError::InvalidDid);
            }

            // Create DID document
            let did_document = DIDDocument {
                did: did.clone(),
                public_key,
                verification_method,
                service_endpoint,
                created_at: timestamp,
                updated_at: timestamp,
                version: 1,
            };

            // Create social recovery config with default settings
            let social_recovery = SocialRecoveryConfig {
                guardians: Vec::new(),
                threshold: 3,
                recovery_period: 100800, // ~2 weeks in blocks (assuming 6s block time)
                last_recovery_attempt: None,
                is_recovery_active: false,
                recovery_approvals: Vec::new(),
            };

            // Create identity
            let identity = Identity {
                account_id: caller,
                did_document,
                reputation_score: 500, // Start with neutral reputation
                verification_level: VerificationLevel::None,
                kyc_tier: KycTier::Tier0Unverified,
                trust_score: 50,
                is_verified: false,
                verified_at: None,
                verification_expires: None,
                social_recovery,
                privacy_settings,
                created_at: timestamp,
                last_activity: timestamp,
            };

            // Store identity
            self.identities.insert(&caller, &identity);
            self.did_to_account.insert(&did, &caller);

            // Initialize reputation metrics
            let reputation_metrics = ReputationMetrics {
                total_transactions: 0,
                successful_transactions: 0,
                failed_transactions: 0,
                dispute_count: 0,
                dispute_resolved_count: 0,
                average_transaction_value: 0,
                total_value_transacted: 0,
                last_updated: timestamp,
                reputation_score: 500,
            };
            self.reputation_metrics.insert(&caller, &reputation_metrics);

            // Emit event
            self.env().emit_event(IdentityCreated {
                account: caller,
                did,
                timestamp,
            });

            // Record audit entry
            self.add_audit_entry(
                caller,
                caller,
                "identity_created".into(),
                "Identity created".into(),
            );

            Ok(())
        }

        /// Verify identity (verifier only)
        #[ink(message)]
        pub fn verify_identity(
            &mut self,
            target_account: AccountId,
            verification_level: VerificationLevel,
            expires_in_days: Option<u64>,
        ) -> Result<(), IdentityError> {
            let caller = self.env().caller();
            let timestamp = self.env().block_timestamp();

            // Check if caller is authorized verifier
            if !self.is_authorized_verifier(caller) {
                return Err(IdentityError::Unauthorized);
            }

            // Get identity
            let mut identity = self
                .identities
                .get(&target_account)
                .ok_or(IdentityError::IdentityNotFound)?;

            // Update verification
            identity.verification_level = verification_level;
            identity.is_verified = true;
            identity.verified_at = Some(timestamp);
            identity.verification_expires = expires_in_days.map(|days| timestamp + days * 86400);
            identity.last_activity = timestamp;

            // Update trust score based on verification level
            identity.trust_score = match verification_level {
                VerificationLevel::None => 0,
                VerificationLevel::Basic => 60,
                VerificationLevel::Standard => 75,
                VerificationLevel::Enhanced => 90,
                VerificationLevel::Premium => 100,
            };

            // Store updated identity
            self.identities.insert(&target_account, &identity);

            // Emit event
            self.env().emit_event(IdentityVerified {
                account: target_account,
                verification_level,
                verified_by: caller,
                timestamp,
            });

            // Record audit entry
            self.add_audit_entry(
                target_account,
                caller,
                "identity_verified".into(),
                "Identity verification level updated".into(),
            );

            Ok(())
        }

        /// Update reputation based on transaction
        #[ink(message)]
        pub fn update_reputation(
            &mut self,
            target_account: AccountId,
            transaction_successful: bool,
            transaction_value: u128,
        ) -> Result<(), IdentityError> {
            let caller = self.env().caller();
            let timestamp = self.env().block_timestamp();

            // Only authorized contracts can update reputation
            if !self.is_authorized_verifier(caller) {
                return Err(IdentityError::Unauthorized);
            }

            // Get and update reputation metrics
            let mut metrics =
                self.reputation_metrics
                    .get(&target_account)
                    .unwrap_or(ReputationMetrics {
                        total_transactions: 0,
                        successful_transactions: 0,
                        failed_transactions: 0,
                        dispute_count: 0,
                        dispute_resolved_count: 0,
                        average_transaction_value: 0,
                        total_value_transacted: 0,
                        last_updated: timestamp,
                        reputation_score: 500,
                    });

            metrics.total_transactions += 1;
            metrics.total_value_transacted += transaction_value;
            metrics.average_transaction_value =
                metrics.total_value_transacted / metrics.total_transactions as u128;

            if transaction_successful {
                metrics.successful_transactions += 1;
                // Increase reputation for successful transactions
                metrics.reputation_score = (metrics.reputation_score + 5).min(1000);
            } else {
                metrics.failed_transactions += 1;
                // Decrease reputation for failed transactions
                metrics.reputation_score = metrics.reputation_score.saturating_sub(10);
            }

            metrics.last_updated = timestamp;

            // Update identity reputation score
            if let Some(mut identity) = self.identities.get(&target_account) {
                let old_score = identity.reputation_score;
                identity.reputation_score = metrics.reputation_score;
                identity.last_activity = timestamp;
                self.identities.insert(&target_account, &identity);

                // Emit event
                self.env().emit_event(ReputationUpdated {
                    account: target_account,
                    old_score,
                    new_score: metrics.reputation_score,
                    timestamp,
                });
            }

            // Store updated metrics
            self.reputation_metrics.insert(&target_account, &metrics);

            Ok(())
        }

        /// Get trust assessment for counterparty
        #[ink(message)]
        pub fn assess_trust(
            &mut self,
            target_account: AccountId,
        ) -> Result<TrustAssessment, IdentityError> {
            let caller = self.env().caller();
            let timestamp = self.env().block_timestamp();

            // Get target identity and reputation
            let target_identity = self
                .identities
                .get(&target_account)
                .ok_or(IdentityError::IdentityNotFound)?;
            let target_metrics =
                self.reputation_metrics
                    .get(&target_account)
                    .unwrap_or(ReputationMetrics {
                        total_transactions: 0,
                        successful_transactions: 0,
                        failed_transactions: 0,
                        dispute_count: 0,
                        dispute_resolved_count: 0,
                        average_transaction_value: 0,
                        total_value_transacted: 0,
                        last_updated: timestamp,
                        reputation_score: target_identity.reputation_score,
                    });

            // Calculate trust score
            let trust_score = self.calculate_trust_score(&target_identity, &target_metrics);

            // Determine risk level based on trust score
            let risk_level = if trust_score >= 80 {
                RiskLevel::Low
            } else if trust_score >= 60 {
                RiskLevel::Medium
            } else if trust_score >= 40 {
                RiskLevel::High
            } else {
                RiskLevel::Critical
            };

            // Create trust assessment
            let assessment = TrustAssessment {
                target_account,
                trust_score,
                risk_level,
                verification_level: target_identity.verification_level,
                reputation_score: target_identity.reputation_score,
                shared_transactions: target_metrics.total_transactions,
                positive_interactions: target_metrics.successful_transactions,
                negative_interactions: target_metrics.failed_transactions,
                assessment_date: timestamp,
                expires_at: timestamp + 86400 * 30, // 30 days
            };

            self.trust_assessments
                .insert(&(caller, target_account), &assessment);

            Ok(assessment)
        }

        /// Add cross-chain verification
        #[ink(message)]
        pub fn add_cross_chain_verification(
            &mut self,
            chain_id: ChainId,
            verification_hash: Hash,
            reputation_score: u32,
        ) -> Result<(), IdentityError> {
            let caller = self.env().caller();
            let timestamp = self.env().block_timestamp();

            // Check if chain is supported
            if !self.supported_chains.contains(&chain_id) {
                return Err(IdentityError::UnsupportedChain);
            }

            // Get identity
            let mut identity = self
                .identities
                .get(&caller)
                .ok_or(IdentityError::IdentityNotFound)?;

            // Add cross-chain verification
            let cross_chain_verification = CrossChainVerification {
                chain_id,
                verified_at: timestamp,
                verification_hash,
                reputation_score,
                is_active: true,
            };

            self.cross_chain_verifications
                .insert(&(caller, chain_id), &cross_chain_verification);
            identity.last_activity = timestamp;

            // Update reputation based on cross-chain verification
            identity.reputation_score = (identity.reputation_score + reputation_score) / 2;

            // Store updated identity
            self.identities.insert(&caller, &identity);

            // Emit event
            self.env().emit_event(CrossChainVerified {
                account: caller,
                chain_id,
                reputation_score,
                timestamp,
            });

            Ok(())
        }

        /// Initiate social recovery
        #[ink(message)]
        pub fn initiate_recovery(
            &mut self,
            new_account: AccountId,
            recovery_signature: Vec<u8>,
        ) -> Result<(), IdentityError> {
            let caller = self.env().caller();
            let timestamp = self.env().block_timestamp();

            // Get identity
            let mut identity = self
                .identities
                .get(&caller)
                .ok_or(IdentityError::IdentityNotFound)?;

            // Check if recovery is already in progress
            if identity.social_recovery.is_recovery_active {
                return Err(IdentityError::RecoveryInProgress);
            }

            // Verify recovery signature
            if !self.verify_recovery_signature(
                &caller,
                &new_account,
                &recovery_signature,
                &identity,
            ) {
                return Err(IdentityError::InvalidSignature);
            }

            // Start recovery process
            identity.social_recovery.is_recovery_active = true;
            identity.social_recovery.last_recovery_attempt = Some(timestamp);
            identity.social_recovery.recovery_approvals = Vec::new();

            // Store updated identity
            self.identities.insert(&caller, &identity);

            // Emit event
            self.env().emit_event(RecoveryInitiated {
                account: caller,
                initiator: caller,
                timestamp,
            });

            Ok(())
        }

        /// Approve recovery (guardian only)
        #[ink(message)]
        pub fn approve_recovery(
            &mut self,
            target_account: AccountId,
            new_account: AccountId,
        ) -> Result<(), IdentityError> {
            let caller = self.env().caller();

            // Get target identity
            let mut identity = self
                .identities
                .get(&target_account)
                .ok_or(IdentityError::IdentityNotFound)?;

            // Check if caller is a guardian
            if !identity.social_recovery.guardians.contains(&caller) {
                return Err(IdentityError::Unauthorized);
            }

            // Check if recovery is active
            if !identity.social_recovery.is_recovery_active {
                return Err(IdentityError::RecoveryNotActive);
            }

            // Add approval
            if !identity
                .social_recovery
                .recovery_approvals
                .contains(&caller)
            {
                identity.social_recovery.recovery_approvals.push(caller);
            }

            // Check if threshold is met
            if identity.social_recovery.recovery_approvals.len()
                >= identity.social_recovery.threshold as usize
            {
                // Complete recovery
                self.complete_recovery(target_account, new_account)?;
            } else {
                // Store updated identity
                self.identities.insert(&target_account, &identity);
            }

            Ok(())
        }

        /// Complete identity recovery
        fn complete_recovery(
            &mut self,
            old_account: AccountId,
            new_account: AccountId,
        ) -> Result<(), IdentityError> {
            let _timestamp = self.env().block_timestamp();

            // Get old identity
            let mut identity = self
                .identities
                .get(&old_account)
                .ok_or(IdentityError::IdentityNotFound)?;

            // Update account ID
            identity.account_id = new_account;
            identity.social_recovery.is_recovery_active = false;
            identity.social_recovery.recovery_approvals = Vec::new();
            identity.last_activity = _timestamp;

            // Remove old identity mapping
            self.identities.remove(&old_account);

            // Add new identity mapping
            self.identities.insert(&new_account, &identity);
            self.did_to_account
                .insert(&identity.did_document.did, &new_account);

            // Update reputation metrics mapping
            if let Some(metrics) = self.reputation_metrics.get(&old_account) {
                self.reputation_metrics.remove(&old_account);
                self.reputation_metrics.insert(&new_account, &metrics);
            }

            // Emit event
            self.env().emit_event(RecoveryCompleted {
                account: old_account,
                new_account,
                timestamp: _timestamp,
            });

            Ok(())
        }

        /// Port an existing identity to a new account
        #[ink(message)]
        pub fn port_identity(&mut self, new_account: AccountId) -> Result<(), IdentityError> {
            let caller = self.env().caller();
            let timestamp = self.env().block_timestamp();

            if caller == new_account {
                return Err(IdentityError::IdentityAlreadyExists);
            }

            // Source identity must exist and must not be revoked
            let mut identity = self
                .identities
                .get(&caller)
                .ok_or(IdentityError::IdentityNotFound)?;

            if self.revocations.contains(&caller) {
                return Err(IdentityError::IdentityRevoked);
            }

            if self.identities.contains(&new_account) {
                return Err(IdentityError::IdentityAlreadyExists);
            }

            identity.account_id = new_account;
            identity.last_activity = timestamp;
            identity.did_document.updated_at = timestamp;
            identity.did_document.version = identity.did_document.version.saturating_add(1);

            self.identities.remove(&caller);
            self.identities.insert(&new_account, &identity);
            self.did_to_account
                .insert(&identity.did_document.did, &new_account);

            if let Some(metrics) = self.reputation_metrics.get(&caller) {
                self.reputation_metrics.remove(&caller);
                self.reputation_metrics.insert(&new_account, &metrics);
            }

            self.env().emit_event(IdentityPorted {
                old_account: caller,
                new_account,
                timestamp,
            });

            self.add_audit_entry(
                new_account,
                caller,
                "identity_ported".into(),
                "Identity ported to new account".into(),
            );

            Ok(())
        }

        /// Privacy-preserving identity verification using zero-knowledge proofs
        #[ink(message)]
        pub fn verify_privacy_preserving(
            &mut self,
            proof: Vec<u8>,
            public_inputs: Vec<u8>,
            verification_type: String,
        ) -> Result<bool, IdentityError> {
            let caller = self.env().caller();
            let _timestamp = self.env().block_timestamp();

            // Get identity
            let identity = self
                .identities
                .get(&caller)
                .ok_or(IdentityError::IdentityNotFound)?;

            // Check if privacy settings allow this verification
            if !identity.privacy_settings.zero_knowledge_proof {
                return Err(IdentityError::PrivacyVerificationFailed);
            }

            // Verify zero-knowledge proof (simplified verification)
            let is_valid =
                self.verify_zero_knowledge_proof(&proof, &public_inputs, &verification_type);

            if is_valid {
                // Update privacy nonce for replay protection
                let current_nonce = self.privacy_nonces.get(&caller).unwrap_or(0);
                self.privacy_nonces.insert(&caller, &(current_nonce + 1));

                // Update last activity
                let mut updated_identity = identity;
                updated_identity.last_activity = _timestamp;
                self.identities.insert(&caller, &updated_identity);
            }

            Ok(is_valid)
        }

        /// Get identity information
        #[ink(message)]
        pub fn get_identity(&self, account: AccountId) -> Option<Identity> {
            self.identities.get(&account)
        }

        /// Get reputation metrics
        #[ink(message)]
        pub fn get_reputation_metrics(&self, account: AccountId) -> Option<ReputationMetrics> {
            self.reputation_metrics.get(&account)
        }

        /// Get trust assessment
        #[ink(message)]
        pub fn get_trust_assessment(
            &self,
            assessor: AccountId,
            target: AccountId,
        ) -> Option<TrustAssessment> {
            self.trust_assessments.get(&(assessor, target))
        }

        /// Check if account meets reputation threshold
        #[ink(message)]
        pub fn meets_reputation_threshold(&self, account: AccountId, threshold: u32) -> bool {
            if let Some(identity) = self.identities.get(&account) {
                identity.reputation_score >= threshold
            } else {
                false
            }
        }

        /// Get cross-chain verification status
        #[ink(message)]
        pub fn get_cross_chain_verification(
            &self,
            account: AccountId,
            chain_id: ChainId,
        ) -> Option<CrossChainVerification> {
            self.cross_chain_verifications.get(&(account, chain_id))
        }

        /// Helper methods
        fn validate_did_format(&self, did: &str) -> bool {
            // Basic DID format validation: did:method:specific-id
            did.starts_with("did:") && did.split(':').count() >= 3
        }

        fn is_authorized_verifier(&self, account: AccountId) -> bool {
            account == self.admin || self.authorized_verifiers.get(&account).unwrap_or(false)
        }

        fn calculate_trust_score(&self, identity: &Identity, metrics: &ReputationMetrics) -> u32 {
            let base_score = identity.trust_score;
            let reputation_factor = identity.reputation_score;
            let verification_bonus = match identity.verification_level {
                VerificationLevel::None => 0,
                VerificationLevel::Basic => 10,
                VerificationLevel::Standard => 20,
                VerificationLevel::Enhanced => 30,
                VerificationLevel::Premium => 40,
            };

            // Calculate success rate
            let success_rate = if metrics.total_transactions > 0 {
                metrics
                    .successful_transactions
                    .saturating_mul(100)
                    .checked_div(metrics.total_transactions)
                    .unwrap_or(50)
            } else {
                50 // Default for no history
            };

            // Weighted calculation with proper type casting
            ((base_score as u64 * 40)
                + (reputation_factor as u64 / 10 * 30)
                + (verification_bonus as u64 * 20)
                + (success_rate * 10)) as u32
                / 100
        }

        fn verify_recovery_signature(
            &self,
            _old_account: &AccountId,
            _new_account: &AccountId,
            signature: &[u8],
            _identity: &Identity,
        ) -> bool {
            // Simplified signature verification
            // In production, this would use proper cryptographic verification
            signature.len() == 64 // Basic length check for Ed25519 signature
        }

        fn verify_zero_knowledge_proof(
            &self,
            proof: &[u8],
            public_inputs: &[u8],
            verification_type: &str,
        ) -> bool {
            // Simplified ZK verification
            // In production, this would integrate with proper ZK proof systems
            match verification_type {
                "identity_proof" => proof.len() >= 32,
                "reputation_proof" => public_inputs.len() >= 8,
                _ => false,
            }
        }

        /// Revoke a compromised identity (admin or authorized verifier only)
        #[ink(message)]
        pub fn revoke_identity(
            &mut self,
            target_account: AccountId,
            reason: String,
        ) -> Result<(), IdentityError> {
            let caller = self.env().caller();
            let timestamp = self.env().block_timestamp();

            if !self.is_authorized_verifier(caller) {
                return Err(IdentityError::Unauthorized);
            }

            // Identity must exist
            let mut identity = self
                .identities
                .get(&target_account)
                .ok_or(IdentityError::IdentityNotFound)?;

            // Mark identity as revoked (set verification to None and is_verified false)
            identity.is_verified = false;
            identity.verification_level = VerificationLevel::None;
            identity.trust_score = 0;
            identity.last_activity = timestamp;
            self.identities.insert(&target_account, &identity);

            // Store revocation record
            let record = RevocationRecord {
                account: target_account,
                revoked_by: caller,
                reason: reason.clone(),
                revoked_at: timestamp,
            };
            self.revocations.insert(&target_account, &record);

            // Add audit entry
            self.add_audit_entry(
                target_account,
                caller,
                "identity_revoked".into(),
                reason.clone(),
            );

            self.env().emit_event(IdentityRevoked {
                account: target_account,
                revoked_by: caller,
                reason,
                timestamp,
            });

            Ok(())
        }

        /// Check if an identity has been revoked
        #[ink(message)]
        pub fn is_revoked(&self, account: AccountId) -> bool {
            self.revocations.contains(&account)
        }

        /// Get the revocation record for an account
        #[ink(message)]
        pub fn get_revocation(&self, account: AccountId) -> Option<RevocationRecord> {
            self.revocations.get(&account)
        }

        /// Get a specific audit entry by id
        #[ink(message)]
        pub fn get_audit_entry(&self, entry_id: u64) -> Option<AuditEntry> {
            self.audit_trail.get(&entry_id)
        }

        /// Get the total number of audit entries
        #[ink(message)]
        pub fn get_audit_count(&self) -> u64 {
            self.audit_count
        }

        /// Get audit entries for a specific account (paginated)
        #[ink(message)]
        pub fn get_account_audit_entries(
            &self,
            account: AccountId,
            offset: u64,
            limit: u64,
        ) -> Vec<AuditEntry> {
            let count = self.account_audit_count.get(&account).unwrap_or(0);
            let mut entries = Vec::new();
            let end = (offset + limit).min(count);
            for i in offset..end {
                if let Some(entry_id) = self.account_audit_index.get(&(account, i)) {
                    if let Some(entry) = self.audit_trail.get(&entry_id) {
                        entries.push(entry);
                    }
                }
            }
            entries
        }

        /// Internal helper: record an audit entry
        fn add_audit_entry(
            &mut self,
            account: AccountId,
            performed_by: AccountId,
            action: String,
            details: String,
        ) {
            let timestamp = self.env().block_timestamp();
            self.audit_count += 1;
            let entry_id = self.audit_count;

            let entry = AuditEntry {
                entry_id,
                account,
                action: action.clone(),
                performed_by,
                timestamp,
                details,
            };

            self.audit_trail.insert(&entry_id, &entry);

            // Update per-account index
            let idx = self.account_audit_count.get(&account).unwrap_or(0);
            self.account_audit_index.insert(&(account, idx), &entry_id);
            self.account_audit_count.insert(&account, &(idx + 1));

            self.env().emit_event(AuditEntryAdded {
                account,
                entry_id,
                action,
                timestamp,
            });
        }

        /// Admin methods
        #[ink(message)]
        pub fn add_authorized_verifier(
            &mut self,
            verifier: AccountId,
        ) -> Result<(), IdentityError> {
            if self.env().caller() != self.admin {
                return Err(IdentityError::Unauthorized);
            }
            self.authorized_verifiers.insert(&verifier, &true);
            Ok(())
        }

        #[ink(message)]
        pub fn remove_authorized_verifier(
            &mut self,
            verifier: AccountId,
        ) -> Result<(), IdentityError> {
            if self.env().caller() != self.admin {
                return Err(IdentityError::Unauthorized);
            }
            self.authorized_verifiers.insert(&verifier, &false);
            Ok(())
        }

        #[ink(message)]
        pub fn add_supported_chain(&mut self, chain_id: ChainId) -> Result<(), IdentityError> {
            if self.env().caller() != self.admin {
                return Err(IdentityError::Unauthorized);
            }
            if !self.supported_chains.contains(&chain_id) {
                self.supported_chains.push(chain_id);
            }
            Ok(())
        }

        #[ink(message)]
        pub fn get_supported_chains(&self) -> Vec<ChainId> {
            self.supported_chains.clone()
        }

        // ===== KYC Tier Initialization - Issue #282 =====

        fn initialize_kyc_tiers(&mut self) {
            let tiers = [
                KycTierPrivileges {
                    tier: KycTier::Tier0Unverified,
                    max_transaction_value: 1_000_000_000_000_000_000, // 1 token
                    daily_transaction_limit: 5,
                    can_trade: false,
                    can_withdraw: false,
                    requires_additional_verification: true,
                    description: Self::pad_description("Unverified - Basic browsing only"),
                },
                KycTierPrivileges {
                    tier: KycTier::Tier1Basic,
                    max_transaction_value: 10_000_000_000_000_000_000, // 10 tokens
                    daily_transaction_limit: 10,
                    can_trade: true,
                    can_withdraw: false,
                    requires_additional_verification: false,
                    description: Self::pad_description("Basic - Limited transactions"),
                },
                KycTierPrivileges {
                    tier: KycTier::Tier2Standard,
                    max_transaction_value: 100_000_000_000_000_000_000, // 100 tokens
                    daily_transaction_limit: 50,
                    can_trade: true,
                    can_withdraw: true,
                    requires_additional_verification: false,
                    description: Self::pad_description("Standard - Full trading access"),
                },
                KycTierPrivileges {
                    tier: KycTier::Tier3Enhanced,
                    max_transaction_value: 1_000_000_000_000_000_000_000, // 1000 tokens
                    daily_transaction_limit: 100,
                    can_trade: true,
                    can_withdraw: true,
                    requires_additional_verification: false,
                    description: Self::pad_description("Enhanced - High value transactions"),
                },
                KycTierPrivileges {
                    tier: KycTier::Tier4Premium,
                    max_transaction_value: u128::MAX,
                    daily_transaction_limit: u64::MAX,
                    can_trade: true,
                    can_withdraw: true,
                    requires_additional_verification: false,
                    description: Self::pad_description("Premium - Unlimited access"),
                },
            ];

            for tier_priv in tiers.iter() {
                self.kyc_tier_privileges.insert(&tier_priv.tier, tier_priv);
            }
        }

        fn pad_description(desc: &str) -> [u8; 128] {
            let mut result = [0u8; 128];
            let bytes = desc.as_bytes();
            let len = bytes.len().min(128);
            result[..len].copy_from_slice(&bytes[..len]);
            result
        }

        // ===== Verification Provider Methods - Issue #283 =====

        #[ink(message)]
        pub fn register_verification_provider(
            &mut self,
            provider_id: AccountId,
            name: [u8; 64],
            provider_type: ProviderType,
            supported_tiers: Vec<KycTier>,
        ) -> Result<(), IdentityError> {
            if self.env().caller() != self.admin {
                return Err(IdentityError::Unauthorized);
            }

            let now = self.env().block_timestamp();
            let provider = VerificationProvider {
                provider_id,
                name,
                provider_type,
                is_active: true,
                verified_identities: 0,
                registered_at: now,
                supported_tiers,
            };

            self.verification_providers.insert(&provider_id, &provider);

            self.add_audit_entry(
                provider_id,
                self.env().caller(),
                "provider_registered".into(),
                "Verification provider registered".into(),
            );

            Ok(())
        }

        #[ink(message)]
        pub fn deactivate_provider(&mut self, provider_id: AccountId) -> Result<(), IdentityError> {
            if self.env().caller() != self.admin {
                return Err(IdentityError::Unauthorized);
            }

            let mut provider = self
                .verification_providers
                .get(&provider_id)
                .ok_or(IdentityError::IdentityNotFound)?;

            provider.is_active = false;
            self.verification_providers.insert(&provider_id, &provider);

            Ok(())
        }

        #[ink(message)]
        pub fn get_verification_provider(
            &self,
            provider_id: AccountId,
        ) -> Option<VerificationProvider> {
            self.verification_providers.get(&provider_id)
        }

        // ===== KYC Tier Verification - Issue #282 & #283 =====

        #[ink(message)]
        pub fn request_kyc_verification(
            &mut self,
            provider_id: AccountId,
            requested_tier: KycTier,
            evidence_hash: Option<Hash>,
        ) -> Result<u64, IdentityError> {
            let caller = self.env().caller();
            let timestamp = self.env().block_timestamp();

            // Verify provider exists and is active
            let provider = self
                .verification_providers
                .get(&provider_id)
                .ok_or(IdentityError::IdentityNotFound)?;

            if !provider.is_active {
                return Err(IdentityError::VerificationFailed);
            }

            // Verify provider supports the requested tier
            if !provider.supported_tiers.contains(&requested_tier) {
                return Err(IdentityError::VerificationFailed);
            }

            self.provider_request_count += 1;
            let request_id = self.provider_request_count;

            let request = ProviderVerificationRequest {
                request_id,
                applicant: caller,
                provider_id,
                requested_tier,
                evidence_hash,
                requested_at: timestamp,
                status: VerificationStatus::Pending,
                completed_at: None,
                result_metadata: [0u8; 128],
            };

            self.provider_verification_requests
                .insert(&request_id, &request);

            self.add_audit_entry(
                caller,
                caller,
                "kyc_requested".into(),
                format!("KYC verification requested for tier {:?}", requested_tier),
            );

            Ok(request_id)
        }

        #[ink(message)]
        pub fn complete_kyc_verification(
            &mut self,
            request_id: u64,
            approved: bool,
            result_metadata: [u8; 128],
        ) -> Result<(), IdentityError> {
            let caller = self.env().caller();
            let timestamp = self.env().block_timestamp();

            // Only authorized providers can complete verification
            let mut request = self
                .provider_verification_requests
                .get(&request_id)
                .ok_or(IdentityError::IdentityNotFound)?;

            if request.provider_id != caller {
                return Err(IdentityError::Unauthorized);
            }

            if request.status != VerificationStatus::Pending {
                return Err(IdentityError::VerificationFailed);
            }

            request.status = if approved {
                VerificationStatus::Approved
            } else {
                VerificationStatus::Rejected
            };
            request.completed_at = Some(timestamp);
            request.result_metadata = result_metadata;

            self.provider_verification_requests
                .insert(&request_id, &request);

            // If approved, update user's KYC tier
            if approved {
                // Update identity
                if let Some(mut identity) = self.identities.get(&request.applicant) {
                    identity.kyc_tier = request.requested_tier;
                    identity.last_activity = timestamp;

                    // Map KYC tier to verification level
                    identity.verification_level = match request.requested_tier {
                        KycTier::Tier0_Unverified | KycTier::Tier0Unverified => {
                            VerificationLevel::None
                        }
                        KycTier::Tier1_Basic | KycTier::Tier1Basic => VerificationLevel::Basic,
                        KycTier::Tier2_Standard | KycTier::Tier2Standard => {
                            VerificationLevel::Standard
                        }
                        KycTier::Tier3_Enhanced | KycTier::Tier3Enhanced => {
                            VerificationLevel::Enhanced
                        }
                        KycTier::Tier4_Premium | KycTier::Tier4Premium => {
                            VerificationLevel::Premium
                        }
                    };

                    identity.is_verified = true;
                    identity.verified_at = Some(timestamp);

                    self.identities.insert(&request.applicant, &identity);
                }

                // Update user's KYC tier mapping
                self.user_kyc_tiers
                    .insert(&request.applicant, &request.requested_tier);

                // Update provider's verified count
                if let Some(mut provider) = self.verification_providers.get(&request.provider_id) {
                    provider.verified_identities += 1;
                    self.verification_providers
                        .insert(&request.provider_id, &provider);
                }

                self.add_audit_entry(
                    request.applicant,
                    caller,
                    "kyc_approved".into(),
                    format!("KYC approved for tier {:?}", request.requested_tier),
                );
            } else {
                self.add_audit_entry(
                    request.applicant,
                    caller,
                    "kyc_rejected".into(),
                    "KYC verification rejected".into(),
                );
            }

            Ok(())
        }

        #[ink(message)]
        pub fn get_user_kyc_tier(&self, account: AccountId) -> Option<KycTier> {
            self.user_kyc_tiers.get(&account)
        }

        #[ink(message)]
        pub fn get_kyc_tier_privileges(&self, tier: KycTier) -> Option<KycTierPrivileges> {
            self.kyc_tier_privileges.get(&tier)
        }

        #[ink(message)]
        pub fn get_provider_verification_request(
            &self,
            request_id: u64,
        ) -> Option<ProviderVerificationRequest> {
            self.provider_verification_requests.get(&request_id)
        }

        #[ink(message)]
        pub fn check_tier_privileges(
            &self,
            account: AccountId,
            transaction_value: u128,
        ) -> Result<bool, IdentityError> {
            let tier = self
                .user_kyc_tiers
                .get(&account)
                .unwrap_or(KycTier::Tier0Unverified);

            let privileges = self
                .kyc_tier_privileges
                .get(&tier)
                .ok_or(IdentityError::IdentityNotFound)?;

            if transaction_value > privileges.max_transaction_value {
                return Ok(false);
            }

            Ok(privileges.can_trade)
        }

        // ===== GDPR Compliance - Issue #526 =====

        /// Export all personal data associated with the caller.
        #[ink(message)]
        pub fn export_personal_data(&self) -> Result<PersonalDataExport, IdentityError> {
            let caller = self.env().caller();
            let identity = self.identities.get(caller);
            let reputation = self.reputation_metrics.get(caller);

            // Build verification history from audit trail
            let mut verification_history: Vec<VerificationHistoryEntry> = Vec::new();
            let audit_count = self.account_audit_count.get(caller).unwrap_or(0);
            let start = if audit_count > 20 { audit_count - 20 } else { 0 };
            for i in start..audit_count {
                if let Some(entry_id) = self.account_audit_index.get((caller, i)) {
                    if let Some(entry) = self.audit_trail.get(entry_id) {
                        if entry.action == "identity_verified" {
                            verification_history.push(VerificationHistoryEntry {
                                verifier: entry.performed_by,
                                verification_level: VerificationLevel::Basic,
                                verified_at: entry.timestamp,
                            });
                        }
                    }
                }
            }

            // Get recent audit entries
            let mut audit_entries: Vec<AuditEntry> = Vec::new();
            let limit = audit_count.min(50);
            for i in 0..limit {
                if let Some(entry_id) = self.account_audit_index.get((caller, i)) {
                    if let Some(entry) = self.audit_trail.get(entry_id) {
                        audit_entries.push(entry);
                    }
                }
            }

            let kyc_tier = self.user_kyc_tiers.get(caller);

            self.env().emit_event(DataExportRequested {
                account: caller,
                timestamp: self.env().block_timestamp(),
            });

            Ok(PersonalDataExport {
                identity,
                reputation,
                verification_history,
                audit_entries,
                kyc_tier,
            })
        }

        /// Request deletion of personal data. Initiates a cooldown period.
        #[ink(message)]
        pub fn request_data_deletion(&mut self) -> Result<(), IdentityError> {
            let caller = self.env().caller();
            let now = self.env().block_timestamp();
            let cooldown = self.deletion_cooldown_blocks as u64 * 6000;
            let deletion = DataDeletionRequest {
                requested_at: now,
                cooldown_ends_at: now.saturating_add(cooldown),
                status: DataDeletionStatus::Requested,
            };
            self.data_deletion_requests.insert(&caller, &deletion);

            self.env().emit_event(DataDeletionRequested {
                account: caller,
                cooldown_ends_at: deletion.cooldown_ends_at,
                timestamp: now,
            });
            Ok(())
        }

        /// Confirm data deletion after cooldown has passed.
        /// Removes PII (identity, reputation, KYC tier) but preserves anonymized audit entries.
        #[ink(message)]
        pub fn confirm_data_deletion(&mut self) -> Result<(), IdentityError> {
            let caller = self.env().caller();
            let now = self.env().block_timestamp();
            let request = self
                .data_deletion_requests
                .get(caller)
                .ok_or(IdentityError::NotFound)?;

            if request.status != DataDeletionStatus::Requested {
                return Err(IdentityError::AlreadyExists);
            }
            if now < request.cooldown_ends_at {
                return Err(IdentityError::TimelockActive);
            }

            // Remove PII data
            self.identities.remove(caller);
            self.reputation_metrics.remove(caller);
            self.user_kyc_tiers.remove(caller);

            // Update deletion request status
            let mut updated = request;
            updated.status = DataDeletionStatus::Completed;
            self.data_deletion_requests.insert(&caller, &updated);

            self.env().emit_event(DataDeletionCompleted {
                account: caller,
                timestamp: now,
            });
            Ok(())
        }

        /// Admin: set the privacy policy document hash.
        #[ink(message)]
        pub fn set_privacy_policy_hash(&mut self, hash: [u8; 32]) -> Result<(), IdentityError> {
            if self.env().caller() != self.admin {
                return Err(IdentityError::Unauthorized);
            }
            self.privacy_policy_hash = hash;
            Ok(())
        }

        /// Query: get the privacy policy document hash.
        #[ink(message)]
        pub fn get_privacy_policy_hash(&self) -> [u8; 32] {
            self.privacy_policy_hash
        }

        /// Query: get deletion request status for caller.
        #[ink(message)]
        pub fn get_data_deletion_status(&self) -> Option<DataDeletionRequest> {
            let caller = self.env().caller();
            self.data_deletion_requests.get(caller)
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use ink::env::test;

        fn default_registry() -> IdentityRegistry {
            test::set_caller::<ink::env::DefaultEnvironment>(
                ink::env::test::default_accounts::<ink::env::DefaultEnvironment>().alice,
            );
            IdentityRegistry::new()
        }

        fn make_privacy() -> PrivacySettings {
            PrivacySettings {
                public_reputation: true,
                public_verification: true,
                data_sharing_consent: true,
                zero_knowledge_proof: false,
                selective_disclosure: Vec::new(),
            }
        }

        #[ink::test]
        fn test_audit_trail_on_create() {
            let mut reg = default_registry();
            let accounts = ink::env::test::default_accounts::<ink::env::DefaultEnvironment>();
            assert_eq!(reg.get_audit_count(), 0);
            reg.create_identity(
                "did:test:audit1".into(),
                vec![1u8; 32],
                "Ed25519".into(),
                None,
                make_privacy(),
            )
            .unwrap();
            assert_eq!(reg.get_audit_count(), 1);
            let entry = reg.get_audit_entry(1).unwrap();
            assert_eq!(entry.action, "identity_created");
            assert_eq!(entry.account, accounts.alice);
        }

        #[ink::test]
        fn test_audit_trail_on_verify() {
            let mut reg = default_registry();
            let accounts = ink::env::test::default_accounts::<ink::env::DefaultEnvironment>();
            reg.create_identity(
                "did:test:audit2".into(),
                vec![1u8; 32],
                "Ed25519".into(),
                None,
                make_privacy(),
            )
            .unwrap();
            reg.add_authorized_verifier(accounts.alice).unwrap();
            reg.verify_identity(accounts.alice, VerificationLevel::Basic, None)
                .unwrap();
            assert_eq!(reg.get_audit_count(), 2);
            let entries = reg.get_account_audit_entries(accounts.alice, 0, 10);
            assert_eq!(entries.len(), 2);
            assert_eq!(entries[1].action, "identity_verified");
        }

        #[ink::test]
        fn test_revoke_identity() {
            let mut reg = default_registry();
            let accounts = ink::env::test::default_accounts::<ink::env::DefaultEnvironment>();
            // Create identity as bob
            ink::env::test::set_caller::<ink::env::DefaultEnvironment>(accounts.bob);
            reg.create_identity(
                "did:test:revoke1".into(),
                vec![1u8; 32],
                "Ed25519".into(),
                None,
                make_privacy(),
            )
            .unwrap();
            // Admin revokes
            ink::env::test::set_caller::<ink::env::DefaultEnvironment>(accounts.alice);
            assert_eq!(
                reg.revoke_identity(accounts.bob, "Compromised".into()),
                Ok(())
            );
            assert!(reg.is_revoked(accounts.bob));
            let record = reg.get_revocation(accounts.bob).unwrap();
            assert_eq!(record.reason, "Compromised");
            assert_eq!(record.revoked_by, accounts.alice);
            let identity = reg.get_identity(accounts.bob).unwrap();
            assert!(!identity.is_verified);
            assert_eq!(identity.trust_score, 0);
        }

        #[ink::test]
        fn test_revoke_unauthorized() {
            let mut reg = default_registry();
            let accounts = ink::env::test::default_accounts::<ink::env::DefaultEnvironment>();
            reg.create_identity(
                "did:test:revoke2".into(),
                vec![1u8; 32],
                "Ed25519".into(),
                None,
                make_privacy(),
            )
            .unwrap();
            ink::env::test::set_caller::<ink::env::DefaultEnvironment>(accounts.charlie);
            assert_eq!(
                reg.revoke_identity(accounts.alice, "Unauthorized".into()),
                Err(IdentityError::Unauthorized)
            );
        }

        #[ink::test]
        fn test_kyc_tier_privileges_initialized() {
            let reg = default_registry();

            // Check that KYC tiers are initialized
            let tier0 = reg.get_kyc_tier_privileges(KycTier::Tier0Unverified);
            assert!(tier0.is_some());
            assert!(!tier0.unwrap().can_trade);

            let tier2 = reg.get_kyc_tier_privileges(KycTier::Tier2Standard);
            assert!(tier2.is_some());
            let tier2 = tier2.unwrap();
            assert!(tier2.can_trade);
            assert!(tier2.can_withdraw);
        }

        #[ink::test]
        fn test_register_verification_provider() {
            let mut reg = default_registry();
            let accounts = ink::env::test::default_accounts::<ink::env::DefaultEnvironment>();
            let provider_id = accounts.bob;

            let name = [0x50u8; 64];
            let supported_tiers = vec![KycTier::Tier1Basic, KycTier::Tier2Standard];

            reg.register_verification_provider(
                provider_id,
                name,
                ProviderType::DocumentVerification,
                supported_tiers.clone(),
            )
            .unwrap();

            let provider = reg.get_verification_provider(provider_id);
            assert!(provider.is_some());
            let provider = provider.unwrap();
            assert!(provider.is_active);
            assert_eq!(provider.supported_tiers, supported_tiers);
            assert_eq!(provider.provider_type, ProviderType::DocumentVerification);
        }

        #[ink::test]
        fn test_kyc_verification_flow() {
            let mut reg = default_registry();
            let accounts = ink::env::test::default_accounts::<ink::env::DefaultEnvironment>();

            // Create identity as bob
            ink::env::test::set_caller::<ink::env::DefaultEnvironment>(accounts.bob);
            reg.create_identity(
                "did:test:kyc1".into(),
                vec![1u8; 32],
                "Ed25519".into(),
                None,
                make_privacy(),
            )
            .unwrap();

            // Register a provider as admin
            ink::env::test::set_caller::<ink::env::DefaultEnvironment>(accounts.alice);
            let provider_id = accounts.charlie;
            reg.register_verification_provider(
                provider_id,
                [0x51u8; 64],
                ProviderType::GovernmentId,
                vec![KycTier::Tier1Basic, KycTier::Tier2Standard],
            )
            .unwrap();

            // Bob requests KYC verification
            ink::env::test::set_caller::<ink::env::DefaultEnvironment>(accounts.bob);
            let request_id = reg
                .request_kyc_verification(provider_id, KycTier::Tier2Standard, Some([0xAB; 32]))
                .request_kyc_verification(provider_id, KycTier::Tier2_Standard, Some([0xAB; 32]))
                .unwrap();

            assert_eq!(request_id, 1);

            // Provider completes verification (as charlie)
            ink::env::test::set_caller::<ink::env::DefaultEnvironment>(accounts.charlie);
            reg.complete_kyc_verification(request_id, true, [0xCD; 128])
                .unwrap();

            // Check Bob's KYC tier was updated
            let bob_tier = reg.get_user_kyc_tier(accounts.bob);
            assert!(bob_tier.is_some());
            assert_eq!(bob_tier.unwrap(), KycTier::Tier2Standard);

            // Check Bob's identity was updated
            let bob_identity = reg.get_identity(accounts.bob).unwrap();
            assert_eq!(bob_identity.kyc_tier, KycTier::Tier2Standard);
            assert_eq!(bob_identity.verification_level, VerificationLevel::Standard);
        }

        #[ink::test]
        fn test_check_tier_privileges() {
            let mut reg = default_registry();
            let accounts = ink::env::test::default_accounts::<ink::env::DefaultEnvironment>();

            // Create identity and get Tier2
            ink::env::test::set_caller::<ink::env::DefaultEnvironment>(accounts.bob);
            reg.create_identity(
                "did:test:tier1".into(),
                vec![1u8; 32],
                "Ed25519".into(),
                None,
                make_privacy(),
            )
            .unwrap();

            // Register provider and complete KYC
            ink::env::test::set_caller::<ink::env::DefaultEnvironment>(accounts.alice);
            reg.register_verification_provider(
                accounts.charlie,
                [0x52u8; 64],
                ProviderType::FinancialVerification,
                vec![KycTier::Tier3Enhanced],
            )
            .unwrap();

            ink::env::test::set_caller::<ink::env::DefaultEnvironment>(accounts.bob);
            let request_id = reg
                .request_kyc_verification(accounts.charlie, KycTier::Tier3Enhanced, None)
                .request_kyc_verification(accounts.charlie, KycTier::Tier3_Enhanced, None)
                .unwrap();

            ink::env::test::set_caller::<ink::env::DefaultEnvironment>(accounts.charlie);
            reg.complete_kyc_verification(request_id, true, [0u8; 128])
                .unwrap();

            // Check privileges
            ink::env::test::set_caller::<ink::env::DefaultEnvironment>(accounts.alice);
            let can_trade = reg
                .check_tier_privileges(accounts.bob, 500_000_000_000_000_000_000)
                .unwrap();
            assert!(can_trade);

            // Unverified user should have limited privileges
            let unverified_can_trade = reg
                .check_tier_privileges(accounts.dave, 1_000_000_000_000_000_000)
                .unwrap();
            assert!(!unverified_can_trade);
        }

        #[ink::test]
        fn test_data_export_returns_identity_data() {
            let mut reg = default_registry();
            let accounts = ink::env::test::default_accounts::<ink::env::DefaultEnvironment>();

            reg.create_identity(
                "did:test:export1".into(),
                vec![1u8; 32],
                "Ed25519".into(),
                None,
                make_privacy(),
            )
            .unwrap();

            let export = reg.export_personal_data().unwrap();
            assert!(export.identity.is_some());
            assert_eq!(export.identity.as_ref().unwrap().account_id, accounts.alice);
            assert!(export.reputation.is_some());
            assert!(export.kyc_tier.is_some());
            assert!(export.verification_history.is_empty());
        }

        #[ink::test]
        fn test_data_deletion_flow() {
            let mut reg = default_registry();
            let accounts = ink::env::test::default_accounts::<ink::env::DefaultEnvironment>();

            reg.create_identity(
                "did:test:deletion1".into(),
                vec![1u8; 32],
                "Ed25519".into(),
                None,
                make_privacy(),
            )
            .unwrap();

            // Request deletion
            reg.request_data_deletion().unwrap();

            // Cannot confirm before cooldown
            assert_eq!(
                reg.confirm_data_deletion(),
                Err(IdentityError::TimelockActive)
            );

            // Advance time past cooldown (100 blocks * 6000ms = 600000ms, so 600 blocks at 1000ms each)
            for _ in 0..1000 {
                ink::env::test::advance_block::<ink::env::DefaultEnvironment>();
            }

            // Confirm deletion after cooldown
            reg.confirm_data_deletion().unwrap();

            // Verify PII data is removed
            assert!(reg.get_identity(accounts.alice).is_none());
            assert!(reg.get_reputation_metrics(accounts.alice).is_none());
            assert!(reg.get_user_kyc_tier(accounts.alice).is_none());

            // Verify deletion status
            let status = reg.get_data_deletion_status().unwrap();
            assert_eq!(status.status, DataDeletionStatus::Completed);
        }

        #[ink::test]
        fn test_set_privacy_policy_hash_admin_only() {
            let mut reg = default_registry();
            let accounts = ink::env::test::default_accounts::<ink::env::DefaultEnvironment>();

            // Admin can set hash
            let hash = [0x42u8; 32];
            assert_eq!(reg.set_privacy_policy_hash(hash), Ok(()));
            assert_eq!(reg.get_privacy_policy_hash(), hash);

            // Non-admin cannot set hash
            ink::env::test::set_caller::<ink::env::DefaultEnvironment>(accounts.bob);
            assert_eq!(
                reg.set_privacy_policy_hash([0x00u8; 32]),
                Err(IdentityError::Unauthorized)
            );
        }
        /// Initiates the recovery process for an identity
        #[ink(message)]
        pub fn initiate_recovery(&mut self) -> Result<(), IdentityError> {
            let caller = self.env().caller();
            let mut identity = self.identities.get(&caller).ok_or(IdentityError::IdentityNotFound)?;

            if identity.social_recovery.is_recovery_active {
                return Err(IdentityError::RecoveryInProgress);
            }

            identity.social_recovery.is_recovery_active = true;
            identity.social_recovery.last_recovery_attempt = Some(self.env().block_timestamp());
            self.identities.insert(&caller, &identity);

            self.env().emit_event(RecoveryInitiated {
                account: caller,
                initiator: caller,
                timestamp: self.env().block_timestamp(),
            });

            Ok(())
        }

        /// Contributes to the recovery of an identity
        #[ink(message)]
        pub fn contribute_to_recovery(
            &mut self,
            account_to_recover: AccountId,
        ) -> Result<(), IdentityError> {
            let caller = self.env().caller();
            let mut identity = self.identities.get(&account_to_recover).ok_or(IdentityError::IdentityNotFound)?;

            if !identity.social_recovery.is_recovery_active {
                return Err(IdentityError::RecoveryNotActive);
            }

            if !identity.social_recovery.guardians.contains(&caller) {
                return Err(IdentityError::Unauthorized);
            }

            if identity.social_recovery.recovery_approvals.contains(&caller) {
                return Err(IdentityError::AlreadyExists);
            }

            identity.social_recovery.recovery_approvals.push(caller);

            if identity.social_recovery.recovery_approvals.len() as u8 >= identity.social_recovery.threshold {
                identity.social_recovery.recovery_completion_timestamp = Some(self.env().block_timestamp() + identity.social_recovery.timelock_period);
            }

            self.identities.insert(&account_to_recover, &identity);

            Ok(())
        }

        /// Completes the recovery of an identity
        #[ink(message)]
        pub fn complete_recovery(
            &mut self,
            old_account: AccountId,
            new_account: AccountId,
        ) -> Result<(), IdentityError> {
            let mut identity = self.identities.get(&old_account).ok_or(IdentityError::IdentityNotFound)?;

            if !identity.social_recovery.is_recovery_active {
                return Err(IdentityError::RecoveryNotActive);
            }

            if identity.social_recovery.recovery_approvals.len() as u8
                < identity.social_recovery.threshold
            {
                return Err(IdentityError::RecoveryThresholdNotMet);
            }

            if let Some(completion_timestamp) = identity.social_recovery.recovery_completion_timestamp {
                if self.env().block_timestamp() < completion_timestamp {
                    return Err(IdentityError::TimelockActive);
                }
            } else {
                return Err(IdentityError::RecoveryThresholdNotMet);
            }

            // Transfer identity to new account
            self.identities.remove(&old_account);
            identity.account_id = new_account;
            self.identities.insert(&new_account, &identity);

            // Reset recovery state
            let mut identity = self.identities.get(&new_account).unwrap();
            identity.social_recovery.is_recovery_active = false;
            identity.social_recovery.recovery_approvals.clear();
            identity.social_recovery.recovery_completion_timestamp = None;
            self.identities.insert(&new_account, &identity);

            self.env().emit_event(RecoveryCompleted {
                account: old_account,
                new_account,
                timestamp: self.env().block_timestamp(),
            });

            Ok(())
        }
    }
}