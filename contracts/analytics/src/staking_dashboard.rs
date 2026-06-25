use ink::storage::Mapping;
use ink::primitives::AccountId;

#[derive(scale::Encode, scale::Decode, Clone, Debug)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub struct StakingStats {
    pub total_staked: u128,
    pub total_stakers: u32,
    pub average_stake: u128,
    pub rewards_distributed: u128,
}

pub struct StakingDashboard {
    pub staker_balances: Mapping<AccountId, u128>,
    pub total_staked: u128,
    pub total_stakers: u32,
    pub rewards_distributed: u128,
}

impl StakingDashboard {
    pub fn get_stats(&self) -> StakingStats {
        StakingStats {
            total_staked: self.total_staked,
            total_stakers: self.total_stakers,
            average_stake: if self.total_stakers > 0 {
                self.total_staked / self.total_stakers as u128
            } else {
                0
            },
            rewards_distributed: self.rewards_distributed,
        }
    }

    pub fn record_stake(&mut self, staker: AccountId, amount: u128) {
        let prev = self.staker_balances.get(staker).unwrap_or(0);
        if prev == 0 { self.total_stakers += 1; }
        self.staker_balances.insert(staker, &(prev + amount));
        self.total_staked += amount;
    }

    pub fn record_unstake(&mut self, staker: AccountId, amount: u128) {
        let prev = self.staker_balances.get(staker).unwrap_or(0);
        let updated = prev.saturating_sub(amount);
        self.staker_balances.insert(staker, &updated);
        if updated == 0 && prev > 0 { self.total_stakers = self.total_stakers.saturating_sub(1); }
        self.total_staked = self.total_staked.saturating_sub(amount);
    }

    pub fn record_reward(&mut self, amount: u128) { self.rewards_distributed += amount; }

    pub fn staker_balance(&self, staker: AccountId) -> u128 {
        self.staker_balances.get(staker).unwrap_or(0)
    }
}
