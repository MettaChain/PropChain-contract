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

    // Defaults for the on-chain governance module. They are themselves
    // changeable via parameter proposals (ParamKind::VotingPeriodBlocks /
    // QuorumBps), so any clearly-wrong choice here can be voted out.
    const DEFAULT_VOTING_PERIOD_BLOCKS: u64 = 28_800; // ~2 days at 6s blocks
    const DEFAULT_QUORUM_BPS: u32 = 1_000; // 10%
    const MAX_ACTIVE_PARAM_PROPOSALS: u32 = 50;
    const BPS_DENOMINATOR: u32 = 10_000;

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

    #[ink(event)]
    pub struct AutoCompoundUpdated {
        #[ink(topic)]
        pub staker: AccountId,
        pub auto_compound: bool,
    }

    #[ink(event)]
    pub struct RewardsReinvested {
        #[ink(topic)]
        pub staker: AccountId,
        pub amount: u128,
    }

    #[ink(event)]
    pub struct ParamProposalCreated {
        #[ink(topic)]
        pub proposal_id: u64,
        #[ink(topic)]
        pub proposer: AccountId,
        pub kind: ParamKind,
        pub voting_end: u64,
    }

    #[ink(event)]
    pub struct ParamVoteCast {
        #[ink(topic)]
        pub proposal_id: u64,
        #[ink(topic)]
        pub voter: AccountId,
        pub support: bool,
        pub weight: u128,
    }
    #[ink(event)]
    pub struct EarlyWithdrawal {
    #[ink(topic)]
    pub staker: AccountId,
    pub amount_returned: u128,
    pub penalty: u128,
}

    #[ink(event)]
    pub struct ParamProposalExecuted {
        #[ink(topic)]
        pub proposal_id: u64,
        pub kind: ParamKind,
        pub executed_at: u64,
    }

    #[ink(event)]
    pub struct ParamProposalRejected {
        #[ink(topic)]
        pub proposal_id: u64,
    }

    #[ink(event)]
    pub struct ParamProposalCancelled {
        #[ink(topic)]
        pub proposal_id: u64,
    }

    // =========================================================================
    // Storage
    // =========================================================================

    // =========================================================================
    // Delegation Events
    // =========================================================================

    #[ink(event)]
    pub struct ValidatorRegistered {
        #[ink(topic)]
        pub validator: AccountId,
        pub self_stake: u128,
        pub commission_rate: u32,
    }

    #[ink(event)]
    pub struct CommissionRateUpdated {
        #[ink(topic)]
        pub validator: AccountId,
        pub old_rate: u32,
        pub new_rate: u32,
    }

    #[ink(event)]
    pub struct StakeDelegated {
        #[ink(topic)]
        pub delegator: AccountId,
        #[ink(topic)]
        pub validator: AccountId,
        pub amount: u128,
    }

    #[ink(event)]
    pub struct UndelegationInitiated {
        #[ink(topic)]
        pub delegator: AccountId,
        #[ink(topic)]
        pub validator: AccountId,
        pub amount: u128,
        pub claimable_at: u64,
    }

    #[ink(event)]
    pub struct UndelegatedTokensClaimed {
        #[ink(topic)]
        pub delegator: AccountId,
        pub amount: u128,
    }

    #[ink(event)]
    pub struct DelegationRewardsClaimed {
        #[ink(topic)]
        pub delegator: AccountId,
        #[ink(topic)]
        pub validator: AccountId,
        pub amount: u128,
    }

    #[ink(event)]
    pub struct ValidatorCommissionClaimed {
        #[ink(topic)]
        pub validator: AccountId,
        pub amount: u128,
    }

    #[ink(event)]
    pub struct ValidatorSlashed {
        #[ink(topic)]
        pub validator: AccountId,
        pub slash_amount: u128,
        pub delegated_reduction: u128,
    }

    #[ink(event)]
    pub struct ValidatorDeactivated {
        #[ink(topic)]
        pub validator: AccountId,
        pub reason: DeactivationReason,
    }

    #[ink(event)]
    pub struct ValidatorReactivated {
        #[ink(topic)]
        pub validator: AccountId,
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
        // ----- Parameter governance -----
        proposal_counter: u64,
        active_proposal_count: u32,
        param_proposals: Mapping<u64, ParamProposal>,
        param_votes: Mapping<(u64, AccountId), bool>,
        voting_period_blocks: u64,
        quorum_bps: u32,
        early_withdrawal_penalty_bps: u128,
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
                proposal_counter: 0,
                active_proposal_count: 0,
                param_proposals: Mapping::default(),
                param_votes: Mapping::default(),
                voting_period_blocks: DEFAULT_VOTING_PERIOD_BLOCKS,
                quorum_bps: DEFAULT_QUORUM_BPS,
                early_withdrawal_penalty_bps: constants::DEFAULT_EARLY_WITHDRAWAL_PENALTY_BPS,
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

        /// Estimate projected staking rewards for a given amount, lock period, and duration.
        /// This is a read-only calculator — no state is modified.
        #[ink(message)]
        pub fn calculate_projected_rewards(
            &self,
            amount: u128,
            lock_period: LockPeriod,
            duration_blocks: u64,
        ) -> u128 {
            if amount == 0 || duration_blocks == 0 {
                return 0;
            }

            let blocks = duration_blocks as u128;

            // base_reward = amount * reward_rate_bps * blocks / REWARD_RATE_PRECISION / blocks_per_year
            let base_reward = amount
                .saturating_mul(self.reward_rate_bps)
                .saturating_mul(blocks)
                / constants::REWARD_RATE_PRECISION
                / 5_256_000;

            if base_reward == 0 {
                return 0;
            }

            // Apply lock period multiplier
            let multiplier = lock_period.multiplier();
            let reward = base_reward.saturating_mul(multiplier) / 100;

            // Apply staking tier bonus
            let tier = self.get_tier_internal(amount);
            let tier_multiplier = tier.reward_multiplier();
            reward.saturating_mul(tier_multiplier) / 100
        }

        /// Returns the estimated reward plus the staking tier for a projected stake.
        #[ink(message)]
        pub fn calculate_projected_rewards_with_tier(
            &self,
            amount: u128,
            lock_period: LockPeriod,
            duration_blocks: u64,
        ) -> (u128, StakingTier) {
            let reward = self.calculate_projected_rewards(amount, lock_period, duration_blocks);
            let tier = self.get_tier_internal(amount);
            (reward, tier)
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
                auto_compound: false,
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

       /// Unstake tokens. If called before the lock period expires, a penalty
/// of `early_withdrawal_penalty_bps` is deducted from the returned amount.
/// The penalty amount is retained in the reward pool. 
        #[ink(message)]
        pub fn unstake(&mut self) -> Result<(), Error> {
             propchain_traits::non_reentrant!(self, {
        let caller = self.env().caller();
        let stake = self.stakes.get(caller).ok_or(Error::StakeNotFound)?;

        let now = self.env().block_number() as u64;
        let amount = stake.amount;
        let is_early = now < stake.lock_until;

        // Calculate penalty for early withdrawal (zero for on-time or flexible)
        let penalty = if is_early && stake.lock_period != LockPeriod::Flexible {
            amount
                .saturating_mul(self.early_withdrawal_penalty_bps)
                .saturating_div(constants::BASIS_POINTS_DENOMINATOR as u128)
        } else {
            0
        };

        let amount_returned = amount.saturating_sub(penalty);

        // Remove governance power
        self.remove_governance_power(&stake);

        self.stakes.remove(caller);
        self.total_staked = self.total_staked.saturating_sub(amount);

        // Penalty stays in the reward pool to benefit remaining stakers
        if penalty > 0 {
            self.reward_pool = self.reward_pool.saturating_add(penalty);
        }

        // Remove from staker list
        if let Some(pos) = self.staker_list.iter().position(|s| *s == caller) {
            self.staker_list.swap_remove(pos);
        }

        if is_early && stake.lock_period != LockPeriod::Flexible {
            self.env().emit_event(EarlyWithdrawal {
                staker: caller,
                amount_returned,
                penalty,
            });
        } else {
            self.env().emit_event(Unstaked {
                staker: caller,
                amount,
            });
        }

        Ok(())
    })
        }
        /// Update the early withdrawal penalty rate. Admin only.
/// `penalty_bps` must not exceed `MAX_EARLY_WITHDRAWAL_PENALTY_BPS`.
///
#[ink(message)]
pub fn set_early_withdrawal_penalty(
    &mut self,
    penalty_bps: u128,
) -> Result<(), Error> {
    if self.env().caller() != self.admin {
        return Err(Error::Unauthorized);
    }
    if penalty_bps > constants::MAX_EARLY_WITHDRAWAL_PENALTY_BPS {
        return Err(Error::InvalidConfig);
    }
    self.early_withdrawal_penalty_bps = penalty_bps;
    Ok(())
}

/// Get the current early withdrawal penalty rate in basis points.
#[ink(message)]
pub fn get_early_withdrawal_penalty_bps(&self) -> u128 {
    self.early_withdrawal_penalty_bps
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

                let now = self.env().block_number() as u64;
                self.reward_pool = self.reward_pool.saturating_sub(rewards);

                if stake.auto_compound {
                    stake.amount = stake.amount.saturating_add(rewards);
                    self.total_staked = self.total_staked.saturating_add(rewards);

                    // Update governance power
                    let power_holder = stake.governance_delegate.unwrap_or(stake.staker);
                    let current_power = self.governance_power.get(power_holder).unwrap_or(0);
                    self.governance_power.insert(power_holder, &current_power.saturating_add(rewards));

                    stake.staked_at = now;
                    stake.reward_debt = self.acc_reward_per_share;
                    self.stakes.insert(caller, &stake);

                    self.env().emit_event(RewardsReinvested {
                        staker: caller,
                        amount: rewards,
                    });
                } else {
                    stake.staked_at = now;
                    stake.reward_debt = self.acc_reward_per_share;
                    self.stakes.insert(caller, &stake);

                    self.env().emit_event(RewardsClaimed {
                        staker: caller,
                        amount: rewards,
                    });
                }

                Ok(rewards)
            })
        }

        /// Opt-in or opt-out of automatic compounding.
        #[ink(message)]
        pub fn set_auto_compound(&mut self, auto_compound: bool) -> Result<(), Error> {
            let caller = self.env().caller();
            let mut stake = self.stakes.get(caller).ok_or(Error::StakeNotFound)?;
            stake.auto_compound = auto_compound;
            self.stakes.insert(caller, &stake);
            self.env().emit_event(AutoCompoundUpdated {
                staker: caller,
                auto_compound,
            });
            Ok(())
        }

        /// Returns the staking tier for a staker.
        #[ink(message)]
        pub fn get_staker_tier(&self, staker: AccountId) -> StakingTier {
            if let Some(stake) = self.stakes.get(staker) {
                self.get_tier_internal(stake.amount)
            } else {
                StakingTier::Bronze
            }
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

        // ----- Parameter governance -----

        /// Returns the current voting period (in blocks) and quorum (in bps).
        #[ink(message)]
        pub fn get_voting_config(&self) -> (u64, u32) {
            (self.voting_period_blocks, self.quorum_bps)
        }

        /// Returns a parameter proposal by id, if any.
        #[ink(message)]
        pub fn get_param_proposal(&self, proposal_id: u64) -> Option<ParamProposal> {
            self.param_proposals.get(proposal_id)
        }

        /// Total number of parameter proposals ever created.
        #[ink(message)]
        pub fn get_proposal_count(&self) -> u64 {
            self.proposal_counter
        }

        /// Whether `voter` has already voted on `proposal_id`.
        #[ink(message)]
        pub fn has_voted(&self, proposal_id: u64, voter: AccountId) -> bool {
            self.param_votes.contains((proposal_id, voter))
        }

        /// Propose a change to a staking parameter. Caller must hold governance
        /// power (i.e. be a staker or hold delegated power).
        #[ink(message)]
        pub fn propose_param_change(&mut self, kind: ParamKind) -> Result<u64, Error> {
            let caller = self.env().caller();
            if self.governance_power.get(caller).unwrap_or(0) == 0 {
                return Err(Error::NoVotingPower);
            }
            if self.active_proposal_count >= MAX_ACTIVE_PARAM_PROPOSALS {
                return Err(Error::TooManyProposals);
            }
            Self::validate_param(&kind)?;

            let now = self.env().block_number() as u64;
            let proposal_id = self.proposal_counter;
            self.proposal_counter = self.proposal_counter.saturating_add(1);

            let proposal = ParamProposal {
                id: proposal_id,
                proposer: caller,
                kind,
                votes_for: 0,
                votes_against: 0,
                voting_end: now.saturating_add(self.voting_period_blocks),
                total_power_snapshot: self.total_staked,
                status: ProposalStatus::Active,
                created_at: now,
            };

            self.param_proposals.insert(proposal_id, &proposal);
            self.active_proposal_count = self.active_proposal_count.saturating_add(1);

            self.env().emit_event(ParamProposalCreated {
                proposal_id,
                proposer: caller,
                kind,
                voting_end: proposal.voting_end,
            });

            Ok(proposal_id)
        }

        /// Cast a vote on an active parameter proposal, weighted by the
        /// caller's current governance power.
        #[ink(message)]
        pub fn vote_on_proposal(
            &mut self,
            proposal_id: u64,
            support: bool,
        ) -> Result<(), Error> {
            let caller = self.env().caller();
            let weight = self.governance_power.get(caller).unwrap_or(0);
            if weight == 0 {
                return Err(Error::NoVotingPower);
            }

            let mut proposal = self
                .param_proposals
                .get(proposal_id)
                .ok_or(Error::ProposalNotFound)?;

            if proposal.status != ProposalStatus::Active {
                return Err(Error::ProposalClosed);
            }

            let now = self.env().block_number() as u64;
            if now >= proposal.voting_end {
                return Err(Error::VotingEnded);
            }

            if self.param_votes.contains((proposal_id, caller)) {
                return Err(Error::AlreadyVoted);
            }
            self.param_votes.insert((proposal_id, caller), &support);

            if support {
                proposal.votes_for = proposal.votes_for.saturating_add(weight);
            } else {
                proposal.votes_against = proposal.votes_against.saturating_add(weight);
            }
            self.param_proposals.insert(proposal_id, &proposal);

            self.env().emit_event(ParamVoteCast {
                proposal_id,
                voter: caller,
                support,
                weight,
            });

            Ok(())
        }

        /// Finalise a proposal once its voting window has closed. If quorum is
        /// reached and `votes_for > votes_against`, the parameter change is
        /// applied; otherwise the proposal is rejected. Anyone may call.
        #[ink(message)]
        pub fn execute_param_proposal(&mut self, proposal_id: u64) -> Result<(), Error> {
            let mut proposal = self
                .param_proposals
                .get(proposal_id)
                .ok_or(Error::ProposalNotFound)?;

            if proposal.status != ProposalStatus::Active {
                return Err(Error::ProposalClosed);
            }

            let now = self.env().block_number() as u64;
            if now < proposal.voting_end {
                return Err(Error::VotingActive);
            }

            let total_votes = proposal.votes_for.saturating_add(proposal.votes_against);
            let quorum_required = proposal
                .total_power_snapshot
                .saturating_mul(self.quorum_bps as u128)
                / BPS_DENOMINATOR as u128;

            self.active_proposal_count = self.active_proposal_count.saturating_sub(1);

            if total_votes < quorum_required {
                proposal.status = ProposalStatus::Rejected;
                self.param_proposals.insert(proposal_id, &proposal);
                self.env()
                    .emit_event(ParamProposalRejected { proposal_id });
                return Err(Error::QuorumNotReached);
            }

            if proposal.votes_for <= proposal.votes_against {
                proposal.status = ProposalStatus::Rejected;
                self.param_proposals.insert(proposal_id, &proposal);
                self.env()
                    .emit_event(ParamProposalRejected { proposal_id });
                return Ok(());
            }

            self.apply_param(proposal.kind);
            proposal.status = ProposalStatus::Executed;
            self.param_proposals.insert(proposal_id, &proposal);

            self.env().emit_event(ParamProposalExecuted {
                proposal_id,
                kind: proposal.kind,
                executed_at: now,
            });

            Ok(())
        }

        /// Cancel an active proposal. Only the proposer or admin may cancel.
        #[ink(message)]
        pub fn cancel_param_proposal(&mut self, proposal_id: u64) -> Result<(), Error> {
            let caller = self.env().caller();
            let mut proposal = self
                .param_proposals
                .get(proposal_id)
                .ok_or(Error::ProposalNotFound)?;

            if proposal.status != ProposalStatus::Active {
                return Err(Error::ProposalClosed);
            }
            if caller != proposal.proposer && caller != self.admin {
                return Err(Error::Unauthorized);
            }

            proposal.status = ProposalStatus::Cancelled;
            self.active_proposal_count = self.active_proposal_count.saturating_sub(1);
            self.param_proposals.insert(proposal_id, &proposal);

            self.env()
                .emit_event(ParamProposalCancelled { proposal_id });

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
            let reward = base_reward.saturating_mul(multiplier) / 100;

            // Apply staking tier bonus multiplier
            let tier = self.get_tier_internal(stake.amount);
            let tier_multiplier = tier.reward_multiplier();
            reward.saturating_mul(tier_multiplier) / 100
        }

        fn get_tier_internal(&self, amount: u128) -> StakingTier {
            if amount >= 500_000 {
                StakingTier::Diamond
            } else if amount >= 100_000 {
                StakingTier::Platinum
            } else if amount >= 50_000 {
                StakingTier::Gold
            } else if amount >= 10_000 {
                StakingTier::Silver
            } else {
                StakingTier::Bronze
            }
        }

        fn validate_param(kind: &ParamKind) -> Result<(), Error> {
            match kind {
                ParamKind::MinStake(v) => {
                    if *v == 0 {
                        return Err(Error::InvalidConfig);
                    }
                }
                ParamKind::RewardRateBps(_) => {}
                ParamKind::VotingPeriodBlocks(v) => {
                    if *v == 0 {
                        return Err(Error::InvalidConfig);
                    }
                }
                ParamKind::QuorumBps(v) => {
                    if *v == 0 || *v > BPS_DENOMINATOR {
                        return Err(Error::InvalidConfig);
                    }
                }
            }
            Ok(())
        }

        fn apply_param(&mut self, kind: ParamKind) {
            match kind {
                ParamKind::MinStake(v) => {
                    self.min_stake = v;
                    self.env().emit_event(StakingConfigUpdated {
                        min_stake: self.min_stake,
                        reward_rate_bps: self.reward_rate_bps,
                    });
                }
                ParamKind::RewardRateBps(v) => {
                    self.reward_rate_bps = v;
                    self.env().emit_event(StakingConfigUpdated {
                        min_stake: self.min_stake,
                        reward_rate_bps: self.reward_rate_bps,
                    });
                }
                ParamKind::VotingPeriodBlocks(v) => {
                    self.voting_period_blocks = v;
                }
                ParamKind::QuorumBps(v) => {
                    self.quorum_bps = v;
                }
            }
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

        // =========================================================================
        // Delegated Staking — Internal Helpers
        // =========================================================================

        /// Sync the per-validator reward accumulator up to the current block.
        /// Must be called before any state mutation that touches a validator's
        /// reward state.
        fn update_validator_rewards(&mut self, validator: AccountId) {
            let mut info = match self.validators.get(validator) {
                Some(v) => v,
                None => return,
            };
            let now = self.env().block_number() as u64;
            let blocks = (now as u128).saturating_sub(info.last_reward_block as u128);
            if blocks == 0 || info.total_delegated == 0 {
                info.last_reward_block = now;
                self.validators.insert(validator, &info);
                return;
            }
            // gross reward for the delegated pool over elapsed blocks
            let gross_reward = info
                .total_delegated
                .saturating_mul(self.reward_rate_bps)
                .saturating_mul(blocks)
                / constants::REWARD_RATE_PRECISION
                / 5_256_000; // blocks per year

            let commission = gross_reward
                .saturating_mul(info.commission_rate as u128)
                / BPS_DENOMINATOR as u128;
            let net_reward = gross_reward.saturating_sub(commission);

            info.accumulated_commission = info.accumulated_commission.saturating_add(commission);
            info.acc_reward_per_share = info.acc_reward_per_share.saturating_add(
                net_reward
                    .saturating_mul(REWARD_PRECISION)
                    / info.total_delegated,
            );
            info.last_reward_block = now;
            self.validators.insert(validator, &info);
        }

        /// Project the pending delegation reward for a record without writing state.
        fn pending_delegation_reward(
            &self,
            record: &DelegationRecord,
            info: &ValidatorInfo,
        ) -> u128 {
            let now = self.env().block_number() as u64;
            let blocks = (now as u128).saturating_sub(info.last_reward_block as u128);

            let projected_acc = if info.total_delegated > 0 && blocks > 0 {
                let gross = info
                    .total_delegated
                    .saturating_mul(self.reward_rate_bps)
                    .saturating_mul(blocks)
                    / constants::REWARD_RATE_PRECISION
                    / 5_256_000;
                let commission = gross
                    .saturating_mul(info.commission_rate as u128)
                    / BPS_DENOMINATOR as u128;
                let net = gross.saturating_sub(commission);
                info.acc_reward_per_share.saturating_add(
                    net.saturating_mul(REWARD_PRECISION) / info.total_delegated,
                )
            } else {
                info.acc_reward_per_share
            };

            (record
                .amount
                .saturating_mul(projected_acc)
                / REWARD_PRECISION)
                .saturating_sub(record.reward_debt)
        }

        // =========================================================================
        // Delegated Staking — Validator Lifecycle
        // =========================================================================

        /// Register the caller as a validator with a self-stake and commission rate.
        #[ink(message)]
        pub fn register_validator(
            &mut self,
            self_stake: u128,
            commission_rate: u32,
        ) -> Result<(), Error> {
            let caller = self.env().caller();
            if self_stake < MIN_VALIDATOR_STAKE {
                return Err(Error::InsufficientValidatorStake);
            }
            if commission_rate > MAX_COMMISSION_RATE {
                return Err(Error::InvalidCommissionRate);
            }
            if self.validators.contains(caller) {
                return Err(Error::AlreadyValidator);
            }
            let now = self.env().block_number() as u64;
            let info = ValidatorInfo {
                self_stake,
                commission_rate,
                total_delegated: 0,
                accumulated_commission: 0,
                is_active: true,
                acc_reward_per_share: 0,
                last_reward_block: now,
            };
            self.validators.insert(caller, &info);
            self.validator_list.push(caller);
            self.env().emit_event(ValidatorRegistered {
                validator: caller,
                self_stake,
                commission_rate,
            });
            Ok(())
        }

        /// Update the caller's commission rate.
        #[ink(message)]
        pub fn update_commission_rate(&mut self, new_rate: u32) -> Result<(), Error> {
            let caller = self.env().caller();
            if !self.validators.contains(caller) {
                return Err(Error::Unauthorized);
            }
            if new_rate > MAX_COMMISSION_RATE {
                return Err(Error::InvalidCommissionRate);
            }
            self.update_validator_rewards(caller);
            let mut info = self.validators.get(caller).unwrap();
            let old_rate = info.commission_rate;
            info.commission_rate = new_rate;
            self.validators.insert(caller, &info);
            self.env().emit_event(CommissionRateUpdated {
                validator: caller,
                old_rate,
                new_rate,
            });
            Ok(())
        }

        /// Voluntarily deactivate the caller's validator.
        #[ink(message)]
        pub fn deactivate_validator(&mut self) -> Result<(), Error> {
            let caller = self.env().caller();
            let mut info = self.validators.get(caller).ok_or(Error::Unauthorized)?;
            info.is_active = false;
            self.validators.insert(caller, &info);
            self.env().emit_event(ValidatorDeactivated {
                validator: caller,
                reason: DeactivationReason::Voluntary,
            });
            Ok(())
        }

        /// Reactivate an inactive validator.
        #[ink(message)]
        pub fn reactivate_validator(&mut self) -> Result<(), Error> {
            let caller = self.env().caller();
            let mut info = self.validators.get(caller).ok_or(Error::Unauthorized)?;
            if info.self_stake < MIN_VALIDATOR_STAKE {
                return Err(Error::InsufficientValidatorStake);
            }
            info.is_active = true;
            self.validators.insert(caller, &info);
            self.env().emit_event(ValidatorReactivated { validator: caller });
            Ok(())
        }

        /// Admin-only: slash a validator and propagate to all delegators.
        #[ink(message)]
        pub fn slash_validator(&mut self, validator: AccountId) -> Result<(), Error> {
            propchain_traits::non_reentrant!(self, {
                self.ensure_admin()?;
                if !self.validators.contains(validator) {
                    return Err(Error::ValidatorNotFound);
                }

                self.update_validator_rewards(validator);
                let mut info = self.validators.get(validator).unwrap();

                // Slash validator self-stake
                let self_slash = info.self_stake.saturating_mul(SLASH_PERCENT) / 100;
                info.self_stake = info.self_stake.saturating_sub(self_slash);

                // Slash each delegator
                let delegators = self
                    .validator_delegators
                    .get(validator)
                    .unwrap_or_default();
                let mut total_delegated_reduction: u128 = 0;
                for delegator in &delegators {
                    let key = (*delegator, validator);
                    if let Some(mut record) = self.delegations.get(key) {
                        let d_slash = record.amount.saturating_mul(SLASH_PERCENT) / 100;
                        record.amount = record.amount.saturating_sub(d_slash);
                        total_delegated_reduction =
                            total_delegated_reduction.saturating_add(d_slash);
                        self.delegations.insert(key, &record);
                    }
                }

                info.total_delegated = info
                    .total_delegated
                    .saturating_sub(total_delegated_reduction);
                self.total_delegated_stake = self
                    .total_delegated_stake
                    .saturating_sub(total_delegated_reduction);

                let was_active = info.is_active;
                if info.self_stake < MIN_VALIDATOR_STAKE {
                    info.is_active = false;
                }
                self.validators.insert(validator, &info);

                self.env().emit_event(ValidatorSlashed {
                    validator,
                    slash_amount: self_slash,
                    delegated_reduction: total_delegated_reduction,
                });

                if was_active && !info.is_active {
                    self.env().emit_event(ValidatorDeactivated {
                        validator,
                        reason: DeactivationReason::Slashed,
                    });
                }

                Ok(())
            })
        }

        // =========================================================================
        // Delegated Staking — Delegation Lifecycle
        // =========================================================================

        /// Delegate `amount` tokens to `validator`.
        #[ink(message)]
        pub fn delegate(
            &mut self,
            validator: AccountId,
            amount: u128,
        ) -> Result<(), Error> {
            propchain_traits::non_reentrant!(self, {
                let caller = self.env().caller();
                let info = self
                    .validators
                    .get(validator)
                    .ok_or(Error::ValidatorNotActive)?;
                if !info.is_active {
                    return Err(Error::ValidatorNotActive);
                }
                if amount < self.min_stake {
                    return Err(Error::InsufficientAmount);
                }
                if self.delegations.contains((caller, validator)) {
                    return Err(Error::AlreadyDelegated);
                }

                self.update_validator_rewards(validator);
                let info = self.validators.get(validator).unwrap();

                let reward_debt = info
                    .acc_reward_per_share
                    .saturating_mul(amount)
                    / REWARD_PRECISION;

                let record = DelegationRecord {
                    delegator: caller,
                    validator,
                    amount,
                    reward_debt,
                    unbonding_start: None,
                };
                self.delegations.insert((caller, validator), &record);

                // Update secondary indices
                let mut delegators = self
                    .validator_delegators
                    .get(validator)
                    .unwrap_or_default();
                delegators.push(caller);
                self.validator_delegators.insert(validator, &delegators);
                self.delegator_validator.insert(caller, &validator);

                // Update validator totals
                let mut info = self.validators.get(validator).unwrap();
                info.total_delegated = info.total_delegated.saturating_add(amount);
                self.validators.insert(validator, &info);
                self.total_delegated_stake =
                    self.total_delegated_stake.saturating_add(amount);

                self.env().emit_event(StakeDelegated {
                    delegator: caller,
                    validator,
                    amount,
                });
                Ok(())
            })
        }

        /// Initiate unbonding for the caller's delegation.
        #[ink(message)]
        pub fn undelegate(&mut self, validator: AccountId) -> Result<(), Error> {
            propchain_traits::non_reentrant!(self, {
                let caller = self.env().caller();
                let mut record = self
                    .delegations
                    .get((caller, validator))
                    .ok_or(Error::DelegationNotFound)?;

                if record.unbonding_start.is_some() {
                    return Err(Error::AlreadyUnbonding);
                }

                self.update_validator_rewards(validator);
                let mut info = self.validators.get(validator).unwrap();

                let now = self.env().block_number() as u64;
                record.unbonding_start = Some(now);
                self.delegations.insert((caller, validator), &record);

                info.total_delegated = info.total_delegated.saturating_sub(record.amount);
                self.validators.insert(validator, &info);
                self.total_delegated_stake = self
                    .total_delegated_stake
                    .saturating_sub(record.amount);

                self.env().emit_event(UndelegationInitiated {
                    delegator: caller,
                    validator,
                    amount: record.amount,
                    claimable_at: now.saturating_add(UNBONDING_PERIOD_BLOCKS),
                });
                Ok(())
            })
        }

        /// Claim tokens after the unbonding period has elapsed.
        #[ink(message)]
        pub fn claim_undelegated(&mut self, validator: AccountId) -> Result<u128, Error> {
            propchain_traits::non_reentrant!(self, {
                let caller = self.env().caller();
                let record = self
                    .delegations
                    .get((caller, validator))
                    .ok_or(Error::DelegationNotFound)?;

                let start = record.unbonding_start.ok_or(Error::DelegationNotFound)?;
                let now = self.env().block_number() as u64;
                if now < start.saturating_add(UNBONDING_PERIOD_BLOCKS) {
                    return Err(Error::UnbondingPeriodActive);
                }

                let amount = record.amount;
                self.delegations.remove((caller, validator));
                self.delegator_validator.remove(caller);

                // Remove from validator_delegators list
                let mut delegators = self
                    .validator_delegators
                    .get(validator)
                    .unwrap_or_default();
                if let Some(pos) = delegators.iter().position(|d| *d == caller) {
                    delegators.swap_remove(pos);
                }
                self.validator_delegators.insert(validator, &delegators);

                self.env().emit_event(UndelegatedTokensClaimed {
                    delegator: caller,
                    amount,
                });
                Ok(amount)
            })
        }

        /// Claim pending delegation rewards (net of validator commission).
        #[ink(message)]
        pub fn claim_delegation_rewards(
            &mut self,
            validator: AccountId,
        ) -> Result<u128, Error> {
            propchain_traits::non_reentrant!(self, {
                let caller = self.env().caller();
                let record = self
                    .delegations
                    .get((caller, validator))
                    .ok_or(Error::DelegationNotFound)?;

                self.update_validator_rewards(validator);
                let info = self.validators.get(validator).unwrap();

                let reward = self.pending_delegation_reward(&record, &info);
                if reward == 0 {
                    return Err(Error::NoRewards);
                }
                if reward > self.reward_pool {
                    return Err(Error::InsufficientPool);
                }

                self.reward_pool = self.reward_pool.saturating_sub(reward);

                let mut record = self.delegations.get((caller, validator)).unwrap();
                record.reward_debt = info
                    .acc_reward_per_share
                    .saturating_mul(record.amount)
                    / REWARD_PRECISION;
                self.delegations.insert((caller, validator), &record);

                self.env().emit_event(DelegationRewardsClaimed {
                    delegator: caller,
                    validator,
                    amount: reward,
                });
                Ok(reward)
            })
        }

        /// Validator claims their accumulated commission.
        #[ink(message)]
        pub fn claim_validator_commission(&mut self) -> Result<u128, Error> {
            propchain_traits::non_reentrant!(self, {
                let caller = self.env().caller();
                if !self.validators.contains(caller) {
                    return Err(Error::Unauthorized);
                }

                self.update_validator_rewards(caller);
                let mut info = self.validators.get(caller).unwrap();

                if info.accumulated_commission == 0 {
                    return Err(Error::NoRewards);
                }
                if info.accumulated_commission > self.reward_pool {
                    return Err(Error::InsufficientPool);
                }

                let commission = info.accumulated_commission;
                self.reward_pool = self.reward_pool.saturating_sub(commission);
                info.accumulated_commission = 0;
                self.validators.insert(caller, &info);

                self.env().emit_event(ValidatorCommissionClaimed {
                    validator: caller,
                    amount: commission,
                });
                Ok(commission)
            })
        }

        // =========================================================================
        // Delegated Staking — Queries
        // =========================================================================

        /// Returns the DelegationRecord for a (delegator, validator) pair.
        #[ink(message)]
        pub fn get_delegation(
            &self,
            delegator: AccountId,
            validator: AccountId,
        ) -> Option<DelegationRecord> {
            self.delegations.get((delegator, validator))
        }

        /// Returns the ValidatorInfo for a validator account.
        #[ink(message)]
        pub fn get_validator_info(&self, validator: AccountId) -> Option<ValidatorInfo> {
            self.validators.get(validator)
        }

        /// Returns the pending (unclaimed) delegation reward for a delegator.
        #[ink(message)]
        pub fn get_pending_delegation_rewards(
            &self,
            delegator: AccountId,
            validator: AccountId,
        ) -> u128 {
            let record = match self.delegations.get((delegator, validator)) {
                Some(r) => r,
                None => return 0,
            };
            let info = match self.validators.get(validator) {
                Some(i) => i,
                None => return 0,
            };
            self.pending_delegation_reward(&record, &info)
        }

        /// Returns the list of all registered validator accounts.
        #[ink(message)]
        pub fn get_validator_list(&self) -> Vec<AccountId> {
            self.validator_list.clone()
        }

        /// Returns the total delegated stake across all validators.
        #[ink(message)]
        pub fn get_total_delegated_stake(&self) -> u128 {
            self.total_delegated_stake
        }
    }

    // =========================================================================
    // Tests
    // =========================================================================
    include!("tests.rs");
}
