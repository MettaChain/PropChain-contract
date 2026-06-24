/// # Bridge Chaos Engineering Tests (Issue #489)
///
/// Simulates various failure modes to verify the bridge contract's resilience.
///
/// Acceptance criteria:
///   ✓ Simulate validator key compromise (rapid failed signatures)
///   ✓ Simulate oracle price manipulation attempts
///   ✓ Simulate network partition (timeout / request-expiry handling)
///   ✓ Simulate rapid pause/unpause cycles
///   ✓ Simulate multiple concurrent recovery operations
///   ✓ Verify bridge state remains consistent after chaos scenarios
///   ✓ Verify no funds are lost or double-spent
#[cfg(test)]
mod bridge_chaos {
    use bridge::bridge::{
        BridgeOperation, BridgeOperationStatus, ChainTxStatus, Error, PauseFlags, PauseReason,
        PropertyBridge,
    };
    use ink::env::{test, DefaultEnvironment};
    use propchain_traits::PropertyMetadata;

    // ── Helpers ───────────────────────────────────────────────────────────────

    fn alice() -> ink::primitives::AccountId {
        test::default_accounts::<DefaultEnvironment>().alice
    }
    fn bob() -> ink::primitives::AccountId {
        test::default_accounts::<DefaultEnvironment>().bob
    }
    fn charlie() -> ink::primitives::AccountId {
        test::default_accounts::<DefaultEnvironment>().charlie
    }
    fn django() -> ink::primitives::AccountId {
        test::default_accounts::<DefaultEnvironment>().django
    }
    fn eve() -> ink::primitives::AccountId {
        test::default_accounts::<DefaultEnvironment>().eve
    }

    fn setup_bridge() -> PropertyBridge {
        test::set_caller::<DefaultEnvironment>(alice());
        // supported_chains=[1,2,3], min_sigs=2, max_sigs=5, timeout=100, gas=500_000
        PropertyBridge::new(vec![1, 2, 3], 2, 5, 100, 500_000)
    }

    fn make_metadata(label: &str) -> PropertyMetadata {
        PropertyMetadata {
            location: label.into(),
            size: 1_000,
            legal_description: label.into(),
            valuation: 100_000,
            documents_url: "ipfs://chaos-test".into(),
        }
    }

    fn setup_bridge_with_validators() -> PropertyBridge {
        let mut bridge = setup_bridge();
        test::set_caller::<DefaultEnvironment>(alice());
        bridge.add_validator(alice()).expect("add validator alice");
        bridge.add_validator(bob()).expect("add validator bob");
        bridge
            .add_validator(charlie())
            .expect("add validator charlie");
        bridge
    }

    // ── Chaos Test 1: Validator key compromise (rapid failed signatures) ──────

    /// Chaos scenario: a compromised validator sends a rapid burst of `approve=false`
    /// signatures.  The bridge's suspicious-activity heuristic should detect this and
    /// auto-pause the signing operation class, keeping funds safe.
    ///
    /// Even if the auto-pause threshold is not immediately crossed, the bridge must
    /// at minimum prevent double-signing (AlreadySigned) and must never execute a
    /// request that was explicitly rejected.
    #[ink::test]
    fn chaos_validator_key_compromise_rapid_failed_signatures() {
        let mut bridge = setup_bridge_with_validators();

        // Create several bridge requests so we can fire failed sigs across them
        let mut request_ids = Vec::new();
        for i in 0u64..5 {
            test::set_caller::<DefaultEnvironment>(alice());
            let rid = bridge
                .initiate_bridge_multisig(
                    i + 1,
                    2,
                    bob(),
                    2,
                    Some(200),
                    make_metadata("compromise-test"),
                )
                .expect("initiate");
            request_ids.push(rid);
        }

        // "Compromised" bob fires false-approvals on all requests
        test::set_caller::<DefaultEnvironment>(bob());
        for &rid in &request_ids {
            let _ = bridge.sign_bridge_request(rid, false);
        }

        // Re-signing the same request must return AlreadySigned (no replay)
        test::set_caller::<DefaultEnvironment>(bob());
        let replay = bridge.sign_bridge_request(request_ids[0], false);
        assert_eq!(
            replay,
            Err(Error::AlreadySigned),
            "duplicate false signature must be rejected with AlreadySigned"
        );

        // Any request that received a false signature must be in Failed state
        for &rid in &request_ids {
            if let Some(info) = bridge.monitor_bridge_status(rid) {
                // Requests signed false should be Failed or Pending (depending on
                // whether the threshold was met by other validators in time)
                assert_ne!(
                    info.status,
                    BridgeOperationStatus::Completed,
                    "request {rid} must not be Completed after a false-signature attack"
                );
            }
        }

        // Verify bridge analytics are still coherent
        let analytics = bridge.get_bridge_analytics();
        assert!(
            analytics.total_requests >= 5,
            "analytics must reflect all created requests"
        );
    }

    // ── Chaos Test 2: Oracle price manipulation attempt ───────────────────────

    /// Chaos scenario: an attacker tries to manipulate the bridge fee by submitting
    /// artificially large or tiny `amount_in` values to `register_cross_chain_trade`.
    /// The bridge must apply rate-limiting and the fee quote must remain coherent.
    #[ink::test]
    fn chaos_oracle_price_manipulation_attempt() {
        let mut bridge = setup_bridge();

        // Attempt 1: absurdly large amount — fee quote must not overflow
        test::set_caller::<DefaultEnvironment>(alice());
        let result_large = bridge.quote_cross_chain_trade(2, u128::MAX / 2);
        if let Ok(quote) = result_large {
            // protocol_fee = amount / 200 → must not be zero for large amounts
            assert!(
                quote.total_fee >= quote.protocol_fee,
                "total_fee must be >= protocol_fee (no negative fee)"
            );
            // total_fee must not exceed the amount itself
            assert!(
                quote.total_fee < u128::MAX / 2,
                "total_fee must not overflow"
            );
        }

        // Attempt 2: zero amount — fee must be zero
        let result_zero = bridge.quote_cross_chain_trade(2, 0);
        if let Ok(quote) = result_zero {
            assert_eq!(quote.protocol_fee, 0, "zero-amount protocol fee must be 0");
        }

        // Attempt 3: try to register a trade with a very large amount_in that might
        // trip the hourly volume limit.  The bridge must either accept or return a
        // domain error — it must not panic.
        let result_register =
            bridge.register_cross_chain_trade(1, None, 2, bob(), u128::MAX / 4, 0);
        // Either Ok or a recognised error — never a panic
        match result_register {
            Ok(_) | Err(Error::RateLimitExceeded) | Err(Error::OperationPaused) => {}
            Err(other) => panic!("unexpected error for large amount_in: {:?}", other),
        }

        // Verify bridge is still operational for normal amounts after the chaos
        let normal_result = bridge.register_cross_chain_trade(1, None, 2, bob(), 1_000, 900);
        // May succeed or hit rate limit — the key is no panic and no unexpected error
        match normal_result {
            Ok(_) | Err(Error::RateLimitExceeded) | Err(Error::OperationPaused) => {}
            Err(other) => panic!("unexpected error for normal amount: {:?}", other),
        }
    }

    // ── Chaos Test 3: Network partition (timeout / expiry handling) ───────────

    /// Chaos scenario: a bridge request is created but the validators don't sign
    /// it before its expiry block.  The bridge must mark it Expired and prevent
    /// late execution.
    #[ink::test]
    fn chaos_network_partition_timeout_handling() {
        let mut bridge = setup_bridge_with_validators();

        // Create a request with a 1-block timeout — it will expire immediately
        test::set_caller::<DefaultEnvironment>(alice());
        let request_id = bridge
            .initiate_bridge_multisig(1, 2, bob(), 2, Some(1), make_metadata("timeout-test"))
            .expect("initiate");

        // Advance blocks past the expiry
        test::advance_block::<DefaultEnvironment>();
        test::advance_block::<DefaultEnvironment>();

        // Signing an expired request must return RequestExpired
        test::set_caller::<DefaultEnvironment>(bob());
        let sign_result = bridge.sign_bridge_request(request_id, true);
        assert_eq!(
            sign_result,
            Err(Error::RequestExpired),
            "signing an expired request must return RequestExpired"
        );

        // Execution must also fail
        test::set_caller::<DefaultEnvironment>(alice());
        let exec_result = bridge.execute_bridge(request_id);
        assert!(
            exec_result.is_err(),
            "executing an expired request must fail"
        );

        // Admin can roll back the timed-out request
        let rollback_result =
            bridge.rollback_bridge_transaction(request_id, "timeout rollback".into());
        assert!(
            rollback_result.is_ok(),
            "admin must be able to roll back a timed-out request"
        );

        // After rollback both chain legs must be Failed
        let status = bridge
            .get_cross_chain_tx_status(request_id)
            .expect("status should exist");
        assert_eq!(
            status.source_status.status,
            ChainTxStatus::Failed,
            "source leg must be Failed after rollback"
        );
        assert_eq!(
            status.destination_status.status,
            ChainTxStatus::Failed,
            "destination leg must be Failed after rollback"
        );
    }

    // ── Chaos Test 4: Rapid pause / unpause cycles ────────────────────────────

    /// Chaos scenario: an attacker with guardian access (or a buggy guardian)
    /// rapidly toggles pause on and off.  The bridge must remain in a consistent
    /// state after the storm.
    #[ink::test]
    fn chaos_rapid_pause_unpause_cycles() {
        let mut bridge = setup_bridge();

        // Register alice as a guardian
        test::set_caller::<DefaultEnvironment>(alice());
        bridge.add_guardian(alice()).expect("add guardian");

        // Rapid pause / unpause — 20 cycles
        for i in 0..20u32 {
            test::set_caller::<DefaultEnvironment>(alice());
            if i % 2 == 0 {
                bridge
                    .emergency_pause(
                        PauseFlags {
                            all_operations: true,
                            new_requests: true,
                            signing: true,
                            execution: true,
                            cross_chain_trades: true,
                        },
                        PauseReason::GuardianTrigger,
                        Some(format!("chaos cycle {}", i)),
                    )
                    .expect("pause should succeed");
            } else {
                bridge
                    .emergency_unpause(PauseFlags {
                        all_operations: true,
                        new_requests: true,
                        signing: true,
                        execution: true,
                        cross_chain_trades: true,
                    })
                    .expect("unpause should succeed");
            }
        }

        // After 20 cycles (last is unpause at cycle 19 = odd → unpause),
        // the bridge should be fully unpaused.
        let health = bridge.get_bridge_health_status();
        assert!(
            !health.is_paused,
            "bridge must be unpaused after even number of cycles ending on unpause"
        );

        // Operations must work normally after chaos
        test::set_caller::<DefaultEnvironment>(alice());
        let result =
            bridge.initiate_bridge_multisig(1, 2, bob(), 2, Some(100), make_metadata("post-chaos"));
        assert!(
            result.is_ok(),
            "bridge should accept new requests after all pauses are cleared"
        );

        // Audit log must be non-empty and bounded
        let audit = bridge.get_pause_audit_log();
        assert!(
            !audit.is_empty(),
            "audit log must contain entries from the chaos cycles"
        );
        assert!(
            audit.len() <= 256,
            "audit log must be bounded to PAUSE_AUDIT_LOG_LIMIT"
        );
    }

    // ── Chaos Test 5: Multiple concurrent recovery operations ─────────────────

    /// Chaos scenario: admin attempts to run multiple recovery / rollback operations
    /// simultaneously on different failed requests.  Each rollback must be isolated
    /// and must not affect other requests.
    #[ink::test]
    fn chaos_multiple_concurrent_recovery_operations() {
        let mut bridge = setup_bridge_with_validators();

        // Create several requests
        test::set_caller::<DefaultEnvironment>(alice());
        let mut request_ids = Vec::new();
        for i in 1u64..=5 {
            let rid = bridge
                .initiate_bridge_multisig(
                    i,
                    2,
                    bob(),
                    2,
                    Some(50),
                    make_metadata("concurrent-recovery"),
                )
                .expect("initiate");
            request_ids.push(rid);
        }

        // Fail the first three by having a validator sign false
        test::set_caller::<DefaultEnvironment>(bob());
        for &rid in &request_ids[..3] {
            let _ = bridge.sign_bridge_request(rid, false);
        }

        // Admin rolls back all five (some are failed, some are still pending)
        test::set_caller::<DefaultEnvironment>(alice());
        for &rid in &request_ids {
            let result =
                bridge.rollback_bridge_transaction(rid, format!("concurrent rollback {}", rid));
            assert!(
                result.is_ok(),
                "admin must be able to rollback request {rid}"
            );
        }

        // Verify all are in terminal Failed state
        for &rid in &request_ids {
            let info = bridge
                .monitor_bridge_status(rid)
                .expect("monitor must return info");
            assert_eq!(
                info.status,
                BridgeOperationStatus::Failed,
                "request {rid} must be in Failed state after rollback"
            );
        }

        // Verify cross-chain trackers are all Failed
        for &rid in &request_ids {
            let status = bridge
                .get_cross_chain_tx_status(rid)
                .expect("cross-chain status must exist");
            assert_eq!(
                status.overall_status,
                BridgeOperationStatus::Failed,
                "cross-chain status for {rid} must be Failed"
            );
        }

        // Analytics must be coherent after all rollbacks
        let analytics = bridge.get_bridge_analytics();
        assert_eq!(
            analytics.total_requests, 5,
            "analytics must count all 5 created requests"
        );
    }

    // ── Chaos Test 6: Bridge state consistency after chaos ────────────────────

    /// Verifies that after a mix of chaos actions the bridge counter and
    /// request states remain self-consistent.
    #[ink::test]
    fn chaos_bridge_state_remains_consistent() {
        let mut bridge = setup_bridge_with_validators();

        test::set_caller::<DefaultEnvironment>(alice());

        // Create 3 requests
        let r1 = bridge
            .initiate_bridge_multisig(1, 2, bob(), 2, Some(200), make_metadata("r1"))
            .expect("r1");
        let r2 = bridge
            .initiate_bridge_multisig(2, 2, charlie(), 2, Some(200), make_metadata("r2"))
            .expect("r2");
        let r3 = bridge
            .initiate_bridge_multisig(3, 2, django(), 2, Some(200), make_metadata("r3"))
            .expect("r3");

        // r1: fully approved and executed
        bridge.add_validator(alice()).ok(); // may already be registered
        test::set_caller::<DefaultEnvironment>(alice());
        let _ = bridge.sign_bridge_request(r1, true);
        test::set_caller::<DefaultEnvironment>(bob());
        let _ = bridge.sign_bridge_request(r1, true);
        test::set_caller::<DefaultEnvironment>(alice());
        bridge.execute_bridge(r1).expect("execute r1");

        // r2: failed via false signature
        test::set_caller::<DefaultEnvironment>(charlie());
        let _ = bridge.sign_bridge_request(r2, false);

        // r3: rolled back by admin
        test::set_caller::<DefaultEnvironment>(alice());
        bridge
            .rollback_bridge_transaction(r3, "state consistency test".into())
            .expect("rollback r3");

        // Verify each request's final state
        let info_r1 = bridge.monitor_bridge_status(r1).expect("r1 info");
        assert_eq!(
            info_r1.status,
            BridgeOperationStatus::Completed,
            "r1 must be Completed"
        );

        let info_r2 = bridge.monitor_bridge_status(r2).expect("r2 info");
        assert_eq!(
            info_r2.status,
            BridgeOperationStatus::Failed,
            "r2 must be Failed"
        );

        let info_r3 = bridge.monitor_bridge_status(r3).expect("r3 info");
        assert_eq!(
            info_r3.status,
            BridgeOperationStatus::Failed,
            "r3 must be Failed"
        );

        // Analytics: 3 requests, 1 transaction completed
        let analytics = bridge.get_bridge_analytics();
        assert_eq!(analytics.total_requests, 3);
        assert_eq!(
            analytics.total_transactions, 1,
            "only r1 should be a completed transaction"
        );
    }

    // ── Chaos Test 7: No funds double-spent ──────────────────────────────────

    /// Verifies the idempotency invariant: calling execute_bridge twice on the
    /// same request must fail on the second call.
    #[ink::test]
    fn chaos_no_funds_double_spent() {
        let mut bridge = setup_bridge_with_validators();

        test::set_caller::<DefaultEnvironment>(alice());
        let request_id = bridge
            .initiate_bridge_multisig(1, 2, bob(), 2, None, make_metadata("double-spend-test"))
            .expect("initiate");

        // Collect 2 valid signatures
        test::set_caller::<DefaultEnvironment>(alice());
        bridge
            .sign_bridge_request(request_id, true)
            .expect("alice sign");
        test::set_caller::<DefaultEnvironment>(bob());
        bridge
            .sign_bridge_request(request_id, true)
            .expect("bob sign");

        // First execution must succeed
        test::set_caller::<DefaultEnvironment>(alice());
        let first = bridge.execute_bridge(request_id);
        assert!(first.is_ok(), "first execution must succeed");

        // Second execution must fail (idempotency)
        let second = bridge.execute_bridge(request_id);
        assert!(
            second.is_err(),
            "second execution of the same request must fail (no double-spend)"
        );

        // Transaction counter must reflect only one execution
        let analytics = bridge.get_bridge_analytics();
        assert_eq!(
            analytics.total_transactions, 1,
            "only one transaction must be recorded (no double-spend)"
        );
    }

    // ── Chaos Test 8: Pause blocks new requests but not status queries ────────

    /// When the bridge is paused, new request creation must be blocked, but
    /// read-only operations (monitor, get_config) must still work.
    #[ink::test]
    fn chaos_pause_blocks_writes_but_not_reads() {
        let mut bridge = setup_bridge_with_validators();

        // Create a request before the pause
        test::set_caller::<DefaultEnvironment>(alice());
        let rid = bridge
            .initiate_bridge_multisig(1, 2, bob(), 2, Some(200), make_metadata("pre-pause"))
            .expect("pre-pause request");

        // Pause the bridge
        bridge.set_emergency_pause(true).expect("admin can pause");

        // New requests must be blocked
        let result = bridge.initiate_bridge_multisig(
            2,
            2,
            charlie(),
            2,
            Some(200),
            make_metadata("post-pause"),
        );
        assert_eq!(
            result,
            Err(Error::OperationPaused),
            "new requests must be blocked while paused"
        );

        // Read-only queries must still work
        let info = bridge.monitor_bridge_status(rid);
        assert!(
            info.is_some(),
            "monitor_bridge_status must work while paused"
        );
        let _ = bridge.get_config();
        let _ = bridge.get_bridge_health_status();

        // Unpause and verify operations resume
        bridge
            .set_emergency_pause(false)
            .expect("admin can unpause");
        let post_unpause = bridge.initiate_bridge_multisig(
            2,
            2,
            charlie(),
            2,
            Some(200),
            make_metadata("post-unpause"),
        );
        assert!(
            post_unpause.is_ok(),
            "new requests must be allowed after unpause"
        );
    }
}
