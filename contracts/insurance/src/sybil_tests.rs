#[cfg(test)]
mod sybil_resistance {
    const MAX_CLAIMS_PER_WINDOW: u32 = 3;

    fn claim_count(claims: &[u8], submitter: u8) -> u32 {
        claims.iter().filter(|&&s| s == submitter).count() as u32
    }

    fn is_flagged(claims: &[u8], submitter: u8) -> bool {
        claim_count(claims, submitter) > MAX_CLAIMS_PER_WINDOW
    }

    #[test]
    fn below_threshold_is_safe() {
        let claims = [1u8, 1, 1];
        assert!(!is_flagged(&claims, 1));
    }

    #[test]
    fn flooding_submitter_is_flagged() {
        let claims = [1u8, 1, 1, 1];
        assert!(is_flagged(&claims, 1));
    }

    #[test]
    fn diverse_submitters_pass() {
        let claims = [1u8, 2, 3];
        assert!(!is_flagged(&claims, 1));
        assert!(!is_flagged(&claims, 2));
        assert!(!is_flagged(&claims, 3));
    }

    #[test]
    fn coordinated_attack_detected_by_volume() {
        let claims = [1u8, 2, 3, 1, 2, 3, 1, 2, 3, 1, 2, 3];
        let submitters = [1u8, 2, 3];
        for s in submitters { assert!(is_flagged(&claims, s)); }
    }
}