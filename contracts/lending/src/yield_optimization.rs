// SPDX-License-Identifier: MIT
pub struct YieldOptimizationStrategy {
    pub pool_id: u128,
    pub target_apy_bps: u32,
    pub rebalance_threshold_bps: u32,
}

impl YieldOptimizationStrategy {
    pub fn new(pool_id: u128, target_apy_bps: u32, rebalance_threshold_bps: u32) -> Self {
        Self {
            pool_id,
            target_apy_bps,
            rebalance_threshold_bps,
        }
    }

    pub fn should_rebalance(&self, current_apy_bps: u32) -> bool {
        if current_apy_bps > self.target_apy_bps {
            current_apy_bps - self.target_apy_bps >= self.rebalance_threshold_bps
        } else {
            self.target_apy_bps - current_apy_bps >= self.rebalance_threshold_bps
        }
    }
}
