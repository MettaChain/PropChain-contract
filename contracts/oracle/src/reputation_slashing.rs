use propchain_traits::AccountId;

pub struct OracleSlashingManager {
    pub min_reputation_score: u32,
}

impl Default for OracleSlashingManager {
    fn default() -> Self {
        Self::new(50)
    }
}

impl OracleSlashingManager {
    pub fn new(min_reputation_score: u32) -> Self {
        Self {
            min_reputation_score,
        }
    }

    pub fn slash_malicious_oracle(
        &self,
        _oracle: AccountId,
        current_score: u32,
        slash_amount: u128,
    ) -> (u32, u128) {
        let new_score = current_score.saturating_sub(20);
        (new_score, slash_amount)
    }
}
