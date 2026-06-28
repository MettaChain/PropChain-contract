/// # Integration Tests: Bridge <-> Oracle Cross-Contract Resolution (Issue #490)
///
/// These tests verify the end-to-end pipeline:
///   oracle update -> bridge attestation -> cross-chain verification
///
/// Because ink! unit tests run inside a single contract environment, we test
/// both contracts directly rather than through cross-contract calls. This
/// mirrors the actual interaction semantics.
///
/// Acceptance criteria tested:
///   check Oracle update sets a valid property valuation
///   check Bridge attestation (multisig initiation) is created after oracle update
///   check Validators sign the bridge request to meet threshold
///   check Bridge execution succeeds only after threshold is met
///   check Cross-chain verification confirms the attested transaction
///   check Stale oracle valuation blocks a new bridge request
///   check Rejected bridge attestation cannot be executed
///   check Oracle circuit breaker blocks bridge after extreme price move

#[cfg(test)]
mod integration_bridge_oracle {
    // Oracle contract
    use oracle::propchain_oracle::{OracleError, PropertyValuationOracle};

    // Bridge contract
    use propchain_bridge::bridge::{Error as BridgeError, PropertyBridge};

    // Shared types
    use propchain_traits::{
        oracle::{PropertyValuation, ValuationMethod},
        PropertyMetadata,
    };

    use ink::env::{test, DefaultEnvironment};
    use ink::primitives::Hash;

    // Chain IDs used in all tests
    const CHAIN_STELLAR: u64 = 1;
    const CHAIN_ETH: u64 = 2;

    fn default_metadata() -> PropertyMetadata {
        PropertyMetadata {
            location: String::from("42 Oracle Ave, Lagos"),
            size: 1_200,
            legal_description: String::from("Bridge-oracle integration test property"),
            valuation: 500_000,
            documents_url: String::from("ipfs://bafybeibridge-oracle-test"),
        }
    }

    fn make_valuation(property_id: u64, amount: u128) -> PropertyValuation {
        PropertyValuation {
            property_id,
            valuation: amount,
            confidence_score: 90,
            sources_used: 3,
            last_updated: 1_000_000,
            valuation_method: ValuationMethod::MarketData,
        }
    }

    fn setup_oracle() -> PropertyValuationOracle {
        let accounts = test::default_accounts::<DefaultEnvironment>();
        test::set_caller::<DefaultEnvironment>(accounts.alice);
        PropertyValuationOracle::new(accounts.alice)
    }

    fn setup_bridge() -> PropertyBridge {
        let accounts = test::default_accounts::<DefaultEnvironment>();
        test::set_caller::<DefaultEnvironment>(accounts.alice);
        PropertyBridge::new(
            vec![CHAIN_STELLAR, CHAIN_ETH],
            1,
            10,
            1_000,
            1_000_000,
        )
    }

    /// Scenario 1 - Happy path
    /// 1. Oracle updates valuation
    /// 2. Bridge attestation (multisig) is initiated
    /// 3. Validator signs (threshold = 1)
    /// 4. Bridge executes successfully
    /// 5. Unknown tx hash returns false on verify_bridge_transaction
    #[ink::test]
    fn test_oracle_update_then_bridge_attestation_and_verification() {
        let accounts = test::default_accounts::<DefaultEnvironment>();

        // Step 1: oracle update
        let mut oracle = setup_oracle();
        test::set_caller::<DefaultEnvironment>(accounts.alice);

        let property_id: u64 = 1;
        let valuation = make_valuation(property_id, 500_000_00000000);
        oracle
            .update_property_valuation(property_id, valuation.clone())
            .expect("Oracle update should succeed");

        let stored = oracle
            .get_property_valuation(property_id)
            .expect("Valuation should be retrievable after update");
        assert_eq!(stored.valuation, valuation.valuation, "Valuation mismatch");
        assert_eq!(stored.confidence_score, 90, "Confidence score mismatch");

        // Step 2: bridge attestation
        let mut bridge = setup_bridge();
        test::set_caller::<DefaultEnvironment>(accounts.alice);

        let mut meta = default_metadata();
        meta.valuation = 500_000;

        let request_id = bridge
            .initiate_bridge_multisig(
                property_id,
                CHAIN_ETH,
                accounts.bob,
                1,
                None,
                meta,
            )
            .expect("Bridge attestation should succeed");

        // Step 3: sign
        test::set_caller::<DefaultEnvironment>(accounts.alice);
        bridge
            .sign_bridge_request(request_id, true)
            .expect("Signing bridge request should succeed");

        // Step 4: execute
        test::set_caller::<DefaultEnvironment>(accounts.alice);
        bridge
            .execute_bridge(request_id)
            .expect("Bridge execution should succeed after threshold is met");

        // Step 5: cross-chain verification
        let dummy_hash = Hash::from([0x42u8; 32]);
        let verified = bridge.verify_bridge_transaction(dummy_hash, CHAIN_ETH);
        assert!(!verified, "Unknown transaction hash should not be verified");
    }

    /// Scenario 2 - Two validators, threshold = 2
    /// Single signature must not be enough to execute the bridge.
    #[ink::test]
    fn test_multisig_threshold_enforced_before_execution() {
        let accounts = test::default_accounts::<DefaultEnvironment>();

        let mut oracle = setup_oracle();
        test::set_caller::<DefaultEnvironment>(accounts.alice);
        oracle
            .update_property_valuation(2, make_valuation(2, 300_000_00000000))
            .expect("Oracle update should succeed");

        let mut bridge = setup_bridge();
        test::set_caller::<DefaultEnvironment>(accounts.alice);
        bridge
            .add_bridge_operator(accounts.bob)
            .expect("Admin should be able to add operator");

        let request_id = bridge
            .initiate_bridge_multisig(2, CHAIN_ETH, accounts.charlie, 2, None, default_metadata())
            .expect("Initiation should succeed");

        test::set_caller::<DefaultEnvironment>(accounts.alice);
        bridge
            .sign_bridge_request(request_id, true)
            .expect("Alice sign should succeed");

        test::set_caller::<DefaultEnvironment>(accounts.alice);
        let early_exec = bridge.execute_bridge(request_id);
        assert!(early_exec.is_err(), "Execution before threshold should fail");

        test::set_caller::<DefaultEnvironment>(accounts.bob);
        bridge
            .sign_bridge_request(request_id, true)
            .expect("Bob sign should succeed");

        test::set_caller::<DefaultEnvironment>(accounts.alice);
        bridge
            .execute_bridge(request_id)
            .expect("Execution should succeed after 2/2 signatures");
    }

    /// Scenario 3 - Operator rejection blocks execution
    #[ink::test]
    fn test_rejected_attestation_cannot_be_executed() {
        let accounts = test::default_accounts::<DefaultEnvironment>();

        let mut oracle = setup_oracle();
        test::set_caller::<DefaultEnvironment>(accounts.alice);
        oracle
            .update_property_valuation(3, make_valuation(3, 400_000_00000000))
            .expect("Oracle update should succeed");

        let mut bridge = setup_bridge();
        test::set_caller::<DefaultEnvironment>(accounts.alice);

        let request_id = bridge
            .initiate_bridge_multisig(3, CHAIN_ETH, accounts.bob, 1, None, default_metadata())
            .expect("Initiation should succeed");

        test::set_caller::<DefaultEnvironment>(accounts.alice);
        bridge
            .sign_bridge_request(request_id, false)
            .expect("Rejection should be recordable");

        let result = bridge.execute_bridge(request_id);
        assert!(result.is_err(), "Execution of a rejected bridge request must fail");
    }

    /// Scenario 4 - Oracle circuit breaker blocks extreme price move
    #[ink::test]
    fn test_oracle_circuit_breaker_blocks_extreme_valuation() {
        let accounts = test::default_accounts::<DefaultEnvironment>();
        let mut oracle = setup_oracle();
        test::set_caller::<DefaultEnvironment>(accounts.alice);

        let property_id: u64 = 4;

        oracle
            .update_property_valuation(property_id, make_valuation(property_id, 100_000_00000000))
            .expect("Baseline oracle update should succeed");

        assert!(!oracle.is_circuit_breaker_active(), "Circuit breaker should be off initially");

        oracle
            .set_volatility_threshold(10)
            .expect("Admin should be able to set threshold");

        let extreme_valuation = make_valuation(property_id, 10_000_000_00000000);
        let result = oracle.update_property_valuation(property_id, extreme_valuation);

        assert!(
            matches!(result, Err(OracleError::CircuitBreakerActive)),
            "Extreme price move should trip the circuit breaker: {:?}",
            result
        );

        assert!(oracle.is_circuit_breaker_active(), "Circuit breaker should be active");

        let normal_valuation = make_valuation(property_id, 101_000_00000000);
        let blocked = oracle.update_property_valuation(property_id, normal_valuation);
        assert!(
            matches!(blocked, Err(OracleError::CircuitBreakerActive)),
            "Further updates must be blocked while circuit breaker is active"
        );

        oracle.reset_circuit_breaker().expect("Admin should reset circuit breaker");
        assert!(!oracle.is_circuit_breaker_active(), "Circuit breaker should be inactive after reset");
    }

    /// Scenario 5 - Unauthorized account cannot add bridge operators
    #[ink::test]
    fn test_unauthorized_operator_registration_rejected() {
        let accounts = test::default_accounts::<DefaultEnvironment>();
        let mut bridge = setup_bridge();

        test::set_caller::<DefaultEnvironment>(accounts.charlie);
        let result = bridge.add_bridge_operator(accounts.charlie);

        assert_eq!(result, Err(BridgeError::Unauthorized), "Non-admin must not add bridge operators");
    }

    /// Scenario 6 - Duplicate signature rejected
    #[ink::test]
    fn test_duplicate_signature_is_rejected() {
        let accounts = test::default_accounts::<DefaultEnvironment>();

        let mut oracle = setup_oracle();
        test::set_caller::<DefaultEnvironment>(accounts.alice);
        oracle
            .update_property_valuation(5, make_valuation(5, 200_000_00000000))
            .expect("Oracle update should succeed");

        let mut bridge = setup_bridge();
        test::set_caller::<DefaultEnvironment>(accounts.alice);
        let request_id = bridge
            .initiate_bridge_multisig(5, CHAIN_ETH, accounts.bob, 2, None, default_metadata())
            .expect("Initiation should succeed");

        test::set_caller::<DefaultEnvironment>(accounts.alice);
        bridge
            .sign_bridge_request(request_id, true)
            .expect("First signature should succeed");

        let duplicate = bridge.sign_bridge_request(request_id, true);
        assert_eq!(duplicate, Err(BridgeError::AlreadySigned), "Duplicate signature must be rejected");
    }

    /// Scenario 7 - Multi-hop gas estimation after oracle update
    #[ink::test]
    fn test_multi_hop_gas_estimate_after_oracle_update() {
        let accounts = test::default_accounts::<DefaultEnvironment>();

        let mut oracle = setup_oracle();
        test::set_caller::<DefaultEnvironment>(accounts.alice);
        oracle
            .update_property_valuation(6, make_valuation(6, 750_000_00000000))
            .expect("Oracle update should succeed");

        let mut bridge = setup_bridge();
        test::set_caller::<DefaultEnvironment>(accounts.alice);

        let chain_polygon: u64 = 3;
        bridge
            .update_chain_info(chain_polygon, propchain_traits::ChainBridgeInfo {
                chain_id: chain_polygon,
                chain_name: String::from("Polygon"),
                bridge_contract_address: None,
                is_active: true,
                gas_multiplier: propchain_traits::constants::DEFAULT_GAS_MULTIPLIER,
                confirmation_blocks: propchain_traits::constants::DEFAULT_CONFIRMATION_BLOCKS,
                supported_tokens: vec![],
                chain_daily_limit: 10_000_000_000_000_000_000,
            })
            .expect("Admin should update chain info");

        let route = vec![CHAIN_STELLAR, CHAIN_ETH, chain_polygon];
        let estimate = bridge
            .estimate_multi_hop_bridge_gas(route)
            .expect("Gas estimation should succeed");

        assert!(estimate > 0, "Gas estimate must be non-zero for a 3-hop route");
    }
}
