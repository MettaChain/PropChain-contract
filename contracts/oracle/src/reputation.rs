use ink::storage::Mapping;
use ink::primitives::AccountId;

pub const MAX_SCORE: u32 = 1000;
pub const INITIAL_SCORE: u32 = 500;
pub const REWARD: u32 = 10;
pub const PENALTY: u32 = 25;

#[derive(scale::Encode, scale::Decode, Clone, Debug, Default)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub struct OracleRecord {
    pub score: u32,
    pub submissions: u32,
    pub correct: u32,
}

pub struct OracleReputation {
    pub records: Mapping<AccountId, OracleRecord>,
}

impl OracleReputation {
    pub fn record(&mut self, oracle: AccountId, correct: bool) {
        let mut rec = self.records.get(oracle).unwrap_or(OracleRecord {
            score: INITIAL_SCORE,
            submissions: 0,
            correct: 0,
        });
        rec.submissions += 1;
        if correct {
            rec.correct += 1;
            rec.score = (rec.score + REWARD).min(MAX_SCORE);
        } else {
            rec.score = rec.score.saturating_sub(PENALTY);
        }
        self.records.insert(oracle, &rec);
    }

    pub fn score(&self, oracle: AccountId) -> u32 {
        self.records.get(oracle).map(|r| r.score).unwrap_or(INITIAL_SCORE)
    }

    pub fn accuracy(&self, oracle: AccountId) -> u32 {
        let rec = self.records.get(oracle).unwrap_or_default();
        if rec.submissions == 0 { return 100; }
        rec.correct * 100 / rec.submissions
    }

    pub fn weight(&self, oracle: AccountId, total_weight: u32) -> u32 {
        let raw_weight = self.score(oracle) * self.accuracy(oracle) / 100;
        if total_weight == 0 {
            return 0;
        }
        (raw_weight as u64 * 1000 / total_weight as u64) as u32
    }
}