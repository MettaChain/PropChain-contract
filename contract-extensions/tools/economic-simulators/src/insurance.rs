use crate::{rng::Rng, ModelError};

const BPS: u128 = 10_000;
const DEFAULT_ITERATIONS: usize = 10_000;
const DEFAULT_SEED: u64 = 0x1516_0001;

/// Frequency and severity assumptions for one insured policy.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PolicyRisk {
    pub coverage_amount: u128,
    /// Existing insurance scoring convention: 0 is highest risk, 100 lowest.
    pub risk_score: u32,
    /// Annual claim probability in basis points, from 0 to 10,000.
    pub claim_frequency_bps: u32,
    pub minimum_claim_severity_bps: u32,
    pub maximum_claim_severity_bps: u32,
}

/// Solvency and premium results across simulated annual claim scenarios.
#[derive(Clone, Debug, PartialEq)]
pub struct PoolSolvencyResult {
    pub premium_per_policy: Vec<u128>,
    pub total_premiums: u128,
    pub expected_annual_claims: u128,
    pub premium_income_covers_expected_claims: bool,
    pub insolvency_probability: f64,
    pub solvency_probability: f64,
    /// Reserves needed in addition to collected annual premiums to cover the
    /// 99th percentile of simulated annual claims.
    pub reserve_recommendation: u128,
    /// Claims if all policies claim at their maximum modeled severity.
    pub worst_case_claims: u128,
    pub iterations: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PremiumComparison {
    pub modeled_premium: u128,
    pub dynamic_reference_premium: u128,
    pub difference: i128,
}

/// Estimate an annual premium using expected losses, the repository's risk
/// score bands, a 20% expense loading, and a capital-shortfall adjustment.
///
/// `claim_frequency_bps` may exceed 10,000 to represent multiple expected
/// claims per year. A pool smaller than the coverage receives a surcharge
/// proportional to the coverage-to-pool ratio, capped at 4x.
pub fn calculate_premium(
    coverage_amount: u128,
    risk_score: u32,
    pool_size: u128,
    claim_frequency_bps: u32,
) -> u128 {
    let expected_claim = coverage_amount.saturating_mul(claim_frequency_bps as u128) / BPS;
    let risk_adjusted = expected_claim.saturating_mul(risk_multiplier_bps(risk_score)) / BPS;
    let expense_loaded = risk_adjusted.saturating_mul(12_000) / BPS;
    let capital_multiplier = if pool_size == 0 {
        40_000
    } else {
        (coverage_amount.saturating_mul(BPS) / pool_size).clamp(10_000, 40_000)
    };
    expense_loaded.saturating_mul(capital_multiplier) / BPS
}

/// Risk multiplier in basis points, following the insurance contract's score
/// bands. Scores outside 0-100 are clamped to the nearest band.
pub fn risk_multiplier_bps(risk_score: u32) -> u128 {
    match risk_score.min(100) {
        0..=10 => 40_000,
        11..=20 => 35_000,
        21..=30 => 30_000,
        31..=40 => 25_000,
        41..=50 => 20_000,
        51..=60 => 17_000,
        61..=70 => 14_000,
        71..=80 => 11_000,
        81..=90 => 8_500,
        _ => 6_000,
    }
}

/// Run a reproducible 10,000-trial annual pool simulation.
pub fn simulate_pool_solvency(
    policies: &[PolicyRisk],
    pool_reserves: u128,
) -> Result<PoolSolvencyResult, ModelError> {
    simulate_pool_solvency_with_options(policies, pool_reserves, DEFAULT_ITERATIONS, DEFAULT_SEED)
}

/// Simulate claim occurrence and severity, with explicit iterations and seed.
pub fn simulate_pool_solvency_with_options(
    policies: &[PolicyRisk],
    pool_reserves: u128,
    iterations: usize,
    seed: u64,
) -> Result<PoolSolvencyResult, ModelError> {
    if policies.is_empty() {
        return Err(ModelError::EmptyPopulation);
    }
    if iterations == 0 {
        return Err(ModelError::ZeroIterations);
    }
    for policy in policies {
        if policy.claim_frequency_bps > 10_000
            || policy.minimum_claim_severity_bps > policy.maximum_claim_severity_bps
            || policy.maximum_claim_severity_bps > 10_000
        {
            return Err(ModelError::InvalidParameter(
                "claim frequency and severity must be valid basis-point probabilities",
            ));
        }
    }

    let premiums = policies
        .iter()
        .map(|policy| {
            calculate_premium(
                policy.coverage_amount,
                policy.risk_score,
                pool_reserves,
                policy.claim_frequency_bps,
            )
        })
        .collect::<Vec<_>>();
    let total_premiums = premiums.iter().copied().fold(0u128, u128::saturating_add);
    let worst_case_claims = policies.iter().fold(0u128, |total, policy| {
        total.saturating_add(
            policy
                .coverage_amount
                .saturating_mul(policy.maximum_claim_severity_bps as u128)
                / BPS,
        )
    });

    let mut rng = Rng::new(seed);
    let mut claim_totals = Vec::with_capacity(iterations);
    let mut total_claims = 0u128;
    let mut insolvent_trials = 0usize;
    for _ in 0..iterations {
        let mut claims = 0u128;
        for policy in policies {
            if rng.chance_bps(policy.claim_frequency_bps) {
                let severity_span =
                    policy.maximum_claim_severity_bps - policy.minimum_claim_severity_bps;
                let severity = policy.minimum_claim_severity_bps
                    + (rng.unit() * (severity_span as f64 + 1.0)) as u32;
                let severity = severity.min(policy.maximum_claim_severity_bps);
                claims = claims.saturating_add(
                    policy.coverage_amount.saturating_mul(severity as u128) / BPS,
                );
            }
        }
        total_claims = total_claims.saturating_add(claims);
        if claims > pool_reserves.saturating_add(total_premiums) {
            insolvent_trials += 1;
        }
        claim_totals.push(claims);
    }

    claim_totals.sort_unstable();
    let percentile_99_index = ((iterations - 1) * 99) / 100;
    let claims_at_99th_percentile = claim_totals[percentile_99_index];
    let expected_annual_claims = total_claims / iterations as u128;

    Ok(PoolSolvencyResult {
        premium_per_policy: premiums,
        total_premiums,
        expected_annual_claims,
        premium_income_covers_expected_claims: total_premiums >= expected_annual_claims,
        insolvency_probability: insolvent_trials as f64 / iterations as f64,
        solvency_probability: 1.0 - insolvent_trials as f64 / iterations as f64,
        reserve_recommendation: claims_at_99th_percentile.saturating_sub(total_premiums),
        worst_case_claims,
        iterations,
    })
}

/// Reproduce the dynamic premium engine's documented fixed-point product for
/// comparison. Every multiplier is in basis points; the divisor matches the
/// contract engine's current `PREMIUM_CALCULATION_DIVISOR`.
pub fn calculate_dynamic_reference_premium(
    coverage_amount: u128,
    base_rate_bps: u32,
    risk_multiplier_bps: u32,
    coverage_multiplier_bps: u32,
    pool_utilization_multiplier_bps: u32,
    time_multiplier_bps: u32,
    discount_multiplier_bps: u32,
    claim_frequency_multiplier_bps: u32,
) -> u128 {
    coverage_amount
        .saturating_mul(base_rate_bps as u128)
        .saturating_mul(risk_multiplier_bps as u128)
        .saturating_mul(coverage_multiplier_bps as u128)
        .saturating_mul(pool_utilization_multiplier_bps as u128)
        .saturating_mul(time_multiplier_bps as u128)
        .saturating_mul(discount_multiplier_bps as u128)
        .saturating_mul(claim_frequency_multiplier_bps as u128)
        / 10_000_000_000_000_000_000_000_000
}

/// Compare this model's result with a premium produced by the contract formula.
pub fn compare_with_dynamic_reference(
    modeled_premium: u128,
    dynamic_reference_premium: u128,
) -> PremiumComparison {
    let difference = if modeled_premium >= dynamic_reference_premium {
        modeled_premium
            .saturating_sub(dynamic_reference_premium)
            .min(i128::MAX as u128) as i128
    } else {
        -(dynamic_reference_premium
            .saturating_sub(modeled_premium)
            .min(i128::MAX as u128) as i128)
    };
    PremiumComparison {
        modeled_premium,
        dynamic_reference_premium,
        difference,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn premium_increases_with_risk_and_pool_capital_shortfall() {
        let low_risk = calculate_premium(100_000, 100, 1_000_000, 500);
        let high_risk = calculate_premium(100_000, 10, 1_000_000, 500);
        let constrained_pool = calculate_premium(100_000, 100, 10_000, 500);
        assert!(high_risk > low_risk);
        assert!(constrained_pool > low_risk);
    }

    #[test]
    fn solvency_simulation_reports_full_claim_stress_and_percentile_reserve() {
        let policies = vec![
            PolicyRisk {
                coverage_amount: 1_000,
                risk_score: 100,
                claim_frequency_bps: 10_000,
                minimum_claim_severity_bps: 10_000,
                maximum_claim_severity_bps: 10_000,
            };
            10
        ];
        let result =
            simulate_pool_solvency_with_options(&policies, 1_000, 100, 9).unwrap();
        assert_eq!(result.worst_case_claims, 10_000);
        assert_eq!(result.solvency_probability, 0.0);
        assert!(result.reserve_recommendation > 0);
    }

    #[test]
    fn compares_against_the_documented_dynamic_formula() {
        assert_eq!(
            calculate_dynamic_reference_premium(100_000, 100, 10_000, 10_000, 10_000, 10_000, 10_000, 10_000),
            1_000_000
        );
        assert_eq!(
            compare_with_dynamic_reference(120, 100).difference,
            20
        );
    }
}
