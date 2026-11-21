use crate::{rng::Rng, ModelError};

const DEFAULT_ITERATIONS: usize = 10_000;
const DEFAULT_SEED: u64 = 0x1517_0001;

/// A campaign milestone reached at a funding threshold, releasing a share of
/// the campaign goal from escrow.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CampaignMilestone {
    pub funding_threshold_bps: u32,
    pub release_amount_bps: u32,
}

/// Inputs for campaign contribution, withdrawal, and milestone simulations.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CampaignConfig {
    pub goal: u128,
    pub deadline_blocks: u64,
    /// Average contribution expected from a participating backer in one block.
    pub contribution_rate: u128,
    pub participation_rate_bps: u32,
    /// Probability of an aggregate cancellation event in each block.
    pub withdrawal_rate_bps: u32,
    /// Share of the deadline used for the early-adopter contribution phase.
    pub early_adopter_share_bps: u32,
    /// Maximum acceleration applied as funding approaches the goal.
    pub momentum_bps: u32,
    pub milestones: Vec<CampaignMilestone>,
    pub iterations: usize,
    pub seed: u64,
}

/// Results include success likelihood, average funding, flow, and releases.
#[derive(Clone, Debug, PartialEq)]
pub struct CampaignResult {
    pub success_probability: f64,
    pub expected_total_raised: u128,
    pub expected_gross_contributions: u128,
    pub average_contribution: u128,
    pub expected_contributor_count: f64,
    pub expected_released_funds: u128,
    pub goal: u128,
    pub deadline_blocks: u64,
    pub iterations: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CampaignSensitivity {
    pub goal: u128,
    pub deadline_blocks: u64,
    pub success_probability: f64,
    pub expected_total_raised: u128,
}

#[derive(Clone, Debug, PartialEq)]
pub struct HistoricalComparison {
    pub model_success_probability: f64,
    pub observed_success_probability: f64,
    /// Positive values mean the model is more optimistic than the observations.
    pub probability_gap: f64,
    pub observed_campaigns: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CampaignObservation {
    pub goal: u128,
    pub final_raised: u128,
}

/// Simulate a campaign with 10,000 trials and documented default assumptions.
pub fn simulate_campaign(
    goal: u128,
    deadline_blocks: u64,
    contribution_rate: u128,
) -> Result<CampaignResult, ModelError> {
    simulate_campaign_with_config(&CampaignConfig {
        goal,
        deadline_blocks,
        contribution_rate,
        participation_rate_bps: 10_000,
        withdrawal_rate_bps: 0,
        early_adopter_share_bps: 2_000,
        momentum_bps: 10_000,
        milestones: Vec::new(),
        iterations: DEFAULT_ITERATIONS,
        seed: DEFAULT_SEED,
    })
}

/// Simulate early and late contributions, momentum, withdrawals, and milestone
/// releases with explicit assumptions and a reproducible seed.
pub fn simulate_campaign_with_config(
    config: &CampaignConfig,
) -> Result<CampaignResult, ModelError> {
    validate_config(config)?;
    let mut rng = Rng::new(config.seed);
    let mut successes = 0usize;
    let mut total_raised = 0u128;
    let mut total_gross = 0u128;
    let mut total_contributors = 0u128;
    let mut total_released = 0u128;

    for _ in 0..config.iterations {
        let mut raised = 0u128;
        let mut gross = 0u128;
        let mut contributors = 0u128;
        let mut released = 0u128;
        let mut released_milestones = vec![false; config.milestones.len()];

        for block in 0..config.deadline_blocks {
            let early_end = config
                .deadline_blocks
                .saturating_mul(config.early_adopter_share_bps as u64)
                / 10_000;
            let late_start = config.deadline_blocks.saturating_sub(early_end);
            let time_factor = if block < early_end {
                1.25
            } else if block >= late_start {
                1.25
            } else {
                1.0
            };
            let progress = (raised as f64 / config.goal as f64).clamp(0.0, 1.0);
            let momentum_factor =
                1.0 + (config.momentum_bps as f64 / 10_000.0) * progress;

            if rng.chance_bps(config.participation_rate_bps) {
                let random_factor = 0.5 + rng.unit();
                let amount = (config.contribution_rate as f64
                    * time_factor
                    * momentum_factor
                    * random_factor)
                    .min(u128::MAX as f64) as u128;
                if amount > 0 {
                    raised = raised.saturating_add(amount);
                    gross = gross.saturating_add(amount);
                    contributors = contributors.saturating_add(1);
                }
            }

            if raised > 0 && rng.chance_bps(config.withdrawal_rate_bps) {
                let cancelled = (raised as f64 * rng.unit()).min(raised as f64) as u128;
                raised = raised.saturating_sub(cancelled);
            }

            let funding_bps = (raised.saturating_mul(10_000) / config.goal).min(10_000) as u32;
            for (index, milestone) in config.milestones.iter().enumerate() {
                if !released_milestones[index] && funding_bps >= milestone.funding_threshold_bps {
                    let requested =
                        config.goal.saturating_mul(milestone.release_amount_bps as u128) / 10_000;
                    let available = raised.saturating_sub(released);
                    let release = requested.min(available);
                    released = released.saturating_add(release);
                    released_milestones[index] = true;
                }
            }
        }

        if raised >= config.goal {
            successes += 1;
        }
        total_raised = total_raised.saturating_add(raised);
        total_gross = total_gross.saturating_add(gross);
        total_contributors = total_contributors.saturating_add(contributors);
        total_released = total_released.saturating_add(released);
    }

    let average_contributors = total_contributors as f64 / config.iterations as f64;
    let average_contribution = if total_contributors == 0 {
        0
    } else {
        total_gross / total_contributors
    };
    Ok(CampaignResult {
        success_probability: successes as f64 / config.iterations as f64,
        expected_total_raised: total_raised / config.iterations as u128,
        expected_gross_contributions: total_gross / config.iterations as u128,
        average_contribution,
        expected_contributor_count: average_contributors,
        expected_released_funds: total_released / config.iterations as u128,
        goal: config.goal,
        deadline_blocks: config.deadline_blocks,
        iterations: config.iterations,
    })
}

/// Rerun a model across goal and deadline alternatives using common seeds.
pub fn campaign_sensitivity(
    config: &CampaignConfig,
    goals: &[u128],
    deadlines: &[u64],
) -> Result<Vec<CampaignSensitivity>, ModelError> {
    let mut results = Vec::with_capacity(goals.len().saturating_mul(deadlines.len()));
    for goal in goals {
        for deadline in deadlines {
            let mut scenario = config.clone();
            scenario.goal = *goal;
            scenario.deadline_blocks = *deadline;
            let result = simulate_campaign_with_config(&scenario)?;
            results.push(CampaignSensitivity {
                goal: *goal,
                deadline_blocks: *deadline,
                success_probability: result.success_probability,
                expected_total_raised: result.expected_total_raised,
            });
        }
    }
    Ok(results)
}

/// Compare a simulated success rate with caller-supplied historical campaigns.
/// The observations are not bundled because no verified historical dataset is
/// part of this repository.
pub fn compare_with_history(
    model: &CampaignResult,
    observations: &[CampaignObservation],
) -> Result<HistoricalComparison, ModelError> {
    if observations.is_empty() {
        return Err(ModelError::EmptyPopulation);
    }
    if observations.iter().any(|entry| entry.goal == 0) {
        return Err(ModelError::InvalidParameter(
            "historical campaign goals must be positive",
        ));
    }
    let successful = observations
        .iter()
        .filter(|entry| entry.final_raised >= entry.goal)
        .count();
    let observed_success_probability = successful as f64 / observations.len() as f64;
    Ok(HistoricalComparison {
        model_success_probability: model.success_probability,
        observed_success_probability,
        probability_gap: model.success_probability - observed_success_probability,
        observed_campaigns: observations.len(),
    })
}

fn validate_config(config: &CampaignConfig) -> Result<(), ModelError> {
    if config.goal == 0 || config.deadline_blocks == 0 {
        return Err(ModelError::InvalidParameter(
            "goal and deadline must be positive",
        ));
    }
    if config.iterations == 0 {
        return Err(ModelError::ZeroIterations);
    }
    if config.participation_rate_bps > 10_000
        || config.withdrawal_rate_bps > 10_000
        || config.early_adopter_share_bps > 10_000
        || config.momentum_bps > 100_000
    {
        return Err(ModelError::InvalidParameter(
            "campaign probabilities must be valid basis points and momentum at most 100,000 bps",
        ));
    }
    if config.milestones.iter().any(|milestone| {
        milestone.funding_threshold_bps == 0
            || milestone.funding_threshold_bps > 10_000
            || milestone.release_amount_bps > 10_000
    }) {
        return Err(ModelError::InvalidParameter(
            "milestone thresholds and releases must be valid basis points",
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config() -> CampaignConfig {
        CampaignConfig {
            goal: 100,
            deadline_blocks: 4,
            contribution_rate: 100,
            participation_rate_bps: 10_000,
            withdrawal_rate_bps: 0,
            early_adopter_share_bps: 2_000,
            momentum_bps: 10_000,
            milestones: vec![CampaignMilestone {
                funding_threshold_bps: 5_000,
                release_amount_bps: 2_500,
            }],
            iterations: 20,
            seed: 11,
        }
    }

    #[test]
    fn simulates_success_and_releases_milestone_funds() {
        let result = simulate_campaign_with_config(&config()).unwrap();
        assert_eq!(result.success_probability, 1.0);
        assert!(result.expected_released_funds > 0);
        assert!(result.average_contribution > 0);
    }

    #[test]
    fn sensitivity_and_historical_comparison_use_explicit_inputs() {
        let base = config();
        let model = simulate_campaign_with_config(&base).unwrap();
        let sensitivity = campaign_sensitivity(&base, &[100, 1_000], &[2, 4]).unwrap();
        assert_eq!(sensitivity.len(), 4);
        let comparison = compare_with_history(
            &model,
            &[CampaignObservation {
                goal: 100,
                final_raised: 90,
            }],
        )
        .unwrap();
        assert_eq!(comparison.observed_success_probability, 0.0);
    }

    #[test]
    fn rejects_zero_goal() {
        assert!(matches!(
            simulate_campaign(0, 10, 1),
            Err(ModelError::InvalidParameter(_))
        ));
    }
}
