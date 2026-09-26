#![allow(clippy::clone_on_copy)] // fires inside ink! generated storage code
#![cfg_attr(not(feature = "std"), no_std)]
#![allow(unexpected_cfgs)]
#![allow(clippy::new_without_default)]

use ink::prelude::string::String;
use ink::prelude::vec::Vec;

/// Standalone staking-dashboard helper with its own events, error type and
/// unit tests (wired into the crate per Issue #983; previously a dead file
/// that was never compiled or tested).
pub mod staking_dashboard;

#[ink::contract]
pub mod propchain_analytics {
    use super::*;

    /// Market metrics representing aggregated property data.
    #[derive(
        Debug, Clone, PartialEq, scale::Encode, scale::Decode, ink::storage::traits::StorageLayout,
    )]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub struct MarketMetrics {
        pub average_price: u128,
        pub total_volume: u128,
        pub properties_listed: u64,
    }

    /// Portfolio performance for an individual owner.
    #[derive(
        Debug, Clone, PartialEq, scale::Encode, scale::Decode, ink::storage::traits::StorageLayout,
    )]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    #[allow(dead_code)]
    pub struct PortfolioPerformance {
        pub total_value: u128,
        pub property_count: u64,
        pub recent_transactions: u64,
    }

    /// A single position within an owner's real estate portfolio.
    #[derive(
        Debug, Clone, PartialEq, scale::Encode, scale::Decode, ink::storage::traits::StorageLayout,
    )]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub struct PortfolioPosition {
        pub property_type: propchain_traits::PropertyType,
        pub value: u128,
    }

    /// Suggestion for portfolio rebalancing at the property type level.
    #[derive(
        Debug, Clone, PartialEq, scale::Encode, scale::Decode, ink::storage::traits::StorageLayout,
    )]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub struct RebalancingSuggestion {
        pub property_type: propchain_traits::PropertyType,
        pub current_allocation_bips: u32,
        pub target_allocation_bips: u32,
        pub recommendation: String,
    }

    /// Trend analysis with historical data.
    #[derive(
        Debug, Clone, PartialEq, scale::Encode, scale::Decode, ink::storage::traits::StorageLayout,
    )]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub struct MarketTrend {
        pub period_start: u64,
        pub period_end: u64,
        pub price_change_percentage: i32,
        pub volume_change_percentage: i32,
    }

    /// Maximum number of items allowed in a batch operation to stay within gas limits.
    const MAX_BATCH_SIZE: usize = 20;

    /// Data structure for a single metric update used in batch operations
    #[derive(
        Debug, Clone, PartialEq, scale::Encode, scale::Decode, ink::storage::traits::StorageLayout,
    )]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub struct MetricUpdate {
        pub average_price: u128,
        pub total_volume: u128,
        pub properties_listed: u64,
    }

    /// User behavior analytics for a specific account.
    #[derive(
        Debug, Clone, PartialEq, scale::Encode, scale::Decode, ink::storage::traits::StorageLayout,
    )]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    #[allow(dead_code)]
    pub struct UserBehavior {
        pub account: AccountId,
        pub total_interactions: u64,
        pub preferred_property_type: String,
        pub risk_score: u8,
    }

    /// Crowd wisdom sentiment derived from prediction markets
    #[derive(
        Debug, Clone, PartialEq, scale::Encode, scale::Decode, ink::storage::traits::StorageLayout,
    )]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub struct MarketSentiment {
        pub bull_volume: u128,
        pub bear_volume: u128,
        pub bull_bear_ratio_bips: u32, // Ratio in basis points (10000 = 100%)
    }

    /// Market Report.
    #[derive(
        Debug, Clone, PartialEq, scale::Encode, scale::Decode, ink::storage::traits::StorageLayout,
    )]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub struct MarketReport {
        pub generated_at: u64,
        pub metrics: MarketMetrics,
        pub trend: MarketTrend,
        pub sentiment: MarketSentiment,
        pub insights: String,
    }

    #[ink(storage)]
    pub struct AnalyticsDashboard {
        /// Administrator of the analytics dashboard
        admin: AccountId,
        /// Current market metrics
        current_metrics: MarketMetrics,
        /// Integrity checksum over `current_metrics`, recomputable from storage.
        ///
        /// Lets `verify_market_metrics_integrity` detect a partial or corrupted
        /// write instead of reporting whatever bytes happen to be stored.
        /// See [`AnalyticsDashboard::metrics_checksum`].
        metrics_checksum: u64,
        /// Ledger timestamp of the last write to `current_metrics`.
        metrics_updated_at: u64,
        /// Account that performed the last write to `current_metrics`.
        metrics_updated_by: AccountId,
        /// Lifetime count of writes to `current_metrics`.
        metrics_update_count: u64,
        /// Whether `current_metrics` currently holds an admin-supplied value
        /// rather than a contract-derived one. See the override semantics note
        /// on `update_market_metrics`.
        metrics_is_override: bool,
        /// Historical market trends
        historical_trends: ink::storage::Mapping<u64, MarketTrend>,
        /// Trend count
        trend_count: u64,
        /// Sentiments per property
        property_sentiments: ink::storage::Mapping<u64, MarketSentiment>,
        /// Overall aggregated sentiment
        overall_sentiment: MarketSentiment,
        /// Owner portfolio holdings by property type
        portfolio_positions: ink::storage::Mapping<AccountId, Vec<PortfolioPosition>>,
        /// Property-type specific market trends for rebalancing
        property_type_trends: ink::storage::Mapping<propchain_traits::PropertyType, MarketTrend>,
        /// Benchmark performance for property types against a basket of reference indices
        benchmark_indices: ink::storage::Mapping<propchain_traits::PropertyType, i32>,
        /// Pending admin key rotation request (Issue #496)
        pending_admin_rotation: Option<propchain_traits::KeyRotationRequest>,
    }

    /// Errors for the analytics contract.
    #[derive(Debug, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub enum AnalyticsError {
        Unauthorized,
        // Admin key rotation (Issue #496)
        KeyRotationCooldown,
        KeyRotationExpired,
        NoPendingRotation,
        RotationUnauthorized,
        RequestExpired,
        BatchSizeExceeded,
    }

    // ── Admin Key Rotation Events (Issue #496) ────────────────────────────────

    #[ink(event)]
    pub struct AdminRotationRequested {
        #[ink(topic)]
        old_admin: AccountId,
        #[ink(topic)]
        new_admin: AccountId,
        effective_at_block: u32,
    }

    #[ink(event)]
    pub struct AdminRotationConfirmed {
        #[ink(topic)]
        old_admin: AccountId,
        #[ink(topic)]
        new_admin: AccountId,
    }

    #[ink(event)]
    pub struct AdminRotationCancelled {
        #[ink(topic)]
        old_admin: AccountId,
        cancelled_by: AccountId,
    }
    #[ink(event)]
    pub struct BatchMetricsUpdated {
        #[ink(topic)]
        count: u64,
        /// The combined metrics actually written, so the event cannot disagree
        /// with the stored value the way a bare count could.
        result: MarketMetrics,
    }

    /// Emitted by `batch_add_trends`.
    ///
    /// This used to reuse `BatchMetricsUpdated`, so a consumer tailing that
    /// event for market-metric changes also received one every time a trend was
    /// added, with no way to tell the two apart.
    #[ink(event)]
    pub struct BatchTrendsAdded {
        #[ink(topic)]
        count: u64,
    }

    /// Emitted on every admin write to `current_metrics`, carrying the values
    /// before and after so an override is always attributable.
    #[ink(event)]
    pub struct MarketMetricsOverridden {
        #[ink(topic)]
        updated_by: AccountId,
        previous_average_price: u128,
        previous_total_volume: u128,
        previous_properties_listed: u64,
        new_average_price: u128,
        new_total_volume: u128,
        new_properties_listed: u64,
        updated_at: u64,
    }

    /// Provenance of the current market metrics.
    ///
    /// Reported alongside `get_market_metrics` so a consumer can tell an
    /// admin-supplied figure from a contract-derived one, and can tell a value
    /// that still matches its integrity checksum from one that does not.
    #[derive(
        Debug, Clone, PartialEq, scale::Encode, scale::Decode, ink::storage::traits::StorageLayout,
    )]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub struct MetricsProvenance {
        pub metrics: MarketMetrics,
        /// Ledger timestamp of the last write.
        pub updated_at: u64,
        /// Account that performed the last write.
        pub updated_by: AccountId,
        /// Lifetime number of writes.
        pub update_count: u64,
        /// `true` when the stored value came from an admin write.
        pub is_override: bool,
        /// `true` when the stored metrics still match their integrity checksum.
        pub is_intact: bool,
    }

    impl AnalyticsDashboard {
        /// Reads the stored metrics as a plain value.
        ///
        /// The single read path for the metrics, so the getter cannot drift
        /// from what is actually stored.
        fn stored_market_metrics(&self) -> MarketMetrics {
            self.current_metrics.clone()
        }

        /// FNV-1a over the three metric fields, widened to 64 bits.
        ///
        /// An integrity check, not a cryptographic commitment: it detects a
        /// partial or corrupted write to `current_metrics`, which is what the
        /// "recompute == stored" invariant is guarding against. It is not meant
        /// to be proof against a malicious writer, who recomputes the checksum
        /// anyway.
        fn metrics_checksum(metrics: &MarketMetrics) -> u64 {
            const OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
            const PRIME: u64 = 0x0000_0100_0000_01b3;

            let mut hash = OFFSET;
            let mut absorb = |mut value: u128| {
                // Feed the 128-bit value a byte at a time, little-endian, so the
                // digest depends on the full width rather than a truncation.
                for _ in 0..16 {
                    hash ^= (value & 0xff) as u64;
                    hash = hash.wrapping_mul(PRIME);
                    value >>= 8;
                }
            };
            absorb(metrics.average_price);
            absorb(metrics.total_volume);
            absorb(metrics.properties_listed as u128);
            hash
        }

        /// `true` when the stored metrics match their recorded checksum.
        fn metrics_integrity_ok(&self) -> bool {
            self.metrics_checksum == Self::metrics_checksum(&self.stored_market_metrics())
        }

        /// Writes `next` to `current_metrics` and updates the derived bookkeeping.
        ///
        /// The single write path, so the checksum, provenance and trace event
        /// can never be left out of an update path.
        fn set_market_metrics(&mut self, next: &MarketMetrics) {
            let previous = self.stored_market_metrics();
            let writer = self.env().caller();
            let now = self.env().block_timestamp();

            self.env().emit_event(MarketMetricsOverridden {
                updated_by: writer,
                previous_average_price: previous.average_price,
                previous_total_volume: previous.total_volume,
                previous_properties_listed: previous.properties_listed,
                new_average_price: next.average_price,
                new_total_volume: next.total_volume,
                new_properties_listed: next.properties_listed,
                updated_at: now,
            });

            self.current_metrics = next.clone();
            self.metrics_checksum = Self::metrics_checksum(&self.current_metrics);
            self.metrics_updated_at = now;
            self.metrics_updated_by = writer;
            self.metrics_update_count = self.metrics_update_count.saturating_add(1);
            self.metrics_is_override = true;
        }

        /// Combines partial metric contributions into one market view.
        ///
        /// Volume-weighted mean price, falling back to the unweighted mean when
        /// the entries carry no volume.
        fn combine_metric_updates(updates: &[MetricUpdate]) -> MarketMetrics {
            if updates.is_empty() {
                return MarketMetrics {
                    average_price: 0,
                    total_volume: 0,
                    properties_listed: 0,
                };
            }

            let total_volume: u128 = updates.iter().map(|u| u.total_volume).sum();
            let properties_listed: u64 = updates.iter().map(|u| u.properties_listed).sum();

            let average_price = if total_volume == 0 {
                // No volume to weight by, so fall back to a plain mean.
                let price_sum: u128 = updates.iter().map(|u| u.average_price).sum();
                price_sum / updates.len() as u128
            } else {
                // Each price is weighted by the volume it represents. Products
                // are accumulated in u128; the intermediate sum can exceed it
                // for large inputs, so fold in two steps to stay exact.
                let mut weighted_sum: u128 = 0;
                for u in updates.iter() {
                    weighted_sum = weighted_sum.saturating_add(
                        u.average_price
                            .saturating_mul(u.total_volume)
                            .checked_div(total_volume)
                            .unwrap_or(0),
                    );
                }
                weighted_sum
            };

            MarketMetrics {
                average_price,
                total_volume,
                properties_listed,
            }
        }

        #[ink(constructor)]
        pub fn new() -> Self {
            let caller = Self::env().caller();
            Self {
                admin: caller,
                current_metrics: MarketMetrics {
                    average_price: 0,
                    total_volume: 0,
                    properties_listed: 0,
                },
                metrics_checksum: Self::metrics_checksum(&MarketMetrics {
                    average_price: 0,
                    total_volume: 0,
                    properties_listed: 0,
                }),
                metrics_updated_at: 0,
                metrics_updated_by: caller,
                metrics_update_count: 0,
                metrics_is_override: false,
                historical_trends: ink::storage::Mapping::default(),
                trend_count: 0,
                property_sentiments: ink::storage::Mapping::default(),
                overall_sentiment: MarketSentiment {
                    bull_volume: 0,
                    bear_volume: 0,
                    bull_bear_ratio_bips: 5000,
                },
                portfolio_positions: ink::storage::Mapping::default(),
                property_type_trends: ink::storage::Mapping::default(),
                benchmark_indices: ink::storage::Mapping::default(),
                pending_admin_rotation: None,
            }
        }

        /// The current market metrics, exactly as stored.
        ///
        /// This is a faithful projection of `current_metrics`: it applies no
        /// defaulting, smoothing or clamping. The provenance of the value is
        /// reported separately by [`AnalyticsDashboard::get_metrics_provenance`]
        /// so a consumer can tell an admin-supplied figure from a
        /// contract-derived one instead of having to trust it.
        #[ink(message)]
        pub fn get_market_metrics(&self) -> MarketMetrics {
            self.stored_market_metrics()
        }

        /// Where the current `get_market_metrics` value came from.
        #[ink(message)]
        pub fn get_metrics_provenance(&self) -> MetricsProvenance {
            MetricsProvenance {
                metrics: self.stored_market_metrics(),
                updated_at: self.metrics_updated_at,
                updated_by: self.metrics_updated_by,
                update_count: self.metrics_update_count,
                is_override: self.metrics_is_override,
                is_intact: self.metrics_integrity_ok(),
            }
        }

        /// Recomputes the checksum over the stored metrics and compares it with
        /// the recorded one.
        ///
        /// Returns `false` if the stored metrics no longer match the checksum
        /// written alongside them, which is what a partial write or corrupted
        /// storage entry looks like. This is the on-chain expression of the
        /// "recompute == stored" invariant; the same comparison is asserted
        /// directly in the unit tests.
        #[ink(message)]
        pub fn verify_market_metrics_integrity(&self) -> bool {
            self.metrics_integrity_ok()
        }

        /// Overwrites the current market metrics. Admin only.
        ///
        /// # Override semantics
        ///
        /// There is no on-chain derivation path for these figures in this
        /// contract: it stores no property valuations, listing set or trade
        /// tape, so `average_price`, `total_volume` and `properties_listed` can
        /// only be supplied by the admin from an off-chain aggregation. This
        /// setter is therefore the single source of truth, and the value it
        /// writes is authoritative until it is written again.
        ///
        /// Consequences a consumer must account for:
        ///
        /// * `metrics_is_override` stays `true` after an admin write. Nothing
        ///   recomputes these numbers on chain, so a consumer that needs
        ///   independent verification has to recompute them off chain from the
        ///   same source and compare, or wait for an oracle integration to land
        ///   (not present in this contract; tracked as a follow-up).
        /// * Every write is traced: `MarketMetricsOverridden` carries the
        ///   previous and new values, and `get_metrics_provenance` reports the
        ///   writer, the timestamp and a lifetime update count. A silent
        ///   divergence between what was reported and what is stored is
        ///   therefore always attributable to a specific admin write.
        #[ink(message)]
        pub fn update_market_metrics(
            &mut self,
            average_price: u128,
            total_volume: u128,
            properties_listed: u64,
        ) -> Result<(), AnalyticsError> {
            self.ensure_admin()?;
            self.set_market_metrics(&MarketMetrics {
                average_price,
                total_volume,
                properties_listed,
            });
            Ok(())
        }

        /// Combines several metric contributions into one market view. Admin only.
        ///
        /// # Aggregation semantics
        ///
        /// Each entry contributes a partial view, so the entries are combined
        /// rather than overwriting one another:
        ///
        /// * `total_volume` is the sum of the entries' volumes.
        /// * `properties_listed` is the sum of the entries' counts.
        /// * `average_price` is the volume-weighted mean of the entries'
        ///   prices. When the entries carry no volume at all the weighting is
        ///   undefined, so it falls back to the unweighted mean.
        ///
        /// This previously overwrote `current_metrics` once per entry, so only
        /// the final entry survived while `BatchMetricsUpdated` reported the
        /// full count — a silent divergence between the event and the stored
        /// value. The event now carries the resulting metrics as well, so the
        /// two cannot disagree.
        #[ink(message)]
        pub fn batch_update_metrics(
            &mut self,
            updates: Vec<MetricUpdate>,
        ) -> Result<(), AnalyticsError> {
            self.ensure_admin()?;
            if updates.len() > MAX_BATCH_SIZE {
                return Err(AnalyticsError::BatchSizeExceeded);
            }

            let combined = Self::combine_metric_updates(&updates);
            self.set_market_metrics(&combined);
            self.env().emit_event(BatchMetricsUpdated {
                count: updates.len() as u64,
                result: combined,
            });
            Ok(())
        }

        /// Batch add multiple market trends in a single transaction.
        #[ink(message)]
        pub fn batch_add_trends(&mut self, trends: Vec<MarketTrend>) -> Result<(), AnalyticsError> {
            self.ensure_admin()?;
            if trends.len() > MAX_BATCH_SIZE {
                return Err(AnalyticsError::BatchSizeExceeded);
            }
            for trend in trends.iter() {
                self.historical_trends.insert(self.trend_count, trend);
                self.trend_count += 1;
            }
            self.env().emit_event(BatchTrendsAdded {
                count: trends.len() as u64,
            });
            Ok(())
        }

        /// Create market trend analysis with historical data.
        ///
        /// Admin only; returns [`AnalyticsError::Unauthorized`] for any other caller.
        #[ink(message)]
        pub fn add_market_trend(&mut self, trend: MarketTrend) -> Result<(), AnalyticsError> {
            self.ensure_admin()?;
            self.historical_trends.insert(self.trend_count, &trend);
            self.trend_count += 1;
            Ok(())
        }
        #[ink(message)]
        pub fn get_historical_trends(&self) -> Vec<MarketTrend> {
            let mut trends = Vec::new();
            for i in 0..self.trend_count {
                if let Some(trend) = self.historical_trends.get(i) {
                    trends.push(trend);
                }
            }
            trends
        }

        /// Derive human-readable insights from the report's own data instead
        /// of shipping a hardcoded sentence.
        ///
        /// The text reflects the latest trend direction (price / volume) and
        /// the aggregated crowd sentiment, so materially different market
        /// states produce different insights.
        fn derive_insights(&self, trend: &MarketTrend) -> String {
            let mut parts: Vec<String> = Vec::new();

            if trend.price_change_percentage > 0 {
                parts.push(String::from("prices are trending upward"));
            } else if trend.price_change_percentage < 0 {
                parts.push(String::from("prices are trending downward"));
            } else {
                parts.push(String::from("prices are stable"));
            }

            if trend.volume_change_percentage > 0 {
                parts.push(String::from("trading volume is increasing"));
            } else if trend.volume_change_percentage < 0 {
                parts.push(String::from("trading volume is decreasing"));
            } else {
                parts.push(String::from("trading volume is flat"));
            }

            let total_volume = self
                .overall_sentiment
                .bull_volume
                .saturating_add(self.overall_sentiment.bear_volume);
            if total_volume == 0 {
                parts.push(String::from("no crowd sentiment data available yet"));
            } else if self.overall_sentiment.bull_volume > self.overall_sentiment.bear_volume {
                parts.push(String::from("crowd sentiment leans bullish"));
            } else if self.overall_sentiment.bear_volume > self.overall_sentiment.bull_volume {
                parts.push(String::from("crowd sentiment leans bearish"));
            } else {
                parts.push(String::from("crowd sentiment is evenly split"));
            }

            let mut text = parts.join(", ");
            text.push('.');
            text
        }

        /// Create automated market reports generation
        #[ink(message)]
        pub fn generate_market_report(&self) -> MarketReport {
            let latest_trend = if self.trend_count > 0 {
                self.historical_trends
                    .get(self.trend_count - 1)
                    .unwrap_or(MarketTrend {
                        period_start: 0,
                        period_end: 0,
                        price_change_percentage: 0,
                        volume_change_percentage: 0,
                    })
            } else {
                MarketTrend {
                    period_start: 0,
                    period_end: 0,
                    price_change_percentage: 0,
                    volume_change_percentage: 0,
                }
            };

            let insights = self.derive_insights(&latest_trend);
            MarketReport {
                generated_at: self.env().block_timestamp(),
                metrics: self.current_metrics.clone(),
                trend: latest_trend,
                sentiment: self.overall_sentiment.clone(),
                insights,
            }
        }

        /// Update market sentiment from prediction markets.
        ///
        /// Admin only (or an authorized prediction-market integration once such
        /// a role exists); returns [`AnalyticsError::Unauthorized`] otherwise.
        #[ink(message)]
        pub fn update_market_sentiment(
            &mut self,
            property_id: u64,
            bull_volume: u128,
            bear_volume: u128,
        ) -> Result<(), AnalyticsError> {
            self.ensure_admin()?; // Prediction market or admin updates this
            let total_volume = bull_volume + bear_volume;
            let ratio = (bull_volume * 10000)
                .checked_div(total_volume)
                .map(|n| n as u32)
                .unwrap_or(5000); // default unbiased

            let new_sentiment = MarketSentiment {
                bull_volume,
                bear_volume,
                bull_bear_ratio_bips: ratio,
            };

            self.property_sentiments.insert(property_id, &new_sentiment);

            // Update overall recursively or by moving average
            self.overall_sentiment.bull_volume = self
                .overall_sentiment
                .bull_volume
                .saturating_add(bull_volume);
            self.overall_sentiment.bear_volume = self
                .overall_sentiment
                .bear_volume
                .saturating_add(bear_volume);

            let total_overall =
                self.overall_sentiment.bull_volume + self.overall_sentiment.bear_volume;
            // Preserve previous ratio when total is zero (matches original `if total_overall > 0` skip semantics).
            self.overall_sentiment.bull_bear_ratio_bips = (self.overall_sentiment.bull_volume
                * 10000)
                .checked_div(total_overall)
                .map(|n| n as u32)
                .unwrap_or(self.overall_sentiment.bull_bear_ratio_bips);
            Ok(())
        }

        /// Update portfolio positions for an owner.
        ///
        /// Admin only; returns [`AnalyticsError::Unauthorized`] for any other caller.
        #[ink(message)]
        pub fn set_portfolio_positions(
            &mut self,
            owner: AccountId,
            positions: Vec<PortfolioPosition>,
        ) -> Result<(), AnalyticsError> {
            self.ensure_admin()?;
            self.portfolio_positions.insert(owner, &positions);
            Ok(())
        }

        /// Retrieve portfolio positions for an owner.
        #[ink(message)]
        pub fn get_portfolio_positions(&self, owner: AccountId) -> Vec<PortfolioPosition> {
            self.portfolio_positions.get(owner).unwrap_or_default()
        }

        /// Update property-type market trends used for portfolio rebalancing recommendations.
        ///
        /// Admin only; returns [`AnalyticsError::Unauthorized`] for any other caller.
        #[ink(message)]
        pub fn update_property_type_trend(
            &mut self,
            property_type: propchain_traits::PropertyType,
            trend: MarketTrend,
        ) -> Result<(), AnalyticsError> {
            self.ensure_admin()?;
            self.property_type_trends.insert(property_type, &trend);
            Ok(())
        }

        /// Get the stored market trend for a specific property type.
        #[ink(message)]
        pub fn get_property_type_trend(
            &self,
            property_type: propchain_traits::PropertyType,
        ) -> MarketTrend {
            self.property_type_trends
                .get(property_type)
                .unwrap_or(MarketTrend {
                    period_start: 0,
                    period_end: 0,
                    price_change_percentage: 0,
                    volume_change_percentage: 0,
                })
        }

        /// Update the benchmark index for a property type against a basket of reference indices.
        ///
        /// Admin only; returns [`AnalyticsError::Unauthorized`] for any other caller.
        #[ink(message)]
        pub fn update_benchmark_index(
            &mut self,
            property_type: propchain_traits::PropertyType,
            performance_change_percentage: i32,
        ) -> Result<(), AnalyticsError> {
            self.ensure_admin()?;
            self.benchmark_indices
                .insert(property_type, &performance_change_percentage);
            Ok(())
        }

        /// Get the stored benchmark index for a property type.
        #[ink(message)]
        pub fn get_benchmark_index(&self, property_type: propchain_traits::PropertyType) -> i32 {
            self.benchmark_indices.get(property_type).unwrap_or(0)
        }

        /// Get portfolio rebalancing suggestions for an owner.
        #[ink(message)]
        pub fn get_rebalancing_suggestions(&self, owner: AccountId) -> Vec<RebalancingSuggestion> {
            let positions = self.get_portfolio_positions(owner);
            let total_value: u128 = positions.iter().map(|p| p.value).sum();
            if total_value == 0 {
                return Vec::new();
            }

            let mut target_scores = Vec::new();
            let mut total_score: u128 = 0;
            for position in positions.iter() {
                let trend = self.get_property_type_trend(position.property_type.clone());
                let benchmark_change = self.get_benchmark_index(position.property_type.clone());
                let normalized_trend = 100i128
                    + (trend.price_change_percentage as i128 * 2)
                    + (trend.volume_change_percentage as i128 / 5);
                let benchmark_gap = (trend.price_change_percentage as i128
                    - benchmark_change as i128)
                    .saturating_mul(3)
                    + (trend.volume_change_percentage as i128 / 2);
                let score = (normalized_trend - benchmark_gap).clamp(50, 150) as u128;
                target_scores.push((position.property_type.clone(), score));
                total_score = total_score.saturating_add(score);
            }

            target_scores
                .into_iter()
                .map(|(property_type, score)| {
                    let target_bips = (score * 10000)
                        .checked_div(total_score)
                        .map(|n| n as u32)
                        .unwrap_or(0);
                    let current_value = positions
                        .iter()
                        .find(|p| p.property_type == property_type)
                        .map(|p| p.value)
                        .unwrap_or(0);
                    let current_bips = ((current_value * 10000) / total_value) as u32;
                    let diff = current_bips as i32 - target_bips as i32;
                    let benchmark_change = self.get_benchmark_index(property_type.clone());
                    let benchmark_gap = (self
                        .get_property_type_trend(property_type.clone())
                        .price_change_percentage
                        - benchmark_change)
                        .abs();
                    let recommendation = if diff > 200 {
                        String::from(
                            "Overweight: reduce exposure because this allocation is above the benchmark-based target.",
                        )
                    } else if diff < -200 {
                        String::from(
                            "Underweight: increase exposure because this allocation is below the benchmark-based target.",
                        )
                    } else if benchmark_gap > 4 {
                        String::from(
                            "Benchmark lag: rebalance toward stronger-performing segments.",
                        )
                    } else {
                        String::from("Aligned with target allocation and benchmark index.")
                    };
                    RebalancingSuggestion {
                        property_type,
                        current_allocation_bips: current_bips,
                        target_allocation_bips: target_bips,
                        recommendation,
                    }
                })
                .collect()
        }

        /// Get a health score for an owner's portfolio from 0 to 100.
        #[ink(message)]
        pub fn get_portfolio_health_score(&self, owner: AccountId) -> u8 {
            let positions = self.get_portfolio_positions(owner);
            let total_value: u128 = positions.iter().map(|p| p.value).sum();
            if total_value == 0 {
                return 0;
            }

            let mut distinct_types: Vec<propchain_traits::PropertyType> = Vec::new();
            let mut max_share_bips: u32 = 0;
            let mut trend_total: i32 = 0;
            let mut benchmark_penalty: i32 = 0;
            for position in positions.iter() {
                if !distinct_types.contains(&position.property_type) {
                    distinct_types.push(position.property_type.clone());
                }
                let share_bips = ((position.value * 10000) / total_value) as u32;
                max_share_bips = max_share_bips.max(share_bips);
                let trend = self.get_property_type_trend(position.property_type.clone());
                let benchmark_change = self.get_benchmark_index(position.property_type.clone());
                trend_total += trend.price_change_percentage;
                benchmark_penalty +=
                    ((trend.price_change_percentage - benchmark_change).abs() / 2).min(15);
            }

            let distinct_bonus = (distinct_types.len() as u8).saturating_mul(10).min(40);
            let concentration_penalty = if max_share_bips > 4000 {
                ((max_share_bips - 4000) / 100) as u8
            } else {
                0
            };
            let trend_bonus =
                (trend_total / (distinct_types.len().max(1) as i32)).clamp(-10, 10) as i8;
            let mut score = 50i32 + distinct_bonus as i32 - concentration_penalty as i32
                + trend_bonus as i32
                - benchmark_penalty;
            score = score.clamp(0, 100);
            score as u8
        }

        /// Add gas usage optimization recommendations
        #[ink(message)]
        pub fn get_gas_optimization_recommendations(&self) -> String {
            String::from("Use batched operations and limit nested looping over dynamic collections (e.g. vectors). Store large items in Mappings instead of Vecs.")
        }

        /// Get admin address
        #[ink(message)]
        pub fn get_admin(&self) -> AccountId {
            self.admin
        }

        /// Ensure only the admin can modify metrics.
        ///
        /// Returns a typed [`AnalyticsError::Unauthorized`] instead of
        /// panicking so integrators can distinguish authorization failures.
        fn ensure_admin(&self) -> Result<(), AnalyticsError> {
            if self.env().caller() != self.admin {
                return Err(AnalyticsError::Unauthorized);
            }
            Ok(())
        }

        // ── Admin Key Rotation (Issue #496) ──────────────────────────────────

        /// Initiate two-step admin rotation with timelock cooldown.
        ///
        /// Only the current admin may call this. The nominated `new_admin` must
        /// confirm after `KEY_ROTATION_COOLDOWN_BLOCKS` blocks have elapsed.
        #[ink(message)]
        pub fn request_admin_rotation(
            &mut self,
            new_admin: AccountId,
        ) -> Result<(), AnalyticsError> {
            let caller = self.env().caller();
            if caller != self.admin {
                return Err(AnalyticsError::Unauthorized);
            }
            if self.pending_admin_rotation.is_some() {
                return Err(AnalyticsError::KeyRotationCooldown);
            }

            let block = self.env().block_number();
            let effective_at =
                block.saturating_add(propchain_traits::constants::KEY_ROTATION_COOLDOWN_BLOCKS);

            self.pending_admin_rotation = Some(propchain_traits::KeyRotationRequest {
                old_account: caller,
                new_account: new_admin,
                requested_at: block,
                effective_at,
                confirmed: false,
            });

            self.env().emit_event(AdminRotationRequested {
                old_admin: caller,
                new_admin,
                effective_at_block: effective_at,
            });
            Ok(())
        }

        /// Confirm a pending admin rotation after the cooldown period.
        ///
        /// Must be called by the nominated new admin.
        #[ink(message)]
        pub fn confirm_admin_rotation(&mut self) -> Result<(), AnalyticsError> {
            let caller = self.env().caller();
            let block = self.env().block_number();

            let request = self
                .pending_admin_rotation
                .as_ref()
                .ok_or(AnalyticsError::NoPendingRotation)?;

            if request.new_account != caller {
                return Err(AnalyticsError::RotationUnauthorized);
            }
            if block < request.effective_at {
                return Err(AnalyticsError::KeyRotationCooldown);
            }
            let expiry = request
                .effective_at
                .saturating_add(propchain_traits::constants::KEY_ROTATION_EXPIRY_BLOCKS);
            if block > expiry {
                self.pending_admin_rotation = None;
                return Err(AnalyticsError::RequestExpired);
            }

            let old_admin = request.old_account;
            self.admin = caller;
            self.pending_admin_rotation = None;

            self.env().emit_event(AdminRotationConfirmed {
                old_admin,
                new_admin: caller,
            });
            Ok(())
        }

        /// Cancel a pending admin rotation.
        ///
        /// Either the current admin or the nominated new admin may cancel.
        #[ink(message)]
        pub fn cancel_admin_rotation(&mut self) -> Result<(), AnalyticsError> {
            let caller = self.env().caller();
            let request = self
                .pending_admin_rotation
                .as_ref()
                .ok_or(AnalyticsError::NoPendingRotation)?;

            if caller != request.old_account && caller != request.new_account {
                return Err(AnalyticsError::RotationUnauthorized);
            }

            let old_admin = request.old_account;
            self.pending_admin_rotation = None;

            self.env().emit_event(AdminRotationCancelled {
                old_admin,
                cancelled_by: caller,
            });
            Ok(())
        }

        /// Get the pending admin rotation request, if any.
        #[ink(message)]
        pub fn get_pending_admin_rotation(&self) -> Option<propchain_traits::KeyRotationRequest> {
            self.pending_admin_rotation.clone()
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        fn trend(price: i32, volume: i32) -> MarketTrend {
            MarketTrend {
                period_start: 0,
                period_end: 100,
                price_change_percentage: price,
                volume_change_percentage: volume,
            }
        }

        /// Insights must be derived from report data, not hardcoded:
        /// materially different market states produce different text.
        #[ink::test]
        fn insights_differ_between_market_states() {
            let mut bullish = AnalyticsDashboard::new();
            bullish
                .add_market_trend(trend(5, 10))
                .expect("trend accepted");
            bullish
                .update_market_sentiment(1, 800, 200)
                .expect("sentiment accepted");
            let bull_report = bullish.generate_market_report();
            assert!(bull_report.insights.contains("upward"));
            assert!(bull_report.insights.contains("increasing"));
            assert!(bull_report.insights.contains("bullish"));

            let mut bearish = AnalyticsDashboard::new();
            bearish
                .add_market_trend(trend(-7, -3))
                .expect("trend accepted");
            bearish
                .update_market_sentiment(1, 150, 850)
                .expect("sentiment accepted");
            let bear_report = bearish.generate_market_report();
            assert!(bear_report.insights.contains("downward"));
            assert!(bear_report.insights.contains("decreasing"));
            assert!(bear_report.insights.contains("bearish"));
            assert_ne!(bull_report.insights, bear_report.insights);
        }

        #[ink::test]
        fn insights_without_data_mention_stability_and_missing_sentiment() {
            let contract = AnalyticsDashboard::new();
            let report = contract.generate_market_report();
            assert!(report.insights.contains("stable"));
            assert!(report.insights.contains("no crowd sentiment data"));
        }

        type Environment = ink::env::DefaultEnvironment;

        fn accounts() -> ink::env::test::DefaultAccounts<Environment> {
            ink::env::test::default_accounts::<Environment>()
        }

        fn set_caller(caller: AccountId) {
            ink::env::test::set_caller::<Environment>(caller);
        }

        fn sample_trend() -> MarketTrend {
            MarketTrend {
                period_start: 1_000,
                period_end: 2_000,
                price_change_percentage: 5,
                volume_change_percentage: 10,
            }
        }

        fn admin_contract() -> AnalyticsDashboard {
            AnalyticsDashboard::new()
        }

        #[ink::test]
        fn unauthorized_update_market_metrics_gets_typed_error() {
            let accounts = accounts();
            let mut c = admin_contract();
            set_caller(accounts.bob);
            assert_eq!(
                c.update_market_metrics(100, 200, 3),
                Err(AnalyticsError::Unauthorized)
            );
        }

        #[ink::test]
        fn unauthorized_batch_update_metrics_gets_typed_error() {
            let accounts = accounts();
            let mut c = admin_contract();
            set_caller(accounts.bob);
            assert_eq!(
                c.batch_update_metrics(Vec::new()),
                Err(AnalyticsError::Unauthorized)
            );
        }

        #[ink::test]
        fn unauthorized_batch_add_trends_gets_typed_error() {
            let accounts = accounts();
            let mut c = admin_contract();
            set_caller(accounts.bob);
            assert_eq!(
                c.batch_add_trends(Vec::new()),
                Err(AnalyticsError::Unauthorized)
            );
        }

        #[ink::test]
        fn unauthorized_add_market_trend_gets_typed_error() {
            let accounts = accounts();
            let mut c = admin_contract();
            set_caller(accounts.bob);
            assert_eq!(
                c.add_market_trend(sample_trend()),
                Err(AnalyticsError::Unauthorized)
            );
            assert_eq!(c.get_historical_trends().len(), 0);
        }

        #[ink::test]
        fn unauthorized_update_market_sentiment_gets_typed_error() {
            let accounts = accounts();
            let mut c = admin_contract();
            set_caller(accounts.bob);
            assert_eq!(
                c.update_market_sentiment(1, 100, 100),
                Err(AnalyticsError::Unauthorized)
            );
        }

        #[ink::test]
        fn unauthorized_set_portfolio_positions_gets_typed_error() {
            let accounts = accounts();
            let mut c = admin_contract();
            set_caller(accounts.bob);
            assert_eq!(
                c.set_portfolio_positions(accounts.alice, Vec::new()),
                Err(AnalyticsError::Unauthorized)
            );
        }

        #[ink::test]
        fn unauthorized_update_property_type_trend_gets_typed_error() {
            let accounts = accounts();
            let mut c = admin_contract();
            set_caller(accounts.bob);
            assert_eq!(
                c.update_property_type_trend(
                    propchain_traits::PropertyType::Residential,
                    sample_trend()
                ),
                Err(AnalyticsError::Unauthorized)
            );
        }

        #[ink::test]
        fn unauthorized_update_benchmark_index_gets_typed_error() {
            let accounts = accounts();
            let mut c = admin_contract();
            set_caller(accounts.bob);
            assert_eq!(
                c.update_benchmark_index(propchain_traits::PropertyType::Residential, 7),
                Err(AnalyticsError::Unauthorized)
            );
        }

        #[ink::test]
        fn admin_succeeds_on_all_gated_messages() {
            let accounts = accounts();
            let mut c = admin_contract();
            assert_eq!(c.update_market_metrics(150, 300, 4), Ok(()));
            assert_eq!(c.get_market_metrics().average_price, 150);
            assert_eq!(c.add_market_trend(sample_trend()), Ok(()));
            assert_eq!(c.get_historical_trends().len(), 1);
            assert_eq!(
                c.batch_update_metrics(vec![MetricUpdate {
                    average_price: 160,
                    total_volume: 320,
                    properties_listed: 5
                }]),
                Ok(())
            );
            assert_eq!(c.batch_add_trends(vec![sample_trend()]), Ok(()));
            assert_eq!(c.get_historical_trends().len(), 2);
            assert_eq!(c.update_market_sentiment(1, 400, 100), Ok(()));
            assert_eq!(c.overall_sentiment.bull_volume, 400);
            let positions = vec![PortfolioPosition {
                property_type: propchain_traits::PropertyType::Residential,
                value: 1_000,
            }];
            assert_eq!(c.set_portfolio_positions(accounts.alice, positions), Ok(()));
            assert_eq!(c.get_portfolio_positions(accounts.alice).len(), 1);
            assert_eq!(
                c.update_property_type_trend(
                    propchain_traits::PropertyType::Commercial,
                    sample_trend()
                ),
                Ok(())
            );
            assert_eq!(
                c.get_property_type_trend(propchain_traits::PropertyType::Commercial)
                    .price_change_percentage,
                5
            );
            assert_eq!(
                c.update_benchmark_index(propchain_traits::PropertyType::Commercial, 9),
                Ok(())
            );
            assert_eq!(
                c.get_benchmark_index(propchain_traits::PropertyType::Commercial),
                9
            );
        }

        // =====================================================================
        // Market metrics consistency and override tracing (issue #1195)
        // =====================================================================

        fn update(average_price: u128, total_volume: u128, properties_listed: u64) -> MetricUpdate {
            MetricUpdate {
                average_price,
                total_volume,
                properties_listed,
            }
        }

        /// The core invariant: what the getter reports equals what storage holds,
        /// and the stored value still matches the checksum written with it.
        #[ink::test]
        fn reported_metrics_match_stored_metrics() {
            let mut c = admin_contract();
            assert_eq!(c.update_market_metrics(150, 300, 4), Ok(()));

            // Recomputed independently from the same values the admin supplied.
            let expected = MarketMetrics {
                average_price: 150,
                total_volume: 300,
                properties_listed: 4,
            };
            assert_eq!(c.get_market_metrics(), expected);
            assert!(c.verify_market_metrics_integrity());
        }

        #[ink::test]
        fn integrity_holds_after_every_update_path() {
            let mut c = admin_contract();
            assert!(c.verify_market_metrics_integrity(), "fresh deploy");

            c.update_market_metrics(150, 300, 4).unwrap();
            assert!(c.verify_market_metrics_integrity(), "single update");

            c.batch_update_metrics(vec![update(10, 100, 1), update(30, 300, 3)])
                .unwrap();
            assert!(
                c.verify_market_metrics_integrity(),
                "batch update must leave a matching checksum"
            );
        }

        #[ink::test]
        fn integrity_detects_a_mismatched_checksum() {
            let mut c = admin_contract();
            c.update_market_metrics(150, 300, 4).unwrap();
            assert!(c.verify_market_metrics_integrity());

            // Simulate a partial or corrupted write to the stored metrics.
            c.current_metrics.average_price = 999;
            assert!(
                !c.verify_market_metrics_integrity(),
                "a value that no longer matches its checksum must be reported"
            );
            assert!(!c.get_metrics_provenance().is_intact);
        }

        #[ink::test]
        fn checksum_covers_every_field() {
            let base = MarketMetrics {
                average_price: 150,
                total_volume: 300,
                properties_listed: 4,
            };
            let base_sum = AnalyticsDashboard::metrics_checksum(&base);

            let mut differs = base.clone();
            differs.average_price += 1;
            assert_ne!(base_sum, AnalyticsDashboard::metrics_checksum(&differs));

            let mut differs = base.clone();
            differs.total_volume += 1;
            assert_ne!(base_sum, AnalyticsDashboard::metrics_checksum(&differs));

            let mut differs = base.clone();
            differs.properties_listed += 1;
            assert_ne!(base_sum, AnalyticsDashboard::metrics_checksum(&differs));
        }

        #[ink::test]
        fn override_is_recorded_in_provenance() {
            let accounts = accounts();
            let mut c = admin_contract();

            // Fresh metrics are not an override: nothing has overridden them.
            assert!(!c.get_metrics_provenance().is_override);
            assert_eq!(c.get_metrics_provenance().update_count, 0);

            c.update_market_metrics(150, 300, 4).unwrap();
            let p = c.get_metrics_provenance();
            assert!(p.is_override, "an admin write is an override");
            assert_eq!(p.update_count, 1);
            assert_eq!(p.updated_by, accounts.alice, "the writer is recorded");
            assert_eq!(p.metrics.average_price, 150);
        }

        #[ink::test]
        fn every_write_is_traced_so_divergence_is_attributable() {
            let mut c = admin_contract();
            c.update_market_metrics(150, 300, 4).unwrap();

            ink::env::test::set_block_timestamp::<Environment>(1_700_000);
            c.update_market_metrics(160, 320, 5).unwrap();
            c.batch_update_metrics(vec![update(200, 400, 6)]).unwrap();

            // A lifetime count plus the last writer and timestamp is what makes
            // an unexplained change in the reported number traceable.
            let p = c.get_metrics_provenance();
            assert_eq!(p.update_count, 3);
            assert_eq!(p.updated_at, 1_700_000, "last write is timestamped");
        }

        #[ink::test]
        fn provenance_reports_the_live_value() {
            let mut c = admin_contract();
            c.update_market_metrics(150, 300, 4).unwrap();
            assert_eq!(c.get_metrics_provenance().metrics, c.get_market_metrics());
        }

        // --- batch aggregation (previously kept only the last entry) ---

        #[ink::test]
        fn batch_update_combines_entries_instead_of_discarding_them() {
            let mut c = admin_contract();
            c.batch_update_metrics(vec![update(100, 1_000, 1), update(300, 3_000, 3)])
                .unwrap();

            let m = c.get_market_metrics();
            assert_eq!(m.total_volume, 4_000, "volumes must sum");
            assert_eq!(m.properties_listed, 4, "counts must sum");
            // Volume-weighted mean: (100*1000 + 300*3000) / 4000 = 250.
            assert_eq!(m.average_price, 250);
        }

        #[ink::test]
        fn batch_update_keeps_the_final_entry_when_there_is_one() {
            let mut c = admin_contract();
            c.batch_update_metrics(vec![update(160, 320, 5)]).unwrap();
            let m = c.get_market_metrics();
            assert_eq!(m.average_price, 160);
            assert_eq!(m.total_volume, 320);
            assert_eq!(m.properties_listed, 5);
        }

        #[ink::test]
        fn batch_update_falls_back_to_unweighted_mean_without_volume() {
            let mut c = admin_contract();
            // Weighting by zero volume is undefined, so use a plain mean.
            c.batch_update_metrics(vec![update(100, 0, 1), update(300, 0, 1)])
                .unwrap();
            assert_eq!(c.get_market_metrics().average_price, 200);
        }

        #[ink::test]
        fn batch_update_of_nothing_leaves_zeroed_metrics() {
            let mut c = admin_contract();
            c.batch_update_metrics(vec![]).unwrap();
            let m = c.get_market_metrics();
            assert_eq!(m.average_price, 0);
            assert_eq!(m.total_volume, 0);
            assert_eq!(m.properties_listed, 0);
            assert!(c.verify_market_metrics_integrity());
        }

        #[ink::test]
        fn batch_update_still_rejects_oversized_batches() {
            let mut c = admin_contract();
            let too_many: Vec<MetricUpdate> =
                (0..=MAX_BATCH_SIZE).map(|_| update(1, 1, 1)).collect();
            assert_eq!(
                c.batch_update_metrics(too_many),
                Err(AnalyticsError::BatchSizeExceeded)
            );
        }

        #[ink::test]
        fn non_admin_cannot_write_metrics() {
            let accounts = accounts();
            let mut c = admin_contract();
            set_caller(accounts.bob);
            assert_eq!(
                c.update_market_metrics(1, 1, 1),
                Err(AnalyticsError::Unauthorized)
            );
            assert_eq!(
                c.batch_update_metrics(vec![update(1, 1, 1)]),
                Err(AnalyticsError::Unauthorized)
            );
            // The rejected writes must not have moved the stored value.
            assert_eq!(c.get_market_metrics().average_price, 0);
            assert!(c.verify_market_metrics_integrity());
        }
    }
}
