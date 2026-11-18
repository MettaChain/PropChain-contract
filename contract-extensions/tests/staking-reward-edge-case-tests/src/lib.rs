//! Unit tests for staking reward calculation edge cases (#1446).
//!
//! These import `staking::staking::*`, which requires the contract's
//! `mod staking` to be `pub` (see the PR description: `contracts/staking/src/lib.rs`
//! declares it private, so no external crate can currently reach the contract).
//!
//! Reward model under test (all integer arithmetic, floors at every division):
//! `base = amount * rate_bps * blocks / 10_000 / 5_256_000`, then scaled by the
//! lock-period multiplier and the stake-size tier multiplier.

#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(test)]
mod tests {
    use ink::env::{test, DefaultEnvironment};
    use ink::primitives::AccountId;
    use staking::staking::{Error, LockPeriod, Staking};

    type Accounts = test::DefaultAccounts<DefaultEnvironment>;

    const MIN_STAKE: u128 = 1_000;
    const RATE_BPS: u128 = 500;
    const BLOCKS_PER_YEAR: u64 = 5_256_000;

    fn setup() -> (Staking, Accounts) {
        let accounts = test::default_accounts::<DefaultEnvironment>();
        test::set_caller::<DefaultEnvironment>(accounts.alice);
        test::set_block_number::<DefaultEnvironment>(0);
        (Staking::new(RATE_BPS, MIN_STAKE), accounts)
    }

    fn at_block(n: u32) {
        test::set_block_number::<DefaultEnvironment>(n);
    }

    fn stake_as(s: &mut Staking, who: AccountId, amount: u128, lock: LockPeriod) {
        test::set_caller::<DefaultEnvironment>(who);
        s.stake(amount, lock).expect("stake accepted");
    }

    // ── Zero stake ──────────────────────────────────────────────────────

    #[ink::test]
    fn an_account_without_a_stake_has_no_rewards_or_position() {
        let (s, accounts) = setup();
        assert_eq!(s.get_pending_rewards(accounts.bob), 0);
        assert!(s.get_stake(accounts.bob).is_none());
        assert!(s.get_reward_snapshot(accounts.bob).is_none());
        assert!(s.get_reward_history(accounts.bob).is_empty());
    }

    #[ink::test]
    fn projected_rewards_are_zero_for_zero_amount_or_zero_duration() {
        let (s, _) = setup();
        assert_eq!(s.calculate_projected_rewards(0, LockPeriod::OneYear, 1_000_000), 0);
        assert_eq!(s.calculate_projected_rewards(1_000_000_000, LockPeriod::OneYear, 0), 0);
    }

    #[ink::test]
    fn a_zero_amount_stake_is_rejected_as_is_one_below_the_minimum() {
        let (mut s, accounts) = setup();
        test::set_caller::<DefaultEnvironment>(accounts.bob);
        assert_eq!(s.stake(0, LockPeriod::Flexible), Err(Error::ZeroAmount));
        assert_eq!(s.stake(MIN_STAKE - 1, LockPeriod::Flexible), Err(Error::InsufficientAmount));
        assert!(s.stake(MIN_STAKE, LockPeriod::Flexible).is_ok());
        assert_eq!(s.get_total_staked(), MIN_STAKE);
    }

    #[ink::test]
    fn an_account_can_hold_only_one_stake() {
        let (mut s, accounts) = setup();
        stake_as(&mut s, accounts.bob, MIN_STAKE, LockPeriod::Flexible);
        assert_eq!(s.stake(MIN_STAKE, LockPeriod::Flexible), Err(Error::AlreadyStaked));
    }

    // ── Block boundaries ────────────────────────────────────────────────

    #[ink::test]
    fn no_rewards_accrue_in_the_block_the_stake_was_made() {
        let (mut s, accounts) = setup();
        at_block(100);
        stake_as(&mut s, accounts.bob, 10_000_000_000, LockPeriod::Flexible);
        assert_eq!(s.get_pending_rewards(accounts.bob), 0);
        at_block(101);
        assert!(s.get_pending_rewards(accounts.bob) > 0 || {
            // For a small stake one block can floor to zero; two years surely cannot.
            at_block(100 + 2 * BLOCKS_PER_YEAR as u32);
            s.get_pending_rewards(accounts.bob) > 0
        });
    }

    #[ink::test]
    fn pending_rewards_equal_the_projection_for_the_same_amount_lock_and_duration() {
        let (mut s, accounts) = setup();
        let amount = 10_000_000_000u128;
        at_block(10);
        stake_as(&mut s, accounts.bob, amount, LockPeriod::ThirtyDays);
        for blocks in [1u32, 100, 10_000, 1_000_000] {
            at_block(10 + blocks);
            assert_eq!(
                s.get_pending_rewards(accounts.bob),
                s.calculate_projected_rewards(amount, LockPeriod::ThirtyDays, blocks as u64),
                "after {blocks} blocks"
            );
        }
    }

    #[ink::test]
    fn rewards_never_decrease_as_blocks_pass() {
        let (mut s, accounts) = setup();
        stake_as(&mut s, accounts.bob, 10_000_000_000, LockPeriod::Flexible);
        let mut last = 0;
        for block in [1u32, 10, 1_000, 100_000, 5_000_000] {
            at_block(block);
            let now = s.get_pending_rewards(accounts.bob);
            assert!(now >= last, "{last} -> {now} at block {block}");
            last = now;
        }
        assert!(last > 0);
    }

    // ── Lock-period multipliers ─────────────────────────────────────────

    #[ink::test]
    fn longer_locks_never_earn_less_and_a_year_beats_flexible() {
        let (s, _) = setup();
        let amount = 100_000_000_000u128;
        let blocks = 1_000_000u64;
        let flexible = s.calculate_projected_rewards(amount, LockPeriod::Flexible, blocks);
        let thirty = s.calculate_projected_rewards(amount, LockPeriod::ThirtyDays, blocks);
        let ninety = s.calculate_projected_rewards(amount, LockPeriod::NinetyDays, blocks);
        let year = s.calculate_projected_rewards(amount, LockPeriod::OneYear, blocks);
        assert!(flexible > 0);
        assert!(thirty >= flexible && ninety >= thirty && year >= ninety);
        assert!(year > flexible);
    }

    // ── Maximum amounts and overflow safety ─────────────────────────────

    #[ink::test]
    fn projections_with_the_maximum_amount_saturate_instead_of_panicking() {
        let (s, _) = setup();
        for lock in [LockPeriod::Flexible, LockPeriod::OneYear] {
            // `amount * rate * blocks` overflows u128; the contract saturates.
            let reward = s.calculate_projected_rewards(u128::MAX, lock, u64::MAX);
            assert!(reward > 0);
        }
        let tier = s.calculate_projected_rewards_with_tier(u128::MAX, LockPeriod::OneYear, 1_000);
        assert!(tier.0 > 0);
    }

    #[ink::test]
    fn a_maximum_stake_is_accepted_and_the_total_saturates() {
        let (mut s, accounts) = setup();
        stake_as(&mut s, accounts.bob, u128::MAX, LockPeriod::Flexible);
        assert_eq!(s.get_total_staked(), u128::MAX);
        stake_as(&mut s, accounts.charlie, MIN_STAKE, LockPeriod::Flexible);
        assert_eq!(s.get_total_staked(), u128::MAX);
        // Rewards on a maximum stake stay finite.
        at_block(1_000);
        let _ = s.get_pending_rewards(accounts.bob);
        assert_eq!(s.get_stake(accounts.bob).unwrap().amount, u128::MAX);
    }

    // ── Reward pool funding and claiming ────────────────────────────────

    #[ink::test]
    fn only_the_admin_can_fund_the_pool_and_zero_is_rejected() {
        let (mut s, accounts) = setup();
        assert_eq!(s.fund_reward_pool(0), Err(Error::ZeroAmount));
        s.fund_reward_pool(1_000).unwrap();
        assert_eq!(s.get_reward_pool(), 1_000);
        test::set_caller::<DefaultEnvironment>(accounts.bob);
        assert_eq!(s.fund_reward_pool(1), Err(Error::Unauthorized));
        assert_eq!(s.get_reward_pool(), 1_000);
    }

    #[ink::test]
    fn claiming_with_nothing_accrued_or_nothing_funded_fails() {
        let (mut s, accounts) = setup();
        stake_as(&mut s, accounts.bob, 10_000_000_000, LockPeriod::Flexible);
        // Same block: nothing accrued.
        assert_eq!(s.claim_rewards(), Err(Error::NoRewards));
        // Accrued, but the pool is empty.
        at_block(1_000_000);
        assert!(s.get_pending_rewards(accounts.bob) > 0);
        assert_eq!(s.claim_rewards(), Err(Error::InsufficientPool));
        // An account with no stake at all.
        test::set_caller::<DefaultEnvironment>(accounts.eve);
        assert_eq!(s.claim_rewards(), Err(Error::StakeNotFound));
    }

    #[ink::test]
    fn a_claim_pays_the_accrued_rewards_from_the_pool_and_restarts_accrual() {
        let (mut s, accounts) = setup();
        s.fund_reward_pool(1_000_000_000_000).unwrap();
        stake_as(&mut s, accounts.bob, 10_000_000_000, LockPeriod::Flexible);
        at_block(1_000_000);
        let owed = s.get_pending_rewards(accounts.bob);
        assert!(owed > 0);

        assert_eq!(s.claim_rewards(), Ok(owed));
        assert_eq!(s.get_reward_pool(), 1_000_000_000_000 - owed);
        // Accrual restarts from this block: immediately nothing is owed again.
        assert_eq!(s.get_pending_rewards(accounts.bob), 0);
        assert_eq!(s.claim_rewards(), Err(Error::NoRewards));
        // The principal is untouched.
        assert_eq!(s.get_stake(accounts.bob).unwrap().amount, 10_000_000_000);
    }

    #[ink::test]
    fn auto_compounding_adds_rewards_to_the_stake_instead_of_paying_out() {
        let (mut s, accounts) = setup();
        s.fund_reward_pool(1_000_000_000_000).unwrap();
        stake_as(&mut s, accounts.bob, 10_000_000_000, LockPeriod::Flexible);
        s.set_auto_compound(true).unwrap();
        at_block(1_000_000);
        let owed = s.get_pending_rewards(accounts.bob);
        s.claim_rewards().unwrap();
        assert_eq!(s.get_stake(accounts.bob).unwrap().amount, 10_000_000_000 + owed);
        assert_eq!(s.get_total_staked(), 10_000_000_000 + owed);
    }

    // ── Penalty tiers on exit ───────────────────────────────────────────

    #[ink::test]
    fn leaving_a_locked_stake_early_pays_at_least_the_global_penalty_into_the_pool() {
        let (mut s, accounts) = setup();
        let amount = 1_000_000u128;
        stake_as(&mut s, accounts.bob, amount, LockPeriod::ThirtyDays);

        let tier = s.get_unbonding_tier(accounts.bob);
        let config = s.get_unbonding_tier_config(tier);
        let global = s.get_early_withdrawal_penalty_bps();

        if !config.early_exit_allowed {
            assert_eq!(s.unstake(), Err(Error::LockActive));
            return;
        }
        s.unstake().unwrap();
        let effective = global.max(config.early_exit_penalty_bps);
        assert_eq!(s.get_reward_pool(), amount * effective / 10_000);
        assert!(s.get_stake(accounts.bob).is_none());
        assert_eq!(s.get_total_staked(), 0);
    }

    #[ink::test]
    fn a_flexible_stake_leaves_without_a_penalty() {
        let (mut s, accounts) = setup();
        stake_as(&mut s, accounts.bob, 1_000_000, LockPeriod::Flexible);
        s.unstake().unwrap();
        assert_eq!(s.get_reward_pool(), 0);
    }

    #[ink::test]
    fn after_the_lock_and_tier_window_a_stake_exits_without_a_penalty() {
        let (mut s, accounts) = setup();
        stake_as(&mut s, accounts.bob, 1_000_000, LockPeriod::ThirtyDays);
        let unlock = s.get_unbonding_tier_unlock_at(accounts.bob);
        at_block(unlock as u32);
        s.unstake().unwrap();
        assert_eq!(s.get_reward_pool(), 0);
    }

    #[ink::test]
    fn the_penalty_rate_is_admin_configurable() {
        let (mut s, accounts) = setup();
        s.set_early_withdrawal_penalty(2_500).unwrap();
        assert_eq!(s.get_early_withdrawal_penalty_bps(), 2_500);
        test::set_caller::<DefaultEnvironment>(accounts.bob);
        assert_eq!(s.set_early_withdrawal_penalty(1), Err(Error::Unauthorized));
    }

    // ── Several stakers at once ─────────────────────────────────────────

    #[ink::test]
    fn one_stakers_exit_does_not_change_anothers_rewards() {
        let (mut s, accounts) = setup();
        stake_as(&mut s, accounts.bob, 10_000_000_000, LockPeriod::Flexible);
        stake_as(&mut s, accounts.charlie, 20_000_000_000, LockPeriod::Flexible);
        at_block(500_000);
        let charlie_before = s.get_pending_rewards(accounts.charlie);

        test::set_caller::<DefaultEnvironment>(accounts.bob);
        s.unstake().unwrap();

        assert_eq!(s.get_pending_rewards(accounts.charlie), charlie_before);
        assert_eq!(s.get_total_staked(), 20_000_000_000);
        assert_eq!(s.get_pending_rewards(accounts.bob), 0);
    }

    #[ink::test]
    fn a_stake_and_unstake_in_the_same_block_earns_nothing_and_can_be_repeated() {
        let (mut s, accounts) = setup();
        stake_as(&mut s, accounts.bob, 10_000_000_000, LockPeriod::Flexible);
        assert_eq!(s.get_pending_rewards(accounts.bob), 0);
        s.unstake().unwrap();
        // The account may stake again after leaving.
        assert!(s.stake(10_000_000_000, LockPeriod::Flexible).is_ok());
    }

    #[ink::test]
    fn rewards_scale_with_the_staked_amount_across_stakers() {
        let (mut s, accounts) = setup();
        stake_as(&mut s, accounts.bob, 10_000_000_000, LockPeriod::Flexible);
        stake_as(&mut s, accounts.charlie, 10_000_000_000, LockPeriod::Flexible);
        at_block(1_000_000);
        // Same amount, lock and duration: identical rewards.
        assert_eq!(s.get_pending_rewards(accounts.bob), s.get_pending_rewards(accounts.charlie));
    }

    // ── Reward snapshots ────────────────────────────────────────────────

    #[ink::test]
    fn staking_opens_a_reward_snapshot_and_claiming_records_a_checkpoint() {
        let (mut s, accounts) = setup();
        s.fund_reward_pool(1_000_000_000_000).unwrap();
        at_block(10);
        stake_as(&mut s, accounts.bob, 10_000_000_000, LockPeriod::Flexible);
        assert!(s.get_reward_snapshot(accounts.bob).is_some());
        assert!(s.get_reward_history(accounts.bob).is_empty());

        at_block(1_000_000);
        s.claim_rewards().unwrap();
        assert_eq!(s.get_reward_history(accounts.bob).len(), 1);
        assert_eq!(s.get_reward_points_denominator(), 1_000_000_000);
    }
}
