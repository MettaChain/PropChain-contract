use crate::{rng::Rng, ModelError};

const BPS_DENOMINATOR: u128 = 10_000;
const DEFAULT_ITERATIONS: usize = 10_000;
const DEFAULT_SEED: u64 = 0x1515_0001;

/// One voter and its voting-power and behavior assumptions.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Voter {
    pub voting_power: u128,
    /// Lock-duration boost, where 10,000 is 1x.
    pub lock_multiplier_bps: u32,
    /// Index of the delegate, if this voter's power is delegated.
    pub delegate_to: Option<usize>,
    /// Probability that the voter supports the proposal.
    pub support_probability_bps: u32,
}

/// Probability that a proposal passes each tested quorum threshold.
#[derive(Clone, Debug, PartialEq)]
pub struct QuorumSensitivity {
    pub quorum_bps: u32,
    pub pass_probability: f64,
}

/// Aggregate results from a Monte Carlo governance vote.
#[derive(Clone, Debug, PartialEq)]
pub struct VoteResult {
    /// Probability of a quorum-qualified majority in favor, from 0 to 1.
    pub pass_probability: f64,
    /// Mean yes/no shares of total voting power, from 0 to 1.
    pub expected_yes_share: f64,
    pub expected_no_share: f64,
    /// Probability that participating power reaches the requested quorum.
    pub quorum_achievement_probability: f64,
    /// Largest effective voter's share of all power, in basis points.
    pub largest_voter_share_bps: u32,
    /// Share of all effective voting power originating from delegators.
    pub delegated_power_share_bps: u32,
    /// Fraction of quorum-qualified trials whose outcome changes if the
    /// largest participating voter reverses their vote.
    pub largest_voter_flip_probability: f64,
    pub iterations: usize,
    pub quorum_sensitivity: Vec<QuorumSensitivity>,
}

/// Simulate a vote using 10,000 trials and a reproducible default seed.
///
/// `participation_rate_bps` and `quorum_bps` use basis points (10,000 = 100%).
pub fn simulate_vote(
    voting_power_distribution: &[Voter],
    participation_rate_bps: u32,
    quorum_bps: u32,
) -> Result<VoteResult, ModelError> {
    simulate_vote_with_options(
        voting_power_distribution,
        participation_rate_bps,
        quorum_bps,
        DEFAULT_ITERATIONS,
        DEFAULT_SEED,
    )
}

/// Simulate votes with an explicit trial count and seed for reproducibility.
pub fn simulate_vote_with_options(
    voting_power_distribution: &[Voter],
    participation_rate_bps: u32,
    quorum_bps: u32,
    iterations: usize,
    seed: u64,
) -> Result<VoteResult, ModelError> {
    if voting_power_distribution.is_empty() {
        return Err(ModelError::EmptyPopulation);
    }
    if participation_rate_bps > 10_000 || quorum_bps > 10_000 {
        return Err(ModelError::InvalidParameter(
            "participation and quorum must be at most 10,000 bps",
        ));
    }
    if iterations == 0 {
        return Err(ModelError::ZeroIterations);
    }

    let (effective_power, delegated_power) = effective_voting_power(voting_power_distribution)?;
    let total_power = effective_power
        .iter()
        .copied()
        .fold(0u128, u128::saturating_add);
    if total_power == 0 {
        return Err(ModelError::InvalidParameter(
            "total effective voting power must be positive",
        ));
    }
    let largest_power = effective_power.iter().copied().max().unwrap_or(0);
    let largest_voter_share_bps =
        ((largest_power.saturating_mul(BPS_DENOMINATOR) / total_power).min(10_000)) as u32;
    let delegated_power_share_bps =
        ((delegated_power.saturating_mul(BPS_DENOMINATOR) / total_power).min(10_000)) as u32;

    let mut rng = Rng::new(seed);
    let mut yes_share_sum = 0.0;
    let mut no_share_sum = 0.0;
    let mut pass_count = 0usize;
    let mut quorum_count = 0usize;
    let mut largest_voter_flip_count = 0usize;
    let mut sensitivity_quorums = (0..=10).map(|step| step * 1_000).collect::<Vec<_>>();
    if !sensitivity_quorums.contains(&quorum_bps) {
        sensitivity_quorums.push(quorum_bps);
        sensitivity_quorums.sort_unstable();
    }
    let mut sensitivity_pass_counts = vec![0usize; sensitivity_quorums.len()];

    for _ in 0..iterations {
        let mut yes = 0u128;
        let mut no = 0u128;
        let mut turnout = 0u128;
        let mut largest_vote = None;

        for (index, power) in effective_power.iter().copied().enumerate() {
            if power == 0 || !rng.chance_bps(participation_rate_bps) {
                continue;
            }
            turnout = turnout.saturating_add(power);
            let supports = rng.chance_bps(voting_power_distribution[index].support_probability_bps);
            if supports {
                yes = yes.saturating_add(power);
            } else {
                no = no.saturating_add(power);
            }
            if power == largest_power && largest_vote.is_none() {
                largest_vote = Some((power, supports));
            }
        }

        let yes_share = yes as f64 / total_power as f64;
        let no_share = no as f64 / total_power as f64;
        yes_share_sum += yes_share;
        no_share_sum += no_share;
        let majority_passes = yes > no;
        let actual_quorum = turnout.saturating_mul(BPS_DENOMINATOR) / total_power;
        let passes = majority_passes && actual_quorum >= quorum_bps as u128;
        if actual_quorum >= quorum_bps as u128 {
            quorum_count += 1;
            if let Some((power, voted_yes)) = largest_vote {
                let flipped_yes = if voted_yes {
                    yes.saturating_sub(power)
                } else {
                    yes.saturating_add(power)
                };
                let flipped_no = if voted_yes {
                    no.saturating_add(power)
                } else {
                    no.saturating_sub(power)
                };
                if (yes > no) != (flipped_yes > flipped_no) {
                    largest_voter_flip_count += 1;
                }
            }
        }
        if passes {
            pass_count += 1;
        }
        for (index, sensitivity_quorum) in sensitivity_quorums.iter().copied().enumerate() {
            if majority_passes && actual_quorum >= sensitivity_quorum as u128 {
                sensitivity_pass_counts[index] += 1;
            }
        }
    }

    Ok(VoteResult {
        pass_probability: pass_count as f64 / iterations as f64,
        expected_yes_share: yes_share_sum / iterations as f64,
        expected_no_share: no_share_sum / iterations as f64,
        quorum_achievement_probability: quorum_count as f64 / iterations as f64,
        largest_voter_share_bps,
        delegated_power_share_bps,
        largest_voter_flip_probability: if quorum_count == 0 {
            0.0
        } else {
            largest_voter_flip_count as f64 / quorum_count as f64
        },
        iterations,
        quorum_sensitivity: sensitivity_quorums
            .into_iter()
            .zip(sensitivity_pass_counts)
            .map(|(quorum_bps, count)| QuorumSensitivity {
                quorum_bps,
                pass_probability: count as f64 / iterations as f64,
            })
            .collect(),
    })
}

fn effective_voting_power(voters: &[Voter]) -> Result<(Vec<u128>, u128), ModelError> {
    let mut power = voters
        .iter()
        .map(|voter| {
            voter.voting_power.saturating_mul(voter.lock_multiplier_bps as u128)
                / BPS_DENOMINATOR
        })
        .collect::<Vec<_>>();
    let mut delegated_total = 0u128;

    for (index, voter) in voters.iter().enumerate() {
        if voter.support_probability_bps > 10_000 {
            return Err(ModelError::InvalidParameter(
                "support probability must be at most 10,000 bps",
            ));
        }
        if let Some(delegate) = voter.delegate_to {
            if delegate >= voters.len() || delegate == index {
                return Err(ModelError::InvalidParameter(
                    "delegation target must be another voter in the distribution",
                ));
            }
            if voters[delegate].delegate_to.is_some() {
                return Err(ModelError::InvalidParameter(
                    "delegation chains are not supported",
                ));
            }
            let delegated = power[index];
            power[delegate] = power[delegate].saturating_add(delegated);
            delegated_total = delegated_total.saturating_add(delegated);
            power[index] = 0;
        }
    }
    Ok((power, delegated_total))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn voter(power: u128, support_probability_bps: u32) -> Voter {
        Voter {
            voting_power: power,
            lock_multiplier_bps: 10_000,
            delegate_to: None,
            support_probability_bps,
        }
    }

    #[test]
    fn applies_lock_weight_and_delegated_power() {
        let voters = [
            Voter {
                voting_power: 100,
                lock_multiplier_bps: 20_000,
                delegate_to: Some(1),
                support_probability_bps: 0,
            },
            voter(100, 10_000),
        ];
        let result = simulate_vote_with_options(&voters, 10_000, 0, 20, 7).unwrap();
        assert_eq!(result.largest_voter_share_bps, 10_000);
        assert_eq!(result.delegated_power_share_bps, 6_666);
        assert_eq!(result.pass_probability, 1.0);
    }

    #[test]
    fn quorum_changes_can_change_pass_probability() {
        let voters = [voter(100, 10_000), voter(100, 0)];
        let result = simulate_vote_with_options(&voters, 5_000, 5_000, 100, 42).unwrap();
        assert!(result.quorum_sensitivity.len() >= 11);
        assert!(result.quorum_achievement_probability < 1.0);
    }

    #[test]
    fn rejects_invalid_delegation_targets() {
        let voters = [Voter {
            delegate_to: Some(0),
            ..voter(10, 5_000)
        }];
        assert!(matches!(
            simulate_vote(&voters, 10_000, 0),
            Err(ModelError::InvalidParameter(_))
        ));
    }
}
