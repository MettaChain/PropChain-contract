//! Integration tests for the staking-to-governance voting-power flow (#1521).
//!
//! The staking contract carries its own parameter-governance module: staking
//! grants `governance_power`, a holder may create a parameter proposal, votes
//! are weighted by that power, and a proposal that reaches quorum and a
//! majority is applied on execution. These tests drive that whole path inside
//! the real compiled contract rather than through a mock.
//!
//! Two properties of the shipped implementation are pinned explicitly because
//! they differ from a token-weighted DAO's usual assumptions:
//!
//! * `governance_power` equals the staked principal one-for-one. The lock
//!   period changes the *reward* multiplier only; it does not change voting
//!   power. `lock_period_changes_rewards_but_not_voting_power` states that.
//! * Pending rewards are not voting power. A claim that leaves the principal
//!   untouched does not move power; only auto-compounding does, because the
//!   reward then becomes part of the principal. `reward_accrual_does_not_...`
//!   covers both.

#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(test)]
mod tests {
    use ink::env::{test, DefaultEnvironment};
    use ink::primitives::AccountId;
    use staking::staking::{Error, LockPeriod, ParamKind, ProposalStatus, Staking};

    type Accounts = test::DefaultAccounts<DefaultEnvironment>;

    const RATE_BPS: u128 = 500;
    const MIN_STAKE: u128 = 1_000;
    /// `DEFAULT_VOTING_PERIOD_BLOCKS` in the contract.
    const VOTING_PERIOD: u64 = 28_800;

    fn setup() -> (Staking, Accounts) {
        let accounts = test::default_accounts::<DefaultEnvironment>();
        test::set_caller::<DefaultEnvironment>(accounts.alice);
        test::set_block_number::<DefaultEnvironment>(0);
        (Staking::new(RATE_BPS, MIN_STAKE), accounts)
    }

    fn at_block(n: u64) {
        test::set_block_number::<DefaultEnvironment>(n as u32);
    }

    fn stake_as(s: &mut Staking, who: AccountId, amount: u128, lock: LockPeriod) {
        test::set_caller::<DefaultEnvironment>(who);
        s.stake(amount, lock).expect("stake accepted");
    }

    fn propose_as(s: &mut Staking, who: AccountId, kind: ParamKind) -> u64 {
        test::set_caller::<DefaultEnvironment>(who);
        s.propose_param_change(kind).expect("proposal created")
    }

    fn vote_as(s: &mut Staking, who: AccountId, id: u64, support: bool) -> Result<(), Error> {
        test::set_caller::<DefaultEnvironment>(who);
        s.vote_on_proposal(id, support)
    }

    // ── Voting power assignment ─────────────────────────────────────────

    #[ink::test]
    fn staking_grants_voting_power_proportional_to_the_staked_amount() {
        let (mut s, accounts) = setup();

        stake_as(&mut s, accounts.bob, 10_000, LockPeriod::Flexible);
        stake_as(&mut s, accounts.charlie, 30_000, LockPeriod::OneYear);

        // One unit staked is one unit of voting power, per account.
        assert_eq!(s.get_governance_power(accounts.bob), 10_000);
        assert_eq!(s.get_governance_power(accounts.charlie), 30_000);
        // An account that never staked has none.
        assert_eq!(s.get_governance_power(accounts.eve), 0);
        // Proportional, not merely monotonic: charlie staked three times as
        // much and holds three times the power.
        assert_eq!(
            s.get_governance_power(accounts.charlie),
            3 * s.get_governance_power(accounts.bob)
        );
        assert_eq!(s.get_total_staked(), 40_000);
    }

    #[ink::test]
    fn lock_period_changes_rewards_but_not_voting_power() {
        let (mut s, accounts) = setup();

        // Same principal, very different lock periods.
        stake_as(&mut s, accounts.bob, 25_000_000, LockPeriod::Flexible);
        stake_as(&mut s, accounts.charlie, 25_000_000, LockPeriod::OneYear);

        // Voting power is amount-only (the contract's `stake` grants the raw
        // amount, and `LockPeriod` appears only in reward pricing).
        assert_eq!(s.get_governance_power(accounts.bob), 25_000_000);
        assert_eq!(
            s.get_governance_power(accounts.charlie),
            s.get_governance_power(accounts.bob)
        );

        // Lock duration is not lost, it is accounted for in rewards instead.
        let blocks = 1_000_000u64;
        let flexible = s.calculate_projected_rewards(25_000_000, LockPeriod::Flexible, blocks);
        let one_year = s.calculate_projected_rewards(25_000_000, LockPeriod::OneYear, blocks);
        assert!(one_year > flexible, "longer lock must still earn more");
    }

    #[ink::test]
    fn accounts_without_voting_power_cannot_propose_or_vote() {
        let (mut s, accounts) = setup();

        test::set_caller::<DefaultEnvironment>(accounts.bob);
        assert_eq!(
            s.propose_param_change(ParamKind::MinStake(2_000)),
            Err(Error::NoVotingPower)
        );
        assert_eq!(s.get_proposal_count(), 0);
        // The power check precedes the proposal lookup, so an id that does not
        // exist still reports the caller's lack of power.
        assert_eq!(s.vote_on_proposal(0, true), Err(Error::NoVotingPower));
    }

    // ── Proposal lifecycle ──────────────────────────────────────────────

    #[ink::test]
    fn a_staker_creates_and_votes_on_a_proposal_with_staked_power() {
        let (mut s, accounts) = setup();
        stake_as(&mut s, accounts.bob, 10_000, LockPeriod::Flexible);
        stake_as(&mut s, accounts.charlie, 30_000, LockPeriod::Flexible);

        let id = propose_as(&mut s, accounts.bob, ParamKind::MinStake(2_000));
        let proposal = s.get_param_proposal(id).expect("proposal stored");
        assert_eq!(id, 0);
        assert_eq!(proposal.proposer, accounts.bob);
        assert_eq!(proposal.status, ProposalStatus::Active);
        assert_eq!(proposal.voting_end, VOTING_PERIOD);
        // Quorum is measured against the total stake at creation time.
        assert_eq!(proposal.total_power_snapshot, 40_000);

        // Bob's ballot carries exactly his staked power.
        vote_as(&mut s, accounts.bob, id, true).unwrap();
        let proposal = s.get_param_proposal(id).unwrap();
        assert_eq!(proposal.votes_for, 10_000);
        assert_eq!(proposal.votes_against, 0);
        assert!(s.has_voted(id, accounts.bob));

        // One ballot per account.
        assert_eq!(
            vote_as(&mut s, accounts.bob, id, false),
            Err(Error::AlreadyVoted)
        );
        // A different staker weighs in with their own amount.
        vote_as(&mut s, accounts.charlie, id, false).unwrap();
        let proposal = s.get_param_proposal(id).unwrap();
        assert_eq!(proposal.votes_against, 30_000);
        assert_eq!(proposal.votes_for, 10_000);
    }

    #[ink::test]
    fn a_winning_vote_executes_the_parameter_change() {
        let (mut s, accounts) = setup();
        stake_as(&mut s, accounts.bob, 60_000, LockPeriod::Flexible);
        stake_as(&mut s, accounts.charlie, 10_000, LockPeriod::Flexible);
        stake_as(&mut s, accounts.django, 5_000, LockPeriod::Flexible);

        let id = propose_as(&mut s, accounts.bob, ParamKind::MinStake(2_000));
        vote_as(&mut s, accounts.bob, id, true).unwrap();
        vote_as(&mut s, accounts.charlie, id, false).unwrap();

        // The window is still open: a fresh ballot is accepted, but execution
        // must wait.
        at_block(VOTING_PERIOD - 1);
        vote_as(&mut s, accounts.django, id, true).unwrap();
        assert_eq!(s.execute_param_proposal(id), Err(Error::VotingActive));

        at_block(VOTING_PERIOD);
        s.execute_param_proposal(id).expect("proposal executed");
        assert_eq!(
            s.get_param_proposal(id).unwrap().status,
            ProposalStatus::Executed
        );
        // The applied effect is observable: the governance-set minimum stake
        // is now what the proposal carried.
        assert_eq!(s.get_min_stake(), 2_000);
    }

    #[ink::test]
    fn a_proposal_with_a_minority_for_vote_is_rejected_without_changing_parameters() {
        let (mut s, accounts) = setup();
        stake_as(&mut s, accounts.bob, 10_000, LockPeriod::Flexible);
        stake_as(&mut s, accounts.charlie, 30_000, LockPeriod::Flexible);

        let id = propose_as(&mut s, accounts.bob, ParamKind::MinStake(2_000));
        vote_as(&mut s, accounts.bob, id, true).unwrap();
        vote_as(&mut s, accounts.charlie, id, false).unwrap();

        at_block(VOTING_PERIOD);
        // Rejection is a successful finalisation, not an error.
        s.execute_param_proposal(id).unwrap();
        assert_eq!(
            s.get_param_proposal(id).unwrap().status,
            ProposalStatus::Rejected
        );
        assert_eq!(s.get_min_stake(), MIN_STAKE, "parameter must not change");
    }

    #[ink::test]
    fn votes_are_only_accepted_inside_the_voting_window() {
        let (mut s, accounts) = setup();
        stake_as(&mut s, accounts.bob, 10_000, LockPeriod::Flexible);
        let id = propose_as(&mut s, accounts.bob, ParamKind::MinStake(2_000));

        // Exactly at `voting_end` the window is closed (`now >= voting_end`).
        at_block(VOTING_PERIOD);
        assert_eq!(
            vote_as(&mut s, accounts.bob, id, true),
            Err(Error::VotingEnded)
        );
        assert_eq!(s.get_param_proposal(id).unwrap().votes_for, 0);
    }

    // ── Unstaking removes the old ballot weight ─────────────────────────

    #[ink::test]
    fn unstaking_removes_voting_power_and_the_old_power_cannot_vote() {
        let (mut s, accounts) = setup();
        stake_as(&mut s, accounts.bob, 10_000, LockPeriod::Flexible);
        let id = propose_as(&mut s, accounts.bob, ParamKind::MinStake(2_000));

        // Leaving a flexible stake is immediate and returns the principal.
        test::set_caller::<DefaultEnvironment>(accounts.bob);
        s.unstake().expect("flexible stake exits");
        assert_eq!(s.get_governance_power(accounts.bob), 0);
        assert_eq!(s.get_total_staked(), 0);
        assert!(s.get_stake(accounts.bob).is_none());

        // The proposal was created while the stake existed, but the old power
        // cannot be spent after it is gone.
        assert_eq!(
            vote_as(&mut s, accounts.bob, id, true),
            Err(Error::NoVotingPower)
        );
        assert_eq!(s.get_param_proposal(id).unwrap().votes_for, 0);

        // With no ballots the proposal cannot reach quorum.
        at_block(VOTING_PERIOD);
        assert_eq!(s.execute_param_proposal(id), Err(Error::QuorumNotReached));
        assert_eq!(
            s.get_param_proposal(id).unwrap().status,
            ProposalStatus::Rejected
        );
    }

    #[ink::test]
    fn one_stakers_exit_reduces_total_power_without_touching_anothers() {
        let (mut s, accounts) = setup();
        stake_as(&mut s, accounts.bob, 50_000, LockPeriod::Flexible);
        stake_as(&mut s, accounts.charlie, 20_000, LockPeriod::Flexible);
        assert_eq!(s.get_governance_power(accounts.bob), 50_000);
        assert_eq!(s.get_total_staked(), 70_000);

        // The contract only allows a full exit, so "reduce" here means the
        // exiting account drops to zero while the other staker keeps theirs.
        test::set_caller::<DefaultEnvironment>(accounts.bob);
        s.unstake().unwrap();
        assert_eq!(s.get_governance_power(accounts.bob), 0);
        assert_eq!(s.get_governance_power(accounts.charlie), 20_000);
        assert_eq!(s.get_total_staked(), 20_000);
    }

    // ── Delegation ──────────────────────────────────────────────────────

    #[ink::test]
    fn delegated_power_votes_on_behalf_of_the_delegator() {
        let (mut s, accounts) = setup();
        stake_as(&mut s, accounts.bob, 10_000, LockPeriod::Flexible);
        stake_as(&mut s, accounts.charlie, 10_000, LockPeriod::Flexible);

        test::set_caller::<DefaultEnvironment>(accounts.bob);
        s.delegate_governance(accounts.charlie)
            .expect("delegation accepted");

        // The delegator's power moves to the delegate.
        assert_eq!(s.get_governance_power(accounts.bob), 0);
        assert_eq!(s.get_governance_power(accounts.charlie), 20_000);
        // The delegator can no longer vote directly.
        assert_eq!(
            vote_as(&mut s, accounts.bob, 0, true),
            Err(Error::NoVotingPower)
        );

        let id = propose_as(&mut s, accounts.charlie, ParamKind::QuorumBps(2_000));
        vote_as(&mut s, accounts.charlie, id, true).unwrap();
        let proposal = s.get_param_proposal(id).unwrap();
        assert_eq!(proposal.votes_for, 20_000);

        at_block(VOTING_PERIOD);
        s.execute_param_proposal(id).unwrap();
        assert_eq!(
            s.get_param_proposal(id).unwrap().status,
            ProposalStatus::Executed
        );
        assert_eq!(s.get_voting_config().1, 2_000);
    }

    // ── Reward accounting is separate from voting power ─────────────────

    #[ink::test]
    fn reward_accrual_does_not_change_voting_power_until_it_is_compounded() {
        let (mut s, accounts) = setup();
        s.fund_reward_pool(1_000_000_000_000).unwrap();

        let principal = 10_000_000_000u128;
        at_block(10);
        stake_as(&mut s, accounts.bob, principal, LockPeriod::Flexible);
        assert_eq!(s.get_governance_power(accounts.bob), principal);

        // Time passes and rewards accrue. Power is principal-only, so it does
        // not move with the reward projection.
        at_block(10 + 1_000_000);
        let owed = s.get_pending_rewards(accounts.bob);
        assert!(owed > 0, "rewards must have accrued");
        assert_eq!(s.get_governance_power(accounts.bob), principal);
        assert_eq!(s.get_stake(accounts.bob).unwrap().amount, principal);

        // A plain claim pays out and leaves both principal and power alone.
        assert_eq!(s.claim_rewards().unwrap(), owed);
        assert_eq!(s.get_governance_power(accounts.bob), principal);
        assert_eq!(s.get_stake(accounts.bob).unwrap().amount, principal);

        // Auto-compounding is different: the reward is added to the stake, so
        // it becomes principal and therefore voting power. That is a new
        // stake, not reward accrual affecting the existing ballot weight.
        s.set_auto_compound(true).unwrap();
        at_block(10 + 2_000_000);
        let reinvested = s.get_pending_rewards(accounts.bob);
        assert!(reinvested > 0);
        assert_eq!(s.claim_rewards().unwrap(), reinvested);
        assert_eq!(
            s.get_stake(accounts.bob).unwrap().amount,
            principal + reinvested
        );
        assert_eq!(s.get_governance_power(accounts.bob), principal + reinvested);
        assert_eq!(s.get_total_staked(), principal + reinvested);
    }
}
