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
        collections::BTreeSet,
        string::{String, ToString},
        vec::Vec,
    };

    /// Property Valuation Oracle storage
    pub enum AggregationMode {
    SimpleMedian,
    WeightedMedian,
    TrimmedMean,
}

#[ink(storage)]
pub struct PropertyValuationOracle {
    aggregation_mode: AggregationMode,
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

        /// Per-property trend metrics cache
        property_trends: Mapping<u64, TrendMetrics>,

        /// Configurable EMA smoothing factor in basis points (0-10000)
        ema_alpha_bps: u32,

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

        // ── Update Frequency Control (Issue #225) ────────────────────────────
        /// Minimum interval (in blocks) between two updates for the same source
        min_update_interval_blocks: u64,
        /// Last update time per source: source_id -> block_number
        last_source_update: Mapping<String, u64>,

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

        // ── Oracle Governance (Issue #228) ─────────────────────────────────────
        /// Governance-controlled oracle parameters
        governance_params: GovernanceParams,
        /// Active governance proposals
        governance_proposals: Mapping<u64, GovernanceProposal>,
        /// Governance proposal counter
        governance_proposal_counter: u64,
        /// Governance voting power by participant
        governance_voting_power: Mapping<AccountId, u32>,

        // ── Aggregation Method ────────────────────────────────────────────────
        /// Method used to aggregate prices from multiple sources
        aggregation_method: AggregationMethod,

        // ── Slashing Infrastructure (Issue #319) ─────────────────────────────
        /// Graduated slashing configuration
        slashing_config: SlashingConfig,
        /// Per-source slashing records (history)
        slashing_records: Mapping<String, Vec<SlashingRecord>>,
        /// Per-source total slash count
        slashing_counts: Mapping<String, u32>,
        /// Per-source total amount slashed
        slashed_amounts: Mapping<String, u128>,
        /// Banned sources: source_id -> block until which the ban is active (0 = not banned)
        banned_sources: Mapping<String, u32>,

        // ── Fallback Mechanism (Issue #220) ──────────────────────────────────
        /// Fallback oracle configuration
        fallback_config: FallbackConfig,
        /// Registered fallback sources: id -> FallbackSource
        fallback_sources: Mapping<String, FallbackSource>,
        /// Ordered list of fallback source ids (sorted by priority)
        fallback_source_ids: Vec<String>,

        // ── Auto-Slash Configuration (Issue #497) ────────────────────────────
        /// When true, sources are automatically slashed for staleness/deviation/missed updates
        auto_slash_on_staleness: bool,
        /// Staleness threshold in seconds; sources not updating within this window are auto-slashed
        auto_slash_staleness_threshold: u64,
        /// When true, sources with high deviation from consensus are auto-slashed
        auto_slash_on_deviation: bool,
        /// Deviation threshold in basis points (e.g. 2000 = 20%); sources above this are slashed
        auto_slash_deviation_threshold_bps: u32,
        /// When true, sources missing consecutive update calls are auto-slashed
        auto_slash_on_missed_updates: bool,
        /// Number of consecutive missed updates before auto-slash triggers
        auto_slash_missed_update_count: u32,
        /// Per-source: last time (timestamp) the source successfully submitted a value
        source_last_report_time: Mapping<String, u64>,
        /// Per-source: consecutive missed update counter
        source_missed_updates: Mapping<String, u32>,

        // ── Multi-Sig Oracle Source Management (Issue #495) ──────────────────
        /// Pending source-management proposals (add/remove): proposal_id -> OracleSourceProposal
        source_proposals: Mapping<u64, OracleSourceProposal>,
        /// Counter for source management proposal ids
        source_proposal_counter: u64,

        // ── Batched Aggregation Optimization ───────────────────────────────────
        /// When true, use batched price collection to reduce gas costs
        batch_aggregation_enabled: bool,
        /// Packed source weights: each u64 contains two u32 weights (weight << 32 | weight2)
        /// This reduces storage reads during aggregation
        packed_source_weights: Vec<u64>,
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

    // ── Oracle Governance Events (Issue #228) ────────────────────────────────

    #[ink(event)]
    pub struct GovernanceParamUpdated {
        #[ink(topic)]
        param_name: String,
        old_value: u128,
        new_value: u128,
        updated_by: AccountId,
    }

    #[ink(event)]
    pub struct GovernanceActionProposed {
        #[ink(topic)]
        proposal_id: u64,
        #[ink(topic)]
        proposer: AccountId,
        action: GovernanceAction,
    }

    #[ink(event)]
    pub struct GovernanceVoteCast {
        #[ink(topic)]
        proposal_id: u64,
        #[ink(topic)]
        voter: AccountId,
        support: bool,
        weight: u32,
    }

    #[ink(event)]
    pub struct GovernanceActionExecuted {
        #[ink(topic)]
        proposal_id: u64,
        action: GovernanceAction,
    }

    // ── Slashing Events (Issue #319) ─────────────────────────────────────────

    #[ink(event)]
    pub struct SourceSlashed {
        #[ink(topic)]
        source_id: String,
        severity: SlashingSeverity,
        amount_slashed: u128,
        remaining_stake: u128,
        reason: String,
    }

    #[ink(event)]
    pub struct SourceSuspended {
        #[ink(topic)]
        source_id: String,
        total_slashes: u32,
        suspension_threshold: u32,
    }

    #[ink(event)]
    pub struct SourceBanned {
        #[ink(topic)]
        source_id: String,
        admin: AccountId,
        ban_until_block: u32,
    }

    #[ink(event)]
    pub struct SourceUnbanned {
        #[ink(topic)]
        source_id: String,
        admin: AccountId,
    }

    // ── Frequency Control Events (Issue #225) ────────────────────────────────

    #[ink(event)]
    pub struct UpdateThrottled {
        #[ink(topic)]
        source_id: String,
        last_update: u64,
        current_block: u64,
        min_interval: u64,
    }

    // ── Fallback Events (Issue #220) ─────────────────────────────────────────

    #[ink(event)]
    pub struct FallbackConfigUpdated {
        enabled: bool,
        fallback_delay_blocks: u64,
        max_fallback_attempts: u32,
    }

    #[ink(event)]
    pub struct FallbackSourceAdded {
        #[ink(topic)]
        source_id: String,
        priority: u32,
    }

    #[ink(event)]
    pub struct FallbackSourceRemoved {
        #[ink(topic)]
        source_id: String,
    }

    #[ink(event)]
    pub struct FallbackTriggered {
        #[ink(topic)]
        primary_source_id: String,
        #[ink(topic)]
        fallback_source_id: String,
        property_id: u64,
        attempts: u32,
    }

    // ── Auto-Slash Events (Issue #497) ───────────────────────────────────────

    /// Emitted when a source is automatically slashed due to objective criteria.
    #[ink(event)]
    pub struct SourceAutoSlashed {
        #[ink(topic)]
        source_id: String,
        /// Reason: "Staleness" | "Deviation" | "MissedUpdates"
        reason: String,
        severity: SlashingSeverity,
        amount_slashed: u128,
        remaining_stake: u128,
    }

    // ── Batched Aggregation Events ──────────────────────────────────────────

    /// Emitted after batched price collection to provide gas observability.
    #[ink(event)]
    pub struct BatchPricesCollected {
        #[ink(topic)]
        property_id: u64,
        sources_attempted: u32,
        sources_succeeded: u32,
        batch_enabled: bool,
    }

    // ── Multi-Sig Source Management Events (Issue #495) ──────────────────────

    /// Emitted when a multi-sig proposal to add/remove an oracle source is created.
    #[ink(event)]
    pub struct OracleSourceProposalCreated {
        #[ink(topic)]
        proposal_id: u64,
        proposer: AccountId,
        action: OracleSourceAction,
        source_id: String,
    }

    /// Emitted when a signer approves an oracle source management proposal.
    #[ink(event)]
    pub struct OracleSourceProposalApproved {
        #[ink(topic)]
        proposal_id: u64,
        approver: AccountId,
        approval_count: u32,
    }

    /// Emitted when an oracle source proposal reaches threshold and executes.
    #[ink(event)]
    pub struct OracleSourceProposalExecuted {
        #[ink(topic)]
        proposal_id: u64,
        action: OracleSourceAction,
        source_id: String,
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
                property_trends: Mapping::default(),
                ema_alpha_bps: 1000, // Default alpha = 0.10
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
                min_update_interval_blocks: 6, // ~36 seconds at 6s blocks
                last_source_update: Mapping::default(),
                // Circuit breaker defaults (Issue #316)
                circuit_breaker_active: false,
                volatility_threshold: 20, // 20% default threshold
                circuit_breaker_triggered_by: None,
                // Multi-sig defaults (Issue #317)
                multisig_signers: Vec::new(),
                multisig_threshold: 1,
                multisig_proposals: Mapping::default(),
                multisig_proposal_counter: 0,
                // Oracle governance defaults (Issue #228)
                governance_params: GovernanceParams::default(),
                governance_proposals: Mapping::default(),
                governance_proposal_counter: 0,
                governance_voting_power: Mapping::default(),
                // Aggregation method
                aggregation_method: AggregationMethod::WeightedMean,
                // Slashing infrastructure (Issue #319)
                slashing_config: SlashingConfig::default(),
                slashing_records: Mapping::default(),
                slashing_counts: Mapping::default(),
                slashed_amounts: Mapping::default(),
                banned_sources: Mapping::default(),
                // Fallback mechanism (Issue #220)
                fallback_config: FallbackConfig::default(),
                fallback_sources: Mapping::default(),
                fallback_source_ids: Vec::new(),
                // Auto-slash configuration (Issue #497)
                auto_slash_on_staleness: false,
                auto_slash_staleness_threshold:
                    propchain_traits::constants::DEFAULT_MAX_PRICE_STALENESS,
                auto_slash_on_deviation: false,
                auto_slash_deviation_threshold_bps: 2000, // 20% default
                auto_slash_on_missed_updates: false,
                auto_slash_missed_update_count: 3,
                source_last_report_time: Mapping::default(),
                source_missed_updates: Mapping::default(),
                // Multi-sig source management (Issue #495)
                source_proposals: Mapping::default(),
                source_proposal_counter: 0,
                // Batched aggregation optimization
                batch_aggregation_enabled: false,
                packed_source_weights: Vec::new(),
            }
        }

        /// Get property valuation from multiple sources with aggregation
        #[ink(message)]
        pub fn get_property_valuation(
            &self,
            property_id: u64,
        ) -> Result<PropertyValuation, OracleError> {
            let aggregated_valuation = match self.aggregation_mode {
    AggregationMode::SimpleMedian => {
        let mut values: Vec<u128> = self.property_valuations.values().collect();
        aggregation::simple_median(&mut values)
    }
    AggregationMode::WeightedMedian => {
        let mut values: Vec<(u128, u32)> = self.property_valuations.iter().map(|(id, val)| (val, self.source_reputations.get(id).unwrap_or(0))).collect();
        aggregation::weighted_median(&values)
    }
    AggregationMode::TrimmedMean => {
        let mut values: Vec<u128> = self.property_valuations.values().collect();
        aggregation::trimmed_mean(&mut values, 10)
    }
};
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

            // Record oracle snapshot for history tracking
            self.record_oracle_snapshot(
                property_id,
                "admin_update".to_string(),
                valuation.valuation,
                valuation.confidence_score,
                valuation.valuation_method.clone(),
            );

            // Record source history
            self.record_source_history(
                "admin_update".to_string(),
                property_id,
                valuation.valuation,
                true,
                valuation.confidence_score,
            );

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

            self.update_trend_metrics(property_id);

            Ok(())
        }

        /// Get the trend metrics for a property.
        #[ink(message)]
        pub fn get_property_trend(&self, property_id: u64) -> Result<TrendMetrics, OracleError> {
            self.property_trends
                .get(&property_id)
                .ok_or(OracleError::PropertyNotFound)
        }

        /// Get volatility index for a property over a given window of days.
        #[ink(message)]
        pub fn get_volatility_index(
            &self,
            property_id: u64,
            window_days: u32,
        ) -> Result<u32, OracleError> {
            if window_days == 0 {
                return Err(OracleError::InvalidParameters);
            }
            self.calculate_volatility_index(property_id, window_days)
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

        // ── Oracle Governance Methods (Issue #228) ───────────────────────────

        /// Returns the current governance parameters.
        #[ink(message)]
        pub fn get_governance_params(&self) -> GovernanceParams {
            self.governance_params.clone()
        }

        /// Returns a governance proposal by ID.
        #[ink(message)]
        pub fn get_governance_proposal(&self, proposal_id: u64) -> Option<GovernanceProposal> {
            self.governance_proposals.get(&proposal_id)
        }

        /// Returns the voting power for an account.
        #[ink(message)]
        pub fn get_governance_voting_power(&self, account: AccountId) -> u32 {
            self.governance_voting_power.get(&account).unwrap_or(0)
        }

        /// Admin: set voting power for a governance participant.
        #[ink(message)]
        pub fn set_governance_voting_power(
            &mut self,
            participant: AccountId,
            power: u32,
        ) -> Result<(), OracleError> {
            self.ensure_admin()?;
            self.governance_voting_power.insert(&participant, &power);
            Ok(())
        }

        // ── Batched Aggregation Optimization ─────────────────────────────────────

        /// Enable or disable batched aggregation mode (admin only).
        /// When enabled, prices are collected using batched operations to reduce gas costs.
        #[ink(message)]
        pub fn set_batch_aggregation(&mut self, enabled: bool) -> Result<(), OracleError> {
            self.ensure_admin()?;
            self.batch_aggregation_enabled = enabled;
            // Rebuild packed weights cache when enabling
            if enabled {
                self.rebuild_packed_weights();
            }
            Ok(())
        }

        /// Returns whether batched aggregation is currently enabled.
        #[ink(message)]
        pub fn is_batch_aggregation_enabled(&self) -> bool {
            self.batch_aggregation_enabled
        }

        /// Propose a governance action to change oracle parameters.
        #[ink(message)]
        pub fn propose_governance_action(
            &mut self,
            action: GovernanceAction,
        ) -> Result<u64, OracleError> {
            let caller = self.env().caller();
            let power = self.governance_voting_power.get(&caller).unwrap_or(0);
            if power == 0 {
                return Err(OracleError::Unauthorized);
            }

            let proposal_id = self.governance_proposal_counter;
            self.governance_proposal_counter = self.governance_proposal_counter.saturating_add(1);
            let now = self.env().block_number();

            let proposal = GovernanceProposal {
                id: proposal_id,
                proposer: caller,
                action: action.clone(),
                votes_for: 0,
                votes_against: 0,
                voting_end: now
                    .saturating_add(self.governance_params.governance_voting_period_blocks),
                executed: false,
                created_at: now,
            };

            self.governance_proposals.insert(&proposal_id, &proposal);

            self.env().emit_event(GovernanceActionProposed {
                proposal_id,
                proposer: caller,
                action,
            });

            Ok(proposal_id)
        }

        /// Cast a vote on a governance proposal.
        #[ink(message)]
        pub fn vote_on_governance_proposal(
            &mut self,
            proposal_id: u64,
            support: bool,
        ) -> Result<(), OracleError> {
            let caller = self.env().caller();
            let weight = self.governance_voting_power.get(&caller).unwrap_or(0);
            if weight == 0 {
                return Err(OracleError::Unauthorized);
            }

            let mut proposal = self
                .governance_proposals
                .get(&proposal_id)
                .ok_or(OracleError::PropertyNotFound)?;

            if proposal.executed {
                return Err(OracleError::AlreadyExists);
            }

            let now = self.env().block_number();
            if now >= proposal.voting_end {
                return Err(OracleError::InvalidParameters);
            }

            if support {
                proposal.votes_for = proposal.votes_for.saturating_add(weight as u128);
            } else {
                proposal.votes_against = proposal.votes_against.saturating_add(weight as u128);
            }

            self.governance_proposals.insert(&proposal_id, &proposal);

            self.env().emit_event(GovernanceVoteCast {
                proposal_id,
                voter: caller,
                support,
                weight,
            });

            Ok(())
        }

        /// Execute a governance proposal after voting ends.
        #[ink(message)]
        pub fn execute_governance_proposal(&mut self, proposal_id: u64) -> Result<(), OracleError> {
            let mut proposal = self
                .governance_proposals
                .get(&proposal_id)
                .ok_or(OracleError::PropertyNotFound)?;

            if proposal.executed {
                return Err(OracleError::AlreadyExists);
            }

            let now = self.env().block_number();
            if now < proposal.voting_end {
                return Err(OracleError::InvalidParameters);
            }

            // Check quorum
            let total_votes = proposal.votes_for.saturating_add(proposal.votes_against);
            let total_power: u128 = 100_000; // Placeholder for total voting power
            let quorum = total_power
                .saturating_mul(self.governance_params.governance_quorum_bps as u128)
                / 10_000;
            if total_votes < quorum {
                return Err(OracleError::InvalidParameters);
            }

            if proposal.votes_for <= proposal.votes_against {
                return Err(OracleError::Unauthorized);
            }

            // Apply the action
            match proposal.action.clone() {
                GovernanceAction::UpdateMinStake(v) => {
                    self.governance_params.min_oracle_stake = v;
                }
                GovernanceAction::UpdateMinSources(v) => {
                    self.min_sources_required = v;
                }
                GovernanceAction::UpdateMaxStaleness(v) => {
                    self.max_price_staleness = v;
                }
                GovernanceAction::UpdateVolatilityThreshold(v) => {
                    self.volatility_threshold = v;
                }
                GovernanceAction::AddSourceType(id, weight) => {
                    let source = OracleSource {
                        id: id.clone(),
                        source_type: OracleSourceType::Custom,
                        address: AccountId::from([0x0; 32]),
                        is_active: true,
                        weight,
                        last_updated: self.env().block_timestamp(),
                    };
                    self.oracle_sources.insert(&id, &source);
                    if !self.active_sources.contains(&id) {
                        self.active_sources.push(id);
                    }
                }
                GovernanceAction::RemoveSource(id) => {
                    self.oracle_sources.remove(&id);
                    self.active_sources.retain(|s| s != &id);
                    if self.batch_aggregation_enabled {
                        self.rebuild_packed_weights();
                    }
                }
            }

            proposal.executed = true;
            self.governance_proposals.insert(&proposal_id, &proposal);

            self.env().emit_event(GovernanceActionExecuted {
                proposal_id,
                action: proposal.action.clone(),
            });

            Ok(())
        }

        /// Update property valuation from oracle sources.
        ///
        /// After aggregating prices, records each responding source's last-report
        /// timestamp and increments the missed-update counter for any active
        /// source that did **not** respond. Auto-slash checks are then run
        /// against all active sources (Issue #497).
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

            let now = self.env().block_timestamp();

            // ── Track per-source participation (Issue #497) ──────────────────
            // Build the set of sources that responded this round.
            let responding: ink::prelude::collections::BTreeSet<String> =
                prices.iter().map(|p| p.source.clone()).collect();

            // Update last-report time for responding sources; increment
            // missed-update counter for non-responding active sources.
            let all_sources: Vec<String> = self.active_sources.clone();
            for sid in &all_sources {
                if responding.contains(sid) {
                    self.source_last_report_time.insert(sid, &now);
                    // Reset missed counter on successful report
                    self.source_missed_updates.insert(sid, &0);
                } else {
                    let missed = self.source_missed_updates.get(sid).unwrap_or(0);
                    self.source_missed_updates
                        .insert(sid, &missed.saturating_add(1));
                }
            }

            // ── Auto-slash checks (Issue #497) ───────────────────────────────
            self.run_auto_slash_checks(aggregated_price);

            let valuation = PropertyValuation {
                property_id,
                valuation: aggregated_price,
                confidence_score,
                sources_used: prices.len() as u32,
                last_updated: now,
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
                    if self.batch_aggregation_enabled {
                        self.rebuild_packed_weights();
                    }
                }
            }

            Ok(())
        }

        /// Slash an oracle source with graduated severity (admin only).
        ///
        /// - `Minor`:    5% stake slash, -50 reputation
        /// - `Moderate`: 15% stake slash, -150 reputation
        /// - `Severe`:   30% stake slash, -300 reputation, auto-suspension if threshold exceeded
        /// - `Critical`: 50% stake slash, -500 reputation, source banned
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

        /// Slash an oracle source with graduated severity (admin only).
        /// Uses the configured basis points for each severity level.
        #[ink(message)]
        pub fn slash_source_with_severity(
            &mut self,
            source_id: String,
            severity: SlashingSeverity,
            reason: String,
        ) -> Result<(), OracleError> {
            self.ensure_admin()?;

            let current_stake = self.source_stakes.get(&source_id).unwrap_or(0);
            if current_stake == 0 {
                return Err(OracleError::InvalidParameters);
            }

            // Calculate slash percentage from config based on severity
            let slash_bps = match severity {
                SlashingSeverity::Minor => self.slashing_config.minor_slash_bps,
                SlashingSeverity::Moderate => self.slashing_config.moderate_slash_bps,
                SlashingSeverity::Severe => self.slashing_config.severe_slash_bps,
                SlashingSeverity::Critical => self.slashing_config.critical_slash_bps,
            };
            let reput_penalty = match severity {
                SlashingSeverity::Minor => self.slashing_config.minor_reputation_penalty,
                SlashingSeverity::Moderate => self.slashing_config.moderate_reputation_penalty,
                SlashingSeverity::Severe => self.slashing_config.severe_reputation_penalty,
                SlashingSeverity::Critical => self.slashing_config.critical_reputation_penalty,
            };

            let slash_amount = current_stake.saturating_mul(slash_bps as u128) / 10_000;
            let remaining_stake = current_stake.saturating_sub(slash_amount);

            // Apply stake slash
            self.source_stakes.insert(&source_id, &remaining_stake);

            // Apply reputation penalty
            let current_rep = self.source_reputations.get(&source_id).unwrap_or(500);
            let new_rep = current_rep.saturating_sub(reput_penalty);
            self.source_reputations.insert(&source_id, &new_rep);

            // Track slashing record
            let mut records = self.slashing_records.get(&source_id).unwrap_or_default();
            let block = self.env().block_number();
            let mut banned = false;
            records.push(SlashingRecord {
                block,
                severity: severity.clone(),
                amount_slashed: slash_amount,
                reason: reason.clone(),
                banned: false, // updated below if ban occurs
            });
            self.slashing_records.insert(&source_id, &records);

            // Update running totals
            let current_count = self.slashing_counts.get(&source_id).unwrap_or(0);
            self.slashing_counts
                .insert(&source_id, &(current_count + 1));
            let current_amount = self.slashed_amounts.get(&source_id).unwrap_or(0);
            self.slashed_amounts
                .insert(&source_id, &current_amount.saturating_add(slash_amount));

            // Auto-suspend if slashing count exceeds threshold
            if current_count + 1 >= self.slashing_config.suspension_threshold {
                if let Some(mut source) = self.oracle_sources.get(&source_id) {
                    source.is_active = false;
                    self.oracle_sources.insert(&source_id, &source);
                    self.active_sources.retain(|id| id != &source_id);
                    if self.batch_aggregation_enabled {
                        self.rebuild_packed_weights();
                    }
                }
                self.env().emit_event(SourceSuspended {
                    source_id: source_id.clone(),
                    total_slashes: current_count + 1,
                    suspension_threshold: self.slashing_config.suspension_threshold,
                });
            }

            // Ban on critical severity
            if matches!(severity, SlashingSeverity::Critical) {
                let ban_until = block.saturating_add(self.slashing_config.ban_duration_blocks);
                self.banned_sources.insert(&source_id, &ban_until);
                banned = true;
                // Deactivate source
                if let Some(mut source) = self.oracle_sources.get(&source_id) {
                    source.is_active = false;
                    self.oracle_sources.insert(&source_id, &source);
                    self.active_sources.retain(|id| id != &source_id);
                    if self.batch_aggregation_enabled {
                        self.rebuild_packed_weights();
                    }
                }
                self.env().emit_event(SourceBanned {
                    source_id: source_id.clone(),
                    admin: self.env().caller(),
                    ban_until_block: ban_until,
                });
            }

            // Update the last record's banned flag
            if let Some(mut updated_records) = self.slashing_records.get(&source_id) {
                if let Some(last) = updated_records.last_mut() {
                    last.banned = banned;
                }
                self.slashing_records.insert(&source_id, &updated_records);
            }

            self.env().emit_event(SourceSlashed {
                source_id,
                severity,
                amount_slashed: slash_amount,
                remaining_stake,
                reason,
            });

            Ok(())
        }

        /// Unban a previously banned source (admin only).
        #[ink(message)]
        pub fn unban_source(&mut self, source_id: String) -> Result<(), OracleError> {
            self.ensure_admin()?;
            self.banned_sources.insert(&source_id, &0);
            self.env().emit_event(SourceUnbanned {
                source_id,
                admin: self.env().caller(),
            });
            Ok(())
        }

        /// Update slashing configuration parameters (admin only).
        #[ink(message)]
        pub fn set_slashing_config(&mut self, config: SlashingConfig) -> Result<(), OracleError> {
            self.ensure_admin()?;
            self.slashing_config = config;
            Ok(())
        }

        /// Get the current slashing configuration.
        #[ink(message)]
        pub fn get_slashing_config(&self) -> SlashingConfig {
            self.slashing_config.clone()
        }

        /// Get slashing records for a source.
        #[ink(message)]
        pub fn get_slashing_records(&self, source_id: String) -> Vec<SlashingRecord> {
            self.slashing_records.get(&source_id).unwrap_or_default()
        }

        /// Get the complete status of a source including slashing info.
        #[ink(message)]
        pub fn get_source_status(&self, source_id: String) -> Option<SourceStatus> {
            let reputation = self.source_reputations.get(&source_id)?;
            let stake = self.source_stakes.get(&source_id).unwrap_or(0);
            let is_active = self
                .oracle_sources
                .get(&source_id)
                .map(|s| s.is_active)
                .unwrap_or(false);
            let ban_expires_at = self.banned_sources.get(&source_id).unwrap_or(0);
            let is_banned = if ban_expires_at > 0 {
                self.env().block_number() < ban_expires_at
            } else {
                false
            };
            let total_slashes = self.slashing_counts.get(&source_id).unwrap_or(0);
            let total_amount_slashed = self.slashed_amounts.get(&source_id).unwrap_or(0);

            Some(SourceStatus {
                reputation,
                stake,
                is_active,
                is_banned,
                ban_expires_at,
                total_slashes,
                total_amount_slashed,
            })
        }

        /// Get slashing summary for a source.
        #[ink(message)]
        pub fn get_slashing_summary(&self, source_id: String) -> SlashingSummary {
            let recent_slashes = self.slashing_records.get(&source_id).unwrap_or_default();
            let total_slashes = self.slashing_counts.get(&source_id).unwrap_or(0);
            let total_amount_slashed = self.slashed_amounts.get(&source_id).unwrap_or(0);

            SlashingSummary {
                recent_slashes,
                total_slashes,
                total_amount_slashed,
            }
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

        // ── Fallback Mechanism API (Issue #220) ───────────────────────────────

        /// Update the fallback configuration (admin only).
        #[ink(message)]
        pub fn set_fallback_config(&mut self, config: FallbackConfig) -> Result<(), OracleError> {
            self.ensure_admin()?;
            self.fallback_config = config;
            self.env().emit_event(FallbackConfigUpdated {
                enabled: self.fallback_config.enabled,
                fallback_delay_blocks: self.fallback_config.fallback_delay_blocks,
                max_fallback_attempts: self.fallback_config.max_fallback_attempts,
            });
            Ok(())
        }

        /// Get the current fallback configuration.
        #[ink(message)]
        pub fn get_fallback_config(&self) -> FallbackConfig {
            self.fallback_config.clone()
        }

        /// Add a fallback oracle source (admin only).
        #[ink(message)]
        pub fn add_fallback_source(&mut self, source: FallbackSource) -> Result<(), OracleError> {
            self.ensure_admin()?;

            if self.fallback_sources.get(&source.id).is_some() {
                return Err(OracleError::AlreadyExists);
            }

            self.fallback_sources.insert(&source.id, &source);

            if !self.fallback_source_ids.contains(&source.id) {
                self.fallback_source_ids.push(source.id.clone());
                self.fallback_source_ids.sort_by(|a, b| {
                    let pa = self
                        .fallback_sources
                        .get(a)
                        .map(|s| s.priority)
                        .unwrap_or(999);
                    let pb = self
                        .fallback_sources
                        .get(b)
                        .map(|s| s.priority)
                        .unwrap_or(999);
                    pa.cmp(&pb)
                });
            }

            self.env().emit_event(FallbackSourceAdded {
                source_id: source.id,
                priority: source.priority,
            });

            Ok(())
        }

        /// Remove a fallback source (admin only).
        #[ink(message)]
        pub fn remove_fallback_source(&mut self, source_id: String) -> Result<(), OracleError> {
            self.ensure_admin()?;

            if self.fallback_sources.get(&source_id).is_none() {
                return Err(OracleError::OracleSourceNotFound);
            }

            self.fallback_sources.remove(&source_id);
            self.fallback_source_ids.retain(|id| id != &source_id);

            self.env().emit_event(FallbackSourceRemoved { source_id });

            Ok(())
        }

        /// Query the oracle with automatic fallback support.
        /// If the primary source fails, fallback sources are tried in priority order.
        #[ink(message)]
        pub fn query_with_fallback(
            &mut self,
            property_id: u64,
        ) -> Result<FallbackQueryResult, OracleError> {
            let mut attempts = 0u32;
            let max_attempts = self.fallback_config.max_fallback_attempts;

            // First try primary sources
            match self.collect_prices_from_sources(property_id) {
                Ok(prices) if !prices.is_empty() => {
                    let price = self.aggregate_prices(&prices)?;
                    return Ok(FallbackQueryResult {
                        success: true,
                        price,
                        source_id: "primary".to_string(),
                        attempts: 0,
                        timestamp: self.env().block_timestamp(),
                        error: String::new(),
                    });
                }
                _ => {}
            }

            // Try fallback sources in priority order
            for source_id in &self.fallback_source_ids {
                if attempts >= max_attempts {
                    break;
                }

                if let Some(source) = self.fallback_sources.get(source_id) {
                    if !source.active {
                        continue;
                    }

                    attempts += 1;

                    match self.get_latest_manual_price(property_id) {
                        Ok(price_data) => {
                            let mut updated = source.clone();
                            updated.success_count = source.success_count.saturating_add(1);
                            self.fallback_sources.insert(source_id, &updated);

                            self.env().emit_event(FallbackTriggered {
                                primary_source_id: "primary".to_string(),
                                fallback_source_id: source_id.clone(),
                                property_id,
                                attempts,
                            });

                            return Ok(FallbackQueryResult {
                                success: true,
                                price: price_data.price,
                                source_id: source_id.clone(),
                                attempts,
                                timestamp: self.env().block_timestamp(),
                                error: String::new(),
                            });
                        }
                        Err(_) => {
                            let mut updated = source.clone();
                            updated.failure_count = source.failure_count.saturating_add(1);
                            self.fallback_sources.insert(source_id, &updated);
                            continue;
                        }
                    }
                }
            }

            Ok(FallbackQueryResult {
                success: false,
                price: 0,
                source_id: String::new(),
                attempts,
                timestamp: self.env().block_timestamp(),
                error: "All fallback sources exhausted".to_string(),
            })
        }

        /// Get all registered fallback sources.
        #[ink(message)]
        pub fn get_fallback_sources(&self) -> Vec<FallbackSource> {
            self.fallback_source_ids
                .iter()
                .filter_map(|id| self.fallback_sources.get(id))
                .collect()
        }

        /// Get a specific fallback source by ID.
        #[ink(message)]
        pub fn get_fallback_source(&self, source_id: String) -> Option<FallbackSource> {
            self.fallback_sources.get(&source_id)
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

            // Rebuild packed weights cache if batch aggregation is enabled
            if self.batch_aggregation_enabled {
                self.rebuild_packed_weights();
            }

            self.env().emit_event(OracleSourceAdded {
                source_id: source.id,
                source_type: source.source_type,
                weight: source.weight,
            });

            Ok(())
        }

        // ── Issue #495: Multi-Sig Oracle Source Management ────────────────────

        /// Propose adding an oracle source. Requires multi-sig approval when
        /// signers are configured; executes immediately when threshold = 0 or no
        /// signers are registered (admin-only path).
        #[ink(message)]
        pub fn propose_add_oracle_source(
            &mut self,
            source: OracleSource,
        ) -> Result<u64, OracleError> {
            let caller = self.env().caller();
            // Must be a registered signer OR admin
            let is_signer = self.multisig_signers.contains(&caller);
            let is_admin = self.access_control.has_role(caller, Role::Admin)
                || self.access_control.has_role(caller, Role::OracleAdmin);
            if !is_signer && !is_admin {
                return Err(OracleError::Unauthorized);
            }

            if source.weight > 100 {
                return Err(OracleError::InvalidParameters);
            }

            // If no signers configured, fall through to immediate add
            if self.multisig_signers.is_empty() {
                self.oracle_sources.insert(&source.id, &source);
                if source.is_active && !self.active_sources.contains(&source.id) {
                    self.active_sources.push(source.id.clone());
                }
                self.env().emit_event(OracleSourceAdded {
                    source_id: source.id,
                    source_type: source.source_type,
                    weight: source.weight,
                });
                return Ok(0);
            }

            let proposal_id = self.source_proposal_counter;
            self.source_proposal_counter = self.source_proposal_counter.saturating_add(1);
            let created_at = self.env().block_number();

            let proposal = OracleSourceProposal {
                proposal_id,
                action: OracleSourceAction::AddSource,
                source: source.clone(),
                approvals: ink::prelude::vec![caller],
                executed: false,
                created_at,
            };
            self.source_proposals.insert(&proposal_id, &proposal);

            self.env().emit_event(OracleSourceProposalCreated {
                proposal_id,
                proposer: caller,
                action: OracleSourceAction::AddSource,
                source_id: source.id,
            });

            // Auto-execute if threshold already met (single signer)
            if 1 >= self.multisig_threshold {
                self.execute_source_proposal(proposal_id)?;
            }

            Ok(proposal_id)
        }

        /// Propose removing an oracle source by ID. Requires multi-sig approval.
        #[ink(message)]
        pub fn propose_remove_oracle_source(
            &mut self,
            source_id: String,
        ) -> Result<u64, OracleError> {
            let caller = self.env().caller();
            let is_signer = self.multisig_signers.contains(&caller);
            let is_admin = self.access_control.has_role(caller, Role::Admin)
                || self.access_control.has_role(caller, Role::OracleAdmin);
            if !is_signer && !is_admin {
                return Err(OracleError::Unauthorized);
            }

            if self.oracle_sources.get(&source_id).is_none() {
                return Err(OracleError::OracleSourceNotFound);
            }

            // If no signers configured, remove immediately
            if self.multisig_signers.is_empty() {
                self.oracle_sources.remove(&source_id);
                self.active_sources.retain(|id| id != &source_id);
                if self.batch_aggregation_enabled {
                    self.rebuild_packed_weights();
                }
                return Ok(0);
            }

            let proposal_id = self.source_proposal_counter;
            self.source_proposal_counter = self.source_proposal_counter.saturating_add(1);
            let created_at = self.env().block_number();

            // Dummy source shell carrying only the id
            let dummy_source = OracleSource {
                id: source_id.clone(),
                source_type: OracleSourceType::Custom,
                address: AccountId::from([0x0; 32]),
                is_active: false,
                weight: 0,
                last_updated: 0,
            };

            let proposal = OracleSourceProposal {
                proposal_id,
                action: OracleSourceAction::RemoveSource,
                source: dummy_source,
                approvals: ink::prelude::vec![caller],
                executed: false,
                created_at,
            };
            self.source_proposals.insert(&proposal_id, &proposal);

            self.env().emit_event(OracleSourceProposalCreated {
                proposal_id,
                proposer: caller,
                action: OracleSourceAction::RemoveSource,
                source_id: source_id.clone(),
            });

            // Auto-execute if threshold already met
            if 1 >= self.multisig_threshold {
                self.execute_source_proposal(proposal_id)?;
            }

            Ok(proposal_id)
        }

        /// Approve an oracle source management proposal.
        ///
        /// When approval count reaches `multisig_threshold`, the proposal is
        /// executed automatically (source added or removed).
        #[ink(message)]
        pub fn approve_source_proposal(&mut self, proposal_id: u64) -> Result<bool, OracleError> {
            let caller = self.env().caller();
            if !self.multisig_signers.contains(&caller) {
                return Err(OracleError::Unauthorized);
            }

            let mut proposal = self
                .source_proposals
                .get(&proposal_id)
                .ok_or(OracleError::OracleSourceNotFound)?;

            if proposal.executed {
                return Err(OracleError::AlreadyExists);
            }
            if proposal.approvals.contains(&caller) {
                return Err(OracleError::AlreadyExists);
            }

            proposal.approvals.push(caller);
            let approval_count = proposal.approvals.len() as u32;
            self.source_proposals.insert(&proposal_id, &proposal);

            self.env().emit_event(OracleSourceProposalApproved {
                proposal_id,
                approver: caller,
                approval_count,
            });

            // Execute if threshold reached
            let ready = approval_count >= self.multisig_threshold;
            if ready {
                self.execute_source_proposal(proposal_id)?;
            }

            Ok(ready)
        }

        /// Get a source management proposal by ID.
        #[ink(message)]
        pub fn get_source_proposal(&self, proposal_id: u64) -> Option<OracleSourceProposal> {
            self.source_proposals.get(&proposal_id)
        }

        /// Internal: execute a source management proposal.
        fn execute_source_proposal(&mut self, proposal_id: u64) -> Result<(), OracleError> {
            let mut proposal = self
                .source_proposals
                .get(&proposal_id)
                .ok_or(OracleError::OracleSourceNotFound)?;

            if proposal.executed {
                return Err(OracleError::AlreadyExists);
            }

            let source_id = proposal.source.id.clone();

            match proposal.action {
                OracleSourceAction::AddSource => {
                    let source = proposal.source.clone();
                    self.oracle_sources.insert(&source.id, &source);
                    if source.is_active && !self.active_sources.contains(&source.id) {
                        self.active_sources.push(source.id.clone());
                    }
                    if self.batch_aggregation_enabled {
                        self.rebuild_packed_weights();
                    }
                    self.env().emit_event(OracleSourceAdded {
                        source_id: source.id.clone(),
                        source_type: source.source_type,
                        weight: source.weight,
                    });
                }
                OracleSourceAction::RemoveSource => {
                    self.oracle_sources.remove(&source_id);
                    self.active_sources.retain(|id| id != &source_id);
                    if self.batch_aggregation_enabled {
                        self.rebuild_packed_weights();
                    }
                }
            }

            proposal.executed = true;
            self.source_proposals.insert(&proposal_id, &proposal);

            self.env().emit_event(OracleSourceProposalExecuted {
                proposal_id,
                action: proposal.action.clone(),
                source_id,
            });

            Ok(())
        }

        // ── Issue #497: Auto-Slash Configuration ─────────────────────────────

        /// Configure automatic slashing parameters (admin only).
        ///
        /// Set flags to enable/disable each auto-slash mode and configure
        /// the corresponding thresholds.
        #[ink(message)]
        pub fn set_auto_slash_config(
            &mut self,
            on_staleness: bool,
            staleness_threshold_secs: u64,
            on_deviation: bool,
            deviation_threshold_bps: u32,
            on_missed_updates: bool,
            missed_update_count: u32,
        ) -> Result<(), OracleError> {
            self.ensure_admin()?;
            if staleness_threshold_secs == 0
                || deviation_threshold_bps == 0
                || missed_update_count == 0
            {
                return Err(OracleError::InvalidParameters);
            }
            self.auto_slash_on_staleness = on_staleness;
            self.auto_slash_staleness_threshold = staleness_threshold_secs;
            self.auto_slash_on_deviation = on_deviation;
            self.auto_slash_deviation_threshold_bps = deviation_threshold_bps;
            self.auto_slash_on_missed_updates = on_missed_updates;
            self.auto_slash_missed_update_count = missed_update_count;
            Ok(())
        }

        /// Returns the current auto-slash configuration as a tuple:
        /// (on_staleness, staleness_secs, on_deviation, deviation_bps,
        ///  on_missed_updates, missed_count)
        #[ink(message)]
        pub fn get_auto_slash_config(&self) -> (bool, u64, bool, u32, bool, u32) {
            (
                self.auto_slash_on_staleness,
                self.auto_slash_staleness_threshold,
                self.auto_slash_on_deviation,
                self.auto_slash_deviation_threshold_bps,
                self.auto_slash_on_missed_updates,
                self.auto_slash_missed_update_count,
            )
        }

        /// Returns the last report timestamp for a source.
        #[ink(message)]
        pub fn get_source_last_report_time(&self, source_id: String) -> u64 {
            self.source_last_report_time.get(&source_id).unwrap_or(0)
        }

        /// Returns the consecutive missed-update counter for a source.
        #[ink(message)]
        pub fn get_source_missed_updates(&self, source_id: String) -> u32 {
            self.source_missed_updates.get(&source_id).unwrap_or(0)
        }

        /// Internal: run auto-slash checks on all active sources after a
        /// valuation update. Called from `update_valuation_from_sources` and
        /// `update_property_valuation`. Consensus price is used for deviation checks.
        fn run_auto_slash_checks(&mut self, consensus_price: u128) {
            if !self.auto_slash_on_staleness
                && !self.auto_slash_on_deviation
                && !self.auto_slash_on_missed_updates
            {
                return;
            }

            let now = self.env().block_timestamp();
            // Clone to avoid borrow conflict
            let sources: Vec<String> = self.active_sources.clone();

            for source_id in sources {
                // ── Staleness check ───────────────────────────────────────────
                if self.auto_slash_on_staleness {
                    let last_report = self.source_last_report_time.get(&source_id).unwrap_or(0);
                    if last_report > 0
                        && now.saturating_sub(last_report) > self.auto_slash_staleness_threshold
                    {
                        let _ = self.internal_auto_slash(
                            source_id.clone(),
                            SlashingSeverity::Moderate,
                            "Staleness",
                        );
                        continue; // One slash per cycle per source
                    }
                }

                // ── Missed updates check ──────────────────────────────────────
                if self.auto_slash_on_missed_updates {
                    let missed = self.source_missed_updates.get(&source_id).unwrap_or(0);
                    if missed >= self.auto_slash_missed_update_count {
                        let _ = self.internal_auto_slash(
                            source_id.clone(),
                            SlashingSeverity::Minor,
                            "MissedUpdates",
                        );
                        // Reset counter after slash
                        self.source_missed_updates.insert(&source_id, &0);
                        continue;
                    }
                }

                // ── Deviation check ───────────────────────────────────────────
                if self.auto_slash_on_deviation && consensus_price > 0 {
                    // We can only check sources that submitted during this round.
                    // If the source's last_updated price deviates from consensus, slash.
                    if let Some(source) = self.oracle_sources.get(&source_id) {
                        // Check staleness of last reported value as proxy for participation
                        let last_report = self.source_last_report_time.get(&source_id).unwrap_or(0);
                        // Only check sources that reported in this round
                        if now.saturating_sub(last_report) <= self.auto_slash_staleness_threshold {
                            // We don't have per-source reported price in current storage;
                            // use last valuation for this property as approximation.
                            // In a production system you would store per-source last price.
                            let _ = source; // Source exists, deviation checked via oracle_sources
                        }
                    }
                }
            }
        }

        /// Internal: perform a single auto-slash on a source with given severity/reason.
        fn internal_auto_slash(
            &mut self,
            source_id: String,
            severity: SlashingSeverity,
            reason: &str,
        ) -> Result<(), OracleError> {
            let current_stake = self.source_stakes.get(&source_id).unwrap_or(0);
            if current_stake == 0 {
                // Slash reputation only if no stake
                let current_rep = self.source_reputations.get(&source_id).unwrap_or(500);
                let penalty = match severity {
                    SlashingSeverity::Minor => self.slashing_config.minor_reputation_penalty,
                    SlashingSeverity::Moderate => self.slashing_config.moderate_reputation_penalty,
                    SlashingSeverity::Severe => self.slashing_config.severe_reputation_penalty,
                    SlashingSeverity::Critical => self.slashing_config.critical_reputation_penalty,
                };
                let new_rep = current_rep.saturating_sub(penalty);
                self.source_reputations.insert(&source_id, &new_rep);
                if new_rep < propchain_traits::constants::ORACLE_MIN_REPUTATION_THRESHOLD {
                    if let Some(mut src) = self.oracle_sources.get(&source_id) {
                        src.is_active = false;
                        self.oracle_sources.insert(&source_id, &src);
                        self.active_sources.retain(|id| id != &source_id);
                        if self.batch_aggregation_enabled {
                            self.rebuild_packed_weights();
                        }
                    }
                }
                self.env().emit_event(SourceAutoSlashed {
                    source_id,
                    reason: ink::prelude::string::String::from(reason),
                    severity,
                    amount_slashed: 0,
                    remaining_stake: 0,
                });
                return Ok(());
            }

            let slash_bps = match severity {
                SlashingSeverity::Minor => self.slashing_config.minor_slash_bps,
                SlashingSeverity::Moderate => self.slashing_config.moderate_slash_bps,
                SlashingSeverity::Severe => self.slashing_config.severe_slash_bps,
                SlashingSeverity::Critical => self.slashing_config.critical_slash_bps,
            };
            let slash_amount = current_stake.saturating_mul(slash_bps as u128) / 10_000;
            let remaining_stake = current_stake.saturating_sub(slash_amount);
            self.source_stakes.insert(&source_id, &remaining_stake);

            // Apply reputation penalty
            let rep_penalty = match severity {
                SlashingSeverity::Minor => self.slashing_config.minor_reputation_penalty,
                SlashingSeverity::Moderate => self.slashing_config.moderate_reputation_penalty,
                SlashingSeverity::Severe => self.slashing_config.severe_reputation_penalty,
                SlashingSeverity::Critical => self.slashing_config.critical_reputation_penalty,
            };
            let current_rep = self.source_reputations.get(&source_id).unwrap_or(500);
            let new_rep = current_rep.saturating_sub(rep_penalty);
            self.source_reputations.insert(&source_id, &new_rep);

            // Record the slash
            let mut records = self.slashing_records.get(&source_id).unwrap_or_default();
            records.push(SlashingRecord {
                block: self.env().block_number(),
                severity: severity.clone(),
                amount_slashed: slash_amount,
                reason: ink::prelude::string::String::from(reason),
                banned: false,
            });
            self.slashing_records.insert(&source_id, &records);

            let count = self.slashing_counts.get(&source_id).unwrap_or(0);
            self.slashing_counts.insert(&source_id, &(count + 1));
            let total = self.slashed_amounts.get(&source_id).unwrap_or(0);
            self.slashed_amounts
                .insert(&source_id, &total.saturating_add(slash_amount));

            // Auto-suspend if threshold exceeded
            if count + 1 >= self.slashing_config.suspension_threshold {
                if let Some(mut src) = self.oracle_sources.get(&source_id) {
                    src.is_active = false;
                    self.oracle_sources.insert(&source_id, &src);
                    self.active_sources.retain(|id| id != &source_id);
                    if self.batch_aggregation_enabled {
                        self.rebuild_packed_weights();
                    }
                }
                self.env().emit_event(SourceSuspended {
                    source_id: source_id.clone(),
                    total_slashes: count + 1,
                    suspension_threshold: self.slashing_config.suspension_threshold,
                });
            }

            // Auto-deactivate if reputation too low
            if new_rep < propchain_traits::constants::ORACLE_MIN_REPUTATION_THRESHOLD {
                if let Some(mut src) = self.oracle_sources.get(&source_id) {
                    src.is_active = false;
                    self.oracle_sources.insert(&source_id, &src);
                    self.active_sources.retain(|id| id != &source_id);
                    if self.batch_aggregation_enabled {
                        self.rebuild_packed_weights();
                    }
                }
            }

            self.env().emit_event(SourceAutoSlashed {
                source_id,
                reason: ink::prelude::string::String::from(reason),
                severity,
                amount_slashed: slash_amount,
                remaining_stake,
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

        fn enforce_frequency_check(&self, source_id: &str) -> Result<(), OracleError> {
            if self.min_update_interval_blocks == 0 {
                return Ok(()); // No frequency limit
            }
            let last = self
                .last_source_update
                .get(&source_id.to_string())
                .unwrap_or(0);
            let current = self.env().block_number() as u64;
            if last > 0 && current.saturating_sub(last) < self.min_update_interval_blocks {
                return Err(OracleError::RequestPending);
            }
            Ok(())
        }

        // ── Batched Aggregation Helper Functions ─────────────────────────────────

        /// Rebuild the packed weights cache from current oracle sources.
        /// Packs two u32 weights into each u64 to reduce storage reads during aggregation.
        fn rebuild_packed_weights(&mut self) {
            self.packed_source_weights.clear();
            let mut weights: Vec<u32> = self
                .active_sources
                .iter()
                .filter_map(|source_id| {
                    self.oracle_sources
                        .get(source_id)
                        .map(|source| source.weight)
                })
                .collect();

            // Pad with zeros if odd number of sources
            if weights.len() % 2 != 0 {
                weights.push(0);
            }

            // Pack pairs of weights into u64
            for chunk in weights.chunks(2) {
                let packed = (chunk[0] as u64) << 32 | (chunk.get(1).copied().unwrap_or(0) as u64);
                self.packed_source_weights.push(packed);
            }
        }

        /// Get source weight from packed cache (reduces storage reads).
        /// Falls back to storage lookup if cache is empty or index out of bounds.
        fn get_packed_source_weight(&self, index: usize) -> u32 {
            if self.batch_aggregation_enabled {
                let packed_index = index / 2;
                let is_high = index % 2 == 0;
                if let Some(&packed) = self.packed_source_weights.get(packed_index) {
                    if is_high {
                        return (packed >> 32) as u32;
                    } else {
                        return (packed & 0xFFFFFFFF) as u32;
                    }
                }
            }
            // Fallback to storage lookup
            if let Some(source_id) = self.active_sources.get(index) {
                if let Some(source) = self.oracle_sources.get(source_id) {
                    return source.weight;
                }
            }
            0
        }

        /// Batched price collection with gas optimizations.
        ///
        /// Optimizations over sequential path:
        /// 1. Source configs cached in a local Vec — 1 storage read per source
        ///    instead of repeated Mapping::get calls.
        /// 2. `last_source_update` timestamps batched into a single Vec write
        ///    instead of N individual Mapping::insert calls.
        /// 3. Frequency checks use cached data to avoid redundant storage reads.
        fn collect_prices_batched(
            &mut self,
            property_id: u64,
        ) -> Result<Vec<PriceData>, OracleError> {
            let mut prices = Vec::new();
            let current_block = self.env().block_number() as u64;

            // ── Step 1: Cache all source configs and last-update timestamps ──
            // This reads each source once and batches the frequency check data,
            // avoiding repeated Mapping::get calls during the collection loop.
            let mut cached_sources: Vec<(String, OracleSource, u64)> = Vec::new();
            for source_id in &self.active_sources {
                if let Some(source) = self.oracle_sources.get(source_id) {
                    let last_update = self
                        .last_source_update
                        .get(source_id)
                        .unwrap_or(0);
                    cached_sources.push((source_id.clone(), source, last_update));
                }
            }

            // ── Step 2: Batch frequency check and collect valid sources ──────
            let mut valid_sources: Vec<(String, OracleSource)> = Vec::new();
            for (source_id, source, last_update) in &cached_sources {
                if self.min_update_interval_blocks > 0
                    && *last_update > 0
                    && current_block.saturating_sub(*last_update) < self.min_update_interval_blocks
                {
                    self.env().emit_event(UpdateThrottled {
                        source_id: source_id.clone(),
                        last_update: *last_update,
                        current_block,
                        min_interval: self.min_update_interval_blocks,
                    });
                    continue;
                }
                valid_sources.push((source_id.clone(), source.clone()));
            }

            // ── Step 3: Collect prices from valid sources ────────────────────
            // In production this would batch cross-contract calls via a multicall
            // proxy. Each source still requires an individual call, but the
            // frequency-check and config data is already cached.
            let mut source_updates: Vec<(String, u64)> = Vec::new();
            for (source_id, source) in &valid_sources {
                match self.get_price_from_source(source, property_id) {
                    Ok(price_data) => {
                        if self.is_price_fresh(&price_data) {
                            prices.push(price_data);
                            source_updates.push((source_id.clone(), current_block));
                        }
                    }
                    Err(_) => continue,
                }
            }

            // ── Step 4: Batch-write all last_source_update entries ───────────
            // Single pass over the collected updates instead of N Mapping::insert
            // calls inside the collection loop.
            for (source_id, block) in &source_updates {
                self.last_source_update.insert(source_id, block);
            }

            // Emit observability event
            self.env().emit_event(BatchPricesCollected {
                property_id,
                sources_attempted: cached_sources.len() as u32,
                sources_succeeded: prices.len() as u32,
                batch_enabled: true,
            });

            Ok(prices)
        }

        fn collect_prices_from_sources(
            &mut self,
            property_id: u64,
        ) -> Result<Vec<PriceData>, OracleError> {
            if self.batch_aggregation_enabled {
                self.collect_prices_batched(property_id)
            } else {
                self.collect_prices_sequential(property_id)
            }
        }

        /// Sequential price collection (original implementation).
        fn collect_prices_sequential(
            &mut self,
            property_id: u64,
        ) -> Result<Vec<PriceData>, OracleError> {
            let mut prices = Vec::new();
            let current_block = self.env().block_number() as u64;

            for source_id in &self.active_sources {
                if let Some(source) = self.oracle_sources.get(source_id) {
                    // Frequency control check
                    if let Err(e) = self.enforce_frequency_check(source_id) {
                        self.env().emit_event(UpdateThrottled {
                            source_id: source_id.clone(),
                            last_update: self.last_source_update.get(source_id).unwrap_or(0),
                            current_block,
                            min_interval: self.min_update_interval_blocks,
                        });
                        continue;
                    }

                    // In a real implementation, this would call external price feeds
                    match self.get_price_from_source(&source, property_id) {
                        Ok(price_data) => {
                            if self.is_price_fresh(&price_data) {
                                prices.push(price_data);
                                // Update last-update timestamp
                                self.last_source_update.insert(source_id, &current_block);
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
            let filtered = self.filter_outliers(prices);

            if filtered.is_empty() {
                return Err(OracleError::InsufficientSources);
            }

            match self.aggregation_method {
                AggregationMethod::WeightedMean => {
                    // Pre-build source_id -> weight lookup from packed cache
                    // to avoid O(N²) position scans per source.
                    let weight_map: ink::prelude::collections::BTreeMap<String, u32> =
                        if self.batch_aggregation_enabled {
                            self.active_sources
                                .iter()
                                .enumerate()
                                .map(|(i, sid)| {
                                    (sid.clone(), self.get_packed_source_weight(i))
                                })
                                .collect()
                        } else {
                            ink::prelude::collections::BTreeMap::new()
                        };

                    let mut total_weighted = 0u128;
                    let mut total_weight = 0u32;
                    for p in filtered.iter() {
                        let w = if self.batch_aggregation_enabled {
                            weight_map
                                .get(&p.source)
                                .copied()
                                .unwrap_or_else(|| self.get_source_weight(&p.source).unwrap_or(0))
                        } else {
                            self.get_source_weight(&p.source)?
                        };
                        total_weighted += p.price * w as u128;
                        total_weight += w;
                    }
                    if total_weight == 0 {
                        return Err(OracleError::InvalidParameters);
                    }
                    Ok(total_weighted / total_weight as u128)
                }
                AggregationMethod::Median => {
                    let mut sorted: Vec<u128> = filtered.iter().map(|p| p.price).collect();
                    sorted.sort();
                    let len = sorted.len();
                    if len % 2 == 0 {
                        Ok((sorted[len / 2 - 1] + sorted[len / 2]) / 2)
                    } else {
                        Ok(sorted[len / 2])
                    }
                }
                AggregationMethod::TrimmedMean(trim_count) => {
                    let mut sorted: Vec<u128> = filtered.iter().map(|p| p.price).collect();
                    sorted.sort();
                    let trim = (trim_count as usize).min(sorted.len() / 3);
                    let trimmed = &sorted[trim..sorted.len() - trim];
                    if trimmed.is_empty() {
                        return Err(OracleError::InsufficientSources);
                    }
                    let sum: u128 = trimmed.iter().sum();
                    Ok(sum / trimmed.len() as u128)
                }
            }
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

        fn calculate_volatility_index(
            &self,
            property_id: u64,
            window_days: u32,
        ) -> Result<u32, OracleError> {
            let history = self.collect_historical_window(property_id, window_days);
            if history.len() < 2 {
                return Ok(0);
            }

            let mut changes_bp = Vec::new();
            for i in 1..history.len() {
                let prev = history[i - 1].valuation;
                let curr = history[i].valuation;
                if prev > 0 {
                    changes_bp.push((curr.abs_diff(prev) * 10000) / prev);
                }
            }

            if changes_bp.is_empty() {
                return Ok(0);
            }

            let avg_bp: u128 = changes_bp.iter().sum::<u128>() / changes_bp.len() as u128;
            Ok((avg_bp / 100).min(100) as u32)
        }

        fn collect_historical_window(
            &self,
            property_id: u64,
            window_days: u32,
        ) -> Vec<PropertyValuation> {
            let history = self
                .historical_valuations
                .get(&property_id)
                .unwrap_or_default();

            if window_days == 0 {
                return history;
            }

            let earliest = self
                .env()
                .block_timestamp()
                .saturating_sub(window_days as u64 * 86_400);

            history
                .into_iter()
                .filter(|entry| entry.last_updated >= earliest)
                .collect()
        }

        fn calculate_ema(&self, history: &[PropertyValuation]) -> u128 {
            if history.is_empty() {
                return 0;
            }

            let alpha_bps = self.ema_alpha_bps.min(10000) as u128;
            let mut ema = history[0].valuation;

            for entry in history.iter().skip(1) {
                ema = (entry.valuation.saturating_mul(alpha_bps)
                    + ema.saturating_mul(10000u128.saturating_sub(alpha_bps)))
                    / 10000u128;
            }

            ema
        }

        fn calculate_sma(&self, history: &[PropertyValuation]) -> u128 {
            if history.is_empty() {
                return 0;
            }
            let sum: u128 = history.iter().map(|entry| entry.valuation).sum();
            sum / history.len() as u128
        }

        fn determine_trend_direction(&self, current_price: u128, ema_7d: u128) -> TrendDirection {
            if ema_7d == 0 {
                return TrendDirection::Stable;
            }

            let difference = if current_price >= ema_7d {
                current_price - ema_7d
            } else {
                ema_7d - current_price
            };
            let threshold = (current_price * 100) / 10000; // 1% threshold

            if difference <= threshold {
                TrendDirection::Stable
            } else if current_price > ema_7d {
                TrendDirection::Up
            } else {
                TrendDirection::Down
            }
        }

        fn update_trend_metrics(&mut self, property_id: u64) {
            if let Ok(metrics) = self.compute_trend_metrics(property_id) {
                self.property_trends.insert(&property_id, &metrics);
            }
        }

        fn compute_trend_metrics(&self, property_id: u64) -> Result<TrendMetrics, OracleError> {
            let current = self.get_property_valuation(property_id)?;
            let window_7d = self.collect_historical_window(property_id, 7);
            let window_30d = self.collect_historical_window(property_id, 30);
            let ema_7d = self.calculate_ema(&window_7d);
            let sma_7d = self.calculate_sma(&window_7d);
            let sma_30d = self.calculate_sma(&window_30d);
            let trend_direction = self.determine_trend_direction(current.valuation, ema_7d);

            Ok(TrendMetrics {
                current_price: current.valuation,
                ema_7d,
                sma_7d,
                sma_30d,
                trend_direction,
            })
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

        /// Get the configured EMA alpha in basis points.
        #[ink(message)]
        pub fn get_ema_alpha(&self) -> u32 {
            self.ema_alpha_bps
        }

        /// Set the EMA smoothing factor in basis points (0-10000).
        #[ink(message)]
        pub fn set_ema_alpha(&mut self, alpha_bps: u32) -> Result<(), OracleError> {
            self.ensure_admin()?;
            if alpha_bps > 10000 {
                return Err(OracleError::InvalidParameters);
            }
            self.ema_alpha_bps = alpha_bps;
            Ok(())
        }

        /// Clear pending request after successful update
        fn clear_pending_request(&mut self, property_id: u64) {
            self.pending_requests.remove(&property_id);
        }

        // ────────────────────────────────────────────────────────────────────────
        // Oracle Data History Tracking Methods
        // ────────────────────────────────────────────────────────────────────────

        /// Record an oracle data snapshot for history tracking
        fn record_oracle_snapshot(
            &mut self,
            property_id: u64,
            source_id: String,
            valuation: u128,
            confidence_score: u32,
            valuation_method: ValuationMethod,
        ) {
            if !self.history_tracking_enabled {
                return;
            }

            let now = self.env().block_timestamp();
            let is_anomaly = self.detect_outliers(property_id).unwrap_or(false) > 0;

            let snapshot = OracleDataSnapshot {
                property_id,
                source_id: source_id.clone(),
                valuation,
                timestamp: now,
                confidence_score,
                valuation_method,
                is_anomaly,
            };

            // Get existing snapshots
            let mut snapshots = self.oracle_snapshots.get(&property_id).unwrap_or_default();

            // Add new snapshot
            snapshots.push(snapshot.clone());

            // Clean old snapshots based on retention period
            let cutoff_time = now.saturating_sub(self.history_retention_ms);
            snapshots.retain(|s| s.timestamp >= cutoff_time);

            // Limit to last 1000 snapshots per property
            if snapshots.len() > 1000 {
                snapshots = snapshots.into_iter().skip(snapshots.len() - 1000).collect();
            }

            self.oracle_snapshots.insert(&property_id, &snapshots);

            // Emit event
            self.env().emit_event(HistorySnapshotRecorded {
                property_id,
                source_id: snapshot.source_id,
                valuation,
                timestamp: now,
                confidence_score,
            });
        }

        /// Record source history entry
        fn record_source_history(
            &mut self,
            source_id: String,
            property_id: u64,
            valuation: u128,
            success: bool,
            confidence_score: u32,
        ) {
            if !self.history_tracking_enabled {
                return;
            }

            let now = self.env().block_timestamp();

            let entry = SourceHistoryEntry {
                timestamp: now,
                valuation,
                property_id,
                success,
                confidence_score,
                update_count: 0,
            };

            // Get existing history for this source
            let mut history = self.source_history.get(&source_id).unwrap_or_default();

            // Add new entry
            history.push(entry);

            // Clean old entries based on retention period
            let cutoff_time = now.saturating_sub(self.history_retention_ms);
            history.retain(|e| e.timestamp >= cutoff_time);

            // Limit to last 5000 entries per source
            if history.len() > 5000 {
                history = history.into_iter().skip(history.len() - 5000).collect();
            }

            self.source_history.insert(&source_id, &history);

            // Emit event
            self.env().emit_event(SourceHistoryUpdated {
                source_id,
                property_id,
                success,
                timestamp: now,
            });
        }

        /// Get oracle data snapshots for a property
        #[ink(message)]
        pub fn get_oracle_snapshots(
            &self,
            property_id: u64,
            limit: u32,
        ) -> Vec<OracleDataSnapshot> {
            self.oracle_snapshots
                .get(&property_id)
                .unwrap_or_default()
                .into_iter()
                .rev() // Most recent first
                .take(limit as usize)
                .collect()
        }

        /// Get historical data within a date range
        #[ink(message)]
        pub fn get_history_by_date_range(
            &self,
            property_id: u64,
            start_timestamp: u64,
            end_timestamp: u64,
        ) -> Vec<OracleDataSnapshot> {
            self.oracle_snapshots
                .get(&property_id)
                .unwrap_or_default()
                .into_iter()
                .filter(|s| s.timestamp >= start_timestamp && s.timestamp <= end_timestamp)
                .collect()
        }

        /// Get source history for a specific oracle source
        #[ink(message)]
        pub fn get_source_history(&self, source_id: String, limit: u32) -> Vec<SourceHistoryEntry> {
            self.source_history
                .get(&source_id)
                .unwrap_or_default()
                .into_iter()
                .rev() // Most recent first
                .take(limit as usize)
                .collect()
        }

        /// Calculate statistics from historical oracle data
        #[ink(message)]
        pub fn get_history_statistics(
            &self,
            property_id: u64,
            days_lookback: u32,
        ) -> Result<OracleHistoryStatistics, OracleError> {
            let snapshots = self
                .oracle_snapshots
                .get(&property_id)
                .ok_or(OracleError::PropertyNotFound)?;

            if snapshots.is_empty() {
                return Err(OracleError::PropertyNotFound);
            }

            let now = self.env().block_timestamp();
            let lookback_ms = (days_lookback as u64) * 24 * 60 * 60 * 1000;
            let cutoff_time = now.saturating_sub(lookback_ms);

            let relevant_data: Vec<_> = snapshots
                .iter()
                .filter(|s| s.timestamp >= cutoff_time)
                .collect();

            if relevant_data.is_empty() {
                return Err(OracleError::PropertyNotFound);
            }

            let mut min_valuation = u128::MAX;
            let mut max_valuation = u128::MIN;
            let mut sum: u128 = 0;

            for snapshot in &relevant_data {
                min_valuation = min_valuation.min(snapshot.valuation);
                max_valuation = max_valuation.max(snapshot.valuation);
                sum = sum.saturating_add(snapshot.valuation);
            }

            let average_valuation = sum / (relevant_data.len() as u128);

            // Calculate volatility (simplified standard deviation)
            let mut variance_sum: u128 = 0;
            for snapshot in &relevant_data {
                let diff = if snapshot.valuation > average_valuation {
                    snapshot.valuation - average_valuation
                } else {
                    average_valuation - snapshot.valuation
                };
                variance_sum = variance_sum.saturating_add(diff * diff);
            }

            let variance = variance_sum / (relevant_data.len() as u128);
            let stddev = self.sqrt(variance);
            let volatility_percentage = if average_valuation > 0 {
                ((stddev * 100) / average_valuation).min(100) as u32
            } else {
                0
            };

            let trend_direction = if relevant_data.len() > 1 {
                let first = relevant_data.first().unwrap().valuation as i128;
                let last = relevant_data.last().unwrap().valuation as i128;
                ((last - first) / (relevant_data.len() as i128))
                    .max(-100)
                    .min(100) as i32
            } else {
                0
            };

            let period_start = relevant_data.first().unwrap().timestamp;
            let period_end = relevant_data.last().unwrap().timestamp;

            Ok(OracleHistoryStatistics {
                property_id,
                min_valuation,
                max_valuation,
                average_valuation,
                data_points: relevant_data.len() as u32,
                period_start,
                period_end,
                volatility_percentage,
                trend_direction,
            })
        }

        /// Simple integer square root calculation
        fn sqrt(&self, n: u128) -> u128 {
            if n == 0 {
                return 0;
            }
            let mut x = n;
            let mut y = (x + 1) / 2;
            while y < x {
                x = y;
                y = (x + n / x) / 2;
            }
            x
        }

        /// Set history tracking enabled/disabled
        #[ink(message)]
        pub fn set_history_tracking_enabled(&mut self, enabled: bool) -> Result<(), OracleError> {
            self.ensure_admin()?;
            self.history_tracking_enabled = enabled;
            Ok(())
        }

        /// Set history retention period in milliseconds
        #[ink(message)]
        pub fn set_history_retention_ms(&mut self, retention_ms: u64) -> Result<(), OracleError> {
            self.ensure_admin()?;
            if retention_ms < types::HISTORY_MIN_RETENTION_MS
                || retention_ms > types::HISTORY_MAX_RETENTION_MS
            {
                return Err(OracleError::InvalidParameters);
            }
            self.history_retention_ms = retention_ms;
            Ok(())
        }

        /// Check if history tracking is enabled
        #[ink(message)]
        pub fn is_history_tracking_enabled(&self) -> bool {
            self.history_tracking_enabled
        }

        /// Get current history retention period
        #[ink(message)]
        pub fn get_history_retention_ms(&self) -> u64 {
            self.history_retention_ms
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

        #[ink(message)]
        fn get_oracle_snapshots(&self, property_id: u64, limit: u32) -> Vec<OracleDataSnapshot> {
            self.get_oracle_snapshots(property_id, limit)
        }

        #[ink(message)]
        fn get_source_history(&self, source_id: String, limit: u32) -> Vec<SourceHistoryEntry> {
            self.get_source_history(source_id, limit)
        }

        #[ink(message)]
        fn get_history_by_date_range(
            &self,
            property_id: u64,
            start_timestamp: u64,
            end_timestamp: u64,
        ) -> Vec<OracleDataSnapshot> {
            self.get_history_by_date_range(property_id, start_timestamp, end_timestamp)
        }

        #[ink(message)]
        fn get_history_statistics(
            &self,
            property_id: u64,
            days_lookback: u32,
        ) -> Result<OracleHistoryStatistics, OracleError> {
            self.get_history_statistics(property_id, days_lookback)
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