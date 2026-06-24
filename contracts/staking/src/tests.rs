// Unit tests for the staking contract (Issue #101 - extracted from lib.rs)

#[cfg(test)]
mod tests {
    // =========================================================================
    // Delegated Staking Tests
    // =========================================================================

    use super::*;

    fn default_accounts() -> ink::env::test::DefaultAccounts<ink::env::DefaultEnvironment> {
        ink::env::test::default_accounts::<ink::env::DefaultEnvironment>()
    }

    fn set_caller(caller: AccountId) {
        ink::env::test::set_caller::<ink::env::DefaultEnvironment>(caller);
    }

    fn advance_block(n: u32) {
        for _ in 0..n {
            ink::env::test::advance_block::<ink::env::DefaultEnvironment>();
        }
    }

    fn create_staking() -> Staking {
        let accounts = default_accounts();
        set_caller(accounts.alice);
        Staking::new(500, 1_000)
    }

    // ---- Validator Registration ----

    #[ink::test]
    fn register_validator_succeeds() {
        let mut staking = create_staking();
        let accounts = default_accounts();
        set_caller(accounts.bob);
        assert!(staking.register_validator(MIN_VALIDATOR_STAKE, 500).is_ok());
        let info = staking.get_validator_info(accounts.bob).unwrap();
        assert_eq!(info.self_stake, MIN_VALIDATOR_STAKE);
        assert_eq!(info.commission_rate, 500);
        assert_eq!(info.total_delegated, 0);
        assert_eq!(info.accumulated_commission, 0);
        assert!(info.is_active);
    }

    #[ink::test]
    fn register_validator_below_min_stake_fails() {
        let mut staking = create_staking();
        let accounts = default_accounts();
        set_caller(accounts.bob);
        assert_eq!(
            staking.register_validator(MIN_VALIDATOR_STAKE - 1, 500),
            Err(Error::InsufficientValidatorStake)
        );
    }

    #[ink::test]
    fn register_validator_invalid_commission_fails() {
        let mut staking = create_staking();
        let accounts = default_accounts();
        set_caller(accounts.bob);
        assert_eq!(
            staking.register_validator(MIN_VALIDATOR_STAKE, MAX_COMMISSION_RATE + 1),
            Err(Error::InvalidCommissionRate)
        );
    }

    #[ink::test]
    fn register_validator_max_commission_succeeds() {
        let mut staking = create_staking();
        let accounts = default_accounts();
        set_caller(accounts.bob);
        assert!(staking.register_validator(MIN_VALIDATOR_STAKE, MAX_COMMISSION_RATE).is_ok());
    }

    #[ink::test]
    fn register_validator_double_registration_fails() {
        let mut staking = create_staking();
        let accounts = default_accounts();
        set_caller(accounts.bob);
        staking.register_validator(MIN_VALIDATOR_STAKE, 500).unwrap();
        assert_eq!(
            staking.register_validator(MIN_VALIDATOR_STAKE, 500),
            Err(Error::AlreadyValidator)
        );
    }

    #[ink::test]
    fn get_validator_list_returns_registered() {
        let mut staking = create_staking();
        let accounts = default_accounts();
        set_caller(accounts.bob);
        staking.register_validator(MIN_VALIDATOR_STAKE, 500).unwrap();
        let list = staking.get_validator_list();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0], accounts.bob);
    }

    // ---- Commission Rate Update ----

    #[ink::test]
    fn update_commission_rate_succeeds() {
        let mut staking = create_staking();
        let accounts = default_accounts();
        set_caller(accounts.bob);
        staking.register_validator(MIN_VALIDATOR_STAKE, 500).unwrap();
        staking.update_commission_rate(1_000).unwrap();
        let info = staking.get_validator_info(accounts.bob).unwrap();
        assert_eq!(info.commission_rate, 1_000);
    }

    #[ink::test]
    fn update_commission_rate_non_validator_fails() {
        let mut staking = create_staking();
        let accounts = default_accounts();
        set_caller(accounts.bob);
        assert_eq!(
            staking.update_commission_rate(500),
            Err(Error::Unauthorized)
        );
    }

    #[ink::test]
    fn update_commission_rate_exceeds_max_fails() {
        let mut staking = create_staking();
        let accounts = default_accounts();
        set_caller(accounts.bob);
        staking.register_validator(MIN_VALIDATOR_STAKE, 500).unwrap();
        assert_eq!(
            staking.update_commission_rate(MAX_COMMISSION_RATE + 1),
            Err(Error::InvalidCommissionRate)
        );
    }

    // ---- Deactivation / Reactivation ----

    #[ink::test]
    fn deactivate_validator_succeeds() {
        let mut staking = create_staking();
        let accounts = default_accounts();
        set_caller(accounts.bob);
        staking.register_validator(MIN_VALIDATOR_STAKE, 500).unwrap();
        staking.deactivate_validator().unwrap();
        let info = staking.get_validator_info(accounts.bob).unwrap();
        assert!(!info.is_active);
    }

    #[ink::test]
    fn deactivate_non_validator_fails() {
        let mut staking = create_staking();
        let accounts = default_accounts();
        set_caller(accounts.bob);
        assert_eq!(staking.deactivate_validator(), Err(Error::Unauthorized));
    }

    #[ink::test]
    fn reactivate_validator_succeeds() {
        let mut staking = create_staking();
        let accounts = default_accounts();
        set_caller(accounts.bob);
        staking.register_validator(MIN_VALIDATOR_STAKE, 500).unwrap();
        staking.deactivate_validator().unwrap();
        staking.reactivate_validator().unwrap();
        let info = staking.get_validator_info(accounts.bob).unwrap();
        assert!(info.is_active);
    }

    #[ink::test]
    fn reactivate_non_validator_fails() {
        let mut staking = create_staking();
        let accounts = default_accounts();
        set_caller(accounts.bob);
        assert_eq!(staking.reactivate_validator(), Err(Error::Unauthorized));
    }

    // ---- Delegate ----

    #[ink::test]
    fn delegate_succeeds() {
        let mut staking = create_staking();
        let accounts = default_accounts();
        set_caller(accounts.bob);
        staking.register_validator(MIN_VALIDATOR_STAKE, 500).unwrap();

        set_caller(accounts.charlie);
        staking.delegate(accounts.bob, 5_000).unwrap();

        let record = staking.get_delegation(accounts.charlie, accounts.bob).unwrap();
        assert_eq!(record.amount, 5_000);
        assert!(record.unbonding_start.is_none());

        let info = staking.get_validator_info(accounts.bob).unwrap();
        assert_eq!(info.total_delegated, 5_000);
        assert_eq!(staking.get_total_delegated_stake(), 5_000);
    }

    #[ink::test]
    fn delegate_to_inactive_validator_fails() {
        let mut staking = create_staking();
        let accounts = default_accounts();
        set_caller(accounts.bob);
        staking.register_validator(MIN_VALIDATOR_STAKE, 500).unwrap();
        staking.deactivate_validator().unwrap();

        set_caller(accounts.charlie);
        assert_eq!(
            staking.delegate(accounts.bob, 5_000),
            Err(Error::ValidatorNotActive)
        );
    }

    #[ink::test]
    fn delegate_to_unregistered_validator_fails() {
        let mut staking = create_staking();
        let accounts = default_accounts();
        set_caller(accounts.charlie);
        assert_eq!(
            staking.delegate(accounts.bob, 5_000),
            Err(Error::ValidatorNotActive)
        );
    }

    #[ink::test]
    fn delegate_below_min_stake_fails() {
        let mut staking = create_staking();
        let accounts = default_accounts();
        set_caller(accounts.bob);
        staking.register_validator(MIN_VALIDATOR_STAKE, 500).unwrap();

        set_caller(accounts.charlie);
        assert_eq!(
            staking.delegate(accounts.bob, 500), // below min_stake of 1_000
            Err(Error::InsufficientAmount)
        );
    }

    #[ink::test]
    fn delegate_double_delegation_fails() {
        let mut staking = create_staking();
        let accounts = default_accounts();
        set_caller(accounts.bob);
        staking.register_validator(MIN_VALIDATOR_STAKE, 500).unwrap();

        set_caller(accounts.charlie);
        staking.delegate(accounts.bob, 5_000).unwrap();
        assert_eq!(
            staking.delegate(accounts.bob, 5_000),
            Err(Error::AlreadyDelegated)
        );
    }

    // ---- Undelegate ----

    #[ink::test]
    fn undelegate_succeeds() {
        let mut staking = create_staking();
        let accounts = default_accounts();
        set_caller(accounts.bob);
        staking.register_validator(MIN_VALIDATOR_STAKE, 500).unwrap();

        set_caller(accounts.charlie);
        staking.delegate(accounts.bob, 5_000).unwrap();
        staking.undelegate(accounts.bob).unwrap();

        let record = staking.get_delegation(accounts.charlie, accounts.bob).unwrap();
        assert!(record.unbonding_start.is_some());

        let info = staking.get_validator_info(accounts.bob).unwrap();
        assert_eq!(info.total_delegated, 0);
        assert_eq!(staking.get_total_delegated_stake(), 0);
    }

    #[ink::test]
    fn undelegate_no_delegation_fails() {
        let mut staking = create_staking();
        let accounts = default_accounts();
        set_caller(accounts.bob);
        staking.register_validator(MIN_VALIDATOR_STAKE, 500).unwrap();

        set_caller(accounts.charlie);
        assert_eq!(
            staking.undelegate(accounts.bob),
            Err(Error::DelegationNotFound)
        );
    }

    #[ink::test]
    fn undelegate_already_unbonding_fails() {
        let mut staking = create_staking();
        let accounts = default_accounts();
        set_caller(accounts.bob);
        staking.register_validator(MIN_VALIDATOR_STAKE, 500).unwrap();

        set_caller(accounts.charlie);
        staking.delegate(accounts.bob, 5_000).unwrap();
        staking.undelegate(accounts.bob).unwrap();
        assert_eq!(
            staking.undelegate(accounts.bob),
            Err(Error::AlreadyUnbonding)
        );
    }

    // ---- Claim Undelegated ----

    #[ink::test]
    fn claim_undelegated_before_period_fails() {
        let mut staking = create_staking();
        let accounts = default_accounts();
        set_caller(accounts.bob);
        staking.register_validator(MIN_VALIDATOR_STAKE, 500).unwrap();

        set_caller(accounts.charlie);
        staking.delegate(accounts.bob, 5_000).unwrap();
        staking.undelegate(accounts.bob).unwrap();
        assert_eq!(
            staking.claim_undelegated(accounts.bob),
            Err(Error::UnbondingPeriodActive)
        );
    }

    #[ink::test]
    fn claim_undelegated_after_period_succeeds() {
        let mut staking = create_staking();
        let accounts = default_accounts();
        set_caller(accounts.bob);
        staking.register_validator(MIN_VALIDATOR_STAKE, 500).unwrap();

        set_caller(accounts.charlie);
        staking.delegate(accounts.bob, 5_000).unwrap();
        staking.undelegate(accounts.bob).unwrap();

        advance_block(UNBONDING_PERIOD_BLOCKS as u32 + 1);

        let amount = staking.claim_undelegated(accounts.bob).unwrap();
        assert_eq!(amount, 5_000);
        assert!(staking.get_delegation(accounts.charlie, accounts.bob).is_none());
    }

    // ---- Claim Delegation Rewards ----

    #[ink::test]
    fn claim_delegation_rewards_no_delegation_fails() {
        let mut staking = create_staking();
        let accounts = default_accounts();
        set_caller(accounts.bob);
        staking.register_validator(MIN_VALIDATOR_STAKE, 500).unwrap();

        set_caller(accounts.charlie);
        assert_eq!(
            staking.claim_delegation_rewards(accounts.bob),
            Err(Error::DelegationNotFound)
        );
    }

    #[ink::test]
    fn claim_delegation_rewards_empty_pool_fails() {
        let mut staking = create_staking();
        let accounts = default_accounts();
        set_caller(accounts.bob);
        staking.register_validator(MIN_VALIDATOR_STAKE, 500).unwrap();

        set_caller(accounts.charlie);
        staking.delegate(accounts.bob, 1_000_000_000_000_000).unwrap();

        advance_block(100_000);

        // reward_pool is 0 — should fail with InsufficientPool (or NoRewards if 0)
        let result = staking.claim_delegation_rewards(accounts.bob);
        assert!(
            result == Err(Error::NoRewards) || result == Err(Error::InsufficientPool),
            "expected NoRewards or InsufficientPool, got {:?}",
            result
        );
    }

    #[ink::test]
    fn claim_delegation_rewards_with_pool_succeeds() {
        let mut staking = create_staking();
        let accounts = default_accounts();

        set_caller(accounts.alice);
        staking.fund_reward_pool(1_000_000_000_000_000).unwrap();

        set_caller(accounts.bob);
        staking.register_validator(MIN_VALIDATOR_STAKE, 0).unwrap(); // 0% commission

        set_caller(accounts.charlie);
        staking.delegate(accounts.bob, 1_000_000_000_000_000).unwrap();

        advance_block(100_000);

        let pending = staking.get_pending_delegation_rewards(accounts.charlie, accounts.bob);
        assert!(pending > 0, "expected pending rewards > 0, got {}", pending);

        let claimed = staking.claim_delegation_rewards(accounts.bob).unwrap();
        assert!(claimed > 0);

        // After claiming, pending should be ~0
        let pending_after = staking.get_pending_delegation_rewards(accounts.charlie, accounts.bob);
        assert_eq!(pending_after, 0);
    }

    // ---- Claim Validator Commission ----

    #[ink::test]
    fn claim_validator_commission_no_commission_fails() {
        let mut staking = create_staking();
        let accounts = default_accounts();
        set_caller(accounts.bob);
        staking.register_validator(MIN_VALIDATOR_STAKE, 500).unwrap();
        assert_eq!(
            staking.claim_validator_commission(),
            Err(Error::NoRewards)
        );
    }

    #[ink::test]
    fn claim_validator_commission_non_validator_fails() {
        let mut staking = create_staking();
        let accounts = default_accounts();
        set_caller(accounts.bob);
        assert_eq!(
            staking.claim_validator_commission(),
            Err(Error::Unauthorized)
        );
    }

    #[ink::test]
    fn claim_validator_commission_with_pool_succeeds() {
        let mut staking = create_staking();
        let accounts = default_accounts();

        set_caller(accounts.alice);
        staking.fund_reward_pool(1_000_000_000_000_000).unwrap();

        set_caller(accounts.bob);
        staking.register_validator(MIN_VALIDATOR_STAKE, 1_000).unwrap(); // 10% commission

        set_caller(accounts.charlie);
        staking.delegate(accounts.bob, 1_000_000_000_000_000).unwrap();

        advance_block(100_000);

        // Trigger accumulator update by calling claim_delegation_rewards
        // (or directly call claim_validator_commission which calls update internally)
        set_caller(accounts.bob);
        let commission = staking.claim_validator_commission().unwrap();
        assert!(commission > 0, "expected commission > 0, got {}", commission);
    }

    // ---- Slash Validator ----

    #[ink::test]
    fn slash_validator_non_admin_fails() {
        let mut staking = create_staking();
        let accounts = default_accounts();
        set_caller(accounts.bob);
        staking.register_validator(MIN_VALIDATOR_STAKE, 500).unwrap();

        set_caller(accounts.charlie);
        assert_eq!(
            staking.slash_validator(accounts.bob),
            Err(Error::Unauthorized)
        );
    }

    #[ink::test]
    fn slash_validator_not_found_fails() {
        let mut staking = create_staking();
        let accounts = default_accounts();
        set_caller(accounts.alice);
        assert_eq!(
            staking.slash_validator(accounts.bob),
            Err(Error::ValidatorNotFound)
        );
    }

    #[ink::test]
    fn slash_validator_reduces_self_stake() {
        let mut staking = create_staking();
        let accounts = default_accounts();
        set_caller(accounts.bob);
        staking.register_validator(MIN_VALIDATOR_STAKE, 500).unwrap();

        set_caller(accounts.alice);
        staking.slash_validator(accounts.bob).unwrap();

        let info = staking.get_validator_info(accounts.bob).unwrap();
        let expected = MIN_VALIDATOR_STAKE * (100 - SLASH_PERCENT) / 100;
        assert_eq!(info.self_stake, expected);
    }

    #[ink::test]
    fn slash_validator_reduces_delegator_amounts() {
        let mut staking = create_staking();
        let accounts = default_accounts();
        set_caller(accounts.bob);
        staking.register_validator(MIN_VALIDATOR_STAKE * 10, 500).unwrap();

        set_caller(accounts.charlie);
        staking.delegate(accounts.bob, 5_000).unwrap();

        set_caller(accounts.alice);
        staking.slash_validator(accounts.bob).unwrap();

        let record = staking.get_delegation(accounts.charlie, accounts.bob).unwrap();
        let expected = 5_000u128 * (100 - SLASH_PERCENT) / 100;
        assert_eq!(record.amount, expected);
    }

    #[ink::test]
    fn slash_validator_below_min_deactivates() {
        let mut staking = create_staking();
        let accounts = default_accounts();
        set_caller(accounts.bob);
        // Register with exactly MIN_VALIDATOR_STAKE so slash drops below minimum
        staking.register_validator(MIN_VALIDATOR_STAKE, 500).unwrap();

        set_caller(accounts.alice);
        staking.slash_validator(accounts.bob).unwrap();

        let info = staking.get_validator_info(accounts.bob).unwrap();
        // After 20% slash: 10_000_000 * 0.8 = 8_000_000 < MIN_VALIDATOR_STAKE
        assert!(!info.is_active);

        // New delegations should be rejected
        set_caller(accounts.charlie);
        assert_eq!(
            staking.delegate(accounts.bob, 5_000),
            Err(Error::ValidatorNotActive)
        );
    }

    // ---- End-to-End Flow ----

    #[ink::test]
    fn full_delegation_lifecycle() {
        let mut staking = create_staking();
        let accounts = default_accounts();

        // Fund pool
        set_caller(accounts.alice);
        staking.fund_reward_pool(1_000_000_000_000_000).unwrap();

        // Register validator with 0% commission
        set_caller(accounts.bob);
        staking.register_validator(MIN_VALIDATOR_STAKE, 0).unwrap();

        // Delegate
        set_caller(accounts.charlie);
        staking.delegate(accounts.bob, 1_000_000_000_000_000).unwrap();

        // Advance blocks to accrue rewards
        advance_block(100_000);

        // Claim rewards
        let reward = staking.claim_delegation_rewards(accounts.bob).unwrap();
        assert!(reward > 0);

        // Undelegate
        staking.undelegate(accounts.bob).unwrap();

        // Advance past unbonding period
        advance_block(UNBONDING_PERIOD_BLOCKS as u32 + 1);

        // Claim undelegated
        let amount = staking.claim_undelegated(accounts.bob).unwrap();
        assert_eq!(amount, 1_000_000_000_000_000);
        assert!(staking.get_delegation(accounts.charlie, accounts.bob).is_none());
    }

    #[ink::test]
    fn slash_multiple_delegators() {
        let mut staking = create_staking();
        let accounts = default_accounts();

        set_caller(accounts.bob);
        staking.register_validator(MIN_VALIDATOR_STAKE * 10, 500).unwrap();

        set_caller(accounts.charlie);
        staking.delegate(accounts.bob, 10_000).unwrap();

        set_caller(accounts.django);
        staking.delegate(accounts.bob, 20_000).unwrap();

        set_caller(accounts.alice);
        staking.slash_validator(accounts.bob).unwrap();

        let r1 = staking.get_delegation(accounts.charlie, accounts.bob).unwrap();
        let r2 = staking.get_delegation(accounts.django, accounts.bob).unwrap();
        assert_eq!(r1.amount, 10_000u128 * (100 - SLASH_PERCENT) / 100);
        assert_eq!(r2.amount, 20_000u128 * (100 - SLASH_PERCENT) / 100);
    }

    #[ink::test]
    fn total_delegated_stake_consistency() {
        let mut staking = create_staking();
        let accounts = default_accounts();

        set_caller(accounts.bob);
        staking.register_validator(MIN_VALIDATOR_STAKE, 500).unwrap();

        set_caller(accounts.charlie);
        staking.delegate(accounts.bob, 5_000).unwrap();

        assert_eq!(staking.get_total_delegated_stake(), 5_000);

        let list = staking.get_validator_list();
        let sum: u128 = list
            .iter()
            .filter_map(|v| staking.get_validator_info(*v))
            .map(|i| i.total_delegated)
            .sum();
        assert_eq!(sum, staking.get_total_delegated_stake());
    }

    #[ink::test]
    fn constructor_sets_defaults() {
        let staking = create_staking();
        let accounts = default_accounts();
        assert_eq!(staking.get_admin(), accounts.alice);
        assert_eq!(staking.get_total_staked(), 0);
        assert_eq!(staking.get_reward_pool(), 0);
        assert_eq!(staking.get_min_stake(), 1_000);
    }

    #[ink::test]
    fn constructor_clamps_zero_min_stake() {
        let accounts = default_accounts();
        set_caller(accounts.alice);
        let staking = Staking::new(500, 0);
        assert_eq!(staking.get_min_stake(), constants::STAKING_MIN_AMOUNT);
    }

    #[ink::test]
    fn stake_succeeds() {
        let mut staking = create_staking();
        let accounts = default_accounts();
        set_caller(accounts.bob);
        let result = staking.stake(10_000, LockPeriod::Flexible);
        assert!(result.is_ok());
        assert_eq!(staking.get_total_staked(), 10_000);

        let info = staking.get_stake(accounts.bob).unwrap();
        assert_eq!(info.amount, 10_000);
        assert_eq!(info.lock_period, LockPeriod::Flexible);
    }

    #[ink::test]
    fn stake_below_minimum_fails() {
        let mut staking = create_staking();
        let accounts = default_accounts();
        set_caller(accounts.bob);
        assert_eq!(
            staking.stake(500, LockPeriod::Flexible),
            Err(Error::InsufficientAmount)
        );
    }

    #[ink::test]
    fn stake_zero_amount_fails() {
        let mut staking = create_staking();
        let accounts = default_accounts();
        set_caller(accounts.bob);
        assert_eq!(
            staking.stake(0, LockPeriod::Flexible),
            Err(Error::ZeroAmount)
        );
    }

    #[ink::test]
    fn double_stake_fails() {
        let mut staking = create_staking();
        let accounts = default_accounts();
        set_caller(accounts.bob);
        staking.stake(10_000, LockPeriod::Flexible).unwrap();
        assert_eq!(
            staking.stake(10_000, LockPeriod::Flexible),
            Err(Error::AlreadyStaked)
        );
    }

    #[ink::test]
    fn unstake_flexible_succeeds() {
        let mut staking = create_staking();
        let accounts = default_accounts();
        set_caller(accounts.bob);
        staking.stake(10_000, LockPeriod::Flexible).unwrap();
        let result = staking.unstake();
        assert!(result.is_ok());
        assert_eq!(staking.get_total_staked(), 0);
        assert!(staking.get_stake(accounts.bob).is_none());
    }

    #[ink::test]
    fn unstake_locked_fails() {
        let mut staking = create_staking();
        let accounts = default_accounts();
        set_caller(accounts.bob);
        staking.stake(10_000, LockPeriod::ThirtyDays).unwrap();
        assert_eq!(staking.unstake(), Err(Error::LockActive));
    }

    #[ink::test]
    fn unstake_no_stake_fails() {
        let mut staking = create_staking();
        let accounts = default_accounts();
        set_caller(accounts.bob);
        assert_eq!(staking.unstake(), Err(Error::StakeNotFound));
    }

    #[ink::test]
    fn claim_rewards_with_pool() {
        let mut staking = create_staking();
        let accounts = default_accounts();

        set_caller(accounts.alice);
        staking.fund_reward_pool(10_000_000_000_000).unwrap();

        set_caller(accounts.bob);
        staking
            .stake(1_000_000_000_000_000, LockPeriod::Flexible)
            .unwrap();

        advance_block(100_000);

        let pending = staking.get_pending_rewards(accounts.bob);
        assert!(
            pending > 0,
            "pending rewards should be > 0, got {}",
            pending
        );

        let result = staking.claim_rewards();
        assert!(result.is_ok());
    }

    #[ink::test]
    fn claim_rewards_no_stake_fails() {
        let mut staking = create_staking();
        let accounts = default_accounts();
        set_caller(accounts.bob);
        assert_eq!(staking.claim_rewards(), Err(Error::StakeNotFound));
    }

    #[ink::test]
    fn delegate_governance_succeeds() {
        let mut staking = create_staking();
        let accounts = default_accounts();
        set_caller(accounts.bob);
        staking.stake(10_000, LockPeriod::Flexible).unwrap();

        assert_eq!(staking.get_governance_power(accounts.bob), 10_000);

        staking.delegate_governance(accounts.charlie).unwrap();
        assert_eq!(staking.get_governance_power(accounts.bob), 0);
        assert_eq!(staking.get_governance_power(accounts.charlie), 10_000);
    }

    #[ink::test]
    fn self_delegation_fails() {
        let mut staking = create_staking();
        let accounts = default_accounts();
        set_caller(accounts.bob);
        staking.stake(10_000, LockPeriod::Flexible).unwrap();
        assert_eq!(
            staking.delegate_governance(accounts.bob),
            Err(Error::InvalidDelegate)
        );
    }

    #[ink::test]
    fn fund_pool_non_admin_fails() {
        let mut staking = create_staking();
        let accounts = default_accounts();
        set_caller(accounts.bob);
        assert_eq!(staking.fund_reward_pool(1000), Err(Error::Unauthorized));
    }

    #[ink::test]
    fn update_config_succeeds() {
        let mut staking = create_staking();
        staking.update_config(5_000, 1000).unwrap();
        assert_eq!(staking.get_min_stake(), 5_000);
    }

    #[ink::test]
    fn update_config_zero_min_fails() {
        let mut staking = create_staking();
        assert_eq!(staking.update_config(0, 1000), Err(Error::InvalidConfig));
    }

    #[ink::test]
    fn lock_period_durations_correct() {
        assert_eq!(LockPeriod::Flexible.duration_blocks(), 0);
        assert_eq!(
            LockPeriod::ThirtyDays.duration_blocks(),
            constants::LOCK_PERIOD_30_DAYS
        );
        assert_eq!(
            LockPeriod::NinetyDays.duration_blocks(),
            constants::LOCK_PERIOD_90_DAYS
        );
        assert_eq!(
            LockPeriod::OneYear.duration_blocks(),
            constants::LOCK_PERIOD_1_YEAR
        );
    }

    #[ink::test]
    fn multipliers_increase_with_lock() {
        assert!(LockPeriod::ThirtyDays.multiplier() > LockPeriod::Flexible.multiplier());
        assert!(LockPeriod::NinetyDays.multiplier() > LockPeriod::ThirtyDays.multiplier());
        assert!(LockPeriod::OneYear.multiplier() > LockPeriod::NinetyDays.multiplier());
    }

    // ----- Parameter governance -----

    fn end_voting_period(staking: &Staking) {
        let (period, _) = staking.get_voting_config();
        advance_block(period as u32 + 1);
    }

    #[ink::test]
    fn propose_requires_voting_power() {
        let mut staking = create_staking();
        let accounts = default_accounts();
        set_caller(accounts.bob);
        assert_eq!(
            staking.propose_param_change(ParamKind::MinStake(2_000)),
            Err(Error::NoVotingPower),
        );
    }

    #[ink::test]
    fn propose_param_change_records_proposal() {
        let mut staking = create_staking();
        let accounts = default_accounts();

        set_caller(accounts.bob);
        staking.stake(10_000, LockPeriod::Flexible).unwrap();

        let id = staking
            .propose_param_change(ParamKind::MinStake(2_000))
            .unwrap();
        assert_eq!(id, 0);
        let p = staking.get_param_proposal(0).unwrap();
        assert_eq!(p.kind, ParamKind::MinStake(2_000));
        assert_eq!(p.status, ProposalStatus::Active);
        assert_eq!(p.total_power_snapshot, 10_000);
    }

    #[ink::test]
    fn propose_invalid_param_fails() {
        let mut staking = create_staking();
        let accounts = default_accounts();
        set_caller(accounts.bob);
        staking.stake(10_000, LockPeriod::Flexible).unwrap();
        assert_eq!(
            staking.propose_param_change(ParamKind::MinStake(0)),
            Err(Error::InvalidConfig),
        );
        assert_eq!(
            staking.propose_param_change(ParamKind::QuorumBps(20_000)),
            Err(Error::InvalidConfig),
        );
    }

    #[ink::test]
    fn vote_weight_uses_governance_power() {
        let mut staking = create_staking();
        let accounts = default_accounts();

        set_caller(accounts.bob);
        staking.stake(10_000, LockPeriod::Flexible).unwrap();

        let id = staking
            .propose_param_change(ParamKind::RewardRateBps(750))
            .unwrap();
        staking.vote_on_proposal(id, true).unwrap();

        let p = staking.get_param_proposal(id).unwrap();
        assert_eq!(p.votes_for, 10_000);
        assert_eq!(p.votes_against, 0);
        assert!(staking.has_voted(id, accounts.bob));
    }

    #[ink::test]
    fn double_vote_fails() {
        let mut staking = create_staking();
        let accounts = default_accounts();
        set_caller(accounts.bob);
        staking.stake(10_000, LockPeriod::Flexible).unwrap();
        let id = staking
            .propose_param_change(ParamKind::RewardRateBps(750))
            .unwrap();
        staking.vote_on_proposal(id, true).unwrap();
        assert_eq!(staking.vote_on_proposal(id, false), Err(Error::AlreadyVoted));
    }

    #[ink::test]
    fn vote_without_power_fails() {
        let mut staking = create_staking();
        let accounts = default_accounts();
        set_caller(accounts.bob);
        staking.stake(10_000, LockPeriod::Flexible).unwrap();
        let id = staking
            .propose_param_change(ParamKind::RewardRateBps(750))
            .unwrap();

        set_caller(accounts.charlie);
        assert_eq!(staking.vote_on_proposal(id, true), Err(Error::NoVotingPower));
    }

    #[ink::test]
    fn execute_applies_winning_proposal() {
        let mut staking = create_staking();
        let accounts = default_accounts();

        set_caller(accounts.bob);
        staking.stake(10_000, LockPeriod::Flexible).unwrap();
        let id = staking
            .propose_param_change(ParamKind::MinStake(2_500))
            .unwrap();
        staking.vote_on_proposal(id, true).unwrap();

        end_voting_period(&staking);
        staking.execute_param_proposal(id).unwrap();

        assert_eq!(staking.get_min_stake(), 2_500);
        let p = staking.get_param_proposal(id).unwrap();
        assert_eq!(p.status, ProposalStatus::Executed);
    }

    #[ink::test]
    fn execute_before_voting_end_fails() {
        let mut staking = create_staking();
        let accounts = default_accounts();
        set_caller(accounts.bob);
        staking.stake(10_000, LockPeriod::Flexible).unwrap();
        let id = staking
            .propose_param_change(ParamKind::MinStake(2_500))
            .unwrap();
        staking.vote_on_proposal(id, true).unwrap();
        assert_eq!(
            staking.execute_param_proposal(id),
            Err(Error::VotingActive),
        );
    }

    #[ink::test]
    fn vote_after_voting_end_fails() {
        let mut staking = create_staking();
        let accounts = default_accounts();
        set_caller(accounts.bob);
        staking.stake(10_000, LockPeriod::Flexible).unwrap();
        let id = staking
            .propose_param_change(ParamKind::MinStake(2_500))
            .unwrap();
        end_voting_period(&staking);
        assert_eq!(staking.vote_on_proposal(id, true), Err(Error::VotingEnded));
    }

    #[ink::test]
    fn execute_quorum_not_reached_rejects() {
        let mut staking = create_staking();
        let accounts = default_accounts();

        // Two stakers; only one of them votes. Quorum is 10% of total stake
        // so a single voter with > 10% of the supply still meets quorum —
        // pick weights so quorum is missed instead.
        set_caller(accounts.bob);
        staking.stake(1_000, LockPeriod::Flexible).unwrap();
        set_caller(accounts.charlie);
        staking.stake(1_000_000, LockPeriod::Flexible).unwrap();

        set_caller(accounts.bob);
        let id = staking
            .propose_param_change(ParamKind::MinStake(2_500))
            .unwrap();
        staking.vote_on_proposal(id, true).unwrap();

        end_voting_period(&staking);
        assert_eq!(
            staking.execute_param_proposal(id),
            Err(Error::QuorumNotReached),
        );

        let p = staking.get_param_proposal(id).unwrap();
        assert_eq!(p.status, ProposalStatus::Rejected);
        // Original min_stake unchanged.
        assert_eq!(staking.get_min_stake(), 1_000);
    }

    #[ink::test]
    fn execute_majority_against_rejects() {
        let mut staking = create_staking();
        let accounts = default_accounts();

        set_caller(accounts.bob);
        staking.stake(10_000, LockPeriod::Flexible).unwrap();
        set_caller(accounts.charlie);
        staking.stake(20_000, LockPeriod::Flexible).unwrap();

        set_caller(accounts.bob);
        let id = staking
            .propose_param_change(ParamKind::MinStake(5_000))
            .unwrap();
        staking.vote_on_proposal(id, true).unwrap();

        set_caller(accounts.charlie);
        staking.vote_on_proposal(id, false).unwrap();

        end_voting_period(&staking);
        // No quorum failure: Ok(()) but proposal rejected and parameter unchanged.
        staking.execute_param_proposal(id).unwrap();
        let p = staking.get_param_proposal(id).unwrap();
        assert_eq!(p.status, ProposalStatus::Rejected);
        assert_eq!(staking.get_min_stake(), 1_000);
    }

    #[ink::test]
    fn execute_twice_fails() {
        let mut staking = create_staking();
        let accounts = default_accounts();
        set_caller(accounts.bob);
        staking.stake(10_000, LockPeriod::Flexible).unwrap();
        let id = staking
            .propose_param_change(ParamKind::MinStake(2_500))
            .unwrap();
        staking.vote_on_proposal(id, true).unwrap();
        end_voting_period(&staking);
        staking.execute_param_proposal(id).unwrap();
        assert_eq!(
            staking.execute_param_proposal(id),
            Err(Error::ProposalClosed),
        );
    }

    #[ink::test]
    fn cancel_by_proposer_succeeds() {
        let mut staking = create_staking();
        let accounts = default_accounts();
        set_caller(accounts.bob);
        staking.stake(10_000, LockPeriod::Flexible).unwrap();
        let id = staking
            .propose_param_change(ParamKind::MinStake(2_500))
            .unwrap();
        staking.cancel_param_proposal(id).unwrap();
        let p = staking.get_param_proposal(id).unwrap();
        assert_eq!(p.status, ProposalStatus::Cancelled);
    }

    #[ink::test]
    fn cancel_by_outsider_fails() {
        let mut staking = create_staking();
        let accounts = default_accounts();
        set_caller(accounts.bob);
        staking.stake(10_000, LockPeriod::Flexible).unwrap();
        let id = staking
            .propose_param_change(ParamKind::MinStake(2_500))
            .unwrap();
        set_caller(accounts.charlie);
        assert_eq!(
            staking.cancel_param_proposal(id),
            Err(Error::Unauthorized),
        );
    }

    #[ink::test]
    fn voting_period_can_be_governed() {
        let mut staking = create_staking();
        let accounts = default_accounts();
        set_caller(accounts.bob);
        staking.stake(10_000, LockPeriod::Flexible).unwrap();

        let id = staking
            .propose_param_change(ParamKind::VotingPeriodBlocks(100))
            .unwrap();
        staking.vote_on_proposal(id, true).unwrap();
        end_voting_period(&staking);
        staking.execute_param_proposal(id).unwrap();

        assert_eq!(staking.get_voting_config().0, 100);
    }

    #[ink::test]
    fn delegate_can_vote_with_full_power() {
        let mut staking = create_staking();
        let accounts = default_accounts();

        set_caller(accounts.bob);
        staking.stake(10_000, LockPeriod::Flexible).unwrap();
        staking.delegate_governance(accounts.charlie).unwrap();

        // Bob has no power any more.
        let id = staking
            .propose_param_change(ParamKind::MinStake(2_000))
            .ok();
        assert!(id.is_none());

        // Charlie now holds Bob's power and can drive the proposal.
        set_caller(accounts.charlie);
        let id = staking
            .propose_param_change(ParamKind::MinStake(2_000))
            .unwrap();
        staking.vote_on_proposal(id, true).unwrap();
        end_voting_period(&staking);
        staking.execute_param_proposal(id).unwrap();

        assert_eq!(staking.get_min_stake(), 2_000);
    }

    #[ink::test]
    fn set_auto_compound_succeeds() {
        let mut staking = create_staking();
        let accounts = default_accounts();
        set_caller(accounts.bob);
        staking.stake(10_000, LockPeriod::Flexible).unwrap();
        
        let stake_info = staking.get_stake(accounts.bob).unwrap();
        assert_eq!(stake_info.auto_compound, false);
        
        staking.set_auto_compound(true).unwrap();
        let stake_info = staking.get_stake(accounts.bob).unwrap();
        assert_eq!(stake_info.auto_compound, true);
    }

    #[ink::test]
    fn auto_compounding_reinvests_rewards() {
        let mut staking = create_staking();
        let accounts = default_accounts();
        
        set_caller(accounts.alice);
        staking.fund_reward_pool(10_000_000_000_000).unwrap();

        set_caller(accounts.bob);
        staking.stake(1_000_000_000_000_000, LockPeriod::Flexible).unwrap();
        staking.set_auto_compound(true).unwrap();

        advance_block(100_000);

        let initial_stake = staking.get_stake(accounts.bob).unwrap().amount;
        let pending = staking.get_pending_rewards(accounts.bob);
        assert!(pending > 0);

        staking.claim_rewards().unwrap();

        let final_stake = staking.get_stake(accounts.bob).unwrap().amount;
        assert_eq!(final_stake, initial_stake + pending);
    }

    #[ink::test]
  
#[ink::test]
fn early_withdrawal_applies_penalty() {
    let mut staking = create_staking();
    let accounts = default_accounts();
    set_caller(accounts.bob);

    staking.stake(10_000_000_000_000, LockPeriod::ThirtyDays).unwrap();

    // Unstake immediately — lock not expired, penalty should apply
    assert!(staking.unstake().is_ok());

    assert_eq!(staking.get_total_staked(), 0);
    // 10% of 10_000_000_000_000 = 1_000_000_000_000 went to reward pool
    assert!(staking.get_reward_pool() >= 1_000_000_000_000);
}

#[ink::test]
fn flexible_lock_no_penalty_on_early_unstake() {
    let mut staking = create_staking();
    let accounts = default_accounts();
    set_caller(accounts.bob);

    staking.stake(10_000_000_000_000, LockPeriod::Flexible).unwrap();

    let pool_before = staking.get_reward_pool();
    assert!(staking.unstake().is_ok());
    assert_eq!(staking.get_total_staked(), 0);
    // No penalty for flexible
    assert_eq!(staking.get_reward_pool(), pool_before);
}

#[ink::test]
fn no_penalty_after_lock_expires() {
    let mut staking = create_staking();
    let accounts = default_accounts();
    set_caller(accounts.bob);

    staking.stake(10_000_000_000_000, LockPeriod::ThirtyDays).unwrap();

    // Advance past the 30-day lock period
    advance_block(constants::LOCK_PERIOD_30_DAYS as u32 + 1);

    let pool_before = staking.get_reward_pool();
    assert!(staking.unstake().is_ok());
    // Reward pool unchanged — no penalty after expiry
    assert_eq!(staking.get_reward_pool(), pool_before);
}

#[ink::test]
fn set_early_withdrawal_penalty_admin_only() {
    let mut staking = create_staking();
    let accounts = default_accounts();

    // Admin (alice) can update
    assert!(staking.set_early_withdrawal_penalty(500).is_ok());
    assert_eq!(staking.get_early_withdrawal_penalty_bps(), 500);

    // Non-admin cannot
    set_caller(accounts.bob);
    assert_eq!(
        staking.set_early_withdrawal_penalty(200),
        Err(Error::Unauthorized)
    );
}

#[ink::test]
fn set_early_withdrawal_penalty_max_cap() {
    let mut staking = create_staking();

    // Above 50% cap is rejected
    assert_eq!(
        staking.set_early_withdrawal_penalty(6_000),
        Err(Error::InvalidConfig)
    );

    // Exactly at cap is fine
    assert!(staking.set_early_withdrawal_penalty(5_000).is_ok());
}
    fn staking_tiers_applied_correctly() {
        let mut staking = create_staking();
        let accounts = default_accounts();
        
        // Bob stakes Bronze amount (< 10_000)
        set_caller(accounts.bob);
        staking.stake(5_000, LockPeriod::Flexible).unwrap();
        assert_eq!(staking.get_staker_tier(accounts.bob), StakingTier::Bronze);

        // Charlie stakes Silver amount (>= 10_000)
        set_caller(accounts.charlie);
        staking.stake(15_000, LockPeriod::Flexible).unwrap();
        assert_eq!(staking.get_staker_tier(accounts.charlie), StakingTier::Silver);

        // Django stakes Gold amount (>= 50_000)
        let django = accounts.django;
        set_caller(django);
        staking.stake(55_000, LockPeriod::Flexible).unwrap();
        assert_eq!(staking.get_staker_tier(django), StakingTier::Gold);

        // Verify tier name and multiplier
        assert_eq!(StakingTier::Bronze.name(), "Bronze");
        assert_eq!(StakingTier::Bronze.reward_multiplier(), 100);
        assert_eq!(StakingTier::Silver.reward_multiplier(), 110);
        assert_eq!(StakingTier::Gold.reward_multiplier(), 120);
        assert_eq!(StakingTier::Platinum.reward_multiplier(), 135);
        assert_eq!(StakingTier::Diamond.reward_multiplier(), 150);
    }

    // ---- Vesting Schedule Tests ----

    #[ink::test]
    fn stake_with_vesting_succeeds() {
        let mut staking = create_staking();
        let accounts = default_accounts();

        // Fund the reward pool
        set_caller(accounts.alice);
        staking.fund_reward_pool(1_000_000_000).unwrap();

        // Create a stake with vesting
        set_caller(accounts.bob);
        assert!(staking
            .stake_with_vesting(10_000, LockPeriod::Flexible, 500_000, 1_000, 2_000)
            .is_ok());

        let stake = staking.get_stake(accounts.bob).unwrap();
        assert_eq!(stake.amount, 10_000);
        assert!(stake.vesting_schedule.is_some());

        let vesting = stake.vesting_schedule.unwrap();
        assert_eq!(vesting.total_amount, 500_000);
        assert_eq!(vesting.vested_amount, 0);
    }

    #[ink::test]
    fn stake_with_vesting_zero_reward_fails() {
        let mut staking = create_staking();
        let accounts = default_accounts();

        set_caller(accounts.alice);
        staking.fund_reward_pool(1_000_000_000).unwrap();

        set_caller(accounts.bob);
        assert_eq!(
            staking.stake_with_vesting(10_000, LockPeriod::Flexible, 0, 1_000, 2_000),
            Err(Error::ZeroAmount)
        );
    }

    #[ink::test]
    fn stake_with_vesting_insufficient_pool_fails() {
        let mut staking = create_staking();
        let accounts = default_accounts();

        set_caller(accounts.alice);
        staking.fund_reward_pool(100_000).unwrap();

        set_caller(accounts.bob);
        assert_eq!(
            staking.stake_with_vesting(10_000, LockPeriod::Flexible, 500_000, 1_000, 2_000),
            Err(Error::InsufficientPool)
        );
    }

    #[ink::test]
    fn stake_with_vesting_zero_vesting_blocks_fails() {
        let mut staking = create_staking();
        let accounts = default_accounts();

        set_caller(accounts.alice);
        staking.fund_reward_pool(1_000_000_000).unwrap();

        set_caller(accounts.bob);
        assert_eq!(
            staking.stake_with_vesting(10_000, LockPeriod::Flexible, 500_000, 1_000, 0),
            Err(Error::InvalidConfig)
        );
    }

    #[ink::test]
    fn vesting_zero_before_cliff() {
        let mut staking = create_staking();
        let accounts = default_accounts();

        set_caller(accounts.alice);
        staking.fund_reward_pool(1_000_000_000).unwrap();

        set_caller(accounts.bob);
        staking
            .stake_with_vesting(10_000, LockPeriod::Flexible, 500_000, 1_000, 2_000)
            .unwrap();

        // At block 0, vested amount should be 0 (cliff is at block 1_000 + start_block)
        let vested = staking.get_vested_amount(accounts.bob);
        assert_eq!(vested, 0);

        let unvested = staking.get_unvested_amount(accounts.bob);
        assert_eq!(unvested, 500_000);

        let claimable = staking.get_claimable_vested_amount(accounts.bob);
        assert_eq!(claimable, 0);
    }

    #[ink::test]
    fn vesting_full_after_end_block() {
        let mut staking = create_staking();
        let accounts = default_accounts();

        set_caller(accounts.alice);
        staking.fund_reward_pool(1_000_000_000).unwrap();

        set_caller(accounts.bob);
        staking
            .stake_with_vesting(10_000, LockPeriod::Flexible, 500_000, 100, 200)
            .unwrap();

        // Advance past the end block
        advance_block(400);

        let vested = staking.get_vested_amount(accounts.bob);
        assert_eq!(vested, 500_000);

        let unvested = staking.get_unvested_amount(accounts.bob);
        assert_eq!(unvested, 0);

        let claimable = staking.get_claimable_vested_amount(accounts.bob);
        assert_eq!(claimable, 500_000);
    }

    #[ink::test]
    fn vesting_linear_between_cliff_and_end() {
        let mut staking = create_staking();
        let accounts = default_accounts();

        set_caller(accounts.alice);
        staking.fund_reward_pool(1_000_000_000).unwrap();

        set_caller(accounts.bob);
        staking
            .stake_with_vesting(10_000, LockPeriod::Flexible, 1_000_000, 100, 200)
            .unwrap();

        // At cliff block (100), vesting starts
        advance_block(100);
        let vested_at_cliff = staking.get_vested_amount(accounts.bob);
        assert_eq!(vested_at_cliff, 0); // Still at cliff, no vesting yet

        // Halfway through vesting (block 150, mid-point between 100 and 300)
        advance_block(50);
        let vested_midpoint = staking.get_vested_amount(accounts.bob);
        assert!(vested_midpoint > 0);
        assert!(vested_midpoint < 1_000_000);
        // Should be approximately 50% of 1_000_000
        assert!(vested_midpoint >= 450_000 && vested_midpoint <= 550_000);
    }

    #[ink::test]
    fn no_rewards_claimable_before_cliff() {
        let mut staking = create_staking();
        let accounts = default_accounts();

        set_caller(accounts.alice);
        staking.fund_reward_pool(1_000_000_000).unwrap();

        set_caller(accounts.bob);
        staking
            .stake_with_vesting(10_000, LockPeriod::Flexible, 500_000, 1_000, 2_000)
            .unwrap();

        // Try to claim before cliff block is reached
        assert_eq!(staking.claim_rewards(), Err(Error::NoRewards));
    }

    #[ink::test]
    fn full_rewards_claimable_after_end_block() {
        let mut staking = create_staking();
        let accounts = default_accounts();

        set_caller(accounts.alice);
        staking.fund_reward_pool(1_000_000_000).unwrap();

        set_caller(accounts.bob);
        staking
            .stake_with_vesting(10_000, LockPeriod::Flexible, 500_000, 100, 200)
            .unwrap();

        // Advance past end block
        advance_block(350);

        let claimed = staking.claim_rewards().unwrap();
        assert_eq!(claimed, 500_000);

        let stake = staking.get_stake(accounts.bob).unwrap();
        let vesting = stake.vesting_schedule.unwrap();
        assert_eq!(vesting.vested_amount, 500_000);
    }

    #[ink::test]
    fn partial_rewards_during_vesting_period() {
        let mut staking = create_staking();
        let accounts = default_accounts();

        set_caller(accounts.alice);
        staking.fund_reward_pool(1_000_000_000).unwrap();

        set_caller(accounts.bob);
        staking
            .stake_with_vesting(10_000, LockPeriod::Flexible, 1_000_000, 100, 200)
            .unwrap();

        // Advance to halfway through vesting (block 150, assuming start at 0)
        advance_block(150);

        let claimable = staking.get_claimable_vested_amount(accounts.bob);
        assert!(claimable > 0);
        assert!(claimable < 1_000_000);

        let claimed = staking.claim_rewards().unwrap();
        assert_eq!(claimed, claimable);

        // Verify vested_amount was updated
        let stake = staking.get_stake(accounts.bob).unwrap();
        let vesting = stake.vesting_schedule.unwrap();
        assert_eq!(vesting.vested_amount, claimed);

        // Advance to end and claim remaining
        advance_block(100);
        let remaining = staking.claim_rewards().unwrap();
        assert!(remaining > 0);
        assert_eq!(remaining + claimed, 1_000_000);
    }

    #[ink::test]
    fn vesting_no_rewards_if_already_claimed() {
        let mut staking = create_staking();
        let accounts = default_accounts();

        set_caller(accounts.alice);
        staking.fund_reward_pool(1_000_000_000).unwrap();

        set_caller(accounts.bob);
        staking
            .stake_with_vesting(10_000, LockPeriod::Flexible, 500_000, 50, 100)
            .unwrap();

        // Advance past end block and claim all
        advance_block(200);
        let first_claim = staking.claim_rewards().unwrap();
        assert_eq!(first_claim, 500_000);

        // Try to claim again without new vesting
        let result = staking.claim_rewards();
        assert_eq!(result, Err(Error::NoRewards));
    }

    #[ink::test]
    fn unstake_returns_unvested_to_pool() {
        let mut staking = create_staking();
        let accounts = default_accounts();

        let initial_pool = 1_000_000_000u128;
        set_caller(accounts.alice);
        staking.fund_reward_pool(initial_pool).unwrap();

        set_caller(accounts.bob);
        staking
            .stake_with_vesting(10_000, LockPeriod::Flexible, 500_000, 1_000, 2_000)
            .unwrap();

        let pool_after_stake = staking.get_reward_pool();
        assert_eq!(pool_after_stake, initial_pool - 500_000);

        // Unstake before vesting is complete
        staking.unstake().unwrap();

        let final_pool = staking.get_reward_pool();
        // Unvested amount (500_000) should be returned to pool
        assert_eq!(final_pool, initial_pool);
    }

    #[ink::test]
    fn vesting_schedule_struct_calculations() {
        let vesting = VestingSchedule {
            total_amount: 1_000,
            vested_amount: 0,
            start_block: 0,
            cliff_block: 100,
            end_block: 300,
        };

        // Before cliff: 0 vested
        assert_eq!(vesting.calculate_vested_at_block(50), 0);

        // At cliff: still 0 vested
        assert_eq!(vesting.calculate_vested_at_block(100), 0);

        // Halfway: ~500 vested
        assert_eq!(vesting.calculate_vested_at_block(200), 500);

        // At end: full amount
        assert_eq!(vesting.calculate_vested_at_block(300), 1_000);

        // After end: still full amount
        assert_eq!(vesting.calculate_vested_at_block(500), 1_000);

        // Claimable when vested_amount = 0
        assert_eq!(vesting.claimable_at_block(200), 500);

        // After claiming 500
        let mut vesting_after_claim = vesting;
        vesting_after_claim.vested_amount = 500;
        assert_eq!(vesting_after_claim.claimable_at_block(200), 0);
        assert_eq!(vesting_after_claim.claimable_at_block(300), 500);
    }
}

