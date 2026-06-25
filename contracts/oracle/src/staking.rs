use ink::storage::Mapping;
use ink::primitives::AccountId;

pub const MIN_ORACLE_STAKE_PERCENT: u128 = 10;
pub const SLASH_PERCENT: u128 = 10;

pub struct OracleStaking {
    pub stakes: Mapping<AccountId, u128>,
    pub total_staked: u128,
    pub slash_pool: u128,
}

impl OracleStaking {
    pub fn stake(&mut self, oracle: AccountId, amount: u128) {
        assert!(amount >= MIN_ORACLE_STAKE, "below minimum stake");
        let current = self.stakes.get(oracle).unwrap_or(0);
        self.stakes.insert(oracle, &(current + amount));
        self.total_staked += amount;
    }

    pub fn unstake(&mut self, oracle: AccountId, amount: u128) {
        let current = self.stakes.get(oracle).unwrap_or(0);
        assert!(current >= amount, "insufficient stake");
        let remaining = current - amount;
        assert!(
            remaining == 0 || remaining >= MIN_ORACLE_STAKE,
            "remaining below minimum"
        );
        self.stakes.insert(oracle, &remaining);
        self.total_staked = self.total_staked.saturating_sub(amount);
    }

    pub fn slash(&mut self, oracle: AccountId) {
        let current = self.stakes.get(oracle).unwrap_or(0);
        if current == 0 { return; }
        let slash = current * SLASH_PERCENT / 100;
        self.stakes.insert(oracle, &(current - slash));
        self.total_staked = self.total_staked.saturating_sub(slash);
        self.slash_pool += slash;
    }

    pub fn get_stake(&self, oracle: AccountId) -> u128 {
        self.stakes.get(oracle).unwrap_or(0)
    }

    pub fn is_eligible(&self, oracle: AccountId) -> bool {
        self.get_stake(oracle) >= MIN_ORACLE_STAKE
    }
}