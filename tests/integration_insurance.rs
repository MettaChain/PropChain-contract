/// # Integration Tests: Property Insurance Platform (Issue #1001)
///
/// These tests verify the end-to-end insurance pipeline:
///   pool setup -> policy creation (premium payment) -> claim submission ->
///   assessor/admin authorization -> payout
/// plus the oracle-driven claim automation path.
///
/// Because ink! unit tests run inside a single contract environment, we test
/// the contract directly rather than through cross-contract calls. This
/// mirrors the actual interaction semantics and reuses the seeding patterns
/// of the in-crate unit suite (`contracts/insurance/src/tests.rs`).
///
/// Acceptance criteria tested:
///   check Pool setup capitalizes the risk pool
///   check Policy creation requires a valid risk assessment and premium
///   check Claim submission by the policyholder succeeds
///   check process_claim rejects unauthorized callers
///   check Authorized assessor approval executes the payout (minus deductible)
///   check Claim cooldown blocks a second claim on the same property
///   check report_oracle_event rejects unauthorized callers
///   check Authorized oracle reports fire parametric triggers exactly once
#[cfg(test)]
#[allow(clippy::module_inception)]
mod integration_insurance {
    use ink::env::{test, DefaultEnvironment};
    use propchain_insurance::propchain_insurance::{
        ClaimStatus, CoverageType, InsuranceError, PayoutMode, PremiumCalculation,
        PropertyInsurance, TriggerComparator, TriggerMetric,
    };

    const COVERAGE_AMOUNT: u128 = 100_000_000;
    const POOL_LIQUIDITY: u128 = 1_000_000_000_000;
    const CLAIM_AMOUNT: u128 = 40_000_000;
    const DURATION_SECONDS: u64 = 86_400 * 365;

    fn setup() -> PropertyInsurance {
        let accounts = test::default_accounts::<DefaultEnvironment>();
        test::set_caller::<DefaultEnvironment>(accounts.alice);
        // Start at 35 days so `now - last_claim(0) > 30-day cooldown`.
        test::set_block_timestamp::<DefaultEnvironment>(3_000_000);
        PropertyInsurance::new(accounts.alice)
    }

    /// Admin seeds an active Fire pool with liquidity and a valid risk
    /// assessment for property 1 (mirrors the unit-test helpers).
    fn seed_pool_and_property(contract: &mut PropertyInsurance) -> u64 {
        let pool_id = contract
            .create_risk_pool(
                String::from("Integration Fire Pool"),
                CoverageType::Fire,
                8000,
                500_000_000_000u128,
            )
            .expect("Admin should create the risk pool");

        contract
            .update_risk_assessment(1, 75, 80, 85, 90, DURATION_SECONDS)
            .expect("Admin should seed the property risk assessment");

        test::set_value_transferred::<DefaultEnvironment>(POOL_LIQUIDITY);
        contract
            .provide_pool_liquidity(pool_id)
            .expect("Liquidity provision should succeed");

        pool_id
    }

    /// Bob purchases an active policy against property 1 / pool 1.
    fn create_active_policy(contract: &mut PropertyInsurance) -> u64 {
        let accounts = test::default_accounts::<DefaultEnvironment>();

        let calc: PremiumCalculation = contract
            .calculate_premium(1, COVERAGE_AMOUNT, CoverageType::Fire)
            .expect("Premium calculation should succeed with seeded assessment");
        assert!(calc.annual_premium > 0, "Annual premium must be positive");

        test::set_caller::<DefaultEnvironment>(accounts.bob);
        test::set_value_transferred::<DefaultEnvironment>(calc.annual_premium * 2);
        contract
            .create_policy(
                1,
                CoverageType::Fire,
                COVERAGE_AMOUNT,
                1,
                DURATION_SECONDS,
                String::from("ipfs://bafybeipolicy-integration"),
            )
            .expect("Funded policy creation should succeed")
    }

    /// Scenario 1 - Full lifecycle: pool → policy → claim → authorization
    /// rejection → assessor approval → payout + cooldown enforcement
    #[ink::test]
    fn test_policy_claim_authorization_and_payout_lifecycle() {
        let accounts = test::default_accounts::<DefaultEnvironment>();
        let mut contract = setup();

        // Step 1: pool + assessment seeding
        let pool_id = seed_pool_and_property(&mut contract);
        assert_eq!(pool_id, 1);

        // Step 2: policyholder buys coverage
        let policy_id = create_active_policy(&mut contract);

        let policy = contract.get_policy(policy_id).unwrap();
        assert_eq!(policy.policyholder, accounts.bob);
        assert_eq!(policy.coverage_amount, COVERAGE_AMOUNT);
        assert_eq!(policy.pool_id, pool_id);

        let pool = contract.get_pool(pool_id).unwrap();
        assert_eq!(pool.active_policies, 1);
        assert!(
            pool.total_premiums_collected > 0,
            "Premium share must be pooled"
        );

        // Step 3: policyholder submits a claim
        test::set_caller::<DefaultEnvironment>(accounts.bob);
        let claim_id = contract
            .submit_claim(
                policy_id,
                CLAIM_AMOUNT,
                String::from("Fire damage to insured building"),
                String::from("ipfs://bafybeievidence"),
            )
            .expect("Policyholder claim submission should succeed");

        // Step 4: unauthorized caller cannot process the claim
        test::set_caller::<DefaultEnvironment>(accounts.charlie);
        assert_eq!(
            contract.process_claim(
                claim_id,
                true,
                String::from("ipfs://bafybeioreport"),
                String::new(),
            ),
            Err(InsuranceError::Unauthorized),
            "Random callers must not be able to process claims"
        );

        // Step 5: admin authorizes charlie as assessor; approval pays out
        test::set_caller::<DefaultEnvironment>(accounts.alice);
        contract
            .authorize_assessor(accounts.charlie)
            .expect("Admin should authorize the assessor");

        test::set_caller::<DefaultEnvironment>(accounts.charlie);
        contract
            .process_claim(
                claim_id,
                true,
                String::from("ipfs://bafybeioreport"),
                String::new(),
            )
            .expect("Authorized assessor should approve the claim");

        let processed = contract.get_claim(claim_id).unwrap();
        assert_eq!(processed.status, ClaimStatus::Paid);
        let expected_payout = CLAIM_AMOUNT - policy.deductible;
        assert_eq!(
            processed.payout_amount, expected_payout,
            "Payout must equal claim minus policy deductible"
        );
        assert_eq!(processed.assessor, Some(accounts.charlie));

        // Pool capital was debited by the payout.
        let pool_after = contract.get_pool(pool_id).unwrap();
        assert!(pool_after.available_capital < POOL_LIQUIDITY);

        // Step 6: cooldown blocks an immediate second claim on this property
        test::set_caller::<DefaultEnvironment>(accounts.bob);
        assert_eq!(
            contract.submit_claim(
                policy_id,
                CLAIM_AMOUNT,
                String::from("Duplicate within cooldown"),
                String::new(),
            ),
            Err(InsuranceError::CooldownPeriodActive),
            "Second claim inside the cooldown window must be rejected"
        );
    }

    /// Scenario 2 - Policy creation validation errors
    #[ink::test]
    fn test_create_policy_rejects_bad_inputs() {
        let accounts = test::default_accounts::<DefaultEnvironment>();
        let mut contract = setup();
        seed_pool_and_property(&mut contract);

        // No risk assessment exists for property 99.
        test::set_caller::<DefaultEnvironment>(accounts.bob);
        test::set_value_transferred::<DefaultEnvironment>(COVERAGE_AMOUNT);
        assert_eq!(
            contract.create_policy(
                99,
                CoverageType::Fire,
                COVERAGE_AMOUNT,
                1,
                DURATION_SECONDS,
                String::new(),
            ),
            Err(InsuranceError::PropertyNotInsurable),
            "Property without a risk assessment must not be insurable"
        );

        // Unknown pool id.
        assert_eq!(
            contract.create_policy(
                1,
                CoverageType::Fire,
                COVERAGE_AMOUNT,
                999,
                DURATION_SECONDS,
                String::new(),
            ),
            Err(InsuranceError::PoolNotFound),
            "Policy must reference an existing pool"
        );

        // Underpaying the calculated premium is rejected.
        test::set_value_transferred::<DefaultEnvironment>(1u128);
        assert_eq!(
            contract.create_policy(
                1,
                CoverageType::Fire,
                COVERAGE_AMOUNT,
                1,
                DURATION_SECONDS,
                String::new(),
            ),
            Err(InsuranceError::InsufficientPremium),
            "Premium below the calculated price must be rejected"
        );

        // Nothing was created by the failed attempts.
        assert_eq!(contract.get_policy_count(), 0);
    }

    /// Scenario 3 - Oracle event authorization + parametric auto-claim firing
    #[ink::test]
    fn test_report_oracle_event_authorization_and_trigger_firing() {
        let accounts = test::default_accounts::<DefaultEnvironment>();
        let mut contract = setup();
        seed_pool_and_property(&mut contract);
        let policy_id = create_active_policy(&mut contract);

        // Unauthorized account cannot report oracle events.
        test::set_caller::<DefaultEnvironment>(accounts.charlie);
        assert_eq!(
            contract.report_oracle_event(1, 900, String::from("ipfs://bafybeireport")),
            Err(InsuranceError::Unauthorized),
            "Non-oracle reporters must be rejected"
        );

        // Admin authorizes charlie as an oracle.
        test::set_caller::<DefaultEnvironment>(accounts.alice);
        contract
            .authorize_oracle(accounts.charlie)
            .expect("Admin should authorize the oracle");

        // Only the policyholder or admin can register triggers.
        test::set_caller::<DefaultEnvironment>(accounts.charlie);
        assert_eq!(
            contract.register_claim_trigger(
                policy_id,
                TriggerMetric::RainfallMm,
                TriggerComparator::GreaterOrEqual,
                500,
                PayoutMode::Fixed(25_000_000),
            ),
            Err(InsuranceError::Unauthorized),
            "Non-holder non-admin must not register triggers"
        );

        // Policyholder registers the trigger.
        test::set_caller::<DefaultEnvironment>(accounts.bob);
        let trigger_id = contract
            .register_claim_trigger(
                policy_id,
                TriggerMetric::RainfallMm,
                TriggerComparator::GreaterOrEqual,
                500,
                PayoutMode::Fixed(25_000_000),
            )
            .expect("Policyholder should register the trigger");
        assert_eq!(trigger_id, 1);

        // Report below threshold: recorded but no claim created.
        test::set_caller::<DefaultEnvironment>(accounts.charlie);
        let no_claim = contract
            .report_oracle_event(trigger_id, 100, String::from("ipfs://bafybeireport"))
            .expect("Below-threshold report should succeed without a claim");
        assert_eq!(no_claim, None, "No auto-claim before threshold is met");

        let trigger = contract.get_claim_trigger(trigger_id).unwrap();
        assert!(!trigger.triggered);
        assert_eq!(trigger.last_observed_value, Some(100));

        // Report meeting the threshold fires the trigger exactly once and
        // creates + approves + pays a claim automatically.
        let auto_claim = contract
            .report_oracle_event(trigger_id, 600, String::from("ipfs://bafybeireport"))
            .expect("Threshold-meeting report should fire the trigger");
        let claim_id = auto_claim.expect("Auto-claim id must be returned when fired");

        let paid = contract.get_claim(claim_id).unwrap();
        assert_eq!(paid.status, ClaimStatus::Paid);
        assert!(paid.payout_amount > 0);

        let fired = contract.get_claim_trigger(trigger_id).unwrap();
        assert!(fired.triggered);
        assert_eq!(fired.triggering_claim_id, Some(claim_id));

        // The trigger cannot fire twice.
        assert_eq!(
            contract.report_oracle_event(trigger_id, 700, String::from("ipfs://bafybeireport")),
            Err(InsuranceError::TriggerAlreadyFired),
            "Trigger must fire at most once"
        );
    }
}
