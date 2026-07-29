//! Closes #802: stable `YieldPosition` port for DeFi aggregators (issue
//! suggests `traits/src/yield.rs`, but `yield` is a reserved Rust keyword
//! and can't be used as a module name, so this lives at `yield_position.rs`
//! instead). Starter trait; implementations in lending/insurance/staking
//! contracts are a follow-up.

/// A stable interface aggregators can query across any yield-bearing
/// position (lending deposits, insurance-pool stakes, etc.).
pub trait YieldPosition {
    /// Current value of the position, in the position's base asset units.
    fn value(&self) -> u128;

    /// Annualized yield rate in basis points.
    fn apy_bps(&self) -> u32;

    /// True if the position can be withdrawn without penalty right now.
    fn is_liquid(&self) -> bool;
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FixedPosition {
        value: u128,
        apy_bps: u32,
        liquid: bool,
    }

    impl YieldPosition for FixedPosition {
        fn value(&self) -> u128 {
            self.value
        }
        fn apy_bps(&self) -> u32 {
            self.apy_bps
        }
        fn is_liquid(&self) -> bool {
            self.liquid
        }
    }

    #[test]
    fn exposes_value_apy_and_liquidity() {
        let position = FixedPosition { value: 1_000, apy_bps: 500, liquid: true };
        assert_eq!(position.value(), 1_000);
        assert_eq!(position.apy_bps(), 500);
        assert!(position.is_liquid());
    }
}
