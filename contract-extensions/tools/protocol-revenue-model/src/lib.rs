//! Protocol fee revenue and treasury projection (issue #1520).
//!
//! The model takes fee-generating volume and projects what the protocol
//! collects, how that fee is split, and what the treasury looks like after it
//! has compounded reserves and paid operating costs.
//!
//! # Model
//!
//! Volume grows by a daily compounded rate `g`:
//!
//! ```text
//! volume(day)  = daily_volume * (1 + g)^day
//! revenue(day) = volume(day) * fee_bps / 10_000
//! ```
//!
//! Each day's fee is split by basis points between the treasury, liquidity
//! providers, stakers and a burn. The treasury then:
//!
//! ```text
//! balance(day) = balance(day - 1)
//!              + balance(day - 1) * reserve_apr / 365   (when balance > 0)
//!              + revenue(day) * treasury_share
//!              - operating_cost_per_day
//! ```
//!
//! # Fee types
//!
//! DEX swaps, lending interest and other protocol fees are modelled as separate
//! [`FeeStream`]s, each with its own volume and fee rate. [`project_fee_revenue`]
//! wraps a single stream; use [`RevenueModel`] with several streams to model a
//! real fee mix.
//!
//! # Scope and limits
//!
//! * Quantities are `f64` in one unit of account (USD). No FX conversion.
//! * The projection is deterministic given its inputs; it is not a forecast.
//!   Growth rates are assumptions supplied by the caller
//!   ([`VolumeScenario`] offers three labelled ones), not fitted parameters.
//! * Token-price effects, emissions, and incentive spend are out of scope.
//!   `operating_cost_per_day` covers cash operating costs only.
//! * Inputs are normalised: negative or non-finite amounts become zero, fee
//!   rates are clamped to `0..=10 000` bps, and a distribution that does not sum
//!   to 10 000 bps is rescaled. Every correction is recorded in
//!   [`RevenueProjection::warnings`].
//!
//! # Example
//!
//! ```
//! use protocol_revenue_model::project_fee_revenue;
//!
//! // 1M/day of volume at 30 bps, flat, for 30 days.
//! let projection = project_fee_revenue(1_000_000.0, 30.0, 0.0, 30);
//!
//! assert_eq!(projection.total_fee_revenue, 90_000.0);
//! assert_eq!(projection.daily_fee_revenue.len(), 30);
//! assert_eq!(projection.monthly_fee_revenue.len(), 1);
//! // The reference policy sends half of the fee to the treasury.
//! assert_eq!(projection.distribution.treasury, 45_000.0);
//! ```

#![deny(missing_docs)]

/// Basis points in a whole (100 % = 10 000 bps).
pub const BPS_DENOMINATOR: f64 = 10_000.0;

/// Days per year, used to annualise a projection.
pub const DAYS_PER_YEAR: f64 = 365.0;

/// Days per month used for runway arithmetic (365.25 / 12).
pub const DAYS_PER_MONTH: f64 = 30.4375;

/// Longest projection the model will build, in days (ten years).
///
/// Bounds the allocation a single call can make.
pub const MAX_PROJECTION_DAYS: u32 = 3_650;

/// Days in one reporting month.
const MONTH_DAYS: usize = 30;

/// One source of protocol fees.
///
/// A DEX swap fee, a share of lending interest, and a flat registry fee are all
/// streams: what differs is the volume they are charged on and the rate.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FeeStream {
    /// Human-readable name used in the projection.
    pub label: &'static str,
    /// Volume the fee is charged on, per day, on day zero.
    pub daily_volume: f64,
    /// Fee in basis points (30 = 0.30 %).
    pub fee_bps: f64,
    /// Daily compounded volume growth (0.002 = 0.2 % per day).
    pub daily_growth_rate: f64,
}

impl FeeStream {
    /// Build a stream. Values are normalised when a [`RevenueModel`] projects
    /// them; this constructor stores them as given.
    pub fn new(
        label: &'static str,
        daily_volume: f64,
        fee_bps: f64,
        daily_growth_rate: f64,
    ) -> Self {
        Self {
            label,
            daily_volume,
            fee_bps,
            daily_growth_rate,
        }
    }

    /// Fee stream for DEX swaps.
    pub fn dex_swaps(daily_volume: f64, fee_bps: f64, daily_growth_rate: f64) -> Self {
        Self::new("DEX swaps", daily_volume, fee_bps, daily_growth_rate)
    }

    /// Fee stream for the protocol's share of lending interest.
    pub fn lending_interest(daily_volume: f64, fee_bps: f64, daily_growth_rate: f64) -> Self {
        Self::new(
            "lending interest",
            daily_volume,
            fee_bps,
            daily_growth_rate,
        )
    }

    /// Fee stream for other protocol fees.
    pub fn other(daily_volume: f64, fee_bps: f64, daily_growth_rate: f64) -> Self {
        Self::new(
            "other protocol fees",
            daily_volume,
            fee_bps,
            daily_growth_rate,
        )
    }

    /// Volume on `day` (zero-based), with the growth rate compounded daily.
    pub fn volume_at(&self, day: u32) -> f64 {
        let volume = if self.daily_volume.is_finite() {
            self.daily_volume.max(0.0)
        } else {
            0.0
        };
        let growth = if self.daily_growth_rate.is_finite() {
            self.daily_growth_rate.max(-1.0)
        } else {
            0.0
        };
        volume * (1.0 + growth).powf(f64::from(day))
    }

    /// Fee collected on `day` (zero-based).
    pub fn revenue_at(&self, day: u32) -> f64 {
        let fee_bps = if self.fee_bps.is_finite() {
            self.fee_bps.clamp(0.0, BPS_DENOMINATOR)
        } else {
            0.0
        };
        self.volume_at(day) * fee_bps / BPS_DENOMINATOR
    }
}

/// How collected fees are split, in basis points.
///
/// The four shares must sum to 10 000 bps. [`Default`] is the reference policy:
/// 50 % treasury, 25 % liquidity providers, 15 % stakers, 10 % burned.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FeeDistribution {
    /// Share retained by the treasury.
    pub treasury_bps: f64,
    /// Share paid to liquidity providers.
    pub lp_bps: f64,
    /// Share paid to stakers.
    pub staker_bps: f64,
    /// Share bought back and burned.
    pub burn_bps: f64,
}

impl Default for FeeDistribution {
    fn default() -> Self {
        Self {
            treasury_bps: 5_000.0,
            lp_bps: 2_500.0,
            staker_bps: 1_500.0,
            burn_bps: 1_000.0,
        }
    }
}

/// Why a [`FeeDistribution`] cannot be used as given.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DistributionError {
    /// A share is negative or not finite.
    NonFiniteOrNegativeShare,
    /// The shares do not sum to 10 000 basis points.
    DoesNotSumToWhole {
        /// The sum that was found, in basis points.
        total_bps: f64,
    },
}

impl FeeDistribution {
    /// Sum of the four shares, in basis points.
    pub fn total_bps(&self) -> f64 {
        self.treasury_bps + self.lp_bps + self.staker_bps + self.burn_bps
    }

    /// Fraction of fees the treasury retains (0.5 for 5 000 bps).
    pub fn treasury_share(&self) -> f64 {
        self.treasury_bps / BPS_DENOMINATOR
    }

    /// Check that every share is usable and the four sum to 10 000 bps.
    pub fn validate(&self) -> Result<(), DistributionError> {
        for share in [self.treasury_bps, self.lp_bps, self.staker_bps, self.burn_bps] {
            if !share.is_finite() || share < 0.0 {
                return Err(DistributionError::NonFiniteOrNegativeShare);
            }
        }
        let total = self.total_bps();
        if (total - BPS_DENOMINATOR).abs() > 1e-6 {
            return Err(DistributionError::DoesNotSumToWhole { total_bps: total });
        }
        Ok(())
    }
}

/// Treasury policy: starting balance, reserve yield and operating costs.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TreasuryPolicy {
    /// Balance at the start of the projection.
    pub initial_balance: f64,
    /// Annual yield earned on positive reserves (0.05 = 5 %).
    pub reserve_yield_apr: f64,
    /// Cash operating cost paid out of the treasury each day.
    pub operating_cost_per_day: f64,
}

impl Default for TreasuryPolicy {
    fn default() -> Self {
        Self {
            initial_balance: 0.0,
            reserve_yield_apr: 0.05,
            operating_cost_per_day: 0.0,
        }
    }
}

/// Totals paid to each recipient over the projection.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DistributionTotals {
    /// Fees retained by the treasury.
    pub treasury: f64,
    /// Fees paid to liquidity providers.
    pub liquidity_providers: f64,
    /// Fees paid to stakers.
    pub stakers: f64,
    /// Fees bought back and burned.
    pub burned: f64,
}

impl DistributionTotals {
    /// Sum of the four destinations.
    pub fn total(&self) -> f64 {
        self.treasury + self.liquidity_providers + self.stakers + self.burned
    }
}

/// Per-stream result within a projection.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StreamProjection {
    /// Stream name.
    pub label: &'static str,
    /// Fee rate used, in basis points.
    pub fee_bps: f64,
    /// Volume on day zero.
    pub initial_daily_volume: f64,
    /// Volume on the last projected day.
    pub final_daily_volume: f64,
    /// Fee collected on the last projected day.
    pub final_daily_revenue: f64,
    /// Fee collected over the whole projection.
    pub total_revenue: f64,
}

/// Treasury balance path and the flows that produced it.
#[derive(Debug, Clone, PartialEq)]
pub struct TreasuryProjection {
    /// Balance at the start of the projection.
    pub initial_balance: f64,
    /// Balance on the last projected day. Negative means the treasury was
    /// overdrawn during the projection.
    pub closing_balance: f64,
    /// Treasury fees received over the projection.
    pub total_inflow: f64,
    /// Operating costs paid over the projection.
    pub total_operating_cost: f64,
    /// Yield earned on reserves over the projection.
    pub total_reserve_yield: f64,
    /// Balance at the end of each projected day.
    pub daily_balance: Vec<f64>,
    /// Lowest balance reached, including the starting balance.
    pub min_balance: f64,
    /// Average daily treasury inflow over the projection.
    pub average_daily_inflow: f64,
    /// Operating cost per day, copied from the policy.
    pub operating_cost_per_day: f64,
    /// `operating_cost_per_day - average_daily_inflow`. Positive means the
    /// treasury shrinks on average.
    pub net_burn_rate_per_day: f64,
    /// First day (one-based) on which the balance went negative.
    pub first_depleted_day: Option<u32>,
    /// Days until the treasury is exhausted. `None` means the projected run-rate
    /// covers operating costs indefinitely.
    pub runway_days: Option<f64>,
}

/// A complete revenue and treasury projection.
#[derive(Debug, Clone, PartialEq)]
pub struct RevenueProjection {
    /// Number of days projected.
    pub days: u32,
    /// Per-stream breakdown.
    pub streams: Vec<StreamProjection>,
    /// Fee collected on each projected day.
    pub daily_fee_revenue: Vec<f64>,
    /// Fee collected in each 30-day month (the last month may be partial).
    pub monthly_fee_revenue: Vec<f64>,
    /// Fee collected over the whole projection.
    pub total_fee_revenue: f64,
    /// Mean daily fee over the projection.
    pub average_daily_fee_revenue: f64,
    /// Fee collected on day zero.
    pub first_day_fee_revenue: f64,
    /// Fee collected over the last 30 days (or the whole projection if shorter).
    pub trailing_30_day_revenue: f64,
    /// Trailing 30-day revenue annualised to 365 days.
    pub annualized_revenue: f64,
    /// Annualised revenue retained by the treasury.
    pub annualized_protocol_revenue: f64,
    /// Split of the total fee between recipients.
    pub distribution: DistributionTotals,
    /// Treasury balance path.
    pub treasury: TreasuryProjection,
    /// Inputs that were clamped, rescaled or otherwise corrected.
    pub warnings: Vec<&'static str>,
}

/// A full model: several fee streams, a distribution policy and a treasury
/// policy over a projection horizon.
#[derive(Debug, Clone, PartialEq)]
pub struct RevenueModel {
    /// Fee-generating streams. All are charged over the same horizon.
    pub streams: Vec<FeeStream>,
    /// How collected fees are split.
    pub distribution: FeeDistribution,
    /// Treasury starting balance, reserve yield and operating costs.
    pub treasury: TreasuryPolicy,
    /// Days to project.
    pub days: u32,
}

impl RevenueModel {
    /// Build a model. Inputs are normalised by [`RevenueModel::project`].
    pub fn new(
        streams: Vec<FeeStream>,
        distribution: FeeDistribution,
        treasury: TreasuryPolicy,
        days: u32,
    ) -> Self {
        Self {
            streams,
            distribution,
            treasury,
            days,
        }
    }

    /// Project revenue and the treasury path.
    pub fn project(&self) -> RevenueProjection {
        let mut warnings = Vec::new();

        let days = if self.days > MAX_PROJECTION_DAYS {
            warnings.push("days clamped to MAX_PROJECTION_DAYS");
            MAX_PROJECTION_DAYS
        } else {
            self.days
        };

        let distribution = normalise_distribution(self.distribution, &mut warnings);
        let policy = normalise_treasury(self.treasury, &mut warnings);
        let streams = self
            .streams
            .iter()
            .map(|stream| normalise_stream(*stream, &mut warnings))
            .collect::<Vec<_>>();

        let mut daily_fee_revenue = Vec::with_capacity(days as usize);
        let mut stream_totals = vec![0.0_f64; streams.len()];

        for day in 0..days {
            let mut day_total = 0.0;
            for (index, stream) in streams.iter().enumerate() {
                let revenue = stream.revenue_at(day);
                stream_totals[index] += revenue;
                day_total += revenue;
            }
            daily_fee_revenue.push(day_total);
        }

        let total_fee_revenue: f64 = daily_fee_revenue.iter().sum();
        let average_daily_fee_revenue = if days == 0 {
            0.0
        } else {
            total_fee_revenue / f64::from(days)
        };

        let monthly_fee_revenue = daily_fee_revenue
            .chunks(MONTH_DAYS)
            .map(|month| month.iter().sum())
            .collect::<Vec<f64>>();

        let trailing_window = days.min(MONTH_DAYS as u32) as usize;
        let trailing_30_day_revenue: f64 = daily_fee_revenue
            .iter()
            .rev()
            .take(trailing_window)
            .sum();
        let annualized_revenue = if trailing_window == 0 {
            0.0
        } else {
            trailing_30_day_revenue / trailing_window as f64 * DAYS_PER_YEAR
        };

        let distribution_totals = DistributionTotals {
            treasury: total_fee_revenue * distribution.treasury_bps / BPS_DENOMINATOR,
            liquidity_providers: total_fee_revenue * distribution.lp_bps / BPS_DENOMINATOR,
            stakers: total_fee_revenue * distribution.staker_bps / BPS_DENOMINATOR,
            burned: total_fee_revenue * distribution.burn_bps / BPS_DENOMINATOR,
        };

        let treasury = project_treasury(
            &daily_fee_revenue,
            distribution.treasury_share(),
            &policy,
        );

        let streams = streams
            .iter()
            .zip(stream_totals)
            .map(|(stream, total_revenue)| StreamProjection {
                label: stream.label,
                fee_bps: stream.fee_bps,
                initial_daily_volume: stream.volume_at(0),
                final_daily_volume: if days == 0 {
                    stream.volume_at(0)
                } else {
                    stream.volume_at(days - 1)
                },
                final_daily_revenue: if days == 0 {
                    0.0
                } else {
                    stream.revenue_at(days - 1)
                },
                total_revenue,
            })
            .collect();

        let first_day_fee_revenue = daily_fee_revenue.first().copied().unwrap_or(0.0);

        RevenueProjection {
            days,
            streams,
            daily_fee_revenue,
            monthly_fee_revenue,
            total_fee_revenue,
            average_daily_fee_revenue,
            first_day_fee_revenue,
            trailing_30_day_revenue,
            annualized_revenue,
            annualized_protocol_revenue: annualized_revenue * distribution.treasury_share(),
            distribution: distribution_totals,
            treasury,
            warnings,
        }
    }
}

/// Project a single fee stream with the reference distribution and treasury
/// policy.
///
/// `daily_volume` is charged `fee_bps` and grows at `growth_rate` per day,
/// compounded daily, for `days` days.
pub fn project_fee_revenue(
    daily_volume: f64,
    fee_bps: f64,
    growth_rate: f64,
    days: u32,
) -> RevenueProjection {
    RevenueModel::new(
        vec![FeeStream::new(
            "protocol fees",
            daily_volume,
            fee_bps,
            growth_rate,
        )],
        FeeDistribution::default(),
        TreasuryPolicy::default(),
        days,
    )
    .project()
}

/// A labelled volume-growth path.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VolumeScenario {
    /// Flat volume: no growth.
    Conservative,
    /// 0.2 % growth per day, compounded daily.
    Moderate,
    /// 0.5 % growth per day, compounded daily.
    Aggressive,
}

impl VolumeScenario {
    /// Daily compounded growth rate for the scenario.
    pub fn daily_growth_rate(self) -> f64 {
        match self {
            VolumeScenario::Conservative => 0.0,
            VolumeScenario::Moderate => 0.002,
            VolumeScenario::Aggressive => 0.005,
        }
    }

    /// Short scenario name.
    pub fn label(self) -> &'static str {
        match self {
            VolumeScenario::Conservative => "conservative",
            VolumeScenario::Moderate => "moderate",
            VolumeScenario::Aggressive => "aggressive",
        }
    }
}

/// One scenario's projection.
#[derive(Debug, Clone, PartialEq)]
pub struct ScenarioProjection {
    /// The scenario applied.
    pub scenario: VolumeScenario,
    /// Short scenario name.
    pub label: &'static str,
    /// Projection under that scenario.
    pub projection: RevenueProjection,
}

/// Project the same fee stream under the conservative, moderate and aggressive
/// volume paths.
///
/// The three paths hold everything except the volume growth rate fixed. The
/// rates are assumptions for comparison, not forecasts.
pub fn compare_volume_scenarios(
    daily_volume: f64,
    fee_bps: f64,
    days: u32,
) -> Vec<ScenarioProjection> {
    [
        VolumeScenario::Conservative,
        VolumeScenario::Moderate,
        VolumeScenario::Aggressive,
    ]
    .iter()
    .map(|scenario| ScenarioProjection {
        scenario: *scenario,
        label: scenario.label(),
        projection: project_fee_revenue(
            daily_volume,
            fee_bps,
            scenario.daily_growth_rate(),
            days,
        ),
    })
    .collect()
}

/// Result of changing the fee rate with a volume response.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FeeAdjustment {
    /// Fee rate before the change, in basis points.
    pub base_fee_bps: f64,
    /// Fee rate after the change, in basis points.
    pub new_fee_bps: f64,
    /// Daily volume before the change.
    pub base_daily_volume: f64,
    /// Daily volume after the change, net of the demand response.
    pub new_daily_volume: f64,
    /// Daily revenue before the change.
    pub base_daily_revenue: f64,
    /// Daily revenue after the change.
    pub new_daily_revenue: f64,
    /// Absolute change in daily revenue.
    pub daily_revenue_delta: f64,
    /// Change in daily revenue as a percentage of the base.
    pub revenue_change_pct: f64,
    /// Revenue over the horizon before the change.
    pub total_base_revenue: f64,
    /// Revenue over the horizon after the change.
    pub total_new_revenue: f64,
    /// Demand elasticity used (`-Δvolume% / Δfee%`).
    pub elasticity: f64,
}

/// Simulate a fee change, including how volume responds.
///
/// Demand elasticity `e` is `-Δvolume% / Δfee%`, so going from `f0` to `f1`
/// gives:
///
/// ```text
/// new_volume   = base_volume * (f1 / f0)^(-e)
/// new_revenue  = new_volume * f1 / 10_000
/// ```
///
/// A base fee of zero cannot be scaled; in that case volume and revenue are
/// unchanged. Growth is not modelled here: this is a steady-state comparison.
pub fn simulate_fee_adjustment(
    daily_volume: f64,
    base_fee_bps: f64,
    new_fee_bps: f64,
    elasticity: f64,
    days: u32,
) -> FeeAdjustment {
    let volume = finite_non_negative(daily_volume);
    let base_fee = finite_non_negative(base_fee_bps).min(BPS_DENOMINATOR);
    let new_fee = finite_non_negative(new_fee_bps).min(BPS_DENOMINATOR);
    let elasticity = if elasticity.is_finite() {
        elasticity.max(0.0)
    } else {
        0.0
    };

    let new_volume = if base_fee > 0.0 && new_fee > 0.0 {
        volume * (new_fee / base_fee).powf(-elasticity)
    } else if base_fee > 0.0 {
        // A zero fee charges nothing, so no revenue is collected regardless of
        // how much volume it attracts.
        0.0
    } else {
        volume
    };

    let base_daily_revenue = volume * base_fee / BPS_DENOMINATOR;
    let new_daily_revenue = new_volume * new_fee / BPS_DENOMINATOR;
    let daily_revenue_delta = new_daily_revenue - base_daily_revenue;
    let revenue_change_pct = if base_daily_revenue > 0.0 {
        daily_revenue_delta / base_daily_revenue * 100.0
    } else {
        0.0
    };
    let horizon = f64::from(days);

    FeeAdjustment {
        base_fee_bps: base_fee,
        new_fee_bps: new_fee,
        base_daily_volume: volume,
        new_daily_volume: new_volume,
        base_daily_revenue,
        new_daily_revenue,
        daily_revenue_delta,
        revenue_change_pct,
        total_base_revenue: base_daily_revenue * horizon,
        total_new_revenue: new_daily_revenue * horizon,
        elasticity,
    }
}

/// Fee rate in `min_fee_bps..=max_fee_bps` that maximises revenue.
///
/// With a constant elasticity `e`, fee revenue is proportional to
/// `fee^(1 - e)`. Revenue therefore rises with the fee when demand is inelastic
/// (`e < 1`), falls when it is elastic (`e > 1`), and is independent of the fee
/// at `e == 1`. The bounds are returned in the first two cases; `None` means
/// there is no unique maximiser.
pub fn revenue_maximizing_fee_bps(
    min_fee_bps: f64,
    max_fee_bps: f64,
    elasticity: f64,
) -> Option<f64> {
    let min = finite_non_negative(min_fee_bps).min(BPS_DENOMINATOR);
    let max = finite_non_negative(max_fee_bps).min(BPS_DENOMINATOR);
    let (low, high) = if min <= max { (min, max) } else { (max, min) };
    if !elasticity.is_finite() || (elasticity - 1.0).abs() <= f64::EPSILON {
        return None;
    }
    if elasticity > 1.0 {
        Some(low)
    } else {
        Some(high)
    }
}

/// Treasury runway under a constant monthly burn and revenue.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Runway {
    /// Starting balance.
    pub balance: f64,
    /// Monthly revenue.
    pub monthly_revenue: f64,
    /// Monthly burn.
    pub monthly_burn: f64,
    /// `monthly_burn - monthly_revenue`. Positive means the balance shrinks.
    pub net_monthly_burn: f64,
    /// Months until the balance is exhausted; `None` if revenue covers the burn.
    pub months: Option<f64>,
    /// The same runway in days.
    pub days: Option<f64>,
}

/// How long a treasury lasts at a constant monthly burn and revenue.
///
/// This is a linear extrapolation: it ignores yield on reserves and any growth
/// in revenue. Use [`RevenueModel::project`] when compounding matters.
pub fn project_runway(balance: f64, monthly_burn: f64, monthly_revenue: f64) -> Runway {
    let balance = finite_non_negative(balance);
    let burn = finite_non_negative(monthly_burn);
    let revenue = finite_non_negative(monthly_revenue);
    let net = burn - revenue;

    if net <= 0.0 {
        return Runway {
            balance,
            monthly_revenue: revenue,
            monthly_burn: burn,
            net_monthly_burn: net,
            months: None,
            days: None,
        };
    }

    let months = balance / net;
    Runway {
        balance,
        monthly_revenue: revenue,
        monthly_burn: burn,
        net_monthly_burn: net,
        months: Some(months),
        days: Some(months * DAYS_PER_MONTH),
    }
}

/// A published protocol-revenue observation, used for comparison.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ProtocolBenchmark {
    /// Protocol name.
    pub protocol: &'static str,
    /// Protocol category.
    pub category: &'static str,
    /// Gross fees collected over the period, in USD.
    pub annual_gross_fees_usd: f64,
    /// Revenue retained by the protocol over the period, in USD.
    pub annual_protocol_revenue_usd: f64,
    /// Where the figures come from.
    pub source: &'static str,
}

impl ProtocolBenchmark {
    /// Share of gross fees the protocol retained.
    pub fn capture_rate(&self) -> f64 {
        if self.annual_gross_fees_usd <= 0.0 {
            0.0
        } else {
            self.annual_protocol_revenue_usd / self.annual_gross_fees_usd
        }
    }
}

/// This projection placed next to one benchmark.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BenchmarkComparison {
    /// Protocol name.
    pub protocol: &'static str,
    /// Protocol category.
    pub category: &'static str,
    /// Gross fees collected by the benchmark over the period, in USD.
    pub annual_gross_fees_usd: f64,
    /// Revenue the benchmark retained over the period, in USD.
    pub annual_protocol_revenue_usd: f64,
    /// Benchmark capture rate.
    pub capture_rate: f64,
    /// Projected annualised gross fees divided by benchmark gross fees.
    pub projected_gross_ratio: f64,
    /// Projected annualised protocol revenue divided by benchmark revenue.
    pub projected_revenue_ratio: f64,
}

/// Compare a projection with protocol benchmarks.
///
/// Gross fees are compared with gross fees, and protocol-retained revenue with
/// protocol-retained revenue, so a projection is never flattered by comparing
/// its gross take with a benchmark's net take.
pub fn compare_to_benchmarks(
    projection: &RevenueProjection,
    benchmarks: &[ProtocolBenchmark],
) -> Vec<BenchmarkComparison> {
    benchmarks
        .iter()
        .map(|benchmark| BenchmarkComparison {
            protocol: benchmark.protocol,
            category: benchmark.category,
            annual_gross_fees_usd: benchmark.annual_gross_fees_usd,
            annual_protocol_revenue_usd: benchmark.annual_protocol_revenue_usd,
            capture_rate: benchmark.capture_rate(),
            projected_gross_ratio: safe_ratio(
                projection.annualized_revenue,
                benchmark.annual_gross_fees_usd,
            ),
            projected_revenue_ratio: safe_ratio(
                projection.annualized_protocol_revenue,
                benchmark.annual_protocol_revenue_usd,
            ),
        })
        .collect()
}

/// Reference benchmarks: 2025 gross fees and protocol revenue for five DeFi
/// protocols, as reported by DefiLlama and Token Terminal.
///
/// These are illustrative snapshots that move continuously with markets; they
/// are here so a projection has a like-for-like yardstick, not as a target.
/// Pass your own [`ProtocolBenchmark`] list when you need a different peer set.
pub fn reference_benchmarks() -> Vec<ProtocolBenchmark> {
    vec![
        ProtocolBenchmark {
            protocol: "Uniswap",
            category: "DEX",
            annual_gross_fees_usd: 892_000_000.0,
            annual_protocol_revenue_usd: 12_000_000.0,
            source: "DefiLlama Pro protocol revenue dashboard, May 2026 (2025 run-rate)",
        },
        ProtocolBenchmark {
            protocol: "Aave",
            category: "Lending",
            annual_gross_fees_usd: 951_000_000.0,
            annual_protocol_revenue_usd: 127_000_000.0,
            source: "DefiLlama Pro protocol revenue dashboard, May 2026 (2025 run-rate)",
        },
        ProtocolBenchmark {
            protocol: "Lido",
            category: "Liquid staking",
            annual_gross_fees_usd: 846_700_000.0,
            annual_protocol_revenue_usd: 84_700_000.0,
            source: "Token Terminal via QuickNode, Jul 2025 (2025 run-rate)",
        },
        ProtocolBenchmark {
            protocol: "Hyperliquid",
            category: "Perp DEX",
            annual_gross_fees_usd: 1_050_000_000.0,
            annual_protocol_revenue_usd: 943_000_000.0,
            source: "DefiLlama Pro protocol revenue dashboard, May 2026 (2025 run-rate)",
        },
        ProtocolBenchmark {
            protocol: "Jupiter",
            category: "Aggregator",
            annual_gross_fees_usd: 444_000_000.0,
            annual_protocol_revenue_usd: 134_000_000.0,
            source: "DefiLlama Pro protocol revenue dashboard, May 2026 (2025 run-rate)",
        },
    ]
}

/// Simulate the treasury balance path and its flows.
fn project_treasury(
    daily_fee_revenue: &[f64],
    treasury_share: f64,
    policy: &TreasuryPolicy,
) -> TreasuryProjection {
    let mut balance = policy.initial_balance;
    let mut daily_balance = Vec::with_capacity(daily_fee_revenue.len());
    let mut total_inflow = 0.0;
    let mut total_reserve_yield = 0.0;
    let mut first_depleted_day = None;
    let mut min_balance = balance;
    let daily_yield_rate = policy.reserve_yield_apr / DAYS_PER_YEAR;

    for (index, fee) in daily_fee_revenue.iter().enumerate() {
        // Yield is earned on positive reserves only: an overdrawn treasury does
        // not accrue interest to pay its way out.
        let yield_earned = if balance > 0.0 {
            balance * daily_yield_rate
        } else {
            0.0
        };
        total_reserve_yield += yield_earned;
        balance += yield_earned;

        let inflow = fee * treasury_share;
        total_inflow += inflow;
        balance += inflow;

        balance -= policy.operating_cost_per_day;

        if balance < 0.0 && first_depleted_day.is_none() {
            first_depleted_day = Some(index as u32 + 1);
        }
        min_balance = min_balance.min(balance);
        daily_balance.push(balance);
    }

    let days = daily_fee_revenue.len();
    let average_daily_inflow = if days == 0 {
        0.0
    } else {
        total_inflow / days as f64
    };
    let net_burn_rate_per_day = policy.operating_cost_per_day - average_daily_inflow;
    let last_inflow = daily_fee_revenue
        .last()
        .map(|fee| fee * treasury_share)
        .unwrap_or(0.0);

    let runway_days = if let Some(day) = first_depleted_day {
        Some(f64::from(day))
    } else if days == 0 {
        if policy.operating_cost_per_day > 0.0 && balance <= 0.0 {
            Some(0.0)
        } else {
            None
        }
    } else if last_inflow >= policy.operating_cost_per_day {
        None
    } else {
        let net_outflow = policy.operating_cost_per_day - last_inflow;
        Some(days as f64 + balance / net_outflow)
    };

    TreasuryProjection {
        initial_balance: policy.initial_balance,
        closing_balance: balance,
        total_inflow,
        total_operating_cost: policy.operating_cost_per_day * days as f64,
        total_reserve_yield,
        daily_balance,
        min_balance,
        average_daily_inflow,
        operating_cost_per_day: policy.operating_cost_per_day,
        net_burn_rate_per_day,
        first_depleted_day,
        runway_days,
    }
}

/// Clamp a distribution into a usable split, rescaling when it does not sum to
/// 10 000 bps.
fn normalise_distribution(
    distribution: FeeDistribution,
    warnings: &mut Vec<&'static str>,
) -> FeeDistribution {
    let mut clean = FeeDistribution {
        treasury_bps: finite_non_negative(distribution.treasury_bps),
        lp_bps: finite_non_negative(distribution.lp_bps),
        staker_bps: finite_non_negative(distribution.staker_bps),
        burn_bps: finite_non_negative(distribution.burn_bps),
    };

    let total = clean.total_bps();
    if total <= 0.0 {
        warnings.push("fee distribution was empty; using the reference split");
        return FeeDistribution::default();
    }
    if (total - BPS_DENOMINATOR).abs() > 1e-6 {
        warnings.push("fee distribution did not sum to 10 000 bps; rescaled");
        let scale = BPS_DENOMINATOR / total;
        clean.treasury_bps *= scale;
        clean.lp_bps *= scale;
        clean.staker_bps *= scale;
        clean.burn_bps *= scale;
    }
    clean
}

/// Clamp the treasury policy into a usable one.
fn normalise_treasury(policy: TreasuryPolicy, warnings: &mut Vec<&'static str>) -> TreasuryPolicy {
    if policy.initial_balance < 0.0 || !policy.initial_balance.is_finite() {
        warnings.push("initial_balance was negative or not finite; using 0");
    }
    if policy.reserve_yield_apr < 0.0 || !policy.reserve_yield_apr.is_finite() {
        warnings.push("reserve_yield_apr was negative or not finite; using 0");
    }
    if policy.operating_cost_per_day < 0.0 || !policy.operating_cost_per_day.is_finite() {
        warnings.push("operating_cost_per_day was negative or not finite; using 0");
    }
    TreasuryPolicy {
        initial_balance: finite_non_negative(policy.initial_balance),
        reserve_yield_apr: finite_non_negative(policy.reserve_yield_apr),
        operating_cost_per_day: finite_non_negative(policy.operating_cost_per_day),
    }
}

/// Clamp a fee stream into a usable one.
fn normalise_stream(stream: FeeStream, warnings: &mut Vec<&'static str>) -> FeeStream {
    if !stream.daily_volume.is_finite() || stream.daily_volume < 0.0 {
        warnings.push("daily_volume was negative or not finite; using 0");
    }
    if !stream.fee_bps.is_finite() || !(0.0..=BPS_DENOMINATOR).contains(&stream.fee_bps) {
        warnings.push("fee_bps was outside 0..=10 000 or not finite; clamped");
    }
    if !stream.daily_growth_rate.is_finite() || stream.daily_growth_rate < -1.0 {
        warnings.push("daily_growth_rate was below -100 % or not finite; clamped");
    }
    FeeStream {
        label: stream.label,
        daily_volume: finite_non_negative(stream.daily_volume),
        fee_bps: if stream.fee_bps.is_finite() {
            stream.fee_bps.clamp(0.0, BPS_DENOMINATOR)
        } else {
            0.0
        },
        daily_growth_rate: if stream.daily_growth_rate.is_finite() {
            stream.daily_growth_rate.max(-1.0)
        } else {
            0.0
        },
    }
}

/// A finite non-negative amount, or zero.
fn finite_non_negative(value: f64) -> f64 {
    if value.is_finite() && value > 0.0 {
        value
    } else {
        0.0
    }
}

/// `numerator / denominator`, or zero when the denominator is not positive.
fn safe_ratio(numerator: f64, denominator: f64) -> f64 {
    if denominator > 0.0 && denominator.is_finite() {
        numerator / denominator
    } else {
        0.0
    }
}

/// Test-only helper kept out of the public surface.
#[cfg(test)]
impl RevenueProjection {
    fn parameters_daily_volume(&self) -> f64 {
        self.streams.first().map(|stream| stream.initial_daily_volume).unwrap_or(0.0)
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    fn assert_close(actual: f64, expected: f64) {
        let tolerance = expected.abs().max(1.0) * 1e-9;
        assert!(
            (actual - expected).abs() <= tolerance,
            "expected {expected}, got {actual}"
        );
    }

    #[test]
    fn flat_revenue_is_volume_times_fee() {
        let projection = project_fee_revenue(1_000_000.0, 30.0, 0.0, 30);

        assert_close(projection.total_fee_revenue, 90_000.0);
        assert_close(projection.first_day_fee_revenue, 3_000.0);
        assert_close(projection.average_daily_fee_revenue, 3_000.0);
        assert_close(projection.trailing_30_day_revenue, 90_000.0);
        // 3 000/day over 365 days.
        assert_close(projection.annualized_revenue, 1_095_000.0);
        assert_eq!(projection.daily_fee_revenue.len(), 30);
        assert_eq!(projection.monthly_fee_revenue.len(), 1);
        assert_close(projection.monthly_fee_revenue[0], 90_000.0);
    }

    #[test]
    fn growth_compounds_daily() {
        let projection = project_fee_revenue(1_000.0, 100.0, 0.01, 3);

        assert_close(projection.daily_fee_revenue[0], 10.0);
        assert_close(projection.daily_fee_revenue[1], 10.1);
        assert_close(projection.daily_fee_revenue[2], 10.201);
        assert_close(projection.total_fee_revenue, 30.301);
        assert_close(projection.streams[0].final_daily_volume, 1_020.1);
        assert_close(projection.streams[0].final_daily_revenue, 10.201);
    }

    #[test]
    fn monthly_buckets_and_partial_months() {
        let projection = project_fee_revenue(1_000.0, 100.0, 0.0, 45);

        assert_eq!(projection.daily_fee_revenue.len(), 45);
        assert_eq!(projection.monthly_fee_revenue.len(), 2);
        assert_close(projection.monthly_fee_revenue[0], 300.0);
        assert_close(projection.monthly_fee_revenue[1], 150.0);
    }

    #[test]
    fn zero_days_produces_an_empty_neutral_projection() {
        let projection = project_fee_revenue(1_000_000.0, 30.0, 0.0, 0);

        assert_eq!(projection.days, 0);
        assert!(projection.daily_fee_revenue.is_empty());
        assert!(projection.monthly_fee_revenue.is_empty());
        assert_close(projection.total_fee_revenue, 0.0);
        assert_close(projection.annualized_revenue, 0.0);
        assert_close(projection.distribution.total(), 0.0);
        assert!(projection.treasury.daily_balance.is_empty());
    }

    #[test]
    fn distribution_shares_sum_to_the_collected_fee() {
        let model = RevenueModel::new(
            vec![FeeStream::dex_swaps(1_000_000.0, 30.0, 0.0)],
            FeeDistribution {
                treasury_bps: 4_000.0,
                lp_bps: 3_000.0,
                staker_bps: 2_000.0,
                burn_bps: 1_000.0,
            },
            TreasuryPolicy::default(),
            10,
        );
        let projection = model.project();

        assert_close(projection.total_fee_revenue, 30_000.0);
        assert_close(projection.distribution.treasury, 12_000.0);
        assert_close(projection.distribution.liquidity_providers, 9_000.0);
        assert_close(projection.distribution.stakers, 6_000.0);
        assert_close(projection.distribution.burned, 3_000.0);
        assert_close(projection.distribution.total(), 30_000.0);
        assert_close(
            projection.annualized_protocol_revenue,
            projection.annualized_revenue * 0.4,
        );
        assert_eq!(projection.warnings.len(), 0);
    }

    #[test]
    fn a_distribution_that_does_not_sum_to_a_whole_is_rescaled() {
        let model = RevenueModel::new(
            vec![FeeStream::other(100_000.0, 100.0, 0.0)],
            FeeDistribution {
                treasury_bps: 6_000.0,
                lp_bps: 6_000.0,
                staker_bps: 0.0,
                burn_bps: 0.0,
            },
            TreasuryPolicy::default(),
            1,
        );
        let projection = model.project();

        assert_close(projection.total_fee_revenue, 1_000.0);
        // 6 000 / 12 000 scaled to 10 000 bps each: an even split.
        assert_close(projection.distribution.treasury, 500.0);
        assert_close(projection.distribution.liquidity_providers, 500.0);
        assert_close(projection.distribution.total(), 1_000.0);
        assert!(projection
            .warnings
            .iter()
            .any(|warning| warning.contains("rescaled")));
    }

    #[test]
    fn treasury_compounds_reserves_and_pays_costs() {
        let model = RevenueModel::new(
            vec![],
            FeeDistribution::default(),
            TreasuryPolicy {
                initial_balance: 100_000.0,
                reserve_yield_apr: 0.10,
                operating_cost_per_day: 1_000.0,
            },
            3,
        );
        let projection = model.project();

        assert_close(projection.treasury.initial_balance, 100_000.0);
        // Day 1: 100k + 100k * 0.10 / 365 - 1k.
        assert_close(
            projection.treasury.daily_balance[0],
            99_027.397_260_273_98,
        );
        assert!(projection.treasury.total_reserve_yield > 0.0);
        assert_close(projection.treasury.total_operating_cost, 3_000.0);
        assert!(projection.treasury.net_burn_rate_per_day > 0.0);
        assert_close(projection.treasury.closing_balance, projection.treasury.daily_balance[2]);
    }

    #[test]
    fn treasury_depletion_is_recorded() {
        let model = RevenueModel::new(
            vec![],
            FeeDistribution::default(),
            TreasuryPolicy {
                initial_balance: 2_500.0,
                reserve_yield_apr: 0.0,
                operating_cost_per_day: 1_000.0,
            },
            5,
        );
        let projection = model.project();

        assert_eq!(projection.treasury.first_depleted_day, Some(3));
        assert_eq!(projection.treasury.runway_days, Some(3.0));
        assert!(projection.treasury.closing_balance < 0.0);
        assert_close(projection.treasury.min_balance, -2_500.0);
    }

    #[test]
    fn revenue_can_extend_the_runway_indefinitely() {
        let model = RevenueModel::new(
            vec![FeeStream::dex_swaps(1_000_000.0, 30.0, 0.0)],
            FeeDistribution::default(),
            TreasuryPolicy {
                initial_balance: 1_000.0,
                reserve_yield_apr: 0.0,
                operating_cost_per_day: 1_000.0,
            },
            30,
        );
        let projection = model.project();

        // 1 500/day of treasury inflow covers 1 000/day of costs.
        assert_eq!(projection.treasury.first_depleted_day, None);
        assert_eq!(projection.treasury.runway_days, None);
        assert!(projection.treasury.net_burn_rate_per_day < 0.0);
        assert_close(projection.treasury.total_inflow, 45_000.0);
    }

    #[test]
    fn runway_matches_the_linear_model() {
        let runway = project_runway(1_000.0, 200.0, 100.0);

        assert_close(runway.net_monthly_burn, 100.0);
        assert_close(runway.months.unwrap(), 10.0);
        assert_close(runway.days.unwrap(), 304.375);

        let sustainable = project_runway(1_000.0, 100.0, 250.0);
        assert_eq!(sustainable.net_monthly_burn, -150.0);
        assert_eq!(sustainable.months, None);
        assert_eq!(sustainable.days, None);
    }

    #[test]
    fn an_elastic_demand_response_can_make_a_fee_cut_profitable() {
        // Halving the fee to 15 bps, elasticity 2: volume quadruples.
        let adjustment = simulate_fee_adjustment(1_000_000.0, 30.0, 15.0, 2.0, 30);

        assert_close(adjustment.base_daily_revenue, 3_000.0);
        assert_close(adjustment.new_daily_volume, 4_000_000.0);
        assert_close(adjustment.new_daily_revenue, 6_000.0);
        assert_close(adjustment.daily_revenue_delta, 3_000.0);
        assert_close(adjustment.revenue_change_pct, 100.0);
        assert_close(adjustment.total_new_revenue, 180_000.0);
    }

    #[test]
    fn an_inelastic_demand_response_makes_a_fee_rise_profitable() {
        // Doubling the fee to 60 bps, elasticity 0.2: volume falls to 0.8706x.
        let adjustment = simulate_fee_adjustment(1_000_000.0, 30.0, 60.0, 0.2, 30);

        assert_close(adjustment.new_daily_revenue, 5_223.303_379_776_745);
        assert!(adjustment.revenue_change_pct > 70.0);
        assert!(adjustment.daily_revenue_delta > 0.0);
    }

    #[test]
    fn a_zero_base_fee_cannot_be_scaled() {
        let adjustment = simulate_fee_adjustment(1_000_000.0, 0.0, 30.0, 1.0, 10);

        assert_close(adjustment.base_daily_revenue, 0.0);
        assert_close(adjustment.new_daily_revenue, 3_000.0);
        // There is no base to compute a percentage change against.
        assert_close(adjustment.revenue_change_pct, 0.0);
    }

    #[test]
    fn revenue_maximising_fee_depends_on_elasticity() {
        assert_eq!(revenue_maximizing_fee_bps(5.0, 100.0, 0.5), Some(100.0));
        assert_eq!(revenue_maximizing_fee_bps(5.0, 100.0, 2.0), Some(5.0));
        assert_eq!(revenue_maximizing_fee_bps(5.0, 100.0, 1.0), None);
        // Bounds are normalised rather than trusted.
        assert_eq!(revenue_maximizing_fee_bps(100.0, 5.0, 2.0), Some(5.0));
    }

    #[test]
    fn scenarios_increase_with_growth_assumptions() {
        let rows = compare_volume_scenarios(1_000_000.0, 30.0, 30);

        assert_eq!(rows.len(), 3);
        assert_eq!(rows[0].label, "conservative");
        assert_eq!(rows[1].label, "moderate");
        assert_eq!(rows[2].label, "aggressive");
        assert!(
            rows[0].projection.total_fee_revenue < rows[1].projection.total_fee_revenue
        );
        assert!(
            rows[1].projection.total_fee_revenue < rows[2].projection.total_fee_revenue
        );
        assert_close(rows[0].projection.total_fee_revenue, 90_000.0);
    }

    #[test]
    fn multiple_streams_add_up() {
        let model = RevenueModel::new(
            vec![
                FeeStream::dex_swaps(1_000_000.0, 30.0, 0.0),
                FeeStream::lending_interest(500_000.0, 50.0, 0.0),
                FeeStream::other(100_000.0, 0.0, 0.0),
            ],
            FeeDistribution::default(),
            TreasuryPolicy::default(),
            1,
        );
        let projection = model.project();

        assert_eq!(projection.streams.len(), 3);
        assert_eq!(projection.streams[0].label, "DEX swaps");
        assert_eq!(projection.streams[1].label, "lending interest");
        assert_close(projection.streams[0].total_revenue, 3_000.0);
        assert_close(projection.streams[1].total_revenue, 2_500.0);
        assert_close(projection.streams[2].total_revenue, 0.0);
        assert_close(projection.total_fee_revenue, 5_500.0);
    }

    #[test]
    fn benchmark_comparison_matches_gross_with_gross() {
        let projection = project_fee_revenue(1_000_000.0, 30.0, 0.0, 365);
        let benchmarks = vec![ProtocolBenchmark {
            protocol: "Reference",
            category: "DEX",
            annual_gross_fees_usd: 1_095_000.0,
            annual_protocol_revenue_usd: 547_500.0,
            source: "test",
        }];
        let rows = compare_to_benchmarks(&projection, &benchmarks);

        assert_eq!(rows.len(), 1);
        assert_close(rows[0].capture_rate, 0.5);
        assert_close(rows[0].projected_gross_ratio, 1.0);
        // Half of projected gross revenue is retained by the treasury.
        assert_close(rows[0].projected_revenue_ratio, 1.0);
    }

    #[test]
    fn reference_benchmarks_are_coherent() {
        let benchmarks = reference_benchmarks();

        assert_eq!(benchmarks.len(), 5);
        for benchmark in &benchmarks {
            assert!(benchmark.annual_gross_fees_usd > 0.0);
            assert!(benchmark.annual_protocol_revenue_usd >= 0.0);
            assert!(
                (0.0..=1.0).contains(&benchmark.capture_rate()),
                "{} has an impossible capture rate",
                benchmark.protocol
            );
            assert!(!benchmark.source.is_empty());
        }
    }

    #[test]
    fn invalid_inputs_are_sanitised() {
        let model = RevenueModel::new(
            vec![FeeStream::new("bad", -1.0, 20_000.0, -5.0)],
            FeeDistribution {
                treasury_bps: -1.0,
                lp_bps: f64::NAN,
                staker_bps: 0.0,
                burn_bps: 0.0,
            },
            TreasuryPolicy {
                initial_balance: f64::INFINITY,
                reserve_yield_apr: -1.0,
                operating_cost_per_day: f64::NAN,
            },
            1,
        );
        let projection = model.project();

        assert_close(projection.parameters_daily_volume(), 0.0);
        assert!(projection
            .warnings
            .iter()
            .any(|warning| warning.contains("reference split")));
        for revenue in &projection.daily_fee_revenue {
            assert!(revenue.is_finite());
        }
        assert!(projection.treasury.closing_balance.is_finite());
        assert_close(projection.treasury.total_reserve_yield, 0.0);
    }

    #[test]
    fn days_are_clamped_to_the_supported_horizon() {
        let projection = project_fee_revenue(1_000.0, 30.0, 0.0, u32::MAX);

        assert_eq!(projection.days, MAX_PROJECTION_DAYS);
        assert_eq!(projection.daily_fee_revenue.len(), MAX_PROJECTION_DAYS as usize);
        assert!(projection
            .warnings
            .iter()
            .any(|warning| warning.contains("days clamped")));
    }

    #[test]
    fn a_stream_with_a_single_day_has_a_trailing_window_of_one() {
        let projection = project_fee_revenue(1_000_000.0, 30.0, 0.0, 1);

        assert_close(projection.trailing_30_day_revenue, 3_000.0);
        assert_close(projection.annualized_revenue, 3_000.0 * 365.0);
    }
}
