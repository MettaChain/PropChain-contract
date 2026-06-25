/// # Regression: Reentrancy guard on each protected function (Issue #487)
///
/// Bug references:
///   - Issue #493 — fractional contract functions lacked reentrancy protection
///   - Staking contract unstake/claim_rewards lacked guard initially
///   - Bridge execute_bridge lacked guard initially
///
/// Each test locks the guard manually (simulating a re-entrant context) and
/// verifies the guarded function returns the appropriate error rather than
/// proceeding with the operation.
#[cfg(test)]
mod reentrancy_guard_regression {

    // ── Fractional contract ───────────────────────────────────────────────────
    use fractional::fractional::{Fractional, FractionalError};

    use ink::env::{test, DefaultEnvironment};

    fn alice() -> ink::primitives::AccountId {
        test::default_accounts::<DefaultEnvironment>().alice
    }
    fn bob() -> ink::primitives::AccountId {
        test::default_accounts::<DefaultEnvironment>().bob
    }

    // ── Issue #493: Fractional — buy_shares ─────────────────────────────────

    /// Regression for issue #493: buy_shares must return ReentrantCall when
    /// the reentrancy guard is already locked.
    #[ink::test]
    fn fractional_buy_shares_reentrant_call_regression() {
        let mut f = Fractional::new();
        test::set_caller::<DefaultEnvironment>(alice());
        f.mint_shares(alice(), 1, 100);
        f.list_shares_for_sale(1, 50, 10).unwrap();

        // Simulate reentrancy by locking the guard externally
        f.reentrancy_guard.enter().expect("guard enter");

        test::set_caller::<DefaultEnvironment>(bob());
        let result = f.buy_shares(alice(), 1, 10);
        assert_eq!(
            result,
            Err(FractionalError::ReentrantCall),
            "[regression #493] buy_shares must return ReentrantCall"
        );
        f.reentrancy_guard.exit();
    }

    // ── Issue #493: Fractional — redeem_shares ──────────────────────────────

    /// Regression for issue #493: redeem_shares must return ReentrantCall when
    /// the reentrancy guard is already locked.
    #[ink::test]
    fn fractional_redeem_shares_reentrant_call_regression() {
        let mut f = Fractional::new();
        test::set_caller::<DefaultEnvironment>(alice());
        f.mint_shares(alice(), 1, 100);
        f.set_last_price(1, 5);

        f.reentrancy_guard.enter().expect("guard enter");
        let result = f.redeem_shares(1, 10);
        assert_eq!(
            result,
            Err(FractionalError::ReentrantCall),
            "[regression #493] redeem_shares must return ReentrantCall"
        );
        f.reentrancy_guard.exit();
    }

    // ── Issue #493: Fractional — swap_shares_for_value ──────────────────────

    /// Regression for issue #493: swap_shares_for_value must return ReentrantCall
    /// when the reentrancy guard is already locked.
    #[ink::test]
    fn fractional_swap_shares_for_value_reentrant_call_regression() {
        let mut f = Fractional::new();
        test::set_caller::<DefaultEnvironment>(alice());
        f.mint_shares(alice(), 1, 1000);

        test::set_value_transferred::<DefaultEnvironment>(10_000);
        f.add_liquidity(1, 100, 0).unwrap();

        f.reentrancy_guard.enter().expect("guard enter");
        let result = f.swap_shares_for_value(1, 10, 0);
        assert_eq!(
            result,
            Err(FractionalError::ReentrantCall),
            "[regression #493] swap_shares_for_value must return ReentrantCall"
        );
        f.reentrancy_guard.exit();
    }

    // ── Issue #493: Fractional — add_liquidity ──────────────────────────────

    /// Regression for issue #493: add_liquidity must return ReentrantCall
    /// when the reentrancy guard is already locked.
    #[ink::test]
    fn fractional_add_liquidity_reentrant_call_regression() {
        let mut f = Fractional::new();
        test::set_caller::<DefaultEnvironment>(alice());
        f.mint_shares(alice(), 1, 1000);

        f.reentrancy_guard.enter().expect("guard enter");
        test::set_value_transferred::<DefaultEnvironment>(500);
        let result = f.add_liquidity(1, 100, 0);
        assert_eq!(
            result,
            Err(FractionalError::ReentrantCall),
            "[regression #493] add_liquidity must return ReentrantCall"
        );
        f.reentrancy_guard.exit();
    }

    // ── Issue #493: Fractional — remove_liquidity ───────────────────────────

    /// Regression for issue #493: remove_liquidity must return ReentrantCall
    /// when the reentrancy guard is already locked.
    #[ink::test]
    fn fractional_remove_liquidity_reentrant_call_regression() {
        let mut f = Fractional::new();
        test::set_caller::<DefaultEnvironment>(alice());
        f.mint_shares(alice(), 1, 1000);
        test::set_value_transferred::<DefaultEnvironment>(1000);
        let lp = f.add_liquidity(1, 200, 0).unwrap();

        f.reentrancy_guard.enter().expect("guard enter");
        let result = f.remove_liquidity(1, lp, 0, 0);
        assert_eq!(
            result,
            Err(FractionalError::ReentrantCall),
            "[regression #493] remove_liquidity must return ReentrantCall"
        );
        f.reentrancy_guard.exit();
    }

    // ── Staking contract — unstake ───────────────────────────────────────────

    /// Regression: Staking::unstake must be reentrancy-guarded and return
    /// ReentrantCall when the guard is locked.
    #[ink::test]
    fn staking_unstake_reentrant_call_regression() {
        use staking::staking::{Error as StakingError, LockPeriod, Staking};

        let mut s = Staking::new(500, 1_000);
        test::set_caller::<DefaultEnvironment>(alice());
        s.stake(5_000, LockPeriod::Flexible).expect("stake");

        s.reentrancy_guard.enter().expect("guard enter");
        let result = s.unstake();
        assert_eq!(
            result,
            Err(StakingError::ReentrantCall),
            "[regression] staking::unstake must return ReentrantCall"
        );
        s.reentrancy_guard.exit();
    }

    // ── Staking contract — claim_rewards ────────────────────────────────────

    /// Regression: Staking::claim_rewards must be reentrancy-guarded.
    #[ink::test]
    fn staking_claim_rewards_reentrant_call_regression() {
        use staking::staking::{Error as StakingError, LockPeriod, Staking};

        let mut s = Staking::new(500, 1_000);
        test::set_caller::<DefaultEnvironment>(alice());
        s.stake(5_000, LockPeriod::Flexible).expect("stake");
        s.fund_reward_pool(100_000).expect("fund pool");

        s.reentrancy_guard.enter().expect("guard enter");
        let result = s.claim_rewards();
        assert_eq!(
            result,
            Err(StakingError::ReentrantCall),
            "[regression] staking::claim_rewards must return ReentrantCall"
        );
        s.reentrancy_guard.exit();
    }
}
