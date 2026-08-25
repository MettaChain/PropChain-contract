#![allow(clippy::clone_on_copy)] // fires inside ink! generated storage code
#![cfg_attr(not(feature = "std"), no_std, no_main)]
#![allow(
    clippy::needless_borrows_for_generic_args,
    clippy::too_many_arguments,
    clippy::upper_case_acronyms
)]

#[ink::contract]
pub mod gdpr_consent {
    use ink::prelude::vec::Vec;
    use ink::storage::Mapping;

    // ── Errors ──────────────────────────────────────────────────────────────

    #[derive(Debug, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub enum Error {
        NotAuthorized,
        ConsentNotFound,
        ConsentAlreadyExists,
        DataSubjectNotFound,
        ProcessingPurposeNotFound,
        RetentionPeriodExceeded,
        InvalidDuration,
        DataRequestNotFound,
    }

    pub type Result<T> = core::result::Result<T, Error>;

    // ── Types ───────────────────────────────────────────────────────────────

    #[derive(Debug, Clone, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(
        feature = "std",
        derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
    )]
    pub enum ConsentStatus {
        Granted,
        Withdrawn,
        Expired,
    }

    #[derive(Debug, Clone, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(
        feature = "std",
        derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
    )]
    pub enum ProcessingPurpose {
        KYC,
        TaxReporting,
        RiskAssessment,
        PropertyValuation,
        TransactionMonitoring,
        Marketing,
        DataAnalytics,
        Other(Vec<u8>),
    }

    #[derive(Debug, Clone, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(
        feature = "std",
        derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
    )]
    pub struct ConsentRecord {
        /// Unique, monotonically increasing identifier of this consent.
        pub consent_id: u64,
        /// The natural person whose data may be processed.
        pub data_subject: AccountId,
        /// The contract admin acting as data processor / controller proxy.
        pub processor: AccountId,
        /// Scope of processing this consent covers (KYC, marketing, ...).
        /// A consent is only valid for exactly this purpose.
        pub purpose: ProcessingPurpose,
        /// Lifecycle state; effective validity additionally requires
        /// `expires_at` to be in the future.
        pub status: ConsentStatus,
        /// Block timestamp at which consent was recorded.
        pub granted_at: u64,
        /// Absolute expiry timestamp (`granted_at + duration_ms`). After
        /// this instant the consent is stale for `GdprConsent::check_consent`
        /// and the admin may transition it to `Expired`.
        pub expires_at: u64,
        /// Set when the subject (or admin) withdrew the consent.
        pub withdrawn_at: Option<u64>,
    }

    #[derive(Debug, Clone, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(
        feature = "std",
        derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
    )]
    pub struct DataRetentionPolicy {
        /// Processing purpose this policy governs.
        pub purpose: ProcessingPurpose,
        /// Maximum number of days data collected under `purpose` may be
        /// retained after collection.
        pub retention_days: u64,
        /// Whether reaching the retention bound triggers automatic erasure
        /// (as opposed to manual review before deletion).
        pub auto_delete: bool,
    }

    #[derive(Debug, Clone, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(
        feature = "std",
        derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
    )]
    pub struct DataAccessRequest {
        pub request_id: u64,
        pub data_subject: AccountId,
        pub requested_at: u64,
        pub fulfilled: bool,
        pub fulfilled_at: Option<u64>,
    }

    // ── Events ──────────────────────────────────────────────────────────────

    #[ink(event)]
    pub struct ConsentGranted {
        #[ink(topic)]
        data_subject: AccountId,
        consent_id: u64,
        purpose: ProcessingPurpose,
        timestamp: u64,
    }

    #[ink(event)]
    pub struct ConsentWithdrawn {
        #[ink(topic)]
        data_subject: AccountId,
        consent_id: u64,
        purpose: ProcessingPurpose,
        timestamp: u64,
    }

    #[ink(event)]
    pub struct ConsentExpired {
        #[ink(topic)]
        data_subject: AccountId,
        consent_id: u64,
        timestamp: u64,
    }

    #[ink(event)]
    pub struct DataAccessRequested {
        #[ink(topic)]
        data_subject: AccountId,
        request_id: u64,
        timestamp: u64,
    }

    #[ink(event)]
    pub struct DataAccessFulfilled {
        #[ink(topic)]
        data_subject: AccountId,
        request_id: u64,
        timestamp: u64,
    }

    #[ink(event)]
    pub struct RetentionPolicyUpdated {
        purpose: ProcessingPurpose,
        retention_days: u64,
        timestamp: u64,
    }

    // ── Storage ─────────────────────────────────────────────────────────────

    #[ink(storage)]
    pub struct GdprConsent {
        admin: AccountId,
        consent_records: Mapping<u64, ConsentRecord>,
        subject_consents: Mapping<AccountId, Vec<u64>>,
        retention_policies: Mapping<u32, DataRetentionPolicy>,
        data_access_requests: Mapping<u64, DataAccessRequest>,
        subject_requests: Mapping<AccountId, Vec<u64>>,
        next_consent_id: u64,
        next_request_id: u64,
    }

    impl GdprConsent {
        #[ink(constructor)]
        pub fn new() -> Self {
            let caller = Self::env().caller();
            Self {
                admin: caller,
                consent_records: Mapping::default(),
                subject_consents: Mapping::default(),
                retention_policies: Mapping::default(),
                data_access_requests: Mapping::default(),
                subject_requests: Mapping::default(),
                next_consent_id: 1,
                next_request_id: 1,
            }
        }

        fn ensure_admin(&self) -> Result<()> {
            if self.env().caller() != self.admin {
                return Err(Error::NotAuthorized);
            }
            Ok(())
        }

        fn now(&self) -> u64 {
            self.env().block_timestamp()
        }

        fn purpose_key(purpose: &ProcessingPurpose) -> u32 {
            match purpose {
                ProcessingPurpose::KYC => 1,
                ProcessingPurpose::TaxReporting => 2,
                ProcessingPurpose::RiskAssessment => 3,
                ProcessingPurpose::PropertyValuation => 4,
                ProcessingPurpose::TransactionMonitoring => 5,
                ProcessingPurpose::Marketing => 6,
                ProcessingPurpose::DataAnalytics => 7,
                ProcessingPurpose::Other(_) => 99,
            }
        }

        // ── Consent Management ──────────────────────────────────────────────

        /// Grants consent for a data subject and processing purpose.
        ///
        /// Only the data subject themself may grant consent for their own
        /// data; the contract admin may additionally record consent on behalf
        /// of a subject (mirroring [`withdraw_consent`]'s authorization rule).
        /// Any other caller is rejected with [`Error::NotAuthorized`] so
        /// third parties cannot fabricate consent records in someone else's
        /// name.
        ///
        /// The record expires after `duration_ms` milliseconds from the
        /// current block timestamp and its id is appended to the subject's
        /// consent list (see [`Self::get_subject_consents`]).
        ///
        /// # Errors
        /// - [`Error::NotAuthorized`] when the caller is neither the subject
        ///   nor the admin.
        /// - [`Error::InvalidDuration`] when `duration_ms` is zero.
        #[ink(message)]
        pub fn grant_consent(
            &mut self,
            data_subject: AccountId,
            purpose: ProcessingPurpose,
            duration_ms: u64,
        ) -> Result<u64> {
            let caller = self.env().caller();
            if caller != data_subject && caller != self.admin {
                return Err(Error::NotAuthorized);
            }

            if duration_ms == 0 {
                return Err(Error::InvalidDuration);
            }

            let consent_id = self.next_consent_id;
            self.next_consent_id = consent_id.checked_add(1).ok_or(Error::InvalidDuration)?;

            let now = self.now();
            let record = ConsentRecord {
                consent_id,
                data_subject,
                processor: self.admin,
                purpose: purpose.clone(),
                status: ConsentStatus::Granted,
                granted_at: now,
                expires_at: now.checked_add(duration_ms).ok_or(Error::InvalidDuration)?,
                withdrawn_at: None,
            };
            self.consent_records.insert(consent_id, &record);

            let mut consents = self.subject_consents.get(data_subject).unwrap_or_default();
            consents.push(consent_id);
            self.subject_consents.insert(data_subject, &consents);

            self.env().emit_event(ConsentGranted {
                data_subject,
                consent_id,
                purpose,
                timestamp: now,
            });
            Ok(consent_id)
        }

        /// Revokes a previously granted consent.
        ///
        /// Callable by the data subject themself or by the contract admin
        /// (e.g. regulator-triggered erasure). Withdrawal takes effect
        /// immediately: [`Self::check_consent`] returns `false` for the
        /// record afterwards, and the withdrawal timestamp is recorded for
        /// audit purposes. A consent can only be withdrawn once.
        ///
        /// # Errors
        /// - [`Error::ConsentNotFound`] when `consent_id` is unknown or the
        ///   record is no longer in the `Granted` state.
        /// - [`Error::NotAuthorized`] when the caller is neither the subject
        ///   nor the admin.
        #[ink(message)]
        pub fn withdraw_consent(&mut self, consent_id: u64) -> Result<()> {
            let caller = self.env().caller();
            let mut record = self
                .consent_records
                .get(consent_id)
                .ok_or(Error::ConsentNotFound)?;

            if record.data_subject != caller && caller != self.admin {
                return Err(Error::NotAuthorized);
            }
            if record.status != ConsentStatus::Granted {
                return Err(Error::ConsentNotFound);
            }

            let now = self.now();
            record.status = ConsentStatus::Withdrawn;
            record.withdrawn_at = Some(now);
            self.consent_records.insert(consent_id, &record);

            self.env().emit_event(ConsentWithdrawn {
                data_subject: record.data_subject,
                consent_id,
                purpose: record.purpose.clone(),
                timestamp: now,
            });
            Ok(())
        }

        /// Returns the full consent record for `consent_id`, or `None` if
        /// unknown.
        ///
        /// The record exposes subject, processor, purpose, status and both
        /// grant/expiry timestamps so integrators can render audit trails.
        /// Note that a record with status [`ConsentStatus::Granted`] may
        /// still be past its `expires_at`; use [`Self::check_consent`] for
        /// the effective validity answer.
        #[ink(message)]
        pub fn get_consent(&self, consent_id: u64) -> Option<ConsentRecord> {
            self.consent_records.get(consent_id)
        }

        /// Lists every consent record ever granted for `data_subject`,
        /// regardless of current status (granted, withdrawn, expired).
        ///
        /// Read-only; order follows grant order. Returns an empty vector for
        /// unknown subjects.
        #[ink(message)]
        pub fn get_subject_consents(&self, data_subject: AccountId) -> Vec<ConsentRecord> {
            match self.subject_consents.get(data_subject) {
                Some(ids) => {
                    let mut records = Vec::new();
                    for id in ids {
                        if let Some(r) = self.consent_records.get(id) {
                            records.push(r);
                        }
                    }
                    records
                }
                None => Vec::new(),
            }
        }

        /// Effective validity check: `true` only when the subject holds a
        /// consent for `purpose` that is currently `Granted` **and** not yet
        /// past its expiry timestamp.
        ///
        /// This is the predicate processors must consult before any
        /// purpose-scoped data operation. Withdrawn and expired consents
        /// both yield `false`.
        #[ink(message)]
        pub fn check_consent(&self, data_subject: AccountId, purpose: ProcessingPurpose) -> bool {
            match self.subject_consents.get(data_subject) {
                Some(ids) => {
                    for id in ids {
                        if let Some(record) = self.consent_records.get(id) {
                            if record.purpose == purpose
                                && record.status == ConsentStatus::Granted
                                && record.expires_at > self.now()
                            {
                                return true;
                            }
                        }
                    }
                    false
                }
                None => false,
            }
        }

        // ── Expiry Management (admin) ───────────────────────────────────────

        /// Admin-only lifecycle transition: marks an already-stale consent
        /// as `Expired`.
        ///
        /// The record must be in the `Granted` state **and** its expiry must
        /// have passed (`expires_at <= now`); consents inside their validity
        /// window cannot be force-expired. Expiring twice is rejected.
        ///
        /// # Errors
        /// - [`Error::NotAuthorized`] when the caller is not the admin.
        /// - [`Error::ConsentNotFound`] when the id is unknown, the consent
        ///   is not yet stale, or it is no longer `Granted`.
        #[ink(message)]
        pub fn expire_consent(&mut self, consent_id: u64) -> Result<()> {
            self.ensure_admin()?;
            let mut record = self
                .consent_records
                .get(consent_id)
                .ok_or(Error::ConsentNotFound)?;
            let now = self.now();
            if record.status != ConsentStatus::Granted || record.expires_at > now {
                return Err(Error::ConsentNotFound);
            }
            record.status = ConsentStatus::Expired;
            self.consent_records.insert(consent_id, &record);
            self.env().emit_event(ConsentExpired {
                data_subject: record.data_subject,
                consent_id,
                timestamp: now,
            });
            Ok(())
        }

        // ── Retention Policies ──────────────────────────────────────────────

        /// Admin-only: stores the retention policy for a processing purpose.
        ///
        /// `retention_days` bounds how long data collected under `purpose`
        /// may be kept, and `auto_delete` declares whether expiry triggers
        /// automatic erasure. Re-setting the same purpose overwrites the
        /// previous policy (the update is emitted as
        /// `RetentionPolicyUpdated`).
        ///
        /// # Errors
        /// - [`Error::NotAuthorized`] when the caller is not the admin.
        #[ink(message)]
        pub fn set_retention_policy(
            &mut self,
            purpose: ProcessingPurpose,
            retention_days: u64,
            auto_delete: bool,
        ) -> Result<()> {
            self.ensure_admin()?;
            let key = Self::purpose_key(&purpose);
            let policy = DataRetentionPolicy {
                purpose: purpose.clone(),
                retention_days,
                auto_delete,
            };
            self.retention_policies.insert(key, &policy);
            self.env().emit_event(RetentionPolicyUpdated {
                purpose,
                retention_days,
                timestamp: self.now(),
            });
            Ok(())
        }

        /// Returns the retention policy configured for `purpose`, or `None`
        /// when no policy has been set.
        ///
        /// Integrators should treat `None` as "no data may be retained
        /// beyond the strict operational minimum" until a policy is
        /// published.
        #[ink(message)]
        pub fn get_retention_policy(
            &self,
            purpose: ProcessingPurpose,
        ) -> Option<DataRetentionPolicy> {
            self.retention_policies.get(Self::purpose_key(&purpose))
        }

        // ── Data Access Requests ────────────────────────────────────────────

        /// Data-subject access request (GDPR art. 15): the caller files a
        /// request for disclosure of all personal data held about them.
        ///
        /// The caller is always the data subject; no admin involvement is
        /// needed to file. The returned id can be tracked via
        /// [`Self::get_data_access_request`] until the admin fulfils it.
        #[ink(message)]
        pub fn request_data_access(&mut self) -> Result<u64> {
            let caller = self.env().caller();
            let request_id = self.next_request_id;
            self.next_request_id = request_id.checked_add(1).ok_or(Error::InvalidDuration)?;

            let request = DataAccessRequest {
                request_id,
                data_subject: caller,
                requested_at: self.now(),
                fulfilled: false,
                fulfilled_at: None,
            };
            self.data_access_requests.insert(request_id, &request);

            let mut requests = self.subject_requests.get(caller).unwrap_or_default();
            requests.push(request_id);
            self.subject_requests.insert(caller, &requests);

            self.env().emit_event(DataAccessRequested {
                data_subject: caller,
                request_id,
                timestamp: self.now(),
            });
            Ok(request_id)
        }

        /// Admin-only: marks a data-access request as fulfilled and stamps
        /// the fulfilment timestamp.
        ///
        /// # Errors
        /// - [`Error::NotAuthorized`] when the caller is not the admin.
        /// - [`Error::DataRequestNotFound`] when `request_id` is unknown.
        #[ink(message)]
        pub fn fulfill_data_access(&mut self, request_id: u64) -> Result<()> {
            self.ensure_admin()?;
            let mut request = self
                .data_access_requests
                .get(request_id)
                .ok_or(Error::DataRequestNotFound)?;
            request.fulfilled = true;
            request.fulfilled_at = Some(self.now());
            self.data_access_requests.insert(request_id, &request);
            self.env().emit_event(DataAccessFulfilled {
                data_subject: request.data_subject,
                request_id,
                timestamp: self.now(),
            });
            Ok(())
        }

        /// Returns the data-access request for `request_id`, or `None` if
        /// unknown. Includes the fulfilment flag and timestamp.
        #[ink(message)]
        pub fn get_data_access_request(&self, request_id: u64) -> Option<DataAccessRequest> {
            self.data_access_requests.get(request_id)
        }

        /// Lists every access request filed by `data_subject`, in filing
        /// order. Empty vector for subjects that never filed.
        #[ink(message)]
        pub fn get_subject_requests(&self, data_subject: AccountId) -> Vec<DataAccessRequest> {
            match self.subject_requests.get(data_subject) {
                Some(ids) => {
                    let mut requests = Vec::new();
                    for id in ids {
                        if let Some(r) = self.data_access_requests.get(id) {
                            requests.push(r);
                        }
                    }
                    requests
                }
                None => Vec::new(),
            }
        }

        /// Returns the contract admin (the account empowered to expire
        /// consents, manage retention policies and fulfil access requests).
        #[ink(message)]
        pub fn admin(&self) -> AccountId {
            self.admin
        }
    }

    impl Default for GdprConsent {
        fn default() -> Self {
            Self::new()
        }
    }

    // ── Tests ──────────────────────────────────────────────────────────────

    #[cfg(test)]
    mod tests {
        use super::*;

        fn default_contract() -> GdprConsent {
            GdprConsent::new()
        }

        #[ink::test]
        fn test_admin_is_caller() {
            let contract = default_contract();
            assert_eq!(contract.admin(), AccountId::from([0x01; 32]));
        }

        #[ink::test]
        fn test_grant_consent() {
            let mut contract = default_contract();
            let subject = AccountId::from([0x02; 32]);
            let id = contract
                .grant_consent(subject, ProcessingPurpose::KYC, 365 * 24 * 60 * 60 * 1000)
                .expect("grant consent");
            assert_eq!(id, 1);
            let record = contract.get_consent(id).expect("should exist");
            assert_eq!(record.data_subject, subject);
            assert_eq!(record.status, ConsentStatus::Granted);
        }

        #[ink::test]
        fn test_withdraw_consent() {
            let mut contract = default_contract();
            let subject = AccountId::from([0x02; 32]);
            let id = contract
                .grant_consent(subject, ProcessingPurpose::KYC, 365 * 24 * 60 * 60 * 1000)
                .expect("grant");
            contract.withdraw_consent(id).expect("withdraw");
            let record = contract.get_consent(id).expect("should exist");
            assert_eq!(record.status, ConsentStatus::Withdrawn);
        }

        #[ink::test]
        fn test_check_consent_valid() {
            let mut contract = default_contract();
            let subject = AccountId::from([0x02; 32]);
            contract
                .grant_consent(subject, ProcessingPurpose::KYC, 365 * 24 * 60 * 60 * 1000)
                .expect("grant");
            assert!(contract.check_consent(subject, ProcessingPurpose::KYC));
        }

        #[ink::test]
        fn test_check_consent_withdrawn() {
            let mut contract = default_contract();
            let subject = AccountId::from([0x02; 32]);
            let id = contract
                .grant_consent(subject, ProcessingPurpose::KYC, 365 * 24 * 60 * 60 * 1000)
                .expect("grant");
            contract.withdraw_consent(id).expect("withdraw");
            assert!(!contract.check_consent(subject, ProcessingPurpose::KYC));
        }

        #[ink::test]
        fn test_get_subject_consents() {
            let mut contract = default_contract();
            let subject = AccountId::from([0x02; 32]);
            contract
                .grant_consent(subject, ProcessingPurpose::KYC, 1000)
                .expect("grant");
            contract
                .grant_consent(subject, ProcessingPurpose::TaxReporting, 1000)
                .expect("grant");
            let records = contract.get_subject_consents(subject);
            assert_eq!(records.len(), 2);
        }

        #[ink::test]
        fn test_retention_policy() {
            let mut contract = default_contract();
            contract
                .set_retention_policy(ProcessingPurpose::KYC, 365, true)
                .expect("set policy");
            let policy = contract
                .get_retention_policy(ProcessingPurpose::KYC)
                .expect("should exist");
            assert_eq!(policy.retention_days, 365);
            assert!(policy.auto_delete);
        }

        #[ink::test]
        fn test_data_access_request() {
            let mut contract = default_contract();
            let id = contract.request_data_access().expect("request");
            assert_eq!(id, 1);
            let request = contract.get_data_access_request(id).expect("should exist");
            assert!(!request.fulfilled);
        }

        #[ink::test]
        fn test_fulfill_data_access() {
            let mut contract = default_contract();
            let id = contract.request_data_access().expect("request");
            contract.fulfill_data_access(id).expect("fulfill");
            let request = contract.get_data_access_request(id).expect("should exist");
            assert!(request.fulfilled);
        }

        #[ink::test]
        fn test_invalid_duration_rejected() {
            let mut contract = default_contract();
            let result =
                contract.grant_consent(AccountId::from([0x02; 32]), ProcessingPurpose::KYC, 0);
            assert_eq!(result, Err(Error::InvalidDuration));
        }

        #[ink::test]
        fn test_unauthorized_caller_cannot_grant_for_other() {
            let mut contract = default_contract();
            let subject = AccountId::from([0x02; 32]);
            let attacker = AccountId::from([0x09; 32]);

            ink::env::test::set_caller::<ink::env::DefaultEnvironment>(attacker);
            let result =
                contract.grant_consent(subject, ProcessingPurpose::KYC, 365 * 24 * 60 * 60 * 1000);
            assert_eq!(result, Err(Error::NotAuthorized));

            // No fabricated consent record exists and processing checks stay false.
            assert!(contract.get_subject_consents(subject).is_empty());
            assert!(!contract.check_consent(subject, ProcessingPurpose::KYC));
        }

        #[ink::test]
        fn test_subject_can_grant_own_consent() {
            let mut contract = default_contract();
            let subject = AccountId::from([0x02; 32]);

            ink::env::test::set_caller::<ink::env::DefaultEnvironment>(subject);
            let id = contract
                .grant_consent(subject, ProcessingPurpose::KYC, 365 * 24 * 60 * 60 * 1000)
                .expect("self-grant");
            let record = contract.get_consent(id).expect("should exist");
            assert_eq!(record.data_subject, subject);
            assert_eq!(record.status, ConsentStatus::Granted);
            assert!(contract.check_consent(subject, ProcessingPurpose::KYC));
        }

        #[ink::test]
        fn test_admin_can_grant_on_behalf_of_subject() {
            let mut contract = default_contract(); // constructor caller is the admin
            let subject = AccountId::from([0x02; 32]);

            let id = contract
                .grant_consent(
                    subject,
                    ProcessingPurpose::TaxReporting,
                    365 * 24 * 60 * 60 * 1000,
                )
                .expect("admin grant");
            let record = contract.get_consent(id).expect("should exist");
            assert_eq!(record.data_subject, subject);
            assert!(contract.check_consent(subject, ProcessingPurpose::TaxReporting));
        }

        #[ink::test]
        fn test_subject_requests_list() {
            let mut contract = default_contract();
            let subject = AccountId::from([0x01; 32]);
            contract.request_data_access().expect("request");
            contract.request_data_access().expect("request");
            let requests = contract.get_subject_requests(subject);
            assert_eq!(requests.len(), 2);
        }

        // ── Consent lifecycle: expiration & retention gating (#974) ────────

        #[ink::test]
        fn test_expire_consent_requires_admin() {
            let mut contract = default_contract();
            let subject = AccountId::from([0x02; 32]);
            let id = contract
                .grant_consent(subject, ProcessingPurpose::KYC, 1_000)
                .expect("grant");
            ink::env::test::set_block_timestamp::<ink::env::DefaultEnvironment>(2_000);
            ink::env::test::set_caller::<ink::env::DefaultEnvironment>(subject);
            assert_eq!(contract.expire_consent(id), Err(Error::NotAuthorized));
        }

        #[ink::test]
        fn test_expire_consent_rejected_before_expiry() {
            let mut contract = default_contract();
            let id = contract
                .grant_consent(AccountId::from([0x02; 32]), ProcessingPurpose::KYC, 10_000)
                .expect("grant");
            // Still inside its validity window, so expiry must be rejected.
            assert_eq!(contract.expire_consent(id), Err(Error::ConsentNotFound));
            let record = contract.get_consent(id).expect("should exist");
            assert_eq!(record.status, ConsentStatus::Granted);
        }

        #[ink::test]
        fn test_expire_consent_after_expiry_marks_expired() {
            let mut contract = default_contract();
            let subject = AccountId::from([0x02; 32]);
            let id = contract
                .grant_consent(subject, ProcessingPurpose::KYC, 1_000)
                .expect("grant");
            assert!(contract.check_consent(subject, ProcessingPurpose::KYC));

            ink::env::test::set_block_timestamp::<ink::env::DefaultEnvironment>(2_000);
            // The consent is naturally stale for consumers...
            assert!(!contract.check_consent(subject, ProcessingPurpose::KYC));
            // ...and the admin can transition it to Expired.
            contract.expire_consent(id).expect("expire after expiry");
            let record = contract.get_consent(id).expect("should exist");
            assert_eq!(record.status, ConsentStatus::Expired);

            // Expiring again is rejected once it is no longer Granted.
            assert_eq!(contract.expire_consent(id), Err(Error::ConsentNotFound));
        }

        #[ink::test]
        fn test_retention_policy_requires_admin() {
            let mut contract = default_contract();
            ink::env::test::set_caller::<ink::env::DefaultEnvironment>(AccountId::from([0x03; 32]));
            assert_eq!(
                contract.set_retention_policy(ProcessingPurpose::Marketing, 30, false),
                Err(Error::NotAuthorized)
            );
            assert!(contract
                .get_retention_policy(ProcessingPurpose::Marketing)
                .is_none());
        }

        #[ink::test]
        fn test_retention_policy_round_trip_and_overwrite() {
            let mut contract = default_contract();
            contract
                .set_retention_policy(ProcessingPurpose::TransactionMonitoring, 90, false)
                .expect("set policy");
            let policy = contract
                .get_retention_policy(ProcessingPurpose::TransactionMonitoring)
                .expect("should exist");
            assert_eq!(policy.retention_days, 90);
            assert!(!policy.auto_delete);

            // Re-setting the same purpose replaces the stored policy.
            contract
                .set_retention_policy(ProcessingPurpose::TransactionMonitoring, 180, true)
                .expect("overwrite policy");
            let policy = contract
                .get_retention_policy(ProcessingPurpose::TransactionMonitoring)
                .expect("should exist");
            assert_eq!(policy.retention_days, 180);
            assert!(policy.auto_delete);
        }
    }
}
