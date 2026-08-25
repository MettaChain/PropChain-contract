//! # Integration Tests: Mock Oracle faucet-call flow (Issue #1010)
//!
//! The mock oracle is deployed with `default-features = false`, i.e. the
//! `mock` feature is OFF: no deterministic seed prices exist and the only
//! prices available are those explicitly pushed via `set_price` /
//! `set_prices` (the "faucet-call" staging flow).
//!
//! Coverage:
//!   check `new()` / `new_with_admin()` constructors book the admin
//!   check `set_price` → pushed price readable, trait read-through works
//!   check `reset` / `reset_all` clear pushed prices (admin-gated)
//!   check `set_prices` batch push
//!   check zero price rejected
//!   check `is_mock_enabled()` reflects feature-off build
//!   check `transfer_admin` gate + handover
//!   check consumer-path oracle trait reads (`Oracle`, `OracleRegistry`)
//!
//! Because ink! unit tests run inside a single contract environment, we
//! exercise both the direct messages and the trait methods on the contract
//! instance — the latter mirrors how a consumer contract reads through it.

#[cfg(test)]
#[allow(clippy::module_inception)]
mod integration_mock_oracle {
    use ink::env::{test, DefaultEnvironment};
    use mock_oracle::mock_oracle_contract::MockOracle;
    use propchain_traits::oracle::{
        Oracle, OracleError, OracleRegistry, OracleSource, OracleSourceType, ValuationMethod,
    };

    fn setup() -> MockOracle {
        let accounts = test::default_accounts::<DefaultEnvironment>();
        test::set_caller::<DefaultEnvironment>(accounts.alice);
        MockOracle::new()
    }

    // ── Constructors ─────────────────────────────────────────────────────

    #[ink::test]
    fn constructor_defaults_admin_to_deployer() {
        let accounts = test::default_accounts::<DefaultEnvironment>();
        test::set_caller::<DefaultEnvironment>(accounts.alice);
        let oracle = MockOracle::new();
        assert_eq!(oracle.admin(), accounts.alice);
    }

    #[ink::test]
    fn constructor_new_with_admin_books_explicit_admin() {
        let accounts = test::default_accounts::<DefaultEnvironment>();
        let oracle = MockOracle::new_with_admin(accounts.bob);
        assert_eq!(oracle.admin(), accounts.bob);
    }

    // ── Faucet-call price flow ───────────────────────────────────────────

    #[ink::test]
    fn set_price_pushes_readable_price() {
        let mut oracle = setup();
        assert_eq!(oracle.set_price(1, 1_000_000), Ok(()));
        assert_eq!(oracle.get_pushed_price(1), Some(1_000_000));
    }

    #[ink::test]
    fn set_price_overrides_previous_push() {
        let mut oracle = setup();
        oracle.set_price(1, 1_000_000).unwrap();
        oracle.set_price(1, 2_500_000).unwrap();
        assert_eq!(oracle.get_pushed_price(1), Some(2_500_000));
    }

    #[ink::test]
    fn set_price_rejects_zero() {
        let mut oracle = setup();
        assert_eq!(oracle.set_price(1, 0), Err(OracleError::InvalidValuation));
        // Nothing was booked.
        assert_eq!(oracle.get_pushed_price(1), None);
    }

    #[ink::test]
    fn reset_clears_a_single_pushed_price() {
        let mut oracle = setup();
        oracle.set_price(1, 100).unwrap();
        oracle.set_price(2, 200).unwrap();

        assert_eq!(oracle.reset(1), Ok(()));
        assert_eq!(oracle.get_pushed_price(1), None);
        // Sibling price survives.
        assert_eq!(oracle.get_pushed_price(2), Some(200));
    }

    #[ink::test]
    fn reset_is_admin_gated() {
        let accounts = test::default_accounts::<DefaultEnvironment>();
        let mut oracle = setup();

        oracle.set_price(1, 100).unwrap();
        test::set_caller::<DefaultEnvironment>(accounts.bob);
        assert_eq!(oracle.reset(1), Err(OracleError::Unauthorized));
        // Price untouched by the rejected call.
        assert_eq!(oracle.get_pushed_price(1), Some(100));

        // Admin can still clear it.
        test::set_caller::<DefaultEnvironment>(accounts.alice);
        assert_eq!(oracle.reset(1), Ok(()));
        assert_eq!(oracle.get_pushed_price(1), None);
    }

    #[ink::test]
    fn reset_all_clears_every_pushed_price() {
        let mut oracle = setup();
        oracle.set_price(1, 100).unwrap();
        oracle.set_price(2, 200).unwrap();
        oracle.set_price(3, 300).unwrap();

        assert_eq!(oracle.reset_all(), Ok(()));
        for id in 1..=3 {
            assert_eq!(oracle.get_pushed_price(id), None);
        }
    }

    #[ink::test]
    fn set_prices_batch_pushes_all_entries() {
        let mut oracle = setup();
        let prices = vec![(10u64, 111u128), (20u64, 222u128), (30u64, 333u128)];
        assert_eq!(oracle.set_prices(prices), Ok(()));

        assert_eq!(oracle.get_pushed_price(10), Some(111));
        assert_eq!(oracle.get_pushed_price(20), Some(222));
        assert_eq!(oracle.get_pushed_price(30), Some(333));
    }

    // ── Feature-flag state ───────────────────────────────────────────────

    #[ink::test]
    fn is_mock_enabled_matches_fallback_behaviour() {
        let oracle = setup();
        // The workspace gate (`cargo test --all-features --workspace`)
        // force-enables the `mock` feature through feature unification,
        // while a bare `-p propchain-tests` run keeps it off. Whichever
        // build is active, `is_mock_enabled()` must agree with whether an
        // unpriced property resolves to a deterministic seed valuation.
        if oracle.is_mock_enabled() {
            assert!(Oracle::get_valuation(&oracle, 999).is_ok());
        } else {
            assert_eq!(
                Oracle::get_valuation(&oracle, 999),
                Err(OracleError::PropertyNotFound)
            );
        }
        assert_eq!(oracle.get_pushed_price(999), None);
    }

    // ── Admin transfer ───────────────────────────────────────────────────

    #[ink::test]
    fn transfer_admin_requires_admin_and_hands_over_rights() {
        let accounts = test::default_accounts::<DefaultEnvironment>();
        let mut oracle = MockOracle::new_with_admin(accounts.alice);

        // Non-admin cannot transfer.
        test::set_caller::<DefaultEnvironment>(accounts.bob);
        assert_eq!(
            oracle.transfer_admin(accounts.charlie),
            Err(OracleError::Unauthorized)
        );

        // Admin hands over to bob…
        test::set_caller::<DefaultEnvironment>(accounts.alice);
        assert_eq!(oracle.transfer_admin(accounts.bob), Ok(()));
        assert_eq!(oracle.admin(), accounts.bob);

        // …and the old admin immediately loses admin rights.
        assert_eq!(oracle.reset(1), Err(OracleError::Unauthorized));
        assert_eq!(oracle.reset_all(), Err(OracleError::Unauthorized));

        // New admin can manage state.
        test::set_caller::<DefaultEnvironment>(accounts.bob);
        assert_eq!(oracle.reset(1), Ok(()));
    }

    // ── Consumer-path trait reads ────────────────────────────────────────

    /// A consumer contract reads valuations through the `Oracle` trait; this
    /// exercises exactly that path against explicitly pushed prices.
    #[ink::test]
    fn consumer_reads_pushed_price_through_oracle_trait() {
        let mut oracle = setup();
        let pushed = 4_200_000u128;
        oracle.set_price(42, pushed).unwrap();

        let valuation = Oracle::get_valuation(&oracle, 42).expect("pushed price must resolve");
        assert_eq!(valuation.property_id, 42);
        assert_eq!(valuation.valuation, pushed);
        assert_eq!(valuation.confidence_score, 95);
        assert_eq!(valuation.sources_used, 1);
        assert_eq!(valuation.valuation_method, ValuationMethod::Automated);

        let with_confidence = Oracle::get_valuation_with_confidence(&oracle, 42)
            .expect("confidence view resolves alongside");
        assert_eq!(with_confidence.valuation.valuation, pushed);
        assert_eq!(with_confidence.volatility_index, 0);
        assert_eq!(with_confidence.outlier_sources, 0);
        // ±10 % confidence interval around the pushed price.
        assert_eq!(with_confidence.confidence_interval.0, pushed - pushed / 10);
        assert_eq!(with_confidence.confidence_interval.1, pushed + pushed / 10);

        // Overwriting the push changes what consumers see.
        oracle.set_price(42, 5_555_555).unwrap();
        let updated = Oracle::get_valuation(&oracle, 42).unwrap();
        assert_eq!(updated.valuation, 5_555_555);
    }

    #[ink::test]
    fn consumer_valuation_requests_are_recorded_in_history() {
        let mut oracle = setup();
        oracle.set_price(7, 900_000).unwrap();
        // Push both legs so batch resolution is identical in feature-off
        // builds and in `--all-features` builds (where unpriced properties
        // would otherwise fall back to deterministic seed prices).
        oracle.set_price(8, 800_000).unwrap();

        let request_id = Oracle::request_valuation(&mut oracle, 7)
            .expect("request resolves instantly on the mock");
        assert_eq!(request_id, 1);

        // A batch resolves every leg in order.
        let resolved = Oracle::batch_request_valuations(&mut oracle, vec![7, 8])
            .expect("batch resolves every priced leg");
        // Request IDs continue sequentially from the single request above.
        assert_eq!(resolved, vec![2u64, 3]);
        // Both legs of the batch were recorded on top of the single request.
        assert_eq!(Oracle::get_historical_valuations(&oracle, 7, 10).len(), 2);

        // History keeps the recorded valuations, newest first.
        let history = Oracle::get_historical_valuations(&oracle, 7, 10);
        assert!(!history.is_empty());
        assert!(history.iter().all(|v| v.valuation == 900_000));
        assert!(history.len() <= 10);

        // Statistics reflect the single known data point.
        let stats = Oracle::get_history_statistics(&oracle, 7, 30).unwrap();
        assert_eq!(stats.property_id, 7);
        assert_eq!(stats.min_valuation, 900_000);
        assert_eq!(stats.max_valuation, 900_000);
        assert_eq!(stats.average_valuation, 900_000);
        assert_eq!(stats.data_points, 1);
    }

    #[ink::test]
    fn consumer_sees_snapshots_only_for_priced_properties() {
        let mut oracle = setup();
        oracle.set_price(5, 123_456).unwrap();

        let snapshots = Oracle::get_oracle_snapshots(&oracle, 5, 10);
        assert_eq!(snapshots.len(), 1);
        assert_eq!(snapshots[0].property_id, 5);
        assert_eq!(snapshots[0].valuation, 123_456);
        assert!(!snapshots[0].is_anomaly);

        // Zero-limit queries stay empty in every configuration.
        assert!(Oracle::get_oracle_snapshots(&oracle, 5, 0).is_empty());

        // Unpriced properties only surface snapshots when the deterministic
        // seed of the `mock` feature is active (see feature-unification note
        // in `is_mock_enabled_matches_fallback_behaviour`).
        let unpriced = Oracle::get_oracle_snapshots(&oracle, 6, 10);
        if oracle.is_mock_enabled() {
            assert!(!unpriced.is_empty());
        } else {
            assert!(unpriced.is_empty());
        }

        // Volatility metrics are static zeros on the mock.
        let volatility = Oracle::get_market_volatility(
            &oracle,
            propchain_traits::PropertyType::Residential,
            "Testville".to_string(),
        )
        .unwrap();
        assert_eq!(volatility.volatility_index, 0);
        assert_eq!(volatility.average_price_change, 0);
    }

    #[ink::test]
    fn registry_trait_reports_perfect_mock_reputation() {
        let accounts = test::default_accounts::<DefaultEnvironment>();
        let mut oracle = setup();

        let source = OracleSource {
            id: "integration-source".to_string(),
            source_type: OracleSourceType::Manual,
            address: accounts.charlie,
            is_active: true,
            weight: 50,
            last_updated: 0,
        };
        assert_eq!(OracleRegistry::add_source(&mut oracle, source), Ok(()));
        assert_eq!(
            OracleRegistry::get_reputation(&oracle, "integration-source".to_string()),
            Some(1000)
        );
        assert_eq!(
            OracleRegistry::remove_source(&mut oracle, "integration-source".to_string()),
            Ok(())
        );
        // Anomaly detection is inert on the mock.
        assert!(!OracleRegistry::detect_anomalies(&oracle, 1, u128::MAX));
    }
}
