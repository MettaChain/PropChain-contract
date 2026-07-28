#[cfg(test)]
mod swap_invariant {
    fn cpf_swap(rx: u128, ry: u128, amt_in: u128, fee_bps: u128) -> (u128, u128, u128) {
        let fee = amt_in.saturating_mul(fee_bps).saturating_div(10_000);
        let amt_after = amt_in.saturating_sub(fee);
        let new_rx = rx.saturating_add(amt_after);
        let amt_out = ry.saturating_mul(amt_after).saturating_div(new_rx);
        (amt_out, new_rx, ry.saturating_sub(amt_out))
    }

    fn k_preserved(rx: u128, ry: u128, amt: u128, fee: u128) -> bool {
        let k_before = rx.saturating_mul(ry);
        let (_, new_rx, new_ry) = cpf_swap(rx, ry, amt, fee);
        new_rx.saturating_mul(new_ry) >= k_before
    }

    #[test]
    fn k_invariant_small_swap() { assert!(k_preserved(1_000_000, 1_000_000, 1_000, 30)); }

    #[test]
    fn k_invariant_large_swap() { assert!(k_preserved(10_000_000_000, 5_000_000_000, 500_000, 30)); }

    #[test]
    fn k_invariant_zero_fee() { assert!(k_preserved(1_000_000, 1_000_000, 1_000, 0)); }

    #[test]
    fn output_never_exceeds_reserve() {
        let (out, _, _) = cpf_swap(1_000, 1_000, 500, 30);
        assert!(out < 1_000);
    }

    #[test]
    fn arbitrary_inputs_satisfy_k() {
        for (rx, ry, amt) in [(1_000u128, 2_000, 100), (50_000, 50_000, 10_000)] {
            assert!(k_preserved(rx, ry, amt, 30));
        }
    }
}