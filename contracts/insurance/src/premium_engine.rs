// Dynamic premium calculation engine based on risk assessment
// Implements actuarial pricing with real-time adjustments
// Claim-frequency adjustment added (rolling-window surcharge)

// Issue #1162: this file was `include!()`-ed into `mod propchain_insurance`,
// so the six types below resolved out of that module's `include!("types.rs")`
// scope. As a real module it has to name them itself. Every one of them is
// `pub` with `pub` fields, which is why the move is additive rather than a
// visibility change.
use crate::propchain_insurance::{
    ActuarialModel, CoverageType, PremiumCalculation, PremiumModifiers, RiskAssessment, RiskPool,
};


/// Dynamic premium calculation with comprehensive risk factors.
///
/// Steps:
///  1. Base rate (actuarial model or coverage-type default)
///  2. Risk multiplier (weighted location / construction / age / claims-history scores)
///  3. Coverage-type multiplier
///  4. Pool-utilization multiplier
///  5. Time-based multiplier
///  6. Discount multiplier (multi-policy, claim-free, safety, loyalty)
///  7. **Claim-frequency multiplier** (rolling-window surcharge — new)
///
/// The claim-frequency step uses `modifiers.recent_claims_count`, which the
/// caller populates with the number of approved claims filed against the
/// property within the rolling observation window (default: 12 months).
/// Defaults match the table in `DYNAMIC_PREMIUM_CALCULATION.md`.
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

    // Step 7: Rolling-window claim-frequency surcharge
    // recent_claims_count = number of approved claims in the observation window
    let claim_freq_multiplier =
        calculate_claim_frequency_multiplier(modifiers.recent_claims_count);

    // Formula: coverage × base_rate × risk × coverage × pool × time × discount × claim_freq
    // All multipliers are in basis points (10 000 = 1.0×), divisor accounts for all 7 factors.
    let annual_premium = coverage_amount
        .saturating_mul(base_rate as u128)
        .saturating_mul(risk_multiplier as u128)
        .saturating_mul(coverage_multiplier as u128)
        .saturating_mul(pool_utilization_multiplier as u128)
        .saturating_mul(time_multiplier as u128)
        .saturating_mul(discount_multiplier as u128)
        .saturating_mul(claim_freq_multiplier as u128)
        / PREMIUM_CALCULATION_DIVISOR;

    // Prorate for policy duration
    let duration_premium =
        annual_premium.saturating_mul(policy_duration_seconds as u128) / SECONDS_PER_YEAR;

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
        claim_freq_multiplier,
        annual_premium: duration_premium,
        monthly_premium,
        deductible,
        breakdown: PremiumBreakdown {
            base_premium: coverage_amount.saturating_mul(base_rate as u128)
                / BASIS_POINTS_DENOMINATOR,
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
            claim_freq_adjustment: calculate_claim_freq_adjustment_amount(
                coverage_amount,
                base_rate,
                risk_multiplier,
                coverage_multiplier,
                pool_utilization_multiplier,
                time_multiplier,
                discount_multiplier,
                claim_freq_multiplier,
            ),
        },
    }
}

/// Calculate base rate from actuarial model
fn calculate_base_rate(
    actuarial_model: Option<&ActuarialModel>,
    coverage_type: &CoverageType,
) -> u32 {
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
        CoverageType::Fire => 120,            // 1.2%
        CoverageType::Flood => 200,           // 2.0%
        CoverageType::Earthquake => 250,      // 2.5%
        CoverageType::Theft => 100,           // 1.0%
        CoverageType::LiabilityDamage => 150, // 1.5%
        CoverageType::NaturalDisaster => 220, // 2.2%
        CoverageType::Comprehensive => 300,   // 3.0%
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
        0..=10 => 400,  // Very high risk
        11..=20 => 350, // High risk
        21..=30 => 300, // High-medium risk
        31..=40 => 250, // Medium-high risk
        41..=50 => 200, // Medium risk
        51..=60 => 170, // Medium-low risk
        61..=70 => 140, // Low-medium risk
        71..=80 => 110, // Low risk
        81..=90 => 85,  // Very low risk
        _ => 60,        // Minimal risk
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
        0..=30 => 90,   // Low utilization - discount
        31..=50 => 100, // Normal utilization
        51..=70 => 115, // Medium-high utilization - slight increase
        71..=85 => 135, // High utilization - significant increase
        _ => 160,       // Critical utilization - major increase
    }
}

/// Time-based adjustment
/// Longer policies get slight discounts for stability
fn calculate_time_multiplier(duration_seconds: u64) -> u32 {
    match duration_seconds {
        0..=2_592_000 => 105,          // < 30 days - short term premium
        2_592_001..=7_776_000 => 100,  // 1-3 months - standard
        7_776_001..=15_552_000 => 95,  // 3-6 months - slight discount
        15_552_001..=31_536_000 => 90, // 6-12 months - good discount
        _ => 85,                       // > 1 year - best discount
    }
}

/// Rolling-window claim-frequency surcharge multiplier.
///
/// Uses the number of approved claims filed within the rolling observation
/// window (typically 12 months) stored in `PremiumModifiers::recent_claims_count`.
///
/// | Claims in window | Multiplier | Basis points |
/// |-----------------|------------|-------------|
/// | 0               | 1.00×      | 10 000       | (no surcharge — baseline)
/// | 1               | 1.10×      | 11 000       |
/// | 2               | 1.25×      | 12 500       |
/// | 3               | 1.50×      | 15 000       |
/// | 4               | 1.75×      | 17 500       |
/// | 5+              | 2.00×      | 20 000       | (high-frequency cap)
fn calculate_claim_frequency_multiplier(recent_claims_count: u32) -> u32 {
    match recent_claims_count {
        0 => 10_000,  // 1.00× — no surcharge
        1 => 11_000,  // 1.10×
        2 => 12_500,  // 1.25×
        3 => 15_000,  // 1.50×
        4 => 17_500,  // 1.75×
        _ => 20_000,  // 2.00× — high-frequency cap (5+ claims)
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
            1 => 500,  // 5%
            2 => 1000, // 10%
            3 => 1500, // 15%
            _ => 2000, // 20% for 4+ years
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
            1..=2 => 300, // 3%
            3..=5 => 600, // 6%
            _ => 1000,    // 10% for 6+ years
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
        0..=20 => 200,  // Very high risk - 20% deductible
        21..=40 => 150, // High risk - 15%
        41..=60 => 100, // Medium risk - 10%
        61..=80 => 75,  // Low risk - 7.5%
        _ => 50,        // Very low risk - 5%
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
fn calculate_risk_adjustment_amount(coverage: u128, base_rate: u32, risk_multiplier: u32) -> u128 {
    let base_premium = coverage.saturating_mul(base_rate as u128) / BASIS_POINTS_DENOMINATOR;
    let risk_adjusted =
        base_premium.saturating_mul(risk_multiplier as u128) / BASIS_POINTS_DENOMINATOR;
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

    let premium_after =
        premium_before.saturating_mul(coverage_multiplier as u128) / BASIS_POINTS_DENOMINATOR;

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

    let premium_after =
        premium_before.saturating_mul(pool_multiplier as u128) / BASIS_POINTS_DENOMINATOR;

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

    let premium_after =
        premium_before.saturating_mul(time_multiplier as u128) / BASIS_POINTS_DENOMINATOR;

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

    let final_premium = premium_before_discount.saturating_mul(discount_multiplier as u128)
        / BASIS_POINTS_DENOMINATOR;

    premium_before_discount.saturating_sub(final_premium)
}

/// Calculate the claim-frequency surcharge amount for the breakdown.
///
/// This is the additional cost the claim-frequency multiplier adds on top of
/// the already-discounted premium (i.e. it is always additive / non-negative).
fn calculate_claim_freq_adjustment_amount(
    coverage: u128,
    base_rate: u32,
    risk_multiplier: u32,
    coverage_multiplier: u32,
    pool_multiplier: u32,
    time_multiplier: u32,
    discount_multiplier: u32,
    claim_freq_multiplier: u32,
) -> u128 {
    // Premium after discount, before claim-frequency step
    let pre_freq = coverage
        .saturating_mul(base_rate as u128)
        .saturating_mul(risk_multiplier as u128)
        .saturating_mul(coverage_multiplier as u128)
        .saturating_mul(pool_multiplier as u128)
        .saturating_mul(time_multiplier as u128)
        .saturating_mul(discount_multiplier as u128)
        / PREMIUM_CALCULATION_DIVISOR;

    let post_freq = pre_freq
        .saturating_mul(claim_freq_multiplier as u128)
        / BASIS_POINTS_DENOMINATOR;

    post_freq.saturating_sub(pre_freq)
}

// Constants
const BASIS_POINTS_DENOMINATOR: u128 = 10_000;
const SECONDS_PER_YEAR: u128 = 31_536_000; // 365 * 24 * 60 * 60
/// Divisor for the 7-factor chain: base_rate × risk × coverage × pool × time × discount × claim_freq
/// All 7 multipliers are in basis points (10 000 = 1.0×), so each divides by 10 000.
/// 10 000^6 = 10^24; but base_rate uses a different scale (it's in raw basis points ÷ 10 000 once),
/// so the combined divisor keeps the same structure as before with an extra ×10 000 for claim_freq.
const PREMIUM_CALCULATION_DIVISOR: u128 = 10_000_000_000_000_000_000_000_000; // 10^25 for 7 factors
const PREMIUM_CALCULATION_DIVISOR_LARGE: u128 = 10_000_000_000_000_000_000_000; // 10^22 for 6 multipliers (breakdown helpers)
const PREMIUM_CALCULATION_DIVISOR_5MULT: u128 = 1_000_000_000_000_000_000_000; // 10^21 for discount calc (5 factors before discount)

#[cfg(test)]
mod tests {
    use super::*;
    use crate::propchain_insurance::RiskLevel;
    use ink::prelude::string::String;

    /// One year, matching `SECONDS_PER_YEAR` above.
    const YEAR: u64 = 31_536_000;

    fn pool(total_capital: u128, available_capital: u128) -> RiskPool {
        RiskPool {
            pool_id: 1,
            name: String::from("unit-test pool"),
            coverage_type: CoverageType::Fire,
            total_capital,
            available_capital,
            total_premiums_collected: 0,
            total_claims_paid: 0,
            active_policies: 1,
            max_coverage_ratio: 0,
            reinsurance_threshold: 0,
            created_at: 0,
            is_active: true,
        }
    }

    fn assessment(
        location: u32,
        construction: u32,
        age: u32,
        claims: u32,
        overall: u32,
    ) -> RiskAssessment {
        RiskAssessment {
            property_id: 1,
            location_risk_score: location,
            construction_risk_score: construction,
            age_risk_score: age,
            claims_history_score: claims,
            overall_risk_score: overall,
            risk_level: RiskLevel::Medium,
            assessed_at: 0,
            valid_until: 0,
        }
    }

    fn modifiers(recent_claims_count: u32) -> PremiumModifiers {
        PremiumModifiers {
            has_multiple_policies: false,
            claim_free_years: 0,
            has_safety_features: false,
            loyalty_years: 0,
            recent_claims_count,
        }
    }

    /// A neutral risk assessment: every component 50, so the weighted score is
    /// exactly 50 and the band lookup is unambiguous.
    fn neutral() -> RiskAssessment {
        assessment(50, 50, 50, 50, 50)
    }

    /// A fully-utilised-ish but non-empty pool, so the pool step is a no-op
    /// multiplier of 100 unless a test is specifically exercising it.
    fn half_full() -> RiskPool {
        pool(1_000_000, 500_000)
    }

    // ---------------------------------------------------------------------
    // The structural check Issue #1162 asked for
    // ---------------------------------------------------------------------

    /// Issue #1162's acceptance criterion: the module is declared exactly
    /// once, and the duplicate-declaration hazard the old comment described is
    /// gone.
    ///
    /// The count is done on whole trimmed lines rather than by substring so
    /// that prose *about* `mod premium_engine;` in a comment does not read as a
    /// second declaration.
    #[test]
    fn premium_engine_is_declared_exactly_once() {
        let lib = include_str!("lib.rs");

        let declarations: Vec<&str> = lib
            .lines()
            .map(str::trim)
            .filter(|line| *line == "mod premium_engine;" || *line == "pub mod premium_engine;")
            .collect();

        assert_eq!(
            declarations.len(),
            1,
            "expected exactly one `mod premium_engine;` declaration in lib.rs, found {declarations:?}"
        );
    }

    /// The engine must no longer be textually spliced into the contract module.
    #[test]
    fn premium_engine_is_no_longer_include_d() {
        let lib = include_str!("lib.rs");
        assert!(
            !lib.contains("include!(\"premium_engine.rs\")"),
            "lib.rs still include!()s premium_engine.rs into the contract module"
        );
    }

    /// The hazard comment the issue asked to have removed must be gone, and the
    /// wording that would reintroduce the trap must not come back.
    #[test]
    fn the_duplicate_declaration_hazard_comment_is_gone() {
        let lib = include_str!("lib.rs");
        for marker in [
            "E0255",
            "Do NOT also",
            "double-declare",
            "defined multiple times",
        ] {
            assert!(
                !lib.contains(marker),
                "lib.rs still carries the hazard marker {marker:?}"
            );
        }
    }

    /// `lib.rs` reaches the engine by path (`use crate::premium_engine::...`).
    /// This test compiling at all is the proof that the `use crate::premium_engine`
    /// assumption #1162 called fragile now holds, because the call below is
    /// written against that exact path rather than an in-scope bare name.
    #[test]
    fn the_engine_is_reachable_by_module_path() {
        let calculation = crate::premium_engine::calculate_dynamic_premium(
            &neutral(),
            100_000_000,
            &CoverageType::Fire,
            &half_full(),
            None,
            &modifiers(0),
            YEAR,
        );
        assert!(calculation.annual_premium > 0);
    }

    // ---------------------------------------------------------------------
    // Base rate
    // ---------------------------------------------------------------------

    #[test]
    fn base_rate_falls_back_to_the_coverage_type_default() {
        // No actuarial model: the table in `coverage_type_base_rate` is used
        // verbatim, so these are exact rather than derived.
        assert_eq!(coverage_type_base_rate(&CoverageType::Fire), 120);
        assert_eq!(coverage_type_base_rate(&CoverageType::Theft), 100);
        assert_eq!(coverage_type_base_rate(&CoverageType::LiabilityDamage), 150);
        assert_eq!(coverage_type_base_rate(&CoverageType::Flood), 200);
        assert_eq!(coverage_type_base_rate(&CoverageType::NaturalDisaster), 220);
        assert_eq!(coverage_type_base_rate(&CoverageType::Earthquake), 250);
        assert_eq!(coverage_type_base_rate(&CoverageType::Comprehensive), 300);
    }

    #[test]
    fn every_coverage_type_has_a_base_rate() {
        // A `match` arm missing a variant would be a compile error, so this test
        // is really pinning that all seven are reachable and non-zero.
        for (coverage, expected) in [
            (CoverageType::Fire, 120u32),
            (CoverageType::Theft, 100),
            (CoverageType::LiabilityDamage, 150),
            (CoverageType::Flood, 200),
            (CoverageType::NaturalDisaster, 220),
            (CoverageType::Earthquake, 250),
            (CoverageType::Comprehensive, 300),
        ] {
            assert_eq!(coverage_type_base_rate(&coverage), expected);
            assert!(expected > 0);
        }
    }

    #[test]
    fn an_actuarial_model_overrides_the_coverage_type_default() {
        let model = ActuarialModel {
            model_id: 1,
            coverage_type: CoverageType::Fire,
            loss_frequency: 2,
            average_loss_severity: 50_000,
            expected_loss_ratio: 600,
            // 95% maps to a confidence adjustment of 100 (i.e. no loading).
            confidence_level: 95,
            last_updated: 0,
            data_points: 500,
        };

        // 600 * 100 / 100 = 600, then the 20% expense loading: 600 * 120 / 100.
        let expected = 600u32 * 100 / 100 * 120 / 100;
        assert_eq!(expected, 720);
        assert_eq!(calculate_base_rate(Some(&model), &CoverageType::Fire), expected);
        // The same model beats the 120 bps default the table would have used.
        assert!(expected > coverage_type_base_rate(&CoverageType::Fire));
    }

    #[test]
    fn a_higher_confidence_level_raises_the_model_base_rate() {
        let model_at = |confidence_level: u32| ActuarialModel {
            model_id: 1,
            coverage_type: CoverageType::Fire,
            loss_frequency: 2,
            average_loss_severity: 50_000,
            expected_loss_ratio: 600,
            confidence_level,
            last_updated: 0,
            data_points: 500,
        };

        let low = calculate_base_rate(Some(&model_at(95)), &CoverageType::Fire);
        let high = calculate_base_rate(Some(&model_at(99)), &CoverageType::Fire);
        assert!(high > low, "99% confidence must price above 95%");
        // 99 -> 120, so 600 * 120 / 100 = 720, then * 120 / 100 = 864.
        assert_eq!(high, 864);
    }

    #[test]
    fn an_unmodelled_confidence_level_falls_back_to_neutral() {
        let model_with = |confidence_level: u32| ActuarialModel {
            model_id: 1,
            coverage_type: CoverageType::Fire,
            loss_frequency: 0,
            average_loss_severity: 0,
            expected_loss_ratio: 600,
            confidence_level,
            last_updated: 0,
            data_points: 0,
        };
        // 94 is not in the 95..=99 table and must not price as 99.
        assert_eq!(
            calculate_base_rate(Some(&model_with(94)), &CoverageType::Fire),
            calculate_base_rate(Some(&model_with(95)), &CoverageType::Fire)
        );
        assert!(
            calculate_base_rate(Some(&model_with(99)), &CoverageType::Fire)
                > calculate_base_rate(Some(&model_with(94)), &CoverageType::Fire)
        );
    }

    // ---------------------------------------------------------------------
    // Risk multiplier
    // ---------------------------------------------------------------------

    #[test]
    fn the_riskiest_and_safest_scores_hit_the_band_ends() {
        // All-zero components -> weighted score 0 -> the 400 bps band.
        assert_eq!(
            calculate_risk_multiplier(&assessment(0, 0, 0, 0, 0)),
            400
        );
        // All-100 components -> weighted score 100 -> the 60 bps band.
        assert_eq!(
            calculate_risk_multiplier(&assessment(100, 100, 100, 100, 100)),
            60
        );
    }

    #[test]
    fn the_risk_multiplier_is_monotonically_non_increasing_in_risk() {
        // A band table that inverted somewhere would silently under-price the
        // riskiest properties, so walk every band boundary in order.
        let mut previous = u32::MAX;
        for score in [0u32, 10, 11, 20, 21, 30, 31, 40, 41, 50, 51, 60, 61, 70, 71, 80, 81, 90, 91, 100] {
            let multiplier = calculate_risk_multiplier(&assessment(score, score, score, score, score));
            assert!(
                multiplier <= previous,
                "multiplier rose from {previous} to {multiplier} at score {score}"
            );
            previous = multiplier;
        }
    }

    #[test]
    fn the_location_component_outweighs_the_age_component() {
        // Weights: location 30%, construction 25%, age 20%, claims 25%. A single
        // component driven to 100 lands on a different weighted score depending
        // on its weight -- 30 for location, 20 for age -- and those fall in
        // different bands, so the two must not price identically. (These
        // component scores are "higher is safer": 100 is the best rating, which
        // is why the heavier-weighted component yields the cheaper multiplier.)
        let location_only = calculate_risk_multiplier(&assessment(100, 0, 0, 0, 0));
        let age_only = calculate_risk_multiplier(&assessment(0, 0, 100, 0, 0));

        // location: 100*30/100 = 30 -> 21..=30 band
        assert_eq!(location_only, 300);
        // age: 100*20/100 = 20 -> 11..=20 band
        assert_eq!(age_only, 350);
        assert!(age_only > location_only);
    }

    #[test]
    fn the_weights_total_one_hundred_so_the_score_is_not_rescaled() {
        // If the four weights did not sum to 100, a component-maxed assessment
        // would not land on weighted score 100 and the cheapest band would be
        // unreachable.
        assert_eq!(
            calculate_risk_multiplier(&assessment(100, 100, 100, 100, 100)),
            60
        );
    }

    // ---------------------------------------------------------------------
    // Pool utilisation
    // ---------------------------------------------------------------------

    #[test]
    fn an_empty_pool_is_priced_as_maximally_loaded() {
        // `total_capital == 0` is the divide-by-zero guard; it must return the
        // 200 bps default rather than saturating the utilisation calculation.
        assert_eq!(calculate_pool_utilization_multiplier(&pool(0, 0)), 200);
    }

    #[test]
    fn pool_utilisation_bands_step_up_as_the_pool_fills() {
        // total 100_000, so utilisation = (100_000 - available) / 100_000.
        let at = |utilisation_pct: u128| calculate_pool_utilization_multiplier(&pool(
            100_000,
            100_000 - utilisation_pct * 1_000,
        ));

        assert_eq!(at(0), 90); // 0% utilised
        assert_eq!(at(30), 90);
        assert_eq!(at(31), 100);
        assert_eq!(at(50), 100);
        assert_eq!(at(51), 115);
        assert_eq!(at(70), 115);
        assert_eq!(at(71), 135);
        assert_eq!(at(85), 135);
        assert_eq!(at(86), 160);
        assert_eq!(at(100), 160);
    }

    #[test]
    fn pool_utilisation_is_monotonically_non_decreasing() {
        let mut previous = 0u32;
        for utilisation_pct in 0..=100u128 {
            let multiplier = calculate_pool_utilization_multiplier(&pool(
                100_000,
                100_000 - utilisation_pct * 1_000,
            ));
            assert!(
                multiplier >= previous,
                "multiplier fell from {previous} to {multiplier} at {utilisation_pct}%"
            );
            previous = multiplier;
        }
    }

    #[test]
    fn an_over_drawn_pool_saturates_instead_of_wrapping() {
        // available > total means `total - available` underflows if it is not
        // saturating. Saturating to 0 means the pool reads as *un*utilised (90),
        // not as a huge wrapped utilisation rate in the 160 bps band.
        assert_eq!(calculate_pool_utilization_multiplier(&pool(1_000, 5_000)), 90);
    }

    // ---------------------------------------------------------------------
    // Duration
    // ---------------------------------------------------------------------

    #[test]
    fn duration_bands_match_the_published_schedule() {
        assert_eq!(calculate_time_multiplier(0), 105);
        assert_eq!(calculate_time_multiplier(2_592_000), 105);
        assert_eq!(calculate_time_multiplier(2_592_001), 100);
        assert_eq!(calculate_time_multiplier(7_776_000), 100);
        assert_eq!(calculate_time_multiplier(7_776_001), 95);
        assert_eq!(calculate_time_multiplier(15_552_000), 95);
        assert_eq!(calculate_time_multiplier(15_552_001), 90);
        assert_eq!(calculate_time_multiplier(YEAR), 90);
        assert_eq!(calculate_time_multiplier(YEAR + 1), 85);
    }

    #[test]
    fn longer_policies_never_cost_more_per_year() {
        let mut previous = u32::MAX;
        for seconds in [0u64, 2_592_000, 2_592_001, 7_776_001, 15_552_001, YEAR, YEAR + 1] {
            let multiplier = calculate_time_multiplier(seconds);
            assert!(
                multiplier <= previous,
                "multiplier rose from {previous} to {multiplier} at {seconds}s"
            );
            previous = multiplier;
        }
    }

    // ---------------------------------------------------------------------
    // Claim-frequency surcharge
    // ---------------------------------------------------------------------

    #[test]
    fn claim_frequency_multiplier_matches_the_published_table() {
        assert_eq!(calculate_claim_frequency_multiplier(0), 10_000);
        assert_eq!(calculate_claim_frequency_multiplier(1), 11_000);
        assert_eq!(calculate_claim_frequency_multiplier(2), 12_500);
        assert_eq!(calculate_claim_frequency_multiplier(3), 15_000);
        assert_eq!(calculate_claim_frequency_multiplier(4), 17_500);
        assert_eq!(calculate_claim_frequency_multiplier(5), 20_000);
    }

    #[test]
    fn claim_frequency_multiplier_caps_at_five_claims() {
        assert_eq!(calculate_claim_frequency_multiplier(6), 20_000);
        assert_eq!(calculate_claim_frequency_multiplier(u32::MAX), 20_000);
    }

    #[test]
    fn claim_frequency_multiplier_never_discounts() {
        // A multiplier below 10_000 would be a rebate for having claimed, which
        // is the opposite of the intent.
        for claims in 0..=10u32 {
            assert!(
                calculate_claim_frequency_multiplier(claims) >= 10_000,
                "{claims} claims priced below baseline"
            );
        }
    }

    // ---------------------------------------------------------------------
    // Discounts
    // ---------------------------------------------------------------------

    #[test]
    fn no_discounts_is_a_no_op() {
        assert_eq!(calculate_discount_multiplier(&modifiers(0)), 10_000);
    }

    #[test]
    fn each_discount_source_lowers_the_multiplier() {
        let baseline = calculate_discount_multiplier(&modifiers(0));
        let with = |m: PremiumModifiers| calculate_discount_multiplier(&m);

        let multi_policy = with(PremiumModifiers {
            has_multiple_policies: true,
            ..modifiers(0)
        });
        let claim_free = with(PremiumModifiers {
            claim_free_years: 4,
            ..modifiers(0)
        });
        let safety = with(PremiumModifiers {
            has_safety_features: true,
            ..modifiers(0)
        });
        let loyalty = with(PremiumModifiers {
            loyalty_years: 6,
            ..modifiers(0)
        });

        assert!(multi_policy < baseline);
        assert!(claim_free < baseline);
        assert!(safety < baseline);
        assert!(loyalty < baseline);
    }

    #[test]
    fn the_total_discount_is_capped_at_forty_percent() {
        // 1500 (multi-policy) + 2000 (4+ claim-free) + 1000 (safety)
        // + 1000 (6+ loyalty) = 5500 bps uncapped, which must clamp to 4000.
        let everything = PremiumModifiers {
            has_multiple_policies: true,
            claim_free_years: 4,
            has_safety_features: true,
            loyalty_years: 6,
            recent_claims_count: 0,
        };
        assert_eq!(calculate_discount_multiplier(&everything), 6_000);
    }

    #[test]
    fn the_discount_multiplier_never_reaches_zero() {
        // 40% cap means the floor is 6_000, so a premium can never be fully
        // discounted away.
        let everything = PremiumModifiers {
            has_multiple_policies: true,
            claim_free_years: 99,
            has_safety_features: true,
            loyalty_years: 99,
            recent_claims_count: 0,
        };
        let multiplier = calculate_discount_multiplier(&everything);
        assert!(multiplier >= 6_000);
    }

    // ---------------------------------------------------------------------
    // Deductible
    // ---------------------------------------------------------------------

    #[test]
    fn a_safer_property_pays_a_lower_deductible() {
        // `overall_risk_score` is "higher is safer", same convention as the
        // component scores: 0..=20 is the most dangerous band.
        let dangerous = calculate_deductible(10_000_000, &assessment(0, 0, 0, 0, 10), &modifiers(0));
        let safe = calculate_deductible(10_000_000, &assessment(0, 0, 0, 0, 90), &modifiers(0));
        assert!(
            dangerous > safe,
            "the most dangerous band must carry the highest deductible"
        );
    }

    #[test]
    fn safety_features_reduce_the_deductible() {
        let plain = calculate_deductible(10_000_000, &neutral(), &modifiers(0));
        let with_safety = calculate_deductible(
            10_000_000,
            &neutral(),
            &PremiumModifiers {
                has_safety_features: true,
                ..modifiers(0)
            },
        );
        assert!(with_safety < plain);
    }

    // ---------------------------------------------------------------------
    // End-to-end, through the public entry point
    // ---------------------------------------------------------------------

    fn quote(recent_claims_count: u32) -> PremiumCalculation {
        calculate_dynamic_premium(
            &neutral(),
            100_000_000,
            &CoverageType::Fire,
            &half_full(),
            None,
            &modifiers(recent_claims_count),
            YEAR,
        )
    }

    #[test]
    fn more_recent_claims_never_cost_less() {
        let mut previous = 0u128;
        for claims in 0..=6u32 {
            let premium = quote(claims).annual_premium;
            assert!(
                premium >= previous,
                "premium fell from {previous} to {premium} at {claims} claims"
            );
            previous = premium;
        }
    }

    #[test]
    fn a_clean_history_pays_the_least_and_the_cap_pays_the_most() {
        assert!(quote(0).annual_premium < quote(1).annual_premium);
        assert!(quote(4).annual_premium < quote(5).annual_premium);
        assert_eq!(quote(5).annual_premium, quote(9).annual_premium);
    }

    #[test]
    fn the_surcharge_is_zero_for_a_clean_history() {
        // `calculate_claim_freq_adjustment_amount` returns `post - pre`, and a
        // multiplier of exactly 10_000 makes `post == pre`, so a property with
        // no claims in the window is never surcharged regardless of how the
        // intermediate magnitudes round.
        assert_eq!(quote(0).breakdown.claim_freq_adjustment, 0);
    }

    #[test]
    fn the_surcharge_is_never_negative() {
        // The multiplier table is >= 10_000 for every claim count, so the
        // adjustment can only ever add cost. A negative entry would read as a
        // rebate for having claimed.
        for claims in 0..=9u32 {
            assert!(
                quote(claims).breakdown.claim_freq_adjustment >= 0,
                "{claims} claims produced a negative surcharge"
            );
        }
    }

    #[test]
    fn the_surcharge_never_shrinks_as_claims_accumulate() {
        let mut previous = 0u128;
        for claims in 0..=9u32 {
            let surcharge = quote(claims).breakdown.claim_freq_adjustment;
            assert!(
                surcharge >= previous,
                "surcharge fell from {previous} to {surcharge} at {claims} claims"
            );
            previous = surcharge;
        }
    }

    #[test]
    fn the_quote_is_deterministic() {
        // A quote recomputed from identical inputs must be byte-identical, or
        // two calls in the same block can disagree.
        assert_eq!(quote(2), quote(2));
    }

    #[test]
    fn the_monthly_premium_is_the_annual_premium_over_twelve() {
        // `annual_premium` is the duration-prorated amount, despite the field
        // name; a one-year duration makes the two consistent.
        let calculation = quote(0);
        assert_eq!(calculation.monthly_premium, calculation.annual_premium / 12);
    }

    #[test]
    fn a_shorter_policy_is_prorated_down() {
        let full_year = calculate_dynamic_premium(
            &neutral(),
            100_000_000,
            &CoverageType::Fire,
            &half_full(),
            None,
            &modifiers(0),
            YEAR,
        );
        let half_year = calculate_dynamic_premium(
            &neutral(),
            100_000_000,
            &CoverageType::Fire,
            &half_full(),
            None,
            &modifiers(0),
            YEAR / 2,
        );
        assert!(half_year.annual_premium < full_year.annual_premium);
    }

    #[test]
    fn a_dangerous_property_never_quotes_cheaper() {
        // The component scores are "higher is safer", so the *dangerous*
        // property is the one with all-zero components (weighted score 0, the
        // 400 bps band) and the safe one is all-100 (weighted score 100, the
        // 60 bps band).
        let safe = calculate_dynamic_premium(
            &assessment(100, 100, 100, 100, 95),
            100_000_000,
            &CoverageType::Fire,
            &half_full(),
            None,
            &modifiers(0),
            YEAR,
        );
        let dangerous = calculate_dynamic_premium(
            &assessment(0, 0, 0, 0, 5),
            100_000_000,
            &CoverageType::Fire,
            &half_full(),
            None,
            &modifiers(0),
            YEAR,
        );
        assert!(
            dangerous.annual_premium > safe.annual_premium,
            "the worst-scoring property must not be the cheapest quote"
        );
        assert!(dangerous.deductible > safe.deductible);
    }

    #[test]
    fn the_breakdown_base_premium_ignores_every_multiplier() {
        // `base_premium` is coverage x base_rate before any loading, which is
        // what makes the adjustment columns readable as deltas.
        let calculation = quote(0);
        assert_eq!(
            calculation.breakdown.base_premium,
            100_000_000u128 * calculation.base_rate as u128 / 10_000
        );
    }

    #[test]
    fn a_zero_coverage_amount_quotes_zero_rather_than_panicking() {
        let calculation = calculate_dynamic_premium(
            &neutral(),
            0,
            &CoverageType::Fire,
            &half_full(),
            None,
            &modifiers(3),
            YEAR,
        );
        assert_eq!(calculation.annual_premium, 0);
        assert_eq!(calculation.monthly_premium, 0);
        assert_eq!(calculation.deductible, 0);
    }

    #[test]
    fn an_enormous_coverage_amount_saturates_instead_of_overflowing() {
        let calculation = calculate_dynamic_premium(
            &neutral(),
            u128::MAX / 4,
            &CoverageType::Comprehensive,
            &pool(0, 0),
            None,
            &modifiers(9),
            YEAR,
        );
        // Saturating arithmetic is the point: the call must return, not panic
        // and not wrap into a small premium.
        assert!(calculation.annual_premium > 0);
    }
}
