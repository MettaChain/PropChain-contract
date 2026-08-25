/// # Integration Tests: GDPR Consent Management (Issue #1005)
///
/// These tests verify the end-to-end consent lifecycle of the
/// `propchain-gdpr` contract:
///   grant -> check -> withdraw -> re-check
/// plus the data access request flow and input validation guards.
///
/// Acceptance criteria tested:
///   check Admin grants consent for a data subject with a positive duration
///   check check_consent reports true while a consent is granted and unexpired
///   check Data subject can withdraw their own consent; check_consent flips to false
///   check Admin can withdraw on behalf of a subject; third parties are rejected
///   check Data subject requests data access; admin fulfills it
///   check Non-admin cannot fulfill a data access request
///   check get_data_access_request reflects fulfilled status afterwards
///   check Zero-duration consent grants are rejected with InvalidDuration
#[cfg(test)]
#[allow(clippy::module_inception)]
mod integration_gdpr {
    // GDPR consent contract
    use ink::env::{test, DefaultEnvironment};
    use propchain_gdpr::gdpr_consent::{
        ConsentStatus, Error as GdprError, GdprConsent, ProcessingPurpose,
    };

    /// Duration used for "valid" grants (1 year in milliseconds).
    const ONE_YEAR_MS: u64 = 365 * 24 * 60 * 60 * 1000;

    /// Deploy the contract with `accounts.alice` as admin.
    fn setup() -> GdprConsent {
        let accounts = test::default_accounts::<DefaultEnvironment>();
        test::set_caller::<DefaultEnvironment>(accounts.alice);
        GdprConsent::new()
    }

    /// Scenario 1 - Full consent lifecycle initiated by the admin
    /// 1. Admin grants KYC consent for bob
    /// 2. check_consent(bob, KYC) is true
    /// 3. Bob withdraws his own consent
    /// 4. check_consent(bob, KYC) is false again
    #[ink::test]
    fn test_grant_check_withdraw_consent_lifecycle() {
        let accounts = test::default_accounts::<DefaultEnvironment>();
        let mut gdpr = setup();

        let consent_id = gdpr
            .grant_consent(accounts.bob, ProcessingPurpose::KYC, ONE_YEAR_MS)
            .expect("Admin should be able to grant consent");

        assert!(
            gdpr.check_consent(accounts.bob, ProcessingPurpose::KYC),
            "Consent must be active right after granting"
        );
        assert!(
            !gdpr.check_consent(accounts.bob, ProcessingPurpose::Marketing),
            "Consent must be purpose-scoped: other purposes stay false"
        );

        // Subject withdraws their own consent
        test::set_caller::<DefaultEnvironment>(accounts.bob);
        gdpr.withdraw_consent(consent_id)
            .expect("Data subject must be able to withdraw own consent");

        assert!(
            !gdpr.check_consent(accounts.bob, ProcessingPurpose::KYC),
            "Withdrawn consent must fail check_consent"
        );

        let record = gdpr.get_consent(consent_id).expect("Record should persist");
        assert_eq!(record.status, ConsentStatus::Withdrawn);
        assert!(
            record.withdrawn_at.is_some(),
            "withdrawn_at must be recorded"
        );
    }

    /// Scenario 2 - Withdrawal authorization matrix:
    /// subject and admin may withdraw, unrelated parties may not.
    #[ink::test]
    fn test_withdraw_authorization_subject_admin_only() {
        let accounts = test::default_accounts::<DefaultEnvironment>();
        let mut gdpr = setup();

        let id_bob = gdpr
            .grant_consent(accounts.bob, ProcessingPurpose::TaxReporting, ONE_YEAR_MS)
            .expect("Grant for bob should succeed");
        let id_charlie = gdpr
            .grant_consent(
                accounts.charlie,
                ProcessingPurpose::RiskAssessment,
                ONE_YEAR_MS,
            )
            .expect("Grant for charlie should succeed");

        // Unrelated party (charlie) cannot withdraw bob's consent
        test::set_caller::<DefaultEnvironment>(accounts.charlie);
        assert_eq!(
            gdpr.withdraw_consent(id_bob),
            Err(GdprError::NotAuthorized),
            "Third party must not withdraw someone else's consent"
        );

        // Subject (bob) CAN withdraw
        test::set_caller::<DefaultEnvironment>(accounts.bob);
        gdpr.withdraw_consent(id_bob)
            .expect("Subject must be able to withdraw own consent");

        // Admin (alice) CAN withdraw on behalf of charlie
        test::set_caller::<DefaultEnvironment>(accounts.alice);
        gdpr.withdraw_consent(id_charlie)
            .expect("Admin must be able to withdraw consent on behalf of a subject");

        assert!(!gdpr.check_consent(accounts.bob, ProcessingPurpose::TaxReporting));
        assert!(!gdpr.check_consent(accounts.charlie, ProcessingPurpose::RiskAssessment));
    }

    /// Scenario 3 - Data access request flow:
    /// subject requests -> non-admin fulfill rejected -> admin fulfills.
    #[ink::test]
    fn test_data_access_request_fulfillment_flow() {
        let accounts = test::default_accounts::<DefaultEnvironment>();
        let mut gdpr = setup();

        // Bob submits a data access request
        test::set_caller::<DefaultEnvironment>(accounts.bob);
        let request_id = gdpr
            .request_data_access()
            .expect("Subject should request access");

        let pending = gdpr
            .get_data_access_request(request_id)
            .expect("Request should exist");
        assert_eq!(pending.data_subject, accounts.bob);
        assert!(!pending.fulfilled, "Request must start unfulfilled");

        // Non-admin fulfillment attempt is rejected
        test::set_caller::<DefaultEnvironment>(accounts.bob);
        assert_eq!(
            gdpr.fulfill_data_access(request_id),
            Err(GdprError::NotAuthorized),
            "Only the admin may fulfill data access requests"
        );
        assert!(
            !gdpr.get_data_access_request(request_id).unwrap().fulfilled,
            "Rejected fulfillment must not mutate state"
        );

        // Admin fulfills the request
        test::set_caller::<DefaultEnvironment>(accounts.alice);
        gdpr.fulfill_data_access(request_id)
            .expect("Admin should fulfill the data access request");

        let fulfilled = gdpr
            .get_data_access_request(request_id)
            .expect("Request should exist");
        assert!(fulfilled.fulfilled, "Request must be marked fulfilled");
        assert!(
            fulfilled.fulfilled_at.is_some(),
            "fulfilled_at must be recorded"
        );

        // The subject sees the fulfilled request in their history
        let history = gdpr.get_subject_requests(accounts.bob);
        assert_eq!(history.len(), 1);
        assert!(history[0].fulfilled);
    }

    /// Scenario 4 - Input validation: zero-duration grants rejected.
    #[ink::test]
    fn test_zero_duration_grant_rejected() {
        let accounts = test::default_accounts::<DefaultEnvironment>();
        let mut gdpr = setup();

        assert_eq!(
            gdpr.grant_consent(accounts.bob, ProcessingPurpose::KYC, 0),
            Err(GdprError::InvalidDuration),
            "Zero-duration consent must be rejected"
        );

        // Nothing was stored for the subject
        assert!(!gdpr.check_consent(accounts.bob, ProcessingPurpose::KYC));
        assert!(gdpr.get_subject_consents(accounts.bob).is_empty());
    }

    /// Scenario 5 - Multiple purposes tracked per subject independently;
    /// withdrawing one purpose leaves others intact.
    #[ink::test]
    fn test_multi_purpose_consents_are_independent() {
        let accounts = test::default_accounts::<DefaultEnvironment>();
        let mut gdpr = setup();

        gdpr.grant_consent(accounts.bob, ProcessingPurpose::KYC, ONE_YEAR_MS)
            .expect("KYC grant should succeed");
        let marketing_id = gdpr
            .grant_consent(accounts.bob, ProcessingPurpose::Marketing, ONE_YEAR_MS)
            .expect("Marketing grant should succeed");

        let records = gdpr.get_subject_consents(accounts.bob);
        assert_eq!(records.len(), 2, "Both consents should be listed");
        assert!(records.iter().all(|r| r.status == ConsentStatus::Granted));

        // Withdrawing marketing keeps KYC active
        test::set_caller::<DefaultEnvironment>(accounts.bob);
        gdpr.withdraw_consent(marketing_id)
            .expect("Withdraw should succeed");

        assert!(
            gdpr.check_consent(accounts.bob, ProcessingPurpose::KYC),
            "KYC consent must remain granted"
        );
        assert!(
            !gdpr.check_consent(accounts.bob, ProcessingPurpose::Marketing),
            "Marketing consent must be withdrawn"
        );
    }

    /// Scenario 6 - Unknown ids surface ConsentNotFound / DataRequestNotFound.
    #[ink::test]
    fn test_unknown_ids_rejected() {
        let accounts = test::default_accounts::<DefaultEnvironment>();
        let mut gdpr = setup();

        assert_eq!(
            gdpr.withdraw_consent(999),
            Err(GdprError::ConsentNotFound),
            "Unknown consent id must be rejected"
        );

        test::set_caller::<DefaultEnvironment>(accounts.alice);
        assert_eq!(
            gdpr.fulfill_data_access(999),
            Err(GdprError::DataRequestNotFound),
            "Unknown request id must be rejected"
        );
    }
}
