//! Integration tests for lending liquidation driven by an oracle price feed
//! (#1522).
//!
//! The flow exercised here is: open a collateralised loan, publish a price on
//! the compiled `MockOracle`, carry that price into the lending contract's
//! oracle feed, and liquidate once the position's health factor falls below
//! 1.0. Both contracts are the real compiled ones; nothing is mocked at the
//! Rust level.
//!
//! ## Where the oracle boundary actually is
//!
//! Lending does **not** call the oracle contract. Since #1088 its liquidation
//! path reads only the admin-injected `oracle_prices` feed and ignores the
//! caller-supplied price vector, specifically so a liquidator cannot steer the
//! outcome. `MockOracle` therefore stands in for the admin's off-chain price
//! pipeline: `publish` pushes a price to the mock oracle, reads it back through
//! the `Oracle` trait (proving the oracle really served it), and then the admin
//! records the same value on lending. The test never writes a price into
//! lending that the oracle did not first produce.
//!
//! ## Liquidation semantics in this contract
//!
//! `liquidate_loan` is an accounting finalisation: it re-prices the pledged
//! collateral from the fresh oracle feed, compares debt/collateral LTV against
//! the weighted liquidation threshold, and moves the loan to `Liquidated`. It
//! does not transfer collateral, does not accept a partial repayment, and does
//! not represent partial liquidation. The health factor is expressed below as
//! `collateral_value * liquidation_threshold_bps / debt` in basis points, so
//! 10_000 is exactly 1.0 and liquidation happens strictly below it. Tests that
//! would require partial liquidation instead pin the real behaviour: a partial
//! *price decline* leaves the position Active until the threshold is crossed,
//! at which point the whole position is closed. This deviation from the
//! issue's wording is called out in the module docs rather than hidden.

#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(test)]
mod tests {
    use ink::env::{test, DefaultEnvironment};
    use ink::primitives::AccountId;
    use mock_oracle::mock_oracle_contract::MockOracle;
    use propchain_lending::propchain_lending::{LendingError, LoanStatus, PropertyLending};
    use propchain_traits::oracle::{Oracle, OracleError};

    type Accounts = test::DefaultAccounts<DefaultEnvironment>;

    const PROPERTY: u64 = 10;
    const ASSESSED_VALUE: u128 = 1_000_000;
    const DEBT: u128 = 500_000;
    const LTV_BPS: u32 = 7_000;
    /// Liquidation threshold in basis points: LTV above 8_000 is liquidatable.
    const LIQ_THRESHOLD_BPS: u32 = 8_000;
    /// `price_max_age` default in the contract.
    const PRICE_MAX_AGE: u64 = 3_600;

    /// A fully funded, underwritten loan. Bob has six on-time repayments
    /// (credit score 620), a 500 000 loan on a 1 000 000 property, and the
    /// admin has assessed the collateral at 8 000 bps.
    fn setup() -> (PropertyLending, MockOracle, Accounts) {
        let accounts = test::default_accounts::<DefaultEnvironment>();
        test::set_caller::<DefaultEnvironment>(accounts.alice);
        test::set_block_timestamp::<DefaultEnvironment>(100);
        test::set_block_number::<DefaultEnvironment>(0);

        let mut lending = PropertyLending::new(accounts.alice);
        for _ in 0..6 {
            lending
                .record_repayment(accounts.bob)
                .expect("admin records repayment");
        }
        lending
            .assess_collateral(PROPERTY, ASSESSED_VALUE, LTV_BPS, LIQ_THRESHOLD_BPS)
            .expect("collateral assessed");

        // The mock oracle is created with alice as faucet admin too.
        let oracle = MockOracle::new();
        (lending, oracle, accounts)
    }

    fn open_loan(lending: &mut PropertyLending, accounts: &Accounts) -> u64 {
        test::set_caller::<DefaultEnvironment>(accounts.bob);
        let loan_id = lending
            .apply_for_loan(PROPERTY, DEBT, ASSESSED_VALUE, 999)
            .expect("application stored");
        test::set_caller::<DefaultEnvironment>(accounts.alice);
        assert!(lending.underwrite_loan(loan_id).expect("underwritten"));
        loan_id
    }

    /// Oracle → lending price publication. The mock oracle value is read back
    /// through the oracle trait before the admin records the identical value
    /// on lending, so the test exercises both compiled contracts.
    fn publish(
        oracle: &mut MockOracle,
        lending: &mut PropertyLending,
        admin: AccountId,
        price: u128,
    ) {
        test::set_caller::<DefaultEnvironment>(admin);
        oracle
            .set_price(PROPERTY, price)
            .expect("mock oracle accepts the price");
        assert_eq!(
            oracle.get_pushed_price(PROPERTY),
            Some(price),
            "oracle must hold the pushed value"
        );
        let served = Oracle::get_valuation(oracle, PROPERTY)
            .expect("oracle serves the pushed value")
            .valuation;
        assert_eq!(served, price);

        test::set_caller::<DefaultEnvironment>(admin);
        lending
            .set_oracle_price(PROPERTY, served)
            .expect("admin records the oracle price");
        assert_eq!(lending.get_oracle_price(PROPERTY).unwrap().value, price);
    }

    fn debt_of(lending: &PropertyLending, loan_id: u64) -> u128 {
        let loan = lending.get_loan(loan_id).expect("loan exists");
        loan.requested_amount.saturating_add(loan.accrued_interest)
    }

    /// Health factor in basis points: `value * threshold / debt`.
    /// `10_000` is 1.0; liquidation triggers strictly below it.
    fn health_factor_bps(value: u128, debt: u128) -> u128 {
        value.saturating_mul(LIQ_THRESHOLD_BPS as u128) / debt.max(1)
    }

    // ── Position setup ──────────────────────────────────────────────────

    #[ink::test]
    fn setup_creates_an_active_position_with_no_oracle_price_yet() {
        let (mut lending, _, accounts) = setup();
        let loan_id = open_loan(&mut lending, &accounts);

        let loan = lending.get_loan(loan_id).expect("stored");
        assert_eq!(loan.status, LoanStatus::Active);
        assert_eq!(loan.requested_amount, DEBT);
        assert_eq!(loan.collateral_value, ASSESSED_VALUE);
        assert_eq!(loan.credit_score, 620);
        assert_eq!(loan.accrued_interest, 0);
        assert!(lending.get_oracle_price(PROPERTY).is_none());

        // The position starts healthy: 16 000 bps = 1.6.
        assert_eq!(
            health_factor_bps(ASSESSED_VALUE, debt_of(&lending, loan_id)),
            16_000
        );
    }

    // ── Healthy and boundary positions ──────────────────────────────────

    #[ink::test]
    fn a_healthy_position_is_not_liquidatable() {
        let (mut lending, mut oracle, accounts) = setup();
        let id = open_loan(&mut lending, &accounts);

        publish(&mut oracle, &mut lending, accounts.alice, ASSESSED_VALUE);
        assert_eq!(
            health_factor_bps(ASSESSED_VALUE, debt_of(&lending, id)),
            16_000
        );

        assert_eq!(
            lending.liquidate_loan(id, vec![]),
            Err(LendingError::LiquidationThresholdNotMet)
        );
        assert_eq!(lending.get_loan(id).unwrap().status, LoanStatus::Active);
        assert_eq!(
            lending.should_liquidate_loan(id, vec![(PROPERTY, ASSESSED_VALUE)]),
            Ok(false)
        );
    }

    #[ink::test]
    fn a_health_factor_of_exactly_one_is_still_safe() {
        let (mut lending, mut oracle, accounts) = setup();
        let id = open_loan(&mut lending, &accounts);

        // 500 000 * 10 000 / 8 000 = 625 000 is the exact LTV boundary.
        let boundary = DEBT * 10_000 / LIQ_THRESHOLD_BPS as u128;
        publish(&mut oracle, &mut lending, accounts.alice, boundary);
        assert_eq!(health_factor_bps(boundary, debt_of(&lending, id)), 10_000);

        assert_eq!(
            lending.liquidate_loan(id, vec![]),
            Err(LendingError::LiquidationThresholdNotMet),
            "exactly 1.0 is not below the threshold"
        );
        assert_eq!(lending.get_loan(id).unwrap().status, LoanStatus::Active);
    }

    #[ink::test]
    fn a_price_drop_below_one_health_trigger_liquidates_the_position() {
        let (mut lending, mut oracle, accounts) = setup();
        let id = open_loan(&mut lending, &accounts);

        let depressed = 600_000u128;
        publish(&mut oracle, &mut lending, accounts.alice, depressed);
        assert_eq!(health_factor_bps(depressed, debt_of(&lending, id)), 9_600);
        assert!(health_factor_bps(depressed, debt_of(&lending, id)) < 10_000);

        let events_before = test::recorded_events().count();
        lending
            .liquidate_loan(id, vec![])
            .expect("liquidation accepted");
        assert_eq!(lending.get_loan(id).unwrap().status, LoanStatus::Liquidated);
        // One `LoanLiquidated` event (plus none from the view call above).
        assert_eq!(test::recorded_events().count(), events_before + 1);
    }

    // ── Liquidation is final ────────────────────────────────────────────

    #[ink::test]
    fn full_liquidation_closes_the_position_entirely() {
        let (mut lending, mut oracle, accounts) = setup();
        let id = open_loan(&mut lending, &accounts);
        publish(&mut oracle, &mut lending, accounts.alice, 600_000);

        lending.liquidate_loan(id, vec![]).expect("liquidated");
        let closed = lending.get_loan(id).expect("record retained");
        assert_eq!(closed.status, LoanStatus::Liquidated);

        // No second liquidation, no risk signal, and the position can no
        // longer be acted on as if it were open.
        assert_eq!(
            lending.liquidate_loan(id, vec![]),
            Err(LendingError::LoanNotActive)
        );
        assert_eq!(
            lending.should_liquidate_loan(id, vec![(PROPERTY, 1)]),
            Ok(false)
        );
        assert_eq!(
            lending.liquidate_loan(9_999, vec![]),
            Err(LendingError::LoanNotFound)
        );
    }

    #[ink::test]
    fn a_partial_price_decline_leaves_the_position_healthy_until_the_threshold() {
        let (mut lending, mut oracle, accounts) = setup();
        let id = open_loan(&mut lending, &accounts);

        // A partial decline: health factor 1.12, then 1.04. The position is
        // still open and not liquidatable after either step.
        publish(&mut oracle, &mut lending, accounts.alice, 700_000);
        assert_eq!(health_factor_bps(700_000, debt_of(&lending, id)), 11_200);
        assert_eq!(
            lending.liquidate_loan(id, vec![]),
            Err(LendingError::LiquidationThresholdNotMet)
        );
        assert_eq!(lending.get_loan(id).unwrap().status, LoanStatus::Active);

        publish(&mut oracle, &mut lending, accounts.alice, 650_000);
        assert_eq!(health_factor_bps(650_000, debt_of(&lending, id)), 10_400);
        assert_eq!(
            lending.liquidate_loan(id, vec![]),
            Err(LendingError::LiquidationThresholdNotMet)
        );
        assert_eq!(lending.get_loan(id).unwrap().status, LoanStatus::Active);

        // The next step crosses 1.0 and closes the whole position. This
        // contract has no partial-liquidation mode, so that is the only
        // liquidation outcome available.
        publish(&mut oracle, &mut lending, accounts.alice, 600_000);
        lending.liquidate_loan(id, vec![]).expect("liquidated");
        assert_eq!(lending.get_loan(id).unwrap().status, LoanStatus::Liquidated);
    }

    // ── Oracle integrity ────────────────────────────────────────────────

    #[ink::test]
    fn liquidation_needs_a_price_from_the_feed() {
        let (mut lending, _, accounts) = setup();
        let id = open_loan(&mut lending, &accounts);

        assert_eq!(
            lending.liquidate_loan(id, vec![]),
            Err(LendingError::PriceFeedMissing)
        );
        assert_eq!(lending.get_loan(id).unwrap().status, LoanStatus::Active);

        // A zero price is not a usable price either.
        test::set_caller::<DefaultEnvironment>(accounts.alice);
        assert_eq!(
            lending.set_oracle_price(PROPERTY, 0),
            Err(LendingError::InvalidParameters)
        );
    }

    #[ink::test]
    fn a_stale_price_blocks_liquidation_and_a_refresh_unblocks_it() {
        let (mut lending, mut oracle, accounts) = setup();
        let id = open_loan(&mut lending, &accounts);

        // Price stamped at t=100.
        publish(&mut oracle, &mut lending, accounts.alice, 600_000);
        test::set_block_timestamp::<DefaultEnvironment>(100 + PRICE_MAX_AGE + 1);
        assert_eq!(
            lending.liquidate_loan(id, vec![]),
            Err(LendingError::PriceFeedStale)
        );
        assert_eq!(lending.get_loan(id).unwrap().status, LoanStatus::Active);

        // Republishing at the current timestamp makes the same liquidation
        // valid again.
        publish(&mut oracle, &mut lending, accounts.alice, 600_000);
        lending
            .liquidate_loan(id, vec![])
            .expect("fresh price liquidates");
        assert_eq!(lending.get_loan(id).unwrap().status, LoanStatus::Liquidated);
    }

    #[ink::test]
    fn a_price_at_the_maximum_age_is_still_accepted() {
        let (mut lending, mut oracle, accounts) = setup();
        let id = open_loan(&mut lending, &accounts);

        publish(&mut oracle, &mut lending, accounts.alice, 600_000);
        test::set_block_timestamp::<DefaultEnvironment>(100 + PRICE_MAX_AGE);
        assert_eq!(lending.get_price_max_age(), PRICE_MAX_AGE);
        lending
            .liquidate_loan(id, vec![])
            .expect("age == max_age is inclusive");
        assert_eq!(lending.get_loan(id).unwrap().status, LoanStatus::Liquidated);
    }

    #[ink::test]
    fn caller_supplied_prices_cannot_force_or_prevent_liquidation() {
        let (mut lending, mut oracle, accounts) = setup();
        let id = open_loan(&mut lending, &accounts);

        // The feed says healthy even though the caller claims a collapse.
        publish(&mut oracle, &mut lending, accounts.alice, ASSESSED_VALUE);
        assert_eq!(
            lending.liquidate_loan(id, vec![(PROPERTY, 1)]),
            Err(LendingError::LiquidationThresholdNotMet)
        );
        assert_eq!(lending.get_loan(id).unwrap().status, LoanStatus::Active);

        // The feed says collapsed even though the caller claims a healthy
        // market; the feed wins.
        publish(&mut oracle, &mut lending, accounts.alice, 400_000);
        lending
            .liquidate_loan(id, vec![(PROPERTY, 9_000_000)])
            .expect("oracle price decides");
        assert_eq!(lending.get_loan(id).unwrap().status, LoanStatus::Liquidated);
    }

    #[ink::test]
    fn only_the_admin_can_publish_a_lending_price() {
        let (mut lending, mut oracle, accounts) = setup();
        let id = open_loan(&mut lending, &accounts);

        test::set_caller::<DefaultEnvironment>(accounts.bob);
        assert_eq!(
            lending.set_oracle_price(PROPERTY, 600_000),
            Err(LendingError::Unauthorized)
        );
        assert_eq!(
            lending.set_price_max_age(1),
            Err(LendingError::Unauthorized)
        );
        assert!(lending.get_oracle_price(PROPERTY).is_none());
        assert_eq!(
            lending.liquidate_loan(id, vec![]),
            Err(LendingError::PriceFeedMissing)
        );

        // The mock oracle rejects zero prices before lending ever sees them.
        test::set_caller::<DefaultEnvironment>(accounts.alice);
        assert_eq!(
            oracle.set_price(PROPERTY, 0),
            Err(OracleError::InvalidValuation)
        );
    }
}
