//! Unit tests for DEX pool swap math and invariants (#1449).
//!
//! Model under test: constant product `x * y = k` with the trading fee taken from
//! the input (`in_after_fee = in * (10_000 - fee_bips) / 10_000`) and
//! `out = in_after_fee * reserve_out / (reserve_in + in_after_fee)`. The *whole*
//! input is added to the pool, so fees stay in the reserves and `k` only grows.

#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(test)]
mod tests {
    use ink::env::{test, DefaultEnvironment};
    use propchain_dex::dex::{Error, PropertyDex};

    const BASE: u128 = 100_000;
    const QUOTE: u128 = 400_000;
    const FEE_BIPS: u32 = 30;

    /// Reference implementation of the output formula, used as the oracle.
    fn quote_out(amount_in: u128, reserve_in: u128, reserve_out: u128, fee_bips: u32) -> u128 {
        let after_fee = amount_in * (10_000 - fee_bips as u128) / 10_000;
        after_fee * reserve_out / (reserve_in + after_fee)
    }

    fn setup_with_fee(fee: u32) -> (PropertyDex, u64) {
        let accounts = test::default_accounts::<DefaultEnvironment>();
        test::set_caller::<DefaultEnvironment>(accounts.alice);
        let mut dex = PropertyDex::new("PRPX".into(), 1_000_000, 10, 4_000);
        let pair = dex.create_pool(1, 2, fee, BASE, QUOTE).expect("pool created");
        (dex, pair)
    }

    fn setup() -> (PropertyDex, u64) {
        setup_with_fee(FEE_BIPS)
    }

    fn reserves(dex: &PropertyDex, pair: u64) -> (u128, u128) {
        let p = dex.get_pool(pair).unwrap();
        (p.reserve_base, p.reserve_quote)
    }

    // ── Pool creation ───────────────────────────────────────────────────

    #[ink::test]
    fn a_new_pool_mints_the_integer_square_root_of_the_reserve_product() {
        let (dex, pair) = setup();
        let p = dex.get_pool(pair).unwrap();
        assert_eq!((p.reserve_base, p.reserve_quote), (BASE, QUOTE));
        assert_eq!(p.total_lp_shares, 200_000); // sqrt(100_000 * 400_000)
        assert_eq!(p.fee_bips, FEE_BIPS);
    }

    #[ink::test]
    fn invalid_pools_are_rejected() {
        let accounts = test::default_accounts::<DefaultEnvironment>();
        test::set_caller::<DefaultEnvironment>(accounts.alice);
        let mut dex = PropertyDex::new("PRPX".into(), 1_000_000, 10, 4_000);
        assert_eq!(dex.create_pool(1, 1, 30, BASE, QUOTE), Err(Error::InvalidPair));
        assert_eq!(dex.create_pool(1, 2, 30, 0, QUOTE), Err(Error::InvalidPair));
        assert_eq!(dex.create_pool(1, 2, 30, BASE, 0), Err(Error::InvalidPair));
        assert_eq!(dex.create_pool(1, 2, 1_000, BASE, QUOTE), Err(Error::InvalidPair));
    }

    // ── Swap output matches the formula ─────────────────────────────────

    #[ink::test]
    fn selling_base_pays_the_formula_amount_and_updates_reserves() {
        let (mut dex, pair) = setup();
        let expected = quote_out(1_000, BASE, QUOTE, FEE_BIPS);
        assert_eq!(expected, 3_948);
        assert_eq!(dex.swap_exact_base_for_quote(pair, 1_000, 0), Ok(expected));
        // The whole input joins the pool; only the output leaves it.
        assert_eq!(reserves(&dex, pair), (BASE + 1_000, QUOTE - expected));
    }

    #[ink::test]
    fn buying_base_with_quote_pays_the_formula_amount() {
        let (mut dex, pair) = setup();
        let expected = quote_out(4_000, QUOTE, BASE, FEE_BIPS);
        assert_eq!(expected, 987);
        assert_eq!(dex.swap_exact_quote_for_base(pair, 4_000, 0), Ok(expected));
        assert_eq!(reserves(&dex, pair), (BASE - expected, QUOTE + 4_000));
    }

    #[ink::test]
    fn a_higher_fee_pays_out_less_for_the_same_input() {
        let (mut low, low_pair) = setup_with_fee(30);
        let (mut high, high_pair) = setup_with_fee(300);
        let out_low = low.swap_exact_base_for_quote(low_pair, 10_000, 0).unwrap();
        let out_high = high.swap_exact_base_for_quote(high_pair, 10_000, 0).unwrap();
        assert!(out_low > out_high);
        assert_eq!(out_low, quote_out(10_000, BASE, QUOTE, 30));
        assert_eq!(out_high, quote_out(10_000, BASE, QUOTE, 300));
    }

    // ── Invariants ──────────────────────────────────────────────────────

    #[ink::test]
    fn the_constant_product_never_decreases_across_swaps_in_either_direction() {
        let (mut dex, pair) = setup();
        let k = |d: &PropertyDex| {
            let (b, q) = reserves(d, pair);
            b * q
        };
        let mut last = k(&dex);
        for step in 0..6u32 {
            if step % 2 == 0 {
                dex.swap_exact_base_for_quote(pair, 2_000 + step as u128 * 500, 0).unwrap();
            } else {
                dex.swap_exact_quote_for_base(pair, 9_000 + step as u128 * 700, 0).unwrap();
            }
            let now = k(&dex);
            assert!(now >= last, "k shrank at step {step}: {last} -> {now}");
            last = now;
        }
        assert!(last > BASE * QUOTE, "fees must have grown k");
    }

    #[ink::test]
    fn selling_base_lowers_the_price_and_buying_base_raises_it() {
        let (mut dex, pair) = setup();
        let start = dex.get_pool(pair).unwrap().last_price;
        dex.swap_exact_base_for_quote(pair, 5_000, 0).unwrap();
        let after_sell = dex.get_pool(pair).unwrap().last_price;
        assert!(after_sell < start);
        dex.swap_exact_quote_for_base(pair, 60_000, 0).unwrap();
        assert!(dex.get_pool(pair).unwrap().last_price > after_sell);
    }

    #[ink::test]
    fn a_round_trip_returns_less_than_was_put_in() {
        let (mut dex, pair) = setup();
        let quote = dex.swap_exact_base_for_quote(pair, 10_000, 0).unwrap();
        let base_back = dex.swap_exact_quote_for_base(pair, quote, 0).unwrap();
        assert!(base_back < 10_000, "fees mean no free round trip: {base_back}");
    }

    #[ink::test]
    fn larger_trades_get_a_worse_average_price() {
        let (mut small, p1) = setup();
        let (mut large, p2) = setup();
        let out_small = small.swap_exact_base_for_quote(p1, 1_000, 0).unwrap();
        let out_large = large.swap_exact_base_for_quote(p2, 50_000, 0).unwrap();
        // Compare quote-per-base by cross-multiplying.
        assert!(out_small * 50_000 > out_large * 1_000);
    }

    #[ink::test]
    fn a_swap_can_never_drain_the_output_reserve() {
        let (mut dex, pair) = setup();
        // A huge input approaches but never reaches the full reserve.
        let out = dex.swap_exact_base_for_quote(pair, BASE * 1_000, 0).unwrap();
        let (_, quote_left) = reserves(&dex, pair);
        assert!(out < QUOTE);
        assert!(quote_left > 0);
    }

    // ── Guards ──────────────────────────────────────────────────────────

    #[ink::test]
    fn slippage_bounds_are_enforced_exactly() {
        let (mut dex, pair) = setup();
        let expected = quote_out(1_000, BASE, QUOTE, FEE_BIPS);
        assert_eq!(
            dex.swap_exact_base_for_quote(pair, 1_000, expected + 1),
            Err(Error::SlippageExceeded)
        );
        // A rejected swap leaves the pool untouched.
        assert_eq!(reserves(&dex, pair), (BASE, QUOTE));
        assert_eq!(dex.swap_exact_base_for_quote(pair, 1_000, expected), Ok(expected));
    }

    #[ink::test]
    fn zero_input_unknown_pools_and_dust_are_rejected() {
        let (mut dex, pair) = setup();
        assert_eq!(dex.swap_exact_base_for_quote(pair, 0, 0), Err(Error::InvalidOrder));
        assert_eq!(dex.swap_exact_quote_for_base(pair, 0, 0), Err(Error::InvalidOrder));
        assert_eq!(dex.swap_exact_base_for_quote(99, 1_000, 0), Err(Error::PoolNotFound));
        // 1 unit of base is worth less than one quote unit after the fee and rounds to zero out.
        assert_eq!(dex.swap_exact_quote_for_base(pair, 1, 0), Err(Error::SlippageExceeded));
    }

    // ── Liquidity ───────────────────────────────────────────────────────

    #[ink::test]
    fn a_balanced_deposit_mints_proportional_shares_and_keeps_the_ratio() {
        let (mut dex, pair) = setup();
        assert_eq!(dex.add_liquidity(pair, BASE, QUOTE), Ok(200_000));
        assert_eq!(reserves(&dex, pair), (2 * BASE, 2 * QUOTE));
        assert_eq!(dex.get_pool(pair).unwrap().total_lp_shares, 400_000);
    }

    #[ink::test]
    fn an_unbalanced_deposit_is_credited_for_the_smaller_side() {
        let (mut dex, pair) = setup();
        // Base side would mint 100_000 shares, quote side only 50_000.
        assert_eq!(dex.add_liquidity(pair, BASE, QUOTE / 4), Ok(50_000));
    }

    #[ink::test]
    fn removing_liquidity_returns_the_pro_rata_reserves() {
        let (mut dex, pair) = setup();
        assert_eq!(dex.remove_liquidity(pair, 50_000), Ok((25_000, 100_000)));
        assert_eq!(reserves(&dex, pair), (75_000, 300_000));
        assert_eq!(dex.get_pool(pair).unwrap().total_lp_shares, 150_000);
    }

    #[ink::test]
    fn liquidity_operations_validate_their_inputs() {
        let (mut dex, pair) = setup();
        assert_eq!(dex.add_liquidity(pair, 0, 10), Err(Error::InvalidPair));
        assert_eq!(dex.add_liquidity(pair, 10, 0), Err(Error::InvalidPair));
        assert_eq!(dex.remove_liquidity(pair, 0), Err(Error::InvalidPair));
        assert_eq!(dex.remove_liquidity(pair, 200_001), Err(Error::InsufficientLiquidity));

        // A different account holds no shares.
        let accounts = test::default_accounts::<DefaultEnvironment>();
        test::set_caller::<DefaultEnvironment>(accounts.bob);
        assert_eq!(dex.remove_liquidity(pair, 1), Err(Error::InsufficientLiquidity));
    }

    #[ink::test]
    fn swaps_after_a_withdrawal_follow_the_new_reserves() {
        let (mut dex, pair) = setup();
        dex.remove_liquidity(pair, 100_000).unwrap(); // half the pool
        let (b, q) = reserves(&dex, pair);
        let expected = quote_out(1_000, b, q, FEE_BIPS);
        assert_eq!(dex.swap_exact_base_for_quote(pair, 1_000, 0), Ok(expected));
    }
}
