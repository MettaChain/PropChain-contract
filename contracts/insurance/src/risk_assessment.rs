// Risk Assessment Model Implementation (Task #254)
// Provides comprehensive risk pricing model for accurate insurance premium calculation

use ink::prelude::{string::String, vec::Vec};

/// Risk assessment model functions for property insurance
pub mod risk_model {
    use super::*;

    const MODEL_VERSION: u32 = 1;
    const ASSESSMENT_VALIDITY_DAYS: u64 = 365; // 365 days validity
    const SECONDS_PER_DAY: u64 = 86_400;

    /// Calculate location risk score based on location code
    /// Higher score = higher risk
    pub fn calculate_location_risk_score(location_code: &str) -> u32 {
        match location_code {
            "high_risk_zone" => 800,
            "flood_prone" => 750,
            "earthquake_zone" => 700,
            "urban_high_crime" => 650,
            "suburban" => 350,
            "rural_low_risk" => 200,
            "premium_safe_zone" => 100,
            _ => 500, // Default to medium risk
        }
    }

    /// Calculate construction risk score based on construction type
    /// Higher score = higher risk
    pub fn calculate_construction_risk_score(construction_type: &str) -> u32 {
        match construction_type {
            "wood_frame" => 750,
            "masonry_veneer" => 600,
            "reinforced_concrete" => 300,
            "steel_frame" => 250,
            "composite_materials" => 400,
            "stone_brick" => 350,
            _ => 500,
        }
    }

    /// Calculate age risk score based on property age
    /// Newer properties are safer
    pub fn calculate_age_risk_score(property_age_years: u32) -> u32 {
        match property_age_years {
            0..=5 => 150,    // Very new - low risk
            6..=15 => 300,   // Modern
            16..=30 => 500,  // Medium age
            31..=50 => 700,  // Older
            51..=100 => 850, // Much older
            _ => 900,        // Very old
        }
    }

    /// Calculate ownership risk score based on owner experience
    /// More experienced owners = lower risk
    pub fn calculate_ownership_risk_score(owner_age_years: u32, years_as_owner: u32) -> u32 {
        // Ownership stability score
        let stability_score = if years_as_owner > 10 {
            100
        } else if years_as_owner > 5 {
            200
        } else if years_as_owner > 2 {
            350
        } else {
            600
        };

        // Owner age factor (too young or too old increases risk)
        let age_factor = if owner_age_years < 25 {
            300
        } else if owner_age_years < 35 {
            200
        } else if owner_age_years < 60 {
            100
        } else if owner_age_years < 75 {
            250
        } else {
            400
        };

        // Combined score (weighted average)
        (stability_score * 60 + age_factor * 40) / 100
    }

    /// Calculate claims history risk score
    /// More claims = higher risk
    pub fn calculate_claims_history_score(
        historical_claims_count: u32,
        historical_claims_amount: u128,
    ) -> u32 {
        let claims_count_score = match historical_claims_count {
            0 => 100,
            1 => 250,
            2 => 400,
            3 => 550,
            4 => 700,
            5..=10 => 850,
            _ => 950,
        };

        // High claim amounts indicate serious incidents
        let claims_amount_factor = if historical_claims_amount > 100_000_000_000 {
            800
        } else if historical_claims_amount > 50_000_000_000 {
            650
        } else if historical_claims_amount > 10_000_000_000 {
            450
        } else {
            200
        };

        // Combined score
        (claims_count_score * 70 + claims_amount_factor * 30) / 100
    }

    /// Calculate safety features score (inverse: lower is better)
    /// More safety features = lower risk
    pub fn calculate_safety_features_score(
        has_security_system: bool,
        has_fire_extinguisher: bool,
        has_alarm_system: bool,
    ) -> u32 {
        let mut safety_score = 0u32;

        // Start with base risk
        safety_score = 600;

        // Each safety feature reduces risk
        if has_security_system {
            safety_score = safety_score.saturating_sub(150);
        }
        if has_fire_extinguisher {
            safety_score = safety_score.saturating_sub(100);
        }
        if has_alarm_system {
            safety_score = safety_score.saturating_sub(150);
        }

        safety_score.min(900).max(100)
    }

    /// Calculate overall risk score from component scores
    /// Uses weighted average of all risk factors
    pub fn calculate_overall_risk_score(
        location_score: u32,
        construction_score: u32,
        age_score: u32,
        ownership_score: u32,
        claims_score: u32,
        safety_score: u32,
    ) -> u32 {
        // Weighted average: location 20%, construction 20%, age 15%,
        // ownership 15%, claims 20%, safety 10%
        (location_score * 200
            + construction_score * 200
            + age_score * 150
            + ownership_score * 150
            + claims_score * 200
            + safety_score * 100)
            / 1000
    }

    /// Map risk score (0-1000) to risk level
    pub fn score_to_risk_level(score: u32) -> crate::propchain_insurance::RiskLevel {
        match score {
            0..=200 => crate::propchain_insurance::RiskLevel::VeryLow,
            201..=400 => crate::propchain_insurance::RiskLevel::Low,
            401..=600 => crate::propchain_insurance::RiskLevel::Medium,
            601..=800 => crate::propchain_insurance::RiskLevel::High,
            _ => crate::propchain_insurance::RiskLevel::VeryHigh,
        }
    }

    /// Calculate premium multiplier based on overall risk score
    /// Returns multiplier as basis points (10000 = 1.0x)
    pub fn calculate_premium_multiplier(overall_score: u32) -> u32 {
        match overall_score {
            0..=200 => 5_000,    // 0.5x multiplier - very low risk
            201..=400 => 7_500,  // 0.75x multiplier - low risk
            401..=600 => 10_000, // 1.0x multiplier - normal
            601..=800 => 15_000, // 1.5x multiplier - high risk
            _ => 25_000,         // 2.5x multiplier - very high risk
        }
    }

    /// Get model version
    pub fn get_model_version() -> u32 {
        MODEL_VERSION
    }

    /// Get assessment validity duration in seconds
    pub fn get_assessment_validity_seconds() -> u64 {
        ASSESSMENT_VALIDITY_DAYS * SECONDS_PER_DAY
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_location_risk_score() {
        assert_eq!(
            risk_model::calculate_location_risk_score("premium_safe_zone"),
            100
        );
        assert_eq!(
            risk_model::calculate_location_risk_score("high_risk_zone"),
            800
        );
        assert_eq!(risk_model::calculate_location_risk_score("unknown"), 500);
    }

    #[test]
    fn test_construction_risk_score() {
        assert_eq!(
            risk_model::calculate_construction_risk_score("steel_frame"),
            250
        );
        assert_eq!(
            risk_model::calculate_construction_risk_score("wood_frame"),
            750
        );
    }

    #[test]
    fn test_age_risk_score() {
        assert_eq!(risk_model::calculate_age_risk_score(3), 150); // Very new
        assert_eq!(risk_model::calculate_age_risk_score(25), 500); // Medium
        assert_eq!(risk_model::calculate_age_risk_score(80), 850); // Much older
    }

    #[test]
    fn test_overall_risk_score_calculation() {
        let score = risk_model::calculate_overall_risk_score(300, 400, 300, 300, 200, 300);
        assert!(score >= 200 && score <= 400);
    }

    #[test]
    fn test_risk_level_mapping() {
        assert_eq!(
            risk_model::score_to_risk_level(100),
            crate::propchain_insurance::RiskLevel::VeryLow
        );
        assert_eq!(
            risk_model::score_to_risk_level(500),
            crate::propchain_insurance::RiskLevel::Medium
        );
        assert_eq!(
            risk_model::score_to_risk_level(900),
            crate::propchain_insurance::RiskLevel::VeryHigh
        );
    }

    #[test]
    fn test_premium_multiplier() {
        assert_eq!(risk_model::calculate_premium_multiplier(150), 5_000); // 0.5x
        assert_eq!(risk_model::calculate_premium_multiplier(500), 10_000); // 1.0x
        assert_eq!(risk_model::calculate_premium_multiplier(850), 25_000); // 2.5x
    }

    #[test]
    fn test_safety_features_reduction() {
        // No safety features
        let no_safety = risk_model::calculate_safety_features_score(false, false, false);
        // With all safety features
        let all_safety = risk_model::calculate_safety_features_score(true, true, true);
        assert!(all_safety < no_safety);
    }
}
