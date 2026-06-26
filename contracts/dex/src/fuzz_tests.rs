// Property-based fuzz tests for DEX swap functions (Issue #480)

#[cfg(test)]
mod fuzz_tests {
    use super::*;
    use ink::env::{test, DefaultEnvironment};

    fn setup_dex() -> PropertyDex {
        let mut dex = PropertyDex::new(String::from("PCG"), 1_000_000, 25, 1_000);
        dex.configure_bridge_route(2, 120_000, 400)
            .expect("bridge route config should work");
        dex
    }

    fn create_pool(dex: &mut PropertyDex) -> u64 {
        dex.create_pool(1, 2, 30, 10_000, 20_000)
            .expect("pool creation should work")
    }

    /// Invariant: k-value (reserve_base * reserve_quote) never decreases after a swap.
    /// The constant-product formula ensures k should stay >= its previous value.
    #[ink::test]
    fn invariant_k_value_never_decreases() {
        let mut dex = setup_dex();
        let pair_id = create_pool(&mut dex);

        let pool = dex.get_pool(pair_id).expect("pool exists");
        let initial_k = pool.reserve_base * pool.reserve_quote;

        for amount_in in [100u128, 500, 1000, 5000] {
            let result = dex.swap_exact_base_for_quote(pair_id, amount_in, 0);
            if let Ok(_) = result {
                let pool = dex.get_pool(pair_id).expect("pool exists");
                let k = pool.reserve_base * pool.reserve_quote;
                assert!(
                    k >= initial_k,
                    "k-value decreased after base->quote swap: {} < {}", k, initial_k
                );
            }
        }

        let pool = dex.get_pool(pair_id).expect("pool exists");
        let initial_k = pool.reserve_base * pool.reserve_quote;

        for amount_in in [100u128, 500, 1000, 5000] {
            let result = dex.swap_exact_quote_for_base(pair_id, amount_in, 0);
            if let Ok(_) = result {
                let pool = dex.get_pool(pair_id).expect("pool exists");
                let k = pool.reserve_base * pool.reserve_quote;
                assert!(
                    k >= initial_k,
                    "k-value decreased after quote->base swap: {} < {}", k, initial_k
                );
            }
        }
    }

    /// Invariant: zero-amount swaps are always rejected.
    #[ink::test]
    fn invariant_zero_amount_rejected() {
        let mut dex = setup_dex();
        let pair_id = create_pool(&mut dex);

        let result = dex.swap_exact_base_for_quote(pair_id, 0, 0);
        assert_eq!(
            result,
            Err(Error::InvalidOrder),
            "zero base->quote swap should be rejected"
        );

        let result = dex.swap_exact_quote_for_base(pair_id, 0, 0);
        assert_eq!(
            result,
            Err(Error::InvalidOrder),
            "zero quote->base swap should be rejected"
        );
    }

    /// Invariant: setting an impossibly high min output always triggers SlippageExceeded.
    #[ink::test]
    fn invariant_slippage_protection() {
        let mut dex = setup_dex();
        let pair_id = create_pool(&mut dex);

        for amount_in in [100u128, 500, 1000, 5000] {
            let result = dex.swap_exact_base_for_quote(pair_id, amount_in, u128::MAX);
            assert_eq!(
                result,
                Err(Error::SlippageExceeded),
                "slippage should be exceeded for amount_in={}",
                amount_in
            );

            let result = dex.swap_exact_quote_for_base(pair_id, amount_in, u128::MAX);
            assert_eq!(
                result,
                Err(Error::SlippageExceeded),
                "slippage should be exceeded for amount_in={}",
                amount_in
            );
        }
    }

    /// Invariant: swap output is monotonic with respect to input amount.
    /// Larger amount_in should always produce larger amount_out.
    #[ink::test]
    fn invariant_swap_monotonicity() {
        let mut dex = setup_dex();
        let pair_id = create_pool(&mut dex);

        let amounts = [100u128, 500, 1000, 2000, 4000];
        let mut prev_out = 0u128;
        for &amount_in in &amounts {
            let result = dex.swap_exact_base_for_quote(pair_id, amount_in, 0);
            if let Ok(amount_out) = result {
                assert!(
                    amount_out > prev_out,
                    "monotonicity violation: amount_out for {} should be > previous {}",
                    amount_in,
                    prev_out
                );
                prev_out = amount_out;
            }
        }
    }
}
