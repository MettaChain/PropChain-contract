#![cfg_attr(not(feature = "std"), no_std, no_main)]
#![allow(clippy::new_without_default, clippy::needless_borrows_for_generic_args)]

#[ink::contract]
mod propchain_prediction_market {
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
    }
}
