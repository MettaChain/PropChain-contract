#![allow(clippy::clone_on_copy)] // fires inside ink! generated storage code
#![cfg_attr(not(feature = "std"), no_std)]
#![allow(unexpected_cfgs)]

use ink::prelude::string::String;
use ink::prelude::vec::Vec;
use ink::storage::Mapping;
use propchain_traits::constants;
use propchain_traits::{DynamicFeeProvider, FeeOperation};

/// Dynamic Fee and Market Mechanism contract for PropChain.
/// Implements congestion-based fees, premium listing auctions, validator incentives,
/// and fee transparency for network participants.
#[ink::contract]
// `errors.rs` is `include!()`d before `strategies.rs`, so its in-file
// `#[cfg(test)] mod tests` lands before the strategies items in the module
// body. `clippy::items_after_test_module` flags that ordering, but moving
// every test after every include isn't worth the structural churn, so
// suppress the lint here.
#[allow(clippy::items_after_test_module)]
pub mod propchain_fees {
    use super::*;

    /// Basis points denominator (10000 = 100%)
    const BASIS_POINTS: u128 = BasisPoints::DENOM as u128;

    /// Default congestion window: number of recent operations to consider
    const CONGESTION_WINDOW: u32 = 100;
    /// Max fee multiplier from congestion (e.g. 3x base)
    const MAX_CONGESTION_MULTIPLIER: u32 = 300; // 300% of base
    /// Length of a congestion window in seconds before it rolls over.
    const CONGESTION_WINDOW_SECS: u64 = 3600; // 1 hour

    include!("types.rs");
    include!("errors.rs");
    include!("strategies.rs");
    include!("rounding.rs");

    #[ink(storage)]
    pub struct FeeManager {
        admin: AccountId,
        /// Fee config per operation type (optional override; else use default)
        operation_config: Mapping<FeeOperation, FeeConfig>,
        /// Default fee config
        default_config: FeeConfig,
        /// Recent operation timestamps for congestion (ring buffer style: count per slot)
        recent_ops_count: u32,
        last_congestion_reset: u64,
        /// Premium listing auctions: auction_id -> PremiumAuction
        auctions: Mapping<u64, PremiumAuction>,
        auction_bids: Mapping<(u64, AccountId), AuctionBid>,
        auction_count: u64,
        /// Accumulated fees (to be distributed)
        fee_treasury: u128,
        /// Validator/participant rewards: account -> pending amount
        pending_rewards: Mapping<AccountId, u128>,
        /// Reward history (for reporting)
        reward_records: Mapping<u64, RewardRecord>,
        reward_record_count: u64,
        /// Total fees collected (all time)
        total_fees_collected: u128,
        /// Total distributed to validators/participants
        total_distributed: u128,
        /// Authorized validators (receive incentive share)
        validators: Mapping<AccountId, bool>,
        /// List of validator accounts for distribution (enumerable)
        validator_list: Vec<AccountId>,
        /// Distribution rate for validators (basis points of collected fees)
        validator_share_bp: BasisPoints,
        /// Distribution rate for treasury (rest)
        treasury_share_bp: BasisPoints,
        /// Dynamic fee configuration based on pool utilisation / market congestion
        dynamic_fee_config: DynamicFeeConfig,
    }

    #[ink(event)]
    pub struct FeeConfigUpdated {
        #[ink(topic)]
        by: AccountId,
        operation: Option<FeeOperation>,
        base_fee: u128,
        timestamp: u64,
    }

    #[ink(event)]
    pub struct PremiumAuctionCreated {
        #[ink(topic)]
        auction_id: u64,
        #[ink(topic)]
        property_id: u64,
        #[ink(topic)]
        seller: AccountId,
        min_bid: u128,
        end_time: u64,
        fee_paid: u128,
    }

    #[ink(event)]
    pub struct PremiumAuctionBid {
        #[ink(topic)]
        auction_id: u64,
        #[ink(topic)]
        bidder: AccountId,
        amount: u128,
        outbid_previous: u128,
    }

    #[ink(event)]
    pub struct PremiumAuctionSettled {
        #[ink(topic)]
        auction_id: u64,
        #[ink(topic)]
        property_id: u64,
        #[ink(topic)]
        winner: AccountId,
        amount: u128,
        timestamp: u64,
    }

    #[ink(event)]
    pub struct RewardsDistributed {
        #[ink(topic)]
        recipient: AccountId,
        amount: u128,
        reason: RewardReason,
        timestamp: u64,
    }

    /// Emitted whenever the dynamic fee rate changes due to a config update
    /// or a shift in pool utilisation tracked via `set_dynamic_fee_config`.
    #[ink(event)]
    pub struct FeeRateUpdated {
        #[ink(topic)]
        by: AccountId,
        /// Previous effective fee rate in basis points
        old_rate_bps: BasisPoints,
        /// New effective fee rate in basis points
        new_rate_bps: BasisPoints,
        timestamp: u64,
    }

    impl FeeManager {
        #[ink(constructor)]
        pub fn new(base_fee: u128, min_fee: u128, max_fee: u128) -> Self {
            let caller = Self::env().caller();
            let timestamp = Self::env().block_timestamp();
            let default_config = FeeConfig {
                base_fee,
                min_fee,
                max_fee,
                congestion_sensitivity: 80,
                demand_factor_bp: BasisPoints::new(500),
                calculation_method: FeeCalculationMethod::Dynamic,
                last_updated: timestamp,
            };
            Self {
                admin: caller,
                operation_config: Mapping::default(),
                default_config,
                recent_ops_count: 0,
                last_congestion_reset: timestamp,
                auctions: Mapping::default(),
                auction_bids: Mapping::default(),
                auction_count: 0,
                fee_treasury: 0,
                pending_rewards: Mapping::default(),
                reward_records: Mapping::default(),
                reward_record_count: 0,
                total_fees_collected: 0,
                total_distributed: 0,
                validators: Mapping::default(),
                validator_list: Vec::new(),
                validator_share_bp: BasisPoints::new(5000), // 50% to validators
                treasury_share_bp: BasisPoints::new(5000),  // 50% to treasury
                dynamic_fee_config: DynamicFeeConfig {
                    // Reference rate used by `calculate_fee` (Issue #1118):
                    // at this base the dynamic model is a no-op, so behaviour
                    // is unchanged until an admin reconfigures it.
                    base_fee_bps: BasisPoints::new(constants::FEE_DYNAMIC_REFERENCE_BPS),
                    congestion_multiplier: 300,         // up to 3× at full utilisation
                    max_fee_bps: BasisPoints::new(200), // hard cap at 2.00 %
                },
            }
        }

        fn ensure_admin(&self) -> Result<(), FeeError> {
            if self.env().caller() != self.admin {
                return Err(FeeError::Unauthorized);
            }
            Ok(())
        }

        /// Get config for operation (operation-specific or default)
        fn get_config(&self, op: FeeOperation) -> FeeConfig {
            self.operation_config
                .get(op)
                .unwrap_or(self.default_config.clone())
        }

        /// True once the current congestion window has fully elapsed.
        fn congestion_window_elapsed(&self, now: u64) -> bool {
            now.saturating_sub(self.last_congestion_reset) >= CONGESTION_WINDOW_SECS
        }

        /// Roll the congestion window over once it has fully elapsed: the
        /// window restarts from `now` with a fresh (empty) operation count.
        ///
        /// This is the single place that mutates `last_congestion_reset`, so
        /// pricing and fee recording always share one notion of the window
        /// (Issue #1121). It is called from every mutating path that prices
        /// against congestion so an idle tail cannot leave the window stale.
        fn rollover_congestion(&mut self) {
            let now = self.env().block_timestamp();
            if self.congestion_window_elapsed(now) {
                self.last_congestion_reset = now;
                self.recent_ops_count = 0;
            }
        }

        /// Compute current congestion index (0-100) from recent activity.
        /// A rolled-over (empty) window reports 0 congestion.
        fn congestion_index(&self) -> u32 {
            let now = self.env().block_timestamp();
            let count = if self.congestion_window_elapsed(now) {
                0
            } else {
                self.recent_ops_count
            };
            // Normalize to 0-100: CONGESTION_WINDOW ops = 100
            (count.saturating_mul(100).saturating_div(CONGESTION_WINDOW)).min(100)
        }

        /// Demand factor in basis points.
        ///
        /// Returns the configured demand factor directly as the additive
        /// market-demand component of the fee model. It deliberately does NOT
        /// scale itself by the congestion index: congestion is already applied
        /// by the dynamic strategy from the same `recent_ops_count`, so
        /// scaling here would double-count recent volume (Issue #1121).
        fn demand_factor_bp(&self) -> BasisPoints {
            self.default_config.demand_factor_bp
        }

        // ========== Dynamic fee calculation ==========

        /// Calculate fee for an operation using configured strategy
        #[ink(message)]
        pub fn calculate_fee(&self, operation: FeeOperation) -> u128 {
            let config = self.get_config(operation);
            let context = FeeContext {
                congestion_index: self.congestion_index(),
                demand_factor_bp: self.demand_factor_bp(),
                operation,
            };
            let fee = FeeCalculator::calculate(&config, &context);
            // Feed the stored DynamicFeeConfig into the live fee path (Issue
            // #1118). Before this, `dynamic_fee_config` was settable and
            // queryable but never reached the calculation, so `FeeRateUpdated`
            // events were a silent lie. For the congestion-driven strategies the
            // effective dynamic rate (in bps) scales the strategy fee relative
            // to the reference rate, so a fresh contract (base_fee_bps ==
            // reference) returns exactly what it did before.
            match config.calculation_method {
                FeeCalculationMethod::Dynamic | FeeCalculationMethod::Exponential => {
                    let dynamic_rate_bps =
                        Self::compute_fee_rate(&self.dynamic_fee_config, context.congestion_index)
                            as u128;
                    fee.saturating_mul(dynamic_rate_bps)
                        .saturating_div(constants::FEE_DYNAMIC_REFERENCE_BPS as u128)
                        .clamp(config.min_fee, config.max_fee)
                }
                FeeCalculationMethod::Fixed | FeeCalculationMethod::Tiered => fee,
            }
        }

        /// Record that a fee was collected (admin only).
        ///
        /// Fees are booked straight into `fee_treasury`/`total_fees_collected`
        /// and later become claimable via `distribute_fees`, so only the admin
        /// may mutate the ledger. Previously this was an unauthenticated
        /// message that let any caller forge the treasury (Issue #1117).
        #[ink(message)]
        pub fn record_fee_collected(
            &mut self,
            _operation: FeeOperation,
            amount: u128,
            from: AccountId,
        ) -> Result<(), FeeError> {
            self.ensure_admin()?;
            let _ = from;
            // Roll the window over first so the increment below is attributed
            // to a fresh window (shared rollover — `rollover_congestion`).
            self.rollover_congestion();
            self.recent_ops_count = self
                .recent_ops_count
                .saturating_add(1)
                .min(CONGESTION_WINDOW);
            self.fee_treasury = self.fee_treasury.saturating_add(amount);
            self.total_fees_collected = self.total_fees_collected.saturating_add(amount);
            Ok(())
        }

        // ========== Automated fee adjustment ==========

        /// Automated fee adjustment based on recent utilization vs target
        #[ink(message)]
        pub fn update_fee_params(&mut self) -> Result<(), FeeError> {
            self.ensure_admin()?;
            // Roll the congestion window so the fee parameters are derived from
            // the current window even when no new fee records arrived during
            // the window's tail (Issue #1121).
            self.rollover_congestion();
            let now = self.env().block_timestamp();
            let congestion = self.congestion_index();
            let mut config = self.default_config.clone();
            if congestion > 70 {
                config.base_fee = config
                    .base_fee
                    .saturating_mul(105)
                    .saturating_div(100)
                    .min(config.max_fee);
            } else if congestion < 30 {
                config.base_fee = config
                    .base_fee
                    .saturating_mul(95)
                    .saturating_div(100)
                    .max(config.min_fee);
            }
            config.last_updated = now;
            self.default_config = config.clone();
            self.env().emit_event(FeeConfigUpdated {
                by: self.env().caller(),
                operation: None,
                base_fee: config.base_fee,
                timestamp: now,
            });
            Ok(())
        }

        /// Set fee config for an operation (admin)
        #[ink(message)]
        pub fn set_operation_config(
            &mut self,
            operation: FeeOperation,
            config: FeeConfig,
        ) -> Result<(), FeeError> {
            self.ensure_admin()?;
            if config.min_fee > config.max_fee || config.base_fee < config.min_fee {
                return Err(FeeError::InvalidConfig);
            }
            self.operation_config.insert(operation, &config);
            self.env().emit_event(FeeConfigUpdated {
                by: self.env().caller(),
                operation: Some(operation),
                base_fee: config.base_fee,
                timestamp: self.env().block_timestamp(),
            });
            Ok(())
        }

        // ========== Auction mechanism for premium listings ==========

        /// Create premium listing auction (pay fee; fee goes to treasury)
        #[ink(message)]
        pub fn create_premium_auction(
            &mut self,
            property_id: u64,
            min_bid: u128,
            duration_seconds: u64,
        ) -> Result<u64, FeeError> {
            let caller = self.env().caller();
            let now = self.env().block_timestamp();
            let fee = self.calculate_fee(FeeOperation::PremiumListingBid);
            if fee > 0 {
                self.fee_treasury = self.fee_treasury.saturating_add(fee);
                self.total_fees_collected = self.total_fees_collected.saturating_add(fee);
            }
            self.auction_count += 1;
            let auction_id = self.auction_count;
            let auction = PremiumAuction {
                property_id,
                seller: caller,
                min_bid,
                current_bid: 0,
                current_bidder: None,
                escrowed_value: 0,
                end_time: now.saturating_add(duration_seconds),
                settled: false,
                fee_paid: fee,
            };
            self.auctions.insert(auction_id, &auction);
            self.env().emit_event(PremiumAuctionCreated {
                auction_id,
                property_id,
                seller: caller,
                min_bid,
                end_time: auction.end_time,
                fee_paid: fee,
            });
            Ok(auction_id)
        }

        /// Place or increase a bid on an active premium listing auction.
        ///
        /// The bid is **payable**: the caller must attach the full bid amount
        /// as transferred value so the contract can hold it in custody until
        /// settlement. Any overpayment above the bid amount is immediately
        /// refunded, and the previous highest bidder is fully refunded their
        /// escrowed amount before the new bid is recorded.
        #[ink(message, payable)]
        pub fn place_bid(&mut self, auction_id: u64, amount: u128) -> Result<(), FeeError> {
            let caller = self.env().caller();
            let now = self.env().block_timestamp();
            let attached = self.env().transferred_value();
            let mut auction = self
                .auctions
                .get(auction_id)
                .ok_or(FeeError::AuctionNotFound)?;
            if auction.settled {
                return Err(FeeError::AlreadySettled);
            }
            if now >= auction.end_time {
                return Err(FeeError::AuctionEnded);
            }
            if amount < auction.min_bid {
                return Err(FeeError::BidTooLow);
            }
            if amount <= auction.current_bid {
                return Err(FeeError::BidTooLow);
            }
            // The attached value must cover the whole bid — it is held in
            // custody by the contract until settlement.
            if attached < amount {
                return Err(FeeError::InsufficientValue);
            }

            // Refund the previous highest bidder before recording the new bid
            // (checks-effects-interactions: state is updated below only after
            // every fallible operation has succeeded).
            let previous_escrow = auction.escrowed_value;
            if let Some(previous_bidder) = auction.current_bidder {
                if previous_escrow > 0
                    && self
                        .env()
                        .transfer(previous_bidder, previous_escrow)
                        .is_err()
                {
                    return Err(FeeError::TransferFailed);
                }
            }

            // Refund any overpayment so exactly `amount` stays in custody.
            let excess = attached - amount;
            if excess > 0 && self.env().transfer(caller, excess).is_err() {
                return Err(FeeError::TransferFailed);
            }

            let outbid = auction.current_bid;
            auction.current_bid = amount;
            auction.current_bidder = Some(caller);
            auction.escrowed_value = amount;
            self.auctions.insert(auction_id, &auction);
            self.auction_bids.insert(
                (auction_id, caller),
                &AuctionBid {
                    bidder: caller,
                    amount,
                    timestamp: now,
                },
            );
            self.env().emit_event(PremiumAuctionBid {
                auction_id,
                bidder: caller,
                amount,
                outbid_previous: outbid,
            });
            Ok(())
        }

        /// Settle auction after end_time; winner is current_bidder.
        ///
        /// The winning bid amount held in custody is paid out to the seller.
        #[ink(message)]
        pub fn settle_auction(&mut self, auction_id: u64) -> Result<(), FeeError> {
            let now = self.env().block_timestamp();
            let mut auction = self
                .auctions
                .get(auction_id)
                .ok_or(FeeError::AuctionNotFound)?;
            if auction.settled {
                return Err(FeeError::AlreadySettled);
            }
            if now < auction.end_time {
                return Err(FeeError::AuctionNotEnded);
            }
            let winner = auction.current_bidder.ok_or(FeeError::AuctionNotFound)?;
            let amount = auction.current_bid;
            // Take the escrowed bid out of custody before persisting so a
            // re-entering caller can never observe (and double-claim) it.
            let escrowed = auction.escrowed_value;
            auction.settled = true;
            auction.escrowed_value = 0;
            self.auctions.insert(auction_id, &auction);
            if escrowed > 0 && self.env().transfer(auction.seller, escrowed).is_err() {
                return Err(FeeError::TransferFailed);
            }
            // fee_paid was already added to fee_treasury at auction creation
            self.env().emit_event(PremiumAuctionSettled {
                auction_id,
                property_id: auction.property_id,
                winner,
                amount,
                timestamp: now,
            });
            Ok(())
        }

        /// Returns the premium listing auction with the given id, if it exists.
        #[ink(message)]
        pub fn get_auction(&self, auction_id: u64) -> Option<PremiumAuction> {
            self.auctions.get(auction_id)
        }

        /// Returns the total number of premium listing auctions created so far.
        ///
        /// Auction ids are assigned sequentially starting at 1, so this value is
        /// also the highest allocated auction id.
        #[ink(message)]
        pub fn get_auction_count(&self) -> u64 {
            self.auction_count
        }

        // ========== Incentives and distribution ==========

        /// Registers `account` as a fee validator eligible for reward distribution.
        ///
        /// Caller requirement: admin only (`FeeError::Unauthorized` otherwise).
        /// Idempotent: registering an already-active validator is a no-op.
        #[ink(message)]
        pub fn add_validator(&mut self, account: AccountId) -> Result<(), FeeError> {
            self.ensure_admin()?;
            if self.validators.get(account).unwrap_or(false) {
                return Ok(());
            }
            self.validators.insert(account, &true);
            self.validator_list.push(account);
            Ok(())
        }

        /// Removes `account` from the fee validator set.
        ///
        /// Caller requirement: admin only (`FeeError::Unauthorized` otherwise).
        /// Removing an address that was never registered succeeds silently.
        /// Any pending rewards for the removed validator remain claimable.
        #[ink(message)]
        pub fn remove_validator(&mut self, account: AccountId) -> Result<(), FeeError> {
            self.ensure_admin()?;
            self.validators.remove(account);
            self.validator_list.retain(|&a| a != account);
            Ok(())
        }

        /// Sets how collected fees are split between validators and the treasury.
        ///
        /// Both shares are expressed in basis points (1 bps = 0.01%, denominator
        /// 10_000). The two shares must not sum to more than 10_000 bps, otherwise
        /// `FeeError::InvalidConfig` is returned and nothing changes.
        /// Caller requirement: admin only (`FeeError::Unauthorized` otherwise).
        #[ink(message)]
        pub fn set_distribution_rates(
            &mut self,
            validator_share_bp: BasisPoints,
            treasury_share_bp: BasisPoints,
        ) -> Result<(), FeeError> {
            self.ensure_admin()?;
            if validator_share_bp
                .get()
                .saturating_add(treasury_share_bp.get())
                > BasisPoints::DENOM
            {
                return Err(FeeError::InvalidConfig);
            }
            self.validator_share_bp = validator_share_bp;
            self.treasury_share_bp = treasury_share_bp;
            Ok(())
        }

        /// Distribute accumulated fees: validator share to validators, rest to treasury.
        ///
        /// Dispersals round down (never over-pay a participant): the validator
        /// share uses floor basis-point math and the per-validator split
        /// truncates. Everything that is not paid out — the treasury share plus
        /// the integer-division remainder — stays in `fee_treasury` and is
        /// carried forward to the next distribution instead of being dropped
        /// (Issue #1120). Invariant: after a distribution,
        /// `sum(distributed to validators) + fee_treasury == fee_treasury before`.
        #[ink(message)]
        pub fn distribute_fees(&mut self) -> Result<(), FeeError> {
            self.ensure_admin()?;
            let amount = self.fee_treasury;
            if amount == 0 {
                return Ok(());
            }
            let validator_total = self.validator_share_bp.mul_floor(amount);
            let validator_list = self.validator_list.clone();
            let validator_count = validator_list.len() as u32;
            let mut paid_out = 0u128;
            if validator_count > 0 && validator_total > 0 {
                let per_validator = validator_total.saturating_div(validator_count as u128);
                paid_out = per_validator.saturating_mul(validator_count as u128);
                for acc in validator_list {
                    let current = self.pending_rewards.get(acc).unwrap_or(0);
                    self.pending_rewards
                        .insert(acc, &current.saturating_add(per_validator));
                    self.record_reward(acc, per_validator, RewardReason::ValidatorReward);
                    self.total_distributed = self.total_distributed.saturating_add(per_validator);
                    self.env().emit_event(RewardsDistributed {
                        recipient: acc,
                        amount: per_validator,
                        reason: RewardReason::ValidatorReward,
                        timestamp: self.env().block_timestamp(),
                    });
                }
            }
            // Carry the treasury share and the division remainder forward; the
            // remainder would otherwise be lost when the treasury is zeroed.
            self.fee_treasury = amount.saturating_sub(paid_out);
            Ok(())
        }

        fn record_reward(&mut self, account: AccountId, amount: u128, reason: RewardReason) {
            self.reward_record_count += 1;
            self.reward_records.insert(
                self.reward_record_count,
                &RewardRecord {
                    account,
                    amount,
                    reason,
                    timestamp: self.env().block_timestamp(),
                },
            );
        }

        /// Claim pending rewards for a participant
        #[ink(message)]
        pub fn claim_rewards(&mut self) -> Result<u128, FeeError> {
            let caller = self.env().caller();
            let amount = self.pending_rewards.get(caller).unwrap_or(0);
            if amount == 0 {
                return Ok(0);
            }
            self.pending_rewards.remove(caller);
            self.env().emit_event(RewardsDistributed {
                recipient: caller,
                amount,
                reason: RewardReason::ValidatorReward,
                timestamp: self.env().block_timestamp(),
            });
            Ok(amount)
        }

        /// Returns the reward amount currently claimable by `account`.
        ///
        /// Balances accrue via `distribute_fees` (validator share) and are
        /// claimed with `claim_rewards`; accounts with no pending rewards
        /// report 0.
        #[ink(message)]
        pub fn pending_reward(&self, account: AccountId) -> u128 {
            self.pending_rewards.get(account).unwrap_or(0)
        }

        // ========== Market-based price discovery & transparency ==========

        /// Recommended fee for an operation (market-based price discovery)
        #[ink(message)]
        pub fn get_recommended_fee(&self, operation: FeeOperation) -> u128 {
            self.calculate_fee(operation)
        }

        /// Fee estimate with optimization recommendation
        #[ink(message)]
        pub fn get_fee_estimate(&self, operation: FeeOperation) -> FeeEstimate {
            let config = self.get_config(operation);
            let congestion = self.congestion_index();
            let demand_bp = self.demand_factor_bp();
            let context = FeeContext {
                congestion_index: congestion,
                demand_factor_bp: demand_bp,
                operation,
            };
            let estimated = FeeCalculator::calculate(&config, &context);
            let congestion_level = if congestion < 33 {
                "low"
            } else if congestion < 66 {
                "medium"
            } else {
                "high"
            };
            let recommendation = if congestion >= 70 {
                "Consider batching operations or submitting during off-peak."
            } else if congestion < 30 {
                "Good time to submit; fees are below average."
            } else {
                "Fees are at typical levels."
            };
            FeeEstimate {
                operation,
                estimated_fee: estimated,
                min_fee: config.min_fee,
                max_fee: config.max_fee,
                congestion_level: congestion_level.into(),
                recommendation: recommendation.into(),
            }
        }

        /// Full fee report for transparency and dashboard
        #[ink(message)]
        pub fn get_fee_report(&self) -> FeeReport {
            let now = self.env().block_timestamp();
            let recommended = self.calculate_fee(FeeOperation::RegisterProperty);
            let mut active_auctions = 0u32;
            for id in 1..=self.auction_count {
                if let Some(a) = self.auctions.get(id) {
                    if !a.settled && now < a.end_time {
                        active_auctions += 1;
                    }
                }
            }
            FeeReport {
                config: self.default_config.clone(),
                congestion_index: self.congestion_index(),
                recommended_fee: recommended,
                total_fees_collected: self.total_fees_collected,
                total_distributed: self.total_distributed,
                operation_count_24h: self.recent_ops_count as u64,
                premium_auctions_active: active_auctions,
                timestamp: now,
            }
        }

        /// Fee optimization recommendations for users
        #[ink(message)]
        pub fn get_fee_recommendations(&self) -> Vec<String> {
            let mut rec = Vec::new();
            let c = self.congestion_index();
            if c >= 70 {
                rec.push("High congestion: use batch operations to reduce total fee.".into());
                rec.push("Consider submitting during off-peak hours.".into());
            } else if c < 30 {
                rec.push("Low congestion: current fees are favorable.".into());
            }
            rec.push("Premium listings: use auctions for better price discovery.".into());
            rec.push("Check get_fee_estimate before each operation type.".into());
            rec
        }

        /// Returns the admin account configured at deployment.
        ///
        /// The admin is the sole caller allowed to change fee parameters,
        /// validator membership, and distribution rates.
        #[ink(message)]
        pub fn admin(&self) -> AccountId {
            self.admin
        }

        /// Returns the fixed `FeeConfig` (base/min/max fee in planck units)
        /// captured at construction time.
        ///
        /// This is the immutable baseline; live parameters are reflected in
        /// `get_fee_report` instead.
        #[ink(message)]
        pub fn default_config(&self) -> FeeConfig {
            self.default_config.clone()
        }

        /// Returns the current unallocated treasury balance available for
        /// distribution.
        ///
        /// Funds enter via `record_fee_collected` and leave when the admin
        /// calls `distribute_fees`.
        #[ink(message)]
        pub fn fee_treasury(&self) -> u128 {
            self.fee_treasury
        }

        // ========== Dynamic fee model (Issue #508) ==========

        /// Return the current effective fee rate in basis points, computed
        /// from the stored `DynamicFeeConfig` and the live utilisation index.
        ///
        /// Formula:
        ///   utilisation  = congestion_index()          (0 – 100)
        ///   multiplier   = 100 + utilisation × (congestion_multiplier − 100) / 100
        ///   effective    = base_fee_bps × multiplier / 100
        ///   effective    = min(effective, max_fee_bps)
        #[ink(message)]
        pub fn get_current_fee_rate(&self) -> u32 {
            Self::compute_fee_rate(&self.dynamic_fee_config, self.congestion_index())
        }

        /// Update the dynamic fee configuration (admin only).
        /// Emits `FeeRateUpdated` with the old and new effective rates.
        #[ink(message)]
        pub fn set_dynamic_fee_config(&mut self, config: DynamicFeeConfig) -> Result<(), FeeError> {
            self.ensure_admin()?;
            if config.base_fee_bps > config.max_fee_bps {
                return Err(FeeError::InvalidConfig);
            }
            if config.congestion_multiplier < 100 {
                // Multiplier below 100 % would make fees decrease with load —
                // not a valid congestion model.
                return Err(FeeError::InvalidConfig);
            }
            let utilisation = self.congestion_index();
            let old_rate = Self::compute_fee_rate(&self.dynamic_fee_config, utilisation);
            let new_rate = Self::compute_fee_rate(&config, utilisation);
            self.dynamic_fee_config = config;
            let now = self.env().block_timestamp();
            self.env().emit_event(FeeRateUpdated {
                by: self.env().caller(),
                old_rate_bps: BasisPoints::new(old_rate),
                new_rate_bps: BasisPoints::new(new_rate),
                timestamp: now,
            });
            Ok(())
        }

        /// Return a copy of the current `DynamicFeeConfig`.
        #[ink(message)]
        pub fn dynamic_fee_config(&self) -> DynamicFeeConfig {
            self.dynamic_fee_config.clone()
        }

        /// Pure helper: compute effective fee rate (bps) for a given config
        /// and utilisation index (0-100).
        fn compute_fee_rate(config: &DynamicFeeConfig, utilisation: u32) -> u32 {
            // multiplier_pct is 100 at 0 % util and congestion_multiplier at 100 % util.
            let util = utilisation.min(100) as u64;
            let base = config.base_fee_bps.get() as u64;
            let cm = config.congestion_multiplier as u64;
            // effective = base * (100 + util * (cm - 100) / 100) / 100
            let multiplier_pct = 100u64.saturating_add(
                util.saturating_mul(cm.saturating_sub(100))
                    .saturating_div(100),
            );
            let effective = base.saturating_mul(multiplier_pct).saturating_div(100);
            (effective as u32).min(config.max_fee_bps.get())
        }
    }

    impl DynamicFeeProvider for FeeManager {
        /// Recommended fee for `operation` under the dynamic fee model.
        ///
        /// Delegates to `calculate_fee`, which applies the configured base fee
        /// (bps), congestion multiplier, and the operation's max-fee cap (bps,
        /// denominator 10_000 in both cases). Read-only; any caller may query it.
        #[ink(message)]
        fn get_recommended_fee(&self, operation: FeeOperation) -> u128 {
            self.calculate_fee(operation)
        }
    }

    // Unit-test module lives in `tests.rs` (mirrors the bridge contract's
    // `include!("tests.rs")` pattern). Named `fee_tests` because the
    // `include!("errors.rs")` above already contributes a sibling `mod tests`.
    #[cfg(test)]
    include!("tests.rs");
}
