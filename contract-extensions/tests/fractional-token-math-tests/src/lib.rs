//! Unit tests for fractional ownership share and pool math (#1456): balances,
//! portfolio valuation, tax summaries, redemption, and the constant-product AMM
//! (LP minting, 0.3% swap fee, spot price, slippage guards, invariant growth).
//!
//! Worked examples (first deposit 1_000 shares : 100_000 value):
//! `LP = isqrt(1_000 * 100_000) = 10_000`, spot = 100, and swapping 100 shares
//! returns `100*9970*100_000 / (1_000*10_000 + 100*9970) = 9_066`.

#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(test)]
mod tests {
    use fractional::fractional::{AmmPool, Fractional, FractionalError, PortfolioItem};
    use ink::env::{test, DefaultEnvironment};

    type Accounts = test::DefaultAccounts<DefaultEnvironment>;

    const TOKEN: u64 = 1;

    fn setup() -> (Fractional, Accounts) {
        let accounts = test::default_accounts::<DefaultEnvironment>();
        test::set_caller::<DefaultEnvironment>(accounts.alice);
        (Fractional::new(), accounts)
    }

    /// Alice holds 10_000 shares and seeds the pool with 1_000 shares : 100_000 value.
    fn with_pool() -> (Fractional, Accounts) {
        let (mut f, accounts) = setup();
        f.mint_shares(accounts.alice, TOKEN, 10_000);
        test::set_value_transferred::<DefaultEnvironment>(100_000);
        assert_eq!(f.add_liquidity(TOKEN, 1_000, 0), Ok(10_000));
        (f, accounts)
    }

    fn pool(f: &Fractional) -> AmmPool {
        f.get_pool(TOKEN).expect("pool exists")
    }

    // ── Balances ────────────────────────────────────────────────────────

    #[ink::test]
    fn minted_shares_accumulate_per_owner_and_token() {
        let (mut f, accounts) = setup();
        f.mint_shares(accounts.bob, TOKEN, 300);
        f.mint_shares(accounts.bob, TOKEN, 200);
        f.mint_shares(accounts.bob, 2, 7);
        f.mint_shares(accounts.charlie, TOKEN, 1);
        assert_eq!(f.balance_of(accounts.bob, TOKEN), 500);
        assert_eq!(f.balance_of(accounts.bob, 2), 7);
        assert_eq!(f.balance_of(accounts.charlie, TOKEN), 1);
        assert_eq!(f.balance_of(accounts.eve, TOKEN), 0);
    }

    #[ink::test]
    fn minting_saturates_instead_of_overflowing() {
        let (mut f, accounts) = setup();
        f.mint_shares(accounts.bob, TOKEN, u128::MAX);
        f.mint_shares(accounts.bob, TOKEN, 10);
        assert_eq!(f.balance_of(accounts.bob, TOKEN), u128::MAX);
    }

    #[ink::test]
    fn consolidation_returns_the_holding_and_rejects_an_empty_one() {
        let (mut f, accounts) = setup();
        assert_eq!(f.consolidate_shares(accounts.bob, TOKEN), Err(FractionalError::InsufficientShares));
        f.mint_shares(accounts.bob, TOKEN, 42);
        assert_eq!(f.consolidate_shares(accounts.bob, TOKEN), Ok(42));
        assert_eq!(f.balance_of(accounts.bob, TOKEN), 42);
    }

    // ── Portfolio valuation and tax summary ─────────────────────────────

    fn item(token_id: u64, shares: u128, price: u128) -> PortfolioItem {
        PortfolioItem { token_id, shares, price_per_share: price }
    }

    #[ink::test]
    fn portfolio_value_is_the_sum_of_shares_times_price() {
        let (f, _) = setup();
        let agg = f.aggregate_portfolio(vec![item(1, 10, 100), item(2, 5, 40)]);
        assert_eq!(agg.total_value, 1_200);
        assert_eq!(agg.positions, vec![(1, 10, 100), (2, 5, 40)]);
    }

    #[ink::test]
    fn a_zero_price_falls_back_to_the_last_known_price() {
        let (mut f, _) = setup();
        f.set_last_price(1, 250);
        let agg = f.aggregate_portfolio(vec![item(1, 4, 0), item(2, 4, 0)]);
        // Token 1 uses the stored price; token 2 has none and is valued at zero.
        assert_eq!(agg.total_value, 1_000);
        assert_eq!(agg.positions, vec![(1, 4, 250), (2, 4, 0)]);
        assert_eq!(f.get_last_price(1), Some(250));
        assert_eq!(f.get_last_price(2), None);
    }

    #[ink::test]
    fn an_explicit_price_overrides_the_stored_price() {
        let (mut f, _) = setup();
        f.set_last_price(1, 250);
        assert_eq!(f.aggregate_portfolio(vec![item(1, 2, 100)]).total_value, 200);
    }

    #[ink::test]
    fn portfolio_math_saturates_and_handles_empty_input() {
        let (f, _) = setup();
        assert_eq!(f.aggregate_portfolio(Vec::new()).total_value, 0);
        let huge = f.aggregate_portfolio(vec![item(1, u128::MAX, 2), item(2, 1, 1)]);
        assert_eq!(huge.total_value, u128::MAX);
    }

    #[ink::test]
    fn the_tax_summary_totals_dividends_and_proceeds_and_counts_transactions() {
        let (f, _) = setup();
        let report = f.summarize_tax(vec![(1, 100), (2, 50)], vec![(1, 400)]);
        assert_eq!(report.total_dividends, 150);
        assert_eq!(report.total_proceeds, 400);
        assert_eq!(report.transactions, 3);
        let empty = f.summarize_tax(Vec::new(), Vec::new());
        assert_eq!((empty.total_dividends, empty.total_proceeds, empty.transactions), (0, 0, 0));
    }

    // ── Redemption ──────────────────────────────────────────────────────

    #[ink::test]
    fn redemption_pays_the_last_price_times_shares_and_burns_the_shares() {
        let (mut f, accounts) = setup();
        f.mint_shares(accounts.bob, TOKEN, 100);
        f.set_last_price(TOKEN, 3);
        test::set_caller::<DefaultEnvironment>(accounts.bob);
        assert_eq!(f.redeem_shares(TOKEN, 40), Ok(120));
        assert_eq!(f.balance_of(accounts.bob, TOKEN), 60);
    }

    #[ink::test]
    fn redemption_rejects_zero_and_over_redemption() {
        let (mut f, accounts) = setup();
        f.mint_shares(accounts.bob, TOKEN, 10);
        test::set_caller::<DefaultEnvironment>(accounts.bob);
        assert_eq!(f.redeem_shares(TOKEN, 0), Err(FractionalError::ZeroAmount));
        assert_eq!(f.redeem_shares(TOKEN, 11), Err(FractionalError::InsufficientShares));
        assert_eq!(f.balance_of(accounts.bob, TOKEN), 10);
    }

    // ── AMM: adding liquidity ───────────────────────────────────────────

    #[ink::test]
    fn the_first_deposit_mints_the_integer_square_root_of_the_product() {
        let (f, accounts) = with_pool();
        let p = pool(&f);
        assert_eq!((p.share_reserve, p.value_reserve, p.lp_supply), (1_000, 100_000, 10_000));
        assert_eq!(f.lp_balance_of(accounts.alice, TOKEN), 10_000);
        // The deposited shares left the provider's balance and the spot price is set.
        assert_eq!(f.balance_of(accounts.alice, TOKEN), 9_000);
        assert_eq!(f.get_spot_price(TOKEN), Ok(100));
    }

    #[ink::test]
    fn a_proportional_deposit_mints_lp_in_proportion_to_the_reserves() {
        let (mut f, accounts) = with_pool();
        test::set_value_transferred::<DefaultEnvironment>(50_000);
        assert_eq!(f.add_liquidity(TOKEN, 500, 0), Ok(5_000));
        let p = pool(&f);
        assert_eq!((p.share_reserve, p.value_reserve, p.lp_supply), (1_500, 150_000, 15_000));
        assert_eq!(f.lp_balance_of(accounts.alice, TOKEN), 15_000);
    }

    #[ink::test]
    fn an_unbalanced_deposit_is_credited_for_the_smaller_side() {
        let (mut f, _) = with_pool();
        // Shares side would give 5_000 LP, value side only 2_500.
        test::set_value_transferred::<DefaultEnvironment>(25_000);
        assert_eq!(f.add_liquidity(TOKEN, 500, 0), Ok(2_500));
    }

    #[ink::test]
    fn add_liquidity_validates_amounts_holdings_and_slippage() {
        let (mut f, accounts) = setup();
        f.mint_shares(accounts.alice, TOKEN, 100);
        test::set_value_transferred::<DefaultEnvironment>(1_000);
        assert_eq!(f.add_liquidity(TOKEN, 0, 0), Err(FractionalError::ZeroAmount));
        assert_eq!(f.add_liquidity(TOKEN, 101, 0), Err(FractionalError::InsufficientShares));
        // sqrt(100 * 1_000) = 316; asking for 317 must fail.
        assert_eq!(f.add_liquidity(TOKEN, 100, 317), Err(FractionalError::SlippageExceeded));
        assert_eq!(f.add_liquidity(TOKEN, 100, 316), Ok(316));
        test::set_value_transferred::<DefaultEnvironment>(0);
        assert_eq!(f.add_liquidity(TOKEN, 1, 0), Err(FractionalError::ZeroAmount));
    }

    // ── AMM: swaps ──────────────────────────────────────────────────────

    #[ink::test]
    fn a_swap_pays_the_constant_product_amount_after_the_fee() {
        let (mut f, _) = with_pool();
        assert_eq!(f.swap_shares_for_value(TOKEN, 100, 0), Ok(9_066));
        let p = pool(&f);
        assert_eq!((p.share_reserve, p.value_reserve), (1_100, 90_934));
        assert_eq!(p.lp_supply, 10_000);
        // Spot price after the trade: 90_934 / 1_100, floored.
        assert_eq!(f.get_spot_price(TOKEN), Ok(82));
    }

    #[ink::test]
    fn the_pool_invariant_grows_with_every_swap_because_of_the_fee() {
        let (mut f, _) = with_pool();
        let k = |p: &AmmPool| p.share_reserve * p.value_reserve;
        let mut last = k(&pool(&f));
        for amount in [10u128, 100, 250] {
            f.swap_shares_for_value(TOKEN, amount, 0).unwrap();
            let now = k(&pool(&f));
            assert!(now > last, "k must not shrink: {last} -> {now}");
            last = now;
        }
    }

    #[ink::test]
    fn each_further_swap_of_the_same_size_pays_less_because_the_price_moves() {
        // One contract instance: ink! off-chain tests share `Mapping` storage between
        // instances, so the price impact is shown by repeating a swap on one pool.
        let (mut f, _) = with_pool();
        let first = f.swap_shares_for_value(TOKEN, 100, 0).unwrap();
        let second = f.swap_shares_for_value(TOKEN, 100, 0).unwrap();
        let third = f.swap_shares_for_value(TOKEN, 100, 0).unwrap();
        assert!(first > second && second > third, "{first} > {second} > {third}");
    }

    #[ink::test]
    fn swap_guards_cover_zero_balance_slippage_and_missing_pool() {
        let (mut f, accounts) = with_pool();
        assert_eq!(f.swap_shares_for_value(TOKEN, 0, 0), Err(FractionalError::ZeroAmount));
        assert_eq!(f.swap_shares_for_value(TOKEN, 9_001, 0), Err(FractionalError::InsufficientShares));
        // Exactly 9_066 is available for 100 shares; 9_067 must trip the guard.
        assert_eq!(f.swap_shares_for_value(TOKEN, 100, 9_067), Err(FractionalError::SlippageExceeded));
        assert_eq!(f.swap_shares_for_value(TOKEN, 100, 9_066), Ok(9_066));

        f.mint_shares(accounts.alice, 9, 10);
        assert_eq!(f.swap_shares_for_value(9, 5, 0), Err(FractionalError::PoolNotFound));
        assert_eq!(f.get_spot_price(9), Err(FractionalError::PoolNotFound));
    }

    // ── AMM: removing liquidity ─────────────────────────────────────────

    #[ink::test]
    fn removing_all_liquidity_returns_the_reserves_pro_rata() {
        let (mut f, accounts) = with_pool();
        assert_eq!(f.remove_liquidity(TOKEN, 10_000, 0, 0), Ok((1_000, 100_000)));
        assert_eq!(f.lp_balance_of(accounts.alice, TOKEN), 0);
        assert_eq!(f.balance_of(accounts.alice, TOKEN), 10_000);
        let p = pool(&f);
        assert_eq!((p.share_reserve, p.value_reserve, p.lp_supply), (0, 0, 0));
    }

    #[ink::test]
    fn a_partial_withdrawal_leaves_the_spot_price_unchanged() {
        let (mut f, _) = with_pool();
        assert_eq!(f.remove_liquidity(TOKEN, 2_500, 0, 0), Ok((250, 25_000)));
        assert_eq!(f.get_spot_price(TOKEN), Ok(100));
        let p = pool(&f);
        assert_eq!((p.share_reserve, p.value_reserve, p.lp_supply), (750, 75_000, 7_500));
    }

    #[ink::test]
    fn remove_liquidity_guards() {
        let (mut f, accounts) = with_pool();
        assert_eq!(f.remove_liquidity(TOKEN, 0, 0, 0), Err(FractionalError::ZeroAmount));
        assert_eq!(f.remove_liquidity(TOKEN, 10_001, 0, 0), Err(FractionalError::InsufficientLpShares));
        // Quote for 1_000 LP is 100 shares and 10_000 value; ask for one more of either.
        assert_eq!(f.remove_liquidity(TOKEN, 1_000, 101, 0), Err(FractionalError::SlippageExceeded));
        assert_eq!(f.remove_liquidity(TOKEN, 1_000, 0, 10_001), Err(FractionalError::SlippageExceeded));
        // A stranger holds no LP.
        test::set_caller::<DefaultEnvironment>(accounts.bob);
        assert_eq!(f.remove_liquidity(TOKEN, 1, 0, 0), Err(FractionalError::InsufficientLpShares));
    }
}
