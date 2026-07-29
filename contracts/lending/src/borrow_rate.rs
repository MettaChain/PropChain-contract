//! Closes #803: public `compute_borrow_rate` view so off-chain tools can
//! quote without simulating contract state. Starter pure function using a
//! simple linear model; wiring as a public contract message + docs formula
//! is a follow-up.

const BASE_RATE_BPS: u32 = 200; // 2%
const SLOPE_BPS: u32 = 1_000; // +10% at 100% utilisation

/// Computes the borrow rate (in basis points) for a given utilisation,
/// expressed in basis points (0-10_000).
pub fn compute_borrow_rate(utilisation_bps: u32) -> u32 {
    let utilisation_bps = utilisation_bps.min(10_000);
    BASE_RATE_BPS + (SLOPE_BPS as u64 * utilisation_bps as u64 / 10_000) as u32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zero_utilisation_yields_base_rate() {
        assert_eq!(compute_borrow_rate(0), BASE_RATE_BPS);
    }

    #[test]
    fn full_utilisation_yields_base_plus_slope() {
        assert_eq!(compute_borrow_rate(10_000), BASE_RATE_BPS + SLOPE_BPS);
    }

    #[test]
    fn utilisation_above_100_percent_is_clamped() {
        assert_eq!(compute_borrow_rate(50_000), compute_borrow_rate(10_000));
    }
}
