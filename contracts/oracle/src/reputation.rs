use ink::storage::Mapping;
use ink::primitives::AccountId;

/// Upper bound on an oracle's reputation score.
pub const MAX_SCORE: u32 = 1000;
/// Score assigned to an oracle with no submission history yet.
pub const INITIAL_SCORE: u32 = 500;
/// Score added for a correct submission.
pub const REWARD: u32 = 10;
/// Score subtracted for an incorrect submission.
///
/// Deliberately larger than REWARD: punishing bad data harder than we
/// reward good data biases the system toward caution, since a wrong
/// oracle answer is more costly to consumers than a missed reward is to
/// the oracle.
pub const PENALTY: u32 = 25;

/// Per-oracle reputation record.
#[derive(scale::Encode, scale::Decode, Clone, Debug, Default, PartialEq, Eq)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub struct OracleRecord {
    pub score: u32,
    pub submissions: u32,
    pub correct: u32,
}

impl OracleRecord {
    fn fresh() -> Self {
        Self { score: INITIAL_SCORE, submissions: 0, correct: 0 }
    }

    /// Accuracy as a percentage (0-100), truncated toward zero.
    ///
    /// An oracle with no submissions is reported at 100% rather than 0%,
    /// since "no track record" and "always wrong" should not look
    /// identical to a caller. If you need to distinguish "unproven" from
    /// "proven good", check `submissions == 0` separately rather than
    /// reading this value alone.
    pub fn accuracy(&self) -> u32 {
        if self.submissions == 0 {
            100
        } else {
            self.correct * 100 / self.submissions
        }
    }

    /// Combined score * accuracy weighting, used to rank oracle influence.
    pub fn weight(&self) -> u32 {
        self.score * self.accuracy() / 100
    }
}

/// Emitted whenever an oracle's reputation is updated, so off-chain
/// indexers/dashboards don't have to poll storage to see changes.
#[derive(scale::Encode, scale::Decode, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub struct ReputationUpdated {
    pub oracle: AccountId,
    pub correct: bool,
    pub new_score: u32,
    pub submissions: u32,
}

pub struct OracleReputation {
    pub records: Mapping<AccountId, OracleRecord>,
}

impl Default for OracleReputation {
    fn default() -> Self {
        Self { records: Mapping::default() }
    }
}

impl OracleReputation {
    pub fn new() -> Self {
        Self::default()
    }

    /// Record a single oracle's submission outcome and return the updated
    /// record so callers (e.g. the contract's public method) can emit an
    /// event without a second storage read.
    pub fn record(&mut self, oracle: AccountId, correct: bool) -> OracleRecord {
        let mut rec = self.records.get(oracle).unwrap_or_else(OracleRecord::fresh);

        rec.submissions += 1;
        if correct {
            rec.correct += 1;
            rec.score = rec.score.saturating_add(REWARD).min(MAX_SCORE);
        } else {
            rec.score = rec.score.saturating_sub(PENALTY);
        }

        self.records.insert(oracle, &rec);
        rec
    }

    /// Record outcomes for several oracles in one pass (e.g. all oracles
    /// who reported on the same round). Returns each oracle's updated
    /// record in input order.
    pub fn record_batch(&mut self, outcomes: &[(AccountId, bool)]) -> ink::prelude::vec::Vec<OracleRecord> {
        outcomes
            .iter()
            .map(|(oracle, correct)| self.record(*oracle, *correct))
            .collect()
    }

    /// Full record for an oracle, defaulting to a fresh record if none
    /// exists yet. Prefer this over calling `score`/`accuracy` separately
    /// when you need more than one field, to avoid redundant storage reads.
    pub fn get(&self, oracle: AccountId) -> OracleRecord {
        self.records.get(oracle).unwrap_or_else(OracleRecord::fresh)
    }

    pub fn score(&self, oracle: AccountId) -> u32 {
        self.get(oracle).score
    }

    pub fn accuracy(&self, oracle: AccountId) -> u32 {
        self.get(oracle).accuracy()
    }

    pub fn weight(&self, oracle: AccountId) -> u32 {
        self.get(oracle).weight()
    }

    /// Whether an oracle has submitted at least once. Useful for
    /// distinguishing "new oracle at default score" from "established
    /// oracle that happens to have drifted back to the default."
    pub fn has_history(&self, oracle: AccountId) -> bool {
        self.records.get(oracle).map(|r| r.submissions > 0).unwrap_or(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn acc(byte: u8) -> AccountId {
        AccountId::from([byte; 32])
    }

    #[test]
    fn new_oracle_starts_at_initial_score_with_full_accuracy() {
        let rep = OracleReputation::new();
        let o = acc(1);
        assert_eq!(rep.score(o), INITIAL_SCORE);
        assert_eq!(rep.accuracy(o), 100);
        assert!(!rep.has_history(o));
    }

    #[test]
    fn correct_submission_increases_score_and_accuracy() {
        let mut rep = OracleReputation::new();
        let o = acc(1);

        let rec = rep.record(o, true);
        assert_eq!(rec.score, INITIAL_SCORE + REWARD);
        assert_eq!(rec.submissions, 1);
        assert_eq!(rec.correct, 1);
        assert_eq!(rep.accuracy(o), 100);
        assert!(rep.has_history(o));
    }

    #[test]
    fn incorrect_submission_decreases_score() {
        let mut rep = OracleReputation::new();
        let o = acc(1);

        let rec = rep.record(o, false);
        assert_eq!(rec.score, INITIAL_SCORE - PENALTY);
        assert_eq!(rec.submissions, 1);
        assert_eq!(rec.correct, 0);
        assert_eq!(rep.accuracy(o), 0);
    }

    #[test]
    fn score_saturates_at_max_score() {
        let mut rep = OracleReputation::new();
        let o = acc(1);

        for _ in 0..200 {
            rep.record(o, true);
        }
        assert_eq!(rep.score(o), MAX_SCORE);
    }

    #[test]
    fn score_saturates_at_zero_not_underflow() {
        let mut rep = OracleReputation::new();
        let o = acc(1);

        for _ in 0..200 {
            rep.record(o, false);
        }
        assert_eq!(rep.score(o), 0);
    }

    #[test]
    fn accuracy_truncates_rather_than_rounds() {
        let mut rep = OracleReputation::new();
        let o = acc(1);

        rep.record(o, true);  // 1/1
        rep.record(o, false); // 1/2 -> 50
        rep.record(o, false); // 1/3 -> 33 (truncated, not 33.33 or 34)

        assert_eq!(rep.accuracy(o), 33);
    }

    #[test]
    fn weight_combines_score_and_accuracy() {
        let mut rep = OracleReputation::new();
        let o = acc(1);

        rep.record(o, true);
        let rec = rep.get(o);
        let expected = rec.score * rec.accuracy() / 100;
        assert_eq!(rep.weight(o), expected);
    }

    #[test]
    fn independent_oracles_do_not_affect_each_other() {
        let mut rep = OracleReputation::new();
        let (a, b) = (acc(1), acc(2));

        rep.record(a, true);
        rep.record(b, false);

        assert_eq!(rep.score(a), INITIAL_SCORE + REWARD);
        assert_eq!(rep.score(b), INITIAL_SCORE - PENALTY);
    }

    #[test]
    fn record_batch_applies_all_outcomes_in_order() {
        let mut rep = OracleReputation::new();
        let (a, b) = (acc(1), acc(2));

        let results = rep.record_batch(&[(a, true), (b, false), (a, true)]);

        assert_eq!(results.len(), 3);
        assert_eq!(rep.get(a).submissions, 2);
        assert_eq!(rep.get(a).correct, 2);
        assert_eq!(rep.get(b).submissions, 1);
        assert_eq!(rep.get(b).correct, 0);
    }
}