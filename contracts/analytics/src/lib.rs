#![cfg_attr(not(feature = "std"), no_std)]
#![allow(unexpected_cfgs)]
#![allow(clippy::new_without_default)]

use ink::prelude::string::String;
use ink::prelude::vec::Vec;
use propchain_traits;

#[ink::contract]
mod propchain_analytics {
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

    impl AnalyticsDashboard {
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
                pending_admin_rotation: None,
            }
        }

        /// Implement property market metrics calculation (average price, volume, etc.)
        #[ink(message)]
        pub fn get_market_metrics(&self) -> MarketMetrics {
            self.current_metrics.clone()
        }

        #[ink(message)]
        pub fn update_market_metrics(
            &mut self,
            average_price: u128,
            total_volume: u128,
            properties_listed: u64,
        ) {
            self.ensure_admin();
            self.current_metrics = MarketMetrics {
                average_price,
                total_volume,
                properties_listed,
            };
        }

        /// Create market trend analysis with historical data
        #[ink(message)]
        pub fn add_market_trend(&mut self, trend: MarketTrend) {
            self.ensure_admin();
            self.historical_trends.insert(self.trend_count, &trend);
            self.trend_count += 1;
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

            MarketReport {
                generated_at: self.env().block_timestamp(),
                metrics: self.current_metrics.clone(),
                trend: latest_trend,
                sentiment: self.overall_sentiment.clone(),
                insights: String::from(
                    "Market is relatively stable. Gas optimization is recommended.",
                ),
            }
        }

        /// Update market sentiment from prediction markets
        #[ink(message)]
        pub fn update_market_sentiment(
            &mut self,
            property_id: u64,
            bull_volume: u128,
            bear_volume: u128,
        ) {
            self.ensure_admin(); // Prediction market or admin updates this
            let total_volume = bull_volume + bear_volume;
            let ratio = if total_volume > 0 {
                ((bull_volume * 10000) / total_volume) as u32
            } else {
                5000 // default unbiased
            };

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
            if total_overall > 0 {
                self.overall_sentiment.bull_bear_ratio_bips =
                    ((self.overall_sentiment.bull_volume * 10000) / total_overall) as u32;
            }
        }

        /// Update portfolio positions for an owner.
        #[ink(message)]
        pub fn set_portfolio_positions(&mut self, owner: AccountId, positions: Vec<PortfolioPosition>) {
            self.ensure_admin();
            self.portfolio_positions.insert(owner, &positions);
        }

        /// Retrieve portfolio positions for an owner.
        #[ink(message)]
        pub fn get_portfolio_positions(&self, owner: AccountId) -> Vec<PortfolioPosition> {
            self.portfolio_positions.get(owner).unwrap_or_default()
        }

        /// Update property-type market trends used for portfolio rebalancing recommendations.
        #[ink(message)]
        pub fn update_property_type_trend(
            &mut self,
            property_type: propchain_traits::PropertyType,
            trend: MarketTrend,
        ) {
            self.ensure_admin();
            self.property_type_trends.insert(property_type, &trend);
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

        /// Get portfolio rebalancing suggestions for an owner.
        #[ink(message)]
        pub fn get_rebalancing_suggestions(
            &self,
            owner: AccountId,
        ) -> Vec<RebalancingSuggestion> {
            let positions = self.get_portfolio_positions(owner);
            let total_value: u128 = positions.iter().map(|p| p.value).sum();
            if total_value == 0 {
                return Vec::new();
            }

            let mut target_scores = Vec::new();
            let mut total_score: u128 = 0;
            for position in positions.iter() {
                let trend = self.get_property_type_trend(position.property_type.clone());
                let normalized_trend = 100i128
                    + (trend.price_change_percentage as i128 * 2)
                    + (trend.volume_change_percentage as i128 / 5);
                let score = normalized_trend.clamp(50, 150) as u128;
                target_scores.push((position.property_type.clone(), score));
                total_score = total_score.saturating_add(score);
            }

            target_scores
                .into_iter()
                .map(|(property_type, score)| {
                    let target_bips = if total_score > 0 {
                        ((score * 10000) / total_score) as u32
                    } else {
                        0
                    };
                    let current_value = positions
                        .iter()
                        .find(|p| p.property_type == property_type)
                        .map(|p| p.value)
                        .unwrap_or(0);
                    let current_bips = ((current_value * 10000) / total_value) as u32;
                    let diff = current_bips as i32 - target_bips as i32;
                    let recommendation = if diff > 200 {
                        String::from("Overweight: consider reducing exposure for this property type.")
                    } else if diff < -200 {
                        String::from("Underweight: consider increasing exposure for this property type.")
                    } else {
                        String::from("Aligned with target allocation.")
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
            for position in positions.iter() {
                if !distinct_types.contains(&position.property_type) {
                    distinct_types.push(position.property_type.clone());
                }
                let share_bips = ((position.value * 10000) / total_value) as u32;
                max_share_bips = max_share_bips.max(share_bips);
                let trend = self.get_property_type_trend(position.property_type.clone());
                trend_total += trend.price_change_percentage as i32;
            }

            let distinct_bonus = (distinct_types.len() as u8).saturating_mul(10).min(40);
            let concentration_penalty = if max_share_bips > 4000 {
                ((max_share_bips - 4000) / 100) as u8
            } else {
                0
            };
            let trend_bonus = ((trend_total as i32) / (distinct_types.len().max(1) as i32)).clamp(-10, 10) as i8;
            let mut score = 50i32 + distinct_bonus as i32 - concentration_penalty as i32 + trend_bonus as i32;
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

        /// Ensure only the admin can modify metrics
        fn ensure_admin(&self) {
            assert_eq!(
                self.env().caller(),
                self.admin,
                "Unauthorized: Analytics admin only"
            );
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
            let effective_at = block
                .saturating_add(propchain_traits::constants::KEY_ROTATION_COOLDOWN_BLOCKS);

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
        pub fn get_pending_admin_rotation(
            &self,
        ) -> Option<propchain_traits::KeyRotationRequest> {
            self.pending_admin_rotation.clone()
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[ink::test]
        fn market_metrics_defaults() {
            let contract = AnalyticsDashboard::new();
            let metrics = contract.get_market_metrics();
            assert_eq!(metrics.average_price, 0);
            assert_eq!(metrics.total_volume, 0);
            assert_eq!(metrics.properties_listed, 0);
        }

        #[ink::test]
        fn update_market_metrics_works() {
            let mut contract = AnalyticsDashboard::new();
            contract.update_market_metrics(1000, 5000, 10);
            let metrics = contract.get_market_metrics();
            assert_eq!(metrics.average_price, 1000);
            assert_eq!(metrics.total_volume, 5000);
            assert_eq!(metrics.properties_listed, 10);
        }

        #[ink::test]
        fn add_market_trend_works() {
            let mut contract = AnalyticsDashboard::new();
            let trend = MarketTrend {
                period_start: 100,
                period_end: 200,
                price_change_percentage: 5,
                volume_change_percentage: 10,
            };
            contract.add_market_trend(trend.clone());
            let trends = contract.get_historical_trends();
            assert_eq!(trends.len(), 1);
            assert_eq!(trends[0].price_change_percentage, 5);
        }

        #[ink::test]
        fn generate_market_report_works() {
            let contract = AnalyticsDashboard::new();
            let report = contract.generate_market_report();
            assert_eq!(report.metrics.average_price, 0);
            assert_eq!(report.sentiment.bull_bear_ratio_bips, 5000);
            assert!(report.insights.contains("Gas optimization"));
        }

        #[ink::test]
        fn portfolio_health_score_is_low_for_concentrated_portfolio() {
            let mut contract = AnalyticsDashboard::new();
            let owner = contract.env().caller();
            let positions = vec![PortfolioPosition {
                property_type: propchain_traits::PropertyType::Residential,
                value: 900,
            }];
            contract.set_portfolio_positions(owner, positions);
            contract.update_property_type_trend(
                propchain_traits::PropertyType::Residential,
                MarketTrend {
                    period_start: 0,
                    period_end: 0,
                    price_change_percentage: -5,
                    volume_change_percentage: 0,
                },
            );
            let score = contract.get_portfolio_health_score(owner);
            assert!(score < 50, "Expected low score for concentrated, weak trend");
        }

        #[ink::test]
        fn portfolio_health_score_is_high_for_diversified_portfolio() {
            let mut contract = AnalyticsDashboard::new();
            let owner = contract.env().caller();
            let positions = vec![
                PortfolioPosition {
                    property_type: propchain_traits::PropertyType::Residential,
                    value: 400,
                },
                PortfolioPosition {
                    property_type: propchain_traits::PropertyType::Commercial,
                    value: 300,
                },
                PortfolioPosition {
                    property_type: propchain_traits::PropertyType::Industrial,
                    value: 300,
                },
            ];
            contract.set_portfolio_positions(owner, positions);
            contract.update_property_type_trend(
                propchain_traits::PropertyType::Residential,
                MarketTrend {
                    period_start: 0,
                    period_end: 0,
                    price_change_percentage: 8,
                    volume_change_percentage: 2,
                },
            );
            contract.update_property_type_trend(
                propchain_traits::PropertyType::Commercial,
                MarketTrend {
                    period_start: 0,
                    period_end: 0,
                    price_change_percentage: 6,
                    volume_change_percentage: 3,
                },
            );
            contract.update_property_type_trend(
                propchain_traits::PropertyType::Industrial,
                MarketTrend {
                    period_start: 0,
                    period_end: 0,
                    price_change_percentage: 7,
                    volume_change_percentage: 1,
                },
            );
            let score = contract.get_portfolio_health_score(owner);
            assert!(score >= 60, "Expected higher score for diversified portfolio");
        }

        #[ink::test]
        fn rebalancing_suggestions_change_when_market_trends_shift() {
            let mut contract = AnalyticsDashboard::new();
            let owner = contract.env().caller();
            let positions = vec![
                PortfolioPosition {
                    property_type: propchain_traits::PropertyType::Residential,
                    value: 500,
                },
                PortfolioPosition {
                    property_type: propchain_traits::PropertyType::Commercial,
                    value: 500,
                },
            ];

            contract.set_portfolio_positions(owner, positions.clone());
            contract.update_property_type_trend(
                propchain_traits::PropertyType::Residential,
                MarketTrend {
                    period_start: 0,
                    period_end: 0,
                    price_change_percentage: 20,
                    volume_change_percentage: 10,
                },
            );
            contract.update_property_type_trend(
                propchain_traits::PropertyType::Commercial,
                MarketTrend {
                    period_start: 0,
                    period_end: 0,
                    price_change_percentage: -10,
                    volume_change_percentage: -5,
                },
            );

            let first_suggestions = contract.get_rebalancing_suggestions(owner);
            let residential_first = first_suggestions
                .iter()
                .find(|s| s.property_type == propchain_traits::PropertyType::Residential)
                .expect("Residential suggestion exists");
            assert!(residential_first.target_allocation_bips > residential_first.current_allocation_bips);
            assert!(residential_first.recommendation.contains("Underweight"));

            contract.update_property_type_trend(
                propchain_traits::PropertyType::Residential,
                MarketTrend {
                    period_start: 0,
                    period_end: 0,
                    price_change_percentage: -15,
                    volume_change_percentage: -10,
                },
            );
            contract.update_property_type_trend(
                propchain_traits::PropertyType::Commercial,
                MarketTrend {
                    period_start: 0,
                    period_end: 0,
                    price_change_percentage: 30,
                    volume_change_percentage: 15,
                },
            );

            let second_suggestions = contract.get_rebalancing_suggestions(owner);
            let residential_second = second_suggestions
                .iter()
                .find(|s| s.property_type == propchain_traits::PropertyType::Residential)
                .expect("Residential suggestion exists after trend shift");
            assert!(residential_second.target_allocation_bips < residential_first.target_allocation_bips);
        }
    }
}

// =========================================================================
// ADMIN KEY ROTATION TESTS (Issue #496) — Analytics
// =========================================================================

#[cfg(test)]
mod analytics_admin_rotation_tests {
    use super::propchain_analytics::{AnalyticsDashboard, AnalyticsError};
    use ink::env::{test, DefaultEnvironment};

    fn setup() -> AnalyticsDashboard {
        let accounts = test::default_accounts::<DefaultEnvironment>();
        test::set_caller::<DefaultEnvironment>(accounts.alice);
        AnalyticsDashboard::new()
    }

    #[ink::test]
    fn test_admin_can_request_rotation() {
        let mut contract = setup();
        let accounts = test::default_accounts::<DefaultEnvironment>();
        assert!(contract.request_admin_rotation(accounts.bob).is_ok());
        let pending = contract.get_pending_admin_rotation();
        assert!(pending.is_some());
        let req = pending.unwrap();
        assert_eq!(req.old_account, accounts.alice);
        assert_eq!(req.new_account, accounts.bob);
    }

    #[ink::test]
    fn test_non_admin_cannot_request_rotation() {
        let mut contract = setup();
        let accounts = test::default_accounts::<DefaultEnvironment>();
        test::set_caller::<DefaultEnvironment>(accounts.bob);
        assert_eq!(
            contract.request_admin_rotation(accounts.charlie),
            Err(AnalyticsError::Unauthorized)
        );
    }

    #[ink::test]
    fn test_rotation_cannot_be_confirmed_before_cooldown() {
        let mut contract = setup();
        let accounts = test::default_accounts::<DefaultEnvironment>();
        contract.request_admin_rotation(accounts.bob).unwrap();
        test::set_caller::<DefaultEnvironment>(accounts.bob);
        assert_eq!(
            contract.confirm_admin_rotation(),
            Err(AnalyticsError::KeyRotationCooldown)
        );
    }

    #[ink::test]
    fn test_old_or_new_admin_can_cancel_rotation() {
        let mut contract = setup();
        let accounts = test::default_accounts::<DefaultEnvironment>();
        // Old admin cancels
        contract.request_admin_rotation(accounts.bob).unwrap();
        assert!(contract.cancel_admin_rotation().is_ok());
        assert!(contract.get_pending_admin_rotation().is_none());

        // New admin cancels
        contract.request_admin_rotation(accounts.bob).unwrap();
        test::set_caller::<DefaultEnvironment>(accounts.bob);
        assert!(contract.cancel_admin_rotation().is_ok());
    }

    #[ink::test]
    fn test_unrelated_cannot_cancel() {
        let mut contract = setup();
        let accounts = test::default_accounts::<DefaultEnvironment>();
        contract.request_admin_rotation(accounts.bob).unwrap();
        test::set_caller::<DefaultEnvironment>(accounts.charlie);
        assert_eq!(
            contract.cancel_admin_rotation(),
            Err(AnalyticsError::RotationUnauthorized)
        );
    }
}
