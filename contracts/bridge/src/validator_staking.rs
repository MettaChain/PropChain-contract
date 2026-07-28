use ink::prelude::vec::Vec;
use ink::storage::Mapping;
use ink::primitives::AccountId;

pub const MIN_VALIDATOR_STAKE: u128 = 10_000_000;
pub const SLASH_PERCENT: u128 = 20;

pub struct ValidatorStaking {
    pub stakes: Mapping<AccountId, u128>,
    pub validators: Vec<AccountId>,
    pub total_staked: u128,
    pub slash_pool: u128,
}

impl ValidatorStaking {
    pub fn stake(&mut self, validator: AccountId, amount: u128) {
        assert!(amount >= MIN_VALIDATOR_STAKE, "below minimum stake");
        let current = self.stakes.get(validator).unwrap_or(0);
        if current == 0 { self.validators.push(validator); }
        self.stakes.insert(validator, &(current + amount));
        self.total_staked += amount;
    }

    pub fn unstake(&mut self, validator: AccountId, amount: u128) {
        let current = self.stakes.get(validator).unwrap_or(0);
        assert!(current >= amount, "insufficient stake");
        let remaining = current - amount;
        self.stakes.insert(validator, &remaining);
        if remaining == 0 { self.validators.retain(|&v| v != validator); }
        self.total_staked = self.total_staked.saturating_sub(amount);
    }

    pub fn slash(&mut self, validator: AccountId) {
        let current = self.stakes.get(validator).unwrap_or(0);
        if current == 0 { return; }
        let slash = current * SLASH_PERCENT / 100;
        self.stakes.insert(validator, &(current - slash));
        self.total_staked = self.total_staked.saturating_sub(slash);
        self.slash_pool += slash;
    }

    pub fn is_active(&self, validator: AccountId) -> bool {
        self.stakes.get(validator).unwrap_or(0) >= MIN_VALIDATOR_STAKE
    }

    pub fn count(&self) -> u32 { self.validators.len() as u32 }
}
