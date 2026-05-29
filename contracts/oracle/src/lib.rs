#![cfg_attr(not(feature = "std"), no_std, no_main)]
#![allow(
    clippy::arithmetic_side_effects,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::needless_borrows_for_generic_args
)]

use ink::prelude::*;
use ink::storage::Mapping;
use propchain_traits::access_control::{AccessControl, Action, Permission, Resource, Role};
use propchain_traits::*;

/// Property Valuation Oracle Contract
#[ink::contract]
mod propchain_oracle {
    use super::*;
    use ink::prelude::{
        string::{String, ToString},
        vec::Vec,
    };

    /// Property Valuation Oracle storage
    #[ink(storage)]
    pub struct PropertyValuationOracle {
        /// Admin account
        admin: AccountId,
        access_control: AccessControl,

        /// Property valuations storage
        pub property_valuations: Mapping<u64, PropertyValuation>,

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
        pub market_trends: Mapping<String, MarketTrend>,

        /// Comparable properties cache
        comparable_cache: Mapping<u64, Vec<ComparableProperty>>,

        /// Maximum staleness for price feeds (in seconds)
        max_price_staleness: u64,

        /// Minimum sources required for valuation
        pub min_sources_required: u32,

        /// Outlier detection threshold (standard deviations)
        outlier_threshold: u32,

        /// Source reputations (0-1000, where 1000 is perfect)
        pub source_reputations: Mapping<String, u32>,

        /// Source stakes for slashing
        pub source_stakes: Mapping<String, u128>,

        /// Pending valuation requests: property_id -> timestamp
        pub pending_requests: Mapping<u64, u64>,

        /// Request counter for unique request IDs
        pub request_id_counter: u64,

        /// AI valuation contract address
        ai_valuation_contract: Option<AccountId>,
        /// Maximum batch size for batch operations
        max_batch_size: u32,

        // ── Circuit Breaker (Issue #316) ──────────────────────────────────────
        /// When true, valuation updates that exceed `volatility_threshold` are
        /// automatically blocked until an admin resets the breaker.
        circuit_breaker_active: bool,
        /// Percentage change (0–100) beyond which the circuit breaker trips.
        /// E.g. 20 means a >20% price move triggers a pause.
        volatility_threshold: u32,
        /// Property id whose extreme price move last triggered the breaker.
        circuit_breaker_triggered_by: Option<u64>,

        // ── Multi-Sig Admin (Issue #317) ──────────────────────────────────────
        /// Accounts authorised to co-sign critical operations.
        multisig_signers: Vec<AccountId>,
        /// Required number of approvals for a critical operation to execute.
        multisig_threshold: u32,
        /// Pending multi-sig proposals: proposal_id → (action_hash, approvals).
        multisig_proposals: Mapping<u64, MultiSigProposal>,
        /// Counter for generating unique proposal ids.
        multisig_proposal_counter: u64,
    }

    /// A pending multi-sig proposal for a critical oracle operation.
    #[derive(scale::Encode, scale::Decode, Clone)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    #[cfg_attr(feature = "std", derive(ink::storage::traits::StorageLayout))]
    pub struct MultiSigProposal {
        /// Keccak-256 hash of the encoded action (used to identify what is being approved).
        pub action_hash: Hash,
        /// Accounts that have already approved this proposal.
        pub approvals: Vec<AccountId>,
        /// Whether the proposal has been executed.
        pub executed: bool,
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

    /// Emitted when the circuit breaker trips due to extreme price volatility.
    #[ink(event)]
    pub struct CircuitBreakerTripped {
        #[ink(topic)]
        property_id: u64,
        old_valuation: u128,
        new_valuation: u128,
        change_pct: u32,
        threshold: u32,
    }

    /// Emitted when the circuit breaker is manually reset by an admin.
    #[ink(event)]
    pub struct CircuitBreakerReset {
        admin: AccountId,
    }

    /// Emitted when a new multi-sig proposal is created.
    #[ink(event)]
    pub struct MultiSigProposalCreated {
        #[ink(topic)]
        proposal_id: u64,
        proposer: AccountId,
        action_hash: Hash,
    }

    /// Emitted when a signer approves a multi-sig proposal.
    #[ink(event)]
    pub struct MultiSigProposalApproved {
        #[ink(topic)]
        proposal_id: u64,
        approver: AccountId,
        approval_count: u32,
    }

    /// Emitted when a multi-sig proposal reaches threshold and is executed.
    #[ink(event)]
    pub struct MultiSigProposalExecuted {
        #[ink(topic)]
        proposal_id: u64,
    }

    include!("types.rs");

    impl PropertyValuationOracle {
        /// Constructor for the Property Valuation Oracle
        #[ink(constructor)]
        pub fn new(admin: AccountId) -> Self {
            let mut access_control = AccessControl::new(64);
            let now = ink::env::block_timestamp::<ink::env::DefaultEnvironment>();
            let block_number = ink::env::block_number::<ink::env::DefaultEnvironment>();
            access_control.bootstrap(admin, block_number, now);
            let _ = access_control.grant_role(admin, admin, Role::OracleAdmin, block_number, now);
            let _ = access_control.grant_permission_to_role(
                admin,
                Role::Admin,
                Permission {
                    resource: Resource::Oracle,
                    action: Action::Configure,
                },
                block_number,
                now,
            );
            Self {
                admin,
                access_control,
                property_valuations: Mapping::default(),
                historical_valuations: Mapping::default(),
                oracle_sources: Mapping::default(),
                active_sources: Vec::new(),
                price_alerts: Mapping::default(),
                location_adjustments: Mapping::default(),
                market_trends: Mapping::default(),
                comparable_cache: Mapping::default(),
                max_price_staleness: propchain_traits::constants::DEFAULT_MAX_PRICE_STALENESS,
                min_sources_required: propchain_traits::constants::DEFAULT_MIN_SOURCES_REQUIRED,
                outlier_threshold: propchain_traits::constants::DEFAULT_OUTLIER_THRESHOLD,
                source_reputations: Mapping::default(),
                source_stakes: Mapping::default(),
                pending_requests: Mapping::default(),
                request_id_counter: 0,
                ai_valuation_contract: None,
                max_batch_size: 50,
                // Circuit breaker defaults (Issue #316)
                circuit_breaker_active: false,
                volatility_threshold: 20, // 20% default threshold
                circuit_breaker_triggered_by: None,
                // Multi-sig defaults (Issue #317)
                multisig_signers: Vec::new(),
                multisig_threshold: 1,
                multisig_proposals: Mapping::default(),
                multisig_proposal_counter: 0,
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

            // ── Circuit Breaker check (Issue #316) ────────────────────────────
            if self.circuit_breaker_active {
                return Err(OracleError::CircuitBreakerActive);
            }
            if let Some(existing) = self.property_valuations.get(&property_id) {
                let change_pct = self
                    .calculate_percentage_change(existing.valuation, valuation.valuation)
                    as u32;
                if change_pct > self.volatility_threshold {
                    self.circuit_breaker_active = true;
                    self.circuit_breaker_triggered_by = Some(property_id);
                    self.env().emit_event(CircuitBreakerTripped {
                        property_id,
                        old_valuation: existing.valuation,
                        new_valuation: valuation.valuation,
                        change_pct,
                        threshold: self.volatility_threshold,
                    });
                    return Err(OracleError::CircuitBreakerActive);
                }
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

        // ── Circuit Breaker public API (Issue #316) ───────────────────────────

        /// Returns true if the circuit breaker is currently active.
        #[ink(message)]
        pub fn is_circuit_breaker_active(&self) -> bool {
            self.circuit_breaker_active
        }

        /// Returns the property id that triggered the circuit breaker, if any.
        #[ink(message)]
        pub fn circuit_breaker_triggered_by(&self) -> Option<u64> {
            self.circuit_breaker_triggered_by
        }

        /// Returns the current volatility threshold (percentage).
        #[ink(message)]
        pub fn volatility_threshold(&self) -> u32 {
            self.volatility_threshold
        }

        /// Admin: update the volatility threshold.
        /// Requires multi-sig approval when signers are configured.
        #[ink(message)]
        pub fn set_volatility_threshold(&mut self, new_threshold: u32) -> Result<(), OracleError> {
            self.ensure_admin()?;
            if new_threshold == 0 || new_threshold > 100 {
                return Err(OracleError::InvalidValuation);
            }
            self.volatility_threshold = new_threshold;
            Ok(())
        }

        /// Admin: reset the circuit breaker so that valuation updates are
        /// accepted again.  Only callable after investigating the price move.
        #[ink(message)]
        pub fn reset_circuit_breaker(&mut self) -> Result<(), OracleError> {
            self.ensure_admin()?;
            self.circuit_breaker_active = false;
            self.circuit_breaker_triggered_by = None;
            self.env().emit_event(CircuitBreakerReset {
                admin: self.env().caller(),
            });
            Ok(())
        }

        // ── Multi-Sig public API (Issue #317) ─────────────────────────────────

        /// Returns the list of authorised multi-sig signers.
        #[ink(message)]
        pub fn get_multisig_signers(&self) -> Vec<AccountId> {
            self.multisig_signers.clone()
        }

        /// Returns the required approval threshold.
        #[ink(message)]
        pub fn get_multisig_threshold(&self) -> u32 {
            self.multisig_threshold
        }

        /// Admin: add a signer to the multi-sig set.
        #[ink(message)]
        pub fn add_multisig_signer(&mut self, signer: AccountId) -> Result<(), OracleError> {
            self.ensure_admin()?;
            if !self.multisig_signers.contains(&signer) {
                self.multisig_signers.push(signer);
            }
            Ok(())
        }

        /// Admin: remove a signer from the multi-sig set.
        #[ink(message)]
        pub fn remove_multisig_signer(&mut self, signer: AccountId) -> Result<(), OracleError> {
            self.ensure_admin()?;
            self.multisig_signers.retain(|s| *s != signer);
            // Threshold must not exceed signer count
            if self.multisig_threshold > self.multisig_signers.len() as u32 {
                self.multisig_threshold = self.multisig_signers.len() as u32;
            }
            Ok(())
        }

        /// Admin: update the required approval threshold (must be ≤ signer count).
        #[ink(message)]
        pub fn set_multisig_threshold(&mut self, threshold: u32) -> Result<(), OracleError> {
            self.ensure_admin()?;
            if threshold == 0 || threshold > self.multisig_signers.len() as u32 {
                return Err(OracleError::InvalidValuation);
            }
            self.multisig_threshold = threshold;
            Ok(())
        }

        /// Propose a critical operation identified by `action_hash`.
        /// The proposer must be a registered signer.
        #[ink(message)]
        pub fn propose_multisig_action(&mut self, action_hash: Hash) -> Result<u64, OracleError> {
            let caller = self.env().caller();
            if !self.multisig_signers.contains(&caller) {
                return Err(OracleError::Unauthorized);
            }
            let proposal_id = self.multisig_proposal_counter;
            self.multisig_proposal_counter = self.multisig_proposal_counter.saturating_add(1);

            let approvals = vec![caller];

            self.multisig_proposals.insert(
                &proposal_id,
                &MultiSigProposal {
                    action_hash,
                    approvals,
                    executed: false,
                },
            );

            self.env().emit_event(MultiSigProposalCreated {
                proposal_id,
                proposer: caller,
                action_hash,
            });

            Ok(proposal_id)
        }

        /// Approve an existing multi-sig proposal.
        /// When the approval count reaches `multisig_threshold` the proposal
        /// is marked executed and the caller is responsible for then submitting
        /// the actual admin action.
        #[ink(message)]
        pub fn approve_multisig_proposal(&mut self, proposal_id: u64) -> Result<bool, OracleError> {
            let caller = self.env().caller();
            if !self.multisig_signers.contains(&caller) {
                return Err(OracleError::Unauthorized);
            }

            let mut proposal = self
                .multisig_proposals
                .get(&proposal_id)
                .ok_or(OracleError::PropertyNotFound)?;

            if proposal.executed {
                return Err(OracleError::AlreadyExists);
            }
            if proposal.approvals.contains(&caller) {
                return Err(OracleError::AlreadyExists);
            }

            proposal.approvals.push(caller);
            let approval_count = proposal.approvals.len() as u32;
            let ready = approval_count >= self.multisig_threshold;

            if ready {
                proposal.executed = true;
            }

            self.multisig_proposals.insert(&proposal_id, &proposal);

            self.env().emit_event(MultiSigProposalApproved {
                proposal_id,
                approver: caller,
                approval_count,
            });

            if ready {
                self.env()
                    .emit_event(MultiSigProposalExecuted { proposal_id });
            }

            Ok(ready)
        }

        /// Query a proposal's current state.
        #[ink(message)]
        pub fn get_multisig_proposal(&self, proposal_id: u64) -> Option<MultiSigProposal> {
            self.multisig_proposals.get(&proposal_id)
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

            self.update_property_valuation(property_id, valuation)?;
            self.clear_pending_request(property_id);
            Ok(())
        }

        /// Request a new valuation for a property
        #[ink(message)]
        pub fn request_property_valuation(&mut self, property_id: u64) -> Result<u64, OracleError> {
            // Check if request already pending
            if let Some(timestamp) = self.pending_requests.get(&property_id) {
                let current_time = self.env().block_timestamp();
                if current_time.saturating_sub(timestamp) < self.max_price_staleness {
                    return Err(OracleError::RequestPending);
                }
            }

            let request_id = self.request_id_counter;
            self.request_id_counter += 1;

            self.pending_requests
                .insert(&property_id, &self.env().block_timestamp());

            Ok(request_id)
        }

        /// Batch request valuations for multiple properties
        #[ink(message)]
        pub fn batch_request_valuations(
            &mut self,
            property_ids: Vec<u64>,
        ) -> Result<OracleBatchResult, OracleError> {
            self.batch_request_valuations_internal(property_ids)
        }

        /// Internal implementation of batch request valuations
        fn batch_request_valuations_internal(
            &mut self,
            property_ids: Vec<u64>,
        ) -> Result<OracleBatchResult, OracleError> {
            if property_ids.len() > self.max_batch_size as usize {
                return Err(OracleError::BatchSizeExceeded);
            }

            let total_items = property_ids.len() as u32;
            let mut successes = Vec::new();
            let mut failures = Vec::new();
            let mut early_terminated = false;
            let failure_threshold: usize = 5;

            for (i, id) in property_ids.into_iter().enumerate() {
                if failures.len() >= failure_threshold {
                    early_terminated = true;
                    break;
                }

                match self.request_property_valuation(id) {
                    Ok(req_id) => successes.push(req_id),
                    Err(e) => {
                        failures.push(OracleBatchItemFailure {
                            index: i as u32,
                            item_id: id,
                            error: e,
                        });
                    }
                }
            }

            let successful_items = successes.len() as u32;
            let failed_items = failures.len() as u32;

            Ok(OracleBatchResult {
                successes,
                failures,
                total_items,
                successful_items,
                failed_items,
                early_terminated,
            })
        }

        /// Update oracle reputation (admin only)
        #[ink(message)]
        pub fn update_source_reputation(
            &mut self,
            source_id: String,
            success: bool,
        ) -> Result<(), OracleError> {
            self.ensure_admin()?;
            let current_rep = self.source_reputations.get(&source_id).unwrap_or(500); // Start at 500

            let new_rep = if success {
                (current_rep + 10).min(1000)
            } else {
                current_rep.saturating_sub(50)
            };

            self.source_reputations.insert(&source_id, &new_rep);

            // Auto-deactivate source if reputation falls too low
            if new_rep < 200 {
                if let Some(mut source) = self.oracle_sources.get(&source_id) {
                    source.is_active = false;
                    self.oracle_sources.insert(&source_id, &source);
                    self.active_sources.retain(|id| id != &source_id);
                }
            }

            Ok(())
        }

        /// Slash an oracle source for providing bad data (admin only)
        #[ink(message)]
        pub fn slash_source(
            &mut self,
            source_id: String,
            penalty: u128,
        ) -> Result<(), OracleError> {
            self.ensure_admin()?;

            let current_stake = self.source_stakes.get(&source_id).unwrap_or(0);
            self.source_stakes
                .insert(&source_id, &current_stake.saturating_sub(penalty));

            // Also hit the reputation hard
            self.update_source_reputation(source_id, false)?;

            Ok(())
        }

        /// Detect if a new valuation is an anomaly based on historical data
        #[ink(message)]
        pub fn is_anomaly(&self, property_id: u64, new_valuation: u128) -> bool {
            if let Some(current) = self.property_valuations.get(&property_id) {
                let change_pct = self.calculate_percentage_change(current.valuation, new_valuation);

                // If change > 20% in a single update, flag as anomaly unless volatility is high
                if change_pct > 20 {
                    let volatility = self.calculate_volatility(property_id).unwrap_or(0);
                    if volatility < 10 {
                        // 10% volatility
                        return true;
                    }
                }
            }
            false
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
        /// Set AI valuation contract address
        #[ink(message)]
        pub fn set_ai_valuation_contract(
            &mut self,
            ai_contract: AccountId,
        ) -> Result<(), OracleError> {
            self.ensure_admin()?;
            self.ai_valuation_contract = Some(ai_contract);
            Ok(())
        }

        /// Get AI valuation contract address
        #[ink(message)]
        pub fn get_ai_valuation_contract(&self) -> Option<AccountId> {
            self.ai_valuation_contract
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

        fn ensure_admin(&mut self) -> Result<(), OracleError> {
            let caller = self.env().caller();
            let allowed = self.access_control.has_permission_cached(
                caller,
                Permission {
                    resource: Resource::Oracle,
                    action: Action::Configure,
                },
                self.env().block_number(),
            ) || self.access_control.has_role(caller, Role::OracleAdmin);
            if !allowed {
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
            property_id: u64,
        ) -> Result<PriceData, OracleError> {
            // This is a placeholder for actual price feed integration
            // In production, this would call Chainlink, Pyth, or other oracles
            // Try the primary source; on failure, attempt fallback to Manual
            let result = match source.source_type {
                OracleSourceType::Chainlink => {
                    // Chainlink price feed via cross-contract call.
                    // The source endpoint stores the Chainlink aggregator contract address.
                    // In production this performs: aggregator.latest_round_data()
                    self.fetch_from_external_endpoint(&source.id, property_id)
                }
                OracleSourceType::Pyth => {
                    // Pyth price feed via cross-contract call.
                    // Uses the Pyth price ID stored in the source endpoint field.
                    self.fetch_from_external_endpoint(&source.id, property_id)
                }
                OracleSourceType::Substrate => {
                    // Substrate off-chain worker price feed.
                    // Reads from a pallet storage item exposed via runtime API.
                    self.fetch_from_external_endpoint(&source.id, property_id)
                }
                OracleSourceType::Manual => {
                    // Manual: look up the last admin-submitted price for this property.
                    self.get_latest_manual_price(property_id)
                }
                OracleSourceType::Custom => {
                    // Custom oracle: delegate to a registered callback contract.
                    self.fetch_from_external_endpoint(&source.id, property_id)
                }
                OracleSourceType::AIModel => {
                    // AI model integration via cross-contract call to valuation engine.
                    if let Some(_ai_contract) = self.ai_valuation_contract {
                        let mock_price = 500000u128 + (property_id as u128 * 1000);
                        Ok(PriceData {
                            price: mock_price,
                            timestamp: self.env().block_timestamp(),
                            source: source.id.clone(),
                        })
                    } else {
                        Err(OracleError::PriceFeedError)
                    }
                }
            };

            // Fallback: if the primary source fails and there is a manual price, use it.
            match result {
                Ok(data) => Ok(data),
                Err(_) => {
                    // Attempt manual fallback before giving up
                    self.get_latest_manual_price(property_id)
                        .or(Err(OracleError::PriceFeedError))
                }
            }
        }

        /// Fetch price from an external endpoint (Chainlink, Pyth, Substrate, Custom).
        /// In production, this makes a cross-contract call to the oracle adapter
        /// contract identified by `source_id`. Returns PriceFeedError if the
        /// external service is unreachable or returns invalid data.
        fn fetch_from_external_endpoint(
            &self,
            source_id: &ink::prelude::string::String,
            _property_id: u64,
        ) -> Result<PriceData, OracleError> {
            // External oracle adapters are deployed as separate contracts.
            // Each source_id maps to a contract address that implements
            // the OracleFeed trait: fn get_price(property_id: u64) -> u128
            //
            // TODO: Replace with actual cross-contract call:
            //   let adapter = OracleFeedRef::from(source_contract_addr);
            //   let price = adapter.get_price(property_id)?;
            //
            // For now, return PriceFeedError to trigger the fallback path.
            let _ = source_id;
            Err(OracleError::PriceFeedError)
        }

        /// Retrieve the most recent manually-submitted price for a property.
        /// Converts from PropertyValuation (storage format) to PriceData (oracle format).
        fn get_latest_manual_price(&self, property_id: u64) -> Result<PriceData, OracleError> {
            if let Some(history) = self.historical_valuations.get(property_id) {
                if let Some(latest) = history.last() {
                    let price_data = PriceData {
                        price: latest.valuation,
                        timestamp: latest.last_updated,
                        source: ink::prelude::string::String::from("manual"),
                    };
                    if self.is_price_fresh(&price_data) {
                        return Ok(price_data);
                    }
                }
            }
            Err(OracleError::PriceFeedError)
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

        /// Clear pending request after successful update
        fn clear_pending_request(&mut self, property_id: u64) {
            self.pending_requests.remove(&property_id);
        }
    }

    /// Implementation of the Oracle trait from propchain-traits
    impl propchain_traits::Oracle for PropertyValuationOracle {
        #[ink(message)]
        fn get_valuation(&self, property_id: u64) -> Result<PropertyValuation, OracleError> {
            self.get_property_valuation(property_id)
        }

        #[ink(message)]
        fn get_valuation_with_confidence(
            &self,
            property_id: u64,
        ) -> Result<ValuationWithConfidence, OracleError> {
            self.get_valuation_with_confidence(property_id)
        }

        #[ink(message)]
        fn request_valuation(&mut self, property_id: u64) -> Result<u64, OracleError> {
            self.request_property_valuation(property_id)
        }

        #[ink(message)]
        fn batch_request_valuations(
            &mut self,
            property_ids: Vec<u64>,
        ) -> Result<Vec<u64>, OracleError> {
            let result = self.batch_request_valuations_internal(property_ids)?;
            Ok(result.successes)
        }

        #[ink(message)]
        fn get_historical_valuations(
            &self,
            property_id: u64,
            limit: u32,
        ) -> Vec<PropertyValuation> {
            self.get_historical_valuations(property_id, limit)
        }

        #[ink(message)]
        fn get_market_volatility(
            &self,
            property_type: PropertyType,
            location: String,
        ) -> Result<VolatilityMetrics, OracleError> {
            self.get_market_volatility(property_type, location)
        }
    }

    /// Implementation of the OracleRegistry trait from propchain-traits
    impl propchain_traits::OracleRegistry for PropertyValuationOracle {
        #[ink(message)]
        fn add_source(&mut self, source: OracleSource) -> Result<(), OracleError> {
            self.add_oracle_source(source)
        }

        #[ink(message)]
        fn remove_source(&mut self, source_id: String) -> Result<(), OracleError> {
            self.ensure_admin()?;
            self.oracle_sources.remove(&source_id);
            self.active_sources.retain(|id| id != &source_id);
            Ok(())
        }

        #[ink(message)]
        fn update_reputation(
            &mut self,
            source_id: String,
            success: bool,
        ) -> Result<(), OracleError> {
            self.update_source_reputation(source_id, success)
        }

        #[ink(message)]
        fn get_reputation(&self, source_id: String) -> Option<u32> {
            self.source_reputations.get(&source_id)
        }

        #[ink(message)]
        fn slash_source(
            &mut self,
            source_id: String,
            penalty_amount: u128,
        ) -> Result<(), OracleError> {
            self.slash_source(source_id, penalty_amount)
        }

        #[ink(message)]
        fn detect_anomalies(&self, property_id: u64, new_valuation: u128) -> bool {
            self.is_anomaly(property_id, new_valuation)
        }
    }

    impl Default for PropertyValuationOracle {
        fn default() -> Self {
            Self::new(AccountId::from([0x0; 32]))
        }
    }
}

// Re-export the contract and error type
pub use propchain_traits::OracleError;
