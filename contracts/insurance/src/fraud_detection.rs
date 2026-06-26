// Fraud Detection Implementation (Task #258)
// Detects and prevents insurance fraud patterns using advanced analytics

use ink::prelude::{string::String, vec::Vec};

/// Fraud detection and prevention functions
pub mod fraud_detection {
    use super::*;

    // Fraud detection constants
    const HIGH_FRAUD_RISK_THRESHOLD: u32 = 700; // Score threshold for high risk
    const MEDIUM_FRAUD_RISK_THRESHOLD: u32 = 450; // Score threshold for medium risk
    const CLAIMS_SHORT_PERIOD_DAYS: u64 = 30; // Days window for multiple claims
    const CLAIMS_SHORT_PERIOD_SECONDS: u64 = CLAIMS_SHORT_PERIOD_DAYS * 86_400;
    const SUSPICIOUS_TIME_WEEKEND_THRESHOLD: u32 = 200; // Extra points for weekend claims
    const ANOMALOUS_CLAIM_MULTIPLIER: u128 = 150; // 150% of average
    const HIGH_RISK_FRAUD_SCORE: u32 = 200;

    /// Detect multiple claims in a short time period
    pub fn detect_multiple_claims_short_period(
        claims_count: u32,
        time_since_last_claim: Option<u64>,
    ) -> (bool, u32) {
        match time_since_last_claim {
            Some(time) if time < CLAIMS_SHORT_PERIOD_SECONDS => {
                if claims_count > 2 {
                    (true, 300)
                } else if claims_count > 1 {
                    (true, 150)
                } else {
                    (false, 0)
                }
            }
            _ => (false, 0),
        }
    }

    /// Detect anomalous claim amounts
    pub fn detect_anomalous_claim_amount(
        claim_amount: u128,
        average_claim_amount: u128,
        max_coverage: u128,
    ) -> (bool, u32) {
        if average_claim_amount == 0 {
            return (false, 0);
        }

        let multiplier = (claim_amount * 100) / average_claim_amount;

        // If claim is 150%+ of average, it's anomalous
        if multiplier > ANOMALOUS_CLAIM_MULTIPLIER {
            // Higher score if it's close to max coverage (claim stuffing)
            if claim_amount > (max_coverage * 90 / 100) {
                (true, 300)
            } else {
                (true, 200)
            }
        } else {
            (false, 0)
        }
    }

    /// Detect suspicious timing patterns (claims on weekends/holidays)
    pub fn detect_suspicious_timing(timestamp: u64) -> (bool, u32) {
        // Convert timestamp to day of week (0 = Sunday, 6 = Saturday)
        let days_since_epoch = timestamp / 86_400;
        let day_of_week = (days_since_epoch + 4) % 7; // Adjust for Unix epoch starting Thursday

        // Suspicious if submitted on weekend (Saturday=6, Sunday=0)
        if day_of_week == 0 || day_of_week == 6 {
            (true, SUSPICIOUS_TIME_WEEKEND_THRESHOLD)
        } else {
            (false, 0)
        }
    }

    /// Detect excessive coverage ratio
    pub fn detect_excessive_coverage_ratio(claim_amount: u128, max_coverage: u128) -> (bool, u32) {
        let coverage_ratio = (claim_amount * 100) / max_coverage;

        // If claim is > 85% of coverage, flag it
        if coverage_ratio > 85 {
            (true, 250)
        } else if coverage_ratio > 75 {
            (true, 100)
        } else {
            (false, 0)
        }
    }

    /// Detect patterns consistent with known fraud
    pub fn detect_historical_fraud_pattern(
        policyholder_claims_count: u32,
        policyholder_rejection_rate: u32, // percentage 0-100
    ) -> (bool, u32) {
        let mut score = 0u32;

        // High number of claims increases risk
        if policyholder_claims_count > 10 {
            score += 300;
        } else if policyholder_claims_count > 5 {
            score += 150;
        } else if policyholder_claims_count > 3 {
            score += 50;
        }

        // High rejection rate is suspicious
        if policyholder_rejection_rate > 50 {
            score += 250;
        } else if policyholder_rejection_rate > 25 {
            score += 100;
        }

        (score > 0, score.min(400))
    }

    /// Detect misrepresentation in claim details
    pub fn detect_misrepresentation(
        description_length: u32,
        evidence_url_valid: bool,
    ) -> (bool, u32) {
        let mut score = 0u32;

        // Very short descriptions are suspicious
        if description_length < 50 {
            score += 150;
        } else if description_length < 100 {
            score += 50;
        }

        // Missing evidence is suspicious
        if !evidence_url_valid {
            score += 200;
        }

        (score > 0, score.min(300))
    }

    /// Detect claims from known fraud networks
    pub fn detect_known_fraud_network(
        is_flagged_account: bool,
        associated_fraud_accounts: u32,
    ) -> (bool, u32) {
        if is_flagged_account {
            return (true, 400);
        }

        if associated_fraud_accounts > 2 {
            (true, 300)
        } else if associated_fraud_accounts > 0 {
            (true, 150)
        } else {
            (false, 0)
        }
    }

    /// Detect duplicate claim patterns (similar to previous fraud)
    pub fn detect_duplicate_claim_patterns(
        similar_claims_count: u32,
        similar_claims_rejection_count: u32,
    ) -> (bool, u32) {
        let rejection_rate = if similar_claims_count > 0 {
            (similar_claims_rejection_count * 100) / similar_claims_count
        } else {
            0
        };

        let mut score = 0u32;

        if similar_claims_count > 5 {
            score += 300;
        } else if similar_claims_count > 2 {
            score += 150;
        }

        if rejection_rate > 70 {
            score += 200;
        } else if rejection_rate > 40 {
            score += 100;
        }

        (score > 0, score.min(400))
    }

    /// Calculate total fraud risk score from individual indicators
    pub fn calculate_fraud_risk_score(indicator_scores: &[u32]) -> u32 {
        if indicator_scores.is_empty() {
            return 0;
        }

        // Fraud scoring is cumulative but capped at 1000
        let total: u32 = indicator_scores.iter().sum();
        total.min(1000)
    }

    /// Determine fraud risk level from score
    pub fn score_to_fraud_risk_level(score: u32) -> crate::propchain_insurance::RiskLevel {
        match score {
            0..=250 => crate::propchain_insurance::RiskLevel::VeryLow, // Very low fraud risk
            251..=450 => crate::propchain_insurance::RiskLevel::Low,   // Low fraud risk
            451..=600 => crate::propchain_insurance::RiskLevel::Medium, // Medium fraud risk
            601..=800 => crate::propchain_insurance::RiskLevel::High,  // High fraud risk
            _ => crate::propchain_insurance::RiskLevel::VeryHigh,      // Very high fraud risk
        }
    }

    /// Determine if claim requires manual review
    pub fn requires_manual_review(fraud_score: u32, indicator_count: u32) -> bool {
        // Require review if score is high OR multiple indicators detected
        fraud_score > MEDIUM_FRAUD_RISK_THRESHOLD || indicator_count > 3
    }

    /// Get fraud risk threshold for automatic rejection
    pub fn get_high_fraud_risk_threshold() -> u32 {
        HIGH_FRAUD_RISK_THRESHOLD
    }

    /// Get medium fraud risk threshold for extra scrutiny
    pub fn get_medium_fraud_risk_threshold() -> u32 {
        MEDIUM_FRAUD_RISK_THRESHOLD
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_multiple_claims_detection() {
        let (detected, score) =
            fraud_detection::detect_multiple_claims_short_period(3, Some(10_000));
        assert!(detected);
        assert_eq!(score, 300);

        let (detected, score) =
            fraud_detection::detect_multiple_claims_short_period(1, Some(10_000));
        assert!(!detected);
    }

    #[test]
    fn test_anomalous_claim_amount() {
        let (detected, score) =
            fraud_detection::detect_anomalous_claim_amount(1_500, 1_000, 10_000);
        assert!(detected);
        assert!(score > 0);

        let (detected, score) =
            fraud_detection::detect_anomalous_claim_amount(1_100, 1_000, 10_000);
        assert!(!detected);
    }

    #[test]
    fn test_excessive_coverage_ratio() {
        let (detected, score) = fraud_detection::detect_excessive_coverage_ratio(9_000, 10_000);
        assert!(detected);

        let (detected, score) = fraud_detection::detect_excessive_coverage_ratio(7_000, 10_000);
        assert!(detected);

        let (detected, score) = fraud_detection::detect_excessive_coverage_ratio(6_000, 10_000);
        assert!(!detected);
    }

    #[test]
    fn test_historical_fraud_pattern() {
        let (detected, score) = fraud_detection::detect_historical_fraud_pattern(12, 60);
        assert!(detected);
        assert!(score > 0);
    }

    #[test]
    fn test_misrepresentation_detection() {
        let (detected, score) = fraud_detection::detect_misrepresentation(30, false);
        assert!(detected);

        let (detected, score) = fraud_detection::detect_misrepresentation(200, true);
        assert!(!detected);
    }

    #[test]
    fn test_fraud_risk_scoring() {
        let scores = vec![100, 200, 150];
        let total = fraud_detection::calculate_fraud_risk_score(&scores);
        assert_eq!(total, 450);
    }

    #[test]
    fn test_fraud_risk_level_mapping() {
        assert_eq!(
            fraud_detection::score_to_fraud_risk_level(100),
            crate::propchain_insurance::RiskLevel::VeryLow
        );
        assert_eq!(
            fraud_detection::score_to_fraud_risk_level(700),
            crate::propchain_insurance::RiskLevel::VeryHigh
        );
    }

    #[test]
    fn test_manual_review_requirement() {
        assert!(fraud_detection::requires_manual_review(500, 2));
        assert!(!fraud_detection::requires_manual_review(200, 1));
    }
}
