#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ValidatorTier { A, B, C }

impl ValidatorTier {
    pub fn unbonding_secs(&self) -> u64 {
        match self {
            Self::A => 28 * 24 * 3600,
            Self::B => 14 * 24 * 3600,
            Self::C =>  7 * 24 * 3600,
        }
    }
}

pub fn unlock_at(staked_at: u64, tier: ValidatorTier) -> u64 {
    staked_at.saturating_add(tier.unbonding_secs())
}

pub fn is_unbonded(staked_at: u64, tier: ValidatorTier, now: u64) -> bool {
    now >= unlock_at(staked_at, tier)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tier_a_28_days() {
        assert_eq!(ValidatorTier::A.unbonding_secs(), 28 * 86_400);
        assert!(!is_unbonded(0, ValidatorTier::A, 27 * 86_400));
        assert!(is_unbonded(0, ValidatorTier::A, 28 * 86_400));
    }

    #[test]
    fn tier_b_14_days() {
        assert!(is_unbonded(0, ValidatorTier::B, 14 * 86_400));
        assert!(!is_unbonded(0, ValidatorTier::B, 13 * 86_400));
    }

    #[test]
    fn tier_c_7_days() {
        assert!(is_unbonded(1000, ValidatorTier::C, 1000 + 7 * 86_400));
        assert!(!is_unbonded(1000, ValidatorTier::C, 1000 + 6 * 86_400));
    }

    #[test]
    fn tiers_ordered_descending() {
        assert!(ValidatorTier::A.unbonding_secs() > ValidatorTier::B.unbonding_secs());
        assert!(ValidatorTier::B.unbonding_secs() > ValidatorTier::C.unbonding_secs());
    }
}