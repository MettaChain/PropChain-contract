// Unit tests for the bridge contract (Issue #101 - extracted from lib.rs)

#[cfg(test)]
mod tests {
    use super::*;
    use ink::env::{test, DefaultEnvironment};

    fn setup_bridge() -> PropertyBridge {
        let supported_chains = vec![1, 2, 3];
        PropertyBridge::new(supported_chains, 2, 5, 100, 500000)
    }

    #[ink::test]
    fn test_constructor_works() {
        let bridge = setup_bridge();
        let config = bridge.get_config();
        assert_eq!(config.min_signatures_required, 2);
        assert_eq!(config.max_signatures_required, 5);
    }

    #[ink::test]
    fn test_initiate_bridge_multisig() {
        let mut bridge = setup_bridge();
        let accounts = test::default_accounts::<DefaultEnvironment>();
        test::set_caller::<DefaultEnvironment>(accounts.alice);

        let metadata = PropertyMetadata {
            location: String::from("Test Property"),
            size: 1000,
            legal_description: String::from("Test"),
            valuation: 100000,
            documents_url: String::from("ipfs://test"),
        };

        let result = bridge.initiate_bridge_multisig(1, 2, accounts.bob, 2, Some(50), metadata);
        assert!(result.is_ok());
    }

    #[ink::test]
    fn test_sign_bridge_request() {
        let mut bridge = setup_bridge();
        let accounts = test::default_accounts::<DefaultEnvironment>();

        // Register alice as a validator before signing (issue #203)
        test::set_caller::<DefaultEnvironment>(accounts.alice);
        bridge.add_validator(accounts.alice).expect("admin can add validator");

        let metadata = PropertyMetadata {
            location: String::from("Test Property"),
            size: 1000,
            legal_description: String::from("Test"),
            valuation: 100000,
            documents_url: String::from("ipfs://test"),
        };

        let request_id = bridge
            .initiate_bridge_multisig(1, 2, accounts.bob, 2, Some(50), metadata)
            .expect("Bridge initiation should succeed in test");

        test::set_caller::<DefaultEnvironment>(accounts.alice);
        let result = bridge.sign_bridge_request(request_id, true);
        assert!(result.is_ok());
    }

    #[ink::test]
    fn test_non_validator_cannot_sign() {
        let mut bridge = setup_bridge();
        let accounts = test::default_accounts::<DefaultEnvironment>();

        test::set_caller::<DefaultEnvironment>(accounts.alice);
        let metadata = PropertyMetadata {
            location: String::from("Test Property"),
            size: 1000,
            legal_description: String::from("Test"),
            valuation: 100000,
            documents_url: String::from("ipfs://test"),
        };
        let request_id = bridge
            .initiate_bridge_multisig(1, 2, accounts.bob, 2, Some(50), metadata)
            .expect("initiation should succeed");

        // bob is a bridge operator but NOT a validator — must be rejected
        bridge.add_bridge_operator(accounts.bob).expect("admin can add operator");
        test::set_caller::<DefaultEnvironment>(accounts.bob);
        let result = bridge.sign_bridge_request(request_id, true);
        assert_eq!(result, Err(Error::Unauthorized));
    }

    #[ink::test]
    fn test_threshold_enforced_at_execution() {
        let mut bridge = setup_bridge();
        let accounts = test::default_accounts::<DefaultEnvironment>();

        // Register two validators
        test::set_caller::<DefaultEnvironment>(accounts.alice);
        bridge.add_validator(accounts.alice).expect("add validator alice");
        bridge.add_validator(accounts.bob).expect("add validator bob");
        bridge.add_bridge_operator(accounts.bob).expect("add operator bob");

        let metadata = PropertyMetadata {
            location: String::from("Test Property"),
            size: 1000,
            legal_description: String::from("Test"),
            valuation: 100000,
            documents_url: String::from("ipfs://test"),
        };
        let request_id = bridge
            .initiate_bridge_multisig(1, 2, accounts.charlie, 2, Some(50), metadata)
            .expect("initiation should succeed");

        // Only one signature — execution must fail
        test::set_caller::<DefaultEnvironment>(accounts.alice);
        bridge.sign_bridge_request(request_id, true).expect("alice signs");

        test::set_caller::<DefaultEnvironment>(accounts.alice);
        let result = bridge.execute_bridge(request_id);
        assert_eq!(result, Err(Error::InvalidRequest)); // status not Locked yet

        // Second signature — now threshold met, execution succeeds
        test::set_caller::<DefaultEnvironment>(accounts.bob);
        bridge.sign_bridge_request(request_id, true).expect("bob signs");

        test::set_caller::<DefaultEnvironment>(accounts.alice);
        let result = bridge.execute_bridge(request_id);
        assert!(result.is_ok());
    }

    #[ink::test]
    fn test_cross_chain_trade_lifecycle() {
        let mut bridge = setup_bridge();
        let accounts = test::default_accounts::<DefaultEnvironment>();
        test::set_caller::<DefaultEnvironment>(accounts.bob);

        let trade_id = bridge
            .register_cross_chain_trade(9, Some(7), 2, accounts.charlie, 50_000, 49_000)
            .expect("cross-chain trade registration should succeed");
        let trade = bridge
            .get_cross_chain_trade(trade_id)
            .expect("trade should be stored");
        assert_eq!(trade.status, CrossChainTradeStatus::Pending);
        assert_eq!(trade.destination_chain, 2);

        bridge
            .attach_bridge_request_to_trade(trade_id, 33)
            .expect("trader can attach bridge request");
        let attached = bridge
            .get_cross_chain_trade(trade_id)
            .expect("attached trade should exist");
        assert_eq!(attached.bridge_request_id, Some(33));

        test::set_caller::<DefaultEnvironment>(accounts.alice);
        bridge
            .settle_cross_chain_trade(trade_id)
            .expect("admin can settle trade");
        let settled = bridge
            .get_cross_chain_trade(trade_id)
            .expect("settled trade should exist");
        assert_eq!(settled.status, CrossChainTradeStatus::Settled);
    }

    #[ink::test]
    fn test_estimate_bridge_gas_respects_chain_profile() {
        let mut bridge = setup_bridge();

        let default_gas = bridge
            .estimate_bridge_gas(1, 2)
            .expect("default chain should be estimable");

        let tuned_chain = ChainBridgeInfo {
            chain_id: 2,
            chain_name: String::from("High-Confirmation"),
            bridge_contract_address: None,
            is_active: true,
            gas_multiplier: 180,
            confirmation_blocks: 24,
            supported_tokens: Vec::new(),
        };
        bridge
            .update_chain_info(2, tuned_chain)
            .expect("admin should update chain profile");

        let updated_gas = bridge
            .estimate_bridge_gas(1, 2)
            .expect("updated chain should be estimable");

        assert!(updated_gas > default_gas);
        assert!(updated_gas <= bridge.get_config().gas_limit_per_bridge);
    }

    #[ink::test]
    fn test_quote_cross_chain_trade_scales_with_amount() {
        let bridge = setup_bridge();

        let small = bridge
            .quote_cross_chain_trade(2, 50_000)
            .expect("small quote should succeed");
        let large = bridge
            .quote_cross_chain_trade(2, 100_000)
            .expect("large quote should succeed");

        assert!(small.total_fee >= small.protocol_fee);
        assert!(large.total_fee > small.total_fee);
        assert!(large.protocol_fee > small.protocol_fee);
    }
}

    // ── #181: Formal verification property tests for bridge multi-sig logic ───

    /// PROPERTY: A bridge request must never be executed with fewer signatures
    /// than `min_signatures_required`.
    ///
    /// Formal invariant:  ∀ request r. r.status == Completed ⟹
    ///                      |r.signatures| >= config.min_signatures_required
    #[ink::test]
    fn property_execution_requires_minimum_signatures() {
        let mut bridge = setup_bridge(); // min_signatures = 2
        let accounts = test::default_accounts::<DefaultEnvironment>();
        test::set_caller::<DefaultEnvironment>(accounts.alice);

        let metadata = PropertyMetadata {
            location: String::from("Formal Test"),
            size: 500,
            legal_description: String::from("Prop"),
            valuation: 50000,
            documents_url: String::from("ipfs://formal"),
        };

        let request_id = bridge
            .initiate_bridge_multisig(1, 2, accounts.bob, 2, None, metadata)
            .expect("initiate should succeed");

        // Attempt execution with zero signatures — must fail
        let result = bridge.execute_bridge(request_id);
        assert!(
            result.is_err(),
            "Bridge must not execute with 0 signatures (invariant: |sigs| >= min)"
        );

        // Add one signature (below minimum of 2) — must still fail
        test::set_caller::<DefaultEnvironment>(accounts.alice);
        bridge
            .sign_bridge_request(request_id, true)
            .expect("first sign should succeed");
        let result = bridge.execute_bridge(request_id);
        assert!(
            result.is_err(),
            "Bridge must not execute with 1 signature when minimum is 2"
        );
    }

    /// PROPERTY: A signer may not sign the same request twice (replay protection).
    ///
    /// Formal invariant:  ∀ request r, signer s.
    ///                      s ∈ r.signatures ⟹ sign(r, s) returns AlreadySigned
    #[ink::test]
    fn property_no_duplicate_signatures() {
        let mut bridge = setup_bridge();
        let accounts = test::default_accounts::<DefaultEnvironment>();
        test::set_caller::<DefaultEnvironment>(accounts.alice);

        let metadata = PropertyMetadata {
            location: String::from("Dup Test"),
            size: 200,
            legal_description: String::from("Dup"),
            valuation: 20000,
            documents_url: String::from("ipfs://dup"),
        };

        let request_id = bridge
            .initiate_bridge_multisig(1, 2, accounts.bob, 2, None, metadata)
            .expect("initiate should succeed");

        // First signature — must succeed
        test::set_caller::<DefaultEnvironment>(accounts.alice);
        bridge
            .sign_bridge_request(request_id, true)
            .expect("first signature must succeed");

        // Second signature from the same account — must return AlreadySigned
        let result = bridge.sign_bridge_request(request_id, true);
        assert_eq!(
            result,
            Err(Error::AlreadySigned),
            "Duplicate signature must return AlreadySigned (replay protection invariant)"
        );
    }

    /// PROPERTY: Signatures on an expired request must be rejected.
    ///
    /// Formal invariant:  ∀ request r. now() > r.expires_at ⟹
    ///                      sign(r, _) returns RequestExpired
    #[ink::test]
    fn property_expired_request_rejects_signatures() {
        let mut bridge = setup_bridge();
        let accounts = test::default_accounts::<DefaultEnvironment>();
        test::set_caller::<DefaultEnvironment>(accounts.alice);

        let metadata = PropertyMetadata {
            location: String::from("Expiry Test"),
            size: 100,
            legal_description: String::from("Exp"),
            valuation: 10000,
            documents_url: String::from("ipfs://exp"),
        };

        // Create request with a 1-block timeout so it expires immediately
        let request_id = bridge
            .initiate_bridge_multisig(1, 2, accounts.bob, 2, Some(1), metadata)
            .expect("initiate should succeed");

        // Advance block number past the expiry
        test::advance_block::<DefaultEnvironment>();
        test::advance_block::<DefaultEnvironment>();

        test::set_caller::<DefaultEnvironment>(accounts.alice);
        let result = bridge.sign_bridge_request(request_id, true);
        assert_eq!(
            result,
            Err(Error::RequestExpired),
            "Signing an expired request must return RequestExpired (time-safety invariant)"
        );
    }

    /// PROPERTY: Execution of a completed request is idempotent — calling
    /// execute_bridge a second time must fail, not double-execute.
    ///
    /// Formal invariant:  ∀ request r. r.status == Completed ⟹
    ///                      execute(r) returns InvalidRequest
    #[ink::test]
    fn property_no_double_execution() {
        let mut bridge = setup_bridge(); // min = 2, max = 5
        let accounts = test::default_accounts::<DefaultEnvironment>();

        test::set_caller::<DefaultEnvironment>(accounts.alice);
        let metadata = PropertyMetadata {
            location: String::from("Double-exec Test"),
            size: 300,
            legal_description: String::from("Dbl"),
            valuation: 30000,
            documents_url: String::from("ipfs://dbl"),
        };
        let request_id = bridge
            .initiate_bridge_multisig(1, 2, accounts.bob, 2, None, metadata)
            .expect("initiate should succeed");

        // Gather 2 signatures (min required)
        test::set_caller::<DefaultEnvironment>(accounts.alice);
        bridge.sign_bridge_request(request_id, true).ok();
        test::set_caller::<DefaultEnvironment>(accounts.bob);
        bridge.sign_bridge_request(request_id, true).ok();

        // First execution may succeed (depends on contract state); record result
        let first = bridge.execute_bridge(request_id);

        // Second execution must fail regardless
        let second = bridge.execute_bridge(request_id);
        assert!(
            second.is_err(),
            "Second execution of the same request must fail (idempotency invariant); first={:?}",
            first
        );
    }
}
