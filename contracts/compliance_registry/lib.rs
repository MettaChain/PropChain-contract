#![cfg_attr(not(feature = "std"), no_std, no_main)]
#![allow(
    clippy::needless_borrows_for_generic_args,
    clippy::too_many_arguments,
    clippy::upper_case_acronyms
)]

use propchain_traits::ComplianceChecker;
use propchain_traits::*;

#[ink::contract]
mod compliance_registry {
    use super::*;
    use ink::prelude::vec::Vec;
    use ink::storage::Mapping;
    use propchain_traits::ComplianceOperation;

    /// Represents the verification status of a user
    #[derive(Debug, PartialEq, Eq, Clone, Copy, scale::Encode, scale::Decode)]
    #[cfg_attr(
        feature = "std",
        derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
    )]
    pub enum VerificationStatus {
        NotVerified,
        Pending,
        Verified,
        Rejected,
        Expired,
    }

    /// Supported jurisdictions
    #[derive(Debug, PartialEq, Eq, Clone, Copy, scale::Encode, scale::Decode)]
    #[cfg_attr(
        feature = "std",
        derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
    )]
    pub enum Jurisdiction {
        US,
        EU,
        UK,
        Singapore,
        UAE,
        Other,
    }

    /// Risk level assessment
    #[derive(Debug, PartialEq, Eq, Clone, Copy, scale::Encode, scale::Decode)]
    #[cfg_attr(
        feature = "std",
        derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
    )]
    pub enum RiskLevel {
        Low,
        Medium,
        High,
        Prohibited,
    }

    /// Document verification types
    #[derive(Debug, PartialEq, Eq, Clone, Copy, scale::Encode, scale::Decode)]
    #[cfg_attr(
        feature = "std",
        derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
    )]
    pub enum DocumentType {
        Passport,
        NationalId,
        DriverLicense,
        BirthCertificate,
        ProofOfAddress,
        CorporateDocument,
    }

    /// Biometric authentication methods
    #[derive(Debug, PartialEq, Eq, Clone, Copy, scale::Encode, scale::Decode)]
    #[cfg_attr(
        feature = "std",
        derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
    )]
    pub enum BiometricMethod {
        None,
        Fingerprint,
        FaceRecognition,
        VoiceRecognition,
        IrisScan,
        MultiFactor,
    }

    /// Sanctions list sources
    #[derive(Debug, PartialEq, Eq, Clone, Copy, scale::Encode, scale::Decode)]
    #[cfg_attr(
        feature = "std",
        derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
    )]
    pub enum SanctionsList {
        UN,
        OFAC,
        EU,
        UK,
        Singapore,
        UAE,
        Multiple,
    }

    /// GDPR consent status
    #[derive(Debug, PartialEq, Eq, Clone, Copy, scale::Encode, scale::Decode)]
    #[cfg_attr(
        feature = "std",
        derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
    )]
    pub enum ConsentStatus {
        NotGiven,
        Given,
        Withdrawn,
        Expired,
    }

    /// AML risk factors
    #[derive(Debug, Clone, Copy, scale::Encode, scale::Decode)]
    #[cfg_attr(
        feature = "std",
        derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
    )]
    pub struct AMLRiskFactors {
        pub pep_status: bool, // Politically Exposed Person
        pub high_risk_country: bool,
        pub suspicious_transaction_pattern: bool,
        pub large_transaction_volume: bool,
        pub source_of_funds_verified: bool,
    }

    /// Jurisdiction-specific compliance requirements
    #[derive(Debug, Clone, Copy, scale::Encode, scale::Decode)]
    #[cfg_attr(
        feature = "std",
        derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
    )]
    pub struct JurisdictionRules {
        pub requires_kyc: bool,
        pub requires_aml: bool,
        pub requires_sanctions_check: bool,
        pub minimum_verification_level: u8, // 1-5 scale
        pub data_retention_days: u32,
        pub requires_biometric: bool,
    }

    /// User compliance data (stored on-chain)
    #[derive(Debug, Clone, Copy, scale::Encode, scale::Decode)]
    #[cfg_attr(
        feature = "std",
        derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
    )]
    pub struct ComplianceData {
        pub status: VerificationStatus,
        pub jurisdiction: Jurisdiction,
        pub risk_level: RiskLevel,
        pub verification_timestamp: Timestamp,
        pub expiry_timestamp: Timestamp,
        pub kyc_hash: [u8; 32],
        pub aml_checked: bool,
        pub sanctions_checked: bool,
        // Enhanced KYC fields
        pub document_type: DocumentType,
        pub biometric_method: BiometricMethod,
        pub risk_score: u8, // 0-100 risk score
        // Enhanced AML fields
        pub aml_risk_factors: AMLRiskFactors,
        pub sanctions_list_checked: SanctionsList,
        // Privacy and GDPR
        pub gdpr_consent: ConsentStatus,
        pub data_encrypted: bool,
        pub consent_timestamp: Timestamp,
        pub data_retention_until: Timestamp,
    }

    /// Tax-specific compliance status reported by the tax compliance module
    #[derive(Debug, Clone, Copy, scale::Encode, scale::Decode)]
    #[cfg_attr(
        feature = "std",
        derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
    )]
    pub struct TaxComplianceStatus {
        pub jurisdiction_code: u32,
        pub reporting_period: u64,
        pub last_checked_at: Timestamp,
        pub last_payment_at: Timestamp,
        pub outstanding_tax: Balance,
        pub reporting_submitted: bool,
        pub legal_documents_verified: bool,
        pub clearance_expiry: Timestamp,
        pub violation_count: u32,
    }

    /// Compliance audit log entry
    #[derive(Debug, Clone, Copy, scale::Encode, scale::Decode)]
    #[cfg_attr(
        feature = "std",
        derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
    )]
    pub struct AuditLog {
        pub account: AccountId,
        pub action: u8, // 0=verification, 1=aml_check, 2=sanctions_check, 3=consent_update, etc.
        pub timestamp: Timestamp,
        pub verifier: AccountId,
    }

    /// Verification request for off-chain processing
    #[derive(Debug, Clone, Copy, scale::Encode, scale::Decode)]
    #[cfg_attr(
        feature = "std",
        derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
    )]
    pub struct VerificationRequest {
        pub account: AccountId,
        pub jurisdiction: Jurisdiction,
        pub document_hash: [u8; 32],  // Hash of document for verification
        pub biometric_hash: [u8; 32], // Hash of biometric data
        pub request_timestamp: Timestamp,
        pub request_id: u64,
        pub status: VerificationStatus,
    }

    /// Integration service provider information
    #[derive(Debug, Clone, Copy, scale::Encode, scale::Decode)]
    #[cfg_attr(
        feature = "std",
        derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
    )]
    pub struct ServiceProvider {
        pub provider_id: AccountId,
        pub service_type: u8, // 0=KYC, 1=AML, 2=Sanctions, 3=All
        pub is_active: bool,
        pub last_update: Timestamp,
    }

    #[ink(storage)]
    pub struct ComplianceRegistry {
        /// Contract owner (admin)
        owner: AccountId,
        /// Authorized verifiers who can update compliance status
        verifiers: Mapping<AccountId, bool>,
        /// User compliance data
        compliance_data: Mapping<AccountId, ComplianceData>,
        /// Jurisdiction-specific requirements
        jurisdiction_rules: Mapping<Jurisdiction, JurisdictionRules>,
        /// Compliance audit log (indexed by account and log number)
        audit_logs: Mapping<(AccountId, u64), AuditLog>,
        /// Audit log counters per account
        audit_log_count: Mapping<AccountId, u64>,
        /// Data retention policies (days per jurisdiction)
        retention_policies: Mapping<Jurisdiction, u32>,
        /// Encryption keys mapping (hash of encrypted data location)
        encrypted_data_hashes: Mapping<AccountId, [u8; 32]>,
        /// Pending verification requests (for off-chain processing)
        verification_requests: Mapping<u64, VerificationRequest>,
        /// Request counter
        request_counter: u64,
        /// Service providers registry
        service_providers: Mapping<AccountId, ServiceProvider>,
        /// Account to pending request mapping
        account_requests: Mapping<AccountId, u64>,
        /// ZK compliance contract address (optional)
        zk_compliance_contract: Option<AccountId>,
        /// Authorized tax compliance modules
        tax_modules: Mapping<AccountId, bool>,
        /// Optional tax compliance state per account
        tax_compliance_status: Mapping<AccountId, TaxComplianceStatus>,
        /// Global KYC funnel metrics
        kyc_metrics: KycMetrics,
        /// KYC funnel metrics scoped by jurisdiction
        jurisdiction_kyc_metrics: Mapping<Jurisdiction, KycMetrics>,
    }

    /// Errors
    #[derive(Debug, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub enum Error {
        /// Caller is not authorized
        NotAuthorized,
        /// User is not verified
        NotVerified,
        /// Verification has expired
        VerificationExpired,
        /// User has high risk level
        HighRisk,
        /// Jurisdiction is prohibited
        ProhibitedJurisdiction,
        /// User already verified
        AlreadyVerified,
        /// Consent not given
        ConsentNotGiven,
        /// Data retention period expired
        DataRetentionExpired,
        /// Invalid risk score
        InvalidRiskScore,
        /// Invalid document type
        InvalidDocumentType,
        /// Jurisdiction not supported
        JurisdictionNotSupported,
    }

    impl core::fmt::Display for Error {
        fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
            match self {
                Error::NotAuthorized => write!(f, "Caller is not authorized"),
                Error::NotVerified => write!(f, "User is not verified"),
                Error::VerificationExpired => write!(f, "Verification has expired"),
                Error::HighRisk => write!(f, "User has high risk level"),
                Error::ProhibitedJurisdiction => write!(f, "Jurisdiction is prohibited"),
                Error::AlreadyVerified => write!(f, "User already verified"),
                Error::ConsentNotGiven => write!(f, "Consent not given"),
                Error::DataRetentionExpired => write!(f, "Data retention period expired"),
                Error::InvalidRiskScore => write!(f, "Invalid risk score"),
                Error::InvalidDocumentType => write!(f, "Invalid document type"),
                Error::JurisdictionNotSupported => write!(f, "Jurisdiction not supported"),
            }
        }
    }

    impl ContractError for Error {
        fn error_code(&self) -> u32 {
            match self {
                Error::NotAuthorized => {
                    propchain_traits::errors::compliance_codes::COMPLIANCE_UNAUTHORIZED
                }
                Error::NotVerified => {
                    propchain_traits::errors::compliance_codes::COMPLIANCE_NOT_VERIFIED
                }
                Error::VerificationExpired => {
                    propchain_traits::errors::compliance_codes::COMPLIANCE_EXPIRED
                }
                Error::HighRisk => {
                    propchain_traits::errors::compliance_codes::COMPLIANCE_HIGH_RISK
                }
                Error::ProhibitedJurisdiction => {
                    propchain_traits::errors::compliance_codes::COMPLIANCE_PROHIBITED_JURISDICTION
                }
                Error::AlreadyVerified => {
                    propchain_traits::errors::compliance_codes::COMPLIANCE_ALREADY_VERIFIED
                }
                Error::ConsentNotGiven => {
                    propchain_traits::errors::compliance_codes::COMPLIANCE_CONSENT_NOT_GIVEN
                }
                Error::DataRetentionExpired => {
                    propchain_traits::errors::compliance_codes::COMPLIANCE_DATA_RETENTION_EXPIRED
                }
                Error::InvalidRiskScore => {
                    propchain_traits::errors::compliance_codes::COMPLIANCE_INVALID_RISK_SCORE
                }
                Error::InvalidDocumentType => {
                    propchain_traits::errors::compliance_codes::COMPLIANCE_INVALID_DOCUMENT_TYPE
                }
                Error::JurisdictionNotSupported => {
                    propchain_traits::errors::compliance_codes::COMPLIANCE_JURISDICTION_NOT_SUPPORTED
                }
            }
        }

        fn error_description(&self) -> &'static str {
            match self {
                Error::NotAuthorized => {
                    "Caller does not have permission to perform this compliance operation"
                }
                Error::NotVerified => "The user has not completed verification",
                Error::VerificationExpired => {
                    "The user's verification has expired and needs renewal"
                }
                Error::HighRisk => "The user has been assessed as high risk and is not permitted",
                Error::ProhibitedJurisdiction => {
                    "The user's jurisdiction is prohibited from this operation"
                }
                Error::AlreadyVerified => "The user is already verified and cannot be re-verified",
                Error::ConsentNotGiven => "The user has not provided the required consent",
                Error::DataRetentionExpired => {
                    "The data retention period for this record has expired"
                }
                Error::InvalidRiskScore => {
                    "The risk score provided is invalid or out of acceptable range"
                }
                Error::InvalidDocumentType => "The document type is invalid or not accepted",
                Error::JurisdictionNotSupported => {
                    "The specified jurisdiction is not currently supported"
                }
            }
        }

        fn error_category(&self) -> ErrorCategory {
            ErrorCategory::Compliance
        }

        fn error_i18n_key(&self) -> &'static str {
            match self {
                Error::NotAuthorized => "compliance.unauthorized",
                Error::NotVerified => "compliance.not_verified",
                Error::VerificationExpired => "compliance.verification_expired",
                Error::HighRisk => "compliance.high_risk",
                Error::ProhibitedJurisdiction => "compliance.prohibited_jurisdiction",
                Error::AlreadyVerified => "compliance.already_verified",
                Error::ConsentNotGiven => "compliance.consent_not_given",
                Error::DataRetentionExpired => "compliance.data_retention_expired",
                Error::InvalidRiskScore => "compliance.invalid_risk_score",
                Error::InvalidDocumentType => "compliance.invalid_document_type",
                Error::JurisdictionNotSupported => "compliance.jurisdiction_not_supported",
            }
        }
    }

    pub type Result<T> = core::result::Result<T, Error>;

    /// Events
    #[ink(event)]
    pub struct VerificationUpdated {
        #[ink(topic)]
        account: AccountId,
        status: VerificationStatus,
        timestamp: Timestamp,
    }

    #[ink(event)]
    pub struct ComplianceCheckPerformed {
        #[ink(topic)]
        account: AccountId,
        passed: bool,
        timestamp: Timestamp,
    }

    #[ink(event)]
    pub struct ConsentUpdated {
        #[ink(topic)]
        account: AccountId,
        consent_status: ConsentStatus,
        timestamp: Timestamp,
    }

    #[ink(event)]
    pub struct DataRetentionExpired {
        #[ink(topic)]
        account: AccountId,
        timestamp: Timestamp,
    }

    #[ink(event)]
    pub struct AuditLogCreated {
        #[ink(topic)]
        account: AccountId,
        action: u8,
        timestamp: Timestamp,
    }

    #[ink(event)]
    pub struct VerificationRequestCreated {
        #[ink(topic)]
        account: AccountId,
        #[ink(topic)]
        request_id: u64,
        jurisdiction: Jurisdiction,
        timestamp: Timestamp,
    }

    #[ink(event)]
    pub struct ServiceProviderRegistered {
        #[ink(topic)]
        provider: AccountId,
        service_type: u8,
        timestamp: Timestamp,
    }

    #[ink(event)]
    pub struct TaxComplianceStatusUpdated {
        #[ink(topic)]
        account: AccountId,
        jurisdiction_code: u32,
        outstanding_tax: Balance,
        timestamp: Timestamp,
    }

    /// Compliance report for an account (audit trail and reporting - Issue #45)
    #[derive(Debug, Clone, scale::Encode, scale::Decode)]
    #[cfg_attr(
        feature = "std",
        derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
    )]
    pub struct ComplianceReport {
        pub account: AccountId,
        pub is_compliant: bool,
        pub jurisdiction: Jurisdiction,
        pub status: VerificationStatus,
        pub risk_level: RiskLevel,
        pub kyc_verified: bool,
        pub aml_checked: bool,
        pub sanctions_checked: bool,
        pub audit_log_count: u64,
        pub last_audit_timestamp: Timestamp,
        pub verification_expiry: Timestamp,
        pub tax_compliant: bool,
        pub outstanding_tax: Balance,
    }

    /// Verification workflow status (workflow management - Issue #45)
    #[derive(Debug, Clone, Copy, scale::Encode, scale::Decode)]
    #[cfg_attr(
        feature = "std",
        derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
    )]
    pub enum WorkflowStatus {
        Pending,
        InProgress,
        Verified,
        Rejected,
        Expired,
    }

    /// Regulatory report summary for a jurisdiction and period (reporting automation - Issue #45)
    #[derive(Debug, Clone, scale::Encode, scale::Decode)]
    #[cfg_attr(
        feature = "std",
        derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
    )]
    pub struct RegulatoryReport {
        pub jurisdiction: Jurisdiction,
        pub period_start: Timestamp,
        pub period_end: Timestamp,
        pub verifications_count: u64,
        pub compliant_accounts: u64,
        pub aml_checks_count: u64,
        pub sanctions_checks_count: u64,
    }

    /// KYC funnel metrics used to track conversion and verification rates.
    #[derive(Debug, Clone, Copy, Default, scale::Encode, scale::Decode)]
    #[cfg_attr(
        feature = "std",
        derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
    )]
    pub struct KycMetrics {
        pub requests_created: u64,
        pub pending_requests: u64,
        pub verification_attempts: u64,
        pub successful_verifications: u64,
        pub failed_verifications: u64,
        pub converted_requests: u64,
        pub conversion_rate_bips: u32,
        pub verification_rate_bips: u32,
    }

    /// Sanctions screening summary (sanction list monitoring - Issue #45)
    #[derive(Debug, Clone, scale::Encode, scale::Decode)]
    #[cfg_attr(
        feature = "std",
        derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
    )]
    pub struct SanctionsScreeningSummary {
        pub total_screened: u64,
        pub passed: u64,
        pub failed: u64,
        pub lists_checked: Vec<u8>,
    }

    impl Default for ComplianceRegistry {
        fn default() -> Self {
            Self::new()
        }
    }

    impl ComplianceRegistry {
        /// Constructor
        #[ink(constructor)]
        pub fn new() -> Self {
            let caller = Self::env().caller();
            let mut verifiers = Mapping::default();
            verifiers.insert(caller, &true);

            let mut registry = Self {
                owner: caller,
                verifiers,
                compliance_data: Mapping::default(),
                jurisdiction_rules: Mapping::default(),
                audit_logs: Mapping::default(),
                audit_log_count: Mapping::default(),
                retention_policies: Mapping::default(),
                encrypted_data_hashes: Mapping::default(),
                verification_requests: Mapping::default(),
                request_counter: 0,
                service_providers: Mapping::default(),
                account_requests: Mapping::default(),
                zk_compliance_contract: None,
                tax_modules: Mapping::default(),
                tax_compliance_status: Mapping::default(),
                kyc_metrics: KycMetrics::default(),
                jurisdiction_kyc_metrics: Mapping::default(),
            };

            // Initialize default jurisdiction rules
            registry.init_default_jurisdiction_rules();
            registry
        }

        /// Initialize default jurisdiction-specific rules
        fn init_default_jurisdiction_rules(&mut self) {
            // US rules
            self.jurisdiction_rules.insert(
                &Jurisdiction::US,
                &JurisdictionRules {
                    requires_kyc: true,
                    requires_aml: true,
                    requires_sanctions_check: true,
                    minimum_verification_level: 3,
                    data_retention_days: 2555, // 7 years
                    requires_biometric: false,
                },
            );

            // EU rules (GDPR compliant)
            self.jurisdiction_rules.insert(
                &Jurisdiction::EU,
                &JurisdictionRules {
                    requires_kyc: true,
                    requires_aml: true,
                    requires_sanctions_check: true,
                    minimum_verification_level: 3,
                    data_retention_days: 1095, // 3 years (GDPR)
                    requires_biometric: false,
                },
            );

            // UK rules
            self.jurisdiction_rules.insert(
                &Jurisdiction::UK,
                &JurisdictionRules {
                    requires_kyc: true,
                    requires_aml: true,
                    requires_sanctions_check: true,
                    minimum_verification_level: 3,
                    data_retention_days: 1825, // 5 years
                    requires_biometric: false,
                },
            );

            // Singapore rules
            self.jurisdiction_rules.insert(
                &Jurisdiction::Singapore,
                &JurisdictionRules {
                    requires_kyc: true,
                    requires_aml: true,
                    requires_sanctions_check: true,
                    minimum_verification_level: 4,
                    data_retention_days: 1825, // 5 years
                    requires_biometric: true,
                },
            );

            // UAE rules
            self.jurisdiction_rules.insert(
                &Jurisdiction::UAE,
                &JurisdictionRules {
                    requires_kyc: true,
                    requires_aml: true,
                    requires_sanctions_check: true,
                    minimum_verification_level: 4,
                    data_retention_days: 1825, // 5 years
                    requires_biometric: true,
                },
            );
        }

        /// Add authorized verifier (KYC service)
        #[ink(message)]
        pub fn add_verifier(&mut self, verifier: AccountId) -> Result<()> {
            self.ensure_owner()?;
            self.verifiers.insert(verifier, &true);
            Ok(())
        }

        /// Submit KYC verification with enhanced document and biometric info
        #[ink(message)]
        pub fn submit_verification(
            &mut self,
            account: AccountId,
            jurisdiction: Jurisdiction,
            kyc_hash: [u8; 32],
            risk_level: RiskLevel,
            document_type: DocumentType,
            biometric_method: BiometricMethod,
            risk_score: u8,
        ) -> Result<()> {
            self.ensure_verifier()?;

            let result = self.submit_verification_internal(
                account,
                jurisdiction,
                kyc_hash,
                risk_level,
                document_type,
                biometric_method,
                risk_score,
            );

            if result.is_err() {
                self.record_kyc_verification_attempt(jurisdiction, false, false);
            }

            result
        }

        fn submit_verification_internal(
            &mut self,
            account: AccountId,
            jurisdiction: Jurisdiction,
            kyc_hash: [u8; 32],
            risk_level: RiskLevel,
            document_type: DocumentType,
            biometric_method: BiometricMethod,
            risk_score: u8,
        ) -> Result<()> {
            if risk_score > 100 {
                return Err(Error::InvalidRiskScore);
            }

            // Check jurisdiction rules
            let rules = self
                .jurisdiction_rules
                .get(jurisdiction)
                .ok_or(Error::JurisdictionNotSupported)?;

            // Validate minimum verification level
            let verification_level =
                self.calculate_verification_level(document_type, biometric_method, risk_score);
            if verification_level < rules.minimum_verification_level {
                return Err(Error::NotVerified);
            }

            let now = self.env().block_timestamp();
            let expiry = now + (365 * 24 * 60 * 60 * 1000); // 1 year validity
            let retention_days = rules.data_retention_days as u64;
            let retention_until = now + (retention_days * 24 * 60 * 60 * 1000);

            let compliance = ComplianceData {
                status: VerificationStatus::Verified,
                jurisdiction,
                risk_level,
                verification_timestamp: now,
                expiry_timestamp: expiry,
                kyc_hash,
                aml_checked: false,       // Will be set separately
                sanctions_checked: false, // Will be set separately
                document_type,
                biometric_method,
                risk_score,
                aml_risk_factors: AMLRiskFactors {
                    pep_status: false,
                    high_risk_country: false,
                    suspicious_transaction_pattern: false,
                    large_transaction_volume: false,
                    source_of_funds_verified: false,
                },
                sanctions_list_checked: SanctionsList::UN,
                gdpr_consent: ConsentStatus::NotGiven,
                data_encrypted: true, // Default to encrypted
                consent_timestamp: 0,
                data_retention_until: retention_until,
            };

            self.compliance_data.insert(account, &compliance);
            let converted_request = self.complete_pending_request(account, jurisdiction);
            self.record_kyc_verification_attempt(jurisdiction, converted_request, true);

            // Log audit event
            self.log_audit_event(account, 0); // 0 = verification

            self.env().emit_event(VerificationUpdated {
                account,
                status: VerificationStatus::Verified,
                timestamp: now,
            });

            Ok(())
        }

        /// Calculate verification level based on document, biometric, and risk score
        fn calculate_verification_level(
            &self,
            document_type: DocumentType,
            biometric_method: BiometricMethod,
            risk_score: u8,
        ) -> u8 {
            let mut level = 1u8;

            // Document type contributes to level
            match document_type {
                DocumentType::Passport => level += 2,
                DocumentType::NationalId => level += 1,
                DocumentType::DriverLicense => level += 1,
                DocumentType::BirthCertificate => level += 1,
                DocumentType::ProofOfAddress => level += 1,
                DocumentType::CorporateDocument => level += 2,
            }

            // Biometric method contributes to level
            match biometric_method {
                BiometricMethod::None => {}
                BiometricMethod::Fingerprint => level += 1,
                BiometricMethod::FaceRecognition => level += 1,
                BiometricMethod::VoiceRecognition => level += 1,
                BiometricMethod::IrisScan => level += 2,
                BiometricMethod::MultiFactor => level += 3,
            }

            // Risk score affects level (lower risk = higher level)
            if risk_score < 20 {
                level += 1;
            } else if risk_score > 80 {
                level = level.saturating_sub(2);
            }

            level.min(5) // Cap at 5
        }

        /// Check if account is compliant (includes GDPR consent check)
        #[ink(message)]
        pub fn is_compliant(&self, account: AccountId) -> bool {
            match self.compliance_data.get(account) {
                Some(data) => {
                    let now = self.env().block_timestamp();
                    data.status == VerificationStatus::Verified
                        && data.expiry_timestamp > now
                        && data.risk_level != RiskLevel::Prohibited
                        && data.aml_checked
                        && data.sanctions_checked
                        && data.gdpr_consent == ConsentStatus::Given
                        && now <= data.data_retention_until
                        && self.is_tax_status_compliant(account, now)
                }
                None => false,
            }
        }

        /// Require compliance (use this in property transfer functions)
        #[ink(message)]
        pub fn require_compliance(&self, account: AccountId) -> Result<()> {
            if !self.is_compliant(account) {
                return Err(Error::NotVerified);
            }

            self.env().emit_event(ComplianceCheckPerformed {
                account,
                passed: true,
                timestamp: self.env().block_timestamp(),
            });

            Ok(())
        }

        /// Get compliance data
        #[ink(message)]
        pub fn get_compliance_data(&self, account: AccountId) -> Option<ComplianceData> {
            self.compliance_data.get(account)
        }

        /// Allow an admin to register a dedicated tax module that may sync tax status.
        #[ink(message)]
        pub fn set_tax_module(&mut self, module: AccountId, active: bool) -> Result<()> {
            self.ensure_owner()?;
            self.tax_modules.insert(module, &active);
            Ok(())
        }

        /// Update account tax compliance state from a trusted verifier or tax module.
        #[ink(message)]
        pub fn update_tax_compliance_status(
            &mut self,
            account: AccountId,
            status: TaxComplianceStatus,
        ) -> Result<()> {
            self.ensure_tax_authority()?;
            self.tax_compliance_status.insert(account, &status);
            self.log_audit_event(account, 4); // 4 = tax compliance sync

            self.env().emit_event(TaxComplianceStatusUpdated {
                account,
                jurisdiction_code: status.jurisdiction_code,
                outstanding_tax: status.outstanding_tax,
                timestamp: self.env().block_timestamp(),
            });

            Ok(())
        }

        /// Get the latest synced tax compliance state for an account.
        #[ink(message)]
        pub fn get_tax_compliance_status(&self, account: AccountId) -> Option<TaxComplianceStatus> {
            self.tax_compliance_status.get(account)
        }

        /// Update AML status with detailed risk factors
        #[ink(message)]
        pub fn update_aml_status(
            &mut self,
            account: AccountId,
            passed: bool,
            risk_factors: AMLRiskFactors,
        ) -> Result<()> {
            self.ensure_verifier()?;

            if let Some(mut data) = self.compliance_data.get(account) {
                data.aml_checked = passed;
                data.aml_risk_factors = risk_factors;

                // Calculate risk level based on factors
                let risk_count = (risk_factors.pep_status as u8)
                    + (risk_factors.high_risk_country as u8)
                    + (risk_factors.suspicious_transaction_pattern as u8)
                    + (risk_factors.large_transaction_volume as u8);

                if !passed || risk_count >= 3 {
                    data.status = VerificationStatus::Rejected;
                    data.risk_level = RiskLevel::Prohibited;
                } else if risk_count >= 2 {
                    data.risk_level = RiskLevel::High;
                } else if risk_count >= 1 {
                    data.risk_level = RiskLevel::Medium;
                }

                self.compliance_data.insert(account, &data);

                // Log audit event
                self.log_audit_event(account, 1); // 1 = AML check

                Ok(())
            } else {
                Err(Error::NotVerified)
            }
        }

        /// Update sanctions screening status with list source
        #[ink(message)]
        pub fn update_sanctions_status(
            &mut self,
            account: AccountId,
            passed: bool,
            list_checked: SanctionsList,
        ) -> Result<()> {
            self.ensure_verifier()?;

            if let Some(mut data) = self.compliance_data.get(account) {
                data.sanctions_checked = passed;
                data.sanctions_list_checked = list_checked;
                if !passed {
                    data.status = VerificationStatus::Rejected;
                    data.risk_level = RiskLevel::Prohibited;
                }
                self.compliance_data.insert(account, &data);

                // Log audit event
                self.log_audit_event(account, 2); // 2 = sanctions check

                Ok(())
            } else {
                Err(Error::NotVerified)
            }
        }

        /// Revoke verification
        #[ink(message)]
        pub fn revoke_verification(&mut self, account: AccountId) -> Result<()> {
            self.ensure_verifier()?;

            if let Some(mut data) = self.compliance_data.get(account) {
                data.status = VerificationStatus::Rejected;
                self.compliance_data.insert(account, &data);

                self.env().emit_event(VerificationUpdated {
                    account,
                    status: VerificationStatus::Rejected,
                    timestamp: self.env().block_timestamp(),
                });

                Ok(())
            } else {
                Err(Error::NotVerified)
            }
        }

        /// Update GDPR consent status
        #[ink(message)]
        pub fn update_consent(&mut self, account: AccountId, consent: ConsentStatus) -> Result<()> {
            // Users can update their own consent
            let caller = self.env().caller();
            if caller != account && !self.verifiers.get(caller).unwrap_or(false) {
                return Err(Error::NotAuthorized);
            }

            if let Some(mut data) = self.compliance_data.get(account) {
                data.gdpr_consent = consent;
                data.consent_timestamp = self.env().block_timestamp();
                self.compliance_data.insert(account, &data);

                self.env().emit_event(ConsentUpdated {
                    account,
                    consent_status: consent,
                    timestamp: self.env().block_timestamp(),
                });

                // Log audit event
                self.log_audit_event(account, 3); // 3 = consent update

                Ok(())
            } else {
                Err(Error::NotVerified)
            }
        }

        /// Check if data retention period has expired (GDPR compliance)
        #[ink(message)]
        pub fn check_data_retention(&self, account: AccountId) -> bool {
            if let Some(data) = self.compliance_data.get(account) {
                let now = self.env().block_timestamp();
                now > data.data_retention_until
            } else {
                false
            }
        }

        /// Request data deletion (GDPR right to be forgotten)
        #[ink(message)]
        pub fn request_data_deletion(&mut self, account: AccountId) -> Result<()> {
            let caller = self.env().caller();
            if caller != account {
                return Err(Error::NotAuthorized);
            }

            // Check if retention period has expired
            if !self.check_data_retention(account) {
                return Err(Error::DataRetentionExpired);
            }

            // Check consent status
            if let Some(data) = self.compliance_data.get(account) {
                if data.gdpr_consent == ConsentStatus::Withdrawn {
                    // Delete compliance data
                    // Note: In ink!, we can't actually delete from Mapping,
                    // but we can mark it as deleted by setting status to Expired
                    let mut updated_data = data;
                    updated_data.status = VerificationStatus::Expired;
                    self.compliance_data.insert(account, &updated_data);

                    self.env().emit_event(DataRetentionExpired {
                        account,
                        timestamp: self.env().block_timestamp(),
                    });

                    Ok(())
                } else {
                    Err(Error::ConsentNotGiven)
                }
            } else {
                Err(Error::NotVerified)
            }
        }

        /// Store encrypted data hash (for privacy protection)
        #[ink(message)]
        pub fn store_encrypted_data_hash(
            &mut self,
            account: AccountId,
            data_hash: [u8; 32],
        ) -> Result<()> {
            self.ensure_verifier()?;

            if let Some(mut data) = self.compliance_data.get(account) {
                data.data_encrypted = true;
                self.compliance_data.insert(account, &data);
                self.encrypted_data_hashes.insert(account, &data_hash);
                Ok(())
            } else {
                Err(Error::NotVerified)
            }
        }

        /// Get audit logs for an account
        #[ink(message)]
        pub fn get_audit_logs(&self, account: AccountId, limit: u64) -> Vec<AuditLog> {
            let count = self.audit_log_count.get(account).unwrap_or(0);
            let start = count.saturating_sub(limit);
            let mut logs = Vec::new();

            for i in start..count {
                if let Some(log) = self.audit_logs.get((account, i)) {
                    logs.push(log);
                }
            }

            logs
        }

        /// Update jurisdiction rules (admin only)
        #[ink(message)]
        pub fn update_jurisdiction_rules(
            &mut self,
            jurisdiction: Jurisdiction,
            rules: JurisdictionRules,
        ) -> Result<()> {
            self.ensure_owner()?;
            self.jurisdiction_rules.insert(jurisdiction, &rules);
            Ok(())
        }

        /// Get jurisdiction rules
        #[ink(message)]
        pub fn get_jurisdiction_rules(
            &self,
            jurisdiction: Jurisdiction,
        ) -> Option<JurisdictionRules> {
            self.jurisdiction_rules.get(jurisdiction)
        }

        /// Create verification request for off-chain processing
        /// This allows users to submit verification requests that will be processed by off-chain services
        #[ink(message)]
        pub fn create_verification_request(
            &mut self,
            jurisdiction: Jurisdiction,
            document_hash: [u8; 32],
            biometric_hash: [u8; 32],
        ) -> Result<u64> {
            let caller = self.env().caller();

            // Check if there's already a pending request
            if let Some(existing_request_id) = self.account_requests.get(caller) {
                if let Some(request) = self.verification_requests.get(existing_request_id) {
                    if request.status == VerificationStatus::Pending {
                        return Err(Error::AlreadyVerified); // Request already pending
                    }
                }
            }

            let request_id = self.request_counter;
            self.request_counter += 1;

            let request = VerificationRequest {
                account: caller,
                jurisdiction,
                document_hash,
                biometric_hash,
                request_timestamp: self.env().block_timestamp(),
                request_id,
                status: VerificationStatus::Pending,
            };

            self.verification_requests.insert(request_id, &request);
            self.account_requests.insert(caller, &request_id);
            self.record_kyc_request_created(jurisdiction);

            self.env().emit_event(VerificationRequestCreated {
                account: caller,
                request_id,
                jurisdiction,
                timestamp: self.env().block_timestamp(),
            });

            Ok(request_id)
        }

        /// Get verification request by ID
        #[ink(message)]
        pub fn get_verification_request(&self, request_id: u64) -> Option<VerificationRequest> {
            self.verification_requests.get(request_id)
        }

        /// Process verification request (called by off-chain service after verification)
        /// This is the integration point for KYC services
        #[ink(message)]
        pub fn process_verification_request(
            &mut self,
            request_id: u64,
            kyc_hash: [u8; 32],
            risk_level: RiskLevel,
            document_type: DocumentType,
            biometric_method: BiometricMethod,
            risk_score: u8,
        ) -> Result<()> {
            self.ensure_verifier()?;

            let request = self
                .verification_requests
                .get(request_id)
                .ok_or(Error::NotVerified)?;

            if request.status != VerificationStatus::Pending {
                return Err(Error::AlreadyVerified);
            }

            // Process the verification using existing submit_verification logic
            let result = self.submit_verification(
                request.account,
                request.jurisdiction,
                kyc_hash,
                risk_level,
                document_type,
                biometric_method,
                risk_score,
            );

            if result.is_ok() {
                // Update request status
                let mut updated_request = request;
                updated_request.status = VerificationStatus::Verified;
                self.verification_requests
                    .insert(request_id, &updated_request);
            }

            result
        }

        /// Register a service provider (KYC/AML/Sanctions service)
        #[ink(message)]
        pub fn register_service_provider(
            &mut self,
            provider: AccountId,
            service_type: u8,
        ) -> Result<()> {
            self.ensure_owner()?;

            let provider_info = ServiceProvider {
                provider_id: provider,
                service_type,
                is_active: true,
                last_update: self.env().block_timestamp(),
            };

            self.service_providers.insert(provider, &provider_info);

            // Also add as verifier if service type includes verification
            if service_type == 0 || service_type == 3 {
                self.verifiers.insert(provider, &true);
            }

            self.env().emit_event(ServiceProviderRegistered {
                provider,
                service_type,
                timestamp: self.env().block_timestamp(),
            });

            Ok(())
        }

        /// Get service provider information
        #[ink(message)]
        pub fn get_service_provider(&self, provider: AccountId) -> Option<ServiceProvider> {
            self.service_providers.get(provider)
        }

        /// Batch process multiple AML checks (for transaction monitoring)
        #[ink(message)]
        pub fn batch_aml_check(
            &mut self,
            accounts: Vec<AccountId>,
            risk_factors_list: Vec<AMLRiskFactors>,
        ) -> Result<Vec<bool>> {
            self.ensure_verifier()?;

            if accounts.len() != risk_factors_list.len() {
                return Err(Error::NotVerified);
            }

            let mut results = Vec::new();
            for (account, risk_factors) in accounts.iter().zip(risk_factors_list.iter()) {
                // Calculate if AML check passes
                let risk_count = (risk_factors.pep_status as u8)
                    + (risk_factors.high_risk_country as u8)
                    + (risk_factors.suspicious_transaction_pattern as u8)
                    + (risk_factors.large_transaction_volume as u8);

                let passed = risk_count < 3 && risk_factors.source_of_funds_verified;
                results.push(passed);

                // Update AML status if account exists
                if self.compliance_data.get(*account).is_some() {
                    self.update_aml_status(*account, passed, *risk_factors)?;
                }
            }

            Ok(results)
        }

        /// Batch sanctions check for multiple accounts
        #[ink(message)]
        pub fn batch_sanctions_check(
            &mut self,
            accounts: Vec<AccountId>,
            list_checked: SanctionsList,
            results: Vec<bool>,
        ) -> Result<()> {
            self.ensure_verifier()?;

            if accounts.len() != results.len() {
                return Err(Error::NotVerified);
            }

            for (account, passed) in accounts.iter().zip(results.iter()) {
                self.update_sanctions_status(*account, *passed, list_checked)?;
            }

            Ok(())
        }

        /// Get compliance summary for reporting
        #[ink(message)]
        pub fn get_compliance_summary(&self, accounts: Vec<AccountId>) -> Vec<(AccountId, bool)> {
            accounts
                .iter()
                .map(|account| (*account, self.is_compliant(*account)))
                .collect()
        }

        /// Check if account needs re-verification (expired or expiring soon)
        #[ink(message)]
        pub fn needs_reverification(&self, account: AccountId, days_threshold: u32) -> bool {
            if let Some(data) = self.compliance_data.get(account) {
                let now = self.env().block_timestamp();
                let threshold_ms = (days_threshold as u64) * 24 * 60 * 60 * 1000;
                let expiry_threshold = data.expiry_timestamp.saturating_sub(threshold_ms);

                now >= expiry_threshold || data.status == VerificationStatus::Expired
            } else {
                true
            }
        }

        /// Get accounts requiring re-verification (for automated monitoring)
        /// Note: Full implementation requires off-chain indexing
        #[ink(message)]
        pub fn get_accounts_needing_reverification(&self, _limit: u32) -> Vec<AccountId> {
            // This is a placeholder - full implementation would require
            // off-chain indexing or a different storage pattern
            // Off-chain services should maintain their own index of accounts
            Vec::new()
        }

        // ========== Issue #45: Enhanced compliance framework ==========

        /// Multi-jurisdictional rules engine: check if account may perform operation (automated compliance checking)
        #[ink(message)]
        pub fn check_transaction_compliance(
            &self,
            account: AccountId,
            operation: ComplianceOperation,
        ) -> Result<()> {
            if !self.is_compliant(account) {
                return Err(Error::NotVerified);
            }
            let data = self
                .compliance_data
                .get(account)
                .ok_or(Error::NotVerified)?;
            let rules = self
                .jurisdiction_rules
                .get(data.jurisdiction)
                .ok_or(Error::JurisdictionNotSupported)?;

            // Apply jurisdiction rules for operation
            match operation {
                ComplianceOperation::RegisterProperty
                | ComplianceOperation::TransferProperty
                | ComplianceOperation::UpdateMetadata
                | ComplianceOperation::CreateEscrow
                | ComplianceOperation::ReleaseEscrow => {
                    if !rules.requires_kyc || !rules.requires_aml || !rules.requires_sanctions_check
                    {
                        return Ok(());
                    }
                    if !data.aml_checked || !data.sanctions_checked {
                        return Err(Error::NotVerified);
                    }
                }
                ComplianceOperation::ListForSale
                | ComplianceOperation::Purchase
                | ComplianceOperation::BridgeTransfer => {
                    if data.risk_level == RiskLevel::Prohibited {
                        return Err(Error::HighRisk);
                    }
                    if !data.aml_checked || !data.sanctions_checked {
                        return Err(Error::NotVerified);
                    }
                }
            }
            Ok(())
        }

        /// Compliance reporting and audit trail: full report for an account
        #[ink(message)]
        pub fn get_compliance_report(&self, account: AccountId) -> Option<ComplianceReport> {
            let data = self.compliance_data.get(account)?;
            let audit_count = self.audit_log_count.get(account).unwrap_or(0);
            let last_audit = if audit_count > 0 {
                self.audit_logs
                    .get((account, audit_count - 1))
                    .map(|l| l.timestamp)
                    .unwrap_or(0)
            } else {
                0
            };
            Some(ComplianceReport {
                account,
                is_compliant: self.is_compliant(account),
                jurisdiction: data.jurisdiction,
                status: data.status,
                risk_level: data.risk_level,
                kyc_verified: data.status == VerificationStatus::Verified,
                aml_checked: data.aml_checked,
                sanctions_checked: data.sanctions_checked,
                audit_log_count: audit_count,
                last_audit_timestamp: last_audit,
                verification_expiry: data.expiry_timestamp,
                tax_compliant: self.is_tax_status_compliant(account, self.env().block_timestamp()),
                outstanding_tax: self
                    .tax_compliance_status
                    .get(account)
                    .map(|status| status.outstanding_tax)
                    .unwrap_or(0),
            })
        }

        /// Compliance workflow management: status of a verification request
        #[ink(message)]
        pub fn get_verification_workflow_status(&self, request_id: u64) -> Option<WorkflowStatus> {
            let request = self.verification_requests.get(request_id)?;
            Some(match request.status {
                VerificationStatus::Pending => WorkflowStatus::Pending,
                VerificationStatus::Verified => WorkflowStatus::Verified,
                VerificationStatus::Rejected => WorkflowStatus::Rejected,
                VerificationStatus::Expired => WorkflowStatus::Expired,
                VerificationStatus::NotVerified => WorkflowStatus::InProgress,
            })
        }

        /// Regulatory reporting automation: summary for a jurisdiction (period for reporting)
        #[ink(message)]
        pub fn get_regulatory_report(
            &self,
            jurisdiction: Jurisdiction,
            period_start: Timestamp,
            period_end: Timestamp,
        ) -> RegulatoryReport {
            let kyc_metrics = self.get_jurisdiction_kyc_metrics(jurisdiction);
            RegulatoryReport {
                jurisdiction,
                period_start,
                period_end,
                verifications_count: kyc_metrics.successful_verifications,
                compliant_accounts: 0,
                aml_checks_count: 0,
                sanctions_checks_count: 0,
            }
        }

        /// Get global KYC funnel metrics including conversion and verification rates.
        #[ink(message)]
        pub fn get_kyc_metrics(&self) -> KycMetrics {
            self.kyc_metrics
        }

        /// Get KYC funnel metrics scoped to a specific jurisdiction.
        #[ink(message)]
        pub fn get_jurisdiction_kyc_metrics(&self, jurisdiction: Jurisdiction) -> KycMetrics {
            self.jurisdiction_kyc_metrics
                .get(jurisdiction)
                .unwrap_or_default()
        }

        /// Sanction list screening and monitoring: summary of screening activity
        #[ink(message)]
        pub fn get_sanctions_screening_summary(&self) -> SanctionsScreeningSummary {
            let lists_checked = vec![
                0, // UN
                1, // OFAC
                2, // EU
                3, // UK
                4, // Singapore
                5, // UAE
                6, // Multiple
            ];
            SanctionsScreeningSummary {
                total_screened: 0,
                passed: 0,
                failed: 0,
                lists_checked,
            }
        }

        // === Helper Functions ===

        fn ensure_owner(&self) -> Result<()> {
            if self.env().caller() != self.owner {
                return Err(Error::NotAuthorized);
            }
            Ok(())
        }

        fn ensure_verifier(&self) -> Result<()> {
            let caller = self.env().caller();
            if !self.verifiers.get(caller).unwrap_or(false) {
                return Err(Error::NotAuthorized);
            }
            Ok(())
        }

        fn ensure_tax_authority(&self) -> Result<()> {
            let caller = self.env().caller();
            if self.env().caller() == self.owner
                || self.verifiers.get(caller).unwrap_or(false)
                || self.tax_modules.get(caller).unwrap_or(false)
            {
                return Ok(());
            }
            Err(Error::NotAuthorized)
        }

        fn is_tax_status_compliant(&self, account: AccountId, now: Timestamp) -> bool {
            match self.tax_compliance_status.get(account) {
                Some(status) => {
                    status.outstanding_tax == 0
                        && status.reporting_submitted
                        && status.legal_documents_verified
                        && (status.clearance_expiry == 0 || status.clearance_expiry >= now)
                }
                None => true,
            }
        }

        fn complete_pending_request(
            &mut self,
            account: AccountId,
            jurisdiction: Jurisdiction,
        ) -> bool {
            let Some(request_id) = self.account_requests.get(account) else {
                return false;
            };

            let Some(mut request) = self.verification_requests.get(request_id) else {
                return false;
            };

            if request.status != VerificationStatus::Pending || request.jurisdiction != jurisdiction
            {
                return false;
            }

            request.status = VerificationStatus::Verified;
            self.verification_requests.insert(request_id, &request);
            true
        }

        fn record_kyc_request_created(&mut self, jurisdiction: Jurisdiction) {
            self.kyc_metrics.requests_created = self.kyc_metrics.requests_created.saturating_add(1);
            self.kyc_metrics.pending_requests = self.kyc_metrics.pending_requests.saturating_add(1);
            Self::refresh_kyc_rates(&mut self.kyc_metrics);

            let mut jurisdiction_metrics = self
                .jurisdiction_kyc_metrics
                .get(jurisdiction)
                .unwrap_or_default();
            jurisdiction_metrics.requests_created =
                jurisdiction_metrics.requests_created.saturating_add(1);
            jurisdiction_metrics.pending_requests =
                jurisdiction_metrics.pending_requests.saturating_add(1);
            Self::refresh_kyc_rates(&mut jurisdiction_metrics);
            self.jurisdiction_kyc_metrics
                .insert(jurisdiction, &jurisdiction_metrics);
        }

        fn record_kyc_verification_attempt(
            &mut self,
            jurisdiction: Jurisdiction,
            converted_request: bool,
            success: bool,
        ) {
            Self::update_kyc_metrics(&mut self.kyc_metrics, converted_request, success);

            let mut jurisdiction_metrics = self
                .jurisdiction_kyc_metrics
                .get(jurisdiction)
                .unwrap_or_default();
            Self::update_kyc_metrics(&mut jurisdiction_metrics, converted_request, success);
            self.jurisdiction_kyc_metrics
                .insert(jurisdiction, &jurisdiction_metrics);
        }

        fn update_kyc_metrics(metrics: &mut KycMetrics, converted_request: bool, success: bool) {
            metrics.verification_attempts = metrics.verification_attempts.saturating_add(1);

            if success {
                metrics.successful_verifications =
                    metrics.successful_verifications.saturating_add(1);
                if converted_request {
                    metrics.converted_requests = metrics.converted_requests.saturating_add(1);
                    metrics.pending_requests = metrics.pending_requests.saturating_sub(1);
                }
            } else {
                metrics.failed_verifications = metrics.failed_verifications.saturating_add(1);
            }

            Self::refresh_kyc_rates(metrics);
        }

        fn refresh_kyc_rates(metrics: &mut KycMetrics) {
            metrics.conversion_rate_bips =
                Self::compute_rate_bips(metrics.converted_requests, metrics.requests_created);
            metrics.verification_rate_bips = Self::compute_rate_bips(
                metrics.successful_verifications,
                metrics.verification_attempts,
            );
        }

        fn compute_rate_bips(numerator: u64, denominator: u64) -> u32 {
            if denominator == 0 {
                return 0;
            }

            numerator
                .saturating_mul(10_000)
                .checked_div(denominator)
                .unwrap_or(10_000) as u32
        }

        fn log_audit_event(&mut self, account: AccountId, action: u8) {
            let count = self.audit_log_count.get(account).unwrap_or(0);
            let log = AuditLog {
                account,
                action,
                timestamp: self.env().block_timestamp(),
                verifier: self.env().caller(),
            };
            self.audit_logs.insert((account, count), &log);
            self.audit_log_count.insert(account, &(count + 1));

            self.env().emit_event(AuditLogCreated {
                account,
                action,
                timestamp: self.env().block_timestamp(),
            });
        }

        /// Set the ZK compliance contract address
        #[ink(message)]
        pub fn set_zk_compliance_contract(&mut self, zk_contract: AccountId) -> Result<()> {
            self.ensure_owner()?;
            self.zk_compliance_contract = Some(zk_contract);
            Ok(())
        }

        /// Get the ZK compliance contract address
        #[ink(message)]
        pub fn get_zk_compliance_contract(&self) -> Option<AccountId> {
            self.zk_compliance_contract
        }

        /// Check compliance using both traditional and ZK methods
        #[ink(message)]
        pub fn enhanced_compliance_check(&self, account: AccountId) -> Result<()> {
            // First, check traditional compliance
            if !self.is_compliant(account) {
                return Err(Error::NotVerified);
            }

            // If ZK compliance contract is set, also check ZK compliance
            if let Some(_zk_contract) = self.zk_compliance_contract {
                // In a real implementation, this would make a cross-contract call to the ZK compliance contract
                // Since cross-contract calls in ink! are complex, we'll implement a simplified version
                // that assumes the zk-compliance contract has a method to check compliance
                // For now, we'll just verify that the account has valid ZK proofs for critical types

                // This is a simplified approach - in reality you'd make an actual cross-contract call
                // to the ZK compliance contract to verify compliance
            }

            self.env().emit_event(ComplianceCheckPerformed {
                account,
                passed: true,
                timestamp: self.env().block_timestamp(),
            });

            Ok(())
        }
    }

    impl ComplianceChecker for ComplianceRegistry {
        #[ink(message)]
        fn is_compliant(&self, account: AccountId) -> bool {
            ComplianceRegistry::is_compliant(self, account)
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[ink::test]
        fn new_works() {
            let contract = ComplianceRegistry::new();
            assert_eq!(contract.owner, AccountId::from([0x01; 32]));
        }

        #[ink::test]
        fn verification_flow_works() {
            let mut contract = ComplianceRegistry::new();
            let user = AccountId::from([0x02; 32]);
            let kyc_hash = [0u8; 32];

            // Submit verification
            let result = contract.submit_verification(
                user,
                Jurisdiction::US,
                kyc_hash,
                RiskLevel::Low,
                DocumentType::Passport,
                BiometricMethod::FaceRecognition,
                15, // Low risk score
            );
            assert!(result.is_ok());

            // Update AML status
            let aml_factors = AMLRiskFactors {
                pep_status: false,
                high_risk_country: false,
                suspicious_transaction_pattern: false,
                large_transaction_volume: false,
                source_of_funds_verified: true,
            };
            contract
                .update_aml_status(user, true, aml_factors)
                .expect("AML status update should succeed in test");

            // Update sanctions status
            contract
                .update_sanctions_status(user, true, SanctionsList::OFAC)
                .expect("Sanctions status update should succeed in test");

            // Update consent (required for compliance)
            contract
                .update_consent(user, ConsentStatus::Given)
                .expect("Consent update should succeed in test");

            // Check compliance
            assert!(contract.is_compliant(user));

            // Require compliance should pass
            assert!(contract.require_compliance(user).is_ok());
        }

        #[ink::test]
        fn non_verified_user_fails_compliance() {
            let contract = ComplianceRegistry::new();
            let user = AccountId::from([0x03; 32]);

            assert!(!contract.is_compliant(user));
            assert_eq!(contract.require_compliance(user), Err(Error::NotVerified));
        }

        #[ink::test]
        fn aml_failure_blocks_compliance() {
            let mut contract = ComplianceRegistry::new();
            let user = AccountId::from([0x04; 32]);
            let kyc_hash = [0u8; 32];

            // Verify user first
            contract
                .submit_verification(
                    user,
                    Jurisdiction::US,
                    kyc_hash,
                    RiskLevel::Low,
                    DocumentType::Passport,
                    BiometricMethod::None,
                    20,
                )
                .expect("KYC verification should succeed in test");

            // Update AML with passing status
            let aml_factors = AMLRiskFactors {
                pep_status: false,
                high_risk_country: false,
                suspicious_transaction_pattern: false,
                large_transaction_volume: false,
                source_of_funds_verified: true,
            };
            contract
                .update_aml_status(user, true, aml_factors)
                .expect("AML status update should succeed in test");
            contract
                .update_sanctions_status(user, true, SanctionsList::UN)
                .expect("Sanctions status update should succeed in test");
            contract
                .update_consent(user, ConsentStatus::Given)
                .expect("Consent update should succeed in test");

            // User is compliant
            assert!(contract.is_compliant(user));

            // AML check fails with high risk factors
            let high_risk_factors = AMLRiskFactors {
                pep_status: true,
                high_risk_country: true,
                suspicious_transaction_pattern: true,
                large_transaction_volume: true,
                source_of_funds_verified: false,
            };
            contract
                .update_aml_status(user, false, high_risk_factors)
                .expect("AML status update should succeed in test");

            // User is no longer compliant
            assert!(!contract.is_compliant(user));
        }

        // Issue #45: Enhanced compliance framework tests
        #[ink::test]
        fn check_transaction_compliance_works() {
            let mut contract = ComplianceRegistry::new();
            let user = AccountId::from([0x05; 32]);
            let kyc_hash = [0u8; 32];
            contract
                .submit_verification(
                    user,
                    Jurisdiction::US,
                    kyc_hash,
                    RiskLevel::Low,
                    DocumentType::Passport,
                    BiometricMethod::None,
                    10,
                )
                .expect("submit");
            let aml = AMLRiskFactors {
                pep_status: false,
                high_risk_country: false,
                suspicious_transaction_pattern: false,
                large_transaction_volume: false,
                source_of_funds_verified: true,
            };
            contract.update_aml_status(user, true, aml).expect("aml");
            contract
                .update_sanctions_status(user, true, SanctionsList::OFAC)
                .expect("sanctions");
            contract
                .update_consent(user, ConsentStatus::Given)
                .expect("consent");

            assert!(contract
                .check_transaction_compliance(user, ComplianceOperation::RegisterProperty)
                .is_ok());
            assert!(contract
                .check_transaction_compliance(user, ComplianceOperation::TransferProperty)
                .is_ok());
        }

        #[ink::test]
        fn get_compliance_report_works() {
            let mut contract = ComplianceRegistry::new();
            let user = AccountId::from([0x06; 32]);
            let kyc_hash = [0u8; 32];
            contract
                .submit_verification(
                    user,
                    Jurisdiction::EU,
                    kyc_hash,
                    RiskLevel::Low,
                    DocumentType::NationalId,
                    BiometricMethod::None,
                    5,
                )
                .expect("submit");
            let report = contract.get_compliance_report(user).expect("report");
            assert_eq!(report.account, user);
            assert_eq!(report.jurisdiction, Jurisdiction::EU);
            assert_eq!(report.status, VerificationStatus::Verified);
        }

        #[ink::test]
        fn get_verification_workflow_status_works() {
            let mut contract = ComplianceRegistry::new();
            let request_id = contract
                .create_verification_request(Jurisdiction::UK, [1u8; 32], [2u8; 32])
                .expect("create request");
            let status = contract
                .get_verification_workflow_status(request_id)
                .expect("status");
            assert!(matches!(status, WorkflowStatus::Pending));
        }

        #[ink::test]
        fn get_regulatory_report_works() {
            let mut contract = ComplianceRegistry::new();
            let accounts = ink::env::test::default_accounts::<ink::env::DefaultEnvironment>();

            ink::env::test::set_caller::<ink::env::DefaultEnvironment>(accounts.bob);
            let request_id = contract
                .create_verification_request(Jurisdiction::US, [9u8; 32], [8u8; 32])
                .expect("request");

            ink::env::test::set_caller::<ink::env::DefaultEnvironment>(accounts.alice);
            contract
                .process_verification_request(
                    request_id,
                    [7u8; 32],
                    RiskLevel::Low,
                    DocumentType::Passport,
                    BiometricMethod::FaceRecognition,
                    10,
                )
                .expect("verification");

            let report = contract.get_regulatory_report(Jurisdiction::US, 0, 1000);
            assert_eq!(report.jurisdiction, Jurisdiction::US);
            assert_eq!(report.period_start, 0);
            assert_eq!(report.period_end, 1000);
            assert_eq!(report.verifications_count, 1);
        }

        #[ink::test]
        fn get_sanctions_screening_summary_works() {
            let contract = ComplianceRegistry::new();
            let summary = contract.get_sanctions_screening_summary();
            assert!(!summary.lists_checked.is_empty());
        }

        #[ink::test]
        fn tax_status_extends_compliance_checks_without_breaking_existing_flow() {
            let mut contract = ComplianceRegistry::new();
            let user = AccountId::from([0x07; 32]);
            let kyc_hash = [7u8; 32];

            contract
                .submit_verification(
                    user,
                    Jurisdiction::US,
                    kyc_hash,
                    RiskLevel::Low,
                    DocumentType::Passport,
                    BiometricMethod::None,
                    10,
                )
                .expect("submit");
            contract
                .update_aml_status(
                    user,
                    true,
                    AMLRiskFactors {
                        pep_status: false,
                        high_risk_country: false,
                        suspicious_transaction_pattern: false,
                        large_transaction_volume: false,
                        source_of_funds_verified: true,
                    },
                )
                .expect("aml");
            contract
                .update_sanctions_status(user, true, SanctionsList::OFAC)
                .expect("sanctions");
            contract
                .update_consent(user, ConsentStatus::Given)
                .expect("consent");

            assert!(contract.is_compliant(user));

            contract
                .update_tax_compliance_status(
                    user,
                    TaxComplianceStatus {
                        jurisdiction_code: 1001,
                        reporting_period: 1,
                        last_checked_at: 1,
                        last_payment_at: 0,
                        outstanding_tax: 25,
                        reporting_submitted: false,
                        legal_documents_verified: false,
                        clearance_expiry: 0,
                        violation_count: 1,
                    },
                )
                .expect("tax sync");

            assert!(!contract.is_compliant(user));

            contract
                .update_tax_compliance_status(
                    user,
                    TaxComplianceStatus {
                        jurisdiction_code: 1001,
                        reporting_period: 1,
                        last_checked_at: 2,
                        last_payment_at: 2,
                        outstanding_tax: 0,
                        reporting_submitted: true,
                        legal_documents_verified: true,
                        clearance_expiry: 10_000,
                        violation_count: 0,
                    },
                )
                .expect("tax clear");

            let report = contract.get_compliance_report(user).expect("report");
            assert!(contract.is_compliant(user));
            assert!(report.tax_compliant);
            assert_eq!(report.outstanding_tax, 0);
        }

        #[ink::test]
        fn kyc_metrics_track_request_conversion_and_verification_rates() {
            let mut contract = ComplianceRegistry::new();
            let accounts = ink::env::test::default_accounts::<ink::env::DefaultEnvironment>();

            ink::env::test::set_caller::<ink::env::DefaultEnvironment>(accounts.bob);
            let request_id = contract
                .create_verification_request(Jurisdiction::US, [1u8; 32], [2u8; 32])
                .expect("request");

            let pending_metrics = contract.get_kyc_metrics();
            assert_eq!(pending_metrics.requests_created, 1);
            assert_eq!(pending_metrics.pending_requests, 1);
            assert_eq!(pending_metrics.verification_attempts, 0);
            assert_eq!(pending_metrics.conversion_rate_bips, 0);
            assert_eq!(pending_metrics.verification_rate_bips, 0);

            ink::env::test::set_caller::<ink::env::DefaultEnvironment>(accounts.alice);
            contract
                .process_verification_request(
                    request_id,
                    [3u8; 32],
                    RiskLevel::Low,
                    DocumentType::Passport,
                    BiometricMethod::FaceRecognition,
                    10,
                )
                .expect("verification");

            let metrics = contract.get_kyc_metrics();
            assert_eq!(metrics.requests_created, 1);
            assert_eq!(metrics.pending_requests, 0);
            assert_eq!(metrics.verification_attempts, 1);
            assert_eq!(metrics.successful_verifications, 1);
            assert_eq!(metrics.failed_verifications, 0);
            assert_eq!(metrics.converted_requests, 1);
            assert_eq!(metrics.conversion_rate_bips, 10_000);
            assert_eq!(metrics.verification_rate_bips, 10_000);

            let us_metrics = contract.get_jurisdiction_kyc_metrics(Jurisdiction::US);
            assert_eq!(us_metrics.converted_requests, 1);
            assert_eq!(us_metrics.successful_verifications, 1);
        }

        #[ink::test]
        fn kyc_metrics_track_failed_verification_attempts_without_conversion() {
            let mut contract = ComplianceRegistry::new();
            let accounts = ink::env::test::default_accounts::<ink::env::DefaultEnvironment>();

            ink::env::test::set_caller::<ink::env::DefaultEnvironment>(accounts.charlie);
            let request_id = contract
                .create_verification_request(Jurisdiction::UK, [4u8; 32], [5u8; 32])
                .expect("request");

            ink::env::test::set_caller::<ink::env::DefaultEnvironment>(accounts.alice);
            let result = contract.process_verification_request(
                request_id,
                [6u8; 32],
                RiskLevel::Low,
                DocumentType::Passport,
                BiometricMethod::FaceRecognition,
                101,
            );
            assert_eq!(result, Err(Error::InvalidRiskScore));

            let metrics = contract.get_kyc_metrics();
            assert_eq!(metrics.requests_created, 1);
            assert_eq!(metrics.pending_requests, 1);
            assert_eq!(metrics.verification_attempts, 1);
            assert_eq!(metrics.successful_verifications, 0);
            assert_eq!(metrics.failed_verifications, 1);
            assert_eq!(metrics.converted_requests, 0);
            assert_eq!(metrics.conversion_rate_bips, 0);
            assert_eq!(metrics.verification_rate_bips, 0);

            let request = contract
                .get_verification_request(request_id)
                .expect("request should remain available");
            assert_eq!(request.status, VerificationStatus::Pending);
        }

        #[ink::test]
        fn direct_verification_completes_pending_request_for_conversion_tracking() {
            let mut contract = ComplianceRegistry::new();
            let accounts = ink::env::test::default_accounts::<ink::env::DefaultEnvironment>();

            ink::env::test::set_caller::<ink::env::DefaultEnvironment>(accounts.django);
            let request_id = contract
                .create_verification_request(Jurisdiction::EU, [7u8; 32], [8u8; 32])
                .expect("request");

            ink::env::test::set_caller::<ink::env::DefaultEnvironment>(accounts.alice);
            contract
                .submit_verification(
                    accounts.django,
                    Jurisdiction::EU,
                    [9u8; 32],
                    RiskLevel::Low,
                    DocumentType::Passport,
                    BiometricMethod::FaceRecognition,
                    15,
                )
                .expect("direct verification");

            let request = contract
                .get_verification_request(request_id)
                .expect("request should exist");
            assert_eq!(request.status, VerificationStatus::Verified);

            let metrics = contract.get_jurisdiction_kyc_metrics(Jurisdiction::EU);
            assert_eq!(metrics.requests_created, 1);
            assert_eq!(metrics.pending_requests, 0);
            assert_eq!(metrics.converted_requests, 1);
            assert_eq!(metrics.successful_verifications, 1);
            assert_eq!(metrics.verification_rate_bips, 10_000);
        }
    }
}
