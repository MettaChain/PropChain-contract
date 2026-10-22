
/// Strategy implementations for fee calculation (Issue #186).

const DEFAULT_MULTIPLIER_BP: u128 = 10_000;
const MAX_DEMAND_FACTOR_BP: u128 = 5_000;
const MAX_CONGESTION_INDEX: u128 = 100;

pub trait FeeStrategy {
    fn calculate(&self, config: &FeeConfig, context: &FeeContext) -> u128;
}

/// Applies the configured fee bounds.
///
/// Invariant: config.min_fee must not exceed config.max_fee.
fn clamp_fee(fee: u128, config: &FeeConfig) -> u128 {
    fee.clamp(config.min_fee, config.max_fee)
}

/// Calculates a fee using a multiplier expressed in basis points.
fn calculate_multiplied_fee(
    base_fee: u128,
    multiplier_bp: u128,
    config: &FeeConfig,
) -> u128 {
    clamp_fee(
        round_fee_up(base_fee, multiplier_bp, BASIS_POINTS),
        config,
    )
}

pub struct FixedStrategy;
pub struct DynamicStrategy;
pub struct TieredStrategy;
pub struct ExponentialStrategy;

impl FeeStrategy for FixedStrategy {
    fn calculate(&self, config: &FeeConfig, _context: &FeeContext) -> u128 {
        clamp_fee(config.base_fee, config)
    }
}

impl FeeStrategy for DynamicStrategy {
    fn calculate(&self, config: &FeeConfig, context: &FeeContext) -> u128 {
        let congestion = (context.congestion_index as u128)
            .min(MAX_CONGESTION_INDEX);

        let sensitivity = config.congestion_sensitivity as u128;
        let maximum_congestion_increase =
            (MAX_CONGESTION_MULTIPLIER.saturating_sub(100)) as u128;

        let congestion_bp = congestion
            .saturating_mul(sensitivity)
            .saturating_mul(maximum_congestion_increase)
            / DEFAULT_MULTIPLIER_BP;

        let demand_bp = (context.demand_factor_bp.get() as u128)
            .min(MAX_DEMAND_FACTOR_BP);

        let multiplier_bp = DEFAULT_MULTIPLIER_BP
            .saturating_add(congestion_bp)
            .saturating_add(demand_bp);

        calculate_multiplied_fee(
            config.base_fee,
            multiplier_bp,
            config,
        )
    }
}

impl TieredStrategy {
    fn multiplier_for(operation: FeeOperation) -> u128 {
        match operation {
            FeeOperation::RegisterProperty => 20_000,   // 2.0x
            FeeOperation::TransferProperty => 15_000,   // 1.5x
            FeeOperation::CreateEscrow => 12_000,       // 1.2x
            FeeOperation::PremiumListingBid => 25_000,  // 2.5x
            _ => DEFAULT_MULTIPLIER_BP,                 // 1.0x
        }
    }
}

impl FeeStrategy for TieredStrategy {
    fn calculate(&self, config: &FeeConfig, context: &FeeContext) -> u128 {
        let multiplier_bp = Self::multiplier_for(context.operation);

        calculate_multiplied_fee(
            config.base_fee,
            multiplier_bp,
            config,
        )
    }
}

impl FeeStrategy for ExponentialStrategy {
    fn calculate(&self, config: &FeeConfig, context: &FeeContext) -> u128 {
        let congestion = (context.congestion_index as u128)
            .min(MAX_CONGESTION_INDEX);

        // Quadratic congestion scaling: congestion² / 100.
        let congestion_squared = congestion.saturating_mul(congestion);

        let exponential_factor_bp = congestion_squared
            .saturating_mul(config.congestion_sensitivity as u128)
            / MAX_CONGESTION_INDEX;

        let multiplier_bp = DEFAULT_MULTIPLIER_BP
            .saturating_add(exponential_factor_bp);

        calculate_multiplied_fee(
            config.base_fee,
            multiplier_bp,
            config,
        )
    }
}

pub struct FeeCalculator;

impl FeeCalculator {
    pub fn calculate(config: &FeeConfig, context: &FeeContext) -> u128 {
        match config.calculation_method {
            FeeCalculationMethod::Fixed => {
                FixedStrategy.calculate(config, context)
            }
            FeeCalculationMethod::Dynamic => {
                DynamicStrategy.calculate(config, context)
            }
            FeeCalculationMethod::Tiered => {
                TieredStrategy.calculate(config, context)
            }
            FeeCalculationMethod::Exponential => {
                ExponentialStrategy.calculate(config, context)
            }
        }
    }
}
