// Unit tests for the bridge contract (Issue #101 - extracted from lib.rs)

#[cfg(test)]
mod tests {
    use super::*;
    use ink::env::{test, DefaultEnvironment};
    use scale::{Decode, Encode};

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
    fn test_emergency_multi_sig_pause_bridge() {
        let mut bridge = setup_bridge();
        let accounts = test::default_accounts::<DefaultEnvironment>();
        test::set_caller::<DefaultEnvironment>(accounts.alice);

        // Add emergency signers
        bridge
            .add_emergency_signer(accounts.bob)
            .expect("add emergency signer");
        bridge
            .add_emergency_signer(accounts.charlie)
            .expect("add emergency signer");

        // Set threshold to 2
        bridge
            .set_emergency_threshold(2)
            .expect("set emergency threshold");

        // Propose pause bridge as bob
        test::set_caller::<DefaultEnvironment>(accounts.bob);
        let pause_flags = propchain_traits::PauseFlags {
            all_operations: true,
            new_requests: true,
            signing: false,
            execution: false,
            cross_chain_trades: false,
        };
        let request_id = bridge
            .propose_pause_bridge(
                pause_flags,
                propchain_traits::PauseReason::ManualAdmin,
                Some(String::from("Test pause")),
                Some(100),
            )
            .expect("propose pause bridge");

        // Sign as charlie
        test::set_caller::<DefaultEnvironment>(accounts.charlie);
        bridge
            .sign_emergency_request(request_id)
            .expect("sign emergency request");

        // Execute as bob
        test::set_caller::<DefaultEnvironment>(accounts.bob);
        bridge
            .execute_emergency_request(request_id)
            .expect("execute emergency request");

        // Verify bridge is paused
        let flags = bridge.get_pause_flags();
        assert!(flags.all_operations);
        assert!(flags.new_requests);
    }

    #[ink::test]
    fn test_emergency_multi_sig_freeze_asset() {
        let mut bridge = setup_bridge();
        let accounts = test::default_accounts::<DefaultEnvironment>();
        test::set_caller::<DefaultEnvironment>(accounts.alice);

        // Add emergency signers
        bridge
            .add_emergency_signer(accounts.bob)
            .expect("add emergency signer");
        bridge
            .add_emergency_signer(accounts.charlie)
            .expect("add emergency signer");

        // Set threshold to 2
        bridge
            .set_emergency_threshold(2)
            .expect("set emergency threshold");

        // Propose freeze asset as bob
        test::set_caller::<DefaultEnvironment>(accounts.bob);
        let asset_address = AccountId::from([1u8; 32]);
        let request_id = bridge
            .propose_freeze_asset(
                asset_address,
                String::from("Suspicious activity detected"),
                true,
                Some(100),
            )
            .expect("propose freeze asset");

        // Sign as charlie
        test::set_caller::<DefaultEnvironment>(accounts.charlie);
        bridge
            .sign_emergency_request(request_id)
            .expect("sign emergency request");

        // Execute as bob
        test::set_caller::<DefaultEnvironment>(accounts.bob);
        bridge
            .execute_emergency_request(request_id)
            .expect("execute emergency request");

        // Verify asset is frozen
        assert!(bridge.is_asset_frozen(asset_address));

        // Unfreeze as admin
        test::set_caller::<DefaultEnvironment>(accounts.alice);
        bridge
            .unfreeze_asset(asset_address)
            .expect("unfreeze asset");

        // Verify asset is not frozen
        assert!(!bridge.is_asset_frozen(asset_address));
    }

    // Freeze-by-token (#12): Uses the new `freeze_token` method which keys on
    // `token_id: TokenId` (a `u64`), matching the bridge initiation paths.
    // The old `propose_freeze_asset` / emergency-multi-sig flow still keys on
    // `AccountId` for contract-level asset freezes.
    #[ink::test]
    fn test_asset_freeze_blocks_bridge_initiation() {
        let mut bridge = setup_bridge();
        let accounts = test::default_accounts::<DefaultEnvironment>();
        test::set_caller::<DefaultEnvironment>(accounts.alice);

        // Freeze token_id = 1 directly via admin-only freeze_token
        bridge
            .freeze_token(1)
            .expect("admin freeze token");

        // Verify token is reported as frozen
        assert!(bridge.is_token_frozen(1));

        // Try to initiate bridge with frozen token - should fail
        let metadata = PropertyMetadata {
            location: String::from("Test Property"),
            size: 1000,
            legal_description: String::from("Test"),
            valuation: 100000,
            documents_url: String::from("ipfs://test"),
        };

        let result = bridge.initiate_bridge_multisig(
            1u64, // Frozen asset token_id
            2,
            accounts.bob,
            2,
            Some(50),
            metadata,
        );
        assert_eq!(result, Err(Error::AssetAlreadyFrozen));

        // Unfreeze the token
        bridge
            .unfreeze_token(1)
            .expect("admin unfreeze token");

        // Verify token is no longer frozen
        assert!(!bridge.is_token_frozen(1));

        // Now bridge initiation should succeed
        let metadata2 = PropertyMetadata {
            location: String::from("Test Property"),
            size: 1000,
            legal_description: String::from("Test"),
            valuation: 100000,
            documents_url: String::from("ipfs://test"),
        };

        let result = bridge.initiate_bridge_multisig(
            1u64,
            2,
            accounts.bob,
            2,
            Some(50),
            metadata2,
        );
        assert!(result.is_ok());
    }

    #[ink::test]
    fn test_freeze_token_non_admin_rejected() {
        let mut bridge = setup_bridge();
        let accounts = test::default_accounts::<DefaultEnvironment>();

        // Bob is not admin - freeze should be rejected
        test::set_caller::<DefaultEnvironment>(accounts.bob);
        let result = bridge.freeze_token(1);
        assert_eq!(result, Err(Error::Unauthorized));

        // Unfreeze should also be rejected
        let result = bridge.unfreeze_token(1);
        assert_eq!(result, Err(Error::Unauthorized));
    }

    #[ink::test]
    fn test_freeze_already_frozen_token_rejected() {
        let mut bridge = setup_bridge();
        let accounts = test::default_accounts::<DefaultEnvironment>();
        test::set_caller::<DefaultEnvironment>(accounts.alice);

        // Freeze token_id = 1
        bridge
            .freeze_token(1)
            .expect("freeze should succeed");

        // Freeze again should fail
        let result = bridge.freeze_token(1);
        assert_eq!(result, Err(Error::AssetAlreadyFrozen));

        // Unfreeze a non-frozen token should fail
        let result = bridge.unfreeze_token(999);
        assert_eq!(result, Err(Error::AssetNotFrozen));
    }

    #[ink::test]
    fn test_batch_verification_configuration() {
        let mut bridge = setup_bridge();
        let accounts = test::default_accounts::<DefaultEnvironment>();
        test::set_caller::<DefaultEnvironment>(accounts.alice);

        // Get default configuration
        let (window_size, window_duration) = bridge.get_batch_config();
        assert_eq!(window_size, 10);
        assert_eq!(window_duration, 300);

        // Update configuration
        bridge
            .configure_batch_verification(20, 600)
            .expect("configure batch verification");

        let (new_window_size, new_window_duration) = bridge.get_batch_config();
        assert_eq!(new_window_size, 20);
        assert_eq!(new_window_duration, 600);
    }

    #[ink::test]
    fn test_batch_verification_window_creation() {
        let mut bridge = setup_bridge();
        let accounts = test::default_accounts::<DefaultEnvironment>();
        test::set_caller::<DefaultEnvironment>(accounts.alice);

        // Add validator and operator for bridge execution
        bridge
            .add_validator(accounts.alice)
            .expect("add validator");
        bridge
            .add_validator(accounts.bob)
            .expect("add validator");

        // Execute a bridge transaction to create a batch window
        let metadata = PropertyMetadata {
            location: String::from("Test Property"),
            size: 1000,
            legal_description: String::from("Test"),
            valuation: 100000,
            documents_url: String::from("ipfs://test"),
        };

        let request_id = bridge
            .initiate_bridge_multisig(1, 2, accounts.bob, 2, Some(50), metadata)
            .expect("initiate bridge");

        test::set_caller::<DefaultEnvironment>(accounts.alice);
        bridge
            .sign_bridge_request(request_id, true)
            .expect("sign");

        test::set_caller::<DefaultEnvironment>(accounts.bob);
        bridge
            .sign_bridge_request(request_id, true)
            .expect("sign");

        test::set_caller::<DefaultEnvironment>(accounts.alice);
        bridge
            .execute_bridge(request_id)
            .expect("execute bridge");

        // Get transaction hash from bridge history
        let history = bridge.get_bridge_history(accounts.alice, 0, 100);
        assert!(!history.is_empty());
        let transaction_hash = history[0].transaction_hash;

        // Check that transaction is in a batch
        let batch_info = bridge.get_transaction_batch(transaction_hash);
        assert!(batch_info.is_some());
        let (source_chain, _window_id) = batch_info.unwrap();
        assert_eq!(source_chain, 1); // Default source chain
    }

    #[ink::test]
    fn test_batch_merkle_root_submission_and_verification() {
        let mut bridge = setup_bridge();
        let accounts = test::default_accounts::<DefaultEnvironment>();
        test::set_caller::<DefaultEnvironment>(accounts.alice);

        // Add validator and operator
        bridge
            .add_validator(accounts.alice)
            .expect("add validator");
        bridge
            .add_validator(accounts.bob)
            .expect("add validator");

        // Execute a bridge transaction
        let metadata = PropertyMetadata {
            location: String::from("Test Property"),
            size: 1000,
            legal_description: String::from("Test"),
            valuation: 100000,
            documents_url: String::from("ipfs://test"),
        };

        let request_id = bridge
            .initiate_bridge_multisig(1, 2, accounts.bob, 2, Some(50), metadata)
            .expect("initiate bridge");

        test::set_caller::<DefaultEnvironment>(accounts.alice);
        bridge
            .sign_bridge_request(request_id, true)
            .expect("sign");

        test::set_caller::<DefaultEnvironment>(accounts.bob);
        bridge
            .sign_bridge_request(request_id, true)
            .expect("sign");

        test::set_caller::<DefaultEnvironment>(accounts.alice);
        bridge
            .execute_bridge(request_id)
            .expect("execute bridge");

        // Get transaction hash and batch info
        let history = bridge.get_bridge_history(accounts.alice, 0, 100);
        let transaction_hash = history[0].transaction_hash;
        let batch_info = bridge.get_transaction_batch(transaction_hash).unwrap();

        // Submit batch Merkle root
        let merkle_root = Hash::from([1u8; 32]);
        bridge
            .submit_batch_merkle_root(batch_info.0, batch_info.1, merkle_root)
            .expect("submit batch merkle root");

        // Verify batch Merkle root
        bridge
            .verify_batch_merkle_root(batch_info.0, batch_info.1, merkle_root)
            .expect("verify batch merkle root");

        // Check that transaction is now verified
        assert!(bridge.verify_bridge_transaction(transaction_hash, batch_info.0));

        // Get batch window信息
        let window_info = bridge.get_batch_window_info(batch_info.0, batch_info.1);
        assert!(window_info.is_some());
        let (stored_root, transactions) = window_info.unwrap();
        assert_eq!(stored_root, merkle_root);
        assert!(transactions.contains(&transaction_hash));
    }

    #[ink::test]
    fn test_batch_verification_unauthorized() {
        let mut bridge = setup_bridge();
        let accounts = test::default_accounts::<DefaultEnvironment>();
        test::set_caller::<DefaultEnvironment>(accounts.bob);

        // Try to configure batch verification as non-admin
        let result = bridge.configure_batch_verification(20, 600);
        assert!(result.is_err());

        // Try to submit batch Merkle root as non-admin/non-operator
        let merkle_root = Hash::from([1u8; 32]);
        let result = bridge.submit_batch_merkle_root(1, 1, merkle_root);
        assert!(result.is_err());

        // Try to verify batch Merkle root as non-admin/non-operator
        let result = bridge.verify_batch_merkle_root(1, 1, merkle_root);
        assert!(result.is_err());
    }

    #[ink::test]
    fn test_initiate_multi_hop_bridge_two_hops() {
        let mut bridge = setup_bridge();
        let accounts = test::default_accounts::<DefaultEnvironment>();
        test::set_caller::<DefaultEnvironment>(accounts.alice);

        bridge
            .add_validator(accounts.alice)
            .expect("admin can add alice validator");
        bridge
            .add_validator(accounts.bob)
            .expect("admin can add bob validator");
        bridge
            .add_bridge_operator(accounts.alice)
            .expect("admin can add alice operator");
        bridge
            .add_bridge_operator(accounts.bob)
            .expect("admin can add bob operator");

        let metadata = PropertyMetadata {
            location: String::from("Test Property"),
            size: 1000,
            legal_description: String::from("Test"),
            valuation: 100000,
            documents_url: String::from("ipfs://test"),
        };
        let route = vec![2, 3];

        let request_id = bridge
            .initiate_multi_hop_bridge(1, route.clone(), accounts.bob, 2, Some(50), metadata)
            .expect("multi-hop initiation should succeed");

        let total_gas = bridge
            .estimate_multi_hop_bridge_gas(route.clone())
            .expect("multi-hop gas estimate should succeed");
        assert!(total_gas > 0);

        assert_eq!(
            bridge
                .get_multi_hop_status(request_id)
                .expect("status query"),
            MultiHopStatus::InProgress
        );

        // First hop approval
        test::set_caller::<DefaultEnvironment>(accounts.alice);
        bridge
            .sign_bridge_request(request_id, true)
            .expect("alice signs");
        test::set_caller::<DefaultEnvironment>(accounts.bob);
        bridge
            .sign_bridge_request(request_id, true)
            .expect("bob signs");

        test::set_caller::<DefaultEnvironment>(accounts.alice);
        bridge
            .execute_bridge(request_id)
            .expect("first hop executes");

        assert_eq!(
            bridge
                .get_multi_hop_status(request_id)
                .expect("status query"),
            MultiHopStatus::InProgress
        );

        // Second hop approval
        test::set_caller::<DefaultEnvironment>(accounts.alice);
        bridge
            .sign_bridge_request(request_id, true)
            .expect("alice signs second hop");
        test::set_caller::<DefaultEnvironment>(accounts.bob);
        bridge
            .sign_bridge_request(request_id, true)
            .expect("bob signs second hop");

        test::set_caller::<DefaultEnvironment>(accounts.alice);
        bridge
            .execute_bridge(request_id)
            .expect("second hop executes");

        assert_eq!(
            bridge
                .get_multi_hop_status(request_id)
                .expect("status query"),
            MultiHopStatus::HopCompleted
        );
    }

    #[ink::test]
    fn test_multi_hop_recovery_from_failed_intermediate_hop() {
        let mut bridge = setup_bridge();
        let accounts = test::default_accounts::<DefaultEnvironment>();
        test::set_caller::<DefaultEnvironment>(accounts.alice);

        bridge
            .add_validator(accounts.alice)
            .expect("admin can add alice validator");
        bridge
            .add_validator(accounts.bob)
            .expect("admin can add bob validator");
        bridge
            .add_bridge_operator(accounts.alice)
            .expect("admin can add alice operator");
        bridge
            .add_bridge_operator(accounts.bob)
            .expect("admin can add bob operator");

        let metadata = PropertyMetadata {
            location: String::from("Test Property"),
            size: 1000,
            legal_description: String::from("Test"),
            valuation: 100000,
            documents_url: String::from("ipfs://test"),
        };
        let route = vec![2, 3];

        let request_id = bridge
            .initiate_multi_hop_bridge(1, route.clone(), accounts.bob, 2, Some(50), metadata)
            .expect("multi-hop initiation should succeed");

        test::set_caller::<DefaultEnvironment>(accounts.alice);
        bridge
            .sign_bridge_request(request_id, true)
            .expect("alice signs first hop");
        test::set_caller::<DefaultEnvironment>(accounts.bob);
        bridge
            .sign_bridge_request(request_id, true)
            .expect("bob signs first hop");

        test::set_caller::<DefaultEnvironment>(accounts.alice);
        bridge
            .execute_bridge(request_id)
            .expect("first hop executes");

        // Fail the second hop
        test::set_caller::<DefaultEnvironment>(accounts.alice);
        bridge
            .sign_bridge_request(request_id, false)
            .expect("alice rejects second hop");

        assert_eq!(
            bridge
                .get_multi_hop_status(request_id)
                .expect("status query"),
            MultiHopStatus::Failed
        );

        test::set_caller::<DefaultEnvironment>(accounts.alice);
        bridge
            .recover_failed_bridge(request_id, RecoveryAction::RetryBridge)
            .expect("recovery should succeed");

        assert_eq!(
            bridge
                .get_multi_hop_status(request_id)
                .expect("status query"),
            MultiHopStatus::InProgress
        );
    }

    #[ink::test]
    fn test_sign_bridge_request() {
        let mut bridge = setup_bridge();
        let accounts = test::default_accounts::<DefaultEnvironment>();

        // Register alice as a validator before signing (issue #203)
        test::set_caller::<DefaultEnvironment>(accounts.alice);
        bridge
            .add_validator(accounts.alice)
            .expect("admin can add validator");

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
        bridge
            .add_bridge_operator(accounts.bob)
            .expect("admin can add operator");
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
        bridge
            .add_validator(accounts.alice)
            .expect("add validator alice");
        bridge
            .add_validator(accounts.bob)
            .expect("add validator bob");
        bridge
            .add_bridge_operator(accounts.bob)
            .expect("add operator bob");

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
        bridge
            .sign_bridge_request(request_id, true)
            .expect("alice signs");

        test::set_caller::<DefaultEnvironment>(accounts.alice);
        let result = bridge.execute_bridge(request_id);
        assert_eq!(result, Err(Error::InvalidRequest)); // status not Locked yet

        // Second signature — now threshold met, execution succeeds
        test::set_caller::<DefaultEnvironment>(accounts.bob);
        bridge
            .sign_bridge_request(request_id, true)
            .expect("bob signs");

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
            chain_daily_limit: 10_000_000_000_000_000_000,
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
        bridge.add_validator(accounts.alice).expect("add validator alice");

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
        bridge.add_validator(accounts.alice).expect("add validator alice");

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
        bridge.add_validator(accounts.alice).expect("add validator alice");

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

    // ── TASK 1: Cross-chain transaction status tracking ─────────────────

    fn make_metadata() -> PropertyMetadata {
        PropertyMetadata {
            location: String::from("Test Property"),
            size: 1000,
            legal_description: String::from("Test"),
            valuation: 100_000,
            documents_url: String::from("ipfs://test"),
        }
    }

    #[ink::test]
    fn cross_chain_status_initialized_on_initiate() {
        let mut bridge = setup_bridge();
        let accounts = test::default_accounts::<DefaultEnvironment>();
        test::set_caller::<DefaultEnvironment>(accounts.alice);

        let request_id = bridge
            .initiate_bridge_multisig(1, 2, accounts.bob, 2, Some(50), make_metadata())
            .expect("initiate should succeed");

        let tracker = bridge
            .get_cross_chain_tx_status(request_id)
            .expect("tracker should exist after initiate");
        assert_eq!(tracker.request_id, request_id);
        assert_eq!(tracker.destination_chain, 2);
        assert_eq!(tracker.source_status.status, ChainTxStatus::Submitted);
        assert_eq!(tracker.destination_status.status, ChainTxStatus::NotStarted);
        assert_eq!(
            tracker.overall_status,
            propchain_traits::bridge::BridgeOperationStatus::Pending
        );
        assert_eq!(tracker.history.len(), 1);
    }

    #[ink::test]
    fn update_chain_tx_status_advances_destination_leg() {
        let mut bridge = setup_bridge();
        let accounts = test::default_accounts::<DefaultEnvironment>();
        test::set_caller::<DefaultEnvironment>(accounts.alice);

        let request_id = bridge
            .initiate_bridge_multisig(1, 2, accounts.bob, 2, Some(50), make_metadata())
            .expect("initiate should succeed");

        // Alice (admin/operator) reports the destination leg progressing.
        bridge
            .update_chain_tx_status(
                request_id,
                2, // destination chain
                ChainTxStatus::Submitted,
                None,
                100,
                0,
                None,
            )
            .expect("first update should succeed");
        bridge
            .update_chain_tx_status(request_id, 2, ChainTxStatus::Confirming, None, 101, 3, None)
            .expect("confirming update should succeed");

        let dest = bridge
            .get_chain_status(request_id, 2)
            .expect("destination status");
        assert_eq!(dest.status, ChainTxStatus::Confirming);
        assert_eq!(dest.confirmations, 3);

        let history = bridge.get_tx_status_history(request_id);
        assert!(history.len() >= 3, "history must record every update");
    }

    #[ink::test]
    fn confirm_destination_delivery_completes_overall_status() {
        let mut bridge = setup_bridge();
        let accounts = test::default_accounts::<DefaultEnvironment>();
        test::set_caller::<DefaultEnvironment>(accounts.alice);
        bridge.add_validator(accounts.alice).expect("add validator");

        let request_id = bridge
            .initiate_bridge_multisig(1, 2, accounts.bob, 2, None, make_metadata())
            .expect("initiate");

        bridge.sign_bridge_request(request_id, true).expect("sign");
        // Need a second signature to reach min_signatures_required = 2.
        bridge
            .add_validator(accounts.charlie)
            .expect("add second validator");
        test::set_caller::<DefaultEnvironment>(accounts.charlie);
        bridge
            .sign_bridge_request(request_id, true)
            .expect("second sign");

        // Alice is also an operator (constructor caller); execute the bridge.
        test::set_caller::<DefaultEnvironment>(accounts.alice);
        bridge.execute_bridge(request_id).expect("execute");

        // After execute: source = Confirmed, destination = Submitted, overall = InTransit.
        let mid = bridge
            .get_cross_chain_tx_status(request_id)
            .expect("tracker");
        assert_eq!(mid.source_status.status, ChainTxStatus::Confirmed);
        assert_eq!(mid.destination_status.status, ChainTxStatus::Submitted);
        assert_eq!(
            mid.overall_status,
            propchain_traits::bridge::BridgeOperationStatus::InTransit
        );

        // Relayer confirms destination delivery.
        let dest_hash = ink::primitives::Hash::from([7u8; 32]);
        bridge
            .confirm_destination_delivery(request_id, dest_hash, 200, 12)
            .expect("confirm destination");

        let final_status = bridge
            .get_cross_chain_tx_status(request_id)
            .expect("tracker");
        assert_eq!(
            final_status.destination_status.status,
            ChainTxStatus::Confirmed
        );
        assert_eq!(
            final_status.overall_status,
            propchain_traits::bridge::BridgeOperationStatus::Completed
        );
        // Tx hash reverse lookup should now resolve.
        let by_hash = bridge
            .get_tx_status_by_hash(dest_hash)
            .expect("lookup by destination hash");
        assert_eq!(by_hash.request_id, request_id);
    }

    #[ink::test]
    fn invalid_chain_id_rejected() {
        let mut bridge = setup_bridge();
        let accounts = test::default_accounts::<DefaultEnvironment>();
        test::set_caller::<DefaultEnvironment>(accounts.alice);

        let request_id = bridge
            .initiate_bridge_multisig(1, 2, accounts.bob, 2, None, make_metadata())
            .expect("initiate");

        let err = bridge
            .update_chain_tx_status(
                request_id,
                999, // not source nor destination
                ChainTxStatus::Submitted,
                None,
                0,
                0,
                None,
            )
            .unwrap_err();
        assert_eq!(err, Error::InvalidChain);
    }

    #[ink::test]
    fn invalid_status_transition_rejected() {
        let mut bridge = setup_bridge();
        let accounts = test::default_accounts::<DefaultEnvironment>();
        test::set_caller::<DefaultEnvironment>(accounts.alice);

        let request_id = bridge
            .initiate_bridge_multisig(1, 2, accounts.bob, 2, None, make_metadata())
            .expect("initiate");

        // Move destination Submitted → Confirmed.
        bridge
            .update_chain_tx_status(request_id, 2, ChainTxStatus::Submitted, None, 100, 0, None)
            .expect("submitted");
        bridge
            .update_chain_tx_status(request_id, 2, ChainTxStatus::Confirmed, None, 101, 12, None)
            .expect("confirmed");

        // Confirmed → Submitted must be rejected.
        let err = bridge
            .update_chain_tx_status(request_id, 2, ChainTxStatus::Submitted, None, 102, 0, None)
            .unwrap_err();
        assert_eq!(err, Error::InvalidStatusTransition);
    }

    #[ink::test]
    fn unauthorized_caller_cannot_update_status() {
        let mut bridge = setup_bridge();
        let accounts = test::default_accounts::<DefaultEnvironment>();
        test::set_caller::<DefaultEnvironment>(accounts.alice);

        let request_id = bridge
            .initiate_bridge_multisig(1, 2, accounts.bob, 2, None, make_metadata())
            .expect("initiate");

        // Bob is neither admin nor operator.
        test::set_caller::<DefaultEnvironment>(accounts.bob);
        let err = bridge
            .update_chain_tx_status(request_id, 2, ChainTxStatus::Submitted, None, 0, 0, None)
            .unwrap_err();
        assert_eq!(err, Error::Unauthorized);
    }

    #[ink::test]
    fn rollback_marks_both_legs_failed() {
        let mut bridge = setup_bridge();
        let accounts = test::default_accounts::<DefaultEnvironment>();
        test::set_caller::<DefaultEnvironment>(accounts.alice);

        let request_id = bridge
            .initiate_bridge_multisig(1, 2, accounts.bob, 2, None, make_metadata())
            .expect("initiate");

        bridge
            .rollback_bridge_transaction(request_id, String::from("manual rollback"))
            .expect("rollback");

        let tracker = bridge
            .get_cross_chain_tx_status(request_id)
            .expect("tracker");
        assert_eq!(tracker.source_status.status, ChainTxStatus::Failed);
        assert_eq!(tracker.destination_status.status, ChainTxStatus::Failed);
        assert_eq!(
            tracker.overall_status,
            propchain_traits::bridge::BridgeOperationStatus::Failed
        );
    }

    #[ink::test]
    fn unknown_request_returns_transaction_not_found() {
        let mut bridge = setup_bridge();
        let accounts = test::default_accounts::<DefaultEnvironment>();
        test::set_caller::<DefaultEnvironment>(accounts.alice);

        let err = bridge
            .update_chain_tx_status(999_999, 2, ChainTxStatus::Submitted, None, 0, 0, None)
            .unwrap_err();
        assert_eq!(err, Error::TransactionNotFound);
    }

    fn count_bitmap_bits(bitmap: &[u8; SIGNATURE_BITMAP_BYTES]) -> u8 {
        bitmap
            .iter()
            .map(|byte| byte.count_ones() as u16)
            .sum::<u16>() as u8
    }

    #[ink::test]
    fn bitmap_signature_tracking_and_signer_queries_work() {
        let mut bridge = setup_bridge();
        let accounts = test::default_accounts::<DefaultEnvironment>();

        bridge.add_validator(accounts.alice).expect("add alice");
        bridge.add_validator(accounts.bob).expect("add bob");
        bridge.add_validator(accounts.charlie).expect("add charlie");

        let request_id = bridge
            .initiate_bridge_multisig(1, 2, accounts.django, 2, Some(50), make_metadata())
            .expect("initiate request");

        test::set_caller::<DefaultEnvironment>(accounts.alice);
        bridge
            .sign_bridge_request(request_id, true)
            .expect("alice signs");

        test::set_caller::<DefaultEnvironment>(accounts.bob);
        bridge
            .sign_bridge_request(request_id, true)
            .expect("bob signs");

        let bitmap = bridge
            .get_signature_bitmap(request_id)
            .expect("bitmap query");
        let signers = bridge.get_signer_list(request_id).expect("signer list");

        assert_eq!(count_bitmap_bits(&bitmap), 2);
        assert_eq!(signers, vec![accounts.alice, accounts.bob]);
    }

    #[ink::test]
    fn bitmap_signature_count_matches_monitoring_count() {
        let mut bridge = setup_bridge();
        let accounts = test::default_accounts::<DefaultEnvironment>();

        bridge.add_validator(accounts.alice).expect("add alice");
        bridge.add_validator(accounts.bob).expect("add bob");
        bridge.add_validator(accounts.charlie).expect("add charlie");

        let request_id = bridge
            .initiate_bridge_multisig(7, 2, accounts.eve, 3, Some(50), make_metadata())
            .expect("initiate request");

        for signer in [accounts.alice, accounts.bob, accounts.charlie] {
            test::set_caller::<DefaultEnvironment>(signer);
            bridge
                .sign_bridge_request(request_id, true)
                .expect("validator signs");
        }

        let bitmap = bridge
            .get_signature_bitmap(request_id)
            .expect("bitmap query");
        let monitoring = bridge
            .monitor_bridge_status(request_id)
            .expect("monitoring query");

        assert_eq!(count_bitmap_bits(&bitmap), 3);
        assert_eq!(monitoring.signatures_collected, 3);
    }

    #[ink::test]
    fn legacy_signature_format_decodes_and_remains_readable() {
        let accounts = test::default_accounts::<DefaultEnvironment>();
        let legacy = LegacyStoredBridgeRequest {
            request_id: 9,
            token_id: 11,
            source_chain: 1,
            destination_chain: 2,
            sender: accounts.alice,
            recipient: accounts.bob,
            required_signatures: 2,
            signatures: vec![accounts.alice, accounts.charlie],
            created_at: 1,
            expires_at: Some(99),
            status: BridgeOperationStatus::Pending,
            multi_hop_status: MultiHopStatus::InProgress,
            route: vec![2, 3],
            current_hop: 0,
            total_gas_estimate: 123,
            metadata: make_metadata(),
        };

        let mut encoded = &legacy.encode()[..];
        let decoded = StoredBridgeRequest::decode(&mut encoded).expect("legacy decode");

        match decoded.signature_storage {
            SignatureStorage::Legacy(signers) => {
                assert_eq!(signers, vec![accounts.alice, accounts.charlie]);
            }
            SignatureStorage::Bitmap(_) => panic!("legacy decode should preserve signer list"),
        }
    }

    #[ink::test]
    fn bitmap_encoding_is_smaller_for_twenty_four_signatures() {
        let signers: Vec<AccountId> = (0u8..24)
            .map(|value| AccountId::from([value; 32]))
            .collect();

        let legacy = LegacyStoredBridgeRequest {
            request_id: 42,
            token_id: 77,
            source_chain: 1,
            destination_chain: 2,
            sender: signers[0],
            recipient: signers[1],
            required_signatures: 20,
            signatures: signers.clone(),
            created_at: 1,
            expires_at: Some(50),
            status: BridgeOperationStatus::Locked,
            multi_hop_status: MultiHopStatus::InProgress,
            route: Vec::new(),
            current_hop: 0,
            total_gas_estimate: 0,
            metadata: make_metadata(),
        };

        let mut bitmap = [0u8; SIGNATURE_BITMAP_BYTES];
        for bit in 0u8..24 {
            let byte_index = (bit / 8) as usize;
            bitmap[byte_index] |= 1u8 << (bit % 8);
        }
        let optimized = StoredBridgeRequest {
            request_id: 42,
            token_id: 77,
            source_chain: 1,
            destination_chain: 2,
            sender: signers[0],
            recipient: signers[1],
            required_signatures: 20,
            signature_storage: SignatureStorage::Bitmap(bitmap),
            created_at: 1,
            expires_at: Some(50),
            status: BridgeOperationStatus::Locked,
            multi_hop_status: MultiHopStatus::InProgress,
            route: Vec::new(),
            current_hop: 0,
            total_gas_estimate: 0,
            metadata: make_metadata(),
        };

        let legacy_bytes = legacy.encode().len();
        let optimized_bytes = optimized.encode().len();

        assert!(
            optimized_bytes < legacy_bytes,
            "bitmap encoding should be smaller than legacy vec encoding"
        );

        println!(
            "legacy_bytes={legacy_bytes}, bitmap_bytes={optimized_bytes}, saved={}",
            legacy_bytes.saturating_sub(optimized_bytes)
        );
    }

    // ── Travel rule (FATF) tests ─────────────────────────────────────────

    fn make_travel_rule_data(accounts: &ink::env::test::DefaultAccounts<DefaultEnvironment>) -> TravelRuleData {
        TravelRuleData {
            originator_name: b"Alice Smith".to_vec(),
            originator_account: accounts.alice,
            beneficiary_name: b"Bob Jones".to_vec(),
            beneficiary_account: accounts.bob,
            transfer_amount: 2_000_000,
            data_hash: [0xAB; 32],
            submitted_at: 0,
        }
    }

    fn setup_bridge_with_locked_request(high_value: bool) -> (PropertyBridge, u64, ink::env::test::DefaultAccounts<DefaultEnvironment>) {
        let mut bridge = setup_bridge();
        let accounts = test::default_accounts::<DefaultEnvironment>();
        test::set_caller::<DefaultEnvironment>(accounts.alice);

        bridge.add_validator(accounts.alice).unwrap();
        bridge.add_validator(accounts.bob).unwrap();
        bridge.add_bridge_operator(accounts.alice).unwrap();
        bridge.add_bridge_operator(accounts.bob).unwrap();

        let valuation = if high_value { 2_000_000 } else { 500 };
        let metadata = PropertyMetadata {
            location: String::from("Test Property"),
            size: 1000,
            legal_description: String::from("Test"),
            valuation,
            documents_url: String::from("ipfs://test"),
        };

        let request_id = bridge
            .initiate_bridge_multisig(1, 2, accounts.bob, 2, Some(100), metadata)
            .unwrap();

        // Collect 2 signatures to reach Locked state
        test::set_caller::<DefaultEnvironment>(accounts.alice);
        bridge.sign_bridge_request(request_id, true).unwrap();
        test::set_caller::<DefaultEnvironment>(accounts.bob);
        bridge.sign_bridge_request(request_id, true).unwrap();

        (bridge, request_id, accounts)
    }

    #[ink::test]
    fn test_execute_bridge_fails_without_travel_rule_data() {
        let (mut bridge, request_id, accounts) =
            setup_bridge_with_locked_request(true);

        // Set threshold below the valuation (1_000_000 < 2_000_000)
        test::set_caller::<DefaultEnvironment>(accounts.alice);
        bridge.set_travel_rule_threshold(2, 1_000_000).unwrap();

        // Execution should be blocked: travel rule data not submitted
        test::set_caller::<DefaultEnvironment>(accounts.alice);
        let result = bridge.execute_bridge(request_id);
        assert_eq!(result, Err(Error::TravelRuleDataRequired));
    }

    #[ink::test]
    fn test_execute_bridge_succeeds_after_travel_rule_data_submission() {
        let (mut bridge, request_id, accounts) =
            setup_bridge_with_locked_request(true);

        // Set threshold below the valuation
        test::set_caller::<DefaultEnvironment>(accounts.alice);
        bridge.set_travel_rule_threshold(2, 1_000_000).unwrap();

        // Submit travel rule data as the originator
        let data = make_travel_rule_data(&accounts);
        bridge.submit_travel_rule_data(request_id, data).unwrap();

        // Verify data is retrievable
        let stored = bridge.get_travel_rule_data(request_id);
        assert!(stored.is_some());
        assert_eq!(stored.unwrap().data_hash, [0xAB; 32]);

        // Now execution should succeed
        let result = bridge.execute_bridge(request_id);
        assert!(result.is_ok(), "bridge execution should succeed after travel rule data is submitted");
    }

    // Issue #736 acceptance: decoding a >1kB SCALE payload through the
    // legacy-detecting wrapper succeeds in a single buffered drain
    // (linear-time vs. the previous O(n^2) byte-by-byte loop).
    #[ink::test]
    fn decode_stored_bridge_request_drains_above_one_kilobyte_linearly() {
        // 128 ChainId entries in `route` = 1024 bytes for that field alone;
        // combined with the rest of the SCALE-encoded V2 layout the payload
        // comfortably exceeds 1024 bytes.
        let mut route: Vec<u64> = Vec::new();
        for i in 0u64..128u64 {
            route.push(1000u64 + i);
        }
        let v2 = StoredBridgeRequestV2 {
            request_id: 1,
            token_id: 2,
            source_chain: 3,
            destination_chain: 4,
            sender: AccountId::from([0xab; 32]),
            recipient: AccountId::from([0xcd; 32]),
            required_signatures: 1,
            signature_storage: SignatureStorage::Bitmap([0u8; SIGNATURE_BITMAP_BYTES]),
            created_at: 7,
            expires_at: Some(8),
            status: BridgeOperationStatus::Pending,
            multi_hop_status: MultiHopStatus::InProgress,
            route,
            current_hop: 0,
            total_gas_estimate: 100,
            metadata: PropertyMetadata {
                location: String::from("LinearDecodeAcceptance"),
                size: 0,
                legal_description: String::from("n/a"),
                valuation: 0,
                documents_url: String::from("ipfs://linear"),
            },
        };

        let encoded = v2.encode();
        assert!(
            encoded.len() >= 1024,
            "test fixture should exceed 1kB; got {} bytes",
            encoded.len()
        );

        let decoded =
            <StoredBridgeRequest as Decode>::decode(&mut &encoded[..])
                .expect("linear decode of >1kB payload should succeed");
        assert_eq!(decoded.request_id, 1);
        assert_eq!(decoded.token_id, 2);
        assert_eq!(decoded.route.len(), 128);
    }

    // ── #764: Daily volume read-only metrics ──────────────────────────────

    #[ink::test]
    fn test_get_daily_volume_non_admin_rejected() {
        let bridge = setup_bridge();
        let accounts = test::default_accounts::<DefaultEnvironment>();

        // Bob is not admin — get_daily_volume should be rejected
        test::set_caller::<DefaultEnvironment>(accounts.bob);
        let result = bridge.get_daily_volume(1);
        assert_eq!(result, Err(Error::Unauthorized));

        // get_account_daily_volume should also be rejected
        let result = bridge.get_account_daily_volume(accounts.bob);
        assert_eq!(result, Err(Error::Unauthorized));
    }

    #[ink::test]
    fn test_get_daily_volume_returns_zero_when_no_trades() {
        let bridge = setup_bridge();
        let accounts = test::default_accounts::<DefaultEnvironment>();
        test::set_caller::<DefaultEnvironment>(accounts.alice);

        // No trades yet – volume should be zero
        let volume = bridge.get_daily_volume(1).expect("admin query");
        assert_eq!(volume, 0);

        let account_vol = bridge
            .get_account_daily_volume(accounts.bob)
            .expect("admin query");
        assert_eq!(account_vol, 0);
    }

    #[ink::test]
    fn test_daily_volume_tracked_via_cross_chain_trade() {
        let mut bridge = setup_bridge();
        let accounts = test::default_accounts::<DefaultEnvironment>();

        // Bob registers a cross-chain trade with amount_in = 100_000
        test::set_caller::<DefaultEnvironment>(accounts.bob);
        bridge
            .register_cross_chain_trade(1, None, 2, accounts.charlie, 100_000, 95_000)
            .expect("register cross-chain trade");

        // Alice queries the chain volume as admin
        test::set_caller::<DefaultEnvironment>(accounts.alice);
        let chain_vol = bridge.get_daily_volume(2).expect("admin get daily volume");
        assert_eq!(
            chain_vol, 100_000,
            "chain daily volume should reflect the trade amount"
        );

        // Bob's account-level volume should also reflect the trade
        let account_vol = bridge
            .get_account_daily_volume(accounts.bob)
            .expect("admin get account daily volume");
        assert_eq!(
            account_vol, 100_000,
            "account daily volume should reflect bob's trade amount"
        );
    }

    #[ink::test]
    fn test_daily_volume_accumulates_multiple_trades() {
        let mut bridge = setup_bridge();
        let accounts = test::default_accounts::<DefaultEnvironment>();

        // Bob does two trades
        test::set_caller::<DefaultEnvironment>(accounts.bob);
        bridge
            .register_cross_chain_trade(1, None, 2, accounts.charlie, 50_000, 47_000)
            .expect("first trade");
        bridge
            .register_cross_chain_trade(2, None, 2, accounts.charlie, 75_000, 70_000)
            .expect("second trade");

        // Alice: chain daily volume should be 125_000
        test::set_caller::<DefaultEnvironment>(accounts.alice);
        let chain_vol = bridge.get_daily_volume(2).expect("admin query");
        assert_eq!(chain_vol, 125_000);

        let account_vol = bridge
            .get_account_daily_volume(accounts.bob)
            .expect("admin query");
        assert_eq!(account_vol, 125_000);
    }

    #[ink::test]
    fn test_daily_volume_rollover_at_midnight_utc() {
        // This test simulates midnight UTC rollover by directly setting the
        // block timestamp before and after the day boundary.
        //
        // The day counter is computed as `block_timestamp / 86_400_000`
        // (milliseconds → UTC day). We set the timestamp to day N, track
        // volume, then advance to day N+1 and verify the volume resets to 0.

        let mut bridge = setup_bridge();
        let accounts = test::default_accounts::<DefaultEnvironment>();

        // Anchor to a known UTC day boundary:
        // Day 20000 starts at timestamp 20000 * 86_400_000 ms
        let day_start = 20000u64 * 86_400_000;

        // Set timestamp to the middle of day 20000
        ink::env::test::set_block_timestamp::<DefaultEnvironment>(
            day_start + 43_200_000,
        );

        // Bob registers a trade on day 20000
        test::set_caller::<DefaultEnvironment>(accounts.bob);
        bridge
            .register_cross_chain_trade(1, None, 2, accounts.charlie, 200_000, 190_000)
            .expect("register trade on day 20000");

        // Verify volume recorded on day 20000
        test::set_caller::<DefaultEnvironment>(accounts.alice);
        let vol_day1 = bridge.get_daily_volume(2).expect("admin query day 20000");
        assert_eq!(vol_day1, 200_000, "volume should be recorded on day 20000");

        let acct_vol_day1 = bridge
            .get_account_daily_volume(accounts.bob)
            .expect("admin query day 20000");
        assert_eq!(acct_vol_day1, 200_000, "account volume on day 20000");

        // ── Rollover to midnight UTC (day 20001) ───────────────────────
        ink::env::test::set_block_timestamp::<DefaultEnvironment>(
            20001u64 * 86_400_000 + 1, // just after midnight
        );

        // Volume should now be zero on the new day
        let vol_day2 = bridge.get_daily_volume(2).expect("admin query day 20001");
        assert_eq!(
            vol_day2, 0,
            "chain daily volume must roll over to 0 at midnight UTC"
        );

        let acct_vol_day2 = bridge
            .get_account_daily_volume(accounts.bob)
            .expect("admin query day 20001");
        assert_eq!(
            acct_vol_day2, 0,
            "account daily volume must roll over to 0 at midnight UTC"
        );

        // A new trade on day 20001 should start recording fresh volume
        test::set_caller::<DefaultEnvironment>(accounts.bob);
        bridge
            .register_cross_chain_trade(2, None, 2, accounts.charlie, 50_000, 47_000)
            .expect("register trade on day 20001");

        test::set_caller::<DefaultEnvironment>(accounts.alice);
        let vol_day2_new = bridge.get_daily_volume(2).expect("admin query after trade");
        assert_eq!(
            vol_day2_new, 50_000,
            "new trades on day 20001 should accumulate fresh volume"
        );

        let acct_vol_day2_new = bridge
            .get_account_daily_volume(accounts.bob)
            .expect("admin query after trade");
        assert_eq!(
            acct_vol_day2_new, 50_000,
            "new trades on day 20001 should accumulate fresh account volume"
        );
    }

    // =========================================================================
    // Overflow-safe time arithmetic (Issue #993)
    // =========================================================================

    fn sample_metadata(location: &str) -> PropertyMetadata {
        PropertyMetadata {
            location: String::from(location),
            size: 100,
            legal_description: String::from("Time safety"),
            valuation: 10_000,
            documents_url: String::from("ipfs://time"),
        }
    }

    fn multi_hop_metadata() -> PropertyMetadata {
        PropertyMetadata {
            location: String::from("Multi Hop Property"),
            size: 1000,
            legal_description: String::from("Test"),
            valuation: 100000,
            documents_url: String::from("ipfs://test"),
        }
    }

    #[ink::test]
    fn test_multisig_timeout_at_u64_max_does_not_wrap() {
        let mut bridge = setup_bridge();
        let accounts = test::default_accounts::<DefaultEnvironment>();
        test::set_caller::<DefaultEnvironment>(accounts.alice);
        bridge.add_validator(accounts.alice).expect("add validator");
        bridge.add_validator(accounts.bob).expect("add validator");

        // A caller-supplied timeout of u64::MAX must not wrap expires_at
        // into the past (which would expire the request on arrival).
        let request_id = bridge
            .initiate_bridge_multisig(
                1,
                2,
                accounts.bob,
                2,
                Some(u64::MAX),
                sample_metadata("Max Timeout"),
            )
            .expect("initiate with max timeout should not overflow");

        // The request must still be signable: an expired request would
        // return RequestExpired here.
        let signed = bridge.sign_bridge_request(request_id, true);
        assert!(
            signed.is_ok(),
            "request with saturated expiry must not be treated as expired"
        );

        test::set_caller::<DefaultEnvironment>(accounts.bob);
        bridge
            .sign_bridge_request(request_id, true)
            .expect("second signature");

        test::set_caller::<DefaultEnvironment>(accounts.alice);
        bridge
            .execute_bridge(request_id)
            .expect("request with max timeout executes normally");
    }

    #[ink::test]
    fn test_emergency_request_timeout_at_u64_max_saturates() {
        let mut bridge = setup_bridge();
        let accounts = test::default_accounts::<DefaultEnvironment>();
        test::set_caller::<DefaultEnvironment>(accounts.alice);
        bridge
            .add_emergency_signer(accounts.bob)
            .expect("add emergency signer");
        bridge
            .add_emergency_signer(accounts.charlie)
            .expect("add emergency signer");
        bridge.set_emergency_threshold(2).expect("threshold");

        test::set_caller::<DefaultEnvironment>(accounts.bob);
        let pause_flags = propchain_traits::PauseFlags {
            all_operations: false,
            new_requests: true,
            signing: false,
            execution: false,
            cross_chain_trades: false,
        };
        let request_id = bridge
            .propose_pause_bridge(
                pause_flags,
                propchain_traits::PauseReason::ManualAdmin,
                Some(String::from("max timeout")),
                Some(u64::MAX),
            )
            .expect("propose with max timeout should not overflow");

        // expires_at must clamp to u64::MAX instead of wrapping below the
        // creation block.
        let request = bridge
            .get_emergency_request(request_id)
            .expect("emergency request stored");
        assert_eq!(request.expires_at, Some(u64::MAX));
    }

    #[ink::test]
    fn test_rate_window_survives_u64_max_duration() {
        let mut bridge = setup_bridge();
        let accounts = test::default_accounts::<DefaultEnvironment>();
        test::set_caller::<DefaultEnvironment>(accounts.alice);
        bridge.add_validator(accounts.alice).expect("add validator");
        bridge.add_validator(accounts.bob).expect("add validator");

        // A u64::MAX window duration makes `window_start + duration`
        // overflow; saturating arithmetic must keep the window open.
        bridge
            .configure_batch_verification(10, u64::MAX)
            .expect("configure batch verification");

        test::set_block_timestamp::<DefaultEnvironment>(1_000);

        let first = bridge
            .initiate_bridge_multisig(
                1,
                2,
                accounts.bob,
                2,
                Some(50),
                sample_metadata("Window One"),
            )
            .expect("first initiate");
        test::set_caller::<DefaultEnvironment>(accounts.alice);
        bridge.sign_bridge_request(first, true).expect("sign");
        test::set_caller::<DefaultEnvironment>(accounts.bob);
        bridge.sign_bridge_request(first, true).expect("sign");
        test::set_caller::<DefaultEnvironment>(accounts.alice);
        bridge.execute_bridge(first).expect("execute");

        // Move far past any wrapped bound: pre-fix this would have been
        // >= window_start + duration (wrapped) and opened a fresh window.
        test::set_block_timestamp::<DefaultEnvironment>(5_000);

        test::set_caller::<DefaultEnvironment>(accounts.alice);
        let second = bridge
            .initiate_bridge_multisig(
                2,
                3,
                accounts.charlie,
                2,
                Some(50),
                sample_metadata("Window Two"),
            )
            .expect("second initiate");
        bridge.sign_bridge_request(second, true).expect("sign");
        test::set_caller::<DefaultEnvironment>(accounts.bob);
        bridge.sign_bridge_request(second, true).expect("sign");
        test::set_caller::<DefaultEnvironment>(accounts.alice);
        bridge.execute_bridge(second).expect("execute second");

        let history = bridge.get_bridge_history(accounts.alice, 0, 100);
        assert!(history.len() >= 2, "both executions recorded");
        let (_, window_one) = bridge
            .get_transaction_batch(history[0].transaction_hash)
            .expect("first tx batched");
        let (_, window_two) = bridge
            .get_transaction_batch(history[1].transaction_hash)
            .expect("second tx batched");

        assert_eq!(
            window_one, window_two,
            "with a u64::MAX duration both transactions share one batch window"
        );
    }

    #[ink::test]
    fn test_multi_hop_empty_route_rejected_without_panic() {
        let mut bridge = setup_bridge();
        let accounts = test::default_accounts::<DefaultEnvironment>();
        test::set_caller::<DefaultEnvironment>(accounts.alice);

        let result =
            bridge.initiate_multi_hop_bridge(1, Vec::new(), accounts.bob, 2, Some(50), multi_hop_metadata());
        assert_eq!(result.unwrap_err(), Error::InvalidChain);
    }

    #[ink::test]
    fn test_multi_hop_single_hop_route_rejected() {
        let mut bridge = setup_bridge();
        let accounts = test::default_accounts::<DefaultEnvironment>();
        test::set_caller::<DefaultEnvironment>(accounts.alice);

        let result = bridge.initiate_multi_hop_bridge(
            1,
            vec![2],
            accounts.bob,
            2,
            Some(50),
            multi_hop_metadata(),
        );
        assert_eq!(result.unwrap_err(), Error::InvalidChain);
    }

    #[ink::test]
    fn test_multi_hop_valid_route_initiates_request() {
        let mut bridge = setup_bridge();
        let accounts = test::default_accounts::<DefaultEnvironment>();
        test::set_caller::<DefaultEnvironment>(accounts.alice);

        // Route must start away from the current chain (1) and stay supported
        let result = bridge.initiate_multi_hop_bridge(
            1,
            vec![2, 3],
            accounts.bob,
            2,
            Some(50),
            multi_hop_metadata(),
        );
        let request_id = result.expect("valid two-hop route should be accepted");
        assert!(request_id > 0);
        assert_eq!(
            bridge.get_multi_hop_status(request_id).unwrap(),
            MultiHopStatus::InProgress
        );
    }

    // ── Adoption tests: rate-limit config (#1107) ────────────────────────

    fn new_rate_cfg(max_requests: u64) -> crate::rate_limit_config::RateLimitConfig {
        crate::rate_limit_config::RateLimitConfig {
            window_seconds: 86_400,
            max_requests_per_day: max_requests,
            max_value_per_day: 1_000_000_000_000_000_000,
        }
    }

    #[ink::test]
    fn rate_limit_config_is_admin_settable_and_enforced() {
        let mut bridge = setup_bridge();
        let accounts = test::default_accounts::<DefaultEnvironment>();

        // Non-admin cannot change the rate-limit config.
        test::set_caller::<DefaultEnvironment>(accounts.bob);
        assert_eq!(
            bridge.set_rate_limit_config(new_rate_cfg(1)),
            Err(Error::Unauthorized)
        );

        // Admin lowers the per-window request cap to 1.
        test::set_caller::<DefaultEnvironment>(accounts.alice);
        bridge.set_rate_limit_config(new_rate_cfg(1)).expect("admin can update");
        assert_eq!(
            bridge.get_rate_limit_config().max_requests_per_day,
            1
        );

        let metadata = PropertyMetadata {
            location: String::from("Test"),
    // ── Granular pause tests (Issue #1112) ────────────────────────────────
    //
    // Each test pauses ONE BridgeOperation and proves the matching message
    // reverts with OperationPaused while unrelated operations keep working,
    // then resumes and shows service is restored.

    fn metadata() -> PropertyMetadata {
        PropertyMetadata {
            location: String::from("Test Property"),
            size: 1000,
            legal_description: String::from("Test"),
            valuation: 100000,
            documents_url: String::from("ipfs://test"),
        };

        // First request within the window is allowed.
        let first = bridge.initiate_bridge_multisig(1, 2, accounts.bob, 2, Some(50), metadata.clone());
        assert!(first.is_ok());

        // Second request from the same account is rate-limited.
        let second = bridge.initiate_bridge_multisig(1, 2, accounts.bob, 2, Some(50), metadata);
        assert_eq!(second, Err(Error::RateLimitExceeded));
    }

    #[ink::test]
    fn rate_limit_config_rejects_invalid_values() {
        let mut bridge = setup_bridge();
        let accounts = test::default_accounts::<DefaultEnvironment>();
        test::set_caller::<DefaultEnvironment>(accounts.alice);

        let bad = crate::rate_limit_config::RateLimitConfig {
            max_requests_per_day: 0,
            ..crate::rate_limit_config::RateLimitConfig::default()
        };
        assert_eq!(bridge.set_rate_limit_config(bad), Err(Error::InvalidRateLimit));
        // Config unchanged on rejection.
        assert_eq!(bridge.get_rate_limit_config().max_requests_per_day, 10);
    }

    // ── Adoption tests: validator staking (#1109) ─────────────────────────

    #[ink::test]
    fn stake_validator_requires_registered_validator() {
        let mut bridge = setup_bridge();
        let accounts = test::default_accounts::<DefaultEnvironment>();

        test::set_caller::<DefaultEnvironment>(accounts.alice);
        assert_eq!(bridge.stake_validator(10_000_000), Err(Error::Unauthorized));

        bridge.add_validator(accounts.alice).expect("add validator");
        bridge.stake_validator(10_000_000).expect("stake as validator");
        assert_eq!(bridge.get_validator_stake(accounts.alice), 10_000_000);
        assert_eq!(bridge.get_total_staked(), 10_000_000);
        assert_eq!(bridge.get_slash_pool(), 0);

        // Below-minimum stake is rejected and the ledger is unchanged.
        assert_eq!(bridge.stake_validator(1), Err(Error::InvalidRequest));
        assert_eq!(bridge.get_total_staked(), 10_000_000);
    }

    #[ink::test]
    fn vote_conflict_slashes_the_signers_stake() {
        let mut bridge = setup_bridge();
        let accounts = test::default_accounts::<DefaultEnvironment>();

        test::set_caller::<DefaultEnvironment>(accounts.alice);
        bridge.add_validator(accounts.alice).expect("add validator");

        let metadata = PropertyMetadata {
            location: String::from("Test"),
            size: 1000,
            legal_description: String::from("Test"),
            valuation: 500,
            documents_url: String::from("ipfs://test"),
        };
        let request_id = bridge
            .initiate_bridge_multisig(1, 2, accounts.bob, 2, Some(100), metadata)
            .expect("initiate");

        bridge.stake_validator(100_000_000).expect("stake validator");

        // Approve, then flip to decline on the same request.
        test::set_caller::<DefaultEnvironment>(accounts.alice);
        bridge.sign_bridge_request(request_id, true).expect("approve");
        let conflict = bridge.sign_bridge_request(request_id, false);
        assert_eq!(conflict, Err(Error::VoteConflict));

        // 20% of 100_000_000 slashed into the pool; stake reduced.
        assert_eq!(bridge.get_validator_stake(accounts.alice), 80_000_000);
        assert_eq!(bridge.get_slash_pool(), 20_000_000);
    }

    #[ink::test]
    fn approvals_need_staked_weight_quorum_not_bare_count() {
        let mut bridge = setup_bridge();
        let accounts = test::default_accounts::<DefaultEnvironment>();

        test::set_caller::<DefaultEnvironment>(accounts.alice);
        bridge.add_validator(accounts.alice).expect("validator alice");
        bridge.add_validator(accounts.bob).expect("validator bob");
        bridge.add_validator(accounts.charlie).expect("validator charlie");
        bridge.add_bridge_operator(accounts.alice).expect("operator alice");

        let metadata = PropertyMetadata {
            location: String::from("Test"),
            size: 1000,
            legal_description: String::from("Test"),
            valuation: 1000,
            documents_url: String::from("ipfs://test"),
        };
        let request_id = bridge
            .initiate_bridge_multisig(1, 2, accounts.bob, 2, Some(100), metadata)
            .expect("initiate");

        // Stake makes the required signatures insufficient on their own:
        // alice(10M) + bob(10M) + charlie(100M) -> total 120M, quorum 72M.
        test::set_caller::<DefaultEnvironment>(accounts.alice);
        bridge.stake_validator(10_000_000).expect("stake alice");
        test::set_caller::<DefaultEnvironment>(accounts.bob);
        bridge.stake_validator(10_000_000).expect("stake bob");
        test::set_caller::<DefaultEnvironment>(accounts.charlie);
        bridge.stake_validator(100_000_000).expect("stake charlie");

        // Two signatures hit the bare required count (2) but only carry 20M
        // of weight — request must NOT lock.
        test::set_caller::<DefaultEnvironment>(accounts.alice);
        bridge.sign_bridge_request(request_id, true).expect("alice signs");
        test::set_caller::<DefaultEnvironment>(accounts.bob);
        bridge.sign_bridge_request(request_id, true).expect("bob signs");

        test::set_caller::<DefaultEnvironment>(accounts.alice);
        assert_eq!(bridge.execute_bridge(request_id), Err(Error::InvalidRequest));

        // Adding charlie's (heavy) signature crosses the quorum and locks.
        test::set_caller::<DefaultEnvironment>(accounts.charlie);
        bridge.sign_bridge_request(request_id, true).expect("charlie signs");

        test::set_caller::<DefaultEnvironment>(accounts.alice);
        bridge.execute_bridge(request_id).expect("locked with quorum");
    }

    #[ink::test]
    fn slash_validator_is_admin_only() {
        let mut bridge = setup_bridge();
        let accounts = test::default_accounts::<DefaultEnvironment>();

        test::set_caller::<DefaultEnvironment>(accounts.alice);
        bridge.add_validator(accounts.alice).expect("add validator");
        bridge.add_validator(accounts.bob).expect("add validator bob");
        bridge.stake_validator(10_000_000).expect("stake alice");
        bridge.stake_validator(10_000_000).expect("stake alice again");
        assert_eq!(bridge.get_validator_stake(accounts.alice), 20_000_000);

        // Non-admin is rejected.
        test::set_caller::<DefaultEnvironment>(accounts.bob);
        assert_eq!(
            bridge.slash_validator(accounts.alice),
            Err(Error::Unauthorized)
        );

        test::set_caller::<DefaultEnvironment>(accounts.alice);
        let penalty = bridge.slash_validator(accounts.alice).expect("admin slashes");
        assert_eq!(penalty, 4_000_000);
        assert_eq!(bridge.get_validator_stake(accounts.alice), 16_000_000);
        assert_eq!(bridge.get_slash_pool(), 4_000_000);
    }

    // ── Adoption tests: bounded modules (#1110) ───────────────────────────

    #[ink::test]
    fn audit_log_is_bounded_and_queryable() {
        let mut bridge = setup_bridge();
        let accounts = test::default_accounts::<DefaultEnvironment>();

        test::set_caller::<DefaultEnvironment>(accounts.alice);
        bridge.set_emergency_pause(true).expect("pause");
        bridge.set_emergency_pause(false).expect("unpause");

        let logs = bridge.get_audit_logs();
        assert_eq!(logs.len(), 2);
        assert!(logs[0].paused);
        assert!(!logs[1].paused);
    }

    #[ink::test]
    fn validator_registry_is_capped_by_bitmap_proof() {
        let mut bridge = setup_bridge();
        let accounts = test::default_accounts::<DefaultEnvironment>();

        test::set_caller::<DefaultEnvironment>(accounts.alice);
        for i in 0..100 {
            let mut bytes = [0u8; 32];
            bytes[31] = (i + 1) as u8;
            bridge
                .add_validator(AccountId::from(bytes))
                .expect("validator added within cap");
        }
        assert_eq!(bridge.get_validators().len(), 100);

        // The 101st validator exceeds the proven 100-slot bitmap bound.
        let extra = AccountId::from([0xffu8; 32]);
        assert_eq!(bridge.add_validator(extra), Err(Error::InsufficientSignatures));
        assert_eq!(bridge.get_validators().len(), 100);
        }
    }

    fn flags_with(cross_chain_trades: bool) -> propchain_traits::PauseFlags {
        propchain_traits::PauseFlags {
            all_operations: false,
            new_requests: false,
            signing: false,
            execution: false,
            cross_chain_trades,
        }
    }

    #[ink::test]
    fn test_granular_pause_new_requests_blocks_only_new_requests() {
        let mut bridge = setup_bridge();
        let accounts = test::default_accounts::<DefaultEnvironment>();
        test::set_caller::<DefaultEnvironment>(accounts.alice);

        bridge
            .emergency_pause(
                propchain_traits::PauseFlags {
                    new_requests: true,
                    ..flags_with(false)
                },
                propchain_traits::PauseReason::ManualAdmin,
                Some(String::from("granular pause test")),
            )
            .expect("admin can pause NewRequest");

        assert!(bridge.is_operation_paused(propchain_traits::BridgeOperation::NewRequest));
        assert!(!bridge.is_operation_paused(propchain_traits::BridgeOperation::CrossChainTrade));

        // NewRequest is gated...
        let blocked = bridge.initiate_bridge_multisig(1, 2, accounts.bob, 2, Some(50), metadata());
        assert_eq!(blocked, Err(Error::OperationPaused));

        // ...while an unrelated operation keeps succeeding.
        let trade_id = bridge
            .register_cross_chain_trade(9, Some(7), 2, accounts.charlie, 50_000, 49_000)
            .expect("unrelated cross-chain trade must still succeed while NewRequest is paused");
        assert!(trade_id > 0);

        // Resume restores service.
        bridge
            .emergency_unpause(propchain_traits::PauseFlags {
                new_requests: true,
                ..flags_with(false)
            })
            .expect("admin can resume NewRequest");
        assert!(!bridge.is_operation_paused(propchain_traits::BridgeOperation::NewRequest));
        let resumed = bridge.initiate_bridge_multisig(1, 2, accounts.bob, 2, Some(50), metadata());
        assert!(resumed.is_ok());
    }

    #[ink::test]
    fn test_granular_pause_signing_blocks_only_signing() {
        let mut bridge = setup_bridge();
        let accounts = test::default_accounts::<DefaultEnvironment>();
        test::set_caller::<DefaultEnvironment>(accounts.alice);
        bridge
            .add_validator(accounts.alice)
            .expect("admin can add validator");

        let request_id = bridge
            .initiate_bridge_multisig(1, 2, accounts.bob, 2, Some(50), metadata())
            .expect("request creation should succeed");

        bridge
            .emergency_pause(
                propchain_traits::PauseFlags {
                    signing: true,
                    ..flags_with(false)
                },
                propchain_traits::PauseReason::ManualAdmin,
                Some(String::from("granular pause test")),
            )
            .expect("admin can pause Signing");

        assert!(bridge.is_operation_paused(propchain_traits::BridgeOperation::Signing));
        assert!(!bridge.is_operation_paused(propchain_traits::BridgeOperation::NewRequest));
        assert!(!bridge.is_operation_paused(propchain_traits::BridgeOperation::Execution));

        // Signing is gated...
        let blocked = bridge.sign_bridge_request(request_id, true);
        assert_eq!(blocked, Err(Error::OperationPaused));

        // ...while request creation keeps working.
        let other = bridge.initiate_bridge_multisig(2, 3, accounts.charlie, 2, Some(50), metadata());
        assert!(other.is_ok());

        // Resume restores service.
        bridge
            .emergency_unpause(propchain_traits::PauseFlags {
                signing: true,
                ..flags_with(false)
            })
            .expect("admin can resume Signing");
        assert!(!bridge.is_operation_paused(propchain_traits::BridgeOperation::Signing));
        let signed = bridge.sign_bridge_request(request_id, true);
        assert!(signed.is_ok());
    }

    #[ink::test]
    fn test_granular_pause_execution_blocks_only_execution() {
        let mut bridge = setup_bridge();
        let accounts = test::default_accounts::<DefaultEnvironment>();
        test::set_caller::<DefaultEnvironment>(accounts.alice);
        bridge
            .add_validator(accounts.alice)
            .expect("add validator alice");
        bridge
            .add_validator(accounts.bob)
            .expect("add validator bob");
        bridge
            .add_bridge_operator(accounts.bob)
            .expect("add operator bob");

        let request_id = bridge
            .initiate_bridge_multisig(1, 2, accounts.charlie, 2, Some(50), metadata())
            .expect("initiation should succeed");

        test::set_caller::<DefaultEnvironment>(accounts.alice);
        bridge
            .sign_bridge_request(request_id, true)
            .expect("alice signs");
        test::set_caller::<DefaultEnvironment>(accounts.bob);
        bridge
            .sign_bridge_request(request_id, true)
            .expect("bob signs");

        test::set_caller::<DefaultEnvironment>(accounts.alice);
        bridge
            .emergency_pause(
                propchain_traits::PauseFlags {
                    execution: true,
                    ..flags_with(false)
                },
                propchain_traits::PauseReason::ManualAdmin,
                Some(String::from("granular pause test")),
            )
            .expect("admin can pause Execution");

        assert!(bridge.is_operation_paused(propchain_traits::BridgeOperation::Execution));
        assert!(!bridge.is_operation_paused(propchain_traits::BridgeOperation::Signing));

        // Fully-signed request is still gated at execution...
        let blocked = bridge.execute_bridge(request_id);
        assert_eq!(blocked, Err(Error::OperationPaused));

        // ...while unrelated operations keep working.
        let trade_id = bridge
            .register_cross_chain_trade(9, Some(7), 2, accounts.charlie, 50_000, 49_000)
            .expect("cross-chain trade must still succeed while Execution is paused");
        assert!(trade_id > 0);

        // Resume restores service.
        bridge
            .emergency_unpause(propchain_traits::PauseFlags {
                execution: true,
                ..flags_with(false)
            })
            .expect("admin can resume Execution");
        assert!(!bridge.is_operation_paused(propchain_traits::BridgeOperation::Execution));
        let executed = bridge.execute_bridge(request_id);
        assert!(executed.is_ok());
    }

    #[ink::test]
    fn test_granular_pause_cross_chain_blocks_only_cross_chain() {
        let mut bridge = setup_bridge();
        let accounts = test::default_accounts::<DefaultEnvironment>();
        test::set_caller::<DefaultEnvironment>(accounts.alice);

        bridge
            .emergency_pause(
                flags_with(true),
                propchain_traits::PauseReason::ManualAdmin,
                Some(String::from("granular pause test")),
            )
            .expect("admin can pause CrossChainTrade");

        assert!(bridge.is_operation_paused(propchain_traits::BridgeOperation::CrossChainTrade));
        assert!(!bridge.is_operation_paused(propchain_traits::BridgeOperation::NewRequest));

        let blocked = bridge.register_cross_chain_trade(9, Some(7), 2, accounts.charlie, 50_000, 49_000);
        assert_eq!(blocked, Err(Error::OperationPaused));

        let other = bridge.initiate_bridge_multisig(1, 2, accounts.bob, 2, Some(50), metadata());
        assert!(other.is_ok());

        bridge
            .emergency_unpause(flags_with(true))
            .expect("admin can resume CrossChainTrade");
        assert!(!bridge.is_operation_paused(propchain_traits::BridgeOperation::CrossChainTrade));
        let trade_id = bridge
            .register_cross_chain_trade(9, Some(7), 2, accounts.charlie, 50_000, 49_000)
            .expect("cross-chain trade resumes after unpause");
        assert!(trade_id > 0);
    }

    // ── Signature payload binding tests (Issue #1111) ─────────────────────

    #[ink::test]
    fn test_signed_payload_covers_amount_source_and_destination() {
        let accounts = test::default_accounts::<DefaultEnvironment>();
        let caller = accounts.bob;

        let full = PropertyBridge::signature_binding_hash(42, true, caller, 7, 1000, 1, 2);

        // Binding only (request_id, approve, caller, block) as before is no
        // longer sufficient: an approval signing that tuple must be rejected
        // once the transfer payload exists.
        let old_shape = propchain_traits::crypto::hash_encoded(&(42u64, true, caller, 7u32));
        assert_ne!(full, old_shape, "payload must bind more than the original tuple");

        // Altering amount, source or destination must change the binding hash.
        assert_ne!(
            full,
            PropertyBridge::signature_binding_hash(42, true, caller, 7, 1001, 1, 2),
            "a different amount must produce a different binding"
        );
        assert_ne!(
            full,
            PropertyBridge::signature_binding_hash(42, true, caller, 7, 1000, 9, 2),
            "a different source chain must produce a different binding"
        );
        assert_ne!(
            full,
            PropertyBridge::signature_binding_hash(42, true, caller, 7, 1000, 1, 9),
            "a different destination chain must produce a different binding"
        );
    }

    #[ink::test]
    fn test_old_shape_approval_is_rejected() {
        let mut bridge = setup_bridge();
        let accounts = test::default_accounts::<DefaultEnvironment>();
        test::set_caller::<DefaultEnvironment>(accounts.alice);
        test::set_block_number::<DefaultEnvironment>(42);
        bridge
            .add_bridge_operator(accounts.alice)
            .expect("admin can add operator");

        let request_id = bridge
            .initiate_bridge_multisig(1, 2, accounts.bob, 2, Some(50), metadata())
            .expect("request creation should succeed");

        bridge
            .register_operator_public_key([7u8; 33])
            .expect("operator can register key");

        // Approval signs the OLD tuple (no amount/chains) — must be rejected
        // even before any signature math, because the stored request's amount
        // and chains are not covered.
        let old_shape_hash =
            <[u8; 32]>::from(propchain_traits::crypto::hash_encoded(&(
                request_id,
                true,
                accounts.alice,
                42u32,
            )));
        let approval = propchain_traits::SignedApproval {
            signature: [0u8; 65],
            message_hash: old_shape_hash,
        };
        let result = bridge.sign_bridge_request_with_signature(request_id, true, Some(approval));
        assert_eq!(result, Err(Error::Unauthorized));

        // An approval that IS bound to the stored amount/chains gets past the
        // binding check and is rejected only at signature verification.
        let bound_hash = <[u8; 32]>::from(PropertyBridge::signature_binding_hash(
            request_id,
            true,
            accounts.alice,
            42,
            100000,
            1,
            2,
        ));
        let approval = propchain_traits::SignedApproval {
            signature: [0u8; 65],
            message_hash: bound_hash,
        };
        let result = bridge.sign_bridge_request_with_signature(request_id, true, Some(approval));
        assert_eq!(result, Err(Error::Unauthorized));
    }

    // ── Issue #1106: TokenFreezeManager must be enforced at execution and
    //    unfreeze must clear the storage key ────────────────────────────────

    #[ink::test]
    fn test_frozen_token_blocks_execution_until_unfrozen() {
        let mut bridge = setup_bridge();
        let accounts = test::default_accounts::<DefaultEnvironment>();
        test::set_caller::<DefaultEnvironment>(accounts.alice);

        bridge
            .add_validator(accounts.alice)
            .expect("add validator");
        bridge.add_validator(accounts.bob).expect("add validator");
        bridge
            .add_bridge_operator(accounts.alice)
            .expect("add operator");

        let request_id = bridge
            .initiate_bridge_multisig(1, 2, accounts.bob, 2, Some(50), metadata())
            .expect("initiate before freeze");

        test::set_caller::<DefaultEnvironment>(accounts.alice);
        bridge
            .sign_bridge_request(request_id, true)
            .expect("alice signs");
        test::set_caller::<DefaultEnvironment>(accounts.bob);
        bridge
            .sign_bridge_request(request_id, true)
            .expect("bob signs");

        // Freeze the underlying token between signing and execution
        test::set_caller::<DefaultEnvironment>(accounts.alice);
        bridge.freeze_token(1).expect("admin freezes");

        let result = bridge.execute_bridge(request_id);
        assert_eq!(result, Err(Error::AssetAlreadyFrozen));
        assert!(
            bridge.get_bridge_history(accounts.alice, 0, 100).is_empty(),
            "a blocked execution must not be recorded in history"
        );

        // Unfreeze the token and execution proceeds cleanly
        bridge.unfreeze_token(1).expect("admin unfreezes");
        let result = bridge.execute_bridge(request_id);
        assert!(result.is_ok(), "execution proceeds after unfreeze");
        assert_eq!(bridge.get_bridge_history(accounts.alice, 0, 100).len(), 1);
    }

    #[ink::test]
    fn test_unfreeze_token_removes_storage_key() {
        let mut bridge = setup_bridge();
        let accounts = test::default_accounts::<DefaultEnvironment>();
        test::set_caller::<DefaultEnvironment>(accounts.alice);

        bridge.freeze_token(1).expect("freeze");
        assert!(bridge.is_token_frozen(1));
        assert!(bridge.frozen_tokens.get(1).is_some());

        bridge.unfreeze_token(1).expect("unfreeze");

        assert!(!bridge.is_token_frozen(1));
        assert!(
            bridge.frozen_tokens.get(1).is_none(),
            "unfreeze must remove the storage key, not write an explicit false"
        );

        // A re-freeze after unfreeze is a clean insert again.
        bridge.freeze_token(1).expect("refreeze");
        assert!(bridge.frozen_tokens.get(1).is_some());
    }

    // ── Issue #1105: recover_failed_bridge actions ─────────────────────────

    #[ink::test]
    fn test_recover_failed_bridge_refunds_gas_to_sender() {
        let mut bridge = setup_bridge();
        let accounts = test::default_accounts::<DefaultEnvironment>();
        test::set_caller::<DefaultEnvironment>(accounts.alice);

        bridge
            .add_validator(accounts.alice)
            .expect("add validator");

        let route = vec![2, 3];
        let request_id = bridge
            .initiate_multi_hop_bridge(1, route.clone(), accounts.bob, 2, Some(50), metadata())
            .expect("initiate multi-hop");
        let gas_estimate = bridge
            .estimate_multi_hop_bridge_gas(route.clone())
            .expect("gas estimate");
        assert!(gas_estimate > 0, "multi-hop carries a positive gas escrow");

        // Reject the request to move it into the Failed state.
        bridge
            .sign_bridge_request(request_id, false)
            .expect("reject vote");

        // Fund the contract so the escrow can actually be paid back.
        let callee = test::callee::<DefaultEnvironment>();
        let contract_before =
            test::get_account_balance::<DefaultEnvironment>(callee).unwrap_or_default();
        test::set_account_balance::<DefaultEnvironment>(callee, contract_before + u128::from(gas_estimate));

        let sender_before =
            test::get_account_balance::<DefaultEnvironment>(accounts.alice).unwrap_or_default();

        let result = bridge.recover_failed_bridge(request_id, RecoveryAction::RefundGas);
        assert_eq!(result, Ok(()));

        let sender_after =
            test::get_account_balance::<DefaultEnvironment>(accounts.alice).unwrap_or_default();
        assert_eq!(sender_after - sender_before, u128::from(gas_estimate));

        assert_eq!(
            bridge.get_multi_hop_status(request_id).expect("status"),
            MultiHopStatus::Failed
        );

        let events = test::recorded_events().collect::<Vec<_>>();
        let decoded = <BridgeRecovered as scale::Decode>::decode(
            &mut &events[events.len() - 1].data[..],
        )
        .expect("decode BridgeRecovered");
        assert_eq!(decoded.request_id, request_id);
        assert_eq!(decoded.recovery_action, RecoveryAction::RefundGas);
        assert_eq!(decoded.refunded_amount, u128::from(gas_estimate));
    }

    #[ink::test]
    fn test_recover_failed_bridge_unlock_requires_full_signer_set() {
        let mut bridge = setup_bridge();
        let accounts = test::default_accounts::<DefaultEnvironment>();
        test::set_caller::<DefaultEnvironment>(accounts.alice);

        bridge
            .add_validator(accounts.alice)
            .expect("add alice");
        bridge.add_validator(accounts.bob).expect("add bob");

        // required = 3 while only two signatures are ever collected: an
        // unlock must be rejected even though the request has already Failed.
        let request_id = bridge
            .initiate_bridge_multisig(1, 2, accounts.bob, 3, Some(50), metadata())
            .expect("initiate");

        test::set_caller::<DefaultEnvironment>(accounts.alice);
        bridge
            .sign_bridge_request(request_id, true)
            .expect("alice signs");
        test::set_caller::<DefaultEnvironment>(accounts.bob);
        bridge
            .sign_bridge_request(request_id, false)
            .expect("bob rejects");

        assert_eq!(
            bridge.recover_failed_bridge(request_id, RecoveryAction::UnlockToken),
            Err(Error::InsufficientSignatures)
        );
    }

    #[ink::test]
    fn test_recover_failed_bridge_unlock_succeeds_with_full_signer_set() {
        let mut bridge = setup_bridge();
        let accounts = test::default_accounts::<DefaultEnvironment>();
        test::set_caller::<DefaultEnvironment>(accounts.alice);

        bridge
            .add_validator(accounts.alice)
            .expect("add alice");
        bridge.add_validator(accounts.bob).expect("add bob");
        bridge
            .add_validator(accounts.charlie)
            .expect("add charlie");

        let request_id = bridge
            .initiate_bridge_multisig(1, 2, accounts.bob, 2, Some(50), metadata())
            .expect("initiate");

        // The full required signer set is collected (request Locked)...
        test::set_caller::<DefaultEnvironment>(accounts.alice);
        bridge
            .sign_bridge_request(request_id, true)
            .expect("alice signs");
        test::set_caller::<DefaultEnvironment>(accounts.bob);
        bridge
            .sign_bridge_request(request_id, true)
            .expect("bob signs");

        // ...then a later rejection vote moves it to Failed.
        test::set_caller::<DefaultEnvironment>(accounts.charlie);
        bridge
            .sign_bridge_request(request_id, false)
            .expect("charlie rejects");

        assert_eq!(
            bridge.recover_failed_bridge(request_id, RecoveryAction::UnlockToken),
            Ok(())
        );
        assert_eq!(
            bridge.get_multi_hop_status(request_id).expect("status"),
            MultiHopStatus::Failed
        );

        let events = test::recorded_events().collect::<Vec<_>>();
        let decoded = <BridgeRecovered as scale::Decode>::decode(
            &mut &events[events.len() - 1].data[..],
        )
        .expect("decode BridgeRecovered");
        assert_eq!(decoded.request_id, request_id);
        assert_eq!(decoded.recovery_action, RecoveryAction::UnlockToken);
        assert_eq!(decoded.refunded_amount, 0);
    }

    // ── Issue #1104: bounded, paginated bridge history ─────────────────────

    #[ink::test]
    fn test_bridge_history_is_capped_and_paginated() {
        let mut bridge = setup_bridge();
        let accounts = test::default_accounts::<DefaultEnvironment>();
        test::set_caller::<DefaultEnvironment>(accounts.alice);

        bridge
            .add_validator(accounts.alice)
            .expect("add alice");
        bridge.add_validator(accounts.bob).expect("add bob");
        bridge
            .add_bridge_operator(accounts.alice)
            .expect("add operator");

        // Raise the rate limits the loading loop would otherwise hit: the
        // default daily cap is 10 requests/account and the default per-block
        // burst threshold is 5 requests.
        bridge.config.max_requests_per_day = u64::MAX;
        bridge
            .suspicious_config
            .max_requests_per_block_per_account = u32::MAX;

        const REQUESTS: usize = 150;
        assert!(REQUESTS > PropertyBridge::MAX_BRIDGE_HISTORY_PER_ACCOUNT);
        for _ in 0..REQUESTS {
            test::set_caller::<DefaultEnvironment>(accounts.alice);
            let request_id = bridge
                .initiate_bridge_multisig(1, 2, accounts.bob, 2, None, metadata())
                .expect("initiate");
            test::set_caller::<DefaultEnvironment>(accounts.alice);
            bridge
                .sign_bridge_request(request_id, true)
                .expect("alice signs");
            test::set_caller::<DefaultEnvironment>(accounts.bob);
            bridge
                .sign_bridge_request(request_id, true)
                .expect("bob signs");
            test::set_caller::<DefaultEnvironment>(accounts.alice);
            bridge
                .execute_bridge(request_id)
                .expect("execute");
        }

        // The stored history is capped: the oldest entries were evicted.
        let full = bridge.get_bridge_history(accounts.alice, 0, 1_000_000);
        assert_eq!(
            full.len(),
            PropertyBridge::MAX_BRIDGE_HISTORY_PER_ACCOUNT,
            "history is capped at MAX_BRIDGE_HISTORY_PER_ACCOUNT"
        );

        // Pagination is zero-indexed and slides over the oldest-first order.
        assert_eq!(bridge.get_bridge_history(accounts.alice, 0, 50).len(), 50);
        assert_eq!(
            bridge.get_bridge_history(accounts.alice, 0, 50)[0].transaction_hash,
            full[0].transaction_hash
        );
        let page_one = bridge.get_bridge_history(accounts.alice, 1, 50);
        assert_eq!(page_one.len(), 50);
        assert_eq!(page_one[0].transaction_hash, full[50].transaction_hash);
        assert_eq!(
            bridge.get_bridge_history(accounts.alice, 2, 50).len(),
            0,
            "third page is empty"
        );

        // Empty / out-of-range reads are clean.
        assert!(bridge.get_bridge_history(accounts.bob, 0, 100).is_empty());
        assert!(bridge
            .get_bridge_history(accounts.alice, 99, 100)
            .is_empty());
    }
}
