/// # Regression: Closed bug reports (Issue #487)
///
/// Each test maps to a specific closed issue.  The test name includes the
/// issue number so CI output pinpoints exactly which regression fired.
#[cfg(test)]
mod closed_bugs_regression {

    use ink::env::{test, DefaultEnvironment};

    fn alice() -> ink::primitives::AccountId {
        test::default_accounts::<DefaultEnvironment>().alice
    }
    fn bob() -> ink::primitives::AccountId {
        test::default_accounts::<DefaultEnvironment>().bob
    }

    // ── Fractional — issue #493: ReentrantCall variant exists ────────────────

    /// Regression for issue #493: FractionalError must contain a ReentrantCall
    /// variant so callers can match on the specific error type.
    #[test]
    fn fractional_error_has_reentrant_call_variant() {
        use fractional::fractional::FractionalError;

        let err = FractionalError::ReentrantCall;
        // Pattern match ensures the variant compiles
        match err {
            FractionalError::ReentrantCall => {} // expected
            _ => panic!("expected ReentrantCall variant"),
        }
    }

    // ── Fractional — issue #493: From<ReentrancyError> for FractionalError ───

    /// Regression: FractionalError must implement From<ReentrancyError> so the
    /// `?` operator inside non_reentrant! works without a manual .map_err call.
    #[test]
    fn fractional_error_from_reentrancy_error() {
        use fractional::fractional::FractionalError;
        use propchain_traits::ReentrancyError;

        let source = ReentrancyError::ReentrantCall;
        let converted = FractionalError::from(source);
        assert_eq!(
            converted,
            FractionalError::ReentrantCall,
            "From<ReentrancyError> must map to FractionalError::ReentrantCall"
        );
    }

    // ── Fractional — issue #493: guard is in storage ─────────────────────────

    /// Regression: Fractional storage must contain a ReentrancyGuard field so
    /// the guard state persists across calls (not per-call stack variable).
    #[ink::test]
    fn fractional_storage_contains_reentrancy_guard() {
        use fractional::fractional::Fractional;

        let f = Fractional::new();
        // The guard field is accessible and starts unlocked
        assert!(
            !f.reentrancy_guard.is_locked(),
            "guard must start unlocked on construction"
        );
    }

    // ── AMM pool — existing invariant: x*y=k ────────────────────────────────

    /// Regression: AMM pool must maintain the constant-product invariant.
    /// A swap must not violate k = share_reserve * value_reserve (allowing for
    /// the 0.3% fee that slightly increases k).
    #[ink::test]
    fn amm_constant_product_invariant_maintained_after_swap() {
        use fractional::fractional::Fractional;

        let mut f = Fractional::new();
        test::set_caller::<DefaultEnvironment>(alice());
        f.mint_shares(alice(), 1, 1000);

        test::set_value_transferred::<DefaultEnvironment>(10_000);
        f.add_liquidity(1, 100, 0).unwrap();

        let pool_before = f.get_pool(1).unwrap();
        let k_before = pool_before.share_reserve * pool_before.value_reserve;

        // Swap 10 shares
        let value_out = f.swap_shares_for_value(1, 10, 0).unwrap();
        assert!(value_out > 0);

        let pool_after = f.get_pool(1).unwrap();
        let k_after = pool_after.share_reserve * pool_after.value_reserve;

        // k should be equal to or slightly higher than before (fee stays in pool)
        assert!(
            k_after >= k_before,
            "AMM k must not decrease after a swap (fee stays in pool)"
        );
    }

    // ── Staking — minimum stake enforced ────────────────────────────────────

    /// Regression: staking below minimum should return InsufficientAmount.
    #[ink::test]
    fn staking_minimum_stake_enforced() {
        use staking::staking::{Error as StakingError, LockPeriod, Staking};

        let mut s = Staking::new(500, 10_000); // min_stake = 10_000
        test::set_caller::<DefaultEnvironment>(alice());

        let result = s.stake(9_999, LockPeriod::Flexible);
        assert_eq!(
            result,
            Err(StakingError::InsufficientAmount),
            "staking below minimum must return InsufficientAmount"
        );
    }

    // ── Staking — double-stake prevented ────────────────────────────────────

    /// Regression: a second stake from the same account must return AlreadyStaked.
    #[ink::test]
    fn staking_double_stake_prevented() {
        use staking::staking::{Error as StakingError, LockPeriod, Staking};

        let mut s = Staking::new(500, 1_000);
        test::set_caller::<DefaultEnvironment>(alice());

        s.stake(5_000, LockPeriod::Flexible).expect("first stake");
        let result = s.stake(5_000, LockPeriod::Flexible);
        assert_eq!(
            result,
            Err(StakingError::AlreadyStaked),
            "second stake from same account must return AlreadyStaked"
        );
    }

    // ── Governance — threshold enforced ─────────────────────────────────────

    /// Regression: proposal execution before threshold is reached must fail.
    #[ink::test]
    fn governance_threshold_enforced_before_execution() {
        use governance::governance::{
            Error as GovError, Governance, GovernanceAction, GovernanceProposal,
        };
        use ink::primitives::Hash;

        test::set_caller::<DefaultEnvironment>(alice());
        let mut gov = Governance::new(vec![alice(), bob()], 2, 0);

        let pid = gov
            .create_proposal(
                Hash::from([1u8; 32]),
                GovernanceAction::ModifyProperty,
                None,
            )
            .expect("create proposal");

        // Only one vote (alice) — threshold is 2
        test::set_caller::<DefaultEnvironment>(alice());
        gov.vote(pid, true).expect("alice votes");

        // Proposal should NOT be approved yet
        let proposal: GovernanceProposal = gov.get_proposal(pid).expect("get proposal");
        use governance::governance::ProposalStatus;
        assert_ne!(
            proposal.status,
            ProposalStatus::Approved,
            "proposal must not be approved with only 1/2 votes"
        );

        // Execute must fail because status is still Active
        let result = gov.execute_proposal(pid);
        assert_eq!(
            result,
            Err(GovError::ProposalClosed),
            "execute_proposal on non-Approved proposal must return ProposalClosed"
        );
    }
}
