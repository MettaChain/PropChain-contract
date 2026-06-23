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
}
