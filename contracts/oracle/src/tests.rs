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

        assert!(oracle
            .update_source_reputation(source_id.clone(), false)
            .is_ok());
        assert_eq!(
            oracle
                .source_reputations
                .get(&source_id)
                .expect("Source reputation should exist after update"),
            460
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
