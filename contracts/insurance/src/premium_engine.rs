// Dynamic premium calculation engine based on risk assessment
// Implements actuarial pricing with real-time adjustments

use crate::{
    ActuarialModel, CoverageType, PremiumCalculation, PremiumModifiers, RiskAssessment, RiskPool,
};

/// Dynamic premium calculation with comprehensive risk factors
pub fn calculate_dynamic_premium(
    risk_assessment: &RiskAssessment,
    coverage_amount: u128,
    coverage_type: &CoverageType,
    pool: &RiskPool,
    actuarial_model: Option<&ActuarialModel>,
    modifiers: &PremiumModifiers,
    policy_duration_seconds: u64,
) -> PremiumCalculation {
    // Step 1: Calculate base rate from actuarial model or use default
    let base_rate = calculate_base_rate(actuarial_model, coverage_type);

    // Step 2: Calculate comprehensive risk multiplier
    let risk_multiplier = calculate_risk_multiplier(risk_assessment);

    // Step 3: Calculate coverage type multiplier
    let coverage_multiplier = coverage_type_multiplier(coverage_type);

    // Step 4: Calculate pool utilization adjustment
    let pool_utilization_multiplier = calculate_pool_utilization_multiplier(pool);

    // Step 5: Calculate time-based adjustments
    let time_multiplier = calculate_time_multiplier(policy_duration_seconds);

    // Step 6: Calculate discounts
    let discount_multiplier = calculate_discount_multiplier(modifiers);

    // Step 7: Calculate final premium
    // Formula: coverage * base_rate * risk_mult * coverage_mult * pool_mult * time_mult * discount_mult
    let annual_premium = coverage_amount
        .saturating_mul(base_rate as u128)
        .saturating_mul(risk_multiplier as u128)
        .saturating_mul(coverage_multiplier as u128)
        .saturating_mul(pool_utilization_multiplier as u128)
        .saturating_mul(time_multiplier as u128)
        .saturating_mul(discount_multiplier as u128)
        / PREMIUM_CALCULATION_DIVISOR;

    // Prorate for policy duration
    let duration_premium = annual_premium
        .saturating_mul(policy_duration_seconds as u128)
        / SECONDS_PER_YEAR;

    let monthly_premium = duration_premium / 12;

    // Calculate dynamic deductible based on risk
    let deductible = calculate_deductible(coverage_amount, risk_assessment, modifiers);

    PremiumCalculation {
        base_rate,
        risk_multiplier,
        coverage_multiplier,
        pool_utilization_multiplier,
        time_multiplier,
        discount_multiplier,
        annual_premium: duration_premium,
        monthly_premium,
        deductible,
        breakdown: PremiumBreakdown {
            base_premium: coverage_amount.saturating_mul(base_rate as u128) / BASIS_POINTS_DENOMINATOR,
            risk_adjustment: calculate_risk_adjustment_amount(
                coverage_amount,
                base_rate,
                risk_multiplier,
            ),
            coverage_adjustment: calculate_coverage_adjustment_amount(
                coverage_amount,
                base_rate,
                risk_multiplier,
                coverage_multiplier,
            ),
            pool_adjustment: calculate_pool_adjustment_amount(
                coverage_amount,
                base_rate,
                risk_multiplier,
                coverage_multiplier,
                pool_utilization_multiplier,
            ),
            time_adjustment: calculate_time_adjustment_amount(
                coverage_amount,
                base_rate,
                risk_multiplier,
                coverage_multiplier,
                pool_utilization_multiplier,
                time_multiplier,
            ),
            discount_amount: calculate_discount_amount(
                coverage_amount,
                base_rate,
                risk_multiplier,
                coverage_multiplier,
                pool_utilization_multiplier,
                time_multiplier,
                discount_multiplier,
            ),
        },
    }
}

/// Calculate base rate from actuarial model
fn calculate_base_rate(actuarial_model: Option<&ActuarialModel>, coverage_type: &CoverageType) -> u32 {
    match actuarial_model {
        Some(model) => {
            // Use actuarial model: expected_loss_ratio * confidence_adjustment
            // Expected loss ratio in basis points (e.g., 600 = 6%)
            let expected_loss = model.expected_loss_ratio;
            
            // Confidence level adjustment (95% = 1.0, 99% = 1.2)
            let confidence_adjustment = match model.confidence_level {
                95 => 100,
                96 => 105,
                97 => 110,
                98 => 115,
                99 => 120,
                _ => 100,
            };

            // Base rate = expected_loss * confidence_adjustment / 100
            let model_rate = expected_loss.saturating_mul(confidence_adjustment as u32) / 100;
            
            // Add expense loading (20% for operational costs)
            model_rate.saturating_mul(120) / 100
        }
        None => {
            // Default rates by coverage type (in basis points)
            coverage_type_base_rate(coverage_type)
        }
    }
}

/// Default base rates by coverage type
fn coverage_type_base_rate(coverage_type: &CoverageType) -> u32 {
    match coverage_type {
        CoverageType::Fire => 120,          // 1.2%
        CoverageType::Flood => 200,         // 2.0%
        CoverageType::Earthquake => 250,    // 2.5%
        CoverageType::Theft => 100,         // 1.0%
        CoverageType::LiabilityDamage => 150, // 1.5%
        CoverageType::NaturalDisaster => 220, // 2.2%
        CoverageType::Comprehensive => 300,  // 3.0%
    }
}

/// Calculate comprehensive risk multiplier from assessment scores
fn calculate_risk_multiplier(assessment: &RiskAssessment) -> u32 {
    // Weighted average of risk components
    // Location risk: 30%, Construction risk: 25%, Age risk: 20%, Claims history: 25%
    let weighted_score = assessment
        .location_risk_score
        .saturating_mul(30)
        .saturating_add(assessment.construction_risk_score.saturating_mul(25))
        .saturating_add(assessment.age_risk_score.saturating_mul(20))
        .saturating_add(assessment.claims_history_score.saturating_mul(25))
        / 100;

    // Convert score (0-100) to multiplier (50-400 basis points)
    // Score 0 = very high risk (4.0x), Score 100 = very low risk (0.5x)
    match weighted_score {
        0..=10 => 400,    // Very high risk
        11..=20 => 350,   // High risk
        21..=30 => 300,   // High-medium risk
        31..=40 => 250,   // Medium-high risk
        41..=50 => 200,   // Medium risk
        51..=60 => 170,   // Medium-low risk
        61..=70 => 140,   // Low-medium risk
        71..=80 => 110,   // Low risk
        81..=90 => 85,    // Very low risk
        _ => 60,          // Minimal risk
    }
}

/// Coverage type multiplier
fn coverage_type_multiplier(coverage_type: &CoverageType) -> u32 {
    match coverage_type {
        CoverageType::Fire => 100,
        CoverageType::Theft => 80,
        CoverageType::Flood => 150,
        CoverageType::Earthquake => 200,
        CoverageType::LiabilityDamage => 120,
        CoverageType::NaturalDisaster => 180,
        CoverageType::Comprehensive => 250,
    }
}

/// Pool utilization adjustment
/// Higher utilization = higher premiums to manage risk
fn calculate_pool_utilization_multiplier(pool: &RiskPool) -> u32 {
    if pool.total_capital == 0 {
        return 200; // Default high multiplier if no capital
    }

    // Utilization rate: (total_capital - available_capital) / total_capital
    let utilized = pool.total_capital.saturating_sub(pool.available_capital);
    let utilization_rate = utilized.saturating_mul(100) / pool.total_capital;

    // Adjust multiplier based on utilization
    match utilization_rate {
        0..=30 => 90,    // Low utilization - discount
        31..=50 => 100,  // Normal utilization
        51..=70 => 115,  // Medium-high utilization - slight increase
        71..=85 => 135,  // High utilization - significant increase
        _ => 160,        // Critical utilization - major increase
    }
}

/// Time-based adjustment
/// Longer policies get slight discounts for stability
fn calculate_time_multiplier(duration_seconds: u64) -> u32 {
    match duration_seconds {
        0..=2_592_000 => 105,    // < 30 days - short term premium
        2_592_001..=7_776_000 => 100,  // 1-3 months - standard
        7_776_001..=15_552_000 => 95,  // 3-6 months - slight discount
        15_552_001..=31_536_000 => 90, // 6-12 months - good discount
        _ => 85,                       // > 1 year - best discount
    }
}

/// Calculate discount multiplier from modifiers
fn calculate_discount_multiplier(modifiers: &PremiumModifiers) -> u32 {
    let mut total_discount_bps: u32 = 0;

    // Multi-policy discount (up to 15%)
    if modifiers.has_multiple_policies {
        total_discount_bps = total_discount_bps.saturating_add(1500);
    }

    // Claim-free discount (up to 20% based on years)
    if modifiers.claim_free_years > 0 {
        let claim_free_discount = match modifiers.claim_free_years {
            1 => 500,    // 5%
            2 => 1000,   // 10%
            3 => 1500,   // 15%
            _ => 2000,   // 20% for 4+ years
        };
        total_discount_bps = total_discount_bps.saturating_add(claim_free_discount);
    }

    // Safety features discount (up to 10%)
    if modifiers.has_safety_features {
        total_discount_bps = total_discount_bps.saturating_add(1000);
    }

    // Loyalty discount (up to 10%)
    if modifiers.loyalty_years > 0 {
        let loyalty_discount = match modifiers.loyalty_years {
            1..=2 => 300,    // 3%
            3..=5 => 600,    // 6%
            _ => 1000,       // 10% for 6+ years
        };
        total_discount_bps = total_discount_bps.saturating_add(loyalty_discount);
    }

    // Cap total discount at 40%
    if total_discount_bps > 4000 {
        total_discount_bps = 4000;
    }

    // Convert discount to multiplier (10000 - discount_bps)
    10_000u32.saturating_sub(total_discount_bps)
}

/// Calculate deductible based on risk and modifiers
fn calculate_deductible(
    coverage_amount: u128,
    assessment: &RiskAssessment,
    modifiers: &PremiumModifiers,
) -> u128 {
    // Base deductible: 5% of coverage
    let base_deductible_rate: u32 = 500; // 5% in basis points

    // Adjust based on risk (higher risk = higher deductible)
    let risk_adjustment: u32 = match assessment.overall_risk_score {
        0..=20 => 200,     // Very high risk - 20% deductible
        21..=40 => 150,    // High risk - 15%
        41..=60 => 100,    // Medium risk - 10%
        61..=80 => 75,     // Low risk - 7.5%
        _ => 50,           // Very low risk - 5%
    };

    let deductible_rate = base_deductible_rate.saturating_add(risk_adjustment);

    // Apply safety feature reduction
    let reduction: u32 = 50;
    let final_rate = if modifiers.has_safety_features {
        deductible_rate.saturating_sub(reduction)
    } else {
        deductible_rate
    };

    coverage_amount.saturating_mul(final_rate as u128) / 10_000
}

/// Calculate risk adjustment amount for breakdown
fn calculate_risk_adjustment_amount(
    coverage: u128,
    base_rate: u32,
    risk_multiplier: u32,
) -> u128 {
    let base_premium = coverage.saturating_mul(base_rate as u128) / BASIS_POINTS_DENOMINATOR;
    let risk_adjusted = base_premium.saturating_mul(risk_multiplier as u128) / BASIS_POINTS_DENOMINATOR;
    risk_adjusted.saturating_sub(base_premium)
}

/// Calculate coverage adjustment amount (difference that coverage_multiplier adds)
fn calculate_coverage_adjustment_amount(
    coverage: u128,
    base_rate: u32,
    risk_multiplier: u32,
    coverage_multiplier: u32,
) -> u128 {
    let premium_before = coverage
        .saturating_mul(base_rate as u128)
        .saturating_mul(risk_multiplier as u128)
        / PREMIUM_CALCULATION_DIVISOR;

    let premium_after = premium_before
        .saturating_mul(coverage_multiplier as u128)
        / BASIS_POINTS_DENOMINATOR;

    premium_after.saturating_sub(premium_before)
}

/// Calculate pool adjustment amount (difference that pool_utilization_multiplier adds)
fn calculate_pool_adjustment_amount(
    coverage: u128,
    base_rate: u32,
    risk_multiplier: u32,
    coverage_multiplier: u32,
    pool_multiplier: u32,
) -> u128 {
    let premium_before = coverage
        .saturating_mul(base_rate as u128)
        .saturating_mul(risk_multiplier as u128)
        .saturating_mul(coverage_multiplier as u128)
        / PREMIUM_CALCULATION_DIVISOR;

    let premium_after = premium_before
        .saturating_mul(pool_multiplier as u128)
        / BASIS_POINTS_DENOMINATOR;

    premium_after.saturating_sub(premium_before)
}

/// Calculate time adjustment amount (difference that time_multiplier adds)
fn calculate_time_adjustment_amount(
    coverage: u128,
    base_rate: u32,
    risk_multiplier: u32,
    coverage_multiplier: u32,
    pool_multiplier: u32,
    time_multiplier: u32,
) -> u128 {
    let premium_before = coverage
        .saturating_mul(base_rate as u128)
        .saturating_mul(risk_multiplier as u128)
        .saturating_mul(coverage_multiplier as u128)
        .saturating_mul(pool_multiplier as u128)
        / PREMIUM_CALCULATION_DIVISOR_LARGE;

    let premium_after = premium_before
        .saturating_mul(time_multiplier as u128)
        / BASIS_POINTS_DENOMINATOR;

    premium_after.saturating_sub(premium_before)
}

/// Calculate discount amount
fn calculate_discount_amount(
    coverage: u128,
    base_rate: u32,
    risk_multiplier: u32,
    coverage_multiplier: u32,
    pool_multiplier: u32,
    time_multiplier: u32,
    discount_multiplier: u32,
) -> u128 {
    let premium_before_discount = coverage
        .saturating_mul(base_rate as u128)
        .saturating_mul(risk_multiplier as u128)
        .saturating_mul(coverage_multiplier as u128)
        .saturating_mul(pool_multiplier as u128)
        .saturating_mul(time_multiplier as u128)
        / PREMIUM_CALCULATION_DIVISOR_5MULT;

    let final_premium = premium_before_discount
        .saturating_mul(discount_multiplier as u128)
        / BASIS_POINTS_DENOMINATOR;

    premium_before_discount.saturating_sub(final_premium)
}

// Constants
const BASIS_POINTS_DENOMINATOR: u128 = 10_000;
const SECONDS_PER_YEAR: u128 = 31_536_000; // 365 * 24 * 60 * 60
const PREMIUM_CALCULATION_DIVISOR: u128 = 1_000_000_000_000_000_000; // 10^18 for 5 multipliers
const PREMIUM_CALCULATION_DIVISOR_LARGE: u128 = 10_000_000_000_000_000_000_000; // 10^22 for 6 multipliers
const PREMIUM_CALCULATION_DIVISOR_5MULT: u128 = 1_000_000_000_000_000_000_000; // 10^21 for discount calc
