#![allow(clippy::clone_on_copy)] // fires inside ink! generated storage code
#![cfg_attr(not(feature = "std"), no_std, no_main)]
#![allow(
    clippy::new_without_default,
    clippy::needless_borrows_for_generic_args,
    clippy::manual_checked_ops
)]

#[ink::contract]
pub mod propchain_prediction_market {
    use ink::storage::Mapping;
    use propchain_contracts::{non_reentrant, ReentrancyError, ReentrancyGuard};

    /// How long a proposed manual resolution stays challengeable, in
    /// block-timestamp seconds (Issue #1148).
    pub const DISPUTE_WINDOW: u64 = 24 * 60 * 60;

    /// A challenge must bond at least `1 / CHALLENGE_BOND_DIVISOR` of the
    /// market's total pool, so a challenge is cheap on an empty market and
    /// expensive on a contested one.
    pub const CHALLENGE_BOND_DIVISOR: u128 = 100;

    #[derive(Debug, Clone, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(
        feature = "std",
        derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
    )]
    pub enum MarketStatus {
        Active,
        Resolved,
        Cancelled,
        /// A resolution has been proposed and the dispute window is still open.
        /// Payouts are locked until the window elapses and the resolution is
        /// finalized (Issue #1148).
        PendingResolution,
    }

    #[derive(Debug, Clone, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(
        feature = "std",
        derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
    )]
    pub enum PredictionDirection {
        Long,  // Predicting value will be >= target_value
        Short, // Predicting value will be < target_value
    }

    #[derive(Debug, Clone, PartialEq, scale::Encode, scale::Decode)]
    #[cfg_attr(
        feature = "std",
        derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
    )]
    pub struct PredictionMarketInfo {
        pub market_id: u64,
        pub property_id: u64,
        pub target_value: u128,
        pub resolution_time: u64,
        pub total_long: u128,
        pub total_short: u128,
        pub status: MarketStatus,
        pub winning_direction: Option<PredictionDirection>,
        pub resolved_value: Option<u128>,
    }

    #[derive(Debug, Clone, PartialEq, scale::Encode, scale::Decode)]
    #[cfg_attr(
        feature = "std",
        derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
    )]
    pub struct Stake {
        pub amount: u128,
        pub direction: PredictionDirection,
        pub claimed: bool,
    }

    #[derive(Debug, Clone, PartialEq, scale::Encode, scale::Decode)]
    #[cfg_attr(
        feature = "std",
        derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
    )]
    pub struct UserReputation {
        pub total_predictions: u32,
        pub successful_predictions: u32,
        pub accuracy_score: u32, // out of 10000 (e.g. 7500 = 75%)
    }

    /// On-chain metric identifier used by oracle markets (e.g. "property.valuation").
    pub type OracleMetric = String;

    /// An oracle-driven market that resolves automatically when the oracle
    /// submits a data reading for the market's metric.
    #[derive(Debug, Clone, PartialEq, scale::Encode, scale::Decode)]
    #[cfg_attr(
        feature = "std",
        derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
    )]
    pub struct OracleMarket {
        pub market_id: u64,
        pub property_id: u64,
        pub metric: OracleMetric,
        /// Value the oracle reading must meet or exceed for Long to win.
        pub threshold: u128,
        /// Block timestamp after which oracle data is accepted.
        pub resolution_time: u64,
        pub resolved: bool,
        pub winning_direction: Option<PredictionDirection>,
        pub resolved_oracle_value: Option<u128>,
        pub total_long: u128,
        pub total_short: u128,
    }

    #[ink(storage)]
    pub struct PredictionMarket {
        admin: AccountId,
        markets: Mapping<u64, PredictionMarketInfo>,
        market_count: u64,

        // market_id -> (user -> Stake)
        stakes: Mapping<(u64, AccountId), Stake>,

        // user -> UserReputation
        reputations: Mapping<AccountId, UserReputation>,

        // Oracle for resolution (simplified)
        oracle_address: Option<AccountId>,

        // Protocol fee basis points
        fee_bips: u32,

        // Reentrancy protection
        reentrancy_guard: ReentrancyGuard,

        // Oracle markets (separate from manual-resolution markets)
        oracle_markets: Mapping<u64, OracleMarket>,
        oracle_market_count: u64,

        // oracle_market_id -> (user -> Stake)
        oracle_stakes: Mapping<(u64, AccountId), Stake>,

        // market_id -> last block timestamp at which a pending resolution can
        // still be challenged. Zero/absent when no resolution is pending.
        dispute_deadline: Mapping<u64, u64>,
    }

    #[ink(event)]
    pub struct MarketCreated {
        #[ink(topic)]
        market_id: u64,
        #[ink(topic)]
        property_id: u64,
        target_value: u128,
        resolution_time: u64,
    }

    #[ink(event)]
    pub struct PredictionStaked {
        #[ink(topic)]
        market_id: u64,
        #[ink(topic)]
        user: AccountId,
        amount: u128,
        direction: PredictionDirection,
    }

    #[ink(event)]
    pub struct MarketResolved {
        #[ink(topic)]
        market_id: u64,
        resolved_value: u128,
        winning_direction: PredictionDirection,
    }

    /// Emitted when the admin proposes a resolution and the dispute window
    /// opens. Payouts stay locked until this market is finalized.
    #[ink(event)]
    pub struct MarketResolutionProposed {
        #[ink(topic)]
        market_id: u64,
        resolved_value: u128,
        winning_direction: PredictionDirection,
        /// Last block timestamp at which the resolution can still be challenged
        dispute_deadline: u64,
    }

    /// Emitted when a staker bonds a challenge that reverts a pending
    /// resolution back to `Active`, returning the market to the resolver.
    #[ink(event)]
    pub struct ResolutionChallenged {
        #[ink(topic)]
        market_id: u64,
        #[ink(topic)]
        challenger: AccountId,
        /// Bond attached to the challenge; it stays in the contract balance
        bond: u128,
    }

    #[ink(event)]
    pub struct RewardClaimed {
        #[ink(topic)]
        market_id: u64,
        #[ink(topic)]
        user: AccountId,
        amount: u128,
    }

    #[ink(event)]
    pub struct BacktestValidated {
        #[ink(topic)]
        market_id: u64,
        historical_accuracy: u32,
        model_version: String,
    }

    #[ink(event)]
    pub struct OracleMarketCreated {
        #[ink(topic)]
        market_id: u64,
        #[ink(topic)]
        property_id: u64,
        metric: OracleMetric,
        threshold: u128,
        resolution_time: u64,
    }

    #[ink(event)]
    pub struct OracleMarketResolved {
        #[ink(topic)]
        market_id: u64,
        oracle_value: u128,
        winning_direction: PredictionDirection,
    }

    #[ink(event)]
    pub struct OracleWinningsClaimed {
        #[ink(topic)]
        market_id: u64,
        #[ink(topic)]
        user: AccountId,
        amount: u128,
    }

    #[derive(Debug, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub enum Error {
        Unauthorized,
        MarketNotFound,
        MarketNotActive,
        MarketNotReadyForResolution,
        MarketAlreadyResolved,
        StakeNotFound,
        RewardAlreadyClaimed,
        InvalidAmount,
        OracleNotSet,
        TransferFailed,
        LoserCannotClaim,
        ReentrantCall,
        OracleMarketNotFound,
        OracleMarketAlreadyResolved,
        OracleMarketNotResolved,
        OracleMarketNotReady,
        // Resolution dispute window (Issue #1148)
        MarketNotPendingResolution,
        DisputeWindowStillOpen,
        DisputeWindowClosed,
        InsufficientChallengeBond,
    }

    impl From<ReentrancyError> for Error {
        fn from(_: ReentrancyError) -> Self {
            Error::ReentrantCall
        }
    }

    impl PredictionMarket {
        #[ink(constructor)]
        pub fn new(admin: AccountId, fee_bips: u32) -> Self {
            Self {
                admin,
                markets: Mapping::default(),
                market_count: 0,
                stakes: Mapping::default(),
                reputations: Mapping::default(),
                oracle_address: None,
                fee_bips,
                reentrancy_guard: ReentrancyGuard::new(),
                oracle_markets: Mapping::default(),
                oracle_market_count: 0,
                oracle_stakes: Mapping::default(),
                dispute_deadline: Mapping::default(),
            }
        }

        /// Sets the oracle account address for this contract. Admin-only.
        ///
        /// Not payable. Note: this address is currently informational only
        /// for the manual-resolution markets (`create_market` /
        /// `resolve_market`), which are resolved directly by the admin, not
        /// by checking this value. It is used as the required caller for
        /// `submit_oracle_data` on oracle-driven markets.
        ///
        /// # Errors
        /// - `Error::Unauthorized` if the caller is not the contract admin.
        #[ink(message)]
        pub fn set_oracle(&mut self, oracle: AccountId) -> Result<(), Error> {
            self.ensure_admin()?;
            self.oracle_address = Some(oracle);
            Ok(())
        }

        /// Creates a new manual-resolution prediction market for a property
        /// metric and returns its `market_id`. Admin-only.
        ///
        /// Not payable. The market starts `Active` with zero stakes on both
        /// sides. Once `resolution_time` (a block timestamp) has passed, the
        /// admin resolves the market with `resolve_market`, comparing the
        /// submitted value against `target_value`. Emits `MarketCreated`.
        ///
        /// This is the manual-resolution counterpart to
        /// `create_oracle_market`; the two market kinds are tracked in
        /// separate id spaces and are not interchangeable in other messages.
        ///
        /// # Errors
        /// - `Error::Unauthorized` if the caller is not the contract admin.
        #[ink(message)]
        pub fn create_market(
            &mut self,
            property_id: u64,
            target_value: u128,
            resolution_time: u64,
        ) -> Result<u64, Error> {
            self.ensure_admin()?;

            let market_id = self.market_count;
            self.market_count += 1;

            let market = PredictionMarketInfo {
                market_id,
                property_id,
                target_value,
                resolution_time,
                total_long: 0,
                total_short: 0,
                status: MarketStatus::Active,
                winning_direction: None,
                resolved_value: None,
            };

            self.markets.insert(&market_id, &market);

            self.env().emit_event(MarketCreated {
                market_id,
                property_id,
                target_value,
                resolution_time,
            });

            Ok(market_id)
        }

        /// Stakes the transferred value on `direction` (Long or Short) for a
        /// manual-resolution market. Payable; the transferred value is the
        /// stake amount.
        ///
        /// Open to any caller. Repeated calls for the same `market_id` by the
        /// same caller add to their existing stake, provided the direction
        /// matches; this contract does not support hedging both directions
        /// on the same market from one account. Emits `PredictionStaked`.
        ///
        /// # Errors
        /// - `Error::InvalidAmount` if no value was transferred, or if the
        ///   caller already holds a stake on this market in the opposite
        ///   direction.
        /// - `Error::MarketNotFound` if `market_id` does not exist.
        /// - `Error::MarketNotActive` if the market is not `Active`, or if
        ///   its `resolution_time` has already passed (staking closes at
        ///   resolution time, before `resolve_market` is even called).
        #[ink(message, payable)]
        pub fn stake_prediction(
            &mut self,
            market_id: u64,
            direction: PredictionDirection,
        ) -> Result<(), Error> {
            let caller = self.env().caller();
            let amount = self.env().transferred_value();
            if amount == 0 {
                return Err(Error::InvalidAmount);
            }

            let mut market = self.markets.get(&market_id).ok_or(Error::MarketNotFound)?;

            if market.status != MarketStatus::Active {
                return Err(Error::MarketNotActive);
            }
            if self.env().block_timestamp() >= market.resolution_time {
                // Too late to predict
                return Err(Error::MarketNotActive);
            }

            // Record stake
            let key = (market_id, caller);
            let mut existing_stake = self.stakes.get(&key).unwrap_or(Stake {
                amount: 0,
                direction: direction.clone(),
                claimed: false,
            });

            // For simplicity, enforce same direction if adding stake
            if existing_stake.amount > 0 && existing_stake.direction != direction {
                // User cannot hedge in this simple version
                return Err(Error::InvalidAmount);
            }

            existing_stake.amount += amount;
            self.stakes.insert(&key, &existing_stake);

            // Update market totals
            match direction {
                PredictionDirection::Long => market.total_long += amount,
                PredictionDirection::Short => market.total_short += amount,
            }

            self.markets.insert(&market_id, &market);

            self.env().emit_event(PredictionStaked {
                market_id,
                user: caller,
                amount,
                direction,
            });

            Ok(())
        }

        /// Proposes a manual-resolution outcome for a market by admin-submitted
        /// `resolved_value`, deciding the winning direction. Admin-only.
        ///
        /// Not payable. `Long` wins if `resolved_value >= target_value`,
        /// otherwise `Short` wins. Can only be called while the market is
        /// `Active`, and only after `resolution_time` has passed.
        ///
        /// The proposal does **not** pay out. The market moves to
        /// `PendingResolution` and a dispute window of `DISPUTE_WINDOW`
        /// seconds opens; only once that window has elapsed and the
        /// resolution has been finalized do stakers get to claim. A staker can
        /// bond a challenge inside the window to revert the market to `Active`
        /// (see `challenge_resolution`), which forces the admin to resolve
        /// again. Emits `MarketResolutionProposed`; `MarketResolved` is emitted
        /// later, by `finalize_resolution`.
        ///
        /// # Screening / trust note
        /// This value is currently supplied directly by the admin account,
        /// not verified against `oracle_address` or any external data feed.
        /// Oracle-driven settlement for this market type is tracked
        /// separately; today's guarantee is only that the admin attests to
        /// `resolved_value`. Oracle-driven markets created via
        /// `create_oracle_market` are resolved differently, via
        /// `submit_oracle_data`.
        ///
        /// # Errors
        /// - `Error::Unauthorized` if the caller is not the contract admin.
        /// - `Error::MarketNotFound` if `market_id` does not exist.
        /// - `Error::MarketAlreadyResolved` if the market is not `Active`
        ///   (already resolved, pending a finalization, or cancelled).
        /// - `Error::MarketNotReadyForResolution` if `resolution_time` has
        ///   not yet passed.
        #[ink(message)]
        pub fn resolve_market(
            &mut self,
            market_id: u64,
            resolved_value: u128,
        ) -> Result<(), Error> {
            self.ensure_admin()?; // In production, this should ideally be called by the Oracle directly or query the oracle.

            let mut market = self.markets.get(&market_id).ok_or(Error::MarketNotFound)?;
            if market.status != MarketStatus::Active {
                return Err(Error::MarketAlreadyResolved);
            }
            if self.env().block_timestamp() < market.resolution_time {
                return Err(Error::MarketNotReadyForResolution);
            }

            let winning_direction = if resolved_value >= market.target_value {
                PredictionDirection::Long
            } else {
                PredictionDirection::Short
            };

            // Issue #1148: record the outcome, but lock payouts until the
            // dispute window has elapsed without a successful challenge.
            let dispute_deadline = self.env().block_timestamp() + DISPUTE_WINDOW;
            market.status = MarketStatus::PendingResolution;
            market.resolved_value = Some(resolved_value);
            market.winning_direction = Some(winning_direction.clone());

            self.markets.insert(&market_id, &market);
            self.dispute_deadline.insert(&market_id, &dispute_deadline);

            self.env().emit_event(MarketResolutionProposed {
                market_id,
                resolved_value,
                winning_direction,
                dispute_deadline,
            });

            Ok(())
        }

        /// Closes the dispute window on an uncontested resolution and opens
        /// payouts. Permissionless: anyone may push it once it is safe, and
        /// there is no reason to want to delay it.
        ///
        /// Not payable. Moves the market from `PendingResolution` to
        /// `Resolved` and emits `MarketResolved`. This is the only path to
        /// `Resolved`, which is what `claim_reward` requires — so the
        /// acceptance criterion is that payouts are gated on the window
        /// elapsing. Idempotence is not offered: a second call returns
        /// `MarketNotPendingResolution`.
        ///
        /// # Errors
        /// - `Error::MarketNotFound` if `market_id` does not exist.
        /// - `Error::MarketNotPendingResolution` if the market is not awaiting
        ///   finalization (never resolved, challenged back to `Active`, or
        ///   already finalized).
        /// - `Error::DisputeWindowStillOpen` if the dispute window has not yet
        ///   elapsed.
        #[ink(message)]
        pub fn finalize_resolution(&mut self, market_id: u64) -> Result<(), Error> {
            let mut market = self.markets.get(&market_id).ok_or(Error::MarketNotFound)?;
            if market.status != MarketStatus::PendingResolution {
                return Err(Error::MarketNotPendingResolution);
            }

            let now = self.env().block_timestamp();
            if now < self.dispute_deadline.get(&market_id).unwrap_or(0) {
                return Err(Error::DisputeWindowStillOpen);
            }

            let resolved_value = market.resolved_value.unwrap_or(0);
            let winning_direction = market
                .winning_direction
                .clone()
                .unwrap_or(PredictionDirection::Long);

            market.status = MarketStatus::Resolved;
            self.markets.insert(&market_id, &market);
            self.dispute_deadline.remove(&market_id);

            self.env().emit_event(MarketResolved {
                market_id,
                resolved_value,
                winning_direction,
            });

            Ok(())
        }

        /// Bonds a challenge to a pending resolution, reverting the market to
        /// `Active` so the admin has to resolve again.
        ///
        /// Payable: the attached value must be at least
        /// `required_challenge_bond(market_id)`. The bond stays in the
        /// contract balance — it is the price of contesting, and it is not
        /// refunded, because a successful challenge is indistinguishable on
        /// chain from a challenge that merely delays a correct resolution.
        ///
        /// Open to any caller, but only while the market is
        /// `PendingResolution` and before the deadline. Emits
        /// `ResolutionChallenged`.
        ///
        /// # Errors
        /// - `Error::MarketNotFound` if `market_id` does not exist.
        /// - `Error::MarketNotPendingResolution` if there is no pending
        ///   resolution to challenge.
        /// - `Error::DisputeWindowClosed` if the window has already elapsed
        ///   (in that case call `finalize_resolution` instead).
        /// - `Error::InsufficientChallengeBond` if the attached value is below
        ///   the required bond.
        #[ink(message, payable)]
        pub fn challenge_resolution(&mut self, market_id: u64) -> Result<(), Error> {
            let mut market = self.markets.get(&market_id).ok_or(Error::MarketNotFound)?;
            if market.status != MarketStatus::PendingResolution {
                return Err(Error::MarketNotPendingResolution);
            }

            let now = self.env().block_timestamp();
            if now >= self.dispute_deadline.get(&market_id).unwrap_or(0) {
                return Err(Error::DisputeWindowClosed);
            }

            let bond = self.env().transferred_value();
            if bond < self.required_challenge_bond(market_id) {
                return Err(Error::InsufficientChallengeBond);
            }

            market.status = MarketStatus::Active;
            market.resolved_value = None;
            market.winning_direction = None;
            self.markets.insert(&market_id, &market);
            self.dispute_deadline.remove(&market_id);

            self.env().emit_event(ResolutionChallenged {
                market_id,
                challenger: self.env().caller(),
                bond,
            });

            Ok(())
        }

        /// The bond a challenge to `market_id` must attach: one percent of the
        /// market's total pool, i.e. zero for a market nobody has staked on.
        #[ink(message)]
        pub fn required_challenge_bond(&self, market_id: u64) -> u128 {
            match self.markets.get(&market_id) {
                Some(market) => {
                    (market.total_long + market.total_short) / CHALLENGE_BOND_DIVISOR
                }
                None => 0,
            }
        }

        /// The last block timestamp at which a pending resolution on
        /// `market_id` can still be challenged. Zero when none is pending.
        #[ink(message)]
        pub fn get_dispute_deadline(&self, market_id: u64) -> u64 {
            self.dispute_deadline.get(&market_id).unwrap_or(0)
        }

        /// Claims the caller's payout from a resolved manual-resolution
        /// market, transferring it to the caller. Not payable.
        ///
        /// The market must have reached `Resolved`, which since #1148 only
        /// happens through `finalize_resolution` once the dispute window has
        /// elapsed without a successful challenge. A market awaiting
        /// finalization reports `MarketNotActive` here, so a staker cannot be
        /// paid against a resolution that is still contestable.
        ///
        /// Open to any caller who holds a stake on `market_id`. A winning
        /// stake's payout is
        /// `stake + stake * losing_pool / winning_pool`, minus a protocol
        /// fee of `fee_bips` (in basis points, set at construction). A
        /// losing stake cannot claim and instead records unsuccessful-
        /// prediction reputation for the caller (see `get_user_reputation`);
        /// a winning claim records successful-prediction reputation. Each
        /// stake can be claimed at most once. Guarded against reentrancy.
        /// Emits `RewardClaimed` on success.
        ///
        /// # Errors
        /// - `Error::MarketNotFound` if `market_id` does not exist.
        /// - `Error::MarketNotActive` if the market has not been resolved
        ///   yet.
        /// - `Error::StakeNotFound` if the caller has no stake on this
        ///   market.
        /// - `Error::RewardAlreadyClaimed` if the caller already claimed
        ///   this stake.
        /// - `Error::LoserCannotClaim` if the caller's stake was on the
        ///   losing direction.
        /// - `Error::TransferFailed` if the payout transfer fails.
        /// - `Error::ReentrantCall` if called reentrantly.
        #[ink(message)]
        pub fn claim_reward(&mut self, market_id: u64) -> Result<(), Error> {
            non_reentrant!(self, {
                let caller = self.env().caller();
                let market = self.markets.get(&market_id).ok_or(Error::MarketNotFound)?;

                if market.status != MarketStatus::Resolved {
                    return Err(Error::MarketNotActive); // Need better error naming
                }

                let winning_dir = market.winning_direction.as_ref().unwrap();

                let key = (market_id, caller);
                let mut stake = self.stakes.get(&key).ok_or(Error::StakeNotFound)?;

                if stake.claimed {
                    return Err(Error::RewardAlreadyClaimed);
                }
                if stake.direction != *winning_dir {
                    // Record bad reputation
                    self.update_reputation(caller, false);
                    return Err(Error::LoserCannotClaim);
                }

                // Calculate reward:
                let (winning_pool, losing_pool) = match winning_dir {
                    PredictionDirection::Long => (market.total_long, market.total_short),
                    PredictionDirection::Short => (market.total_short, market.total_long),
                };

                // Proportion of the winning pool
                // total_reward = user_stake + (user_stake * losing_pool) / winning_pool
                let total_reward = stake.amount + (stake.amount * losing_pool) / winning_pool;

                let fee = (total_reward * self.fee_bips as u128) / 10000;
                let final_payout = total_reward.saturating_sub(fee);

                stake.claimed = true;
                self.stakes.insert(&key, &stake);

                // Record good reputation
                self.update_reputation(caller, true);

                // Transfer payout to user
                if self.env().transfer(caller, final_payout).is_err() {
                    return Err(Error::TransferFailed);
                }

                self.env().emit_event(RewardClaimed {
                    market_id,
                    user: caller,
                    amount: final_payout,
                });

                Ok(())
            })
        }

        /// Returns `user`'s prediction reputation: total predictions made,
        /// how many resolved in the user's favor, and an accuracy score out
        /// of 10000 (e.g. `7500` = 75%).
        ///
        /// Open to any caller. Reputation is only updated by
        /// `claim_reward` (manual-resolution markets), not by
        /// `claim_winnings` (oracle markets). A user who has never claimed a
        /// manual-resolution reward gets a zeroed-out `UserReputation`
        /// rather than an error.
        #[ink(message)]
        pub fn get_user_reputation(&self, user: AccountId) -> UserReputation {
            self.reputations.get(&user).unwrap_or(UserReputation {
                total_predictions: 0,
                successful_predictions: 0,
                accuracy_score: 0,
            })
        }

        /// Returns the manual-resolution market info for `market_id`, if it
        /// exists.
        ///
        /// Open to any caller. Returns `None` if `market_id` was never
        /// created via `create_market`. For oracle-driven markets, use
        /// `get_oracle_market` instead -- the two id spaces are separate.
        #[ink(message)]
        pub fn get_market(&self, market_id: u64) -> Option<PredictionMarketInfo> {
            self.markets.get(&market_id)
        }

        /// Records a backtest-accuracy attestation for a market and emits
        /// `BacktestValidated`. Admin-only. Not payable.
        ///
        /// This message does not verify `historical_accuracy` or
        /// `model_version` against anything (no proof check, no stored
        /// mapping) -- it only accepts the admin's submitted values and
        /// emits the event for off-chain consumption. It does not affect
        /// market resolution, staking, or payouts.
        ///
        /// # Errors
        /// - `Error::Unauthorized` if the caller is not the contract admin.
        #[ink(message)]
        pub fn submit_backtest_data(
            &mut self,
            market_id: u64,
            historical_accuracy: u32,
            model_version: String,
        ) -> Result<(), Error> {
            self.ensure_admin()?;

            // In a full implementation, this could verify ZK proofs or store the backtest mapping.
            // For now we simulate accepting the validation and emitting an event.
            self.env().emit_event(BacktestValidated {
                market_id,
                historical_accuracy,
                model_version,
            });
            Ok(())
        }

        /// Creates an oracle-driven market for a property metric.
        /// Anyone may predict Long (value >= threshold) or Short (value < threshold).
        /// The oracle address resolves the market by calling `submit_oracle_data`.
        #[ink(message)]
        pub fn create_oracle_market(
            &mut self,
            property_id: u64,
            metric: OracleMetric,
            threshold: u128,
            resolution_time: u64,
        ) -> Result<u64, Error> {
            self.ensure_admin()?;

            let market_id = self.oracle_market_count;
            self.oracle_market_count += 1;

            let market = OracleMarket {
                market_id,
                property_id,
                metric: metric.clone(),
                threshold,
                resolution_time,
                resolved: false,
                winning_direction: None,
                resolved_oracle_value: None,
                total_long: 0,
                total_short: 0,
            };

            self.oracle_markets.insert(&market_id, &market);

            self.env().emit_event(OracleMarketCreated {
                market_id,
                property_id,
                metric,
                threshold,
                resolution_time,
            });

            Ok(market_id)
        }

        /// Stake a prediction on an oracle market (payable — stake = transferred value).
        #[ink(message, payable)]
        pub fn stake_oracle_market(
            &mut self,
            market_id: u64,
            direction: PredictionDirection,
        ) -> Result<(), Error> {
            let caller = self.env().caller();
            let amount = self.env().transferred_value();
            if amount == 0 {
                return Err(Error::InvalidAmount);
            }

            let mut market = self
                .oracle_markets
                .get(&market_id)
                .ok_or(Error::OracleMarketNotFound)?;

            if market.resolved {
                return Err(Error::OracleMarketAlreadyResolved);
            }
            if self.env().block_timestamp() >= market.resolution_time {
                return Err(Error::MarketNotActive);
            }

            let key = (market_id, caller);
            let mut existing = self.oracle_stakes.get(&key).unwrap_or(Stake {
                amount: 0,
                direction: direction.clone(),
                claimed: false,
            });

            if existing.amount > 0 && existing.direction != direction {
                return Err(Error::InvalidAmount);
            }

            existing.amount += amount;
            self.oracle_stakes.insert(&key, &existing);

            match direction {
                PredictionDirection::Long => market.total_long += amount,
                PredictionDirection::Short => market.total_short += amount,
            }

            self.oracle_markets.insert(&market_id, &market);

            Ok(())
        }

        /// Called by the oracle address to submit a data reading and resolve the market.
        /// Long wins when `oracle_value >= threshold`; Short wins otherwise.
        #[ink(message)]
        pub fn submit_oracle_data(
            &mut self,
            market_id: u64,
            oracle_value: u128,
        ) -> Result<(), Error> {
            let caller = self.env().caller();
            let oracle = self.oracle_address.ok_or(Error::OracleNotSet)?;
            if caller != oracle && caller != self.admin {
                return Err(Error::Unauthorized);
            }

            let mut market = self
                .oracle_markets
                .get(&market_id)
                .ok_or(Error::OracleMarketNotFound)?;

            if market.resolved {
                return Err(Error::OracleMarketAlreadyResolved);
            }
            if self.env().block_timestamp() < market.resolution_time {
                return Err(Error::OracleMarketNotReady);
            }

            let winning_direction = if oracle_value >= market.threshold {
                PredictionDirection::Long
            } else {
                PredictionDirection::Short
            };

            market.resolved = true;
            market.winning_direction = Some(winning_direction.clone());
            market.resolved_oracle_value = Some(oracle_value);

            self.oracle_markets.insert(&market_id, &market);

            self.env().emit_event(OracleMarketResolved {
                market_id,
                oracle_value,
                winning_direction,
            });

            Ok(())
        }

        /// Returns oracle market info.
        #[ink(message)]
        pub fn get_oracle_market(&self, market_id: u64) -> Option<OracleMarket> {
            self.oracle_markets.get(&market_id)
        }

        /// Claim winnings from a resolved oracle market.
        #[ink(message)]
        pub fn claim_winnings(&mut self, market_id: u64) -> Result<(), Error> {
            non_reentrant!(self, {
                let caller = self.env().caller();

                let market = self
                    .oracle_markets
                    .get(&market_id)
                    .ok_or(Error::OracleMarketNotFound)?;

                if !market.resolved {
                    return Err(Error::OracleMarketNotResolved);
                }

                let winning_dir = market.winning_direction.as_ref().unwrap();

                let key = (market_id, caller);
                let mut stake = self.oracle_stakes.get(&key).ok_or(Error::StakeNotFound)?;

                if stake.claimed {
                    return Err(Error::RewardAlreadyClaimed);
                }
                if stake.direction != *winning_dir {
                    self.update_reputation(caller, false);
                    return Err(Error::LoserCannotClaim);
                }

                let (winning_pool, losing_pool) = match winning_dir {
                    PredictionDirection::Long => (market.total_long, market.total_short),
                    PredictionDirection::Short => (market.total_short, market.total_long),
                };

                let total_reward = if winning_pool > 0 {
                    stake.amount + (stake.amount * losing_pool) / winning_pool
                } else {
                    stake.amount
                };

                let fee = (total_reward * self.fee_bips as u128) / 10000;
                let final_payout = total_reward.saturating_sub(fee);

                stake.claimed = true;
                self.oracle_stakes.insert(&key, &stake);

                self.update_reputation(caller, true);

                if self.env().transfer(caller, final_payout).is_err() {
                    return Err(Error::TransferFailed);
                }

                self.env().emit_event(OracleWinningsClaimed {
                    market_id,
                    user: caller,
                    amount: final_payout,
                });

                Ok(())
            })
        }

        fn update_reputation(&mut self, user: AccountId, success: bool) {
            let mut rep = self.get_user_reputation(user);
            // Don't count multiple claims from same market as multiple successes,
            // but for simplicity our claim logic is 1-to-1 with market right now.
            rep.total_predictions += 1;
            if success {
                rep.successful_predictions += 1;
            }
            // score out of 10000
            rep.accuracy_score =
                ((rep.successful_predictions as u64 * 10000) / rep.total_predictions as u64) as u32;
            self.reputations.insert(&user, &rep);
        }

        fn ensure_admin(&self) -> Result<(), Error> {
            if self.env().caller() != self.admin {
                return Err(Error::Unauthorized);
            }
            Ok(())
        }
    }

    #[cfg(test)]
    #[allow(unused_variables)]
    mod tests {
        use super::*;

        #[ink::test]
        fn new_works() {
            let accounts = ink::env::test::default_accounts::<ink::env::DefaultEnvironment>();
            let contract = PredictionMarket::new(accounts.alice, 100);
            assert_eq!(contract.admin, accounts.alice);
        }

        #[ink::test]
        fn market_creation_works() {
            let accounts = ink::env::test::default_accounts::<ink::env::DefaultEnvironment>();
            let mut contract = PredictionMarket::new(accounts.alice, 100);

            let market_id = contract.create_market(1, 500_000, 1000).unwrap();
            assert_eq!(market_id, 0);

            let market = contract.get_market(market_id).unwrap();
            assert_eq!(market.target_value, 500_000);
            assert_eq!(market.status, MarketStatus::Active);
        }

        // ── Oracle market tests (Issue #505) ──────────────────────────────────

        fn setup_with_oracle() -> (
            PredictionMarket,
            ink::env::test::DefaultAccounts<ink::env::DefaultEnvironment>,
        ) {
            let accounts = ink::env::test::default_accounts::<ink::env::DefaultEnvironment>();
            ink::env::test::set_caller::<ink::env::DefaultEnvironment>(accounts.alice);
            let mut contract = PredictionMarket::new(accounts.alice, 100);
            contract.set_oracle(accounts.eve).unwrap();
            (contract, accounts)
        }

        #[ink::test]
        fn oracle_market_creation_works() {
            let (mut contract, _) = setup_with_oracle();

            let market_id = contract
                .create_oracle_market(1, String::from("property.valuation"), 500_000, 9999)
                .unwrap();

            assert_eq!(market_id, 0);
            let market = contract.get_oracle_market(market_id).unwrap();
            assert_eq!(market.property_id, 1);
            assert_eq!(market.threshold, 500_000);
            assert!(!market.resolved);
        }

        #[ink::test]
        fn oracle_market_creation_requires_admin() {
            let (mut contract, accounts) = setup_with_oracle();

            ink::env::test::set_caller::<ink::env::DefaultEnvironment>(accounts.bob);
            let result =
                contract.create_oracle_market(1, String::from("property.valuation"), 500_000, 9999);
            assert_eq!(result, Err(Error::Unauthorized));
        }

        #[ink::test]
        fn get_oracle_market_returns_none_for_unknown_id() {
            let (contract, _) = setup_with_oracle();
            assert!(contract.get_oracle_market(999).is_none());
        }

        #[ink::test]
        fn oracle_market_resolves_long_when_value_above_threshold() {
            let (mut contract, accounts) = setup_with_oracle();

            let market_id = contract
                .create_oracle_market(1, String::from("property.valuation"), 500_000, 0)
                .unwrap();

            ink::env::test::set_caller::<ink::env::DefaultEnvironment>(accounts.eve);
            contract.submit_oracle_data(market_id, 600_000).unwrap();

            let market = contract.get_oracle_market(market_id).unwrap();
            assert!(market.resolved);
            assert_eq!(market.winning_direction, Some(PredictionDirection::Long));
            assert_eq!(market.resolved_oracle_value, Some(600_000));
        }

        #[ink::test]
        fn oracle_market_resolves_short_when_value_below_threshold() {
            let (mut contract, accounts) = setup_with_oracle();

            let market_id = contract
                .create_oracle_market(1, String::from("property.valuation"), 500_000, 0)
                .unwrap();

            ink::env::test::set_caller::<ink::env::DefaultEnvironment>(accounts.eve);
            contract.submit_oracle_data(market_id, 400_000).unwrap();

            let market = contract.get_oracle_market(market_id).unwrap();
            assert!(market.resolved);
            assert_eq!(market.winning_direction, Some(PredictionDirection::Short));
        }

        #[ink::test]
        fn oracle_data_cannot_be_submitted_twice() {
            let (mut contract, accounts) = setup_with_oracle();

            let market_id = contract
                .create_oracle_market(1, String::from("property.valuation"), 500_000, 0)
                .unwrap();

            ink::env::test::set_caller::<ink::env::DefaultEnvironment>(accounts.eve);
            contract.submit_oracle_data(market_id, 600_000).unwrap();

            let result = contract.submit_oracle_data(market_id, 600_000);
            assert_eq!(result, Err(Error::OracleMarketAlreadyResolved));
        }

        #[ink::test]
        fn non_oracle_cannot_submit_data() {
            let (mut contract, accounts) = setup_with_oracle();

            let market_id = contract
                .create_oracle_market(1, String::from("property.valuation"), 500_000, 0)
                .unwrap();

            ink::env::test::set_caller::<ink::env::DefaultEnvironment>(accounts.bob);
            let result = contract.submit_oracle_data(market_id, 600_000);
            assert_eq!(result, Err(Error::Unauthorized));
        }

        #[ink::test]
        fn loser_cannot_claim_oracle_winnings() {
            let (mut contract, accounts) = setup_with_oracle();

            let market_id = contract
                .create_oracle_market(1, String::from("property.valuation"), 500_000, 1_000)
                .unwrap();

            // Bob stakes Short (loses when oracle returns 600k > threshold)
            ink::env::test::set_caller::<ink::env::DefaultEnvironment>(accounts.bob);
            ink::env::test::set_value_transferred::<ink::env::DefaultEnvironment>(1_000);
            contract
                .stake_oracle_market(market_id, PredictionDirection::Short)
                .unwrap();

            // Advance past resolution_time so oracle submission is accepted
            ink::env::test::set_block_timestamp::<ink::env::DefaultEnvironment>(1_001);

            // Oracle resolves Long wins
            ink::env::test::set_caller::<ink::env::DefaultEnvironment>(accounts.eve);
            contract.submit_oracle_data(market_id, 600_000).unwrap();

            // Bob tries to claim — should fail
            ink::env::test::set_caller::<ink::env::DefaultEnvironment>(accounts.bob);
            let result = contract.claim_winnings(market_id);
            assert_eq!(result, Err(Error::LoserCannotClaim));
        }

        // ── Manual resolution & claim tests (Issue #1017) ─────────────────────

        /// Creates a manual-resolution market (target 500_000, resolution_time 1_000)
        /// with a 1% protocol fee, administered by Alice.
        fn setup_manual_market() -> (
            PredictionMarket,
            ink::env::test::DefaultAccounts<ink::env::DefaultEnvironment>,
            u64,
        ) {
            let accounts = ink::env::test::default_accounts::<ink::env::DefaultEnvironment>();
            ink::env::test::set_caller::<ink::env::DefaultEnvironment>(accounts.alice);
            let mut contract = PredictionMarket::new(accounts.alice, 100); // fee_bips = 100 (1%)
            let market_id = contract
                .create_market(1, 500_000, 1_000)
                .expect("market creation must succeed");
            (contract, accounts, market_id)
        }

        /// Sets `account` as the caller with `amount` as transferred value,
        /// ready for a payable `stake_prediction` call.
        fn set_staker(account: AccountId, amount: u128) {
            ink::env::test::set_caller::<ink::env::DefaultEnvironment>(account);
            ink::env::test::set_value_transferred::<ink::env::DefaultEnvironment>(amount);
        }

        /// Advances the chain past the dispute window and finalizes the
        /// pending resolution, which is what opens payouts (#1148).
        fn settle_market(contract: &mut PredictionMarket, market_id: u64, resolved_at: u64) {
            ink::env::test::set_block_timestamp::<ink::env::DefaultEnvironment>(
                resolved_at + DISPUTE_WINDOW,
            );
            contract
                .finalize_resolution(market_id)
                .expect("finalization must succeed once the window has elapsed");
        }

        #[ink::test]
        fn resolve_market_requires_admin() {
            let (mut contract, accounts, market_id) = setup_manual_market();

            ink::env::test::set_caller::<ink::env::DefaultEnvironment>(accounts.bob);
            let result = contract.resolve_market(market_id, 600_000);
            assert_eq!(result, Err(Error::Unauthorized));

            // Market state must be untouched by the failed resolution attempt
            let market = contract.get_market(market_id).unwrap();
            assert_eq!(market.status, MarketStatus::Active);
            assert_eq!(market.winning_direction, None);
            assert_eq!(market.resolved_value, None);
        }

        #[ink::test]
        fn resolve_market_before_deadline_rejected() {
            let (mut contract, _, market_id) = setup_manual_market();

            // Block timestamp defaults to 0, which is < resolution_time (1_000)
            let result = contract.resolve_market(market_id, 600_000);
            assert_eq!(result, Err(Error::MarketNotReadyForResolution));

            let market = contract.get_market(market_id).unwrap();
            assert_eq!(market.status, MarketStatus::Active);
        }

        #[ink::test]
        fn stake_after_resolution_deadline_rejected() {
            let (mut contract, accounts, market_id) = setup_manual_market();

            // Advance to exactly the resolution deadline: staking is closed
            ink::env::test::set_block_timestamp::<ink::env::DefaultEnvironment>(1_000);
            ink::env::test::set_caller::<ink::env::DefaultEnvironment>(accounts.bob);
            ink::env::test::set_value_transferred::<ink::env::DefaultEnvironment>(1_000);
            let result = contract.stake_prediction(market_id, PredictionDirection::Long);
            assert_eq!(result, Err(Error::MarketNotActive));
        }

        #[ink::test]
        fn resolve_market_long_wins_when_value_equals_target() {
            let (mut contract, accounts, market_id) = setup_manual_market();

            set_staker(accounts.bob, 1_000);
            contract
                .stake_prediction(market_id, PredictionDirection::Long)
                .unwrap();

            // Boundary: block_timestamp == resolution_time allows resolution
            ink::env::test::set_block_timestamp::<ink::env::DefaultEnvironment>(1_000);
            ink::env::test::set_caller::<ink::env::DefaultEnvironment>(accounts.alice);

            // Boundary: resolved_value == target_value resolves Long (>= semantics)
            contract.resolve_market(market_id, 500_000).unwrap();

            // #1148: the proposal only opens the dispute window; it does not
            // resolve. The window elapses and anyone can finalize.
            let market = contract.get_market(market_id).unwrap();
            assert_eq!(market.status, MarketStatus::PendingResolution);
            assert_eq!(market.winning_direction, Some(PredictionDirection::Long));
            assert_eq!(market.resolved_value, Some(500_000));
            assert_eq!(market.total_long, 1_000);

            ink::env::test::set_block_timestamp::<ink::env::DefaultEnvironment>(
                1_000 + DISPUTE_WINDOW,
            );
            contract.finalize_resolution(market_id).unwrap();

            let market = contract.get_market(market_id).unwrap();
            assert_eq!(market.status, MarketStatus::Resolved);
            assert_eq!(market.winning_direction, Some(PredictionDirection::Long));
            assert_eq!(market.resolved_value, Some(500_000));
        }

        #[ink::test]
        fn double_resolution_rejected() {
            let (mut contract, accounts, market_id) = setup_manual_market();

            set_staker(accounts.bob, 1_000);
            contract
                .stake_prediction(market_id, PredictionDirection::Long)
                .unwrap();
            set_staker(accounts.charlie, 1_000);
            contract
                .stake_prediction(market_id, PredictionDirection::Short)
                .unwrap();

            ink::env::test::set_block_timestamp::<ink::env::DefaultEnvironment>(1_001);
            ink::env::test::set_caller::<ink::env::DefaultEnvironment>(accounts.alice);
            contract.resolve_market(market_id, 600_000).unwrap();

            // Second resolution attempt hits the MarketAlreadyResolved guard
            let result = contract.resolve_market(market_id, 400_000);
            assert_eq!(result, Err(Error::MarketAlreadyResolved));

            // Original resolution outcome is preserved
            let market = contract.get_market(market_id).unwrap();
            assert_eq!(market.resolved_value, Some(600_000));
            assert_eq!(market.winning_direction, Some(PredictionDirection::Long));
        }

        #[ink::test]
        fn winner_claim_pays_proportional_payout_minus_fee() {
            use scale::Decode as _;

            let (mut contract, accounts, market_id) = setup_manual_market();

            // Bob stakes Long 1_000, Charlie stakes Short 3_000
            set_staker(accounts.bob, 1_000);
            contract
                .stake_prediction(market_id, PredictionDirection::Long)
                .unwrap();
            set_staker(accounts.charlie, 3_000);
            contract
                .stake_prediction(market_id, PredictionDirection::Short)
                .unwrap();

            ink::env::test::set_block_timestamp::<ink::env::DefaultEnvironment>(1_001);
            ink::env::test::set_caller::<ink::env::DefaultEnvironment>(accounts.alice);
            contract.resolve_market(market_id, 600_000).unwrap(); // Long wins
            settle_market(&mut contract, market_id, 1_001);

            // Payout math (see claim_reward):
            //   total_reward = 1_000 + (1_000 * 3_000) / 1_000 = 4_000
            //   fee          = 4_000 * 100 / 10_000           = 40    (fee_bips=100 → 1%)
            //   payout       = 4_000 - 40                     = 3_960
            let bob_before =
                ink::env::test::get_account_balance::<ink::env::DefaultEnvironment>(accounts.bob)
                    .expect("bob account must exist");

            ink::env::test::set_caller::<ink::env::DefaultEnvironment>(accounts.bob);
            contract.claim_reward(market_id).unwrap();

            let bob_after =
                ink::env::test::get_account_balance::<ink::env::DefaultEnvironment>(accounts.bob)
                    .expect("bob account must exist");
            assert_eq!(bob_after - bob_before, 3_960);

            // Winner gains a perfect reputation entry
            let rep = contract.get_user_reputation(accounts.bob);
            assert_eq!(rep.total_predictions, 1);
            assert_eq!(rep.successful_predictions, 1);
            assert_eq!(rep.accuracy_score, 10_000);

            // Events so far: MarketCreated + 2xPredictionStaked +
            // MarketResolutionProposed + MarketResolved + RewardClaimed
            let events = ink::env::test::recorded_events().collect::<Vec<_>>();
            assert_eq!(events.len(), 6);
            let claimed = RewardClaimed::decode(&mut &events[5].data[..]).expect("decode event");
            assert_eq!(claimed.market_id, market_id);
            assert_eq!(claimed.user, accounts.bob);
            assert_eq!(claimed.amount, 3_960);
        }

        #[ink::test]
        fn loser_claim_yields_nothing() {
            let (mut contract, accounts, market_id) = setup_manual_market();

            set_staker(accounts.bob, 1_000);
            contract
                .stake_prediction(market_id, PredictionDirection::Long)
                .unwrap();
            set_staker(accounts.charlie, 1_000);
            contract
                .stake_prediction(market_id, PredictionDirection::Short)
                .unwrap();

            ink::env::test::set_block_timestamp::<ink::env::DefaultEnvironment>(1_001);
            ink::env::test::set_caller::<ink::env::DefaultEnvironment>(accounts.alice);
            contract.resolve_market(market_id, 600_000).unwrap(); // Long wins
            settle_market(&mut contract, market_id, 1_001);

            let charlie_before =
                ink::env::test::get_account_balance::<ink::env::DefaultEnvironment>(
                    accounts.charlie,
                )
                .expect("charlie account must exist");

            // Losing side gets Err and no balance movement
            ink::env::test::set_caller::<ink::env::DefaultEnvironment>(accounts.charlie);
            let result = contract.claim_reward(market_id);
            assert_eq!(result, Err(Error::LoserCannotClaim));

            let charlie_after =
                ink::env::test::get_account_balance::<ink::env::DefaultEnvironment>(
                    accounts.charlie,
                )
                .expect("charlie account must exist");
            assert_eq!(charlie_after, charlie_before);

            // Failed claim is recorded as a bad prediction
            let rep = contract.get_user_reputation(accounts.charlie);
            assert_eq!(rep.total_predictions, 1);
            assert_eq!(rep.successful_predictions, 0);
            assert_eq!(rep.accuracy_score, 0);

            // Stake stays unclaimed but marked as loser-owned; retry keeps failing
            assert_eq!(
                contract.claim_reward(market_id),
                Err(Error::LoserCannotClaim)
            );
        }

        #[ink::test]
        fn double_claim_rejected() {
            let (mut contract, accounts, market_id) = setup_manual_market();

            set_staker(accounts.bob, 1_000);
            contract
                .stake_prediction(market_id, PredictionDirection::Long)
                .unwrap();
            set_staker(accounts.charlie, 1_000);
            contract
                .stake_prediction(market_id, PredictionDirection::Short)
                .unwrap();

            ink::env::test::set_block_timestamp::<ink::env::DefaultEnvironment>(1_001);
            ink::env::test::set_caller::<ink::env::DefaultEnvironment>(accounts.alice);
            contract.resolve_market(market_id, 600_000).unwrap();
            settle_market(&mut contract, market_id, 1_001);

            ink::env::test::set_caller::<ink::env::DefaultEnvironment>(accounts.bob);
            contract.claim_reward(market_id).unwrap();

            // Second claim attempt must hit the RewardAlreadyClaimed guard
            let result = contract.claim_reward(market_id);
            assert_eq!(result, Err(Error::RewardAlreadyClaimed));
        }

        #[ink::test]
        fn claim_by_non_participant_rejected() {
            let (mut contract, accounts, market_id) = setup_manual_market();

            set_staker(accounts.bob, 1_000);
            contract
                .stake_prediction(market_id, PredictionDirection::Long)
                .unwrap();

            ink::env::test::set_block_timestamp::<ink::env::DefaultEnvironment>(1_001);
            ink::env::test::set_caller::<ink::env::DefaultEnvironment>(accounts.alice);
            contract.resolve_market(market_id, 600_000).unwrap();
            settle_market(&mut contract, market_id, 1_001);

            // Frank never staked on this market
            ink::env::test::set_caller::<ink::env::DefaultEnvironment>(accounts.frank);
            let result = contract.claim_reward(market_id);
            assert_eq!(result, Err(Error::StakeNotFound));
        }

        #[ink::test]
        fn claim_before_resolution_rejected() {
            let (mut contract, accounts, market_id) = setup_manual_market();

            set_staker(accounts.bob, 1_000);
            contract
                .stake_prediction(market_id, PredictionDirection::Long)
                .unwrap();

            // Market is still Active: claiming must fail without paying out
            let bob_before =
                ink::env::test::get_account_balance::<ink::env::DefaultEnvironment>(accounts.bob)
                    .expect("bob account must exist");

            ink::env::test::set_caller::<ink::env::DefaultEnvironment>(accounts.bob);
            let result = contract.claim_reward(market_id);
            assert_eq!(result, Err(Error::MarketNotActive));

            let bob_after =
                ink::env::test::get_account_balance::<ink::env::DefaultEnvironment>(accounts.bob)
                    .expect("bob account must exist");
            assert_eq!(bob_after, bob_before);
        }

        // ---- Resolution dispute window (Issue #1148) ----

        /// A resolution is a proposal, not a settlement: the market parks in
        /// `PendingResolution`, records a dispute deadline, and no staker can
        /// be paid while the window is open.
        #[ink::test]
        fn resolution_opens_dispute_window_and_locks_payouts() {
            let (mut contract, accounts, market_id) = setup_manual_market();

            set_staker(accounts.bob, 1_000);
            contract
                .stake_prediction(market_id, PredictionDirection::Long)
                .unwrap();

            ink::env::test::set_block_timestamp::<ink::env::DefaultEnvironment>(1_001);
            ink::env::test::set_caller::<ink::env::DefaultEnvironment>(accounts.alice);
            contract.resolve_market(market_id, 600_000).unwrap();

            let market = contract.get_market(market_id).unwrap();
            assert_eq!(market.status, MarketStatus::PendingResolution);
            assert_eq!(market.resolved_value, Some(600_000));
            assert_eq!(
                contract.get_dispute_deadline(market_id),
                1_001 + DISPUTE_WINDOW,
                "the window opens one DISPUTE_WINDOW after the resolution"
            );

            // Finalizing early is refused, so payouts stay locked.
            let result = contract.finalize_resolution(market_id);
            assert_eq!(result, Err(Error::DisputeWindowStillOpen));

            // ... and a claim in the meantime cannot pay out.
            let bob_before =
                ink::env::test::get_account_balance::<ink::env::DefaultEnvironment>(accounts.bob)
                    .expect("bob account must exist");
            ink::env::test::set_caller::<ink::env::DefaultEnvironment>(accounts.bob);
            assert_eq!(
                contract.claim_reward(market_id),
                Err(Error::MarketNotActive),
                "a pending resolution must not be claimable"
            );
            assert_eq!(
                ink::env::test::get_account_balance::<ink::env::DefaultEnvironment>(accounts.bob)
                    .expect("bob account must exist"),
                bob_before,
                "no balance may move before the window elapses"
            );
        }

        /// Anyone may finalize once the window has elapsed, and the staker is
        /// then paid exactly as before.
        #[ink::test]
        fn uncontested_resolution_settles_after_the_window() {
            let (mut contract, accounts, market_id) = setup_manual_market();

            set_staker(accounts.bob, 1_000);
            contract
                .stake_prediction(market_id, PredictionDirection::Long)
                .unwrap();
            set_staker(accounts.charlie, 3_000);
            contract
                .stake_prediction(market_id, PredictionDirection::Short)
                .unwrap();

            ink::env::test::set_block_timestamp::<ink::env::DefaultEnvironment>(1_001);
            ink::env::test::set_caller::<ink::env::DefaultEnvironment>(accounts.alice);
            contract.resolve_market(market_id, 600_000).unwrap();

            // Finalize is permissionless: charlie pushes the button.
            ink::env::test::set_block_timestamp::<ink::env::DefaultEnvironment>(
                1_001 + DISPUTE_WINDOW,
            );
            ink::env::test::set_caller::<ink::env::DefaultEnvironment>(accounts.charlie);
            contract.finalize_resolution(market_id).unwrap();

            assert_eq!(
                contract.get_market(market_id).unwrap().status,
                MarketStatus::Resolved
            );
            assert_eq!(contract.get_dispute_deadline(market_id), 0);

            // Bob: 1_000 + 1_000 * 3_000 / 1_000 = 4_000 gross, less 1% fee.
            let bob_before =
                ink::env::test::get_account_balance::<ink::env::DefaultEnvironment>(accounts.bob)
                    .expect("bob account must exist");
            ink::env::test::set_caller::<ink::env::DefaultEnvironment>(accounts.bob);
            contract.claim_reward(market_id).unwrap();
            assert_eq!(
                ink::env::test::get_account_balance::<ink::env::DefaultEnvironment>(accounts.bob)
                    .expect("bob account must exist")
                    - bob_before,
                3_960
            );
        }

        /// A bonded challenge inside the window reverts the market to `Active`,
        /// which is the flip-back the issue asks for: the admin has to resolve
        /// again, and a corrected resolution can then be proposed and settled.
        #[ink::test]
        fn challenge_flips_pending_resolution_back_to_active() {
            let (mut contract, accounts, market_id) = setup_manual_market();

            set_staker(accounts.bob, 1_000);
            contract
                .stake_prediction(market_id, PredictionDirection::Long)
                .unwrap();
            set_staker(accounts.charlie, 1_000);
            contract
                .stake_prediction(market_id, PredictionDirection::Short)
                .unwrap();

            ink::env::test::set_block_timestamp::<ink::env::DefaultEnvironment>(1_001);
            ink::env::test::set_caller::<ink::env::DefaultEnvironment>(accounts.alice);
            contract.resolve_market(market_id, 600_000).unwrap();

            // Pool is 2_000, so the bond is 2_000 / 100 = 20.
            assert_eq!(contract.required_challenge_bond(market_id), 20);

            set_staker(accounts.bob, 20);
            contract.challenge_resolution(market_id).unwrap();

            let market = contract.get_market(market_id).unwrap();
            assert_eq!(
                market.status,
                MarketStatus::Active,
                "a successful challenge must return the market to Active"
            );
            assert_eq!(
                market.resolved_value, None,
                "the contested value must be cleared"
            );
            assert_eq!(market.winning_direction, None);
            assert_eq!(contract.get_dispute_deadline(market_id), 0);

            // The stale proposal cannot be finalized.
            ink::env::test::set_block_timestamp::<ink::env::DefaultEnvironment>(
                1_001 + DISPUTE_WINDOW,
            );
            assert_eq!(
                contract.finalize_resolution(market_id),
                Err(Error::MarketNotPendingResolution)
            );

            // And nobody was paid on the contested outcome.
            let bob_before =
                ink::env::test::get_account_balance::<ink::env::DefaultEnvironment>(accounts.bob)
                    .expect("bob account must exist");
            ink::env::test::set_caller::<ink::env::DefaultEnvironment>(accounts.bob);
            assert_eq!(contract.claim_reward(market_id), Err(Error::MarketNotActive));
            assert_eq!(
                ink::env::test::get_account_balance::<ink::env::DefaultEnvironment>(accounts.bob)
                    .expect("bob account must exist")
                    - bob_before,
                0,
                "the challenge bond is spent, but no payout follows from it"
            );

            // The admin resolves again with the corrected value; Short wins.
            ink::env::test::set_caller::<ink::env::DefaultEnvironment>(accounts.alice);
            contract.resolve_market(market_id, 400_000).unwrap();
            assert_eq!(
                contract.get_market(market_id).unwrap().status,
                MarketStatus::PendingResolution
            );
            settle_market(&mut contract, market_id, 1_001 + DISPUTE_WINDOW);

            let market = contract.get_market(market_id).unwrap();
            assert_eq!(market.status, MarketStatus::Resolved);
            assert_eq!(market.resolved_value, Some(400_000));
            assert_eq!(market.winning_direction, Some(PredictionDirection::Short));
        }

        /// The bond scales with the pool, and an under-bonded challenge is
        /// refused without disturbing the pending resolution.
        #[ink::test]
        fn challenge_bond_must_cover_one_percent_of_the_pool() {
            let (mut contract, accounts, market_id) = setup_manual_market();

            set_staker(accounts.bob, 5_000);
            contract
                .stake_prediction(market_id, PredictionDirection::Long)
                .unwrap();

            ink::env::test::set_block_timestamp::<ink::env::DefaultEnvironment>(1_001);
            ink::env::test::set_caller::<ink::env::DefaultEnvironment>(accounts.alice);
            contract.resolve_market(market_id, 600_000).unwrap();
            assert_eq!(contract.required_challenge_bond(market_id), 50);

            // One wei short of the bond.
            set_staker(accounts.bob, 49);
            assert_eq!(
                contract.challenge_resolution(market_id),
                Err(Error::InsufficientChallengeBond)
            );
            assert_eq!(
                contract.get_market(market_id).unwrap().status,
                MarketStatus::PendingResolution,
                "a refused challenge must leave the resolution alone"
            );

            // Exactly the required bond is accepted.
            set_staker(accounts.bob, 50);
            contract.challenge_resolution(market_id).unwrap();
            assert_eq!(
                contract.get_market(market_id).unwrap().status,
                MarketStatus::Active
            );
        }

        /// An empty market has a zero bond: there is nothing to protect, so
        /// anyone may contest a resolution nobody staked on.
        #[ink::test]
        fn challenge_on_unstaked_market_needs_no_bond() {
            let (mut contract, accounts, market_id) = setup_manual_market();

            assert_eq!(contract.required_challenge_bond(market_id), 0);

            ink::env::test::set_block_timestamp::<ink::env::DefaultEnvironment>(1_001);
            ink::env::test::set_caller::<ink::env::DefaultEnvironment>(accounts.alice);
            contract.resolve_market(market_id, 600_000).unwrap();

            set_staker(accounts.frank, 0);
            contract.challenge_resolution(market_id).unwrap();
            assert_eq!(
                contract.get_market(market_id).unwrap().status,
                MarketStatus::Active
            );
        }

        /// Once the window has closed a challenge is too late; the resolution
        /// must be finalized instead.
        #[ink::test]
        fn challenge_after_the_window_is_refused() {
            let (mut contract, accounts, market_id) = setup_manual_market();

            set_staker(accounts.bob, 1_000);
            contract
                .stake_prediction(market_id, PredictionDirection::Long)
                .unwrap();

            ink::env::test::set_block_timestamp::<ink::env::DefaultEnvironment>(1_001);
            ink::env::test::set_caller::<ink::env::DefaultEnvironment>(accounts.alice);
            contract.resolve_market(market_id, 600_000).unwrap();

            ink::env::test::set_block_timestamp::<ink::env::DefaultEnvironment>(
                1_001 + DISPUTE_WINDOW,
            );
            set_staker(accounts.bob, 1_000);
            assert_eq!(
                contract.challenge_resolution(market_id),
                Err(Error::DisputeWindowClosed)
            );

            // The boundary timestamp is still finalizable: the window is
            // inclusive of the deadline for finalization.
            contract.finalize_resolution(market_id).unwrap();
            assert_eq!(
                contract.get_market(market_id).unwrap().status,
                MarketStatus::Resolved
            );
        }

        /// Finalizing a market that was never resolved, or finalizing twice,
        /// both report the pending-resolution guard.
        #[ink::test]
        fn finalize_guards_reject_non_pending_markets() {
            let (mut contract, accounts, market_id) = setup_manual_market();

            ink::env::test::set_caller::<ink::env::DefaultEnvironment>(accounts.alice);
            assert_eq!(
                contract.finalize_resolution(market_id),
                Err(Error::MarketNotPendingResolution),
                "an Active market has nothing to finalize"
            );
            assert_eq!(
                contract.finalize_resolution(9_999),
                Err(Error::MarketNotFound),
                "an unknown market cannot be finalized"
            );

            ink::env::test::set_block_timestamp::<ink::env::DefaultEnvironment>(1_001);
            contract.resolve_market(market_id, 600_000).unwrap();
            settle_market(&mut contract, market_id, 1_001);
            assert_eq!(
                contract.finalize_resolution(market_id),
                Err(Error::MarketNotPendingResolution),
                "finalization is not idempotent"
            );
        }

        /// A market awaiting finalization takes no new stakes, so nobody can
        /// join a book that is about to settle. (`stake_prediction` refuses any
        /// non-`Active` market; the `resolution_time` guard would also fire
        /// here, and both report `MarketNotActive`.)
        #[ink::test]
        fn pending_market_rejects_new_stakes() {
            let (mut contract, accounts, market_id) = setup_manual_market();

            set_staker(accounts.bob, 1_000);
            contract
                .stake_prediction(market_id, PredictionDirection::Long)
                .unwrap();

            ink::env::test::set_block_timestamp::<ink::env::DefaultEnvironment>(1_001);
            ink::env::test::set_caller::<ink::env::DefaultEnvironment>(accounts.alice);
            contract.resolve_market(market_id, 600_000).unwrap();

            set_staker(accounts.charlie, 500);
            assert_eq!(
                contract.stake_prediction(market_id, PredictionDirection::Short),
                Err(Error::MarketNotActive)
            );
        }
    }
}
