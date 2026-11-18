//! Unit tests for the oracle contract's price-feed updates (#1450).
//!
//! The oracle accepts admin-pushed valuations; it has no per-source "submission"
//! permission list for plain updates, no confidence-score computation of its own
//! and no on-chain staleness rejection of stored valuations (staleness exists
//! only for the *median price cache*, which needs registered sources). Tests
//! therefore cover: authorization, replacement, per-asset independence, history,
//! the volatility circuit breaker and anomaly check (the deviation controls), and
//! that valuations keep their supplied `last_updated` so consumers can judge
//! staleness themselves.

#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(test)]
mod tests {
    use ink::env::{test, DefaultEnvironment};
    use oracle::propchain_oracle::{OracleError, PropertyValuationOracle};
    use propchain_traits::oracle::{PropertyValuation, ValuationMethod};

    type Accounts = test::DefaultAccounts<DefaultEnvironment>;

    const BASE: u128 = 100_000_000_00;

    fn valuation(property_id: u64, amount: u128, updated: u64) -> PropertyValuation {
        PropertyValuation {
            property_id,
            valuation: amount,
            confidence_score: 90,
            sources_used: 3,
            last_updated: updated,
            valuation_method: ValuationMethod::MarketData,
        }
    }

    fn setup() -> (PropertyValuationOracle, Accounts) {
        let accounts = test::default_accounts::<DefaultEnvironment>();
        test::set_caller::<DefaultEnvironment>(accounts.alice);
        test::set_block_timestamp::<DefaultEnvironment>(1_000);
        (PropertyValuationOracle::new(accounts.alice), accounts)
    }

    // ── Submission and authorization ────────────────────────────────────

    #[ink::test]
    fn the_admin_submits_a_valuation_and_it_can_be_read_back() {
        let (mut o, _) = setup();
        o.update_property_valuation(1, valuation(1, BASE, 900)).unwrap();
        let stored = o.get_property_valuation(1).unwrap();
        assert_eq!(stored.valuation, BASE);
        assert_eq!(stored.confidence_score, 90);
        assert_eq!(stored.sources_used, 3);
        assert_eq!(stored.valuation_method, ValuationMethod::MarketData);
    }

    #[ink::test]
    fn an_unauthorized_account_cannot_submit_and_nothing_is_stored() {
        let (mut o, accounts) = setup();
        test::set_caller::<DefaultEnvironment>(accounts.bob);
        assert_eq!(
            o.update_property_valuation(1, valuation(1, BASE, 900)),
            Err(OracleError::Unauthorized)
        );
        assert_eq!(o.get_property_valuation(1), Err(OracleError::PriceFeedError));
        assert!(o.get_historical_valuations(1, 10).is_empty());
    }

    #[ink::test]
    fn a_zero_valuation_is_rejected() {
        let (mut o, _) = setup();
        assert_eq!(
            o.update_property_valuation(1, valuation(1, 0, 900)),
            Err(OracleError::InvalidValuation)
        );
    }

    #[ink::test]
    fn an_unknown_property_has_no_price() {
        let (o, _) = setup();
        assert_eq!(o.get_property_valuation(404), Err(OracleError::PriceFeedError));
        assert_eq!(o.get_property_trend(404), Err(OracleError::PropertyNotFound));
    }

    // ── Replacement, independence and history ───────────────────────────

    #[ink::test]
    fn a_new_update_overrides_the_previous_price() {
        let (mut o, _) = setup();
        o.update_property_valuation(1, valuation(1, BASE, 900)).unwrap();
        o.update_property_valuation(1, valuation(1, BASE + BASE / 10, 950)).unwrap();
        assert_eq!(o.get_property_valuation(1).unwrap().valuation, BASE + BASE / 10);
    }

    #[ink::test]
    fn assets_have_independent_price_feeds() {
        let (mut o, _) = setup();
        o.update_property_valuation(1, valuation(1, BASE, 900)).unwrap();
        o.update_property_valuation(2, valuation(2, BASE * 3, 900)).unwrap();
        o.update_property_valuation(1, valuation(1, BASE + 1_000, 950)).unwrap();
        assert_eq!(o.get_property_valuation(1).unwrap().valuation, BASE + 1_000);
        assert_eq!(o.get_property_valuation(2).unwrap().valuation, BASE * 3);
    }

    #[ink::test]
    fn history_lists_updates_most_recent_first_and_honours_the_limit() {
        let (mut o, _) = setup();
        for (i, amount) in [BASE, BASE + 10, BASE + 20].iter().enumerate() {
            o.update_property_valuation(1, valuation(1, *amount, 900 + i as u64)).unwrap();
        }
        let all: Vec<u128> = o.get_historical_valuations(1, 10).iter().map(|v| v.valuation).collect();
        assert_eq!(all, vec![BASE + 20, BASE + 10, BASE]);
        let two: Vec<u128> = o.get_historical_valuations(1, 2).iter().map(|v| v.valuation).collect();
        assert_eq!(two, vec![BASE + 20, BASE + 10]);
        assert!(o.get_historical_valuations(1, 0).is_empty());
        assert!(o.get_historical_valuations(2, 10).is_empty());
    }

    #[ink::test]
    fn the_latest_price_keeps_its_supplied_timestamp_so_consumers_can_judge_staleness() {
        let (mut o, _) = setup();
        o.update_property_valuation(1, valuation(1, BASE, 900)).unwrap();
        // A day passes with no new update: the stored price is unchanged, and its
        // `last_updated` shows exactly how old it is.
        test::set_block_timestamp::<DefaultEnvironment>(1_000 + 86_400);
        let stored = o.get_property_valuation(1).unwrap();
        assert_eq!(stored.valuation, BASE);
        assert_eq!(stored.last_updated, 900);
    }

    #[ink::test]
    fn trend_metrics_follow_the_latest_price() {
        let (mut o, _) = setup();
        o.update_property_valuation(1, valuation(1, BASE, 900)).unwrap();
        o.update_property_valuation(1, valuation(1, BASE + BASE / 20, 950)).unwrap();
        assert_eq!(o.get_property_trend(1).unwrap().current_price, BASE + BASE / 20);
    }

    // ── Deviation controls: circuit breaker and anomaly check ───────────

    #[ink::test]
    fn the_default_deviation_threshold_is_twenty_percent_and_is_inclusive() {
        let (mut o, _) = setup();
        assert_eq!(o.volatility_threshold(), 20);
        o.update_property_valuation(1, valuation(1, BASE, 900)).unwrap();
        // Exactly +20% is not above the threshold.
        o.update_property_valuation(1, valuation(1, BASE + BASE / 5, 950)).unwrap();
        assert!(!o.is_circuit_breaker_active());
    }

    #[ink::test]
    fn a_move_beyond_the_threshold_trips_the_breaker_and_blocks_updates_until_reset() {
        let (mut o, accounts) = setup();
        o.update_property_valuation(1, valuation(1, BASE, 900)).unwrap();
        // +21% trips it.
        assert_eq!(
            o.update_property_valuation(1, valuation(1, BASE + BASE * 21 / 100, 950)),
            Err(OracleError::CircuitBreakerActive)
        );
        assert!(o.is_circuit_breaker_active());
        assert_eq!(o.circuit_breaker_triggered_by(), Some(1));
        // The stored price is untouched.
        assert_eq!(o.get_property_valuation(1).unwrap().valuation, BASE);

        // Even a harmless update is blocked while the breaker is active.
        assert_eq!(
            o.update_property_valuation(1, valuation(1, BASE + 1, 960)),
            Err(OracleError::CircuitBreakerActive)
        );
        assert_eq!(
            o.update_property_valuation(2, valuation(2, BASE, 960)),
            Err(OracleError::CircuitBreakerActive)
        );

        // Only the admin can reset it.
        test::set_caller::<DefaultEnvironment>(accounts.bob);
        assert_eq!(o.reset_circuit_breaker(), Err(OracleError::Unauthorized));
        test::set_caller::<DefaultEnvironment>(accounts.alice);
        o.reset_circuit_breaker().unwrap();
        assert!(!o.is_circuit_breaker_active());
        assert_eq!(o.circuit_breaker_triggered_by(), None);
        assert!(o.update_property_valuation(1, valuation(1, BASE + 1, 970)).is_ok());
    }

    #[ink::test]
    fn the_threshold_is_configurable_within_bounds_by_the_admin_only() {
        let (mut o, accounts) = setup();
        assert_eq!(o.set_volatility_threshold(0), Err(OracleError::InvalidValuation));
        assert_eq!(o.set_volatility_threshold(101), Err(OracleError::InvalidValuation));
        o.set_volatility_threshold(5).unwrap();
        assert_eq!(o.volatility_threshold(), 5);

        o.update_property_valuation(1, valuation(1, BASE, 900)).unwrap();
        assert_eq!(
            o.update_property_valuation(1, valuation(1, BASE + BASE * 6 / 100, 950)),
            Err(OracleError::CircuitBreakerActive)
        );

        test::set_caller::<DefaultEnvironment>(accounts.bob);
        assert_eq!(o.set_volatility_threshold(50), Err(OracleError::Unauthorized));
    }

    #[ink::test]
    fn a_large_single_move_is_flagged_as_an_anomaly_when_volatility_is_low() {
        let (mut o, _) = setup();
        o.update_property_valuation(1, valuation(1, BASE, 900)).unwrap();
        assert!(o.is_anomaly(1, BASE + BASE * 30 / 100));
        assert!(o.is_anomaly(1, BASE - BASE * 30 / 100));
        // At or below 20% it is not an anomaly, and neither is an unknown property.
        assert!(!o.is_anomaly(1, BASE + BASE / 5));
        assert!(!o.is_anomaly(1, BASE));
        assert!(!o.is_anomaly(99, BASE * 10));
    }

    #[ink::test]
    fn price_alerts_can_be_registered_and_do_not_interfere_with_updates() {
        let (mut o, accounts) = setup();
        o.set_price_alert(1, 5, accounts.charlie).unwrap();
        o.update_property_valuation(1, valuation(1, BASE, 900)).unwrap();
        o.update_property_valuation(1, valuation(1, BASE + BASE / 10, 950)).unwrap();
        assert_eq!(o.get_property_valuation(1).unwrap().valuation, BASE + BASE / 10);
    }
}
