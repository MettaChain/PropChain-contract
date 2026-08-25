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

    #[derive(Debug, Clone, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(
        feature = "std",
        derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
    )]
    pub enum MarketStatus {
        Active,
        Resolved,
        Cancelled,
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
            }
        }

        #[ink(message)]
        pub fn set_oracle(&mut self, oracle: AccountId) -> Result<(), Error> {
            self.ensure_admin()?;
            self.oracle_address = Some(oracle);
            Ok(())
        }

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

            market.status = MarketStatus::Resolved;
            market.resolved_value = Some(resolved_value);
            market.winning_direction = Some(winning_direction.clone());

            self.markets.insert(&market_id, &market);

            self.env().emit_event(MarketResolved {
                market_id,
                resolved_value,
                winning_direction,
            });

            Ok(())
        }

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

        #[ink(message)]
        pub fn get_user_reputation(&self, user: AccountId) -> UserReputation {
            self.reputations.get(&user).unwrap_or(UserReputation {
                total_predictions: 0,
                successful_predictions: 0,
                accuracy_score: 0,
            })
        }

        #[ink(message)]
        pub fn get_market(&self, market_id: u64) -> Option<PredictionMarketInfo> {
            self.markets.get(&market_id)
        }

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

            let market = contract.get_market(market_id).unwrap();
            assert_eq!(market.status, MarketStatus::Resolved);
            assert_eq!(market.winning_direction, Some(PredictionDirection::Long));
            assert_eq!(market.resolved_value, Some(500_000));
            assert_eq!(market.total_long, 1_000);
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

            // Events so far: MarketCreated + 2xPredictionStaked + MarketResolved + RewardClaimed
            let events = ink::env::test::recorded_events().collect::<Vec<_>>();
            assert_eq!(events.len(), 5);
            let claimed = RewardClaimed::decode(&mut &events[4].data[..]).expect("decode event");
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
    }
}
