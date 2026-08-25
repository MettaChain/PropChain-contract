/// # Integration Tests: Compliance Registry (Issue #1014)
///
/// These tests verify the end-to-end compliance pipeline:
///   verifier onboarding -> KYC submission -> AML/sanctions screening -> consent
///   -> compliance gating -> revocation
///
/// Because ink! unit tests run inside a single contract environment, we test
/// the contract directly rather than through cross-contract calls. This
/// mirrors the actual interaction semantics.
///
/// Acceptance criteria tested:
///   check Admin adds a dedicated verifier account
///   check Verifier submits verification with document/biometric/risk data
///   check AML + sanctions screening and GDPR consent unlock full compliance
///   check require_compliance succeeds for fully compliant accounts
///   check Revoked accounts fail require_compliance with Error::NotVerified
///   check Non-verifiers cannot submit or revoke verifications
///   check get_compliance_data reflects all stored fields
#[cfg(test)]
#[allow(clippy::module_inception)]
mod integration_compliance {
    // Compliance registry contract
    use compliance_registry::compliance_registry::{
        AMLRiskFactors, BiometricMethod, ComplianceRegistry, ConsentStatus, DocumentType, Error,
        Jurisdiction, RiskLevel, SanctionsList, VerificationStatus,
    };
    use ink::env::{test, DefaultEnvironment};

    fn setup_contract() -> ComplianceRegistry {
        let accounts = test::default_accounts::<DefaultEnvironment>();
        test::set_caller::<DefaultEnvironment>(accounts.alice);
        ComplianceRegistry::new()
    }

    /// Admin adds `verifier` as an authorized verifier.
    fn add_verifier(contract: &mut ComplianceRegistry, verifier: ink::primitives::AccountId) {
        let accounts = test::default_accounts::<DefaultEnvironment>();
        test::set_caller::<DefaultEnvironment>(accounts.alice);
        contract
            .add_verifier(verifier)
            .expect("Owner should add a verifier");
    }

    /// Clean AML risk factors (no PEP, no risk flags).
    fn clean_aml_factors() -> AMLRiskFactors {
        AMLRiskFactors {
            pep_status: false,
            high_risk_country: false,
            suspicious_transaction_pattern: false,
            large_transaction_volume: false,
            source_of_funds_verified: true,
        }
    }

    /// Runs the full screening flow against `user` performed by `verifier`.
    fn fully_screen_user(contract: &mut ComplianceRegistry, user: ink::primitives::AccountId) {
        let accounts = test::default_accounts::<DefaultEnvironment>();
        add_verifier(contract, accounts.bob);

        // Verifier submits the base KYC verification
        test::set_caller::<DefaultEnvironment>(accounts.bob);
        contract
            .submit_verification(
                user,
                Jurisdiction::US,
                [7u8; 32],
                RiskLevel::Low,
                DocumentType::Passport,
                BiometricMethod::FaceRecognition,
                10,
            )
            .expect("Verifier should submit verification");

        // AML and sanctions screening pass
        contract
            .update_aml_status(user, true, clean_aml_factors())
            .expect("Verifier should record a passing AML check");
        contract
            .update_sanctions_status(user, true, SanctionsList::OFAC)
            .expect("Verifier should record a passing sanctions check");

        // The user grants GDPR consent themselves
        test::set_caller::<DefaultEnvironment>(user);
        contract
            .update_consent(user, ConsentStatus::Given)
            .expect("User should be able to grant consent");
    }

    /// Scenario 1 - Full lifecycle:
    /// verifier onboarding -> submission -> screening -> compliance -> revocation
    #[ink::test]
    fn test_verifier_flow_unlocks_compliance_until_revocation() {
        let accounts = test::default_accounts::<DefaultEnvironment>();
        let mut contract = setup_contract();
        let user = accounts.charlie;

        // Before any submission the user is not compliant
        assert!(
            !contract.is_compliant(user),
            "Unverified account must not be compliant"
        );

        fully_screen_user(&mut contract, user);

        assert!(
            contract.is_compliant(user),
            "Fully screened user should be compliant"
        );
        assert_eq!(
            contract.require_compliance(user),
            Ok(()),
            "require_compliance must succeed for compliant accounts"
        );

        // Verifier revokes the verification
        test::set_caller::<DefaultEnvironment>(accounts.bob);
        contract
            .revoke_verification(user)
            .expect("Verifier should revoke verification");

        assert!(
            !contract.is_compliant(user),
            "Revoked account must not be compliant"
        );
        assert_eq!(
            contract.require_compliance(user),
            Err(Error::NotVerified),
            "Revoked account must fail require_compliance with NotVerified"
        );
    }

    /// Scenario 2 - Screening alone is not enough; every gate matters.
    /// Right after submission (before AML/sanctions/consent) compliance stays closed.
    #[ink::test]
    fn test_submission_without_screening_does_not_unlock_compliance() {
        let accounts = test::default_accounts::<DefaultEnvironment>();
        let mut contract = setup_contract();
        let user = accounts.charlie;
        add_verifier(&mut contract, accounts.bob);

        test::set_caller::<DefaultEnvironment>(accounts.bob);
        contract
            .submit_verification(
                user,
                Jurisdiction::US,
                [7u8; 32],
                RiskLevel::Low,
                DocumentType::Passport,
                BiometricMethod::FaceRecognition,
                10,
            )
            .expect("Verifier should submit verification");

        assert!(
            !contract.is_compliant(user),
            "KYC without AML/sanctions/consent must not be compliant"
        );
        assert_eq!(
            contract.require_compliance(user),
            Err(Error::NotVerified),
            "Partially screened user must fail require_compliance"
        );
    }

    /// Scenario 3 - Non-verifiers cannot submit or revoke verifications,
    /// and non-owners cannot onboard verifiers.
    #[ink::test]
    fn test_non_verifier_submissions_are_rejected() {
        let accounts = test::default_accounts::<DefaultEnvironment>();
        let mut contract = setup_contract();

        // Non-owner cannot add verifiers
        test::set_caller::<DefaultEnvironment>(accounts.bob);
        let forbidden_add = contract.add_verifier(accounts.bob);
        assert_eq!(
            forbidden_add,
            Err(Error::NotAuthorized),
            "Only the owner may add verifiers"
        );

        // Non-verifier cannot submit verification
        test::set_caller::<DefaultEnvironment>(accounts.django);
        let rejected = contract.submit_verification(
            accounts.charlie,
            Jurisdiction::US,
            [7u8; 32],
            RiskLevel::Low,
            DocumentType::Passport,
            BiometricMethod::FaceRecognition,
            10,
        );
        assert_eq!(
            rejected,
            Err(Error::NotAuthorized),
            "Non-verifiers must not submit verifications"
        );

        // Non-verifier cannot revoke either
        let rejected_revoke = contract.revoke_verification(accounts.charlie);
        assert_eq!(
            rejected_revoke,
            Err(Error::NotAuthorized),
            "Non-verifiers must not revoke verifications"
        );
    }

    /// Scenario 4 - get_compliance_data mirrors everything that was stored.
    #[ink::test]
    fn test_get_compliance_data_reflects_stored_fields() {
        let accounts = test::default_accounts::<DefaultEnvironment>();
        let mut contract = setup_contract();
        let user = accounts.charlie;

        assert!(
            contract.get_compliance_data(user).is_none(),
            "No compliance data before first submission"
        );

        fully_screen_user(&mut contract, user);

        let data = contract
            .get_compliance_data(user)
            .expect("Compliance data should exist after submission");
        assert_eq!(data.status, VerificationStatus::Verified);
        assert_eq!(data.jurisdiction, Jurisdiction::US);
        assert_eq!(data.risk_level, RiskLevel::Low);
        assert_eq!(data.kyc_hash, [7u8; 32]);
        assert_eq!(data.document_type, DocumentType::Passport);
        assert_eq!(data.biometric_method, BiometricMethod::FaceRecognition);
        assert_eq!(data.risk_score, 10);
        assert!(data.aml_checked);
        assert!(data.sanctions_checked);
        assert_eq!(data.gdpr_consent, ConsentStatus::Given);

        // Revocation flips the stored status while keeping the rest intact
        test::set_caller::<DefaultEnvironment>(accounts.bob);
        contract
            .revoke_verification(user)
            .expect("Verifier should revoke verification");

        let revoked = contract
            .get_compliance_data(user)
            .expect("Revoked record remains queryable");
        assert_eq!(revoked.status, VerificationStatus::Rejected);
    }
}
