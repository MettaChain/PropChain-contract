#![cfg_attr(not(feature = "std"), no_std, no_main)]
#![allow(
    clippy::needless_borrows_for_generic_args,
    clippy::too_many_arguments,
    clippy::upper_case_acronyms
)]

#[ink::contract]
mod gdpr_consent {
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
        pub consent_id: u64,
        pub data_subject: AccountId,
        pub processor: AccountId,
        pub purpose: ProcessingPurpose,
        pub status: ConsentStatus,
        pub granted_at: u64,
        pub expires_at: u64,
        pub withdrawn_at: Option<u64>,
    }

    #[derive(Debug, Clone, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(
        feature = "std",
        derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
    )]
    pub struct DataRetentionPolicy {
        pub purpose: ProcessingPurpose,
        pub retention_days: u64,
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

        #[ink(message)]
        pub fn grant_consent(
            &mut self,
            data_subject: AccountId,
            purpose: ProcessingPurpose,
            duration_ms: u64,
        ) -> Result<u64> {
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

        #[ink(message)]
        pub fn get_consent(&self, consent_id: u64) -> Option<ConsentRecord> {
            self.consent_records.get(consent_id)
        }

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

        #[ink(message)]
        pub fn get_retention_policy(
            &self,
            purpose: ProcessingPurpose,
        ) -> Option<DataRetentionPolicy> {
            self.retention_policies.get(Self::purpose_key(&purpose))
        }

        // ── Data Access Requests ────────────────────────────────────────────

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

        #[ink(message)]
        pub fn get_data_access_request(&self, request_id: u64) -> Option<DataAccessRequest> {
            self.data_access_requests.get(request_id)
        }

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
        fn test_subject_requests_list() {
            let mut contract = default_contract();
            let subject = AccountId::from([0x01; 32]);
            contract.request_data_access().expect("request");
            contract.request_data_access().expect("request");
            let requests = contract.get_subject_requests(subject);
            assert_eq!(requests.len(), 2);
        }
    }
}
