/// # Integration Tests: Governance ↔ Staking Interaction (Issue #488)
///
/// These tests verify the voting-power delegation pipeline between the
/// Staking and Governance contracts.  Because ink! unit tests run inside a
/// single contract environment we test both contracts directly rather than
/// through cross-contract calls, which mirrors the actual interaction semantics.
///
/// Acceptance criteria tested:
///   ✓ Staking creates governance power correctly
///   ✓ Delegation transfers governance power to delegate
///   ✓ Unstaking removes governance power
///   ✓ Governance proposal creation requires minimum voting power
///   ✓ Voting weight matches staked amount (including delegation)
///   ✓ Multiple stakers vote on same proposal with correct weights
#[cfg(test)]
mod integration_governance_staking {
    // ── Staking contract ─────────────────────────────────────────────────────
    use staking::staking::{Error as StakingError, LockPeriod, Staking};

    // ── Governance contract ───────────────────────────────────────────────────
    use governance::governance::{
        Error as GovernanceError, Governance, GovernanceAction, GovernanceProposal,
    };

    use ink::env::{test, DefaultEnvironment};
    use ink::primitives::Hash;

    // ── Account helpers ───────────────────────────────────────────────────────

    fn alice() -> ink::primitives::AccountId {
        test::default_accounts::<DefaultEnvironment>().alice
    }
    fn bob() -> ink::primitives::AccountId {
        test::default_accounts::<DefaultEnvironment>().bob
    }
    fn charlie() -> ink::primitives::AccountId {
        test::default_accounts::<DefaultEnvironment>().charlie
    }
    fn django() -> ink::primitives::AccountId {
        test::default_accounts::<DefaultEnvironment>().django
    }

    /// Convenience: construct a Staking contract with sensible defaults.
    fn new_staking() -> Staking {
        test::set_caller::<DefaultEnvironment>(alice());
        // reward_rate_bps = 500 (5%), min_stake = 1_000
        Staking::new(500, 1_000)
    }

    /// Convenience: construct a Governance contract where alice, bob, charlie
    /// are signers and the threshold is 2 out of 3.
    fn new_governance() -> Governance {
        test::set_caller::<DefaultEnvironment>(alice());
        Governance::new(vec![alice(), bob(), charlie()], 2, 0)
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Test 1: Staking creates governance power correctly
    // ─────────────────────────────────────────────────────────────────────────

    /// Stakes tokens and verifies that governance power equals the staked amount.
    #[ink::test]
    fn test_staking_creates_governance_power() {
        let mut staking = new_staking();
        test::set_caller::<DefaultEnvironment>(alice());

        let stake_amount: u128 = 10_000;
        staking
            .stake(stake_amount, LockPeriod::Flexible)
            .expect("stake should succeed");

        // Governance power must equal staked amount immediately after staking
        let power = staking.get_governance_power(alice());
        assert_eq!(
            power, stake_amount,
            "governance power must equal staked amount after staking"
        );
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Test 2: Delegation transfers governance power to delegate
    // ─────────────────────────────────────────────────────────────────────────

    /// After delegating, the delegate receives the staker's governance power
    /// and the staker no longer holds it directly.
    #[ink::test]
    fn test_delegation_transfers_governance_power() {
        let mut staking = new_staking();
        let stake_amount: u128 = 20_000;

        // Alice stakes
        test::set_caller::<DefaultEnvironment>(alice());
        staking
            .stake(stake_amount, LockPeriod::Flexible)
            .expect("alice stake");

        // Alice delegates governance to bob
        staking
            .delegate_governance(bob())
            .expect("delegation should succeed");

        // Bob should now hold alice's governance power
        let bob_power = staking.get_governance_power(bob());
        assert_eq!(
            bob_power, stake_amount,
            "bob should hold alice's governance power after delegation"
        );

        // Alice's own governance power slot should be zero (power moved to bob)
        let alice_power = staking.get_governance_power(alice());
        assert_eq!(
            alice_power, 0,
            "alice's direct governance power should be zero after delegating"
        );
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Test 3: Unstaking removes governance power
    // ─────────────────────────────────────────────────────────────────────────

    /// After unstaking, governance power drops back to zero.
    #[ink::test]
    fn test_unstaking_removes_governance_power() {
        let mut staking = new_staking();

        test::set_caller::<DefaultEnvironment>(alice());
        staking
            .stake(15_000, LockPeriod::Flexible)
            .expect("alice stake");

        // Verify power present
        assert_eq!(staking.get_governance_power(alice()), 15_000);

        // Unstake (flexible lock, no penalty)
        staking.unstake().expect("unstake should succeed");

        // Governance power must be zero after unstaking
        let power_after = staking.get_governance_power(alice());
        assert_eq!(
            power_after, 0,
            "governance power must be zero after unstaking"
        );
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Test 4: Governance proposal creation requires minimum voting power
    // ─────────────────────────────────────────────────────────────────────────

    /// Only stakers (who hold governance power) can create parameter proposals.
    /// Non-stakers are rejected with NoVotingPower.
    #[ink::test]
    fn test_proposal_creation_requires_voting_power() {
        let mut staking = new_staking();

        // Django has no stake → proposal creation must fail
        test::set_caller::<DefaultEnvironment>(django());
        let result = staking.propose_param_change(staking::staking::ParamKind::MinStake(2_000));
        assert_eq!(
            result,
            Err(StakingError::NoVotingPower),
            "non-staker must not be able to create a proposal"
        );

        // Alice stakes and can now create a proposal
        test::set_caller::<DefaultEnvironment>(alice());
        staking
            .stake(5_000, LockPeriod::Flexible)
            .expect("alice stake");

        let proposal_id = staking
            .propose_param_change(staking::staking::ParamKind::MinStake(2_000))
            .expect("staker should be able to create a proposal");

        let proposal = staking
            .get_param_proposal(proposal_id)
            .expect("proposal should exist");
        assert_eq!(proposal.id, proposal_id);
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Test 5: Voting weight matches staked amount (including delegation)
    // ─────────────────────────────────────────────────────────────────────────

    /// Voting weight equals the staked amount.  When alice delegates to bob,
    /// bob's effective voting weight equals both stakes combined.
    #[ink::test]
    fn test_voting_weight_matches_staked_amount_with_delegation() {
        let mut staking = new_staking();

        let alice_stake: u128 = 10_000;
        let bob_stake: u128 = 5_000;

        // Alice stakes and delegates to bob
        test::set_caller::<DefaultEnvironment>(alice());
        staking
            .stake(alice_stake, LockPeriod::Flexible)
            .expect("alice stake");
        staking
            .delegate_governance(bob())
            .expect("alice delegates to bob");

        // Bob stakes (creating his own power)
        test::set_caller::<DefaultEnvironment>(bob());
        staking
            .stake(bob_stake, LockPeriod::Flexible)
            .expect("bob stake");

        // Bob's governance power = his own stake + alice's delegated power
        let bob_power = staking.get_governance_power(bob());
        assert_eq!(
            bob_power,
            alice_stake + bob_stake,
            "bob's governance power should equal his stake plus alice's delegated stake"
        );

        // Alice's own governance power slot is zero (she delegated)
        let alice_power = staking.get_governance_power(alice());
        assert_eq!(alice_power, 0);
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Test 6: Multiple stakers vote on same proposal with correct weights
    // ─────────────────────────────────────────────────────────────────────────

    /// Three stakers with different amounts vote on the same proposal.
    /// votes_for and votes_against reflect the total staked weight.
    #[ink::test]
    fn test_multiple_stakers_vote_with_correct_weights() {
        let mut staking = new_staking();

        let alice_stake: u128 = 30_000;
        let bob_stake: u128 = 20_000;
        let charlie_stake: u128 = 10_000;

        // Fund the reward pool so staking.stake doesn't panic later on rewards
        test::set_caller::<DefaultEnvironment>(alice());
        staking.fund_reward_pool(100_000).expect("fund pool");

        // All three stake
        test::set_caller::<DefaultEnvironment>(alice());
        staking
            .stake(alice_stake, LockPeriod::Flexible)
            .expect("alice stake");

        test::set_caller::<DefaultEnvironment>(bob());
        staking
            .stake(bob_stake, LockPeriod::Flexible)
            .expect("bob stake");

        test::set_caller::<DefaultEnvironment>(charlie());
        staking
            .stake(charlie_stake, LockPeriod::Flexible)
            .expect("charlie stake");

        // Alice creates a proposal
        test::set_caller::<DefaultEnvironment>(alice());
        let proposal_id = staking
            .propose_param_change(staking::staking::ParamKind::MinStake(2_000))
            .expect("alice creates proposal");

        // Alice and Bob vote FOR, Charlie votes AGAINST
        test::set_caller::<DefaultEnvironment>(alice());
        staking
            .vote_on_proposal(proposal_id, true)
            .expect("alice votes for");

        test::set_caller::<DefaultEnvironment>(bob());
        staking
            .vote_on_proposal(proposal_id, true)
            .expect("bob votes for");

        test::set_caller::<DefaultEnvironment>(charlie());
        staking
            .vote_on_proposal(proposal_id, false)
            .expect("charlie votes against");

        // Check accumulated vote weights
        let proposal = staking
            .get_param_proposal(proposal_id)
            .expect("proposal should exist");

        assert_eq!(
            proposal.votes_for,
            alice_stake + bob_stake,
            "votes_for should equal alice + bob stake"
        );
        assert_eq!(
            proposal.votes_against, charlie_stake,
            "votes_against should equal charlie's stake"
        );
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Test 7: Governance signers can create and vote on proposals (Governance contract)
    // ─────────────────────────────────────────────────────────────────────────

    /// Verify the Governance contract's proposal lifecycle integrates with
    /// the expected voting-power model (signer-based, not stake-based).
    #[ink::test]
    fn test_governance_proposal_lifecycle() {
        let mut gov = new_governance();

        // Alice creates a proposal
        test::set_caller::<DefaultEnvironment>(alice());
        let proposal_id = gov
            .create_proposal(
                Hash::from([1u8; 32]),
                GovernanceAction::ModifyProperty,
                None,
            )
            .expect("alice should be able to create a proposal as a signer");

        // Bob votes for
        test::set_caller::<DefaultEnvironment>(bob());
        gov.vote(proposal_id, true).expect("bob votes for");

        // After bob's vote the threshold (2) is met → proposal moves to Approved
        let proposal: GovernanceProposal = gov
            .get_proposal(proposal_id)
            .expect("proposal should exist");

        use governance::governance::ProposalStatus;
        assert_eq!(
            proposal.status,
            ProposalStatus::Approved,
            "proposal should be Approved after threshold votes"
        );
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Test 8: Non-signer cannot create governance proposals
    // ─────────────────────────────────────────────────────────────────────────

    #[ink::test]
    fn test_non_signer_cannot_create_governance_proposal() {
        let mut gov = new_governance();

        // Django is not a signer
        test::set_caller::<DefaultEnvironment>(django());
        let result =
            gov.create_proposal(Hash::from([2u8; 32]), GovernanceAction::SaleApproval, None);
        assert_eq!(
            result,
            Err(GovernanceError::NotASigner),
            "non-signer must not create governance proposals"
        );
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Test 9: Delegation and then re-delegation updates power correctly
    // ─────────────────────────────────────────────────────────────────────────

    /// When alice delegates to bob and then delegates again to charlie,
    /// charlie ends up with alice's power and bob's power is removed.
    #[ink::test]
    fn test_re_delegation_updates_governance_power() {
        let mut staking = new_staking();
        let stake_amount: u128 = 10_000;

        test::set_caller::<DefaultEnvironment>(alice());
        staking
            .stake(stake_amount, LockPeriod::Flexible)
            .expect("alice stake");

        // First delegation: alice → bob
        staking
            .delegate_governance(bob())
            .expect("first delegation");
        assert_eq!(staking.get_governance_power(bob()), stake_amount);
        assert_eq!(staking.get_governance_power(alice()), 0);

        // Re-delegation: alice → charlie
        staking
            .delegate_governance(charlie())
            .expect("second delegation");
        assert_eq!(
            staking.get_governance_power(charlie()),
            stake_amount,
            "charlie should hold alice's power after re-delegation"
        );
        assert_eq!(
            staking.get_governance_power(bob()),
            0,
            "bob's power should be cleared after alice re-delegates"
        );
    }
}
