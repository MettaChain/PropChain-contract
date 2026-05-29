// Tests for dynamic premium calculation engine

#![cfg(test)]

use super::*;
use ink::env::test;

#[test]
fn test_dynamic_premium_basic_calculation() {
    // Setup risk assessment
    let assessment = RiskAssessment {
        property_id: 1,
        location_risk_score: 70,
        construction_risk_score: 65,
        age_risk_score: 60,
        claims_history_score: 75,
        overall_risk_score: 68,
        risk_level: RiskLevel::Low,
        assessed_at: 1000,
        valid_until: 2000,
    };

    // Setup pool with good capitalization
    let pool = RiskPool {
        pool_id: 1,
        name: "Test Pool".to_string(),
        coverage_type: CoverageType::Comprehensive,
        total_capital: 1_000_000,
        available_capital: 800_000, // 20% utilization
        total_premiums_collected: 100_000,
        total_claims_paid: 50_000,
        active_policies: 10,
        max_coverage_ratio: 500, // 5%
        reinsurance_threshold: 500_000,
        created_at: 1000,
        is_active: true,
    };

    // No actuarial model
    let actuarial_model = None;

    // Basic modifiers
    let modifiers = PremiumModifiers {
        has_multiple_policies: false,
        claim_free_years: 0,
        has_safety_features: false,
        loyalty_years: 0,
    };

    let result = calculate_dynamic_premium(
        &assessment,
        500_000, // $500,000 coverage
        &CoverageType::Comprehensive,
        &pool,
        actuarial_model,
        &modifiers,
        31_536_000, // 1 year
    );

    // Verify calculation components
    assert!(result.annual_premium > 0);
    assert!(result.monthly_premium > 0);
    assert!(result.deductible > 0);
    
    // Multipliers should be set
    assert!(result.base_rate > 0);
    assert!(result.risk_multiplier > 0);
    assert!(result.coverage_multiplier > 0);
    assert!(result.pool_utilization_multiplier > 0);
    assert!(result.time_multiplier > 0);
    assert!(result.discount_multiplier > 0);
}

#[test]
fn test_dynamic_premium_with_discounts() {
    let assessment = RiskAssessment {
        property_id: 1,
        location_risk_score: 80,
        construction_risk_score: 75,
        age_risk_score: 70,
        claims_history_score: 85,
        overall_risk_score: 78,
        risk_level: RiskLevel::VeryLow,
        assessed_at: 1000,
        valid_until: 2000,
    };

    let pool = RiskPool {
        pool_id: 1,
        name: "Test Pool".to_string(),
        coverage_type: CoverageType::Fire,
        total_capital: 1_000_000,
        available_capital: 900_000, // 10% utilization
        total_premiums_collected: 100_000,
        total_claims_paid: 30_000,
        active_policies: 5,
        max_coverage_ratio: 500,
        reinsurance_threshold: 500_000,
        created_at: 1000,
        is_active: true,
    };

    // All discounts applied
    let modifiers = PremiumModifiers {
        has_multiple_policies: true,      // 15% discount
        claim_free_years: 5,              // 20% discount
        has_safety_features: true,        // 10% discount
        loyalty_years: 7,                 // 10% discount
    };

    let result = calculate_dynamic_premium(
        &assessment,
        300_000,
        &CoverageType::Fire,
        &pool,
        None,
        &modifiers,
        31_536_000,
    );

    // With all discounts, premium should be significantly lower
    // Total discount capped at 40%, so multiplier should be 6000 (60%)
    assert_eq!(result.discount_multiplier, 6000);
    
    // Premium should be reasonable
    assert!(result.annual_premium > 0);
    assert!(result.annual_premium < 30_000); // Should be less than 10% of coverage
}

#[test]
fn test_dynamic_premium_high_risk_property() {
    let assessment = RiskAssessment {
        property_id: 1,
        location_risk_score: 20,  // High risk location
        construction_risk_score: 25, // Poor construction
        age_risk_score: 15,       // Very old
        claims_history_score: 10, // Many previous claims
        overall_risk_score: 18,
        risk_level: RiskLevel::VeryHigh,
        assessed_at: 1000,
        valid_until: 2000,
    };

    let pool = RiskPool {
        pool_id: 1,
        name: "Test Pool".to_string(),
        coverage_type: CoverageType::NaturalDisaster,
        total_capital: 1_000_000,
        available_capital: 200_000, // 80% utilization - high!
        total_premiums_collected: 200_000,
        total_claims_paid: 150_000,
        active_policies: 50,
        max_coverage_ratio: 500,
        reinsurance_threshold: 500_000,
        created_at: 1000,
        is_active: true,
    };

    let modifiers = PremiumModifiers {
        has_multiple_policies: false,
        claim_free_years: 0,
        has_safety_features: false,
        loyalty_years: 0,
    };

    let result = calculate_dynamic_premium(
        &assessment,
        400_000,
        &CoverageType::NaturalDisaster,
        &pool,
        None,
        &modifiers,
        31_536_000,
    );

    // High risk should result in high premiums
    assert!(result.risk_multiplier >= 350); // Very high risk multiplier
    assert!(result.pool_utilization_multiplier >= 135); // High utilization
    assert!(result.annual_premium > 20_000); // Should be substantial
}

#[test]
fn test_pool_utilization_impact() {
    let assessment = RiskAssessment {
        property_id: 1,
        location_risk_score: 60,
        construction_risk_score: 60,
        age_risk_score: 60,
        claims_history_score: 60,
        overall_risk_score: 60,
        risk_level: RiskLevel::Medium,
        assessed_at: 1000,
        valid_until: 2000,
    };

    // Low utilization pool
    let low_util_pool = RiskPool {
        pool_id: 1,
        name: "Low Util Pool".to_string(),
        coverage_type: CoverageType::Fire,
        total_capital: 1_000_000,
        available_capital: 800_000, // 20% utilization
        total_premiums_collected: 50_000,
        total_claims_paid: 10_000,
        active_policies: 5,
        max_coverage_ratio: 500,
        reinsurance_threshold: 500_000,
        created_at: 1000,
        is_active: true,
    };

    // High utilization pool
    let high_util_pool = RiskPool {
        pool_id: 2,
        name: "High Util Pool".to_string(),
        coverage_type: CoverageType::Fire,
        total_capital: 1_000_000,
        available_capital: 150_000, // 85% utilization
        total_premiums_collected: 200_000,
        total_claims_paid: 180_000,
        active_policies: 80,
        max_coverage_ratio: 500,
        reinsurance_threshold: 500_000,
        created_at: 1000,
        is_active: true,
    };

    let modifiers = PremiumModifiers {
        has_multiple_policies: false,
        claim_free_years: 0,
        has_safety_features: false,
        loyalty_years: 0,
    };

    let low_util_result = calculate_dynamic_premium(
        &assessment,
        500_000,
        &CoverageType::Fire,
        &low_util_pool,
        None,
        &modifiers,
        31_536_000,
    );

    let high_util_result = calculate_dynamic_premium(
        &assessment,
        500_000,
        &CoverageType::Fire,
        &high_util_pool,
        None,
        &modifiers,
        31_536_000,
    );

    // High utilization pool should have higher premiums
    assert!(high_util_result.pool_utilization_multiplier > low_util_result.pool_utilization_multiplier);
    assert!(high_util_result.annual_premium > low_util_result.annual_premium);
}

#[test]
fn test_duration_impact_on_premium() {
    let assessment = RiskAssessment {
        property_id: 1,
        location_risk_score: 70,
        construction_risk_score: 70,
        age_risk_score: 70,
        claims_history_score: 70,
        overall_risk_score: 70,
        risk_level: RiskLevel::Low,
        assessed_at: 1000,
        valid_until: 2000,
    };

    let pool = RiskPool {
        pool_id: 1,
        name: "Test Pool".to_string(),
        coverage_type: CoverageType::Comprehensive,
        total_capital: 1_000_000,
        available_capital: 700_000,
        total_premiums_collected: 100_000,
        total_claims_paid: 40_000,
        active_policies: 20,
        max_coverage_ratio: 500,
        reinsurance_threshold: 500_000,
        created_at: 1000,
        is_active: true,
    };

    let modifiers = PremiumModifiers {
        has_multiple_policies: false,
        claim_free_years: 0,
        has_safety_features: false,
        loyalty_years: 0,
    };

    // Short term: 1 month
    let short_term = calculate_dynamic_premium(
        &assessment,
        500_000,
        &CoverageType::Comprehensive,
        &pool,
        None,
        &modifiers,
        2_592_000, // 30 days
    );

    // Long term: 2 years
    let long_term = calculate_dynamic_premium(
        &assessment,
        500_000,
        &CoverageType::Comprehensive,
        &pool,
        None,
        &modifiers,
        63_072_000, // 2 years
    );

    // Long term should have better time multiplier
    assert!(long_term.time_multiplier < short_term.time_multiplier);
    
    // Total premium for 2 years should be less than 2x the 1 month premium
    let short_term_annualized = short_term.annual_premium * 24;
    assert!(long_term.annual_premium < short_term_annualized);
}

#[test]
fn test_actuarial_model_impact() {
    let assessment = RiskAssessment {
        property_id: 1,
        location_risk_score: 65,
        construction_risk_score: 65,
        age_risk_score: 65,
        claims_history_score: 65,
        overall_risk_score: 65,
        risk_level: RiskLevel::Medium,
        assessed_at: 1000,
        valid_until: 2000,
    };

    let pool = RiskPool {
        pool_id: 1,
        name: "Test Pool".to_string(),
        coverage_type: CoverageType::Flood,
        total_capital: 1_000_000,
        available_capital: 600_000,
        total_premiums_collected: 100_000,
        total_claims_paid: 50_000,
        active_policies: 15,
        max_coverage_ratio: 500,
        reinsurance_threshold: 500_000,
        created_at: 1000,
        is_active: true,
    };

    let modifiers = PremiumModifiers {
        has_multiple_policies: false,
        claim_free_years: 0,
        has_safety_features: false,
        loyalty_years: 0,
    };

    // With actuarial model
    let actuarial_model = ActuarialModel {
        model_id: 1,
        coverage_type: CoverageType::Flood,
        loss_frequency: 500, // 5% frequency
        average_loss_severity: 100_000,
        expected_loss_ratio: 600, // 6%
        confidence_level: 95,
        last_updated: 1000,
        data_points: 1000,
    };

    let with_model = calculate_dynamic_premium(
        &assessment,
        500_000,
        &CoverageType::Flood,
        &pool,
        Some(&actuarial_model),
        &modifiers,
        31_536_000,
    );

    // Without actuarial model (uses defaults)
    let without_model = calculate_dynamic_premium(
        &assessment,
        500_000,
        &CoverageType::Flood,
        &pool,
        None,
        &modifiers,
        31_536_000,
    );

    // Both should produce valid results
    assert!(with_model.annual_premium > 0);
    assert!(without_model.annual_premium > 0);
    
    // They may differ based on actuarial vs default rates
    // Actuarial model uses 6% * 1.0 (95% confidence) * 1.2 (expense loading) = 7.2%
    // Default flood rate is 2.0%
    // So actuarial should be higher in this case
    assert!(with_model.base_rate > without_model.base_rate);
}

#[test]
fn test_premium_breakdown_accuracy() {
    let assessment = RiskAssessment {
        property_id: 1,
        location_risk_score: 70,
        construction_risk_score: 70,
        age_risk_score: 70,
        claims_history_score: 70,
        overall_risk_score: 70,
        risk_level: RiskLevel::Low,
        assessed_at: 1000,
        valid_until: 2000,
    };

    let pool = RiskPool {
        pool_id: 1,
        name: "Test Pool".to_string(),
        coverage_type: CoverageType::Fire,
        total_capital: 1_000_000,
        available_capital: 700_000,
        total_premiums_collected: 100_000,
        total_claims_paid: 40_000,
        active_policies: 10,
        max_coverage_ratio: 500,
        reinsurance_threshold: 500_000,
        created_at: 1000,
        is_active: true,
    };

    let modifiers = PremiumModifiers {
        has_multiple_policies: false,
        claim_free_years: 0,
        has_safety_features: false,
        loyalty_years: 0,
    };

    let result = calculate_dynamic_premium(
        &assessment,
        500_000,
        &CoverageType::Fire,
        &pool,
        None,
        &modifiers,
        31_536_000,
    );

    // Verify breakdown adds up correctly
    let total_from_breakdown = result.breakdown.base_premium
        .saturating_add(result.breakdown.risk_adjustment)
        .saturating_add(result.breakdown.coverage_adjustment)
        .saturating_add(result.breakdown.pool_adjustment)
        .saturating_add(result.breakdown.time_adjustment)
        .saturating_sub(result.breakdown.discount_amount);

    // Should be approximately equal (allow for rounding)
    let diff = if total_from_breakdown > result.annual_premium {
        total_from_breakdown - result.annual_premium
    } else {
        result.annual_premium - total_from_breakdown
    };
    
    // Allow small rounding difference
    assert!(diff < 100);
}
