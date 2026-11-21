//! Economic model of cross-chain bridge security (issue #1519).
//!
//! The model answers one question a bridge operator has to be able to answer:
//! **given what validators have at stake, how much value can the bridge safely
//! custody?** Everything else here — collusion cost, attack profitability,
//! slashing, stress tests — exists to make that answer checkable.
//!
//! # Model
//!
//! A bridge is secured by `n` validators that each post `s` stake. A transfer
//! is approved by `t` signatures, so an attacker needs to control `t`
//! validators:
//!
//! ```text
//! collusion_cost      = t * s                  (capital the attacker must lock)
//! slash_penalty       = collusion_cost * σ      (σ = slash fraction, per event)
//! rewards_forgone     = collusion_cost * r * h  (r = reward APR, h = years)
//! value_at_risk       = min(max_transfer, tvl)  (one fraudulent approval)
//! expected_net_profit = (1 - d) * value_at_risk
//!                     - d * slash_penalty
//!                     - rewards_forgone
//! ```
//!
//! `d` is the probability the colluding quorum is detected and slashed before
//! it can withdraw. An attack is *profitable* only when
//! `expected_net_profit > 0`; a break-even at exactly zero is treated as
//! unprofitable, because the attacker earns nothing for the risk.
//!
//! Two quantities fall out of the above and are reported directly:
//!
//! ```text
//! break_even_value_at_risk = (rewards_forgone + d * slash_penalty) / (1 - d)
//! break_even_detection     = (value_at_risk - rewards_forgone)
//!                            / (value_at_risk + slash_penalty)
//! ```
//!
//! The first is the largest value at risk that stays unprofitable to attack
//! (the bridge's *security budget*); the second is how much detection the
//! deployment needs for its current value at risk.
//!
//! # Assumptions and scope
//!
//! * Quantities are `f64` in a single unit of account (USD, or the settlement
//!   token). No currency conversion is modelled.
//! * Rewards and slashing are linear in stake. This matches the contracts: the
//!   bridge slashes `SLASH_PERCENT` of the current stake
//!   ([`contracts/bridge/src/validator_staking.rs`](../../../contracts/bridge/src/validator_staking.rs))
//!   and staking rewards are claimed per unit of stake.
//! * `expected_net_profit` is a one-shot, risk-neutral expected value. It does
//!   not model repeated attempts, governance capture, key compromise without
//!   slashing, or MEV. Those are separate attack surfaces.
//! * [`model_bridge_security`] uses a **reference scenario** for the inputs the
//!   issue's three-argument signature cannot carry (volume, reward rate,
//!   detection probability). Those values are named constants, not measurements
//!   of a live bridge; pass [`BridgeParameters`] through
//!   [`model_bridge_security_with`] for a real deployment.
//!
//! # Example
//!
//! ```
//! use bridge_security_model::{model_bridge_security, ThresholdScheme};
//!
//! // 7 validators, 5 required signatures, 10M stake each.
//! let result = model_bridge_security(7, 5, 10_000_000.0);
//!
//! assert_eq!(result.collusion.collusion_cost, 50_000_000.0);
//! assert!(result.threshold_is_supermajority);
//! // 10M at risk, 30 % detection, 50M colluding stake: the attack loses money.
//! assert!(!result.attack.profitable);
//! assert_eq!(
//!     ThresholdScheme::TwoOfThree.required_validators(7),
//!     result.parameters.threshold
//! );
//! ```

#![deny(missing_docs)]

/// Basis points in a whole (100 % = 10 000 bps).
pub const BPS_DENOMINATOR: f64 = 10_000.0;

/// Percentage of a validator's stake removed on a proven equivocation (20 %).
///
/// Mirrors `SLASH_PERCENT` in `contracts/bridge/src/validator_staking.rs` and
/// `contracts/staking/src/types.rs`.
pub const SLASH_PERCENT: f64 = 20.0;

/// Minimum stake a validator needs to be economically active.
///
/// Mirrors `MIN_VALIDATOR_STAKE` in the bridge and staking contracts.
pub const MIN_VALIDATOR_STAKE: f64 = 10_000_000.0;

/// Smallest validator set a recommendation will propose.
///
/// Below four validators a two-thirds threshold leaves no room for a single
/// honest member to be outvoted *and* a replacement to join, so the search for
/// a recommended set starts here.
pub const MIN_RECOMMENDED_VALIDATORS: u32 = 4;

/// Upper bound on the recommendation search.
///
/// Guards against an unbounded loop when no set size can deter the attack
/// (for example, with zero stake). A deployment that needs more validators than
/// this should be modelled with a larger `stake_per_validator` instead.
pub const MAX_RECOMMENDED_VALIDATORS: u32 = 1_000;

/// Value at risk assumed by [`model_bridge_security`], in the unit of account.
pub const REFERENCE_VALUE_AT_RISK: f64 = 10_000_000.0;

/// Annual reward rate assumed by [`model_bridge_security`] (10 %).
pub const REFERENCE_REWARD_APR: f64 = 0.10;

/// Detection probability assumed by [`model_bridge_security`] (30 %).
pub const REFERENCE_DETECTION_PROBABILITY: f64 = 0.30;

/// Horizon, in days, over which rewards are forgone in the reference scenario.
pub const REFERENCE_HORIZON_DAYS: f64 = 365.0;

/// Days in a year, used to convert a day count to a fraction of a year.
const DAYS_PER_YEAR: f64 = 365.0;

/// A signature threshold expressed as a fraction of the validator set.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThresholdScheme {
    /// Two thirds of the set must sign.
    TwoOfThree,
    /// Three fifths of the set must sign.
    ThreeOfFive,
    /// An explicit `required`-of-`total` rule.
    Custom {
        /// Signatures required.
        required: u32,
        /// Size of the validator set the rule applies to.
        total: u32,
    },
}

impl ThresholdScheme {
    /// Signatures required from a set of `validator_count` validators.
    ///
    /// Fractions round up, so a set never needs fewer signatures than the
    /// fraction demands, and the result is clamped to `1..=validator_count`.
    pub fn required_validators(&self, validator_count: u32) -> u32 {
        let total = validator_count.max(1);
        let required = match *self {
            ThresholdScheme::TwoOfThree => ceil_fraction(total, 2, 3),
            ThresholdScheme::ThreeOfFive => ceil_fraction(total, 3, 5),
            ThresholdScheme::Custom { required, .. } => required,
        };
        required.clamp(1, total)
    }

    /// The threshold as a fraction of the set (0.666… for two thirds).
    pub fn fraction(&self) -> f64 {
        match *self {
            ThresholdScheme::TwoOfThree => 2.0 / 3.0,
            ThresholdScheme::ThreeOfFive => 3.0 / 5.0,
            ThresholdScheme::Custom { required, total } => {
                if total == 0 {
                    0.0
                } else {
                    f64::from(required.min(total)) / f64::from(total)
                }
            }
        }
    }

    /// Whether the threshold is a strict majority of the set.
    ///
    /// A strict majority is what stops a single minority coalition from both
    /// approving and blocking transfers. Two thirds and three fifths are both
    /// supermajorities; a bare majority is not.
    pub fn is_supermajority(&self) -> bool {
        self.fraction() > 0.5
    }

    /// Human-readable name used in reports.
    pub fn label(&self) -> &'static str {
        match *self {
            ThresholdScheme::TwoOfThree => "2-of-3",
            ThresholdScheme::ThreeOfFive => "3-of-5",
            ThresholdScheme::Custom { .. } => "custom",
        }
    }
}

/// Bridge transfer volume, from which value at risk is derived.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TransferVolume {
    /// Total value transferred across the bridge per day.
    pub daily_transfer_volume: f64,
    /// Mean size of one transfer.
    pub average_transfer: f64,
    /// Largest single transfer the bridge will approve.
    pub max_transfer: f64,
    /// Total value locked in the bridge.
    pub bridge_tvl: f64,
}

impl TransferVolume {
    /// Value a single fraudulent approval can extract.
    ///
    /// Bounded by both the per-transfer cap and the bridge's total value
    /// locked: an attacker cannot withdraw more than the bridge holds.
    pub fn value_at_risk(&self) -> f64 {
        if self.max_transfer <= 0.0 || self.bridge_tvl <= 0.0 {
            return 0.0;
        }
        self.max_transfer.min(self.bridge_tvl)
    }

    /// Transfers per day implied by the daily volume and average transfer.
    ///
    /// Returns zero when the average transfer is zero rather than dividing by
    /// it.
    pub fn transfers_per_day(&self) -> f64 {
        if self.average_transfer <= 0.0 || self.daily_transfer_volume <= 0.0 {
            return 0.0;
        }
        self.daily_transfer_volume / self.average_transfer
    }
}

impl Default for TransferVolume {
    fn default() -> Self {
        Self {
            daily_transfer_volume: 5_000_000.0,
            average_transfer: 25_000.0,
            max_transfer: REFERENCE_VALUE_AT_RISK,
            bridge_tvl: 100_000_000.0,
        }
    }
}

/// Economic parameters of a bridge deployment.
///
/// All monetary values are in one unit of account. Ratios
/// (`reward_apr`, `slash_fraction`, `detection_probability`) are fractions, so
/// a 20 % slash is `0.20`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BridgeParameters {
    /// Number of validators in the set.
    pub validator_count: u32,
    /// Signatures required to approve a transfer.
    pub threshold: u32,
    /// Stake posted by each validator.
    pub stake_per_validator: f64,
    /// Bridge transfer volume and value at risk.
    pub volume: TransferVolume,
    /// Annual reward rate earned on stake (0.10 = 10 %).
    pub reward_apr: f64,
    /// Fraction of a validator's stake removed when slashed (0.20 = 20 %).
    pub slash_fraction: f64,
    /// Probability that a colluding quorum is detected before it can withdraw.
    pub detection_probability: f64,
    /// Horizon over which colluders forgo rewards, in days.
    pub horizon_days: f64,
}

impl BridgeParameters {
    /// The reference scenario used by [`model_bridge_security`].
    ///
    /// The volume, reward, detection and horizon inputs are labelled reference
    /// values ([`REFERENCE_VALUE_AT_RISK`] and friends), not measurements of a
    /// live deployment. Override them with struct update syntax:
    ///
    /// ```
    /// use bridge_security_model::{model_bridge_security_with, BridgeParameters, TransferVolume};
    ///
    /// let params = BridgeParameters {
    ///     volume: TransferVolume { max_transfer: 250_000_000.0, bridge_tvl: 250_000_000.0, ..Default::default() },
    ///     detection_probability: 0.5,
    ///     ..BridgeParameters::reference(11, 8, 25_000_000.0)
    /// };
    /// let report = model_bridge_security_with(&params);
    /// assert!(report.attack.profitable);
    /// ```
    pub fn reference(
        validator_count: u32,
        threshold: u32,
        stake_per_validator: f64,
    ) -> Self {
        Self {
            validator_count,
            threshold,
            stake_per_validator,
            volume: TransferVolume::default(),
            reward_apr: REFERENCE_REWARD_APR,
            slash_fraction: SLASH_PERCENT / 100.0,
            detection_probability: REFERENCE_DETECTION_PROBABILITY,
            horizon_days: REFERENCE_HORIZON_DAYS,
        }
    }

    /// Value at risk implied by this deployment's volume.
    pub fn value_at_risk(&self) -> f64 {
        self.volume.value_at_risk()
    }

    /// Horizon expressed in years.
    pub fn horizon_years(&self) -> f64 {
        self.horizon_days.max(0.0) / DAYS_PER_YEAR
    }
}

/// Cost of controlling the threshold validators, and what it puts at risk.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CollusionEconomics {
    /// Validators that have to collude to reach the threshold.
    pub colluding_validators: u32,
    /// Total stake across the whole validator set.
    pub total_stake: f64,
    /// Capital an attacker must lock to control the threshold:
    /// `threshold * stake_per_validator`.
    pub collusion_cost: f64,
    /// Share of total stake the colluding quorum needs.
    pub stake_share: f64,
    /// Stake slashed from the quorum if it is detected, at the configured
    /// slash fraction.
    pub slash_penalty: f64,
    /// Rewards the quorum forgoes over the horizon.
    pub rewards_forgone: f64,
}

/// Whether attacking the bridge pays, and at what point it stops paying.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AttackProfitability {
    /// Value a single fraudulent approval can extract.
    pub value_at_risk: f64,
    /// Probability-weighted gross extraction, `(1 - d) * value_at_risk`.
    pub expected_gross_extraction: f64,
    /// Probability-weighted slashing, `d * slash_penalty`.
    pub expected_slash_penalty: f64,
    /// Rewards the quorum forgoes over the horizon.
    pub rewards_forgone: f64,
    /// Expected profit of the attack after slashing and forgone rewards.
    pub expected_net_profit: f64,
    /// Whether the expected profit is strictly positive.
    pub profitable: bool,
    /// Largest value at risk that stays unprofitable to attack.
    pub break_even_value_at_risk: f64,
    /// Detection probability at which the attack stops being profitable.
    pub break_even_detection_probability: f64,
}

/// Honest versus malicious returns for one validator.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ValidatorIncentives {
    /// Return from behaving honestly over the horizon, `stake * apr * years`.
    pub honest_return: f64,
    /// Expected return from joining the colluding quorum.
    pub malicious_return: f64,
    /// Whether honesty pays at least as well as collusion. This is the
    /// incentive-compatibility condition the bridge needs.
    pub incentive_compatible: bool,
    /// Detection probability at which collusion stops paying better than
    /// honesty.
    pub break_even_detection_probability: f64,
}

/// Outcome of a compromised-validator stress test.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StressTest {
    /// Validators assumed compromised.
    pub compromised_validators: u32,
    /// Stake held by the compromised validators.
    pub compromised_stake: f64,
    /// Validators assumed honest.
    pub honest_validators: u32,
    /// Stake held by the honest validators.
    pub honest_stake: f64,
    /// Signatures required to approve a transfer.
    pub threshold: u32,
    /// Whether the compromised set alone can approve a fraudulent transfer.
    pub compromised_can_breach: bool,
    /// Whether the honest set can still reach the threshold on its own.
    pub honest_can_still_reach_threshold: bool,
    /// Further validators the attacker still needs to reach the threshold.
    pub additional_colluders_needed: u32,
    /// Stake that would be slashed from the compromised set if detected.
    pub slash_if_detected: f64,
}

/// A validator-set recommendation derived from the security budget.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Recommendation {
    /// Smallest set size at which the attack is unprofitable, if one exists
    /// within [`MAX_RECOMMENDED_VALIDATORS`].
    pub recommended_validator_count: Option<u32>,
    /// Two-thirds threshold for the recommended set (the current threshold
    /// when no set size is recommended).
    pub recommended_threshold: u32,
    /// Per-validator stake needed to deter the attack at the current
    /// threshold. Infinite when neither slashing nor forgone rewards apply.
    pub minimum_stake_to_deter_attack: f64,
    /// Whether the attack is unprofitable at the recommendation.
    pub attack_unprofitable_at_recommendation: bool,
}

/// Complete security report for one bridge configuration.
#[derive(Debug, Clone, PartialEq)]
pub struct SecurityResult {
    /// Normalised inputs the report was computed from.
    pub parameters: BridgeParameters,
    /// Threshold as a fraction of the validator set.
    pub threshold_fraction: f64,
    /// Whether the threshold is a strict majority.
    pub threshold_is_supermajority: bool,
    /// Whether each validator's stake clears [`MIN_VALIDATOR_STAKE`].
    pub minimum_stake_met: bool,
    /// Cost of controlling the threshold validators.
    pub collusion: CollusionEconomics,
    /// Whether an attack pays.
    pub attack: AttackProfitability,
    /// Honest versus malicious returns for one validator.
    pub incentives: ValidatorIncentives,
    /// Outcome if the threshold validators are compromised.
    pub stress: StressTest,
    /// Validator-set recommendation.
    pub recommendation: Recommendation,
    /// `break_even_value_at_risk / value_at_risk`. Above 1 means the bridge
    /// has headroom; infinite when the bridge holds no value.
    pub safety_margin: f64,
    /// Inputs that were clamped or otherwise corrected before modelling.
    pub warnings: Vec<&'static str>,
}

/// Model bridge security with the reference scenario from
/// [`BridgeParameters::reference`].
///
/// Use [`model_bridge_security_with`] to supply a deployment's real volume,
/// reward rate and detection probability.
pub fn model_bridge_security(
    validator_count: u32,
    threshold: u32,
    stake_per_validator: f64,
) -> SecurityResult {
    model_bridge_security_with(&BridgeParameters::reference(
        validator_count,
        threshold,
        stake_per_validator,
    ))
}

/// Model bridge security for an explicit parameter set.
///
/// Inputs are normalised before use: the validator count is at least one, the
/// threshold is clamped to `1..=validator_count`, negative or non-finite values
/// become zero, and ratios are clamped to `0..=1`. Every correction is recorded
/// in [`SecurityResult::warnings`], so a report computed from corrected inputs
/// says so.
pub fn model_bridge_security_with(parameters: &BridgeParameters) -> SecurityResult {
    let (params, warnings) = normalise(parameters);

    let n = params.validator_count;
    let t = params.threshold;
    let stake = params.stake_per_validator;
    let years = params.horizon_years();
    let value_at_risk = params.value_at_risk();
    let detection = params.detection_probability;

    let total_stake = f64::from(n) * stake;
    let collusion_cost = f64::from(t) * stake;
    let slash_penalty = collusion_cost * params.slash_fraction;
    let rewards_forgone = collusion_cost * params.reward_apr * years;

    let expected_gross_extraction = (1.0 - detection) * value_at_risk;
    let expected_slash_penalty = detection * slash_penalty;
    let expected_net_profit =
        expected_gross_extraction - expected_slash_penalty - rewards_forgone;

    let attack = AttackProfitability {
        value_at_risk,
        expected_gross_extraction,
        expected_slash_penalty,
        rewards_forgone,
        expected_net_profit,
        profitable: expected_net_profit > 0.0,
        break_even_value_at_risk: break_even_value_at_risk(
            rewards_forgone,
            expected_slash_penalty,
            detection,
        ),
        break_even_detection_probability: break_even_detection_probability(
            value_at_risk,
            rewards_forgone,
            slash_penalty,
        ),
    };

    let incentives = ValidatorIncentives {
        honest_return: stake * params.reward_apr * years,
        malicious_return: if t == 0 {
            0.0
        } else {
            expected_net_profit / f64::from(t)
        },
        incentive_compatible: stake * params.reward_apr * years
            >= if t == 0 {
                0.0
            } else {
                expected_net_profit / f64::from(t)
            },
        break_even_detection_probability: attack.break_even_detection_probability,
    };

    let recommendation = recommend(&params, &attack);

    let safety_margin = if value_at_risk > 0.0 {
        attack.break_even_value_at_risk / value_at_risk
    } else {
        f64::INFINITY
    };

    SecurityResult {
        threshold_fraction: f64::from(t) / f64::from(n),
        threshold_is_supermajority: f64::from(t) / f64::from(n) > 0.5,
        minimum_stake_met: stake >= MIN_VALIDATOR_STAKE,
        collusion: CollusionEconomics {
            colluding_validators: t,
            total_stake,
            collusion_cost,
            stake_share: if total_stake > 0.0 {
                collusion_cost / total_stake
            } else {
                0.0
            },
            slash_penalty,
            rewards_forgone,
        },
        attack,
        incentives,
        stress: stress_test(&params, t),
        recommendation,
        safety_margin,
        warnings,
        parameters: params,
    }
}

/// Compromised-validator stress test.
///
/// `compromised_validators` is clamped to the set size. The test reports
/// whether the compromised set can approve a fraudulent transfer on its own,
/// whether the honest remainder can still reach the threshold, and how many
/// more colluders the attacker needs.
pub fn stress_test(parameters: &BridgeParameters, compromised_validators: u32) -> StressTest {
    let (params, _) = normalise(parameters);
    let n = params.validator_count;
    let t = params.threshold;
    let compromised = compromised_validators.min(n);
    let honest = n - compromised;

    StressTest {
        compromised_validators: compromised,
        compromised_stake: f64::from(compromised) * params.stake_per_validator,
        honest_validators: honest,
        honest_stake: f64::from(honest) * params.stake_per_validator,
        threshold: t,
        compromised_can_breach: compromised >= t,
        honest_can_still_reach_threshold: honest >= t,
        additional_colluders_needed: t.saturating_sub(compromised),
        slash_if_detected: f64::from(compromised)
            * params.stake_per_validator
            * params.slash_fraction,
    }
}

/// Honest versus malicious returns under the given incentives.
pub fn simulate_validator_behavior(parameters: &BridgeParameters) -> ValidatorIncentives {
    model_bridge_security_with(parameters).incentives
}

/// One row of a threshold-scheme comparison.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SchemeComparison {
    /// The scheme being evaluated.
    pub scheme: ThresholdScheme,
    /// Short name of the scheme.
    pub label: &'static str,
    /// Signatures the scheme requires from this validator set.
    pub required_validators: u32,
    /// Required signatures as a fraction of the set.
    pub threshold_fraction: f64,
    /// Capital needed to control the threshold validators.
    pub collusion_cost: f64,
    /// Largest value at risk that stays unprofitable to attack.
    pub break_even_value_at_risk: f64,
    /// Whether the attack is profitable at the modelled value at risk.
    pub attack_profitable: bool,
    /// Detection probability at which the attack stops being profitable.
    pub break_even_detection_probability: f64,
}

/// Compare 2-of-3 and 3-of-5 thresholds on the same stake, volume and horizon.
///
/// The comparison holds every economic input fixed and varies only how many
/// signatures a transfer needs. With more than 3 validators, 2-of-3 rounds up
/// more often than 3-of-5, so it is the stricter of the two: it needs more
/// colluding validators, which raises collusion cost and the security budget.
pub fn compare_threshold_schemes(parameters: &BridgeParameters) -> Vec<SchemeComparison> {
    [ThresholdScheme::TwoOfThree, ThresholdScheme::ThreeOfFive]
        .iter()
        .map(|scheme| {
            let required = scheme.required_validators(parameters.validator_count);
            let candidate = BridgeParameters {
                threshold: required,
                ..*parameters
            };
            let report = model_bridge_security_with(&candidate);
            SchemeComparison {
                scheme: *scheme,
                label: scheme.label(),
                required_validators: required,
                threshold_fraction: f64::from(required)
                    / f64::from(parameters.validator_count.max(1)),
                collusion_cost: report.collusion.collusion_cost,
                break_even_value_at_risk: report.attack.break_even_value_at_risk,
                attack_profitable: report.attack.profitable,
                break_even_detection_probability: report
                    .attack
                    .break_even_detection_probability,
            }
        })
        .collect()
}

/// Largest value at risk that leaves the attack unprofitable.
fn break_even_value_at_risk(
    rewards_forgone: f64,
    expected_slash_penalty: f64,
    detection: f64,
) -> f64 {
    if detection >= 1.0 {
        // With certain detection the expected gross extraction is zero, so any
        // value at risk is unprofitable.
        return 0.0;
    }
    (rewards_forgone + expected_slash_penalty) / (1.0 - detection)
}

/// Detection probability at which the attack stops being profitable.
fn break_even_detection_probability(
    value_at_risk: f64,
    rewards_forgone: f64,
    slash_penalty: f64,
) -> f64 {
    let denominator = value_at_risk + slash_penalty;
    if denominator <= 0.0 {
        return 1.0;
    }
    ((value_at_risk - rewards_forgone) / denominator).clamp(0.0, 1.0)
}

/// Search for the smallest validator set at which the attack is unprofitable.
fn recommend(
    params: &BridgeParameters,
    attack: &AttackProfitability,
) -> Recommendation {
    let per_stake_cost =
        params.reward_apr * params.horizon_years() + params.detection_probability * params.slash_fraction;
    let required_threshold = if per_stake_cost > 0.0 && params.stake_per_validator > 0.0 {
        (attack.value_at_risk * (1.0 - params.detection_probability)
            / (params.stake_per_validator * per_stake_cost))
            .ceil()
    } else {
        f64::INFINITY
    };

    let mut recommended = None;
    if required_threshold.is_finite() && required_threshold <= f64::from(MAX_RECOMMENDED_VALIDATORS) {
        for n in MIN_RECOMMENDED_VALIDATORS..=MAX_RECOMMENDED_VALIDATORS {
            let threshold = ThresholdScheme::TwoOfThree.required_validators(n);
            // A unanimous threshold has no liveness slack; require at least one
            // validator outside the quorum.
            if f64::from(threshold) >= required_threshold && threshold < n {
                recommended = Some(n);
                break;
            }
        }
    }

    let recommended_threshold = recommended
        .map(|n| ThresholdScheme::TwoOfThree.required_validators(n))
        .unwrap_or(params.threshold);

    let minimum_stake_to_deter_attack = if per_stake_cost > 0.0 && recommended_threshold > 0 {
        attack.value_at_risk * (1.0 - params.detection_probability)
            / (f64::from(recommended_threshold) * per_stake_cost)
    } else {
        f64::INFINITY
    };

    // Checked directly rather than by re-running the model: the model calls
    // this function to build its recommendation, so re-entering it would
    // recurse without bound.
    let attack_unprofitable_at_recommendation = recommended
        .map(|n| {
            let threshold = f64::from(ThresholdScheme::TwoOfThree.required_validators(n));
            let colluding_stake = threshold * params.stake_per_validator;
            let net = (1.0 - params.detection_probability) * attack.value_at_risk
                - params.detection_probability * colluding_stake * params.slash_fraction
                - colluding_stake * params.reward_apr * params.horizon_years();
            net <= 0.0
        })
        .unwrap_or(false);

    Recommendation {
        recommended_validator_count: recommended,
        recommended_threshold,
        minimum_stake_to_deter_attack,
        attack_unprofitable_at_recommendation,
    }
}

/// Clamp parameters into the ranges the formulas are defined for, recording
/// each correction.
fn normalise(parameters: &BridgeParameters) -> (BridgeParameters, Vec<&'static str>) {
    let mut warnings = Vec::new();

    let validator_count = if parameters.validator_count == 0 {
        warnings.push("validator_count must be at least 1; using 1");
        1
    } else {
        parameters.validator_count
    };

    let threshold = parameters.threshold.clamp(1, validator_count);
    if threshold != parameters.threshold {
        warnings.push("threshold clamped to 1..=validator_count");
    }

    let stake_per_validator = non_negative(
        parameters.stake_per_validator,
        "stake_per_validator",
        &mut warnings,
    );
    let reward_apr = non_negative(parameters.reward_apr, "reward_apr", &mut warnings);
    let detection_probability = ratio(
        parameters.detection_probability,
        "detection_probability",
        &mut warnings,
    );
    let slash_fraction = ratio(
        parameters.slash_fraction,
        "slash_fraction",
        &mut warnings,
    );
    let horizon_days = non_negative(parameters.horizon_days, "horizon_days", &mut warnings);

    let volume = TransferVolume {
        daily_transfer_volume: non_negative(
            parameters.volume.daily_transfer_volume,
            "daily_transfer_volume",
            &mut warnings,
        ),
        average_transfer: non_negative(
            parameters.volume.average_transfer,
            "average_transfer",
            &mut warnings,
        ),
        max_transfer: non_negative(parameters.volume.max_transfer, "max_transfer", &mut warnings),
        bridge_tvl: non_negative(parameters.volume.bridge_tvl, "bridge_tvl", &mut warnings),
    };

    (
        BridgeParameters {
            validator_count,
            threshold,
            stake_per_validator,
            volume,
            reward_apr,
            slash_fraction,
            detection_probability,
            horizon_days,
        },
        warnings,
    )
}

/// Replace a negative or non-finite value with zero, recording it.
fn non_negative(value: f64, name: &'static str, warnings: &mut Vec<&'static str>) -> f64 {
    if !value.is_finite() {
        warnings.push(match name {
            "stake_per_validator" => "stake_per_validator was not finite; using 0",
            "reward_apr" => "reward_apr was not finite; using 0",
            "horizon_days" => "horizon_days was not finite; using 0",
            "daily_transfer_volume" => "daily_transfer_volume was not finite; using 0",
            "average_transfer" => "average_transfer was not finite; using 0",
            "max_transfer" => "max_transfer was not finite; using 0",
            "bridge_tvl" => "bridge_tvl was not finite; using 0",
            _ => "a monetary input was not finite; using 0",
        });
        return 0.0;
    }
    if value < 0.0 {
        warnings.push(match name {
            "stake_per_validator" => "stake_per_validator was negative; using 0",
            "reward_apr" => "reward_apr was negative; using 0",
            "horizon_days" => "horizon_days was negative; using 0",
            "daily_transfer_volume" => "daily_transfer_volume was negative; using 0",
            "average_transfer" => "average_transfer was negative; using 0",
            "max_transfer" => "max_transfer was negative; using 0",
            "bridge_tvl" => "bridge_tvl was negative; using 0",
            _ => "a monetary input was negative; using 0",
        });
        return 0.0;
    }
    value
}

/// Clamp a value into `0..=1`, replacing non-finite values with zero.
fn ratio(value: f64, name: &'static str, warnings: &mut Vec<&'static str>) -> f64 {
    if !value.is_finite() {
        warnings.push(match name {
            "detection_probability" => "detection_probability was not finite; using 0",
            _ => "slash_fraction was not finite; using 0",
        });
        return 0.0;
    }
    let clamped = value.clamp(0.0, 1.0);
    if (clamped - value).abs() > f64::EPSILON {
        warnings.push(match name {
            "detection_probability" => "detection_probability clamped to 0..=1",
            _ => "slash_fraction clamped to 0..=1",
        });
    }
    clamped
}

/// Ceiling of `count * numerator / denominator` for positive integers.
fn ceil_fraction(count: u32, numerator: u32, denominator: u32) -> u32 {
    count
        .saturating_mul(numerator)
        .saturating_add(denominator - 1)
        / denominator
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Compare floats within a relative tolerance appropriate for money.
    fn assert_close(actual: f64, expected: f64) {
        let tolerance = expected.abs().max(1.0) * 1e-9;
        assert!(
            (actual - expected).abs() <= tolerance,
            "expected {expected}, got {actual}"
        );
    }

    #[test]
    fn threshold_fractions_round_up() {
        assert_eq!(ThresholdScheme::TwoOfThree.required_validators(4), 3);
        assert_eq!(ThresholdScheme::TwoOfThree.required_validators(6), 4);
        assert_eq!(ThresholdScheme::TwoOfThree.required_validators(7), 5);
        assert_eq!(ThresholdScheme::ThreeOfFive.required_validators(5), 3);
        assert_eq!(ThresholdScheme::ThreeOfFive.required_validators(10), 6);
        // A one-validator set still needs the one signature it has.
        assert_eq!(ThresholdScheme::TwoOfThree.required_validators(0), 1);
    }

    #[test]
    fn collusion_cost_and_slashing_follow_the_bridge_constants() {
        let result = model_bridge_security(7, 5, 10_000_000.0);

        assert_close(result.collusion.collusion_cost, 50_000_000.0);
        assert_close(result.collusion.total_stake, 70_000_000.0);
        assert_close(result.collusion.stake_share, 50.0 / 70.0);
        // 20 % of 50M, matching SLASH_PERCENT in the bridge contract.
        assert_close(result.collusion.slash_penalty, 10_000_000.0);
        // One year of 10 % rewards on the colluding stake.
        assert_close(result.collusion.rewards_forgone, 5_000_000.0);
        assert!(result.minimum_stake_met);
        assert_close(SLASH_PERCENT, 20.0);
    }

    #[test]
    fn reference_scenario_attack_loses_money_at_ten_million() {
        let result = model_bridge_security(7, 5, 10_000_000.0);

        // (1 - 0.3) * 10M = 7M; 0.3 * 10M = 3M; forgone = 5M.
        assert_close(result.attack.expected_gross_extraction, 7_000_000.0);
        assert_close(result.attack.expected_slash_penalty, 3_000_000.0);
        assert_close(result.attack.expected_net_profit, -1_000_000.0);
        assert!(!result.attack.profitable);
        // Break-even value at risk: (5M + 3M) / 0.7.
        assert_close(result.attack.break_even_value_at_risk, 11_428_571.428_571_428);
        // Break-even detection: (10M - 5M) / (10M + 10M) = 0.25.
        assert_close(result.attack.break_even_detection_probability, 0.25);
        assert_close(result.safety_margin, 1.142_857_142_857_142_9);
    }

    #[test]
    fn higher_value_at_risk_makes_the_attack_profitable() {
        let params = BridgeParameters {
            volume: TransferVolume {
                max_transfer: 100_000_000.0,
                bridge_tvl: 100_000_000.0,
                ..Default::default()
            },
            ..BridgeParameters::reference(7, 5, 10_000_000.0)
        };
        let result = model_bridge_security_with(&params);

        assert_close(result.attack.value_at_risk, 100_000_000.0);
        assert!(result.attack.profitable);
        assert!(result.attack.expected_net_profit > 0.0);
        assert!(result.safety_margin < 1.0);
    }

    #[test]
    fn value_at_risk_is_capped_by_the_tvl() {
        let volume = TransferVolume {
            max_transfer: 900_000_000.0,
            bridge_tvl: 25_000_000.0,
            daily_transfer_volume: 50_000_000.0,
            average_transfer: 25_000.0,
        };
        assert_close(volume.value_at_risk(), 25_000_000.0);
        assert_close(volume.transfers_per_day(), 2_000.0);
    }

    #[test]
    fn certain_detection_removes_expected_profit() {
        let params = BridgeParameters {
            detection_probability: 1.0,
            ..BridgeParameters::reference(7, 5, 10_000_000.0)
        };
        let result = model_bridge_security_with(&params);

        assert_close(result.attack.expected_gross_extraction, 0.0);
        assert!(result.attack.expected_net_profit < 0.0);
        assert!(!result.attack.profitable);
        assert_close(result.attack.break_even_value_at_risk, 0.0);
    }

    #[test]
    fn detection_above_break_even_flips_the_incentive() {
        let below = model_bridge_security_with(&BridgeParameters {
            detection_probability: 0.2,
            volume: TransferVolume {
                max_transfer: 50_000_000.0,
                bridge_tvl: 50_000_000.0,
                ..Default::default()
            },
            ..BridgeParameters::reference(7, 5, 10_000_000.0)
        });
        let above = model_bridge_security_with(&BridgeParameters {
            detection_probability: 0.9,
            volume: TransferVolume {
                max_transfer: 50_000_000.0,
                bridge_tvl: 50_000_000.0,
                ..Default::default()
            },
            ..BridgeParameters::reference(7, 5, 10_000_000.0)
        });

        assert!(below.attack.break_even_detection_probability > 0.2);
        assert!(above.attack.break_even_detection_probability < 0.9);
        assert!(below.attack.expected_net_profit > above.attack.expected_net_profit);
        assert!(below.attack.profitable);
        assert!(!above.attack.profitable);
    }

    #[test]
    fn honest_and_malicious_returns_are_compared() {
        let params = BridgeParameters::reference(7, 5, 10_000_000.0);
        let incentives = simulate_validator_behavior(&params);

        // Honest: 10M * 10 % * 1 year = 1M.
        assert_close(incentives.honest_return, 1_000_000.0);
        // Malicious: -1M expected profit split across 5 colluders.
        assert_close(incentives.malicious_return, -200_000.0);
        assert!(incentives.incentive_compatible);
    }

    #[test]
    fn stress_test_reports_breach_and_honest_shortfall() {
        let params = BridgeParameters::reference(7, 5, 10_000_000.0);

        let breached = stress_test(&params, 5);
        assert!(breached.compromised_can_breach);
        assert!(!breached.honest_can_still_reach_threshold);
        assert_eq!(breached.honest_validators, 2);
        assert_close(breached.compromised_stake, 50_000_000.0);
        assert_close(breached.slash_if_detected, 10_000_000.0);
        assert_eq!(breached.additional_colluders_needed, 0);

        let partial = stress_test(&params, 3);
        assert!(!partial.compromised_can_breach);
        assert!(!partial.honest_can_still_reach_threshold);
        assert_eq!(partial.honest_validators, 4);
        assert_eq!(partial.additional_colluders_needed, 2);

        let harmless = stress_test(&params, 0);
        assert!(!harmless.compromised_can_breach);
        assert!(harmless.honest_can_still_reach_threshold);
        assert_eq!(harmless.additional_colluders_needed, 5);
    }

    #[test]
    fn compromise_is_clamped_to_the_validator_set() {
        let params = BridgeParameters::reference(5, 4, 10_000_000.0);
        let stressed = stress_test(&params, 99);

        assert_eq!(stressed.compromised_validators, 5);
        assert_eq!(stressed.honest_validators, 0);
        assert!(!stressed.honest_can_still_reach_threshold);
    }

    #[test]
    fn two_thirds_is_stricter_than_three_fifths() {
        let params = BridgeParameters::reference(10, 7, 10_000_000.0);
        let rows = compare_threshold_schemes(&params);

        assert_eq!(rows.len(), 2);
        let two_thirds = rows[0];
        let three_fifths = rows[1];

        assert_eq!(two_thirds.label, "2-of-3");
        assert_eq!(two_thirds.required_validators, 7);
        assert_eq!(three_fifths.required_validators, 6);
        assert!(two_thirds.collusion_cost > three_fifths.collusion_cost);
        assert!(two_thirds.break_even_value_at_risk > three_fifths.break_even_value_at_risk);
        assert!(two_thirds.break_even_detection_probability < three_fifths.break_even_detection_probability);
    }

    #[test]
    fn recommendation_scales_with_the_value_at_risk() {
        let small = model_bridge_security(7, 5, 10_000_000.0);
        // 7M of expected extraction needs 4.375 signatures' worth of stake at
        // 10M each, so the smallest two-thirds set that covers it is 7.
        assert_eq!(small.recommendation.recommended_validator_count, Some(7));
        assert!(small.recommendation.attack_unprofitable_at_recommendation);

        let large = model_bridge_security_with(&BridgeParameters {
            volume: TransferVolume {
                max_transfer: 1_000_000_000.0,
                bridge_tvl: 1_000_000_000.0,
                ..Default::default()
            },
            ..BridgeParameters::reference(7, 5, 10_000_000.0)
        });
        let recommended = large
            .recommendation
            .recommended_validator_count
            .expect("a very large value at risk needs more validators");
        assert!(recommended > 4);
        assert!(large.recommendation.attack_unprofitable_at_recommendation);
    }

    #[test]
    fn recommendation_is_absent_when_no_set_size_can_deter() {
        let result = model_bridge_security(7, 5, 1.0);
        assert_eq!(result.recommendation.recommended_validator_count, None);
    }

    #[test]
    fn minimum_stake_to_deter_is_reported() {
        let result = model_bridge_security(7, 5, 10_000_000.0);
        // 10M * 0.7 / (5 * (0.1 + 0.3 * 0.2)) = 7M / 0.8 = 8.75M.
        assert_close(
            result.recommendation.minimum_stake_to_deter_attack,
            8_750_000.0,
        );
    }

    #[test]
    fn invalid_inputs_are_normalised_and_flagged() {
        let result = model_bridge_security(0, 99, -1.0);

        assert_eq!(result.parameters.validator_count, 1);
        assert_eq!(result.parameters.threshold, 1);
        assert_close(result.parameters.stake_per_validator, 0.0);
        assert!(!result.warnings.is_empty());
        assert!(!result.minimum_stake_met);

        for value in [
            result.collusion.collusion_cost,
            result.collusion.total_stake,
            result.collusion.stake_share,
            result.collusion.slash_penalty,
            result.collusion.rewards_forgone,
            result.attack.value_at_risk,
            result.attack.expected_net_profit,
            result.attack.break_even_value_at_risk,
            result.attack.break_even_detection_probability,
            result.threshold_fraction,
            result.safety_margin,
        ] {
            assert!(value.is_finite(), "model produced a non-finite number");
        }
    }

    #[test]
    fn non_finite_inputs_do_not_poison_the_report() {
        let params = BridgeParameters {
            reward_apr: f64::NAN,
            detection_probability: f64::INFINITY,
            ..BridgeParameters::reference(7, 5, 10_000_000.0)
        };
        let result = model_bridge_security_with(&params);

        assert_close(result.parameters.reward_apr, 0.0);
        assert_close(result.parameters.detection_probability, 0.0);
        assert!(result.attack.expected_net_profit.is_finite());
    }

    #[test]
    fn zero_stake_cannot_deter_an_attack() {
        let result = model_bridge_security(7, 5, 0.0);

        assert_close(result.collusion.collusion_cost, 0.0);
        assert_close(result.collusion.slash_penalty, 0.0);
        assert!(result.attack.profitable);
        assert_eq!(result.recommendation.recommended_validator_count, None);
        assert_close(
            result.recommendation.minimum_stake_to_deter_attack,
            8_750_000.0,
        );
    }

    #[test]
    fn custom_threshold_reports_its_fraction() {
        let scheme = ThresholdScheme::Custom {
            required: 2,
            total: 3,
        };
        assert_close(scheme.fraction(), 2.0 / 3.0);
        assert!(scheme.is_supermajority());
        assert_eq!(scheme.required_validators(11), 2);
        assert_eq!(ThresholdScheme::ThreeOfFive.fraction(), 0.6);
    }

    #[test]
    fn uniform_threshold_is_a_supermajority_without_slack() {
        assert!(ThresholdScheme::TwoOfThree.is_supermajority());
        assert!(ThresholdScheme::ThreeOfFive.is_supermajority());
        let params = BridgeParameters::reference(5, 5, 10_000_000.0);
        let result = model_bridge_security_with(&params);
        assert!(result.threshold_is_supermajority);
        assert_close(result.threshold_fraction, 1.0);
        assert_eq!(result.parameters.threshold, 5);
    }

    #[test]
    fn empty_volume_has_no_value_at_risk() {
        let params = BridgeParameters {
            volume: TransferVolume {
                daily_transfer_volume: 0.0,
                average_transfer: 0.0,
                max_transfer: 0.0,
                bridge_tvl: 0.0,
            },
            ..BridgeParameters::reference(7, 5, 10_000_000.0)
        };
        let result = model_bridge_security_with(&params);

        assert_close(result.attack.value_at_risk, 0.0);
        assert!(!result.attack.profitable);
        assert!(result.safety_margin.is_infinite());
    }

    #[test]
    fn model_is_deterministic() {
        let first = model_bridge_security(9, 6, 12_500_000.0);
        let second = model_bridge_security(9, 6, 12_500_000.0);
        assert_eq!(first, second);
    }
}
