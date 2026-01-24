#![cfg_attr(not(feature = "std"), no_std, no_main)]

#[ink::contract]
mod compliance_registry {
    use ink::prelude::vec::Vec;
    use ink::storage::Mapping;

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
        pub document_hash: [u8; 32], // Hash of document for verification
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
    }

    /// Errors
    #[derive(Debug, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub enum Error {
        NotAuthorized,
        NotVerified,
        VerificationExpired,
        HighRisk,
        ProhibitedJurisdiction,
        AlreadyVerified,
        ConsentNotGiven,
        DataRetentionExpired,
        InvalidRiskScore,
        InvalidDocumentType,
        JurisdictionNotSupported,
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

    impl ComplianceRegistry {
        /// Constructor
        #[ink(constructor)]
        pub fn new() -> Self {
            let caller = Self::env().caller();
            let mut verifiers = Mapping::default();
            verifiers.insert(caller, &true);

            let mut registry =             Self {
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

            if risk_score > 100 {
                return Err(Error::InvalidRiskScore);
            }

            // Check jurisdiction rules
            let rules = self.jurisdiction_rules.get(jurisdiction)
                .ok_or(Error::JurisdictionNotSupported)?;

            // Validate minimum verification level
            let verification_level = self.calculate_verification_level(
                document_type,
                biometric_method,
                risk_score,
            );
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
                aml_checked: false, // Will be set separately
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
                BiometricMethod::None => {},
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
        pub fn get_jurisdiction_rules(&self, jurisdiction: Jurisdiction) -> Option<JurisdictionRules> {
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

            let request = self.verification_requests.get(request_id)
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
                self.verification_requests.insert(request_id, &updated_request);
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
        pub fn get_accounts_needing_reverification(
            &self,
            _limit: u32,
        ) -> Vec<AccountId> {
            // This is a placeholder - full implementation would require
            // off-chain indexing or a different storage pattern
            // Off-chain services should maintain their own index of accounts
            Vec::new()
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
            contract.update_aml_status(user, true, aml_factors).unwrap();

            // Update sanctions status
            contract.update_sanctions_status(user, true, SanctionsList::OFAC).unwrap();

            // Update consent (required for compliance)
            contract.update_consent(user, ConsentStatus::Given).unwrap();

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
            contract.submit_verification(
                user,
                Jurisdiction::US,
                kyc_hash,
                RiskLevel::Low,
                DocumentType::Passport,
                BiometricMethod::None,
                20,
            ).unwrap();

            // Update AML with passing status
            let aml_factors = AMLRiskFactors {
                pep_status: false,
                high_risk_country: false,
                suspicious_transaction_pattern: false,
                large_transaction_volume: false,
                source_of_funds_verified: true,
            };
            contract.update_aml_status(user, true, aml_factors).unwrap();
            contract.update_sanctions_status(user, true, SanctionsList::UN).unwrap();
            contract.update_consent(user, ConsentStatus::Given).unwrap();

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
            contract.update_aml_status(user, false, high_risk_factors).unwrap();

            // User is no longer compliant
            assert!(!contract.is_compliant(user));
        }
    }
}