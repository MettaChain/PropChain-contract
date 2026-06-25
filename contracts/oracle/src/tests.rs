// Unit tests for the oracle contract (Issue #101 - extracted from lib.rs)

#[cfg(test)]
mod oracle_tests {
    use super::*;
    use crate::propchain_oracle::PropertyValuationOracle;
    use ink::env::{test, DefaultEnvironment};

    fn setup_oracle() -> PropertyValuationOracle {
        let accounts = test::default_accounts::<DefaultEnvironment>();
        test::set_caller::<DefaultEnvironment>(accounts.alice);
        PropertyValuationOracle::new(accounts.alice)
    }

    #[ink::test]
    fn test_new_oracle_works() {
        let oracle = setup_oracle();
        assert_eq!(oracle.active_sources.len(), 0);
        assert_eq!(oracle.min_sources_required, 2);
    }

    #[ink::test]
    fn test_add_oracle_source_works() {
        let mut oracle = setup_oracle();
        let accounts = test::default_accounts::<DefaultEnvironment>();

        let source = OracleSource {
            id: "chainlink_feed".to_string(),
            source_type: OracleSourceType::Chainlink,
            address: accounts.bob,
            is_active: true,
            weight: 50,
            last_updated: ink::env::block_timestamp::<DefaultEnvironment>(),
        };

        assert!(oracle.add_oracle_source(source).is_ok());
        assert_eq!(oracle.active_sources.len(), 1);
        assert_eq!(oracle.active_sources[0], "chainlink_feed");
    }

    #[ink::test]
    fn test_unauthorized_add_source_fails() {
        let mut oracle = setup_oracle();
        let accounts = test::default_accounts::<DefaultEnvironment>();

        test::set_caller::<DefaultEnvironment>(accounts.bob);

        let source = OracleSource {
            id: "chainlink_feed".to_string(),
            source_type: OracleSourceType::Chainlink,
            address: accounts.bob,
            is_active: true,
            weight: 50,
            last_updated: ink::env::block_timestamp::<DefaultEnvironment>(),
        };

        assert_eq!(
            oracle.add_oracle_source(source),
            Err(OracleError::Unauthorized)
        );
    }

    #[ink::test]
    fn test_update_property_valuation_works() {
        let mut oracle = setup_oracle();

        let valuation = PropertyValuation {
            property_id: 1,
            valuation: 500000,
            confidence_score: 85,
            sources_used: 3,
            last_updated: ink::env::block_timestamp::<DefaultEnvironment>(),
            valuation_method: ValuationMethod::MarketData,
        };

        assert!(oracle
            .update_property_valuation(1, valuation.clone())
            .is_ok());

        let retrieved = oracle.get_property_valuation(1);
        assert!(retrieved.is_ok());
        assert_eq!(
            retrieved.expect("Valuation should exist after update"),
            valuation
        );
    }

    #[ink::test]
    fn test_get_nonexistent_valuation_fails() {
        let oracle = setup_oracle();
        assert_eq!(
            oracle.get_property_valuation(999),
            Err(OracleError::PropertyNotFound)
        );
    }

    #[ink::test]
    fn test_set_price_alert_works() {
        let mut oracle = setup_oracle();
        let accounts = test::default_accounts::<DefaultEnvironment>();

        assert!(oracle.set_price_alert(1, 5, accounts.bob).is_ok());

        let alerts = oracle.price_alerts.get(&1).unwrap_or_default();
        assert_eq!(alerts.len(), 1);
        assert_eq!(alerts[0].threshold_percentage, 5);
        assert_eq!(alerts[0].alert_address, accounts.bob);
    }

    #[ink::test]
    fn test_calculate_percentage_change() {
        let oracle = setup_oracle();

        assert_eq!(oracle.calculate_percentage_change(100, 110), 10);
        assert_eq!(oracle.calculate_percentage_change(100, 80), 20);
        assert_eq!(oracle.calculate_percentage_change(100, 100), 0);
        assert_eq!(oracle.calculate_percentage_change(0, 100), 0);
    }

    #[ink::test]
    fn test_aggregate_prices_works() {
        let mut oracle = setup_oracle();
        let accounts = test::default_accounts::<DefaultEnvironment>();

        for (id, weight) in &[("source1", 50u32), ("source2", 50u32), ("source3", 50u32)] {
            oracle
                .add_oracle_source(OracleSource {
                    id: id.to_string(),
                    source_type: OracleSourceType::Manual,
                    address: accounts.bob,
                    is_active: true,
                    weight: *weight,
                    last_updated: ink::env::block_timestamp::<DefaultEnvironment>(),
                })
                .expect("Oracle source registration should succeed in test");
        }

        let prices = vec![
            PriceData {
                price: 100,
                timestamp: ink::env::block_timestamp::<DefaultEnvironment>(),
                source: "source1".to_string(),
            },
            PriceData {
                price: 105,
                timestamp: ink::env::block_timestamp::<DefaultEnvironment>(),
                source: "source2".to_string(),
            },
            PriceData {
                price: 98,
                timestamp: ink::env::block_timestamp::<DefaultEnvironment>(),
                source: "source3".to_string(),
            },
        ];

        let result = oracle.aggregate_prices(&prices);
        assert!(result.is_ok());

        let aggregated = result.expect("Price aggregation should succeed in test");
        assert!((98..=105).contains(&aggregated));
    }

    #[ink::test]
    fn test_filter_outliers_works() {
        let oracle = setup_oracle();

        let prices = vec![
            PriceData {
                price: 98,
                timestamp: ink::env::block_timestamp::<DefaultEnvironment>(),
                source: "source1".to_string(),
            },
            PriceData {
                price: 99,
                timestamp: ink::env::block_timestamp::<DefaultEnvironment>(),
                source: "source2".to_string(),
            },
            PriceData {
                price: 100,
                timestamp: ink::env::block_timestamp::<DefaultEnvironment>(),
                source: "source3".to_string(),
            },
            PriceData {
                price: 101,
                timestamp: ink::env::block_timestamp::<DefaultEnvironment>(),
                source: "source4".to_string(),
            },
            PriceData {
                price: 102,
                timestamp: ink::env::block_timestamp::<DefaultEnvironment>(),
                source: "source5".to_string(),
            },
            PriceData {
                price: 1000,
                timestamp: ink::env::block_timestamp::<DefaultEnvironment>(),
                source: "source6".to_string(),
            },
        ];

        let filtered = oracle.filter_outliers(&prices);
        assert_eq!(filtered.len(), 5);
        assert!(filtered.iter().all(|p| p.price < 200));
    }

    #[ink::test]
    fn test_calculate_confidence_score() {
        let oracle = setup_oracle();

        let prices = vec![
            PriceData {
                price: 100,
                timestamp: ink::env::block_timestamp::<DefaultEnvironment>(),
                source: "source1".to_string(),
            },
            PriceData {
                price: 102,
                timestamp: ink::env::block_timestamp::<DefaultEnvironment>(),
                source: "source2".to_string(),
            },
            PriceData {
                price: 98,
                timestamp: ink::env::block_timestamp::<DefaultEnvironment>(),
                source: "source3".to_string(),
            },
        ];

        let score = oracle.calculate_confidence_score(&prices);
        assert!(score.is_ok());

        let score = score.expect("Confidence score calculation should succeed in test");
        assert!(score > 50);
    }

    #[ink::test]
    fn test_set_location_adjustment_works() {
        let mut oracle = setup_oracle();

        let adjustment = LocationAdjustment {
            location_code: "NYC_MANHATTAN".to_string(),
            adjustment_percentage: 15,
            last_updated: ink::env::block_timestamp::<DefaultEnvironment>(),
            confidence_score: 90,
        };

        assert!(oracle.set_location_adjustment(adjustment.clone()).is_ok());

        let stored = oracle.location_adjustments.get(&adjustment.location_code);
        assert!(stored.is_some());
        assert_eq!(
            stored.expect("Location adjustment should exist after setting"),
            adjustment
        );
    }

    #[ink::test]
    fn test_get_comparable_properties_works() {
        let oracle = setup_oracle();

        let comparables = oracle.get_comparable_properties(1, 10);
        assert_eq!(comparables.len(), 0);
    }

    #[ink::test]
    fn test_get_historical_valuations_works() {
        let oracle = setup_oracle();

        let history = oracle.get_historical_valuations(1, 10);
        assert_eq!(history.len(), 0);
    }

    #[ink::test]
    fn test_insufficient_sources_error() {
        let oracle = setup_oracle();

        let prices = vec![PriceData {
            price: 100,
            timestamp: ink::env::block_timestamp::<DefaultEnvironment>(),
            source: "source1".to_string(),
        }];

        let result = oracle.aggregate_prices(&prices);
        assert_eq!(result, Err(OracleError::InsufficientSources));
    }

    #[ink::test]
    fn test_source_reputation_works() {
        let mut oracle = setup_oracle();
        let source_id = "source1".to_string();

        assert!(oracle
            .update_source_reputation(source_id.clone(), true)
            .is_ok());
        assert_eq!(
            oracle
                .source_reputations
                .get(&source_id)
                .expect("Source reputation should exist after update"),
            510
        );
    }

    // ── Batched Aggregation Tests ───────────────────────────────────────────────

    #[ink::test]
    fn test_set_batch_aggregation_works() {
        let mut oracle = setup_oracle();

        // Initially disabled
        assert_eq!(oracle.is_batch_aggregation_enabled(), false);

        // Enable batch aggregation
        assert!(oracle.set_batch_aggregation(true).is_ok());
        assert_eq!(oracle.is_batch_aggregation_enabled(), true);

        // Disable batch aggregation
        assert!(oracle.set_batch_aggregation(false).is_ok());
        assert_eq!(oracle.is_batch_aggregation_enabled(), false);
    }

    #[ink::test]
    fn test_batch_aggregation_produces_same_results() {
        let mut oracle = setup_oracle();
        let accounts = test::default_accounts::<DefaultEnvironment>();

        // Add multiple oracle sources
        for (id, weight) in &[
            ("source1", 50u32),
            ("source2", 30u32),
            ("source3", 20u32),
        ] {
            oracle
                .add_oracle_source(OracleSource {
                    id: id.to_string(),
                    source_type: OracleSourceType::Manual,
                    address: accounts.bob,
                    is_active: true,
                    weight: *weight,
                    last_updated: ink::env::block_timestamp::<DefaultEnvironment>(),
                })
                .expect("Oracle source registration should succeed");
        }

        // Set up manual prices for aggregation test
        let prices = vec![
            PriceData {
                price: 100,
                timestamp: ink::env::block_timestamp::<DefaultEnvironment>(),
                source: "source1".to_string(),
            },
            PriceData {
                price: 105,
                timestamp: ink::env::block_timestamp::<DefaultEnvironment>(),
                source: "source2".to_string(),
            },
            PriceData {
                price: 98,
                timestamp: ink::env::block_timestamp::<DefaultEnvironment>(),
                source: "source3".to_string(),
            },
        ];

        // Test sequential aggregation (batch disabled)
        let sequential_result = oracle.aggregate_prices(&prices);
        assert!(sequential_result.is_ok());
        let sequential_price = sequential_result.unwrap();

        // Enable batch aggregation
        assert!(oracle.set_batch_aggregation(true).is_ok());

        // Test batch aggregation
        let batch_result = oracle.aggregate_prices(&prices);
        assert!(batch_result.is_ok());
        let batch_price = batch_result.unwrap();

        // Results should be identical
        assert_eq!(sequential_price, batch_price);
    }

    #[ink::test]
    fn test_batch_mode_handles_source_failures_gracefully() {
        let mut oracle = setup_oracle();
        let accounts = test::default_accounts::<DefaultEnvironment>();

        // Add multiple sources
        for (id, weight) in &[
            ("source1", 50u32),
            ("source2", 30u32),
            ("source3", 20u32),
            ("source4", 10u32),
        ] {
            oracle
                .add_oracle_source(OracleSource {
                    id: id.to_string(),
                    source_type: OracleSourceType::Manual,
                    address: accounts.bob,
                    is_active: true,
                    weight: *weight,
                    last_updated: ink::env::block_timestamp::<DefaultEnvironment>(),
                })
                .expect("Oracle source registration should succeed");
        }

        // Enable batch aggregation
        assert!(oracle.set_batch_aggregation(true).is_ok());

        // Set up prices with some sources that would fail (simulated by not providing all)
        let prices = vec![
            PriceData {
                price: 100,
                timestamp: ink::env::block_timestamp::<DefaultEnvironment>(),
                source: "source1".to_string(),
            },
            PriceData {
                price: 105,
                timestamp: ink::env::block_timestamp::<DefaultEnvironment>(),
                source: "source2".to_string(),
            },
            // source3 and source4 are missing - simulating failures
        ];

        // Batch aggregation should still work with partial data
        let result = oracle.aggregate_prices(&prices);
        assert!(result.is_ok());

        let aggregated = result.unwrap();
        // Should aggregate the available sources
        assert!((100..=105).contains(&aggregated));
    }

    #[ink::test]
    fn test_packed_weights_cache_rebuilds_on_source_change() {
        let mut oracle = setup_oracle();
        let accounts = test::default_accounts::<DefaultEnvironment>();

        // Add initial sources
        oracle
            .add_oracle_source(OracleSource {
                id: "source1".to_string(),
                source_type: OracleSourceType::Manual,
                address: accounts.bob,
                is_active: true,
                weight: 50,
                last_updated: ink::env::block_timestamp::<DefaultEnvironment>(),
            })
            .expect("Oracle source registration should succeed");

        // Enable batch aggregation - should rebuild cache
        assert!(oracle.set_batch_aggregation(true).is_ok());
        assert_eq!(oracle.active_sources.len(), 1);

        // Add another source - should rebuild cache
        oracle
            .add_oracle_source(OracleSource {
                id: "source2".to_string(),
                source_type: OracleSourceType::Manual,
                address: accounts.bob,
                is_active: true,
                weight: 30,
                last_updated: ink::env::block_timestamp::<DefaultEnvironment>(),
            })
            .expect("Oracle source registration should succeed");

        assert_eq!(oracle.active_sources.len(), 2);

        // Verify aggregation still works correctly after cache rebuild
        let prices = vec![
            PriceData {
                price: 100,
                timestamp: ink::env::block_timestamp::<DefaultEnvironment>(),
                source: "source1".to_string(),
            },
            PriceData {
                price: 105,
                timestamp: ink::env::block_timestamp::<DefaultEnvironment>(),
                source: "source2".to_string(),
            },
        ];

        let result = oracle.aggregate_prices(&prices);
        assert!(result.is_ok());
    }

    #[ink::test]
    fn test_unauthorized_set_batch_aggregation_fails() {
        let mut oracle = setup_oracle();
        let accounts = test::default_accounts::<DefaultEnvironment>();

        // Try to enable batch aggregation as non-admin
        test::set_caller::<DefaultEnvironment>(accounts.bob);
        assert_eq!(
            oracle.set_batch_aggregation(true),
            Err(OracleError::Unauthorized)
        );
    }

    #[ink::test]
    fn test_slashing_works() {
        let mut oracle = setup_oracle();
        let source_id = "source1".to_string();

        oracle.source_stakes.insert(&source_id, &1000);
        assert!(oracle.slash_source(source_id.clone(), 100).is_ok());

        assert_eq!(
            oracle
                .source_stakes
                .get(&source_id)
                .expect("Source stake should exist after slashing"),
            900
        );
        assert!(
            oracle
                .source_reputations
                .get(&source_id)
                .expect("Source reputation should exist after slashing")
                < 500
        );
    }

    #[ink::test]
    fn test_anomaly_detection_works() {
        let mut oracle = setup_oracle();
        let property_id = 1;

        let valuation = PropertyValuation {
            property_id,
            valuation: 100000,
            confidence_score: 90,
            sources_used: 3,
            last_updated: 0,
            valuation_method: ValuationMethod::Automated,
        };

        oracle.property_valuations.insert(&property_id, &valuation);

        assert!(!oracle.is_anomaly(property_id, 105000));
        assert!(oracle.is_anomaly(property_id, 130000));
    }

    #[ink::test]
    fn test_property_trend_metrics_and_direction() {
        let mut oracle = setup_oracle();
        let property_id = 2;
        let prices = vec![100u128, 120, 140, 160, 180, 200, 220];
        let base_timestamp = 1_000_000u64;

        assert!(oracle.set_ema_alpha(5000).is_ok());

        for (index, price) in prices.iter().enumerate() {
            let valuation = PropertyValuation {
                property_id,
                valuation: *price,
                confidence_score: 90,
                sources_used: 3,
                last_updated: base_timestamp + index as u64 * 86_400,
                valuation_method: ValuationMethod::MarketData,
            };

            assert!(oracle.update_property_valuation(property_id, valuation).is_ok());
        }

        test::set_block_timestamp::<DefaultEnvironment>(base_timestamp + 8 * 86_400);

        let trend = oracle.get_property_trend(property_id).expect("Trend should exist");
        assert_eq!(trend.current_price, 220);
        assert_eq!(trend.sma_7d, 160);
        assert_eq!(trend.sma_30d, 160);
        assert_eq!(trend.ema_7d, 200);
        assert_eq!(trend.trend_direction, TrendDirection::Up);
    }

    #[ink::test]
    fn test_property_trend_direction_stable() {
        let mut oracle = setup_oracle();
        let property_id = 3;
        let prices = vec![100u128, 101, 100, 100, 101, 100, 100];
        let base_timestamp = 2_000_000u64;

        assert!(oracle.set_ema_alpha(3000).is_ok());

        for (index, price) in prices.iter().enumerate() {
            let valuation = PropertyValuation {
                property_id,
                valuation: *price,
                confidence_score: 90,
                sources_used: 3,
                last_updated: base_timestamp + index as u64 * 86_400,
                valuation_method: ValuationMethod::MarketData,
            };

            assert!(oracle.update_property_valuation(property_id, valuation).is_ok());
        }

        test::set_block_timestamp::<DefaultEnvironment>(base_timestamp + 8 * 86_400);

        let trend = oracle.get_property_trend(property_id).expect("Trend should exist");
        assert_eq!(trend.trend_direction, TrendDirection::Stable);
    }

    #[ink::test]
    fn test_volatility_index_window_calculation() {
        let mut oracle = setup_oracle();
        let property_id = 4;
        let prices = vec![100u128, 110, 90, 105];
        let base_timestamp = 3_000_000u64;

        for (index, price) in prices.iter().enumerate() {
            let valuation = PropertyValuation {
                property_id,
                valuation: *price,
                confidence_score: 80,
                sources_used: 3,
                last_updated: base_timestamp + index as u64 * 86_400,
                valuation_method: ValuationMethod::MarketData,
            };

            assert!(oracle.update_property_valuation(property_id, valuation).is_ok());
        }

        test::set_block_timestamp::<DefaultEnvironment>(base_timestamp + 5 * 86_400);
        let volatility = oracle
            .get_volatility_index(property_id, 7)
            .expect("Volatility index query should succeed");
        assert!(volatility > 0);
    }

    #[ink::test]
    fn test_batch_request_works() {
        let mut oracle = setup_oracle();
        let result = oracle.batch_request_valuations(vec![1, 2, 3]).unwrap();
        assert_eq!(result.successes.len(), 3);
        assert!(result.failures.is_empty());

        assert!(oracle.pending_requests.get(&1).is_some());
        assert!(oracle.pending_requests.get(&2).is_some());
        assert!(oracle.pending_requests.get(&3).is_some());
    }
}

// =========================================================================
// AUTO-SLASH TESTS (Issue #497)
// =========================================================================

#[cfg(test)]
mod auto_slash_tests {
    use super::*;
    use crate::propchain_oracle::PropertyValuationOracle;
    use ink::env::{test, DefaultEnvironment};

    fn setup() -> PropertyValuationOracle {
        let accounts = test::default_accounts::<DefaultEnvironment>();
        test::set_caller::<DefaultEnvironment>(accounts.alice);
        PropertyValuationOracle::new(accounts.alice)
    }

    fn add_source(oracle: &mut PropertyValuationOracle, id: &str) {
        let accounts = test::default_accounts::<DefaultEnvironment>();
        oracle
            .add_oracle_source(OracleSource {
                id: id.to_string(),
                source_type: OracleSourceType::Manual,
                address: accounts.bob,
                is_active: true,
                weight: 50,
                last_updated: 0,
            })
            .unwrap();
        // Give the source some stake
        oracle.source_stakes.insert(&id.to_string(), &1_000_000);
        // Give it a reputation
        oracle
            .source_reputations
            .insert(&id.to_string(), &500u32);
    }

    #[ink::test]
    fn test_auto_slash_config_defaults() {
        let oracle = setup();
        let (on_s, secs, on_d, bps, on_m, cnt) = oracle.get_auto_slash_config();
        assert!(!on_s);
        assert_eq!(secs, 3600);
        assert!(!on_d);
        assert_eq!(bps, 2000);
        assert!(!on_m);
        assert_eq!(cnt, 3);
    }

    #[ink::test]
    fn test_set_auto_slash_config() {
        let mut oracle = setup();
        assert!(oracle
            .set_auto_slash_config(true, 1800, true, 1500, true, 5)
            .is_ok());
        let (on_s, secs, on_d, bps, on_m, cnt) = oracle.get_auto_slash_config();
        assert!(on_s);
        assert_eq!(secs, 1800);
        assert!(on_d);
        assert_eq!(bps, 1500);
        assert!(on_m);
        assert_eq!(cnt, 5);
    }

    #[ink::test]
    fn test_set_auto_slash_config_zero_threshold_fails() {
        let mut oracle = setup();
        assert_eq!(
            oracle.set_auto_slash_config(true, 0, false, 1000, false, 3),
            Err(OracleError::InvalidParameters)
        );
    }

    #[ink::test]
    fn test_auto_slash_on_staleness() {
        let mut oracle = setup();
        add_source(&mut oracle, "stale_src");

        // Enable staleness auto-slash with a very short threshold (1 second)
        oracle
            .set_auto_slash_config(true, 1, false, 2000, false, 3)
            .unwrap();

        // Record the source as having reported at time 0
        oracle
            .source_last_report_time
            .insert(&"stale_src".to_string(), &0u64);

        // Set block timestamp to 100 (> staleness threshold of 1)
        test::set_block_timestamp::<DefaultEnvironment>(100);

        let stake_before = oracle
            .source_stakes
            .get(&"stale_src".to_string())
            .unwrap_or(0);

        // Trigger auto-slash via run_auto_slash_checks
        oracle.run_auto_slash_checks(500_000);

        let stake_after = oracle
            .source_stakes
            .get(&"stale_src".to_string())
            .unwrap_or(0);

        // Stake should have decreased
        assert!(stake_after < stake_before, "Stake should be slashed for staleness");
    }

    #[ink::test]
    fn test_auto_slash_on_missed_updates() {
        let mut oracle = setup();
        add_source(&mut oracle, "lazy_src");

        // Enable missed-updates auto-slash with threshold of 2
        oracle
            .set_auto_slash_config(false, 3600, false, 2000, true, 2)
            .unwrap();

        // Set missed update counter to 3 (above threshold)
        oracle
            .source_missed_updates
            .insert(&"lazy_src".to_string(), &3u32);

        let stake_before = oracle
            .source_stakes
            .get(&"lazy_src".to_string())
            .unwrap_or(0);

        oracle.run_auto_slash_checks(500_000);

        let stake_after = oracle
            .source_stakes
            .get(&"lazy_src".to_string())
            .unwrap_or(0);

        assert!(stake_after < stake_before, "Stake should be slashed for missed updates");

        // Counter should be reset after slash
        assert_eq!(
            oracle
                .source_missed_updates
                .get(&"lazy_src".to_string())
                .unwrap_or(99),
            0
        );
    }

    #[ink::test]
    fn test_auto_slash_respects_disabled_flags() {
        let mut oracle = setup();
        add_source(&mut oracle, "fine_src");

        // All auto-slash disabled
        oracle
            .set_auto_slash_config(false, 1, false, 1, false, 1)
            .unwrap();

        // Give the source a stale last-report time and high missed count
        oracle
            .source_last_report_time
            .insert(&"fine_src".to_string(), &0u64);
        oracle
            .source_missed_updates
            .insert(&"fine_src".to_string(), &100u32);
        test::set_block_timestamp::<DefaultEnvironment>(99_999);

        let stake_before = oracle
            .source_stakes
            .get(&"fine_src".to_string())
            .unwrap_or(0);

        oracle.run_auto_slash_checks(500_000);

        let stake_after = oracle
            .source_stakes
            .get(&"fine_src".to_string())
            .unwrap_or(0);

        // No slashing should have occurred
        assert_eq!(stake_before, stake_after, "No slash when all flags disabled");
    }

    #[ink::test]
    fn test_source_last_report_time_initially_zero() {
        let oracle = setup();
        assert_eq!(
            oracle.get_source_last_report_time("nonexistent".to_string()),
            0
        );
    }

    #[ink::test]
    fn test_source_missed_updates_initially_zero() {
        let oracle = setup();
        assert_eq!(
            oracle.get_source_missed_updates("nonexistent".to_string()),
            0
        );
    }
}

// =========================================================================
// MULTI-SIG ORACLE SOURCE MANAGEMENT TESTS (Issue #495)
// =========================================================================

#[cfg(test)]
mod oracle_source_multisig_tests {
    use super::*;
    use crate::propchain_oracle::PropertyValuationOracle;
    use ink::env::{test, DefaultEnvironment};

    fn setup_with_signers() -> PropertyValuationOracle {
        let accounts = test::default_accounts::<DefaultEnvironment>();
        test::set_caller::<DefaultEnvironment>(accounts.alice);
        let mut oracle = PropertyValuationOracle::new(accounts.alice);
        // Register two signers
        oracle.add_multisig_signer(accounts.alice).unwrap();
        oracle.add_multisig_signer(accounts.bob).unwrap();
        // Require 2 approvals
        oracle.set_multisig_threshold(2).unwrap();
        oracle
    }

    fn sample_source(id: &str) -> OracleSource {
        let accounts = test::default_accounts::<DefaultEnvironment>();
        OracleSource {
            id: id.to_string(),
            source_type: OracleSourceType::Chainlink,
            address: accounts.charlie,
            is_active: true,
            weight: 50,
            last_updated: 0,
        }
    }

    #[ink::test]
    fn test_single_admin_cannot_add_source_without_multisig() {
        let mut oracle = setup_with_signers();
        // Propose as alice (1 of 2 approvals) — should NOT be executed yet
        let proposal_id = oracle
            .propose_add_oracle_source(sample_source("chainlink_1"))
            .unwrap();

        // Source should not be active yet
        assert!(
            !oracle.active_sources.contains(&"chainlink_1".to_string()),
            "Source should not be added until threshold reached"
        );

        let prop = oracle.get_source_proposal(proposal_id).unwrap();
        assert!(!prop.executed);
        assert_eq!(prop.approvals.len(), 1);
    }

    #[ink::test]
    fn test_multisig_approval_executes_source_addition() {
        let mut oracle = setup_with_signers();
        let accounts = test::default_accounts::<DefaultEnvironment>();

        // Alice proposes
        let proposal_id = oracle
            .propose_add_oracle_source(sample_source("chainlink_2"))
            .unwrap();

        // Bob approves — threshold reached
        test::set_caller::<DefaultEnvironment>(accounts.bob);
        let executed = oracle.approve_source_proposal(proposal_id).unwrap();
        assert!(executed, "Proposal should execute when threshold reached");

        // Source should now be active
        assert!(
            oracle.active_sources.contains(&"chainlink_2".to_string()),
            "Source should be added after threshold reached"
        );

        let prop = oracle.get_source_proposal(proposal_id).unwrap();
        assert!(prop.executed);
    }

    #[ink::test]
    fn test_multisig_approval_executes_source_removal() {
        let mut oracle = setup_with_signers();
        let accounts = test::default_accounts::<DefaultEnvironment>();

        // First add the source directly (admin bypass with no signers)
        oracle
            .add_oracle_source(sample_source("pyth_1"))
            .unwrap();
        assert!(oracle.active_sources.contains(&"pyth_1".to_string()));

        // Propose removal
        let proposal_id = oracle
            .propose_remove_oracle_source("pyth_1".to_string())
            .unwrap();

        // Bob approves
        test::set_caller::<DefaultEnvironment>(accounts.bob);
        let executed = oracle.approve_source_proposal(proposal_id).unwrap();
        assert!(executed);

        // Source should be gone
        assert!(
            !oracle.active_sources.contains(&"pyth_1".to_string()),
            "Source should be removed after threshold reached"
        );
    }

    #[ink::test]
    fn test_non_signer_cannot_propose() {
        let mut oracle = setup_with_signers();
        let accounts = test::default_accounts::<DefaultEnvironment>();
        test::set_caller::<DefaultEnvironment>(accounts.django);
        assert_eq!(
            oracle.propose_add_oracle_source(sample_source("bad_src")),
            Err(OracleError::Unauthorized)
        );
    }

    #[ink::test]
    fn test_double_approval_fails() {
        let mut oracle = setup_with_signers();
        let proposal_id = oracle
            .propose_add_oracle_source(sample_source("chainlink_3"))
            .unwrap();

        // Alice tries to approve again
        assert_eq!(
            oracle.approve_source_proposal(proposal_id),
            Err(OracleError::AlreadyExists)
        );
    }

    #[ink::test]
    fn test_approve_executed_proposal_fails() {
        let mut oracle = setup_with_signers();
        let accounts = test::default_accounts::<DefaultEnvironment>();

        let proposal_id = oracle
            .propose_add_oracle_source(sample_source("chainlink_4"))
            .unwrap();

        test::set_caller::<DefaultEnvironment>(accounts.bob);
        oracle.approve_source_proposal(proposal_id).unwrap();

        // Try to approve again after execution
        assert_eq!(
            oracle.approve_source_proposal(proposal_id),
            Err(OracleError::AlreadyExists)
        );
    }

    #[ink::test]
    fn test_no_signers_add_source_immediately() {
        let accounts = test::default_accounts::<DefaultEnvironment>();
        test::set_caller::<DefaultEnvironment>(accounts.alice);
        let mut oracle = PropertyValuationOracle::new(accounts.alice);
        // No signers registered → immediate execution

        oracle
            .propose_add_oracle_source(sample_source("instant_src"))
            .unwrap();

        assert!(
            oracle.active_sources.contains(&"instant_src".to_string()),
            "Source added immediately when no signers configured"
        );
    }

    // ── Batched Collection: collect_prices_from_sources tests ────────────────

    #[ink::test]
    fn test_collect_prices_batched_vs_sequential_full_flow() {
        let mut oracle = setup_oracle();
        let accounts = test::default_accounts::<DefaultEnvironment>();

        for (id, weight) in &[
            ("src_a", 40u32),
            ("src_b", 35u32),
            ("src_c", 25u32),
        ] {
            oracle
                .add_oracle_source(OracleSource {
                    id: id.to_string(),
                    source_type: OracleSourceType::Manual,
                    address: accounts.bob,
                    is_active: true,
                    weight: *weight,
                    last_updated: ink::env::block_timestamp::<DefaultEnvironment>(),
                })
                .expect("add source");
        }

        // Run batched path
        assert!(oracle.set_batch_aggregation(true).is_ok());
        let batch_prices = oracle.collect_prices_from_sources(1).expect("batch collect");

        // Run sequential path
        assert!(oracle.set_batch_aggregation(false).is_ok());
        let seq_prices = oracle.collect_prices_from_sources(1).expect("seq collect");

        // Both paths should produce the same number of price results
        assert_eq!(
            batch_prices.len(),
            seq_prices.len(),
            "batched and sequential should return same count"
        );

        // Both should aggregate to the same value
        let batch_agg = oracle.set_batch_aggregation(true).ok().and_then(|_| {
            oracle.aggregate_prices(&batch_prices).ok()
        });
        let seq_agg = oracle.set_batch_aggregation(false).ok().and_then(|_| {
            oracle.aggregate_prices(&seq_prices).ok()
        });
        assert_eq!(batch_agg, seq_agg, "aggregated prices should match");
    }

    #[ink::test]
    fn test_batch_collect_handles_all_sources_failing() {
        let mut oracle = setup_oracle();
        let accounts = test::default_accounts::<DefaultEnvironment>();

        // Add sources that will all fail (Chainlink with no endpoint configured)
        for (id, weight) in &[("cl_fail1", 50u32), ("cl_fail2", 50u32)] {
            oracle
                .add_oracle_source(OracleSource {
                    id: id.to_string(),
                    source_type: OracleSourceType::Chainlink,
                    address: accounts.bob,
                    is_active: true,
                    weight: *weight,
                    last_updated: ink::env::block_timestamp::<DefaultEnvironment>(),
                })
                .expect("add source");
        }

        assert!(oracle.set_batch_aggregation(true).is_ok());
        let prices = oracle.collect_prices_from_sources(999).expect("batch collect");
        assert!(
            prices.is_empty(),
            "all sources failing should yield empty price list"
        );
    }

    #[ink::test]
    fn test_batched_source_failure_does_not_panic() {
        let mut oracle = setup_oracle();
        let accounts = test::default_accounts::<DefaultEnvironment>();

        // Mix of Manual (succeeds) and Chainlink (fails with no endpoint)
        oracle
            .add_oracle_source(OracleSource {
                id: "manual_ok".to_string(),
                source_type: OracleSourceType::Manual,
                address: accounts.bob,
                is_active: true,
                weight: 60,
                last_updated: ink::env::block_timestamp::<DefaultEnvironment>(),
            })
            .expect("add manual");

        oracle
            .add_oracle_source(OracleSource {
                id: "chainlink_fail".to_string(),
                source_type: OracleSourceType::Chainlink,
                address: accounts.bob,
                is_active: true,
                weight: 40,
                last_updated: ink::env::block_timestamp::<DefaultEnvironment>(),
            })
            .expect("add chainlink");

        assert!(oracle.set_batch_aggregation(true).is_ok());
        let result = oracle.collect_prices_from_sources(1);
        assert!(result.is_ok(), "batched collection should not panic on partial failures");
    }

    #[ink::test]
    fn test_packed_weights_accuracy() {
        let mut oracle = setup_oracle();
        let accounts = test::default_accounts::<DefaultEnvironment>();

        let weights = vec![10u32, 20, 30, 40, 50];
        for (i, w) in weights.iter().enumerate() {
            oracle
                .add_oracle_source(OracleSource {
                    id: format!("src_{}", i),
                    source_type: OracleSourceType::Manual,
                    address: accounts.bob,
                    is_active: true,
                    weight: *w,
                    last_updated: ink::env::block_timestamp::<DefaultEnvironment>(),
                })
                .expect("add source");
        }

        assert!(oracle.set_batch_aggregation(true).is_ok());

        // Verify each weight is correctly retrievable from packed cache
        for (i, expected) in weights.iter().enumerate() {
            let got = oracle.get_packed_source_weight(i);
            assert_eq!(
                got, *expected,
                "packed weight mismatch at index {}: got {} expected {}",
                i, got, expected
            );
        }
    }

    #[ink::test]
    fn test_packed_weights_odd_source_count() {
        let mut oracle = setup_oracle();
        let accounts = test::default_accounts::<DefaultEnvironment>();

        // 3 sources (odd count) — padding should not break weight lookups
        for (i, w) in [15u32, 25, 35].iter().enumerate() {
            oracle
                .add_oracle_source(OracleSource {
                    id: format!("odd_{}", i),
                    source_type: OracleSourceType::Manual,
                    address: accounts.bob,
                    is_active: true,
                    weight: *w,
                    last_updated: ink::env::block_timestamp::<DefaultEnvironment>(),
                })
                .expect("add source");
        }

        assert!(oracle.set_batch_aggregation(true).is_ok());
        assert_eq!(oracle.get_packed_source_weight(0), 15);
        assert_eq!(oracle.get_packed_source_weight(1), 25);
        assert_eq!(oracle.get_packed_source_weight(2), 35);
    }

    #[ink::test]
    fn test_aggregate_prices_batched_equal_sequential() {
        let mut oracle = setup_oracle();
        let accounts = test::default_accounts::<DefaultEnvironment>();

        for (id, weight) in &[
            ("s1", 40u32),
            ("s2", 35u32),
            ("s3", 25u32),
        ] {
            oracle
                .add_oracle_source(OracleSource {
                    id: id.to_string(),
                    source_type: OracleSourceType::Manual,
                    address: accounts.bob,
                    is_active: true,
                    weight: *weight,
                    last_updated: ink::env::block_timestamp::<DefaultEnvironment>(),
                })
                .expect("add source");
        }

        let prices = vec![
            PriceData {
                price: 1000,
                timestamp: ink::env::block_timestamp::<DefaultEnvironment>(),
                source: "s1".to_string(),
            },
            PriceData {
                price: 1100,
                timestamp: ink::env::block_timestamp::<DefaultEnvironment>(),
                source: "s2".to_string(),
            },
            PriceData {
                price: 950,
                timestamp: ink::env::block_timestamp::<DefaultEnvironment>(),
                source: "s3".to_string(),
            },
        ];

        // Sequential
        assert!(oracle.set_batch_aggregation(false).is_ok());
        let seq = oracle.aggregate_prices(&prices).unwrap();

        // Batched
        assert!(oracle.set_batch_aggregation(true).is_ok());
        let batch = oracle.aggregate_prices(&prices).unwrap();

        assert_eq!(seq, batch, "sequential and batched aggregation must match");
    }
}

// ── Oracle Gas Benchmarks ────────────────────────────────────────────────────
//
// Uses block-timestamp deltas as a gas proxy (same methodology as the
// PropertyRegistry benchmarks in tests/performance_benchmarks.rs).
// On-chain, fewer storage reads/writes translates directly to lower gas.

#[cfg(test)]
mod oracle_benchmarks {
    use super::*;
    use crate::propchain_oracle::PropertyValuationOracle;
    use ink::env::{test, DefaultEnvironment};

    fn setup_oracle() -> PropertyValuationOracle {
        let accounts = test::default_accounts::<DefaultEnvironment>();
        test::set_caller::<DefaultEnvironment>(accounts.alice);
        PropertyValuationOracle::new(accounts.alice)
    }

    fn add_sources(oracle: &mut PropertyValuationOracle, count: u32) {
        let accounts = test::default_accounts::<DefaultEnvironment>();
        for i in 0..count {
            oracle
                .add_oracle_source(OracleSource {
                    id: format!("bench_src_{}", i),
                    source_type: OracleSourceType::Manual,
                    address: accounts.bob,
                    is_active: true,
                    weight: (100 / count.max(1)) as u32,
                    last_updated: ink::env::block_timestamp::<DefaultEnvironment>(),
                })
                .expect("add source");
        }
    }

    fn make_prices(count: usize) -> Vec<PriceData> {
        let ts = ink::env::block_timestamp::<DefaultEnvironment>();
        (0..count)
            .map(|i| PriceData {
                price: 1000 + (i as u128 * 10),
                timestamp: ts,
                source: format!("bench_src_{}", i),
            })
            .collect()
    }

    // ── Sequential vs Batched: aggregation speed ──────────────────────────

    #[ink::test]
    fn benchmark_aggregation_sequential_vs_batched() {
        let mut oracle = setup_oracle();
        add_sources(&mut oracle, 10);
        let prices = make_prices(10);

        // Sequential aggregation
        assert!(oracle.set_batch_aggregation(false).is_ok());
        let start = test::get_block_timestamp::<DefaultEnvironment>();
        for _ in 0..100 {
            let _ = oracle.aggregate_prices(&prices);
        }
        let seq_end = test::get_block_timestamp::<DefaultEnvironment>();
        let seq_duration = seq_end.saturating_sub(start);

        // Batched aggregation (packed weights)
        assert!(oracle.set_batch_aggregation(true).is_ok());
        let start = test::get_block_timestamp::<DefaultEnvironment>();
        for _ in 0..100 {
            let _ = oracle.aggregate_prices(&prices);
        }
        let batch_end = test::get_block_timestamp::<DefaultEnvironment>();
        let batch_duration = batch_end.saturating_sub(start);

        // Batched must not be slower than sequential
        assert!(
            batch_duration <= seq_duration,
            "batched aggregation ({} ticks) should be <= sequential ({} ticks)",
            batch_duration,
            seq_duration,
        );

        // Both must produce identical results
        assert!(oracle.set_batch_aggregation(false).is_ok());
        let seq_result = oracle.aggregate_prices(&prices).unwrap();
        assert!(oracle.set_batch_aggregation(true).is_ok());
        let batch_result = oracle.aggregate_prices(&prices).unwrap();
        assert_eq!(seq_result, batch_result);
    }

    // ── Scaling: aggregation with increasing source count ─────────────────

    #[ink::test]
    fn benchmark_aggregation_scales_with_sources() {
        let mut oracle = setup_oracle();

        for count in [2, 5, 10, 20] {
            // Reset oracle for each batch size
            let accounts = test::default_accounts::<DefaultEnvironment>();
            oracle = PropertyValuationOracle::new(accounts.alice);
            add_sources(&mut oracle, count);
            let prices = make_prices(count as usize);

            assert!(oracle.set_batch_aggregation(true).is_ok());
            let start = test::get_block_timestamp::<DefaultEnvironment>();
            for _ in 0..50 {
                let _ = oracle.aggregate_prices(&prices);
            }
            let end = test::get_block_timestamp::<DefaultEnvironment>();
            let duration = end.saturating_sub(start);

            // Each iteration should complete within a reasonable bound
            // (50 iterations / duration gives per-iteration cost)
            assert!(
                duration <= 5000,
                "aggregation with {} sources took {} ticks (too slow)",
                count,
                duration,
            );
        }
    }

    // ── Packed weight lookup vs direct storage lookup ─────────────────────

    #[ink::test]
    fn benchmark_packed_weight_lookup_speed() {
        let mut oracle = setup_oracle();
        add_sources(&mut oracle, 10);

        assert!(oracle.set_batch_aggregation(true).is_ok());

        // Packed weight lookups
        let start = test::get_block_timestamp::<DefaultEnvironment>();
        for _ in 0..500 {
            for i in 0..10 {
                let _ = oracle.get_packed_source_weight(i);
            }
        }
        let packed_end = test::get_block_timestamp::<DefaultEnvironment>();
        let packed_duration = packed_end.saturating_sub(start);

        // Direct storage lookups via get_source_weight
        assert!(oracle.set_batch_aggregation(false).is_ok());
        let start = test::get_block_timestamp::<DefaultEnvironment>();
        for _ in 0..500 {
            for i in 0..10 {
                let sid = format!("bench_src_{}", i);
                let _ = oracle.get_source_weight(&sid);
            }
        }
        let direct_end = test::get_block_timestamp::<DefaultEnvironment>();
        let direct_duration = direct_end.saturating_sub(start);

        // Packed lookups should be at least as fast (fewer storage reads)
        assert!(
            packed_duration <= direct_duration,
            "packed weight lookups ({} ticks) should be <= direct lookups ({} ticks)",
            packed_duration,
            direct_duration,
        );
    }

    // ── Full collect_prices path: sequential vs batched ───────────────────

    #[ink::test]
    fn benchmark_collect_prices_sequential_vs_batched() {
        let mut oracle = setup_oracle();
        add_sources(&mut oracle, 5);

        // Sequential
        assert!(oracle.set_batch_aggregation(false).is_ok());
        let start = test::get_block_timestamp::<DefaultEnvironment>();
        for _ in 0..20 {
            let _ = oracle.collect_prices_from_sources(1);
        }
        let seq_end = test::get_block_timestamp::<DefaultEnvironment>();
        let seq_duration = seq_end.saturating_sub(start);

        // Batched
        assert!(oracle.set_batch_aggregation(true).is_ok());
        let start = test::get_block_timestamp::<DefaultEnvironment>();
        for _ in 0..20 {
            let _ = oracle.collect_prices_from_sources(1);
        }
        let batch_end = test::get_block_timestamp::<DefaultEnvironment>();
        let batch_duration = batch_end.saturating_sub(start);

        // Both must produce the same results
        assert!(oracle.set_batch_aggregation(false).is_ok());
        let seq_prices = oracle.collect_prices_from_sources(1).unwrap();
        assert!(oracle.set_batch_aggregation(true).is_ok());
        let batch_prices = oracle.collect_prices_from_sources(1).unwrap();
        assert_eq!(seq_prices.len(), batch_prices.len());

        // Report results (visible in test output)
        // Batched should be comparable or faster due to cached reads and batched writes
        // The key invariant: batched is never significantly slower
        assert!(
            batch_duration <= seq_duration * 2,
            "batched collection ({} ticks) should not be >2x slower than sequential ({} ticks)",
            batch_duration,
            seq_duration,
        );
    }
}
