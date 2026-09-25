use propchain_traits::AccountId;

/// Outcome of a reputation slash, reporting the *actual* effect rather than
/// the requested severity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, scale::Encode, scale::Decode)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub struct SlashOutcome {
    /// Reputation after the slash (0–1000 scale).
    pub new_reputation: u32,
    /// Amount of stake actually deducted (capped at the available stake).
    pub actual_amount_slashed: u128,
    /// Whether the slashed oracle dropped below `min_reputation_score` and
    /// should therefore be frozen (deactivated) by the caller.
    pub frozen: bool,
}

/// Pure reputation/slashing decision engine used by the oracle contract.
///
/// Issue #1097: the previous implementation subtracted a hardcoded 20 from an
/// unrelated value and returned the requested slash amount unconditionally,
/// which made it dead, misleading code. This corrected version computes the
/// real reductions and signals when a source must be frozen.
pub struct OracleSlashingManager {
    /// Reputation score below which a source is considered frozen.
    pub min_reputation_score: u32,
}

impl Default for OracleSlashingManager {
    fn default() -> Self {
        Self::new(propchain_traits::constants::ORACLE_MIN_REPUTATION_THRESHOLD)
    }
}

impl OracleSlashingManager {
    pub fn new(min_reputation_score: u32) -> Self {
        Self {
            min_reputation_score,
        }
    }

    /// Slashes a malicious oracle.
    ///
    /// `requested_slash` is the nominal stake penalty expressed as an absolute
    /// token amount; the actual deduction is capped at `current_stake`.
    /// `reputation_penalty` is subtracted from `current_reputation`, and the
    /// returned `frozen` flag flips once the resulting reputation falls below
    /// `min_reputation_score` so callers can deactivate the source.
    pub fn slash_malicious_oracle(
        &self,
        _oracle: AccountId,
        current_reputation: u32,
        current_stake: u128,
        requested_slash: u128,
        reputation_penalty: u32,
    ) -> SlashOutcome {
        let actual_amount_slashed = core::cmp::min(requested_slash, current_stake);
        let new_reputation = current_reputation.saturating_sub(reputation_penalty);
        SlashOutcome {
            new_reputation,
            actual_amount_slashed,
            frozen: new_reputation < self.min_reputation_score,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn addr() -> AccountId {
        AccountId::from([0x01u8; 32])
    }

    #[test]
    fn slash_reduces_reputation_and_stake_truthfully() {
        let manager = OracleSlashingManager::default();
        let outcome = manager.slash_malicious_oracle(addr(), 500, 1_000_000, 150_000, 150);
        assert_eq!(outcome.new_reputation, 350);
        assert_eq!(outcome.actual_amount_slashed, 150_000);
        assert!(!outcome.frozen);
    }

    #[test]
    fn slash_never_deducts_more_than_available_stake() {
        let manager = OracleSlashingManager::default();
        // Requested slash exceeds the available stake: cut is clamped.
        let outcome = manager.slash_malicious_oracle(addr(), 500, 10_000, 150_000, 150);
        assert_eq!(outcome.actual_amount_slashed, 10_000);
        assert_eq!(outcome.new_reputation, 350);
        assert!(!outcome.frozen);
    }

    #[test]
    fn slash_freezes_source_below_minimum_reputation() {
        let manager = OracleSlashingManager::new(200);
        // Dropping from 210 below 200 freezes the source.
        let outcome = manager.slash_malicious_oracle(addr(), 210, 1_000_000, 50_000, 50);
        assert_eq!(outcome.new_reputation, 160);
        assert!(outcome.frozen);
        // Reputation floors at zero, never underflows.
        let outcome = manager.slash_malicious_oracle(addr(), 10, 1_000_000, 50_000, 50);
        assert_eq!(outcome.new_reputation, 0);
        assert!(outcome.frozen);
    }

    #[test]
    fn repeated_outliers_drive_reputation_down_until_frozen() {
        let manager = OracleSlashingManager::new(200);
        let mut rep = 500u32;
        let mut frozen = false;
        let mut slashes = 0;
        while !frozen && slashes < 100 {
            let outcome = manager.slash_malicious_oracle(addr(), rep, 1_000_000, 150_000, 150);
            rep = outcome.new_reputation;
            frozen = outcome.frozen;
            slashes += 1;
        }
        assert!(frozen, "repeated slashes must eventually freeze the source");
        assert!(rep < 200);
        assert_eq!(slashes, 3); // 500 -> 350 -> 200 -> 50: frozen on the 3rd
    }
}
