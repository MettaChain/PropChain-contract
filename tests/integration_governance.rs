/// # Integration Tests: Governance Multisig Proposal Lifecycle (Issue #1002)
///
/// These tests verify the end-to-end governance flow:
///   signer setup -> create_proposal -> votes to threshold -> timelock ->
///   execute_proposal
///
/// Because ink! unit tests run inside a single contract environment, we test
/// the contract directly rather than through cross-contract calls. This
/// mirrors the actual interaction semantics.
///
/// Acceptance criteria tested:
///   check Signer setup via constructor (admin = deployer)
///   check Proposal creation by a signer
///   check Votes by signers until the approval threshold is met
///   check Execution is rejected before threshold and while timelock is active
///   check Timelock elapses after advancing blocks; execution then succeeds
///   check Non-signers cannot propose or vote
///   check Double voting is rejected
///   check Certain majority-against proposals are rejected and not executable
#[cfg(test)]
#[allow(clippy::module_inception)]
mod integration_governance {
    use governance::governance::{
        Error as GovernanceError, Governance, GovernanceAction, ProposalStatus,
    };
    use ink::env::{test, DefaultEnvironment};
    use ink::primitives::Hash;

    const TIMELOCK_BLOCKS: u64 = 10;

    fn description_hash(byte: u8) -> Hash {
        Hash::from([byte; 32])
    }

    /// Two signers (alice, bob), threshold 2. Alice deploys and becomes admin.
    fn setup() -> Governance {
        let accounts = test::default_accounts::<DefaultEnvironment>();
        test::set_caller::<DefaultEnvironment>(accounts.alice);
        Governance::new(vec![accounts.alice, accounts.bob], 2, TIMELOCK_BLOCKS)
    }

    /// Three signers (alice, bob, charlie), threshold 2.
    fn setup_three_signers() -> Governance {
        let accounts = test::default_accounts::<DefaultEnvironment>();
        test::set_caller::<DefaultEnvironment>(accounts.alice);
        Governance::new(
            vec![accounts.alice, accounts.bob, accounts.charlie],
            2,
            TIMELOCK_BLOCKS,
        )
    }

    fn create_proposal(governance: &mut Governance) -> u64 {
        let accounts = test::default_accounts::<DefaultEnvironment>();
        test::set_caller::<DefaultEnvironment>(accounts.alice);
        governance
            .create_proposal(
                description_hash(0xA1),
                GovernanceAction::ChangeThreshold,
                None,
            )
            .expect("Signer should be able to create a proposal")
    }

    /// Scenario 1 - Happy path: propose → vote to threshold → timelock → execute
    #[ink::test]
    fn test_proposal_vote_threshold_timelock_then_execute_succeeds() {
        let accounts = test::default_accounts::<DefaultEnvironment>();
        let mut governance = setup();

        // Constructor sanity: admin is deployer, signers + threshold stored.
        assert_eq!(governance.get_admin(), accounts.alice);
        assert_eq!(governance.get_signers(), vec![accounts.alice, accounts.bob]);
        assert_eq!(governance.get_threshold(), 2);

        // Step 1: signer creates a proposal
        let proposal_id = create_proposal(&mut governance);
        let proposal = governance
            .get_proposal(proposal_id)
            .expect("Proposal should exist after creation");
        assert_eq!(proposal.status, ProposalStatus::Active);
        assert_eq!(proposal.proposer, accounts.alice);

        // Step 2: execution before threshold must be rejected (still Active)
        test::set_caller::<DefaultEnvironment>(accounts.alice);
        assert_eq!(
            governance.execute_proposal(proposal_id),
            Err(GovernanceError::ProposalClosed),
            "Execution must be rejected before approval threshold"
        );

        // Step 3: first approval vote - below threshold, still Active
        test::set_caller::<DefaultEnvironment>(accounts.alice);
        governance
            .vote(proposal_id, true)
            .expect("Signer vote should succeed");

        // Step 4: second approval vote reaches threshold → Approved + timelock
        test::set_caller::<DefaultEnvironment>(accounts.bob);
        governance
            .vote(proposal_id, true)
            .expect("Second signer vote should succeed");
        let proposal = governance.get_proposal(proposal_id).unwrap();
        assert_eq!(proposal.status, ProposalStatus::Approved);
        assert_eq!(
            proposal.timelock_until,
            proposal.created_at + TIMELOCK_BLOCKS,
            "Timelock must start when threshold is reached"
        );

        // Step 5: execution during timelock must be rejected
        test::set_caller::<DefaultEnvironment>(accounts.alice);
        assert_eq!(
            governance.execute_proposal(proposal_id),
            Err(GovernanceError::TimelockActive),
            "Execution must be blocked while the timelock is active"
        );

        // Step 6: advance past the timelock window, execution succeeds
        let current_block = ink::env::block_number::<DefaultEnvironment>() as u64;
        test::set_block_number::<DefaultEnvironment>(
            (current_block.max(proposal.timelock_until) + 1) as u32,
        );
        governance
            .execute_proposal(proposal_id)
            .expect("Execution should succeed once timelock has elapsed");

        let executed = governance.get_proposal(proposal_id).unwrap();
        assert_eq!(executed.status, ProposalStatus::Executed);
        assert!(executed.executed_at > 0);
        assert_eq!(governance.get_active_proposal_count(), 0);
    }

    /// Scenario 2 - Non-signers cannot create proposals nor vote
    #[ink::test]
    fn test_non_signer_cannot_propose_or_vote() {
        let accounts = test::default_accounts::<DefaultEnvironment>();
        let mut governance = setup();

        test::set_caller::<DefaultEnvironment>(accounts.charlie);
        assert_eq!(
            governance.create_proposal(
                description_hash(0xB2),
                GovernanceAction::AddSigner,
                Some(accounts.charlie),
            ),
            Err(GovernanceError::NotASigner),
            "Non-signer must not create proposals"
        );

        let proposal_id = create_proposal(&mut governance);

        test::set_caller::<DefaultEnvironment>(accounts.charlie);
        assert_eq!(
            governance.vote(proposal_id, true),
            Err(GovernanceError::NotASigner),
            "Non-signer must not vote"
        );

        // The invalid vote must not have been recorded.
        let proposal = governance.get_proposal(proposal_id).unwrap();
        assert_eq!(proposal.votes_for, 0);
        assert_eq!(proposal.votes_against, 0);
    }

    /// Scenario 3 - Double voting by the same signer is rejected
    #[ink::test]
    fn test_double_vote_is_rejected() {
        let accounts = test::default_accounts::<DefaultEnvironment>();
        let mut governance = setup();

        let proposal_id = create_proposal(&mut governance);

        test::set_caller::<DefaultEnvironment>(accounts.alice);
        governance
            .vote(proposal_id, true)
            .expect("First vote should succeed");

        assert_eq!(
            governance.vote(proposal_id, true),
            Err(GovernanceError::AlreadyVoted),
            "Duplicate vote by the same signer must be rejected"
        );
        assert_eq!(
            governance.vote(proposal_id, false),
            Err(GovernanceError::AlreadyVoted),
            "Vote flip via duplicate ballot must also be rejected"
        );

        let proposal = governance.get_proposal(proposal_id).unwrap();
        assert_eq!(proposal.votes_for, 1, "Only one vote must be counted");
        assert_eq!(proposal.votes_against, 0);
    }

    /// Scenario 4 - Majority-against proposal becomes unrecoverable and
    /// cannot be executed afterwards
    #[ink::test]
    fn test_majority_against_rejects_proposal_and_blocks_execution() {
        let accounts = test::default_accounts::<DefaultEnvironment>();
        let mut governance = setup_three_signers();

        let proposal_id = create_proposal(&mut governance);

        // Two rejections out of three signers make approval mathematically
        // impossible: remaining votes (1) + votes_for (0) < threshold (2).
        test::set_caller::<DefaultEnvironment>(accounts.alice);
        governance
            .vote(proposal_id, false)
            .expect("Alice reject works");

        // Not yet decided: approval is still theoretically possible.
        assert_eq!(
            governance.get_proposal(proposal_id).unwrap().status,
            ProposalStatus::Active
        );

        test::set_caller::<DefaultEnvironment>(accounts.bob);
        governance
            .vote(proposal_id, false)
            .expect("Bob reject works");

        let rejected = governance.get_proposal(proposal_id).unwrap();
        assert_eq!(rejected.status, ProposalStatus::Rejected);
        assert_eq!(rejected.votes_against, 2);

        test::set_caller::<DefaultEnvironment>(accounts.alice);
        assert_eq!(
            governance.execute_proposal(proposal_id),
            Err(GovernanceError::ProposalClosed),
            "A rejected proposal must not be executable"
        );
    }
}
