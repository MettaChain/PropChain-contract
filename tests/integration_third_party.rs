/// # Integration Tests: Third-Party Service Registry (Issue #1013)
///
/// These tests verify the end-to-end third-party orchestration pipeline:
///   service registration -> KYC request -> provider verification -> on-chain record
///
/// Because ink! unit tests run inside a single contract environment, we test
/// the contract directly rather than through cross-contract calls. This
/// mirrors the actual interaction semantics.
///
/// Acceptance criteria tested:
///   check Admin registers a KYC service with fee parameters
///   check User initiates a KYC request through an active service
///   check Provider approves the request and a KYC record is stored
///   check is_kyc_verified reflects the verification level
///   check Unregistered services are rejected for KYC operations
///   check Suspended services block KYC operations until reactivated
///   check get_service_config / get_kyc_record / get_payment_request state assertions
///   check Fiat payment initiate -> complete happy path
///   check Non-providers cannot update KYC status or complete payments
#[cfg(test)]
#[allow(clippy::module_inception)]
mod integration_third_party {
    // Third-party contract
    use ink::env::{test, DefaultEnvironment};
    use propchain_third_party::propchain_third_party::{
        Error as ThirdPartyError, KycRecord, PaymentRequest, RequestStatus, ServiceConfig,
        ServiceStatus, ServiceType, ThirdPartyIntegration,
    };

    fn setup_contract() -> ThirdPartyIntegration {
        let accounts = test::default_accounts::<DefaultEnvironment>();
        test::set_caller::<DefaultEnvironment>(accounts.alice);
        ThirdPartyIntegration::new()
    }

    /// Registers a KYC-provider service operated by `bob` and returns its id.
    fn register_kyc_service(contract: &mut ThirdPartyIntegration) -> u32 {
        let accounts = test::default_accounts::<DefaultEnvironment>();
        test::set_caller::<DefaultEnvironment>(accounts.alice);
        contract
            .register_service(
                ServiceType::KycProvider,
                String::from("PropChain KYC"),
                accounts.bob,
                String::from("https://kyc.propchain.io"),
                String::from("v1"),
                100,
            )
            .expect("Admin should register a KYC service")
    }

    /// Registers a payment-gateway service operated by `bob` and returns its id.
    fn register_payment_service(contract: &mut ThirdPartyIntegration) -> u32 {
        let accounts = test::default_accounts::<DefaultEnvironment>();
        test::set_caller::<DefaultEnvironment>(accounts.alice);
        contract
            .register_service(
                ServiceType::PaymentGateway,
                String::from("Fiat Gateway"),
                accounts.bob,
                String::from("https://pay.propchain.io"),
                String::from("v2"),
                250,
            )
            .expect("Admin should register a payment service")
    }

    /// Scenario 1 - Full KYC happy path
    /// 1. Admin registers the KYC service
    /// 2. User initiates a KYC request through the active service
    /// 3. Provider approves with verification level 3 valid for 365 days
    /// 4. is_kyc_verified honors the level threshold and the record is queryable
    #[ink::test]
    fn test_full_kyc_lifecycle_from_registration_to_verification() {
        let accounts = test::default_accounts::<DefaultEnvironment>();
        let mut contract = setup_contract();

        let service_id = register_kyc_service(&mut contract);
        assert_eq!(service_id, 1, "First registered service should get id 1");

        let user = accounts.charlie;

        // Step 2: user initiates the request for themselves
        test::set_caller::<DefaultEnvironment>(user);
        let request_id = contract
            .initiate_kyc_request(service_id, user, String::from("ref-kyc-001"))
            .expect("Active service should accept KYC requests");
        assert_eq!(request_id, 1, "First KYC request should get id 1");

        // Step 3: provider approves the request
        test::set_caller::<DefaultEnvironment>(accounts.bob);
        contract
            .update_kyc_status(request_id, RequestStatus::Approved, 3, 365)
            .expect("Provider should approve its own KYC request");

        // Step 4: verification checks
        assert!(
            contract.is_kyc_verified(user, 3),
            "User should be verified at level 3"
        );
        assert!(
            !contract.is_kyc_verified(user, 4),
            "User should NOT satisfy a level-4 requirement"
        );

        let record: KycRecord = contract.get_kyc_record(user).expect("KYC record stored");
        assert_eq!(record.user, user);
        assert_eq!(record.provider_id, service_id);
        assert_eq!(record.verification_level, 3);
        assert!(record.is_active);
        assert!(record.expires_at > record.verified_at);

        let config: ServiceConfig = contract
            .get_service_config(service_id)
            .expect("Service config stored");
        assert_eq!(config.service_id, service_id);
        assert_eq!(config.service_type, ServiceType::KycProvider);
        assert_eq!(config.provider_account, accounts.bob);
        assert_eq!(config.status, ServiceStatus::Active);
        assert_eq!(config.fee_percentage, 100);
        assert_eq!(config.fees_collected, 0);
        assert_eq!(config.endpoint_url, "https://kyc.propchain.io");
    }

    /// Scenario 2 - Unregistered / wrong-type services are rejected
    #[ink::test]
    fn test_unregistered_and_wrong_type_services_are_rejected() {
        let accounts = test::default_accounts::<DefaultEnvironment>();
        let mut contract = setup_contract();

        // Unknown service id
        test::set_caller::<DefaultEnvironment>(accounts.charlie);
        let unknown =
            contract.initiate_kyc_request(999, accounts.charlie, String::from("ref-unknown"));
        assert_eq!(
            unknown,
            Err(ThirdPartyError::ServiceNotFound),
            "Operations against unregistered services must fail"
        );

        // A payment gateway cannot process KYC requests
        let gateway_id = register_payment_service(&mut contract);
        test::set_caller::<DefaultEnvironment>(accounts.charlie);
        let wrong_type = contract.initiate_kyc_request(
            gateway_id,
            accounts.charlie,
            String::from("ref-wrong-type"),
        );
        assert_eq!(
            wrong_type,
            Err(ThirdPartyError::ServiceNotFound),
            "KYC requests through non-KYC services must be rejected"
        );
    }

    /// Scenario 3 - Suspended service blocks KYC ops; reactivation restores them
    #[ink::test]
    fn test_suspended_service_blocks_kyc_until_reactivated() {
        let accounts = test::default_accounts::<DefaultEnvironment>();
        let mut contract = setup_contract();
        let service_id = register_kyc_service(&mut contract);

        // Admin suspends the service
        test::set_caller::<DefaultEnvironment>(accounts.alice);
        contract
            .update_service_status(service_id, ServiceStatus::Suspended)
            .expect("Admin should suspend a service");

        // KYC initiation now blocked
        test::set_caller::<DefaultEnvironment>(accounts.charlie);
        let blocked = contract.initiate_kyc_request(
            service_id,
            accounts.charlie,
            String::from("ref-blocked"),
        );
        assert_eq!(
            blocked,
            Err(ThirdPartyError::ServiceInactive),
            "Suspended services must block KYC operations"
        );

        // Random account can neither suspend nor resume the service
        test::set_caller::<DefaultEnvironment>(accounts.django);
        let forbidden = contract.update_service_status(service_id, ServiceStatus::Active);
        assert_eq!(
            forbidden,
            Err(ThirdPartyError::Unauthorized),
            "Non-admin/non-provider must not change service status"
        );

        // Provider resumes the service and KYC works again
        test::set_caller::<DefaultEnvironment>(accounts.bob);
        contract
            .update_service_status(service_id, ServiceStatus::Active)
            .expect("Provider should reactivate its own service");

        test::set_caller::<DefaultEnvironment>(accounts.charlie);
        let resumed = contract.initiate_kyc_request(
            service_id,
            accounts.charlie,
            String::from("ref-resumed"),
        );
        assert!(
            resumed.is_ok(),
            "Reactivated service must accept KYC requests"
        );
    }

    /// Scenario 4 - Only the service provider can update KYC status
    #[ink::test]
    fn test_non_provider_cannot_update_kyc_status() {
        let accounts = test::default_accounts::<DefaultEnvironment>();
        let mut contract = setup_contract();
        let service_id = register_kyc_service(&mut contract);

        test::set_caller::<DefaultEnvironment>(accounts.charlie);
        let request_id = contract
            .initiate_kyc_request(service_id, accounts.charlie, String::from("ref-perm"))
            .expect("Request creation should succeed");

        // A random account must not update the request
        test::set_caller::<DefaultEnvironment>(accounts.django);
        let rejected = contract.update_kyc_status(request_id, RequestStatus::Approved, 2, 30);
        assert_eq!(
            rejected,
            Err(ThirdPartyError::Unauthorized),
            "Only the provider may update KYC status"
        );

        // The real provider still can
        test::set_caller::<DefaultEnvironment>(accounts.bob);
        contract
            .update_kyc_status(request_id, RequestStatus::Rejected, 0, 0)
            .expect("Provider should be able to reject its own request");

        assert!(
            !contract.is_kyc_verified(accounts.charlie, 1),
            "A rejected request must not mark the user as verified"
        );
    }

    /// Scenario 5 - Query helpers return None for unknown identifiers
    #[ink::test]
    fn test_state_queries_return_none_for_unknown_ids() {
        let accounts = test::default_accounts::<DefaultEnvironment>();
        let contract = setup_contract();

        assert!(contract.get_service_config(42).is_none());
        assert!(contract.get_kyc_record(accounts.charlie).is_none());
        assert!(contract.get_payment_request(42).is_none());
    }

    /// Scenario 6 - Fiat payment happy path: initiate -> complete -> recorded
    #[ink::test]
    fn test_fiat_payment_initiation_and_completion_round_trip() {
        let accounts = test::default_accounts::<DefaultEnvironment>();
        let mut contract = setup_contract();
        let gateway_id = register_payment_service(&mut contract);

        // Payer initiates the fiat payment
        test::set_caller::<DefaultEnvironment>(accounts.charlie);
        let request_id = contract
            .initiate_fiat_payment(
                gateway_id,
                accounts.django, // target contract
                1,               // operation type
                50_000_000_000,  // fiat amount
                String::from("USD"),
                String::from("INV-2026-042"),
            )
            .expect("Active gateway should accept payments");
        assert_eq!(request_id, 1, "First payment should get id 1");

        let pending: PaymentRequest = contract
            .get_payment_request(request_id)
            .expect("Payment request stored");
        assert_eq!(pending.payer, accounts.charlie);
        assert_eq!(pending.service_id, gateway_id);
        assert_eq!(pending.fiat_amount, 50_000_000_000);
        assert_eq!(pending.fiat_currency, "USD");
        assert_eq!(pending.status, RequestStatus::Pending);
        assert_eq!(pending.equivalent_tokens, 0);
        assert_eq!(pending.complete_time, None);

        // Non-provider cannot complete the payment
        test::set_caller::<DefaultEnvironment>(accounts.charlie);
        let forbidden = contract.complete_payment(request_id, true, 500);
        assert_eq!(
            forbidden,
            Err(ThirdPartyError::Unauthorized),
            "Only the provider may complete payments"
        );

        // Provider completes it successfully
        test::set_caller::<DefaultEnvironment>(accounts.bob);
        contract
            .complete_payment(request_id, true, 500)
            .expect("Provider should complete the payment");

        let completed: PaymentRequest = contract
            .get_payment_request(request_id)
            .expect("Completed payment still queryable");
        assert_eq!(completed.status, RequestStatus::Approved);
        assert_eq!(completed.equivalent_tokens, 500);
        assert!(completed.complete_time.is_some());

        // Completing again is not allowed once finalized
        let replay = contract.complete_payment(request_id, true, 500);
        assert_eq!(
            replay,
            Err(ThirdPartyError::InvalidStatusTransition),
            "Finalized payments must not be completed twice"
        );
    }

    /// Scenario 7 - Fee percentage above 10000 bips is rejected at registration
    #[ink::test]
    fn test_invalid_fee_percentage_rejected_at_registration() {
        let accounts = test::default_accounts::<DefaultEnvironment>();
        let mut contract = setup_contract();

        test::set_caller::<DefaultEnvironment>(accounts.alice);
        let result = contract.register_service(
            ServiceType::KycProvider,
            String::from("Greedy KYC"),
            accounts.bob,
            String::from("https://greedy.example.com"),
            String::from("v1"),
            20_000,
        );
        assert_eq!(
            result,
            Err(ThirdPartyError::InvalidFeePercentage),
            "Fee percentage above 10 000 bips must be rejected"
        );
    }
}
