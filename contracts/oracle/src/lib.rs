#![cfg_attr(not(feature = "std"), no_std, no_main)]
#![allow(
    clippy::arithmetic_side_effects,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::needless_borrows_for_generic_args
)]

use ink::prelude::*;
use ink::storage::Mapping;
use propchain_traits::*;

/// Property Valuation Oracle Contract
#[ink::contract]
mod propchain_oracle {
    use super::*;
    use ink::prelude::{
        string::{String, ToString},
        vec::Vec,
    };

    /// Error types for the Property Valuation Oracle
    #[derive(Debug, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub enum OracleError {
        PropertyNotFound,
        InsufficientSources,
        InvalidValuation,
        Unauthorized,
        OracleSourceNotFound,
        InvalidParameters,
        PriceFeedError,
        AlertNotFound,
    }

    /// Property Valuation Oracle storage
    #[ink(storage)]
    pub struct PropertyValuationOracle {
        /// Admin account
        admin: AccountId,

        /// Property valuations storage
        property_valuations: Mapping<u64, PropertyValuation>,

        /// Historical valuations per property
        historical_valuations: Mapping<u64, Vec<PropertyValuation>>,

        /// Oracle sources configuration
        oracle_sources: Mapping<String, OracleSource>,

        /// Active oracle sources list
        pub active_sources: Vec<String>,

        /// Price alerts configuration
        pub price_alerts: Mapping<u64, Vec<PriceAlert>>,

        /// Location-based adjustments
        pub location_adjustments: Mapping<String, LocationAdjustment>,

        /// Market trends data
        market_trends: Mapping<String, MarketTrend>,

        /// Comparable properties cache
        comparable_cache: Mapping<u64, Vec<ComparableProperty>>,

        /// Maximum staleness for price feeds (in seconds)
        max_price_staleness: u64,

        /// Minimum sources required for valuation
        pub min_sources_required: u32,

        /// Outlier detection threshold (standard deviations)
        outlier_threshold: u32,
    }

    /// Events emitted by the oracle
    #[ink(event)]
    pub struct ValuationUpdated {
        #[ink(topic)]
        property_id: u64,
        valuation: u128,
        confidence_score: u32,
        timestamp: u64,
    }

    #[ink(event)]
    pub struct PriceAlertTriggered {
        #[ink(topic)]
        property_id: u64,
        old_valuation: u128,
        new_valuation: u128,
        change_percentage: u32,
        alert_address: AccountId,
    }

    #[ink(event)]
    pub struct OracleSourceAdded {
        #[ink(topic)]
        source_id: String,
        source_type: OracleSourceType,
        weight: u32,
    }

    impl PropertyValuationOracle {
        /// Constructor for the Property Valuation Oracle
        #[ink(constructor)]
        pub fn new(admin: AccountId) -> Self {
            Self {
                admin,
                property_valuations: Mapping::default(),
                historical_valuations: Mapping::default(),
                oracle_sources: Mapping::default(),
                active_sources: Vec::new(),
                price_alerts: Mapping::default(),
                location_adjustments: Mapping::default(),
                market_trends: Mapping::default(),
                comparable_cache: Mapping::default(),
                max_price_staleness: 3600, // 1 hour
                min_sources_required: 2,
                outlier_threshold: 2, // 2 standard deviations
            }
        }

        /// Get property valuation from multiple sources with aggregation
        #[ink(message)]
        pub fn get_property_valuation(
            &self,
            property_id: u64,
        ) -> Result<PropertyValuation, OracleError> {
            self.property_valuations
                .get(&property_id)
                .ok_or(OracleError::PropertyNotFound)
        }

        /// Get property valuation with confidence metrics
        #[ink(message)]
        pub fn get_valuation_with_confidence(
            &self,
            property_id: u64,
        ) -> Result<ValuationWithConfidence, OracleError> {
            let valuation = self.get_property_valuation(property_id)?;

            // Calculate volatility and confidence interval
            let volatility = self.calculate_volatility(property_id)?;
            let confidence_interval = self.calculate_confidence_interval(&valuation)?;
            let outlier_sources = self.detect_outliers(property_id)?;

            Ok(ValuationWithConfidence {
                valuation,
                volatility_index: volatility,
                confidence_interval,
                outlier_sources,
            })
        }

        /// Update property valuation (admin only)
        #[ink(message)]
        pub fn update_property_valuation(
            &mut self,
            property_id: u64,
            valuation: PropertyValuation,
        ) -> Result<(), OracleError> {
            self.ensure_admin()?;

            // Validate valuation
            if valuation.valuation == 0 {
                return Err(OracleError::InvalidValuation);
            }

            // Store historical valuation
            self.store_historical_valuation(property_id, valuation.clone());

            // Update current valuation
            self.property_valuations.insert(&property_id, &valuation);

            // Check price alerts
            self.check_price_alerts(property_id, valuation.valuation)?;

            // Emit event
            self.env().emit_event(ValuationUpdated {
                property_id,
                valuation: valuation.valuation,
                confidence_score: valuation.confidence_score,
                timestamp: self.env().block_timestamp(),
            });

            Ok(())
        }

        /// Update property valuation from oracle sources
        #[ink(message)]
        pub fn update_valuation_from_sources(
            &mut self,
            property_id: u64,
        ) -> Result<(), OracleError> {
            // Collect prices from all active sources
            let prices = self.collect_prices_from_sources(property_id)?;

            if prices.len() < self.min_sources_required as usize {
                return Err(OracleError::InsufficientSources);
            }

            // Aggregate prices with outlier detection
            let aggregated_price = self.aggregate_prices(&prices)?;
            let confidence_score = self.calculate_confidence_score(&prices)?;

            let valuation = PropertyValuation {
                property_id,
                valuation: aggregated_price,
                confidence_score,
                sources_used: prices.len() as u32,
                last_updated: self.env().block_timestamp(),
                valuation_method: ValuationMethod::MarketData,
            };

            self.update_property_valuation(property_id, valuation)
        }

        /// Get historical valuations for a property
        #[ink(message)]
        pub fn get_historical_valuations(
            &self,
            property_id: u64,
            limit: u32,
        ) -> Vec<PropertyValuation> {
            self.historical_valuations
                .get(&property_id)
                .unwrap_or_default()
                .into_iter()
                .rev() // Most recent first
                .take(limit as usize)
                .collect()
        }

        /// Get market volatility metrics
        #[ink(message)]
        pub fn get_market_volatility(
            &self,
            property_type: PropertyType,
            location: String,
        ) -> Result<VolatilityMetrics, OracleError> {
            let key = format!("{:?}_{}", property_type, location);
            self.market_trends
                .get(&key)
                .map(|trend| VolatilityMetrics {
                    property_type: trend.property_type,
                    location: trend.location,
                    volatility_index: (trend.trend_percentage.unsigned_abs()).min(100),
                    average_price_change: trend.trend_percentage,
                    period_days: trend.period_months * 30, // Approximate
                    last_updated: trend.last_updated,
                })
                .ok_or(OracleError::InvalidParameters)
        }

        /// Set price alert for a property
        #[ink(message)]
        pub fn set_price_alert(
            &mut self,
            property_id: u64,
            threshold_percentage: u32,
            alert_address: AccountId,
        ) -> Result<(), OracleError> {
            let alert = PriceAlert {
                property_id,
                threshold_percentage,
                alert_address,
                last_triggered: 0,
                is_active: true,
            };

            let mut alerts = self.price_alerts.get(&property_id).unwrap_or_default();
            alerts.push(alert);
            self.price_alerts.insert(&property_id, &alerts);

            Ok(())
        }

        /// Add oracle source (admin only)
        #[ink(message)]
        pub fn add_oracle_source(&mut self, source: OracleSource) -> Result<(), OracleError> {
            self.ensure_admin()?;

            if source.weight > 100 {
                return Err(OracleError::InvalidParameters);
            }

            self.oracle_sources.insert(&source.id, &source);

            if source.is_active && !self.active_sources.contains(&source.id) {
                self.active_sources.push(source.id.clone());
            }

            self.env().emit_event(OracleSourceAdded {
                source_id: source.id,
                source_type: source.source_type,
                weight: source.weight,
            });

            Ok(())
        }

        /// Set location adjustment factor (admin only)
        #[ink(message)]
        pub fn set_location_adjustment(
            &mut self,
            adjustment: LocationAdjustment,
        ) -> Result<(), OracleError> {
            self.ensure_admin()?;
            self.location_adjustments
                .insert(&adjustment.location_code, &adjustment);
            Ok(())
        }

        /// Update market trend data (admin only)
        #[ink(message)]
        pub fn update_market_trend(&mut self, trend: MarketTrend) -> Result<(), OracleError> {
            self.ensure_admin()?;
            let key = format!("{:?}_{}", trend.property_type, trend.location);
            self.market_trends.insert(&key, &trend);
            Ok(())
        }

        /// Get comparable properties for AVM analysis
        #[ink(message)]
        pub fn get_comparable_properties(
            &self,
            property_id: u64,
            radius_km: u32,
        ) -> Vec<ComparableProperty> {
            self.comparable_cache
                .get(&property_id)
                .unwrap_or_default()
                .into_iter()
                .filter(|comp| comp.distance_km <= radius_km)
                .collect()
        }

        // Helper methods

        fn ensure_admin(&self) -> Result<(), OracleError> {
            if self.env().caller() != self.admin {
                return Err(OracleError::Unauthorized);
            }
            Ok(())
        }

        fn collect_prices_from_sources(
            &self,
            property_id: u64,
        ) -> Result<Vec<PriceData>, OracleError> {
            let mut prices = Vec::new();

            for source_id in &self.active_sources {
                if let Some(source) = self.oracle_sources.get(source_id) {
                    // In a real implementation, this would call external price feeds
                    // For now, we'll simulate price collection
                    match self.get_price_from_source(&source, property_id) {
                        Ok(price_data) => {
                            if self.is_price_fresh(&price_data) {
                                prices.push(price_data);
                            }
                        }
                        Err(_) => continue, // Skip failed sources
                    }
                }
            }

            Ok(prices)
        }

        fn get_price_from_source(
            &self,
            source: &OracleSource,
            _property_id: u64,
        ) -> Result<PriceData, OracleError> {
            // This is a placeholder for actual price feed integration
            // In production, this would call Chainlink, Pyth, or other oracles
            match source.source_type {
                OracleSourceType::Chainlink => {
                    // Implement Chainlink integration
                    Err(OracleError::PriceFeedError)
                }
                OracleSourceType::Pyth => {
                    // Implement Pyth integration
                    Err(OracleError::PriceFeedError)
                }
                OracleSourceType::Manual => {
                    // Manual price updates only
                    Err(OracleError::PriceFeedError)
                }
                OracleSourceType::Custom => {
                    // Custom oracle logic
                    Err(OracleError::PriceFeedError)
                }
            }
        }

        fn is_price_fresh(&self, price_data: &PriceData) -> bool {
            let current_time = self.env().block_timestamp();
            current_time.saturating_sub(price_data.timestamp) <= self.max_price_staleness
        }

        pub fn aggregate_prices(&self, prices: &[PriceData]) -> Result<u128, OracleError> {
            if prices.len() < self.min_sources_required as usize {
                return Err(OracleError::InsufficientSources);
            }

            // Remove outliers
            let filtered_prices = self.filter_outliers(prices);

            if filtered_prices.is_empty() {
                return Err(OracleError::InsufficientSources);
            }

            // Weighted average based on source weights
            let mut total_weighted_price = 0u128;
            let mut total_weight = 0u32;

            for price_data in &filtered_prices {
                let weight = self.get_source_weight(&price_data.source)?;
                total_weighted_price += price_data.price * weight as u128;
                total_weight += weight;
            }

            if total_weight == 0 {
                return Err(OracleError::InvalidParameters);
            }

            Ok(total_weighted_price / total_weight as u128)
        }

        pub fn filter_outliers(&self, prices: &[PriceData]) -> Vec<PriceData> {
            if prices.len() < 3 {
                return prices.to_vec();
            }

            // Calculate mean
            let sum: u128 = prices.iter().map(|p| p.price).sum();
            let mean = sum / prices.len() as u128;

            // Calculate standard deviation using fixed point arithmetic
            let variance: u128 = prices
                .iter()
                .map(|p| {
                    let diff = p.price.abs_diff(mean);
                    diff * diff
                })
                .sum();

            let variance_avg = variance / prices.len() as u128;
            // Integer square root via Newton-Raphson.
            // Starting from variance_avg is always an upper bound (sqrt(x) <= x for x >= 1),
            // so the sequence decreases monotonically to floor(sqrt(variance_avg)).
            let std_dev = if variance_avg == 0 {
                0u128
            } else {
                let mut x = variance_avg;
                loop {
                    let y = (x + variance_avg / x) / 2;
                    if y >= x {
                        break x; // converged
                    }
                    x = y;
                }
            };

            // Filter outliers (beyond threshold standard deviations)
            prices
                .iter()
                .filter(|p| {
                    let diff = p.price.abs_diff(mean);
                    diff <= std_dev * self.outlier_threshold as u128
                })
                .cloned()
                .collect()
        }

        fn get_source_weight(&self, source_id: &str) -> Result<u32, OracleError> {
            self.oracle_sources
                .get(&source_id.to_string())
                .map(|source| source.weight)
                .ok_or(OracleError::OracleSourceNotFound)
        }

        pub fn calculate_confidence_score(&self, prices: &[PriceData]) -> Result<u32, OracleError> {
            if prices.is_empty() {
                return Ok(0);
            }

            // Simple confidence based on number of sources and price variance
            let source_confidence = (prices.len() as u32 * 25).min(75); // Max 75 from sources

            // Calculate coefficient of variation
            let sum: u128 = prices.iter().map(|p| p.price).sum();
            let mean = sum / prices.len() as u128;

            let variance: u128 = prices
                .iter()
                .map(|p| {
                    let diff = p.price.abs_diff(mean);
                    diff * diff
                })
                .sum();

            // Calculate coefficient of variation using fixed point arithmetic
            let std_dev = if !prices.is_empty() {
                let variance_avg = variance / prices.len() as u128;
                // Simple approximation of square root (for fixed point)
                let mut approx = variance_avg;
                for _ in 0..5 {
                    // Newton-Raphson approximation
                    if approx > 0 {
                        approx = (approx + variance_avg / approx) / 2;
                    }
                }
                approx
            } else {
                0
            };

            let cv = if mean > 0 {
                (std_dev * 10000) / mean // Multiply by 10000 for precision
            } else {
                10000
            };

            // Lower CV = higher confidence (CV is in basis points)
            let variance_confidence = if cv <= 10000 {
                ((10000 - cv) / 400) as u32 // Scale to 0-25 range
            } else {
                0
            };

            Ok(source_confidence + variance_confidence)
        }

        fn calculate_volatility(&self, property_id: u64) -> Result<u32, OracleError> {
            let historical = self.get_historical_valuations(property_id, 30); // Last 30 valuations

            if historical.len() < 2 {
                return Ok(0);
            }

            // Calculate price changes
            let mut changes = Vec::new();
            for i in 1..historical.len() {
                let prev = historical[i - 1].valuation;
                let curr = historical[i].valuation;

                if prev > 0 {
                    let change = (curr.abs_diff(prev) * 10000) / prev;
                    changes.push(change);
                }
            }

            // Average absolute change as volatility index (in basis points)
            let total_change: u128 = changes.iter().sum();
            let avg_change_bp = total_change / changes.len() as u128;
            Ok((avg_change_bp / 100).min(100) as u32) // Convert to percentage
        }

        fn calculate_confidence_interval(
            &self,
            valuation: &PropertyValuation,
        ) -> Result<(u128, u128), OracleError> {
            // Simple confidence interval based on confidence score
            let margin = valuation.valuation * (100 - valuation.confidence_score) as u128 / 10000; // 1% per confidence point

            Ok((
                valuation.valuation.saturating_sub(margin),
                valuation.valuation + margin,
            ))
        }

        fn detect_outliers(&self, _property_id: u64) -> Result<u32, OracleError> {
            // This would implement outlier detection logic
            // For now, return 0
            Ok(0)
        }

        fn store_historical_valuation(&mut self, property_id: u64, valuation: PropertyValuation) {
            let mut history = self
                .historical_valuations
                .get(&property_id)
                .unwrap_or_default();
            history.push(valuation);

            // Keep only last 100 valuations
            if history.len() > 100 {
                let start_index = history.len() - 100;
                history = history.into_iter().skip(start_index).collect();
            }

            self.historical_valuations.insert(&property_id, &history);
        }

        fn check_price_alerts(
            &mut self,
            property_id: u64,
            new_valuation: u128,
        ) -> Result<(), OracleError> {
            if let Some(last_valuation) = self.property_valuations.get(&property_id) {
                let change_percentage =
                    self.calculate_percentage_change(last_valuation.valuation, new_valuation);

                if let Some(alerts) = self.price_alerts.get(&property_id) {
                    for alert in alerts {
                        if alert.is_active
                            && change_percentage >= alert.threshold_percentage as u128
                        {
                            self.env().emit_event(PriceAlertTriggered {
                                property_id,
                                old_valuation: last_valuation.valuation,
                                new_valuation,
                                change_percentage: change_percentage as u32,
                                alert_address: alert.alert_address,
                            });
                        }
                    }
                }
            }
            Ok(())
        }

        pub fn calculate_percentage_change(&self, old_value: u128, new_value: u128) -> u128 {
            if old_value == 0 {
                return 0;
            }

            let diff = new_value.abs_diff(old_value);

            (diff * 100) / old_value
        }
    }

    impl Default for PropertyValuationOracle {
        fn default() -> Self {
            Self::new(AccountId::from([0x0; 32]))
        }
    }
}

// Re-export the contract
// Re-export the contract and error type
pub use propchain_oracle::{OracleError, PropertyValuationOracle};

#[cfg(test)]
mod oracle_tests {
    use super::*;
    // use ink::codegen::env::Env; // Removed invalid import
    use ink::env::{
        test,
        DefaultEnvironment,
    };

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

        // Switch to non-admin caller
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
            valuation: 500000, // $500,000
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
        assert_eq!(retrieved.unwrap(), valuation);
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

        // Test 10% increase
        assert_eq!(oracle.calculate_percentage_change(100, 110), 10);

        // Test 20% decrease
        assert_eq!(oracle.calculate_percentage_change(100, 80), 20);

        // Test no change
        assert_eq!(oracle.calculate_percentage_change(100, 100), 0);

        // Test zero old value
        assert_eq!(oracle.calculate_percentage_change(0, 100), 0);
    }

    #[ink::test]
    fn test_aggregate_prices_works() {
        let mut oracle = setup_oracle();
        let accounts = test::default_accounts::<DefaultEnvironment>();

        // Register oracle sources so get_source_weight succeeds
        for (id, weight) in &[("source1", 50u32), ("source2", 50u32), ("source3", 50u32)] {
            oracle.add_oracle_source(OracleSource {
                id: id.to_string(),
                source_type: OracleSourceType::Manual,
                address: accounts.bob,
                is_active: true,
                weight: *weight,
                last_updated: ink::env::block_timestamp::<DefaultEnvironment>(),
            }).unwrap();
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

        let aggregated = result.unwrap();
        // Should be close to the weighted average of 100, 105, 98 ≈ 101
        assert!((98..=105).contains(&aggregated));
    }

    #[ink::test]
    fn test_filter_outliers_works() {
        let oracle = setup_oracle();

        // 5 tightly-clustered values + 1 extreme outlier.
        // With these values: mean ≈ 250, std_dev ≈ 335.
        // 1000's deviation (750) > 2 * 335 (670), so it is filtered.
        // The 5 normal values are all within 2σ and are kept.
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
                price: 1000, // True outlier: ~2.2 sigma from mean
                timestamp: ink::env::block_timestamp::<DefaultEnvironment>(),
                source: "source6".to_string(),
            },
        ];

        let filtered = oracle.filter_outliers(&prices);
        // The 1000 outlier should be filtered, leaving the 5 normal prices
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

        let score = score.unwrap();
        // Should be reasonably high due to low variance and multiple sources
        assert!(score > 50);
    }

    #[ink::test]
    fn test_set_location_adjustment_works() {
        let mut oracle = setup_oracle();

        let adjustment = LocationAdjustment {
            location_code: "NYC_MANHATTAN".to_string(),
            adjustment_percentage: 15, // 15% premium
            last_updated: ink::env::block_timestamp::<DefaultEnvironment>(),
            confidence_score: 90,
        };

        assert!(oracle.set_location_adjustment(adjustment.clone()).is_ok());

        let stored = oracle.location_adjustments.get(&adjustment.location_code);
        assert!(stored.is_some());
        assert_eq!(stored.unwrap(), adjustment);
    }

    #[ink::test]
    fn test_get_comparable_properties_works() {
        let oracle = setup_oracle();

        // Test with empty cache
        let comparables = oracle.get_comparable_properties(1, 10);
        assert_eq!(comparables.len(), 0);
    }

    #[ink::test]
    fn test_get_historical_valuations_works() {
        let oracle = setup_oracle();

        // Test with no history
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

        // With min_sources_required = 2, this should fail
        let result = oracle.aggregate_prices(&prices);
        assert_eq!(result, Err(OracleError::InsufficientSources));
    }
}
