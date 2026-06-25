#[derive(Clone, Debug, PartialEq)]
pub struct LiquidityPosition {
    pub owner: [u8; 32],
    pub pair_id: u64,
    pub lower_tick: i32,
    pub upper_tick: i32,
    pub liquidity: u128,
}

impl LiquidityPosition {
    pub fn new(owner: [u8; 32], pair_id: u64, lower_tick: i32, upper_tick: i32, liquidity: u128) -> Option<Self> {
        if lower_tick >= upper_tick || liquidity == 0 { return None; }
        Some(Self { owner, pair_id, lower_tick, upper_tick, liquidity })
    }

    pub fn is_active(&self, current_tick: i32) -> bool {
        current_tick >= self.lower_tick && current_tick < self.upper_tick
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    const OWNER: [u8; 32] = [1u8; 32];

    #[test]
    fn active_when_tick_in_range() {
        let pos = LiquidityPosition::new(OWNER, 1, -100, 100, 1_000).unwrap();
        assert!(pos.is_active(0));
        assert!(pos.is_active(-100));
        assert!(!pos.is_active(100));
    }

    #[test]
    fn invalid_range_returns_none() {
        assert!(LiquidityPosition::new(OWNER, 1, 100, 100, 1_000).is_none());
        assert!(LiquidityPosition::new(OWNER, 1, 100, -100, 1_000).is_none());
    }

    #[test]
    fn zero_liquidity_returns_none() {
        assert!(LiquidityPosition::new(OWNER, 1, -100, 100, 0).is_none());
    }

    #[test]
    fn inactive_outside_range() {
        let pos = LiquidityPosition::new(OWNER, 1, 0, 50, 500).unwrap();
        assert!(!pos.is_active(-1));
        assert!(!pos.is_active(50));
    }
}