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
        ink::env::test::set_block_number::<ink::env::DefaultEnvironment>(0);
        let mut staking = Staking::new(500, 1_000);
        staking.set_slashing_coordinator(accounts.alice).unwrap();
        staking
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
    fn set_slashing_coordinator_non_admin_fails() {
        let accounts = default_accounts();
        set_caller(accounts.alice);
        let mut staking = Staking::new(500, 1_000);

        // A non-admin caller cannot configure who is allowed to slash.
        set_caller(accounts.bob);
        assert_eq!(
            staking.set_slashing_coordinator(accounts.bob),
            Err(Error::Unauthorized)
        );

        // Admin can still set it afterwards, and the coordinator can then slash.
        set_caller(accounts.alice);
        staking.set_slashing_coordinator(accounts.bob).unwrap();

        set_caller(accounts.bob);
        staking.register_validator(MIN_VALIDATOR_STAKE, 500).unwrap();
        staking.slash_validator(accounts.bob).unwrap();
        let info = staking.get_validator_info(accounts.bob).unwrap();
        let expected = MIN_VALIDATOR_STAKE * (100 - SLASH_PERCENT) / 100;
        assert_eq!(info.self_stake, expected);
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
    fn unstake_locked_applies_penalty() {
        let mut staking = create_staking();
        let accounts = default_accounts();
        set_caller(accounts.bob);
        staking.stake(10_000, LockPeriod::ThirtyDays).unwrap();
        // Unstake early - should succeed (apply penalty instead of blocking)
        let result = staking.unstake();
        assert!(result.is_ok());
        assert_eq!(staking.get_total_staked(), 0);
        assert!(staking.get_stake(accounts.bob).is_none());
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
        
        let stake_info = staking.get_stake(accounts.bob).unwrap();        assert!(!stake_info.auto_compound);

        staking.set_auto_compound(true).unwrap();
        let stake_info = staking.get_stake(accounts.bob).unwrap();
        assert!(stake_info.auto_compound);
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

#[ink::test]
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

        // Halfway through vesting (block 200, mid-point between 100 and 300)
        advance_block(100);
        let vested_midpoint = staking.get_vested_amount(accounts.bob);
        assert!(vested_midpoint > 0);
        assert!(vested_midpoint < 1_000_000);
        // Should be approximately 50% of 1_000_000
        assert!((450_000..=550_000).contains(&vested_midpoint));
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

        // Advance to halfway through vesting (block 200, halfway between 100 and 300)
        advance_block(200);

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

    // =========================================================================
    // Unbonding boundary + early-withdrawal penalty math (Issue #1000)
    // =========================================================================

    #[ink::test]
    fn claim_undelegated_exactly_at_boundary_succeeds() {
        let mut staking = create_staking();
        let accounts = default_accounts();
        set_caller(accounts.bob);
        staking.register_validator(MIN_VALIDATOR_STAKE, 500).unwrap();

        set_caller(accounts.charlie);
        staking.delegate(accounts.bob, 5_000).unwrap();
        staking.undelegate(accounts.bob).unwrap();

        // The check is `now < start + UNBONDING_PERIOD_BLOCKS`, so claiming
        // at exactly the boundary block must succeed (off-by-one guard).
        advance_block(UNBONDING_PERIOD_BLOCKS as u32);

        let amount = staking.claim_undelegated(accounts.bob).unwrap();
        assert_eq!(amount, 5_000);
        assert!(staking.get_delegation(accounts.charlie, accounts.bob).is_none());
    }

    #[ink::test]
    fn claim_undelegated_one_block_before_boundary_fails() {
        let mut staking = create_staking();
        let accounts = default_accounts();
        set_caller(accounts.bob);
        staking.register_validator(MIN_VALIDATOR_STAKE, 500).unwrap();

        set_caller(accounts.charlie);
        staking.delegate(accounts.bob, 5_000).unwrap();
        staking.undelegate(accounts.bob).unwrap();

        advance_block(UNBONDING_PERIOD_BLOCKS as u32 - 1);
        assert_eq!(
            staking.claim_undelegated(accounts.bob),
            Err(Error::UnbondingPeriodActive)
        );

        // The very next block crosses the boundary and succeeds.
        advance_block(1);
        assert_eq!(staking.claim_undelegated(accounts.bob).unwrap(), 5_000);
    }

    #[ink::test]
    fn unstake_locked_penalty_exact_math_on_non_round_amount() {
        let mut staking = create_staking();
        let accounts = default_accounts();

        // Non-default penalty keeps the test independent of the constant.
        set_caller(accounts.alice);
        staking.set_early_withdrawal_penalty(333).unwrap();

        set_caller(accounts.bob);
        staking.stake(12_345, LockPeriod::ThirtyDays).unwrap();

        let pool_before = staking.get_reward_pool();

        // Immediate unstake: penalty = 12_345 * 333 / 10_000 = 411.0885,
        // truncated to 411 by the integer division. A formula regression
        // (e.g. rounding up or swapping mul/div order) changes this value.
        staking.unstake().unwrap();
        let expected_penalty = 12_345u128 * 333 / 10_000;
        assert_eq!(expected_penalty, 411);
        assert_eq!(
            staking.get_reward_pool(),
            pool_before + expected_penalty
        );
        assert_eq!(staking.get_total_staked(), 0);
        assert!(staking.get_stake(accounts.bob).is_none());
    }

    #[ink::test]
    fn unstake_locked_default_penalty_is_ten_percent() {
        let mut staking = create_staking();
        let accounts = default_accounts();
        set_caller(accounts.bob);

        assert_eq!(staking.get_early_withdrawal_penalty_bps(), 1_000);
        staking.stake(9_999, LockPeriod::ThirtyDays).unwrap();

        let pool_before = staking.get_reward_pool();
        staking.unstake().unwrap();
        // 9_999 * 1_000 / 10_000 = 999.9 -> 999 retained in the pool.
        assert_eq!(staking.get_reward_pool() - pool_before, 999);
    }
    // =========================================================================
    // Reward Math: accrual over blocks, validator commission, reinvestment
    //
    // The assertions below pin exact numbers rather than `> 0`. Reward
    // distribution is money math: a wrong commission ordering or a rounding
    // slip silently moves value between the validator and its delegators, and
    // an inequality assertion cannot see that.
    // =========================================================================

    /// Reference implementation of the gross (pre-commission) reward the
    /// contract accrues for `amount` over `blocks`, mirroring
    /// `update_validator_rewards` / `calculate_rewards` term by term.
    fn gross_reward(amount: u128, reward_rate_bps: u128, blocks: u128) -> u128 {
        amount.saturating_mul(reward_rate_bps).saturating_mul(blocks)
            / constants::REWARD_RATE_PRECISION
            / 5_256_000
    }

    /// Fixed inputs shared by the reward-math tests: 1e15 delegated at the
    /// default 5% (500 bps) annual rate for 100_000 blocks.
    const REWARD_TEST_STAKE: u128 = 1_000_000_000_000_000;
    const REWARD_TEST_BLOCKS: u32 = 100_000;
    /// Pool large enough that no test below can hit `InsufficientPool`.
    const REWARD_TEST_POOL: u128 = 10_000_000_000_000_000;

    fn fund_pool(staking: &mut Staking, amount: u128) {
        let accounts = default_accounts();
        set_caller(accounts.alice);
        staking.fund_reward_pool(amount).unwrap();
    }

    // ---- Reward accrual over blocks ----

    #[ink::test]
    fn delegation_reward_accrues_exactly_over_blocks() {
        let mut staking = create_staking();
        let accounts = default_accounts();
        fund_pool(&mut staking, REWARD_TEST_POOL);

        // 0% commission isolates the accrual term from the commission split.
        set_caller(accounts.bob);
        staking.register_validator(MIN_VALIDATOR_STAKE, 0).unwrap();

        set_caller(accounts.charlie);
        staking.delegate(accounts.bob, REWARD_TEST_STAKE).unwrap();

        // gross = 1e15 * 500 * 100_000 / 10_000 / 5_256_000
        let expected_gross = gross_reward(REWARD_TEST_STAKE, 500, REWARD_TEST_BLOCKS as u128);
        assert_eq!(expected_gross, 951_293_759_512);

        advance_block(REWARD_TEST_BLOCKS);

        // acc_reward_per_share = net * 1e12 / 1e15, then reward = 1e15 * acc / 1e12,
        // so the delegator sees the gross truncated to per-share precision.
        assert_eq!(
            staking.get_pending_delegation_rewards(accounts.charlie, accounts.bob),
            951_293_759_000
        );

        let pool_before = staking.get_reward_pool();
        let claimed = staking.claim_delegation_rewards(accounts.bob).unwrap();
        assert_eq!(claimed, 951_293_759_000);
        assert_eq!(staking.get_reward_pool(), pool_before - claimed);
        assert_eq!(
            staking.get_pending_delegation_rewards(accounts.charlie, accounts.bob),
            0
        );
    }

    #[ink::test]
    fn delegation_reward_accrual_is_linear_in_blocks() {
        let mut staking = create_staking();
        let accounts = default_accounts();
        fund_pool(&mut staking, REWARD_TEST_POOL);

        set_caller(accounts.bob);
        staking.register_validator(MIN_VALIDATOR_STAKE, 0).unwrap();

        set_caller(accounts.charlie);
        staking.delegate(accounts.bob, REWARD_TEST_STAKE).unwrap();

        advance_block(50_000);
        let half = staking.get_pending_delegation_rewards(accounts.charlie, accounts.bob);
        assert_eq!(half, 475_646_879_000);

        advance_block(50_000);
        let full = staking.get_pending_delegation_rewards(accounts.charlie, accounts.bob);
        assert_eq!(full, 951_293_759_000);
        // Accrual is block-linear: doubling the window doubles the reward, up
        // to one unit of per-share truncation (the 100_000-block projection
        // truncates once, the 50_000-block one twice).
        assert_eq!(full - half, 475_646_880_000);
        assert_eq!(full - half * 2, 1_000);
    }

    #[ink::test]
    fn delegation_reward_is_zero_without_block_progress() {
        let mut staking = create_staking();
        let accounts = default_accounts();
        fund_pool(&mut staking, REWARD_TEST_POOL);

        set_caller(accounts.bob);
        staking.register_validator(MIN_VALIDATOR_STAKE, 0).unwrap();

        set_caller(accounts.charlie);
        staking.delegate(accounts.bob, REWARD_TEST_STAKE).unwrap();

        assert_eq!(
            staking.get_pending_delegation_rewards(accounts.charlie, accounts.bob),
            0
        );
        assert_eq!(
            staking.claim_delegation_rewards(accounts.bob),
            Err(Error::NoRewards)
        );
    }

    // ---- Validator commission: the split is pinned with fixed numbers ----

    #[ink::test]
    fn commission_splits_gross_reward_between_validator_and_delegator() {
        let mut staking = create_staking();
        let accounts = default_accounts();
        fund_pool(&mut staking, REWARD_TEST_POOL);

        // 10% commission.
        set_caller(accounts.bob);
        staking.register_validator(MIN_VALIDATOR_STAKE, 1_000).unwrap();

        set_caller(accounts.charlie);
        staking.delegate(accounts.bob, REWARD_TEST_STAKE).unwrap();

        advance_block(REWARD_TEST_BLOCKS);

        // gross            = 951_293_759_512
        // commission (10%) =  95_129_375_951   (gross * 1_000 / 10_000, truncated)
        // net              = 856_164_383_561   (gross - commission)
        let gross = gross_reward(REWARD_TEST_STAKE, 500, REWARD_TEST_BLOCKS as u128);
        assert_eq!(gross, 951_293_759_512);

        let delegator_reward = staking.claim_delegation_rewards(accounts.bob).unwrap();
        assert_eq!(delegator_reward, 856_164_383_000);

        set_caller(accounts.bob);
        let commission = staking.claim_validator_commission().unwrap();
        assert_eq!(commission, 95_129_375_951);

        // Commission is taken OUT of the gross reward, not added on top of the
        // delegator's share: the two payouts must sum to at most the gross.
        // (The 561-unit shortfall is per-share truncation, left in the pool.)
        assert_eq!(delegator_reward + commission, gross - 561);
        assert!(delegator_reward + commission <= gross);
    }

    /// Runs the same fixed inputs at one commission rate and pins the exact
    /// (delegator, validator) split. Each rate needs its own `#[ink::test]`:
    /// the off-chain env shares one storage backend per test, so a second
    /// `Staking` instance in the same test would see the first one's state.
    fn assert_commission_split(rate: u32, expected_delegator: u128, expected_commission: u128) {
        let mut staking = create_staking();
        let accounts = default_accounts();
        fund_pool(&mut staking, REWARD_TEST_POOL);

        set_caller(accounts.bob);
        staking.register_validator(MIN_VALIDATOR_STAKE, rate).unwrap();

        set_caller(accounts.charlie);
        staking.delegate(accounts.bob, REWARD_TEST_STAKE).unwrap();
        advance_block(REWARD_TEST_BLOCKS);

        let delegator_reward = staking.claim_delegation_rewards(accounts.bob).unwrap();
        assert_eq!(
            delegator_reward, expected_delegator,
            "delegator share wrong at {} bps commission",
            rate
        );

        let commission = staking
            .get_validator_info(accounts.bob)
            .unwrap()
            .accumulated_commission;
        assert_eq!(
            commission, expected_commission,
            "validator commission wrong at {} bps",
            rate
        );
    }

    #[ink::test]
    fn commission_zero_bps_gives_delegator_the_whole_reward() {
        assert_commission_split(0, 951_293_759_000, 0);
    }

    #[ink::test]
    fn commission_1000_bps_takes_a_tenth() {
        assert_commission_split(1_000, 856_164_383_000, 95_129_375_951);
    }

    #[ink::test]
    fn commission_2000_bps_takes_a_fifth() {
        assert_commission_split(2_000, 761_035_007_000, 190_258_751_902);
    }

    #[ink::test]
    fn full_commission_leaves_delegator_with_nothing() {
        let mut staking = create_staking();
        let accounts = default_accounts();
        fund_pool(&mut staking, REWARD_TEST_POOL);

        set_caller(accounts.bob);
        staking
            .register_validator(MIN_VALIDATOR_STAKE, MAX_COMMISSION_RATE)
            .unwrap();

        set_caller(accounts.charlie);
        staking.delegate(accounts.bob, REWARD_TEST_STAKE).unwrap();
        advance_block(REWARD_TEST_BLOCKS);

        assert_eq!(
            staking.get_pending_delegation_rewards(accounts.charlie, accounts.bob),
            0
        );
        assert_eq!(
            staking.claim_delegation_rewards(accounts.bob),
            Err(Error::NoRewards)
        );

        set_caller(accounts.bob);
        // The validator takes the whole gross reward.
        assert_eq!(
            staking.claim_validator_commission().unwrap(),
            gross_reward(REWARD_TEST_STAKE, 500, REWARD_TEST_BLOCKS as u128)
        );
    }

    #[ink::test]
    fn commission_is_split_pro_rata_across_delegators() {
        let mut staking = create_staking();
        let accounts = default_accounts();
        fund_pool(&mut staking, REWARD_TEST_POOL);

        set_caller(accounts.bob);
        staking.register_validator(MIN_VALIDATOR_STAKE, 1_000).unwrap();

        // Both delegate at block 0, so both accrue over the same window.
        set_caller(accounts.charlie);
        staking.delegate(accounts.bob, REWARD_TEST_STAKE).unwrap();
        set_caller(accounts.django);
        staking.delegate(accounts.bob, REWARD_TEST_STAKE * 3).unwrap();

        advance_block(REWARD_TEST_BLOCKS);

        // gross now accrues on 4e15 delegated:
        //   gross      = 3_805_175_038_051
        //   commission =   380_517_503_805
        //   net        = 3_424_657_534_246
        //   acc/share  = net * 1e12 / 4e15 = 856_164_383
        let charlie = staking.get_pending_delegation_rewards(accounts.charlie, accounts.bob);
        let django = staking.get_pending_delegation_rewards(accounts.django, accounts.bob);
        assert_eq!(charlie, 856_164_383_000);
        assert_eq!(django, 2_568_493_149_000);
        assert_eq!(django, charlie * 3);
    }

    // ---- update_commission_rate settles the old rate before switching ----

    #[ink::test]
    fn commission_rate_change_is_not_retroactive() {
        let mut staking = create_staking();
        let accounts = default_accounts();
        fund_pool(&mut staking, REWARD_TEST_POOL);

        set_caller(accounts.bob);
        staking.register_validator(MIN_VALIDATOR_STAKE, 1_000).unwrap();

        set_caller(accounts.charlie);
        staking.delegate(accounts.bob, REWARD_TEST_STAKE).unwrap();

        // Window 1: 100_000 blocks at 10%.
        advance_block(REWARD_TEST_BLOCKS);
        set_caller(accounts.bob);
        staking.update_commission_rate(2_000).unwrap();

        // The rate change settles the first window at the OLD rate.
        let info = staking.get_validator_info(accounts.bob).unwrap();
        assert_eq!(info.commission_rate, 2_000);
        assert_eq!(info.accumulated_commission, 95_129_375_951);

        // Window 2: another 100_000 blocks, now at 20%.
        advance_block(REWARD_TEST_BLOCKS);

        set_caller(accounts.charlie);
        let delegator_reward = staking.claim_delegation_rewards(accounts.bob).unwrap();
        // 856_164_383_000 (window 1 @10%) + 761_035_007_000 (window 2 @20%)
        assert_eq!(delegator_reward, 1_617_199_390_000);

        set_caller(accounts.bob);
        let commission = staking.claim_validator_commission().unwrap();
        // 95_129_375_951 (window 1) + 190_258_751_902 (window 2)
        assert_eq!(commission, 285_388_127_853);
    }

    #[ink::test]
    fn commission_rate_change_emits_old_and_new_rate() {
        let mut staking = create_staking();
        let accounts = default_accounts();
        set_caller(accounts.bob);
        staking.register_validator(MIN_VALIDATOR_STAKE, 500).unwrap();

        staking.update_commission_rate(2_500).unwrap();

        let events = ink::env::test::recorded_events().collect::<Vec<_>>();
        let updated = <CommissionRateUpdated as scale::Decode>::decode(
            &mut &events[events.len() - 1].data[..],
        )
        .expect("decode CommissionRateUpdated");
        assert_eq!(updated.validator, accounts.bob);
        assert_eq!(updated.old_rate, 500);
        assert_eq!(updated.new_rate, 2_500);
    }

    // ---- Reinvestment (auto-compound) flag path ----

    #[ink::test]
    fn reinvestment_flag_compounds_rewards_into_the_stake() {
        let mut staking = create_staking();
        let accounts = default_accounts();
        fund_pool(&mut staking, REWARD_TEST_POOL);

        set_caller(accounts.bob);
        staking
            .stake(REWARD_TEST_STAKE, LockPeriod::Flexible)
            .unwrap();
        staking.set_auto_compound(true).unwrap();

        advance_block(REWARD_TEST_BLOCKS);

        // base   = 951_293_759_512 (as above)
        // × 100/100 flexible multiplier, × 150/100 Diamond tier
        //        = 1_426_940_639_268
        const EXPECTED: u128 = 1_426_940_639_268;
        assert_eq!(staking.get_pending_rewards(accounts.bob), EXPECTED);

        let pool_before = staking.get_reward_pool();
        let total_staked_before = staking.get_total_staked();
        let power_before = staking.get_governance_power(accounts.bob);

        assert_eq!(staking.claim_rewards().unwrap(), EXPECTED);

        // The reward is added to the stake instead of being paid out.
        let stake = staking.get_stake(accounts.bob).unwrap();
        assert_eq!(stake.amount, REWARD_TEST_STAKE + EXPECTED);
        assert_eq!(staking.get_total_staked(), total_staked_before + EXPECTED);
        assert_eq!(
            staking.get_governance_power(accounts.bob),
            power_before + EXPECTED
        );
        assert_eq!(staking.get_reward_pool(), pool_before - EXPECTED);

        // The accrual clock is reset, so nothing is claimable twice.
        assert_eq!(staking.get_pending_rewards(accounts.bob), 0);
        assert_eq!(staking.claim_rewards(), Err(Error::NoRewards));

        let events = ink::env::test::recorded_events().collect::<Vec<_>>();
        let reinvested =
            <RewardsReinvested as scale::Decode>::decode(&mut &events[events.len() - 1].data[..])
                .expect("decode RewardsReinvested");
        assert_eq!(reinvested.staker, accounts.bob);
        assert_eq!(reinvested.amount, EXPECTED);
    }

    #[ink::test]
    fn reinvestment_flag_off_pays_out_instead_of_compounding() {
        let mut staking = create_staking();
        let accounts = default_accounts();
        fund_pool(&mut staking, REWARD_TEST_POOL);

        set_caller(accounts.bob);
        staking
            .stake(REWARD_TEST_STAKE, LockPeriod::Flexible)
            .unwrap();
        assert!(!staking.get_stake(accounts.bob).unwrap().auto_compound);

        advance_block(REWARD_TEST_BLOCKS);

        const EXPECTED: u128 = 1_426_940_639_268;
        let total_staked_before = staking.get_total_staked();
        let power_before = staking.get_governance_power(accounts.bob);

        assert_eq!(staking.claim_rewards().unwrap(), EXPECTED);

        // Same amount leaves the pool, but the stake is untouched.
        let stake = staking.get_stake(accounts.bob).unwrap();
        assert_eq!(stake.amount, REWARD_TEST_STAKE);
        assert_eq!(staking.get_total_staked(), total_staked_before);
        assert_eq!(staking.get_governance_power(accounts.bob), power_before);

        let events = ink::env::test::recorded_events().collect::<Vec<_>>();
        let claimed =
            <RewardsClaimed as scale::Decode>::decode(&mut &events[events.len() - 1].data[..])
                .expect("decode RewardsClaimed");
        assert_eq!(claimed.staker, accounts.bob);
        assert_eq!(claimed.amount, EXPECTED);
    }

    #[ink::test]
    fn reinvestment_compounds_on_the_grown_principal() {
        let mut staking = create_staking();
        let accounts = default_accounts();
        fund_pool(&mut staking, REWARD_TEST_POOL);

        set_caller(accounts.bob);
        staking
            .stake(REWARD_TEST_STAKE, LockPeriod::Flexible)
            .unwrap();
        staking.set_auto_compound(true).unwrap();

        advance_block(REWARD_TEST_BLOCKS);
        let first = staking.claim_rewards().unwrap();
        assert_eq!(first, 1_426_940_639_268);

        // Second identical window now accrues on principal + first reward.
        advance_block(REWARD_TEST_BLOCKS);
        let second = staking.claim_rewards().unwrap();
        let grown = REWARD_TEST_STAKE + first;
        let expected_second =
            gross_reward(grown, 500, REWARD_TEST_BLOCKS as u128) * 100 / 100 * 150 / 100;
        assert_eq!(second, expected_second);
        assert!(second > first, "compounding must grow the payout");

        assert_eq!(
            staking.get_stake(accounts.bob).unwrap().amount,
            grown + second
        );
    }

    #[ink::test]
    fn reinvestment_credits_the_governance_delegate() {
        let mut staking = create_staking();
        let accounts = default_accounts();
        fund_pool(&mut staking, REWARD_TEST_POOL);

        set_caller(accounts.bob);
        staking
            .stake(REWARD_TEST_STAKE, LockPeriod::Flexible)
            .unwrap();
        staking.set_auto_compound(true).unwrap();
        staking.delegate_governance(accounts.charlie).unwrap();

        assert_eq!(staking.get_governance_power(accounts.bob), 0);
        assert_eq!(
            staking.get_governance_power(accounts.charlie),
            REWARD_TEST_STAKE
        );

        advance_block(REWARD_TEST_BLOCKS);
        let reward = staking.claim_rewards().unwrap();

        // Compounded rewards follow the delegation, not the staker.
        assert_eq!(staking.get_governance_power(accounts.bob), 0);
        assert_eq!(
            staking.get_governance_power(accounts.charlie),
            REWARD_TEST_STAKE + reward
        );
    }

    #[ink::test]
    fn reinvestment_flag_can_be_toggled_back_off() {
        let mut staking = create_staking();
        let accounts = default_accounts();
        fund_pool(&mut staking, REWARD_TEST_POOL);

        set_caller(accounts.bob);
        staking
            .stake(REWARD_TEST_STAKE, LockPeriod::Flexible)
            .unwrap();
        staking.set_auto_compound(true).unwrap();

        advance_block(REWARD_TEST_BLOCKS);
        let compounded = staking.claim_rewards().unwrap();
        let grown = REWARD_TEST_STAKE + compounded;
        assert_eq!(staking.get_stake(accounts.bob).unwrap().amount, grown);

        staking.set_auto_compound(false).unwrap();
        advance_block(REWARD_TEST_BLOCKS);
        staking.claim_rewards().unwrap();

        // With the flag off the principal stops growing.
        assert_eq!(staking.get_stake(accounts.bob).unwrap().amount, grown);
    }

    #[ink::test]
    fn reinvestment_respects_the_reward_pool_ceiling() {
        let mut staking = create_staking();
        let accounts = default_accounts();
        // Fund one unit less than the reward the window will produce.
        fund_pool(&mut staking, 1_426_940_639_267);

        set_caller(accounts.bob);
        staking
            .stake(REWARD_TEST_STAKE, LockPeriod::Flexible)
            .unwrap();
        staking.set_auto_compound(true).unwrap();

        advance_block(REWARD_TEST_BLOCKS);

        assert_eq!(staking.claim_rewards(), Err(Error::InsufficientPool));
        // Nothing was compounded.
        assert_eq!(
            staking.get_stake(accounts.bob).unwrap().amount,
            REWARD_TEST_STAKE
        );
        assert_eq!(staking.get_reward_pool(), 1_426_940_639_267);
    }
}
