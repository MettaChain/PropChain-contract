use ink::primitives::AccountId;
use ink::storage::Mapping;
use scale::{Decode, Encode};

// ── Error type ────────────────────────────────────────────────────────────────

/// All recoverable errors the dashboard can surface.
#[derive(Encode, Decode, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub enum StakingError {
    /// The requested unstake amount exceeds the staker's current balance.
    InsufficientStake,
    /// `amount` was zero — staking or unstaking zero tokens is a no-op error.
    ZeroAmount,
    /// Arithmetic overflow in an internal accumulator.
    Overflow,
    /// The staker's tokens are still in the lock-up period.
    StillLocked,
    /// The staker has no rewards to claim.
    NoRewardsToClaim,
}

pub type Result<T> = core::result::Result<T, StakingError>;

// ── Events ────────────────────────────────────────────────────────────────────

/// Emitted when a staker adds tokens.
#[ink::event]
pub struct Staked {
    #[ink(topic)]
    pub staker: AccountId,
    pub amount: u128,
    pub new_balance: u128,
}

/// Emitted when a staker removes tokens.
#[ink::event]
pub struct Unstaked {
    #[ink(topic)]
    pub staker: AccountId,
    pub amount: u128,
    pub new_balance: u128,
}

/// Emitted when a staker claims their accumulated rewards.
#[ink::event]
pub struct RewardClaimed {
    #[ink(topic)]
    pub staker: AccountId,
    pub amount: u128,
}

/// Emitted when the contract distributes a reward tranche.
#[ink::event]
pub struct RewardDistributed {
    pub amount: u128,
    pub reward_per_token_delta: u128,
}

// ── Per-staker record ─────────────────────────────────────────────────────────

/// Everything the contract needs to know about a single staker.
#[derive(Encode, Decode, Clone, Debug, Default, PartialEq, Eq)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub struct StakerRecord {
    /// Current staked balance.
    pub balance: u128,
    /// Snapshot of `reward_per_token_stored` at the time of the last
    /// stake / unstake / claim.  Used to compute pending rewards without
    /// iterating over all stakers.
    pub reward_per_token_paid: u128,
    /// Rewards accrued but not yet claimed.
    pub pending_rewards: u128,
    /// Block number at which the staker last staked (used for lock-up check).
    pub staked_at_block: u32,
}

// ── Dashboard stats snapshot ──────────────────────────────────────────────────

/// Point-in-time view returned by `get_stats`.
#[derive(Encode, Decode, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub struct StakingStats {
    /// Total tokens currently staked across all stakers.
    pub total_staked: u128,
    /// Number of accounts with a non-zero stake.
    pub total_stakers: u32,
    /// Mean stake per staker (0 when there are no stakers).
    pub average_stake: u128,
    /// Cumulative rewards distributed since contract deployment.
    pub rewards_distributed: u128,
    /// Unclaimed rewards sitting in the contract.
    pub unclaimed_rewards: u128,
    /// Running `reward_per_token` accumulator (scaled by 1e18).
    pub reward_per_token_stored: u128,
}

// ── Dashboard ─────────────────────────────────────────────────────────────────

/// Scaling factor for `reward_per_token_stored` to preserve precision in
/// integer arithmetic.  1e18 matches the common EVM convention.
const REWARD_PRECISION: u128 = 1_000_000_000_000_000_000;

/// Number of blocks a stake is locked before unstaking is allowed.
/// Set to 0 to disable the lock-up requirement.
const LOCK_PERIOD_BLOCKS: u32 = 10;

pub struct StakingDashboard {
    /// Per-staker data (balance, reward checkpoint, pending rewards).
    pub staker_records: Mapping<AccountId, StakerRecord>,
    /// Sum of all staked balances.
    pub total_staked: u128,
    /// Number of stakers with a non-zero balance.
    pub total_stakers: u32,
    /// Cumulative rewards distributed (monotonically increasing).
    pub rewards_distributed: u128,
    /// Rewards that have been distributed but not yet claimed.
    pub unclaimed_rewards: u128,
    /// Accumulator: total rewards per staked token, scaled by `REWARD_PRECISION`.
    /// Updated on every `distribute_reward` call.
    pub reward_per_token_stored: u128,
}

impl StakingDashboard {
    // ── Public read helpers ───────────────────────────────────────────────────

    /// Snapshot of aggregate statistics.
    pub fn get_stats(&self) -> StakingStats {
        StakingStats {
            total_staked: self.total_staked,
            total_stakers: self.total_stakers,
            average_stake: if self.total_stakers > 0 {
                self.total_staked / self.total_stakers as u128
            } else {
                0
            },
            rewards_distributed: self.rewards_distributed,
            unclaimed_rewards: self.unclaimed_rewards,
            reward_per_token_stored: self.reward_per_token_stored,
        }
    }

    /// Current staked balance for `staker`.
    pub fn staker_balance(&self, staker: AccountId) -> u128 {
        self.staker_records
            .get(staker)
            .map(|r| r.balance)
            .unwrap_or(0)
    }

    /// Rewards `staker` has earned but not yet claimed.
    pub fn pending_rewards(&self, staker: AccountId) -> u128 {
        let record = self.staker_records.get(staker).unwrap_or_default();
        self.compute_earned(&record)
    }

    // ── Write operations ──────────────────────────────────────────────────────

    /// Record a stake of `amount` tokens for `staker` at `current_block`.
    ///
    /// - Rejects zero amounts.
    /// - Accumulates pending rewards before updating the balance so the
    ///   staker's existing stake earns rewards at the old rate first.
    /// - Returns the staker's new total balance.
    pub fn record_stake(
        &mut self,
        staker: AccountId,
        amount: u128,
        current_block: u32,
    ) -> Result<u128> {
        if amount == 0 {
            return Err(StakingError::ZeroAmount);
        }

        let mut record = self.staker_records.get(staker).unwrap_or_default();

        // Settle any rewards earned up to this point at the current rate.
        self.settle_rewards(&mut record);

        let new_balance = record
            .balance
            .checked_add(amount)
            .ok_or(StakingError::Overflow)?;
        let new_total = self
            .total_staked
            .checked_add(amount)
            .ok_or(StakingError::Overflow)?;

        if record.balance == 0 {
            // First stake for this account.
            self.total_stakers = self
                .total_stakers
                .checked_add(1)
                .ok_or(StakingError::Overflow)?;
        }

        record.balance = new_balance;
        record.staked_at_block = current_block;
        self.staker_records.insert(staker, &record);
        self.total_staked = new_total;

        Ok(new_balance)
    }

    /// Record an unstake of `amount` tokens for `staker` at `current_block`.
    ///
    /// - Rejects zero amounts.
    /// - Enforces the `LOCK_PERIOD_BLOCKS` lock-up.
    /// - Accumulates pending rewards before reducing the balance.
    /// - Returns the staker's new total balance.
    pub fn record_unstake(
        &mut self,
        staker: AccountId,
        amount: u128,
        current_block: u32,
    ) -> Result<u128> {
        if amount == 0 {
            return Err(StakingError::ZeroAmount);
        }

        let mut record = self.staker_records.get(staker).unwrap_or_default();

        if amount > record.balance {
            return Err(StakingError::InsufficientStake);
        }

        // Enforce lock-up period.
        if LOCK_PERIOD_BLOCKS > 0
            && current_block < record.staked_at_block.saturating_add(LOCK_PERIOD_BLOCKS)
        {
            return Err(StakingError::StillLocked);
        }

        // Settle rewards before balance changes.
        self.settle_rewards(&mut record);

        // Both subtractions are safe because we validated `amount <= record.balance`
        // and `amount <= total_staked` follows from per-staker accounting.
        record.balance -= amount;
        self.total_staked = self.total_staked.saturating_sub(amount);

        if record.balance == 0 {
            self.total_stakers = self.total_stakers.saturating_sub(1);
        }

        let new_balance = record.balance;
        self.staker_records.insert(staker, &record);

        Ok(new_balance)
    }

    /// Claim all pending rewards for `staker`.
    ///
    /// Returns the amount claimed.  Callers are responsible for transferring
    /// the tokens to the staker.
    pub fn claim_rewards(&mut self, staker: AccountId) -> Result<u128> {
        let mut record = self.staker_records.get(staker).unwrap_or_default();
        self.settle_rewards(&mut record);

        let claimable = record.pending_rewards;
        if claimable == 0 {
            return Err(StakingError::NoRewardsToClaim);
        }

        record.pending_rewards = 0;
        self.staker_records.insert(staker, &record);
        self.unclaimed_rewards = self.unclaimed_rewards.saturating_sub(claimable);

        Ok(claimable)
    }

    /// Distribute `amount` tokens as rewards, spread proportionally across
    /// all current stakers via the `reward_per_token_stored` accumulator.
    ///
    /// No-op (returns `Ok(0)`) when there are no stakers.
    pub fn distribute_reward(&mut self, amount: u128) -> Result<u128> {
        if amount == 0 {
            return Err(StakingError::ZeroAmount);
        }

        if self.total_staked == 0 {
            // Nobody to distribute to — discard silently.
            return Ok(0);
        }

        // reward_per_token_delta = amount * REWARD_PRECISION / total_staked
        let delta = amount
            .checked_mul(REWARD_PRECISION)
            .ok_or(StakingError::Overflow)?
            .checked_div(self.total_staked)
            .ok_or(StakingError::Overflow)?;

        self.reward_per_token_stored = self
            .reward_per_token_stored
            .checked_add(delta)
            .ok_or(StakingError::Overflow)?;

        self.rewards_distributed = self
            .rewards_distributed
            .checked_add(amount)
            .ok_or(StakingError::Overflow)?;

        self.unclaimed_rewards = self
            .unclaimed_rewards
            .checked_add(amount)
            .ok_or(StakingError::Overflow)?;

        Ok(delta)
    }

    // ── Private helpers ───────────────────────────────────────────────────────

    /// Compute how many tokens `record` has earned since its last checkpoint.
    fn compute_earned(&self, record: &StakerRecord) -> u128 {
        let rate_delta = self
            .reward_per_token_stored
            .saturating_sub(record.reward_per_token_paid);
        // earned = balance * rate_delta / REWARD_PRECISION + pending
        record
            .balance
            .saturating_mul(rate_delta)
            .saturating_div(REWARD_PRECISION)
            .saturating_add(record.pending_rewards)
    }

    /// Bring `record.pending_rewards` up to date and reset the checkpoint.
    /// Must be called before any balance mutation.
    fn settle_rewards(&self, record: &mut StakerRecord) {
        record.pending_rewards = self.compute_earned(record);
        record.reward_per_token_paid = self.reward_per_token_stored;
    }
}

// ── Unit tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use ink::env::test::default_accounts;

    /// Build a fresh dashboard with zeroed-out state.
    fn fresh() -> StakingDashboard {
        ink::env::test::run_test::<ink::env::DefaultEnvironment, _>(|_| {
            Ok(StakingDashboard {
                staker_records: Mapping::default(),
                total_staked: 0,
                total_stakers: 0,
                rewards_distributed: 0,
                unclaimed_rewards: 0,
                reward_per_token_stored: 0,
            })
        })
        .unwrap()
    }

    fn accounts() -> ink::env::test::DefaultAccounts<ink::env::DefaultEnvironment> {
        default_accounts::<ink::env::DefaultEnvironment>()
    }

    // ── record_stake ──────────────────────────────────────────────────────────

    #[ink::test]
    fn stake_zero_is_rejected() {
        let mut d = fresh();
        let acc = accounts();
        assert_eq!(
            d.record_stake(acc.alice, 0, 1),
            Err(StakingError::ZeroAmount)
        );
    }

    #[ink::test]
    fn stake_increments_balance_and_counters() {
        let mut d = fresh();
        let acc = accounts();
        d.record_stake(acc.alice, 100, 1).unwrap();

        assert_eq!(d.staker_balance(acc.alice), 100);
        assert_eq!(d.total_staked, 100);
        assert_eq!(d.total_stakers, 1);
    }

    #[ink::test]
    fn second_stake_from_same_staker_does_not_increment_staker_count() {
        let mut d = fresh();
        let acc = accounts();
        d.record_stake(acc.alice, 100, 1).unwrap();
        d.record_stake(acc.alice, 50, 2).unwrap();

        assert_eq!(d.total_stakers, 1);
        assert_eq!(d.staker_balance(acc.alice), 150);
    }

    #[ink::test]
    fn two_different_stakers_both_counted() {
        let mut d = fresh();
        let acc = accounts();
        d.record_stake(acc.alice, 100, 1).unwrap();
        d.record_stake(acc.bob, 200, 1).unwrap();

        assert_eq!(d.total_stakers, 2);
        assert_eq!(d.total_staked, 300);
    }

    // ── record_unstake ────────────────────────────────────────────────────────

    #[ink::test]
    fn unstake_zero_is_rejected() {
        let mut d = fresh();
        let acc = accounts();
        d.record_stake(acc.alice, 100, 1).unwrap();
        assert_eq!(
            d.record_unstake(acc.alice, 0, 100),
            Err(StakingError::ZeroAmount)
        );
    }

    #[ink::test]
    fn unstake_more_than_balance_is_rejected() {
        let mut d = fresh();
        let acc = accounts();
        d.record_stake(acc.alice, 100, 1).unwrap();
        assert_eq!(
            d.record_unstake(acc.alice, 101, 100),
            Err(StakingError::InsufficientStake)
        );
    }

    #[ink::test]
    fn unstake_during_lock_period_is_rejected() {
        let mut d = fresh();
        let acc = accounts();
        d.record_stake(acc.alice, 100, 1).unwrap();
        // Current block is still within the lock window.
        let still_locked_block = 1 + LOCK_PERIOD_BLOCKS - 1;
        assert_eq!(
            d.record_unstake(acc.alice, 100, still_locked_block),
            Err(StakingError::StillLocked)
        );
    }

    #[ink::test]
    fn unstake_after_lock_period_succeeds() {
        let mut d = fresh();
        let acc = accounts();
        d.record_stake(acc.alice, 100, 1).unwrap();
        let unlocked_block = 1 + LOCK_PERIOD_BLOCKS;
        let new_balance = d.record_unstake(acc.alice, 100, unlocked_block).unwrap();

        assert_eq!(new_balance, 0);
        assert_eq!(d.total_stakers, 0);
        assert_eq!(d.total_staked, 0);
    }

    #[ink::test]
    fn partial_unstake_leaves_correct_balance() {
        let mut d = fresh();
        let acc = accounts();
        d.record_stake(acc.alice, 100, 1).unwrap();
        let new_balance = d
            .record_unstake(acc.alice, 40, 1 + LOCK_PERIOD_BLOCKS)
            .unwrap();

        assert_eq!(new_balance, 60);
        assert_eq!(d.total_stakers, 1); // still staking
    }

    // ── distribute_reward + pending_rewards ───────────────────────────────────

    #[ink::test]
    fn distribute_reward_with_no_stakers_is_noop() {
        let mut d = fresh();
        let result = d.distribute_reward(1_000).unwrap();
        assert_eq!(result, 0);
        assert_eq!(d.rewards_distributed, 0);
    }

    #[ink::test]
    fn distribute_reward_zero_is_rejected() {
        let mut d = fresh();
        assert_eq!(d.distribute_reward(0), Err(StakingError::ZeroAmount));
    }

    #[ink::test]
    fn single_staker_earns_full_reward() {
        let mut d = fresh();
        let acc = accounts();
        d.record_stake(acc.alice, 1_000, 1).unwrap();
        d.distribute_reward(500).unwrap();

        assert_eq!(d.pending_rewards(acc.alice), 500);
    }

    #[ink::test]
    fn two_stakers_split_reward_proportionally() {
        let mut d = fresh();
        let acc = accounts();
        // Alice stakes twice as much as Bob.
        d.record_stake(acc.alice, 200, 1).unwrap();
        d.record_stake(acc.bob, 100, 1).unwrap();
        d.distribute_reward(300).unwrap();

        let alice_earned = d.pending_rewards(acc.alice);
        let bob_earned = d.pending_rewards(acc.bob);

        // Alice should earn 200 and Bob 100 (integer division may cause ±1 dust).
        assert!((alice_earned as i128 - 200).abs() <= 1);
        assert!((bob_earned as i128 - 100).abs() <= 1);
    }

    #[ink::test]
    fn rewards_settle_before_new_stake() {
        let mut d = fresh();
        let acc = accounts();
        d.record_stake(acc.alice, 1_000, 1).unwrap();
        d.distribute_reward(1_000).unwrap();
        // Alice adds more stake — her existing rewards should be settled first.
        d.record_stake(acc.alice, 500, 5).unwrap();

        // Alice earned 1_000 before the second stake; no new rewards since.
        assert_eq!(d.pending_rewards(acc.alice), 1_000);
    }

    // ── claim_rewards ─────────────────────────────────────────────────────────

    #[ink::test]
    fn claim_with_no_rewards_is_rejected() {
        let mut d = fresh();
        let acc = accounts();
        d.record_stake(acc.alice, 100, 1).unwrap();
        assert_eq!(d.claim_rewards(acc.alice), Err(StakingError::NoRewardsToClaim));
    }

    #[ink::test]
    fn claim_resets_pending_rewards() {
        let mut d = fresh();
        let acc = accounts();
        d.record_stake(acc.alice, 1_000, 1).unwrap();
        d.distribute_reward(500).unwrap();

        let claimed = d.claim_rewards(acc.alice).unwrap();
        assert_eq!(claimed, 500);
        assert_eq!(d.pending_rewards(acc.alice), 0);
        assert_eq!(d.unclaimed_rewards, 0);
    }

    #[ink::test]
    fn multiple_reward_distributions_accumulate_correctly() {
        let mut d = fresh();
        let acc = accounts();
        d.record_stake(acc.alice, 1_000, 1).unwrap();
        d.distribute_reward(100).unwrap();
        d.distribute_reward(200).unwrap();
        d.distribute_reward(700).unwrap();

        assert_eq!(d.pending_rewards(acc.alice), 1_000);
        assert_eq!(d.rewards_distributed, 1_000);
        assert_eq!(d.unclaimed_rewards, 1_000);
    }

    // ── get_stats ─────────────────────────────────────────────────────────────

    #[ink::test]
    fn stats_average_is_zero_with_no_stakers() {
        let d = fresh();
        assert_eq!(d.get_stats().average_stake, 0);
    }

    #[ink::test]
    fn stats_reflect_current_state() {
        let mut d = fresh();
        let acc = accounts();
        d.record_stake(acc.alice, 300, 1).unwrap();
        d.record_stake(acc.bob, 100, 1).unwrap();
        d.distribute_reward(80).unwrap();

        let stats = d.get_stats();
        assert_eq!(stats.total_staked, 400);
        assert_eq!(stats.total_stakers, 2);
        assert_eq!(stats.average_stake, 200);
        assert_eq!(stats.rewards_distributed, 80);
        assert_eq!(stats.unclaimed_rewards, 80);
    }
}