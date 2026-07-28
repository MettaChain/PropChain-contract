#[derive(Clone, Default)]
pub struct RewardSnapshot {
    pub staker: [u8; 32],
    pub accrued_per_token: u128,
    pub last_updated_block: u64,
}

impl RewardSnapshot {
    pub fn new(staker: [u8; 32]) -> Self {
        Self { staker, accrued_per_token: 0, last_updated_block: 0 }
    }

    pub fn pending(&self, global_accrued: u128, staked: u128) -> u128 {
        global_accrued
            .saturating_sub(self.accrued_per_token)
            .saturating_mul(staked)
            .saturating_div(1_000_000_000)
    }

    pub fn flush(&mut self, global_accrued: u128, block: u64) {
        self.accrued_per_token = global_accrued;
        self.last_updated_block = block;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    const S: [u8; 32] = [1u8; 32];

    #[test]
    fn pending_before_flush_is_positive() {
        let snap = RewardSnapshot { staker: S, accrued_per_token: 0, last_updated_block: 0 };
        assert!(snap.pending(500_000_000, 2_000_000_000) > 0);
    }

    #[test]
    fn no_pending_after_flush() {
        let mut snap = RewardSnapshot::new(S);
        snap.flush(500_000_000, 100);
        assert_eq!(snap.pending(500_000_000, 1_000_000_000), 0);
    }

    #[test]
    fn rewards_accumulate_between_flushes() {
        let mut snap = RewardSnapshot::new(S);
        snap.flush(100_000_000, 1);
        assert!(snap.pending(200_000_000, 1_000_000_000) > 0);
    }

    #[test]
    fn zero_stake_yields_no_pending() {
        let snap = RewardSnapshot::new(S);
        assert_eq!(snap.pending(1_000_000_000, 0), 0);
    }
}