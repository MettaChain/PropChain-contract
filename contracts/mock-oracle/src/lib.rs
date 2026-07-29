#![cfg_attr(not(feature = "std"), no_std, no_main)]
#![allow(
    clippy::arithmetic_side_effects,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::items_after_test_module,
    clippy::needless_borrows_for_generic_args,
    clippy::too_many_arguments,
    dead_code
)]

//! # Mock Oracle Contract
//!
//! A deterministic oracle for testnet staging that accepts prices via a
//! faucet-call message (`set_price`).  When the **`mock`** feature is enabled
//! (the default) it returns deterministic prices derived from the property id
//! when no explicit price has been pushed.  When `mock` is disabled it only
//! returns prices that were explicitly set.
//!
//! ## Feature flags
//!
//! - **`mock`** (default on): Deterministic price seeds for reproducible E2Es.
//! - **`mock` off**: Only explicitly pushed prices are returned; no fallback.
//!
//! ## Messages
//!
//! | Message | Description |
//! |---|---|
//! | `set_price(property_id, price)` | Push/override a price for a property. |
//! | `set_prices(prices)` | Batch version of `set_price`. |
//! | `reset(property_id)` | Clear a previously pushed price. |
//! | `reset_all()` | Clear all pushed prices. |
//! | `is_mock_enabled()` | Read the compile-time mock feature flag. |

use ink::prelude::string::{String, ToString};
use ink::prelude::vec::Vec;
use ink::storage::Mapping;
use propchain_traits::oracle::*;
use propchain_traits::property::PropertyType;

#[ink::contract]
mod mock_oracle_contract {
    use super::*;

    /// Default confidence score for mock valuations.
    const MOCK_CONFIDENCE: u32 = 95;

    /// Default number of sources reported by the mock.
    const MOCK_SOURCES_USED: u32 = 1;

    // ── Storage ───────────────────────────────────────────────────────────

    #[ink(storage)]
    pub struct MockOracle {
        /// Externally pushed prices: property_id → price.
        /// Takes precedence over the deterministic seed when set.
        pushed_prices: Mapping<u64, u128>,
        /// Track which property ids have had prices pushed (for `reset_all`).
        pushed_keys: Vec<u64>,
        /// Admin account that can override prices.
        admin: AccountId,
        /// Request counter for `request_valuation`.
        request_counter: u64,
        /// History of valuations for each property (used for history queries).
        history: Mapping<u64, Vec<PropertyValuation>>,
        /// Track whether the contract has been initialised with default prices.
        initialised: bool,
    }

    // ── Events ────────────────────────────────────────────────────────────

    /// Emitted when a price is pushed via `set_price`.
    #[ink(event)]
    pub struct PricePushed {
        #[ink(topic)]
        property_id: u64,
        price: u128,
        #[ink(topic)]
        caller: AccountId,
        timestamp: u64,
    }

    /// Emitted when a price is reset / cleared.
    #[ink(event)]
    pub struct PriceReset {
        #[ink(topic)]
        property_id: u64,
        #[ink(topic)]
        caller: AccountId,
        timestamp: u64,
    }

    /// Emitted when all prices are cleared.
    #[ink(event)]
    pub struct AllPricesReset {
        #[ink(topic)]
        caller: AccountId,
        timestamp: u64,
    }

    /// Emitted when a valuation is requested.
    #[ink(event)]
    pub struct ValuationRequested {
        #[ink(topic)]
        property_id: u64,
        request_id: u64,
        timestamp: u64,
    }

    // ── Constructor ───────────────────────────────────────────────────────

    impl MockOracle {
        /// Deploy the mock oracle.
        ///
        /// The caller becomes the admin.
        #[ink(constructor)]
        pub fn new() -> Self {
            Self {
                pushed_prices: Mapping::default(),
                pushed_keys: Vec::new(),
                admin: Self::env().caller(),
                request_counter: 0,
                history: Mapping::default(),
                initialised: true,
            }
        }

        /// Deploy with an explicit admin.
        #[ink(constructor)]
        pub fn new_with_admin(admin: AccountId) -> Self {
            Self {
                pushed_prices: Mapping::default(),
                pushed_keys: Vec::new(),
                admin,
                request_counter: 0,
                history: Mapping::default(),
                initialised: true,
            }
        }

        // ── Admin guard ───────────────────────────────────────────────────

        fn ensure_admin(&self) -> Result<(), OracleError> {
            if self.env().caller() != self.admin {
                return Err(OracleError::Unauthorized);
            }
            Ok(())
        }

        // ── Price helpers ─────────────────────────────────────────────────

        /// Return the effective price for a property.
        ///
        /// 1. If a price was explicitly pushed via `set_price`, return it.
        /// 2. Otherwise, if the `mock` feature is enabled, derive a
        ///    deterministic seed from `property_id`.
        /// 3. Otherwise return `None` / error.
        fn resolve_price(&self, property_id: u64) -> Result<u128, OracleError> {
            if let Some(pushed) = self.pushed_prices.get(&property_id) {
                return Ok(pushed);
            }
            #[cfg(feature = "mock")]
            {
                // Deterministic seed: 500_000 + property_id * 1_000
                let seed = 500_000u128.saturating_add(property_id as u128 * 1_000);
                Ok(seed)
            }
            #[cfg(not(feature = "mock"))]
            {
                let _ = property_id; // suppress unused warning
                Err(OracleError::PropertyNotFound)
            }
        }

        /// Build a `PropertyValuation` struct for the given property.
        fn build_valuation(&self, property_id: u64, price: u128) -> PropertyValuation {
            PropertyValuation {
                property_id,
                valuation: price,
                confidence_score: MOCK_CONFIDENCE,
                sources_used: MOCK_SOURCES_USED,
                last_updated: self.env().block_timestamp(),
                valuation_method: propchain_traits::oracle::ValuationMethod::Automated,
            }
        }

        /// Record a valuation in the history log.
        fn record_history(&mut self, property_id: u64, valuation: PropertyValuation) {
            let mut hist = self.history.get(&property_id).unwrap_or_default();
            hist.push(valuation);
            self.history.insert(&property_id, &hist);
        }

        // ── Faucet-call messages ─────────────────────────────────────────

        /// Push / override a price for a single property (anyone can call).
        #[ink(message)]
        pub fn set_price(&mut self, property_id: u64, price: u128) -> Result<(), OracleError> {
            if price == 0 {
                return Err(OracleError::InvalidValuation);
            }
            self.pushed_prices.insert(&property_id, &price);
            if !self.pushed_keys.contains(&property_id) {
                self.pushed_keys.push(property_id);
            }
            let caller = self.env().caller();
            self.env().emit_event(PricePushed {
                property_id,
                price,
                caller,
                timestamp: self.env().block_timestamp(),
            });
            Ok(())
        }

        /// Batch push prices (anyone can call).
        #[ink(message)]
        pub fn set_prices(&mut self, prices: Vec<(u64, u128)>) -> Result<(), OracleError> {
            for (property_id, price) in &prices {
                if *price == 0 {
                    return Err(OracleError::InvalidValuation);
                }
                self.pushed_prices.insert(property_id, price);
                if !self.pushed_keys.contains(property_id) {
                    self.pushed_keys.push(*property_id);
                }
            }
            let caller = self.env().caller();
            let now = self.env().block_timestamp();
            for (property_id, price) in &prices {
                self.env().emit_event(PricePushed {
                    property_id: *property_id,
                    price: *price,
                    caller,
                    timestamp: now,
                });
            }
            Ok(())
        }

        /// Clear a pushed price for a property.
        #[ink(message)]
        pub fn reset(&mut self, property_id: u64) -> Result<(), OracleError> {
            self.ensure_admin()?;
            self.pushed_prices.remove(&property_id);
            self.env().emit_event(PriceReset {
                property_id,
                caller: self.env().caller(),
                timestamp: self.env().block_timestamp(),
            });
            Ok(())
        }

        /// Clear all pushed prices.
        #[ink(message)]
        pub fn reset_all(&mut self) -> Result<(), OracleError> {
            self.ensure_admin()?;
            // Remove each pushed key from the mapping.
            let keys: Vec<u64> = self.pushed_keys.clone();
            for key in &keys {
                self.pushed_prices.remove(key);
            }
            self.pushed_keys.clear();
            self.env().emit_event(AllPricesReset {
                caller: self.env().caller(),
                timestamp: self.env().block_timestamp(),
            });
            Ok(())
        }

        /// Return `true` if the `mock` feature is compiled in.
        #[ink(message)]
        pub fn is_mock_enabled(&self) -> bool {
            cfg!(feature = "mock")
        }

        /// Return the current admin account.
        #[ink(message)]
        pub fn admin(&self) -> AccountId {
            self.admin
        }

        /// Transfer admin role.
        #[ink(message)]
        pub fn transfer_admin(&mut self, new_admin: AccountId) -> Result<(), OracleError> {
            self.ensure_admin()?;
            self.admin = new_admin;
            Ok(())
        }

        /// Return the price explicitly pushed for a property, if any.
        #[ink(message)]
        pub fn get_pushed_price(&self, property_id: u64) -> Option<u128> {
            self.pushed_prices.get(&property_id)
        }
    }

    // ── Oracle trait implementation ───────────────────────────────────────

    impl Oracle for MockOracle {
        #[ink(message)]
        fn get_valuation(&self, property_id: u64) -> Result<PropertyValuation, OracleError> {
            let price = self.resolve_price(property_id)?;
            Ok(self.build_valuation(property_id, price))
        }

        #[ink(message)]
        fn get_valuation_with_confidence(
            &self,
            property_id: u64,
        ) -> Result<ValuationWithConfidence, OracleError> {
            let valuation = self.get_valuation(property_id)?;
            let price = valuation.valuation;
            Ok(ValuationWithConfidence {
                valuation,
                volatility_index: 0,
                confidence_interval: (
                    price.saturating_sub(price / 10),
                    price.saturating_add(price / 10),
                ),
                outlier_sources: 0,
            })
        }

        #[ink(message)]
        fn request_valuation(&mut self, property_id: u64) -> Result<u64, OracleError> {
            self.request_counter = self.request_counter.saturating_add(1);
            let request_id = self.request_counter;
            // Resolve and record the valuation immediately (mock = instant).
            let price = self.resolve_price(property_id)?;
            let valuation = self.build_valuation(property_id, price);
            self.record_history(property_id, valuation);
            self.env().emit_event(ValuationRequested {
                property_id,
                request_id,
                timestamp: self.env().block_timestamp(),
            });
            Ok(request_id)
        }

        #[ink(message)]
        fn batch_request_valuations(
            &mut self,
            property_ids: Vec<u64>,
        ) -> Result<Vec<u64>, OracleError> {
            let mut ids = Vec::with_capacity(property_ids.len());
            for pid in property_ids {
                let rid = self.request_valuation(pid)?;
                ids.push(rid);
            }
            Ok(ids)
        }

        #[ink(message)]
        fn get_historical_valuations(
            &self,
            property_id: u64,
            limit: u32,
        ) -> Vec<PropertyValuation> {
            self.history
                .get(&property_id)
                .unwrap_or_default()
                .into_iter()
                .rev()
                .take(limit as usize)
                .collect()
        }

        #[ink(message)]
        fn get_market_volatility(
            &self,
            _property_type: PropertyType,
            _location: String,
        ) -> Result<VolatilityMetrics, OracleError> {
            // Mock: return zero volatility.
            Ok(VolatilityMetrics {
                property_type: _property_type,
                location: _location,
                volatility_index: 0,
                average_price_change: 0,
                period_days: 30,
                last_updated: self.env().block_timestamp(),
            })
        }

        #[ink(message)]
        fn get_oracle_snapshots(&self, property_id: u64, limit: u32) -> Vec<OracleDataSnapshot> {
            if limit == 0 {
                return Vec::new();
            }
            // Convert stored valuation history into snapshot format.
            let price = self.resolve_price(property_id).ok();
            if let Some(p) = price {
                vec![OracleDataSnapshot {
                    property_id,
                    source_id: "mock-oracle".to_string(),
                    valuation: p,
                    timestamp: self.env().block_timestamp(),
                    confidence_score: MOCK_CONFIDENCE,
                    valuation_method: propchain_traits::oracle::ValuationMethod::Automated,
                    is_anomaly: false,
                }]
                .into_iter()
                .take(limit as usize)
                .collect()
            } else {
                Vec::new()
            }
        }

        #[ink(message)]
        fn get_source_history(&self, _source_id: String, limit: u32) -> Vec<SourceHistoryEntry> {
            // Mock: return empty, or a single entry if any price was ever pushed.
            let _ = limit;
            Vec::new()
        }

        #[ink(message)]
        fn get_history_by_date_range(
            &self,
            property_id: u64,
            _start_timestamp: u64,
            _end_timestamp: u64,
        ) -> Vec<OracleDataSnapshot> {
            // Return current snapshot within range (simple mock).
            let price = self.resolve_price(property_id).ok();
            if let Some(p) = price {
                let ts = self.env().block_timestamp();
                if ts >= _start_timestamp && ts <= _end_timestamp {
                    vec![OracleDataSnapshot {
                        property_id,
                        source_id: "mock-oracle".to_string(),
                        valuation: p,
                        timestamp: ts,
                        confidence_score: MOCK_CONFIDENCE,
                        valuation_method: propchain_traits::oracle::ValuationMethod::Automated,
                        is_anomaly: false,
                    }]
                } else {
                    Vec::new()
                }
            } else {
                Vec::new()
            }
        }

        #[ink(message)]
        fn get_history_statistics(
            &self,
            property_id: u64,
            _days_lookback: u32,
        ) -> Result<OracleHistoryStatistics, OracleError> {
            let price = self.resolve_price(property_id)?;
            let now = self.env().block_timestamp();
            Ok(OracleHistoryStatistics {
                property_id,
                min_valuation: price,
                max_valuation: price,
                average_valuation: price,
                data_points: 1,
                period_start: now.saturating_sub(_days_lookback as u64 * 86_400),
                period_end: now,
                volatility_percentage: 0,
                trend_direction: 0,
            })
        }
    }

    // ── OracleRegistry trait implementation ───────────────────────────────

    impl OracleRegistry for MockOracle {
        #[ink(message)]
        fn add_source(&mut self, _source: OracleSource) -> Result<(), OracleError> {
            // Mock: accept silently.
            Ok(())
        }

        #[ink(message)]
        fn remove_source(&mut self, _source_id: String) -> Result<(), OracleError> {
            // Mock: accept silently.
            Ok(())
        }

        #[ink(message)]
        fn update_reputation(
            &mut self,
            _source_id: String,
            _success: bool,
        ) -> Result<(), OracleError> {
            // Mock: no-op.
            Ok(())
        }

        #[ink(message)]
        fn get_reputation(&self, _source_id: String) -> Option<u32> {
            // Mock: perfect reputation.
            Some(1000)
        }

        #[ink(message)]
        fn slash_source(
            &mut self,
            _source_id: String,
            _penalty_amount: u128,
        ) -> Result<(), OracleError> {
            // Mock: no-op.
            Ok(())
        }

        #[ink(message)]
        fn detect_anomalies(&self, _property_id: u64, _new_valuation: u128) -> bool {
            // Mock: never an anomaly.
            false
        }
    }

    // ── Tests ─────────────────────────────────────────────────────────────

    #[cfg(test)]
    mod tests {
        use ink::env::{test, DefaultEnvironment};

        use super::*;

        fn setup() -> MockOracle {
            let accounts = test::default_accounts::<DefaultEnvironment>();
            test::set_caller::<DefaultEnvironment>(accounts.alice);
            MockOracle::new()
        }

        #[ink::test]
        fn constructor_sets_admin() {
            let accounts = test::default_accounts::<DefaultEnvironment>();
            test::set_caller::<DefaultEnvironment>(accounts.alice);
            let oracle = MockOracle::new();
            assert_eq!(oracle.admin(), accounts.alice);
            assert!(oracle.initialised);
        }

        #[ink::test]
        fn constructor_with_admin() {
            let accounts = test::default_accounts::<DefaultEnvironment>();
            let oracle = MockOracle::new_with_admin(accounts.bob);
            assert_eq!(oracle.admin(), accounts.bob);
        }

        #[ink::test]
        fn set_price_and_read() {
            let mut oracle = setup();
            assert!(oracle.set_price(1, 1_000_000).is_ok());
            let val = oracle.get_valuation(1).unwrap();
            assert_eq!(val.valuation, 1_000_000);
            assert_eq!(val.confidence_score, MOCK_CONFIDENCE);
        }

        #[ink::test]
        fn set_price_rejects_zero() {
            let mut oracle = setup();
            assert_eq!(oracle.set_price(1, 0), Err(OracleError::InvalidValuation));
        }

        #[ink::test]
        fn mock_feature_returns_deterministic_price() {
            let oracle = setup();
            // When mock is enabled (default), property 1 → 500_000 + 1 * 1_000 = 501_000
            let val = oracle.get_valuation(1).unwrap();
            #[cfg(feature = "mock")]
            assert_eq!(val.valuation, 501_000);
            #[cfg(not(feature = "mock"))]
            assert_eq!(val, Err(OracleError::PropertyNotFound));
        }

        #[ink::test]
        fn batch_set_prices() {
            let mut oracle = setup();
            let prices = vec![(1u64, 100u128), (2u64, 200u128), (3u64, 300u128)];
            assert!(oracle.set_prices(prices).is_ok());
            assert_eq!(oracle.get_valuation(1).unwrap().valuation, 100);
            assert_eq!(oracle.get_valuation(2).unwrap().valuation, 200);
            assert_eq!(oracle.get_valuation(3).unwrap().valuation, 300);
        }

        #[ink::test]
        fn reset_price() {
            let mut oracle = setup();
            oracle.set_price(1, 1_000_000).unwrap();
            assert_eq!(oracle.get_valuation(1).unwrap().valuation, 1_000_000);
            oracle.reset(1).unwrap();
            // After reset, falls back to deterministic seed (if mock enabled)
            #[cfg(feature = "mock")]
            assert_eq!(oracle.get_valuation(1).unwrap().valuation, 501_000);
            #[cfg(not(feature = "mock"))]
            assert_eq!(oracle.get_valuation(1), Err(OracleError::PropertyNotFound));
        }

        #[ink::test]
        fn reset_all() {
            let mut oracle = setup();
            oracle.set_price(1, 100).unwrap();
            oracle.set_price(2, 200).unwrap();
            oracle.reset_all().unwrap();
            #[cfg(feature = "mock")]
            {
                assert_eq!(oracle.get_valuation(1).unwrap().valuation, 501_000);
                assert_eq!(oracle.get_valuation(2).unwrap().valuation, 502_000);
            }
        }

        #[ink::test]
        fn request_valuation_increments_counter() {
            let mut oracle = setup();
            let rid1 = oracle.request_valuation(1).unwrap();
            let rid2 = oracle.request_valuation(2).unwrap();
            assert_eq!(rid1, 1);
            assert_eq!(rid2, 2);
        }

        #[ink::test]
        fn batch_request_valuations() {
            let mut oracle = setup();
            let ids = oracle.batch_request_valuations(vec![10, 20, 30]).unwrap();
            assert_eq!(ids.len(), 3);
            assert_eq!(ids[0], 1);
            assert_eq!(ids[1], 2);
            assert_eq!(ids[2], 3);
        }

        #[ink::test]
        fn get_valuation_with_confidence() {
            let oracle = setup();
            let vwc = oracle.get_valuation_with_confidence(1).unwrap();
            assert_eq!(vwc.valuation.valuation, 501_000);
            assert_eq!(vwc.volatility_index, 0);
            assert_eq!(vwc.outlier_sources, 0);
        }

        #[ink::test]
        fn get_market_volatility() {
            let oracle = setup();
            let mv = oracle
                .get_market_volatility(PropertyType::Residential, "TestLocation".to_string())
                .unwrap();
            assert_eq!(mv.volatility_index, 0);
        }

        #[ink::test]
        fn get_history_statistics() {
            let oracle = setup();
            let stats = oracle.get_history_statistics(1, 30).unwrap();
            assert_eq!(stats.property_id, 1);
            assert_eq!(stats.average_valuation, 501_000);
            assert_eq!(stats.data_points, 1);
        }

        #[ink::test]
        fn oracle_registry_trait() {
            let mut oracle = setup();
            let source = OracleSource {
                id: "test-source".to_string(),
                source_type: propchain_traits::oracle::OracleSourceType::Manual,
                address: AccountId::from([0x01; 32]),
                is_active: true,
                weight: 50,
                last_updated: 0,
            };
            assert!(OracleRegistry::add_source(&mut oracle, source).is_ok());
            assert!(OracleRegistry::remove_source(&mut oracle, "test-source".to_string()).is_ok());
            assert!(
                OracleRegistry::update_reputation(&mut oracle, "src".to_string(), true).is_ok()
            );
            assert_eq!(
                OracleRegistry::get_reputation(&oracle, "src".to_string()),
                Some(1000)
            );
            assert!(OracleRegistry::slash_source(&mut oracle, "src".to_string(), 0).is_ok());
            assert!(!OracleRegistry::detect_anomalies(&oracle, 1, 1_000_000));
        }

        #[ink::test]
        fn is_mock_enabled() {
            let oracle = setup();
            #[cfg(feature = "mock")]
            assert!(oracle.is_mock_enabled());
            #[cfg(not(feature = "mock"))]
            assert!(!oracle.is_mock_enabled());
        }

        #[ink::test]
        fn transfer_admin() {
            let accounts = test::default_accounts::<DefaultEnvironment>();
            let mut oracle = setup();
            assert!(oracle.transfer_admin(accounts.bob).is_ok());
            assert_eq!(oracle.admin(), accounts.bob);
        }

        #[ink::test]
        fn non_admin_cannot_reset() {
            let accounts = test::default_accounts::<DefaultEnvironment>();
            let mut oracle = setup();
            test::set_caller::<DefaultEnvironment>(accounts.bob);
            assert_eq!(oracle.reset(1), Err(OracleError::Unauthorized));
        }
    }
}
