
//! Unit tests for DEX pool swap math and invariants (#1449).
//!
//! The model under test uses the constant-product formula `x * y = k`.
//! Trading fees are deducted from the input when calculating output:
//!
//! `in_after_fee = in * (10_000 - fee_bips) / 10_000`
//! `out = in_after_fee * reserve_out / (reserve_in + in_after_fee)`
//!
//! The entire input is added to the pool reserves, meaning fees remain
//! in the pool and the constant product can only increase.
//!
//! Tests cover pool creation, swap calculations, price movements,
//! constant-product invariants, slippage protection, and liquidity management.

#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(test)]
mod tests {
    use ink::env::{test, DefaultEnvironment};

    use propchain_dex::dex::{Error, PropertyDex};

    // ---------------------------------------------------------------------
    // Test constants
    // ---------------------------------------------------------------------

    const BASE: u128 = 100_000;
    const QUOTE: u128 = 400_000;
    const FEE_BIPS: u32 = 30;
    const FEE_DENOMINATOR: u128 = 10_000;

    // ---------------------------------------------------------------------
    // Test helpers
    // ---------------------------------------------------------------------

    /// Calculates the expected swap output using the constant-product formula.
    ///
    /// This serves as an independent reference implementation (oracle)
    /// for validating the contract's swap calculations.
    fn quote_out(
        amount_in: u128,
        reserve_in: u128,
        reserve_out: u128,
        fee_bips: u32,
    ) -> u128 {
        let fee_adjusted_input =
            amount_in * (FEE_DENOMINATOR - fee_bips as u128) / FEE_DENOMINATOR;

        fee_adjusted_input * reserve_out / (reserve_in + fee_adjusted_input)
    }

    /// Creates a DEX instance and initializes a pool with the specified fee.
    fn setup_with_fee(fee: u32) -> (PropertyDex, u64) {
        let accounts = test::default_accounts::<DefaultEnvironment>();

        test::set_caller::<DefaultEnvironment>(accounts.alice);

        let mut dex = PropertyDex::new("PRPX".into(), 1_000_000, 10, 4_000);

        let pair = dex
            .create_pool(1, 2, fee, BASE, QUOTE)
            .expect("pool creation should succeed");

        (dex, pair)
    }

    /// Creates a DEX instance using the default test fee.
    fn setup() -> (PropertyDex, u64) {
        setup_with_fee(FEE_BIPS)
    }

    /// Returns the current base and quote reserves for a pool.
    fn reserves(dex: &PropertyDex, pair: u64) -> (u128, u128) {
        let pool = dex.get_pool(pair).expect("pool should exist");

        (pool.reserve_base, pool.reserve_quote)
    }

    /// Creates an additional pool in the same DEX instance.
    ///
    /// Multiple pools are kept in one instance because ink! off-chain
    /// tests share Mapping storage between contract instances.
    fn add_pool(dex: &mut PropertyDex, fee: u32) -> u64 {
        dex.create_pool(3, 4, fee, BASE, QUOTE)
            .expect("additional pool creation should succeed")
    }

    // ---------------------------------------------------------------------
    // Pool creation
    // ---------------------------------------------------------------------

    #[ink::test]
    fn a_new_pool_mints_the_integer_square_root_of_the_reserve_product() {
        let (dex, pair) = setup();

        let pool = dex.get_pool(pair).unwrap();

        assert_eq!(
            (pool.reserve_base, pool.reserve_quote),
            (BASE, QUOTE)
        );
        assert_eq!(pool.total_lp_shares, 200_000);
        assert_eq!(pool.fee_bips, FEE_BIPS);
    }

    #[ink::test]
    fn invalid_pools_are_rejected() {
        let accounts = test::default_accounts::<DefaultEnvironment>();

        test::set_caller::<DefaultEnvironment>(accounts.alice);

        let mut dex = PropertyDex::new("PRPX".into(), 1_000_000, 10, 4_000);

        // A pool cannot contain identical tokens.
        assert_eq!(
            dex.create_pool(1, 1, FEE_BIPS, BASE, QUOTE),
            Err(Error::InvalidPair)
        );

        // Both initial reserves must be greater than zero.
        assert_eq!(
            dex.create_pool(1, 2, FEE_BIPS, 0, QUOTE),
            Err(Error::InvalidPair)
        );

        assert_eq!(
            dex.create_pool(1, 2, FEE_BIPS, BASE, 0),
            Err(Error::InvalidPair)
        );

        // Fees above the supported limit must be rejected.
        assert_eq!(
            dex.create_pool(1, 2, 1_000, BASE, QUOTE),
            Err(Error::InvalidPair)
        );
    }

    // ---------------------------------------------------------------------
    // Swap output calculations
    // ---------------------------------------------------------------------

    #[ink::test]
    fn selling_base_pays_the_formula_amount_and_updates_reserves() {
        let (mut dex, pair) = setup();

        let expected = quote_out(1_000, BASE, QUOTE, FEE_BIPS);

        assert_eq!(expected, 3_948);
        assert_eq!(
            dex.swap_exact_base_for_quote(pair, 1_000, 0),
            Ok(expected)
        );

        // The entire input enters the pool; only the output is removed.
        assert_eq!(
            reserves(&dex, pair),
            (BASE + 1_000, QUOTE - expected)
        );
    }

    #[ink::test]
    fn buying_base_with_quote_pays_the_formula_amount() {
        let (mut dex, pair) = setup();

        let expected = quote_out(4_000, QUOTE, BASE, FEE_BIPS);

        assert_eq!(expected, 987);
        assert_eq!(
            dex.swap_exact_quote_for_base(pair, 4_000, 0),
            Ok(expected)
        );

        assert_eq!(
            reserves(&dex, pair),
            (BASE - expected, QUOTE + 4_000)
        );
    }

    #[ink::test]
    fn a_higher_fee_pays_out_less_for_the_same_input() {
        let (mut dex, low_fee_pair) = setup_with_fee(30);
        let high_fee_pair = add_pool(&mut dex, 300);

        let out_low = dex
            .swap_exact_base_for_quote(low_fee_pair, 10_000, 0)
            .unwrap();

        let out_high = dex
            .swap_exact_base_for_quote(high_fee_pair, 10_000, 0)
            .unwrap();

        assert!(out_low > out_high);

        assert_eq!(out_low, quote_out(10_000, BASE, QUOTE, 30));
        assert_eq!(out_high, quote_out(10_000, BASE, QUOTE, 300));
    }

    // ---------------------------------------------------------------------
    // Constant-product and pricing invariants
    // ---------------------------------------------------------------------

    #[ink::test]
    fn the_constant_product_never_decreases_across_swaps_in_either_direction() {
        let (mut dex, pair) = setup();

        let calculate_k = |dex: &PropertyDex| {
            let (base_reserve, quote_reserve) = reserves(dex, pair);
            base_reserve * quote_reserve
        };

        let mut previous_k = calculate_k(&dex);

        for step in 0..6u32 {
            if step % 2 == 0 {
                let amount_in = 2_000 + step as u128 * 500;

                dex.swap_exact_base_for_quote(pair, amount_in, 0)
                    .unwrap();
            } else {
                let amount_in = 9_000 + step as u128 * 700;

                dex.swap_exact_quote_for_base(pair, amount_in, 0)
                    .unwrap();
            }

            let current_k = calculate_k(&dex);

            assert!(
                current_k >= previous_k,
                "constant product decreased at step {step}: {previous_k} -> {current_k}"
            );

            previous_k = current_k;
        }

        assert!(
            previous_k > BASE * QUOTE,
            "trading fees should increase the constant product"
        );
    }

    #[ink::test]
    fn selling_base_lowers_the_price_and_buying_base_raises_it() {
        let (mut dex, pair) = setup();

        let initial_price = dex.get_pool(pair).unwrap().last_price;

        dex.swap_exact_base_for_quote(pair, 5_000, 0)
            .unwrap();

        let price_after_sell = dex.get_pool(pair).unwrap().last_price;

        assert!(price_after_sell < initial_price);

        dex.swap_exact_quote_for_base(pair, 60_000, 0)
            .unwrap();

        let price_after_buy = dex.get_pool(pair).unwrap().last_price;

        assert!(price_after_buy > price_after_sell);
    }

    #[ink::test]
    fn a_round_trip_returns_less_than_was_put_in() {
        let (mut dex, pair) = setup();

        let quote_received = dex
            .swap_exact_base_for_quote(pair, 10_000, 0)
            .unwrap();

        let base_returned = dex
            .swap_exact_quote_for_base(pair, quote_received, 0)
            .unwrap();

        assert!(
            base_returned < 10_000,
            "trading fees should prevent a profitable round trip: {base_returned}"
        );
    }

    #[ink::test]
    fn larger_trades_get_a_worse_average_price() {
        let (mut dex, small_trade_pair) = setup();
        let large_trade_pair = add_pool(&mut dex, FEE_BIPS);

        let out_small = dex
            .swap_exact_base_for_quote(small_trade_pair, 1_000, 0)
            .unwrap();

        let out_large = dex
            .swap_exact_base_for_quote(large_trade_pair, 50_000, 0)
            .unwrap();

        // Compare average quote-per-base output using cross multiplication.
        assert!(out_small * 50_000 > out_large * 1_000);
    }

    #[ink::test]
    fn a_swap_can_never_drain_the_output_reserve() {
        let (mut dex, pair) = setup();

        // A sufficiently large input approaches but cannot consume the
        // entire output reserve under the constant-product formula.
        let output = dex
            .swap_exact_base_for_quote(pair, BASE * 1_000, 0)
            .unwrap();

        let (_, remaining_quote) = reserves(&dex, pair);

        assert!(output < QUOTE);
        assert!(remaining_quote > 0);
    }

    // ---------------------------------------------------------------------
    // Swap validation and slippage protection
    // ---------------------------------------------------------------------

    #[ink::test]
    fn slippage_bounds_are_enforced_exactly() {
        let (mut dex, pair) = setup();

        let expected = quote_out(1_000, BASE, QUOTE, FEE_BIPS);

        assert_eq!(
            dex.swap_exact_base_for_quote(pair, 1_000, expected + 1),
            Err(Error::SlippageExceeded)
        );

        // A rejected swap must not modify the pool reserves.
        assert_eq!(reserves(&dex, pair), (BASE, QUOTE));

        // The exact expected output remains an acceptable minimum.
        assert_eq!(
            dex.swap_exact_base_for_quote(pair, 1_000, expected),
            Ok(expected)
        );
    }

    #[ink::test]
    fn zero_input_unknown_pools_and_dust_are_rejected() {
        let (mut dex, pair) = setup();

        assert_eq!(
            dex.swap_exact_base_for_quote(pair, 0, 0),
            Err(Error::InvalidOrder)
        );

        assert_eq!(
            dex.swap_exact_quote_for_base(pair, 0, 0),
            Err(Error::InvalidOrder)
        );

        assert_eq!(
            dex.swap_exact_base_for_quote(99, 1_000, 0),
            Err(Error::PoolNotFound)
        );

        // One unit of quote rounds down to zero base output.
        assert_eq!(
            dex.swap_exact_quote_for_base(pair, 1, 0),
            Err(Error::SlippageExceeded)
        );
    }

    // ---------------------------------------------------------------------
    // Liquidity management
    // ---------------------------------------------------------------------

    #[ink::test]
    fn a_balanced_deposit_mints_proportional_shares_and_keeps_the_ratio() {
        let (mut dex, pair) = setup();

        assert_eq!(
            dex.add_liquidity(pair, BASE, QUOTE),
            Ok(200_000)
        );

        assert_eq!(
            reserves(&dex, pair),
            (2 * BASE, 2 * QUOTE)
        );

        assert_eq!(
            dex.get_pool(pair).unwrap().total_lp_shares,
            400_000
        );
    }

    #[ink::test]
    fn an_unbalanced_deposit_is_credited_for_the_smaller_side() {
        let (mut dex, pair) = setup();

        // Base would mint 100,000 shares, while quote would mint 50,000.
        // The deposit is therefore credited using the smaller contribution.
        assert_eq!(
            dex.add_liquidity(pair, BASE, QUOTE / 4),
            Ok(50_000)
        );
    }

    #[ink::test]
    fn removing_liquidity_returns_the_pro_rata_reserves() {
        let (mut dex, pair) = setup();

        assert_eq!(
            dex.remove_liquidity(pair, 50_000),
            Ok((25_000, 100_000))
        );

        assert_eq!(
            reserves(&dex, pair),
            (75_000, 300_000)
        );

        assert_eq!(
            dex.get_pool(pair).unwrap().total_lp_shares,
            150_000
        );
    }

    #[ink::test]
    fn liquidity_operations_validate_their_inputs() {
        let (mut dex, pair) = setup();

        assert_eq!(
            dex.add_liquidity(pair, 0, 10),
            Err(Error::InvalidPair)
        );

        assert_eq!(
            dex.add_liquidity(pair, 10, 0),
            Err(Error::InvalidPair)
        );

        assert_eq!(
            dex.remove_liquidity(pair, 0),
            Err(Error::InvalidPair)
        );

        assert_eq!(
            dex.remove_liquidity(pair, 200_001),
            Err(Error::InsufficientLiquidity)
        );

        // An account without liquidity shares cannot withdraw from the pool.
        let accounts = test::default_accounts::<DefaultEnvironment>();

        test::set_caller::<DefaultEnvironment>(accounts.bob);

        assert_eq!(
            dex.remove_liquidity(pair, 1),
            Err(Error::InsufficientLiquidity)
        );
    }

    #[ink::test]
    fn swaps_after_a_withdrawal_follow_the_new_reserves() {
        let (mut dex, pair) = setup();

        // Remove half of the initial liquidity.
        dex.remove_liquidity(pair, 100_000).unwrap();

        let (base_reserve, quote_reserve) = reserves(&dex, pair);

        let expected = quote_out(
            1_000,
            base_reserve,
            quote_reserve,
            FEE_BIPS,
        );

        assert_eq!(
            dex.swap_exact_base_for_quote(pair, 1_000, 0),
            Ok(expected)
        );
    }
}
