#![cfg_attr(not(feature = "std"), no_std)]
#![allow(unexpected_cfgs)]

use ink::prelude::string::String;
use ink::prelude::vec::Vec;
use ink::storage::Mapping;
use propchain_traits::DynamicFeeProvider;
use propchain_traits::FeeOperation;
use propchain_traits::*;

/// Dynamic Fee and Market Mechanism contract for PropChain.
/// Implements congestion-based fees, premium listing auctions, validator incentives,
/// and fee transparency for network participants.
#[ink::contract]
mod propchain_fees {
    use super::*;

    /// Basis points denominator (10000 = 100%)
    const BASIS_POINTS: u128 = 10_000;

    /// Default congestion window: number of recent operations to consider
    const CONGESTION_WINDOW: u32 = 100;
    /// Max fee multiplier from congestion (e.g. 3x base)
    const MAX_CONGESTION_MULTIPLIER: u32 = 300; // 300% of base

    include!("types.rs");
    include!("errors.rs");
    include!("strategies.rs");

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
        validator_share_bp: u32,
        /// Distribution rate for treasury (rest)
        treasury_share_bp: u32,
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
                demand_factor_bp: 500,
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
                validator_share_bp: 5000, // 50% to validators
                treasury_share_bp: 5000,  // 50% to treasury
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

        /// Compute current congestion index (0-100) from recent activity
        fn congestion_index(&self) -> u32 {
            let now = self.env().block_timestamp();
            let window_secs = 3600u64; // 1 hour window
            if now.saturating_sub(self.last_congestion_reset) > window_secs {
                return 0; // Reset after window
            }
            let count = self.recent_ops_count;
            // Normalize to 0-100: CONGESTION_WINDOW ops = 100
            (count.saturating_mul(100).saturating_div(CONGESTION_WINDOW)).min(100)
        }

        /// Demand factor in basis points (from recent volume)
        fn demand_factor_bp(&self) -> u32 {
            let ci = self.congestion_index();
            self.default_config
                .demand_factor_bp
                .saturating_mul(ci)
                .saturating_div(100)
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
            FeeCalculator::calculate(&config, &context)
        }

        /// Record that a fee was collected (called by registry or self after charging)
        #[ink(message)]
        pub fn record_fee_collected(
            &mut self,
            _operation: FeeOperation,
            amount: u128,
            from: AccountId,
        ) -> Result<(), FeeError> {
            let _ = from;
            self.recent_ops_count = self
                .recent_ops_count
                .saturating_add(1)
                .min(CONGESTION_WINDOW);
            let now = self.env().block_timestamp();
            if now.saturating_sub(self.last_congestion_reset) > 3600 {
                self.last_congestion_reset = now;
                self.recent_ops_count = 1;
            }
            self.fee_treasury = self.fee_treasury.saturating_add(amount);
            self.total_fees_collected = self.total_fees_collected.saturating_add(amount);
            Ok(())
        }

        // ========== Automated fee adjustment ==========

        /// Automated fee adjustment based on recent utilization vs target
        #[ink(message)]
        pub fn update_fee_params(&mut self) -> Result<(), FeeError> {
            self.ensure_admin()?;
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

        /// Place or increase bid (bid must be > current_bid and >= min_bid)
        #[ink(message)]
        pub fn place_bid(&mut self, auction_id: u64, amount: u128) -> Result<(), FeeError> {
            let caller = self.env().caller();
            let now = self.env().block_timestamp();
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
            let outbid = auction.current_bid;
            auction.current_bid = amount;
            auction.current_bidder = Some(caller);
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

        /// Settle auction after end_time; winner is current_bidder
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
            auction.settled = true;
            self.auctions.insert(auction_id, &auction);
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

        #[ink(message)]
        pub fn get_auction(&self, auction_id: u64) -> Option<PremiumAuction> {
            self.auctions.get(auction_id)
        }

        #[ink(message)]
        pub fn get_auction_count(&self) -> u64 {
            self.auction_count
        }

        // ========== Incentives and distribution ==========

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

        #[ink(message)]
        pub fn remove_validator(&mut self, account: AccountId) -> Result<(), FeeError> {
            self.ensure_admin()?;
            self.validators.remove(account);
            self.validator_list.retain(|&a| a != account);
            Ok(())
        }

        #[ink(message)]
        pub fn set_distribution_rates(
            &mut self,
            validator_share_bp: u32,
            treasury_share_bp: u32,
        ) -> Result<(), FeeError> {
            self.ensure_admin()?;
            if validator_share_bp.saturating_add(treasury_share_bp) > 10_000 {
                return Err(FeeError::InvalidConfig);
            }
            self.validator_share_bp = validator_share_bp;
            self.treasury_share_bp = treasury_share_bp;
            Ok(())
        }

        /// Distribute accumulated fees: validator share to validators, rest to treasury
        #[ink(message)]
        pub fn distribute_fees(&mut self) -> Result<(), FeeError> {
            self.ensure_admin()?;
            let amount = self.fee_treasury;
            if amount == 0 {
                return Ok(());
            }
            let validator_total = amount
                .saturating_mul(self.validator_share_bp as u128)
                .saturating_div(BASIS_POINTS);
            let validator_list = self.validator_list.clone();
            let validator_count = validator_list.len() as u32;
            if validator_count > 0 && validator_total > 0 {
                let per_validator = validator_total.saturating_div(validator_count as u128);
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
            self.fee_treasury = 0;
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

        #[ink(message)]
        pub fn admin(&self) -> AccountId {
            self.admin
        }

        #[ink(message)]
        pub fn default_config(&self) -> FeeConfig {
            self.default_config.clone()
        }

        #[ink(message)]
        pub fn fee_treasury(&self) -> u128 {
            self.fee_treasury
        }
    }

    impl DynamicFeeProvider for FeeManager {
        #[ink(message)]
        fn get_recommended_fee(&self, operation: FeeOperation) -> u128 {
            self.calculate_fee(operation)
        }
    }
}
