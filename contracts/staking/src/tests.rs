// Unit tests for the staking contract (Issue #101 - extracted from lib.rs)

#[cfg(test)]
mod tests {
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
        staking.fund_reward_pool(1_000_000_000_000).unwrap();

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
}
