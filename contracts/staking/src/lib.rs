#![cfg_attr(not(feature = "std"), no_std, no_main)]

#[ink::contract]
mod staking {
    use ink::prelude::vec::Vec;
    use ink::storage::Mapping;
    use propchain_traits::constants;
    use propchain_traits::errors::*;

    include!("errors.rs");
    include!("types.rs");

    impl From<propchain_traits::ReentrancyError> for Error {
        fn from(_: propchain_traits::ReentrancyError) -> Self {
            Error::ReentrantCall
        }
    }

    // =========================================================================
    // Events
    // =========================================================================

    #[ink(event)]
    pub struct Staked {
        #[ink(topic)]
        pub staker: AccountId,
        pub amount: u128,
        pub lock_period: LockPeriod,
        pub lock_until: u64,
    }

    #[ink(event)]
    pub struct Unstaked {
        #[ink(topic)]
        pub staker: AccountId,
        pub amount: u128,
    }

    #[ink(event)]
    pub struct RewardsClaimed {
        #[ink(topic)]
        pub staker: AccountId,
        pub amount: u128,
    }

    #[ink(event)]
    pub struct GovernanceDelegated {
        #[ink(topic)]
        pub staker: AccountId,
        #[ink(topic)]
        pub delegate: AccountId,
    }

    #[ink(event)]
    pub struct RewardPoolFunded {
        #[ink(topic)]
        pub funder: AccountId,
        pub amount: u128,
    }

    #[ink(event)]
    pub struct StakingConfigUpdated {
        #[ink(topic)]
        pub min_stake: u128,
        #[ink(topic)]
        pub reward_rate_bps: u128,
    }

    // =========================================================================
    // Storage
    // =========================================================================

    #[ink(storage)]
    pub struct Staking {
        admin: AccountId,
        stakes: Mapping<AccountId, StakeInfo>,
        total_staked: u128,
        reward_pool: u128,
        reward_rate_bps: u128,
        min_stake: u128,
        acc_reward_per_share: u128,
        last_reward_block: u64,
        governance_power: Mapping<AccountId, u128>,
        staker_list: Vec<AccountId>,
        reentrancy_guard: propchain_traits::ReentrancyGuard,
    }

    // =========================================================================
    // Implementation
    // =========================================================================

    impl Staking {
        /// Creates a new Staking contract.
        ///
        /// # Arguments
        /// * `reward_rate_bps` - Annual reward rate in basis points (e.g. 500 = 5%)
        /// * `min_stake` - Minimum stake amount
        #[ink(constructor)]
        pub fn new(reward_rate_bps: u128, min_stake: u128) -> Self {
            let caller = Self::env().caller();
            let safe_min = if min_stake == 0 {
                constants::STAKING_MIN_AMOUNT
            } else {
                min_stake
            };

            Self {
                admin: caller,
                stakes: Mapping::default(),
                total_staked: 0,
                reward_pool: 0,
                reward_rate_bps,
                min_stake: safe_min,
                acc_reward_per_share: 0,
                last_reward_block: 0,
                governance_power: Mapping::default(),
                staker_list: Vec::new(),
                reentrancy_guard: propchain_traits::ReentrancyGuard::new(),
            }
        }

        // ----- Queries -----

        /// Returns the stake info for an account.
        #[ink(message)]
        pub fn get_stake(&self, staker: AccountId) -> Option<StakeInfo> {
            self.stakes.get(staker)
        }

        /// Returns total amount staked across all stakers.
        #[ink(message)]
        pub fn get_total_staked(&self) -> u128 {
            self.total_staked
        }

        /// Returns the current reward pool balance.
        #[ink(message)]
        pub fn get_reward_pool(&self) -> u128 {
            self.reward_pool
        }

        /// Returns the admin address.
        #[ink(message)]
        pub fn get_admin(&self) -> AccountId {
            self.admin
        }

        /// Calculates pending rewards for a staker.
        #[ink(message)]
        pub fn get_pending_rewards(&self, staker: AccountId) -> u128 {
            if let Some(stake) = self.stakes.get(staker) {
                self.calculate_rewards(&stake)
            } else {
                0
            }
        }

        /// Returns the governance power for an account (own + delegated).
        #[ink(message)]
        pub fn get_governance_power(&self, account: AccountId) -> u128 {
            self.governance_power.get(account).unwrap_or(0)
        }

        /// Returns the minimum stake amount.
        #[ink(message)]
        pub fn get_min_stake(&self) -> u128 {
            self.min_stake
        }

        // ----- Mutations -----

        /// Stake tokens with a chosen lock period.
        #[ink(message)]
        pub fn stake(&mut self, amount: u128, lock_period: LockPeriod) -> Result<(), Error> {
            let caller = self.env().caller();

            if amount == 0 {
                return Err(Error::ZeroAmount);
            }
            if amount < self.min_stake {
                return Err(Error::InsufficientAmount);
            }
            if self.stakes.contains(caller) {
                return Err(Error::AlreadyStaked);
            }

            let now = self.env().block_number() as u64;
            let lock_until = now.saturating_add(lock_period.duration_blocks());

            let stake_info = StakeInfo {
                staker: caller,
                amount,
                staked_at: now,
                lock_until,
                lock_period,
                reward_debt: self.acc_reward_per_share,
                governance_delegate: None,
            };

            self.stakes.insert(caller, &stake_info);
            self.total_staked = self.total_staked.saturating_add(amount);
            self.staker_list.push(caller);

            // Grant governance power to self by default
            let current_power = self.governance_power.get(caller).unwrap_or(0);
            self.governance_power
                .insert(caller, &current_power.saturating_add(amount));

            self.env().emit_event(Staked {
                staker: caller,
                amount,
                lock_period,
                lock_until,
            });

            Ok(())
        }

        /// Unstake tokens. Fails if the lock period is still active.
        #[ink(message)]
        pub fn unstake(&mut self) -> Result<(), Error> {
            propchain_traits::non_reentrant!(self, {
                let caller = self.env().caller();
                let stake = self.stakes.get(caller).ok_or(Error::StakeNotFound)?;

                let now = self.env().block_number() as u64;
                if now < stake.lock_until {
                    return Err(Error::LockActive);
                }

                let amount = stake.amount;

                // Remove governance power
                self.remove_governance_power(&stake);

                self.stakes.remove(caller);
                self.total_staked = self.total_staked.saturating_sub(amount);

                // Remove from staker list
                if let Some(pos) = self.staker_list.iter().position(|s| *s == caller) {
                    self.staker_list.swap_remove(pos);
                }

                self.env().emit_event(Unstaked {
                    staker: caller,
                    amount,
                });

                Ok(())
            })
        }

        /// Claim accumulated rewards.
        #[ink(message)]
        pub fn claim_rewards(&mut self) -> Result<u128, Error> {
            propchain_traits::non_reentrant!(self, {
                let caller = self.env().caller();
                let mut stake = self.stakes.get(caller).ok_or(Error::StakeNotFound)?;

                let rewards = self.calculate_rewards(&stake);
                if rewards == 0 {
                    return Err(Error::NoRewards);
                }
                if rewards > self.reward_pool {
                    return Err(Error::InsufficientPool);
                }

                self.reward_pool = self.reward_pool.saturating_sub(rewards);
                stake.reward_debt = self.acc_reward_per_share;
                self.stakes.insert(caller, &stake);

                self.env().emit_event(RewardsClaimed {
                    staker: caller,
                    amount: rewards,
                });

                Ok(rewards)
            })
        }

        /// Delegate governance power to another address.
        #[ink(message)]
        pub fn delegate_governance(&mut self, delegate: AccountId) -> Result<(), Error> {
            let caller = self.env().caller();
            let mut stake = self.stakes.get(caller).ok_or(Error::StakeNotFound)?;

            if delegate == caller {
                return Err(Error::InvalidDelegate);
            }

            // Remove old delegation
            self.remove_governance_power(&stake);

            // Set new delegate
            stake.governance_delegate = Some(delegate);
            self.stakes.insert(caller, &stake);

            // Grant power to delegate
            let delegate_power = self.governance_power.get(delegate).unwrap_or(0);
            self.governance_power
                .insert(delegate, &delegate_power.saturating_add(stake.amount));

            self.env().emit_event(GovernanceDelegated {
                staker: caller,
                delegate,
            });

            Ok(())
        }

        /// Fund the reward pool. Only admin may call.
        #[ink(message)]
        pub fn fund_reward_pool(&mut self, amount: u128) -> Result<(), Error> {
            self.ensure_admin()?;

            if amount == 0 {
                return Err(Error::ZeroAmount);
            }

            self.reward_pool = self.reward_pool.saturating_add(amount);

            self.env().emit_event(RewardPoolFunded {
                funder: self.env().caller(),
                amount,
            });

            Ok(())
        }

        /// Update staking configuration. Only admin may call.
        #[ink(message)]
        pub fn update_config(
            &mut self,
            min_stake: u128,
            reward_rate_bps: u128,
        ) -> Result<(), Error> {
            self.ensure_admin()?;

            if min_stake == 0 {
                return Err(Error::InvalidConfig);
            }

            self.min_stake = min_stake;
            self.reward_rate_bps = reward_rate_bps;

            self.env().emit_event(StakingConfigUpdated {
                min_stake,
                reward_rate_bps,
            });

            Ok(())
        }

        // ----- Internal helpers -----

        fn ensure_admin(&self) -> Result<(), Error> {
            if self.env().caller() != self.admin {
                return Err(Error::Unauthorized);
            }
            Ok(())
        }

        fn calculate_rewards(&self, stake: &StakeInfo) -> u128 {
            let now = self.env().block_number() as u64;
            let blocks_staked = now.saturating_sub(stake.staked_at) as u128;
            if blocks_staked == 0 {
                return 0;
            }

            // reward = amount * rate * blocks * multiplier / (precision * 100 * blocks_per_year)
            // Simplified: per-block reward scaled by multiplier
            let base_reward = stake
                .amount
                .saturating_mul(self.reward_rate_bps)
                .saturating_mul(blocks_staked)
                / constants::REWARD_RATE_PRECISION
                / 5_256_000; // blocks per year

            let multiplier = stake.lock_period.multiplier();
            base_reward.saturating_mul(multiplier) / 100
        }

        fn remove_governance_power(&mut self, stake: &StakeInfo) {
            let power_holder = stake.governance_delegate.unwrap_or(stake.staker);
            let current = self.governance_power.get(power_holder).unwrap_or(0);
            let new_power = current.saturating_sub(stake.amount);
            if new_power == 0 {
                self.governance_power.remove(power_holder);
            } else {
                self.governance_power.insert(power_holder, &new_power);
            }
        }
    }

    // =========================================================================
    // Tests
    // =========================================================================
}
