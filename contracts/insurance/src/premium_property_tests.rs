#[cfg(test)]
mod premium_properties {
    fn calc_premium(base_rate: u128, risk_factor: u128, coverage: u128) -> u128 {
        base_rate.saturating_add(
            risk_factor.saturating_mul(coverage).saturating_div(1_000)
        )
    }

    #[test]
    fn monotone_in_coverage() {
        for cov in [100u128, 500, 1000, 5000, 10_000] {
            assert!(calc_premium(10, 5, cov + 100) >= calc_premium(10, 5, cov));
        }
    }

    #[test]
    fn monotone_in_risk_factor() {
        for risk in [1u128, 2, 5, 10, 20] {
            assert!(calc_premium(10, risk + 1, 1000) >= calc_premium(10, risk, 1000));
        }
    }

    #[test]
    fn bounded_below_by_base_rate() {
        let base = 50u128;
        for cov in [0u128, 1, 100, 10_000] {
            assert!(calc_premium(base, 1, cov) >= base);
        }
    }

    #[test]
    fn zero_coverage_yields_base_rate() {
        assert_eq!(calc_premium(100, 5, 0), 100);
    }

    #[test]
    fn saturating_prevents_overflow() {
        assert_eq!(calc_premium(u128::MAX, 1, 1), u128::MAX);
    }
}