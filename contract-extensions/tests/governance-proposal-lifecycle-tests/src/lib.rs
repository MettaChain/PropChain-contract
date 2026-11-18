//! Unit tests for the governance proposal lifecycle (#1447):
//! `Active -> Approved (threshold, timelock starts) -> Executed`, with the
//! `Rejected` and `Cancelled` exits, double-vote and signer checks, and
//! delegated voting weight.

#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(test)]
mod tests {
    use governance::governance::{Error, Governance, GovernanceAction, ProposalStatus};
    use ink::env::{test, DefaultEnvironment};
    use ink::primitives::{AccountId, Hash};

    type Accounts = test::DefaultAccounts<DefaultEnvironment>;

    const TIMELOCK: u64 = 10;

    fn hash(byte: u8) -> Hash {
        Hash::from([byte; 32])
    }

    /// Alice (admin), bob and charlie are signers; threshold 2.
    fn setup() -> (Governance, Accounts) {
        let accounts = test::default_accounts::<DefaultEnvironment>();
        test::set_caller::<DefaultEnvironment>(accounts.alice);
        test::set_block_number::<DefaultEnvironment>(0);
        let g = Governance::new(vec![accounts.alice, accounts.bob, accounts.charlie], 2, TIMELOCK);
        (g, accounts)
    }

    fn propose(g: &mut Governance, who: AccountId) -> u64 {
        test::set_caller::<DefaultEnvironment>(who);
        g.create_proposal(hash(0xA1), GovernanceAction::ChangeThreshold, None).expect("proposal created")
    }

    fn vote(g: &mut Governance, who: AccountId, id: u64, support: bool) -> Result<(), Error> {
        test::set_caller::<DefaultEnvironment>(who);
        g.vote(id, support)
    }

    fn status(g: &Governance, id: u64) -> ProposalStatus {
        g.get_proposal(id).unwrap().status
    }

    // ── Setup ───────────────────────────────────────────────────────────

    #[ink::test]
    fn the_constructor_stores_signers_and_corrects_an_invalid_threshold() {
        let accounts = test::default_accounts::<DefaultEnvironment>();
        test::set_caller::<DefaultEnvironment>(accounts.alice);
        let g = Governance::new(vec![accounts.alice, accounts.bob], 2, TIMELOCK);
        assert_eq!(g.get_admin(), accounts.alice);
        assert_eq!(g.get_signers(), vec![accounts.alice, accounts.bob]);
        assert_eq!(g.get_threshold(), 2);

        // Zero or above-roster thresholds fall back to "all signers".
        assert_eq!(Governance::new(vec![accounts.alice, accounts.bob], 0, TIMELOCK).get_threshold(), 2);
        assert_eq!(Governance::new(vec![accounts.alice, accounts.bob], 9, TIMELOCK).get_threshold(), 2);
        assert_eq!(Governance::new(vec![accounts.alice, accounts.bob], 1, TIMELOCK).get_threshold(), 1);
    }

    // ── Creating proposals ──────────────────────────────────────────────

    #[ink::test]
    fn a_signer_creates_an_active_proposal_with_the_current_threshold() {
        let (mut g, accounts) = setup();
        let id = propose(&mut g, accounts.bob);
        let p = g.get_proposal(id).unwrap();
        assert_eq!(p.status, ProposalStatus::Active);
        assert_eq!(p.proposer, accounts.bob);
        assert_eq!(p.threshold, 2);
        assert_eq!((p.votes_for, p.votes_against), (0, 0));
        assert_eq!((p.executed_at, p.timelock_until), (0, 0));
        assert!(!p.is_emergency);
        assert_eq!(g.get_active_proposal_count(), 1);
    }

    #[ink::test]
    fn proposal_ids_increase_and_unknown_ids_are_absent() {
        let (mut g, accounts) = setup();
        let a = propose(&mut g, accounts.alice);
        let b = propose(&mut g, accounts.alice);
        assert_eq!((a, b), (0, 1));
        assert!(g.get_proposal(2).is_none());
        assert_eq!(g.get_active_proposal_count(), 2);
    }

    #[ink::test]
    fn non_signers_cannot_propose_vote_or_execute() {
        let (mut g, accounts) = setup();
        let id = propose(&mut g, accounts.alice);
        test::set_caller::<DefaultEnvironment>(accounts.eve);
        assert_eq!(
            g.create_proposal(hash(1), GovernanceAction::ChangeThreshold, None),
            Err(Error::NotASigner)
        );
        assert_eq!(g.vote(id, true), Err(Error::NotASigner));
        assert_eq!(g.execute_proposal(id), Err(Error::NotASigner));
    }

    // ── Voting and approval ─────────────────────────────────────────────

    #[ink::test]
    fn votes_below_the_threshold_leave_the_proposal_active() {
        let (mut g, accounts) = setup();
        let id = propose(&mut g, accounts.alice);
        vote(&mut g, accounts.alice, id, true).unwrap();
        let p = g.get_proposal(id).unwrap();
        assert_eq!(p.status, ProposalStatus::Active);
        assert_eq!(p.votes_for, 1);
    }

    #[ink::test]
    fn reaching_the_threshold_approves_the_proposal_and_starts_the_timelock() {
        let (mut g, accounts) = setup();
        test::set_block_number::<DefaultEnvironment>(5);
        let id = propose(&mut g, accounts.alice);
        vote(&mut g, accounts.alice, id, true).unwrap();
        vote(&mut g, accounts.bob, id, true).unwrap();
        let p = g.get_proposal(id).unwrap();
        assert_eq!(p.status, ProposalStatus::Approved);
        assert_eq!(p.votes_for, 2);
        // The timelock starts when the threshold is reached.
        assert_eq!(p.timelock_until, 5 + TIMELOCK);
    }

    #[ink::test]
    fn a_signer_cannot_vote_twice_and_unknown_proposals_are_reported() {
        let (mut g, accounts) = setup();
        let id = propose(&mut g, accounts.alice);
        vote(&mut g, accounts.alice, id, true).unwrap();
        assert_eq!(vote(&mut g, accounts.alice, id, true), Err(Error::AlreadyVoted));
        assert_eq!(vote(&mut g, accounts.alice, id, false), Err(Error::AlreadyVoted));
        assert_eq!(g.get_proposal(id).unwrap().votes_for, 1);
        assert_eq!(vote(&mut g, accounts.bob, 99, true), Err(Error::ProposalNotFound));
    }

    #[ink::test]
    fn voting_stops_once_the_proposal_is_no_longer_active() {
        let (mut g, accounts) = setup();
        let id = propose(&mut g, accounts.alice);
        vote(&mut g, accounts.alice, id, true).unwrap();
        vote(&mut g, accounts.bob, id, true).unwrap(); // Approved
        assert_eq!(vote(&mut g, accounts.charlie, id, true), Err(Error::ProposalClosed));
    }

    // ── Execution and the timelock ──────────────────────────────────────

    fn approved(g: &mut Governance, accounts: &Accounts) -> u64 {
        let id = propose(g, accounts.alice);
        vote(g, accounts.alice, id, true).unwrap();
        vote(g, accounts.bob, id, true).unwrap();
        id
    }

    #[ink::test]
    fn an_unapproved_proposal_cannot_be_executed() {
        let (mut g, accounts) = setup();
        let id = propose(&mut g, accounts.alice);
        vote(&mut g, accounts.alice, id, true).unwrap();
        test::set_caller::<DefaultEnvironment>(accounts.alice);
        assert_eq!(g.execute_proposal(id), Err(Error::ProposalClosed));
        assert_eq!(g.execute_proposal(99), Err(Error::ProposalNotFound));
    }

    #[ink::test]
    fn execution_waits_for_the_timelock_and_is_inclusive_at_the_deadline() {
        let (mut g, accounts) = setup();
        let id = approved(&mut g, &accounts);
        let until = g.get_proposal(id).unwrap().timelock_until;
        assert_eq!(until, TIMELOCK);

        test::set_caller::<DefaultEnvironment>(accounts.charlie);
        test::set_block_number::<DefaultEnvironment>((until - 1) as u32);
        assert_eq!(g.execute_proposal(id), Err(Error::TimelockActive));

        test::set_block_number::<DefaultEnvironment>(until as u32);
        g.execute_proposal(id).unwrap();
        let p = g.get_proposal(id).unwrap();
        assert_eq!(p.status, ProposalStatus::Executed);
        assert_eq!(p.executed_at, until);
        assert_eq!(g.get_active_proposal_count(), 0);
    }

    #[ink::test]
    fn an_executed_proposal_cannot_be_executed_cancelled_or_voted_again() {
        let (mut g, accounts) = setup();
        let id = approved(&mut g, &accounts);
        test::set_block_number::<DefaultEnvironment>((TIMELOCK + 1) as u32);
        test::set_caller::<DefaultEnvironment>(accounts.alice);
        g.execute_proposal(id).unwrap();
        assert_eq!(g.execute_proposal(id), Err(Error::ProposalClosed));
        assert_eq!(g.cancel_proposal(id), Err(Error::ProposalClosed));
        assert_eq!(vote(&mut g, accounts.charlie, id, true), Err(Error::ProposalClosed));
    }

    // ── Rejection ───────────────────────────────────────────────────────

    #[ink::test]
    fn a_proposal_is_rejected_as_soon_as_the_threshold_becomes_unreachable() {
        let (mut g, accounts) = setup();
        let id = propose(&mut g, accounts.alice);
        vote(&mut g, accounts.alice, id, false).unwrap();
        // One against out of three signers: two approvals are still possible.
        assert_eq!(status(&g, id), ProposalStatus::Active);
        vote(&mut g, accounts.bob, id, false).unwrap();
        // Two against: at most one approval remains, below the threshold of 2.
        assert_eq!(status(&g, id), ProposalStatus::Rejected);
        assert_eq!(g.get_active_proposal_count(), 0);
    }

    #[ink::test]
    fn a_rejected_proposal_cannot_be_executed_or_voted_on() {
        let (mut g, accounts) = setup();
        let id = propose(&mut g, accounts.alice);
        vote(&mut g, accounts.alice, id, false).unwrap();
        vote(&mut g, accounts.bob, id, false).unwrap();
        test::set_block_number::<DefaultEnvironment>(1_000);
        test::set_caller::<DefaultEnvironment>(accounts.alice);
        assert_eq!(g.execute_proposal(id), Err(Error::ProposalClosed));
        assert_eq!(vote(&mut g, accounts.charlie, id, true), Err(Error::ProposalClosed));
    }

    // ── Cancellation ────────────────────────────────────────────────────

    #[ink::test]
    fn the_proposer_or_admin_can_cancel_and_others_cannot() {
        let (mut g, accounts) = setup();
        let by_proposer = propose(&mut g, accounts.bob);
        let by_admin = propose(&mut g, accounts.bob);

        test::set_caller::<DefaultEnvironment>(accounts.charlie);
        assert_eq!(g.cancel_proposal(by_proposer), Err(Error::Unauthorized));

        test::set_caller::<DefaultEnvironment>(accounts.bob);
        g.cancel_proposal(by_proposer).unwrap();
        test::set_caller::<DefaultEnvironment>(accounts.alice);
        g.cancel_proposal(by_admin).unwrap();

        assert_eq!(status(&g, by_proposer), ProposalStatus::Cancelled);
        assert_eq!(status(&g, by_admin), ProposalStatus::Cancelled);
        assert_eq!(g.get_active_proposal_count(), 0);
    }

    #[ink::test]
    fn an_approved_proposal_can_be_cancelled_during_the_timelock() {
        let (mut g, accounts) = setup();
        let id = approved(&mut g, &accounts);
        test::set_caller::<DefaultEnvironment>(accounts.alice);
        g.cancel_proposal(id).unwrap();
        test::set_block_number::<DefaultEnvironment>((TIMELOCK + 5) as u32);
        assert_eq!(g.execute_proposal(id), Err(Error::ProposalClosed));
        assert_eq!(g.cancel_proposal(id), Err(Error::ProposalClosed));
    }

    // ── Delegation ──────────────────────────────────────────────────────

    #[ink::test]
    fn a_delegators_weight_is_added_to_the_delegates_vote() {
        let (mut g, accounts) = setup();
        let id = propose(&mut g, accounts.alice);
        test::set_caller::<DefaultEnvironment>(accounts.bob);
        g.delegate_to(accounts.alice).unwrap();
        assert_eq!(g.get_delegate_of(accounts.bob), Some(accounts.alice));
        assert_eq!(g.get_delegated_count(accounts.alice), 1);

        // Alice now carries weight 2 and reaches the threshold alone.
        vote(&mut g, accounts.alice, id, true).unwrap();
        let p = g.get_proposal(id).unwrap();
        assert_eq!(p.votes_for, 2);
        assert_eq!(p.status, ProposalStatus::Approved);
    }

    #[ink::test]
    fn a_delegator_cannot_vote_until_they_undelegate() {
        let (mut g, accounts) = setup();
        let id = propose(&mut g, accounts.alice);
        test::set_caller::<DefaultEnvironment>(accounts.bob);
        g.delegate_to(accounts.alice).unwrap();
        assert_eq!(g.vote(id, true), Err(Error::StillDelegated));

        g.undelegate().unwrap();
        assert_eq!(g.get_delegate_of(accounts.bob), None);
        assert_eq!(g.get_delegated_count(accounts.alice), 0);
        assert!(g.vote(id, true).is_ok());
    }

    #[ink::test]
    fn delegation_targets_must_be_other_signers() {
        let (mut g, accounts) = setup();
        test::set_caller::<DefaultEnvironment>(accounts.bob);
        assert_eq!(g.delegate_to(accounts.bob), Err(Error::InvalidDelegationTarget));
        assert_eq!(g.delegate_to(accounts.eve), Err(Error::NotASigner));
        test::set_caller::<DefaultEnvironment>(accounts.eve);
        assert_eq!(g.delegate_to(accounts.alice), Err(Error::NotASigner));
    }
}
