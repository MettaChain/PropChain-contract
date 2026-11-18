//! Unit tests for lending liquidation logic (#1448).
//!
//! Liquidation compares `debt * 10_000 / collateral_value` (basis points) with
//! the collateral's liquidation threshold. `should_liquidate_loan` is a view that
//! uses caller-supplied values; `liquidate_loan` ignores caller values and prices
//! collateral from the admin-fed oracle only, rejecting missing or stale prices.

#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(test)]
mod tests {
    use ink::env::{test, DefaultEnvironment};
    use propchain_lending::propchain_lending::{LendingError, LoanStatus, PropertyLending};

    type Accounts = test::DefaultAccounts<DefaultEnvironment>;

    const PROPERTY: u64 = 10;
    const DEBT: u128 = 500_000;
    const LIQ_THRESHOLD_BPS: u128 = 8_000;
    /// Largest collateral value at which `500_000 * 10_000 / value` is still `<= 8_000`.
    const SAFE_VALUE: u128 = 625_000;

    /// Admin = alice. Bob has six on-time repayments, a 500k loan on a 1M
    /// property (7_000 bps LTV cap, 8_000 bps liquidation threshold), underwritten.
    fn setup() -> (PropertyLending, Accounts, u64) {
        let accounts = test::default_accounts::<DefaultEnvironment>();
        test::set_caller::<DefaultEnvironment>(accounts.alice);
        test::set_block_timestamp::<DefaultEnvironment>(100);
        let mut lending = PropertyLending::new(accounts.alice);
        for _ in 0..6 {
            lending.record_repayment(accounts.bob).unwrap();
        }
        lending.assess_collateral(PROPERTY, 1_000_000, 7_000, LIQ_THRESHOLD_BPS as u32).unwrap();

        test::set_caller::<DefaultEnvironment>(accounts.bob);
        let loan_id = lending.apply_for_loan(PROPERTY, DEBT, 1_000_000, 999).unwrap();
        test::set_caller::<DefaultEnvironment>(accounts.alice);
        assert!(lending.underwrite_loan(loan_id).unwrap());
        (lending, accounts, loan_id)
    }

    fn feed(lending: &mut PropertyLending, value: u128) {
        lending.set_oracle_price(PROPERTY, value).unwrap();
    }

    // ── The liquidation signal (view) ───────────────────────────────────

    #[ink::test]
    fn a_healthy_loan_is_not_flagged() {
        let (lending, _, id) = setup();
        assert_eq!(lending.should_liquidate_loan(id, vec![(PROPERTY, 2_000_000)]), Ok(false));
        // With no supplied price the assessed value is used: 5_000 bps < 8_000.
        assert_eq!(lending.should_liquidate_loan(id, vec![]), Ok(false));
    }

    #[ink::test]
    fn the_threshold_boundary_is_exclusive() {
        let (lending, _, id) = setup();
        let ltv = |v: u128| DEBT * 10_000 / v;
        assert_eq!(ltv(SAFE_VALUE), LIQ_THRESHOLD_BPS);
        // Exactly at the threshold: not liquidatable.
        assert_eq!(lending.should_liquidate_loan(id, vec![(PROPERTY, SAFE_VALUE)]), Ok(false));
        // A little lower collateral value pushes LTV above the threshold.
        assert!(ltv(SAFE_VALUE - 1_000) > LIQ_THRESHOLD_BPS);
        assert_eq!(lending.should_liquidate_loan(id, vec![(PROPERTY, SAFE_VALUE - 1_000)]), Ok(true));
    }

    #[ink::test]
    fn a_collateral_collapse_is_flagged() {
        let (lending, _, id) = setup();
        assert_eq!(lending.should_liquidate_loan(id, vec![(PROPERTY, 500_000)]), Ok(true));
        // A zero value is treated as 1, so it is flagged rather than dividing by zero.
        assert_eq!(lending.should_liquidate_loan(id, vec![(PROPERTY, 0)]), Ok(true));
    }

    #[ink::test]
    fn unknown_and_inactive_loans_are_never_flagged() {
        let (lending, accounts, _) = setup();
        assert_eq!(lending.should_liquidate_loan(99, vec![]), Err(LendingError::LoanNotFound));

        // A pending (not yet underwritten) loan short-circuits to false.
        let mut lending = lending;
        test::set_caller::<DefaultEnvironment>(accounts.charlie);
        let pending = lending.apply_for_loan(PROPERTY, 300_000, 1_000_000, 900).unwrap();
        assert_eq!(lending.get_loan(pending).unwrap().status, LoanStatus::Pending);
        assert_eq!(lending.should_liquidate_loan(pending, vec![(PROPERTY, 1)]), Ok(false));
    }

    // ── Executing a liquidation ─────────────────────────────────────────

    #[ink::test]
    fn a_breached_loan_is_liquidated_using_the_oracle_price() {
        let (mut lending, _, id) = setup();
        feed(&mut lending, 500_000);
        lending.liquidate_loan(id, vec![]).unwrap();
        assert_eq!(lending.get_loan(id).unwrap().status, LoanStatus::Liquidated);
    }

    #[ink::test]
    fn a_healthy_oracle_price_blocks_liquidation_even_at_the_boundary() {
        let (mut lending, _, id) = setup();
        feed(&mut lending, SAFE_VALUE);
        assert_eq!(
            lending.liquidate_loan(id, vec![]),
            Err(LendingError::LiquidationThresholdNotMet)
        );
        assert_eq!(lending.get_loan(id).unwrap().status, LoanStatus::Active);
    }

    #[ink::test]
    fn caller_supplied_prices_cannot_force_or_prevent_liquidation() {
        let (mut lending, _, id) = setup();
        // Oracle says healthy; the caller claims a collapse.
        feed(&mut lending, 2_000_000);
        assert_eq!(
            lending.liquidate_loan(id, vec![(PROPERTY, 100_000)]),
            Err(LendingError::LiquidationThresholdNotMet)
        );
        // Oracle says collapsed; the caller claims a healthy market.
        feed(&mut lending, 400_000);
        assert!(lending.liquidate_loan(id, vec![(PROPERTY, 9_000_000)]).is_ok());
    }

    #[ink::test]
    fn liquidation_needs_a_fresh_oracle_price() {
        let (mut lending, _, id) = setup();
        // No price yet.
        assert_eq!(lending.liquidate_loan(id, vec![]), Err(LendingError::PriceFeedMissing));

        feed(&mut lending, 400_000);
        // Exactly at the maximum age (3_600 s) is still accepted...
        test::set_block_timestamp::<DefaultEnvironment>(100 + lending.get_price_max_age());
        assert!(lending.liquidate_loan(id, vec![]).is_ok());
    }

    #[ink::test]
    fn a_stale_price_is_rejected_one_second_past_the_window() {
        let (mut lending, _, id) = setup();
        feed(&mut lending, 400_000);
        test::set_block_timestamp::<DefaultEnvironment>(100 + lending.get_price_max_age() + 1);
        assert_eq!(lending.liquidate_loan(id, vec![]), Err(LendingError::PriceFeedStale));
        assert_eq!(lending.get_loan(id).unwrap().status, LoanStatus::Active);

        // Refreshing the price makes the same liquidation possible.
        feed(&mut lending, 400_000);
        assert!(lending.liquidate_loan(id, vec![]).is_ok());
    }

    #[ink::test]
    fn the_staleness_window_is_configurable_by_the_admin_only() {
        let (mut lending, accounts, id) = setup();
        lending.set_price_max_age(60).unwrap();
        assert_eq!(lending.get_price_max_age(), 60);
        feed(&mut lending, 400_000);
        test::set_block_timestamp::<DefaultEnvironment>(100 + 61);
        assert_eq!(lending.liquidate_loan(id, vec![]), Err(LendingError::PriceFeedStale));

        assert_eq!(lending.set_price_max_age(0), Err(LendingError::InvalidParameters));
        test::set_caller::<DefaultEnvironment>(accounts.bob);
        assert_eq!(lending.set_price_max_age(10), Err(LendingError::Unauthorized));
    }

    // ── State after liquidation ─────────────────────────────────────────

    #[ink::test]
    fn a_liquidated_loan_cannot_be_liquidated_again_or_flagged() {
        let (mut lending, _, id) = setup();
        feed(&mut lending, 400_000);
        lending.liquidate_loan(id, vec![]).unwrap();
        assert_eq!(lending.liquidate_loan(id, vec![]), Err(LendingError::LoanNotActive));
        assert_eq!(lending.should_liquidate_loan(id, vec![(PROPERTY, 1)]), Ok(false));
    }

    #[ink::test]
    fn pending_and_unknown_loans_cannot_be_liquidated() {
        let (mut lending, accounts, _) = setup();
        test::set_caller::<DefaultEnvironment>(accounts.charlie);
        let pending = lending.apply_for_loan(PROPERTY, 300_000, 1_000_000, 900).unwrap();
        feed(&mut lending, 1);
        assert_eq!(lending.liquidate_loan(pending, vec![]), Err(LendingError::LoanNotActive));
        assert_eq!(lending.liquidate_loan(99, vec![]), Err(LendingError::LoanNotFound));
    }

    // ── Oracle feed administration ──────────────────────────────────────

    #[ink::test]
    fn oracle_prices_are_admin_only_and_must_be_positive() {
        let (mut lending, accounts, _) = setup();
        assert_eq!(lending.set_oracle_price(PROPERTY, 0), Err(LendingError::InvalidParameters));
        test::set_caller::<DefaultEnvironment>(accounts.bob);
        assert_eq!(lending.set_oracle_price(PROPERTY, 1_000), Err(LendingError::Unauthorized));
        assert!(lending.get_oracle_price(PROPERTY).is_none());

        test::set_caller::<DefaultEnvironment>(accounts.alice);
        test::set_block_timestamp::<DefaultEnvironment>(777);
        lending.set_oracle_price(PROPERTY, 1_234).unwrap();
        let price = lending.get_oracle_price(PROPERTY).unwrap();
        assert_eq!((price.value, price.reported_at), (1_234, 777));
    }

    #[ink::test]
    fn only_the_admin_assesses_collateral() {
        let (mut lending, accounts, _) = setup();
        test::set_caller::<DefaultEnvironment>(accounts.bob);
        assert_eq!(
            lending.assess_collateral(11, 1, 1, 1),
            Err(LendingError::Unauthorized)
        );
        assert!(lending.get_collateral(11).is_none());
    }
}
