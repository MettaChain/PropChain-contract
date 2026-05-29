// Strategy implementations for fee calculation (Issue #186)

pub trait FeeStrategy {
    fn calculate(&self, config: &FeeConfig, context: &FeeContext) -> u128;
}

pub struct FixedStrategy;
pub struct DynamicStrategy;
pub struct TieredStrategy;
pub struct ExponentialStrategy;

impl FeeStrategy for FixedStrategy {
    fn calculate(&self, config: &FeeConfig, _context: &FeeContext) -> u128 {
        config.base_fee.clamp(config.min_fee, config.max_fee)
    }
}

impl FeeStrategy for DynamicStrategy {
    fn calculate(&self, config: &FeeConfig, context: &FeeContext) -> u128 {
        // Congestion multiplier: 0-100 -> 0% to (MAX_CONGESTION_MULTIPLIER-100)%
        let congestion_bp = (context.congestion_index as u128)
            .saturating_mul(config.congestion_sensitivity as u128)
            .saturating_mul((MAX_CONGESTION_MULTIPLIER - 100) as u128)
            / 10_000;
        let demand_bp = context.demand_factor_bp.min(5000); // Cap demand at 50%
        let total_multiplier_bp = 10_000u128
            .saturating_add(congestion_bp)
            .saturating_add(demand_bp as u128);
        let fee = config
            .base_fee
            .saturating_mul(total_multiplier_bp)
            .saturating_div(BASIS_POINTS);
        fee.clamp(config.min_fee, config.max_fee)
    }
}

impl FeeStrategy for TieredStrategy {
    fn calculate(&self, config: &FeeConfig, context: &FeeContext) -> u128 {
        // Simplified tiered approach based on operation complexity/impact
        let multiplier_bp = match context.operation {
            FeeOperation::RegisterProperty => 20000, // 2x
            FeeOperation::TransferProperty => 15000, // 1.5x
            FeeOperation::CreateEscrow => 12000,     // 1.2x
            FeeOperation::PremiumListingBid => 25000, // 2.5x
            _ => 10000,                             // 1x
        };
        let fee = config
            .base_fee
            .saturating_mul(multiplier_bp)
            .saturating_div(BASIS_POINTS);
        fee.clamp(config.min_fee, config.max_fee)
    }
}

impl FeeStrategy for ExponentialStrategy {
    fn calculate(&self, config: &FeeConfig, context: &FeeContext) -> u128 {
        // Non-linear scaling: (congestion/100)^2
        let c = context.congestion_index as u128;
        let congestion_sq = c.saturating_mul(c); // 0 to 10000
        let exp_factor_bp = congestion_sq
            .saturating_mul(config.congestion_sensitivity as u128)
            .saturating_div(100); 
        
        let total_multiplier_bp = 10_000u128.saturating_add(exp_factor_bp);
        let fee = config
            .base_fee
            .saturating_mul(total_multiplier_bp)
            .saturating_div(BASIS_POINTS);
        fee.clamp(config.min_fee, config.max_fee)
    }
}

pub struct FeeCalculator;

impl FeeCalculator {
    pub fn calculate(config: &FeeConfig, context: &FeeContext) -> u128 {
        match config.calculation_method {
            FeeCalculationMethod::Fixed => FixedStrategy.calculate(config, context),
            FeeCalculationMethod::Dynamic => DynamicStrategy.calculate(config, context),
            FeeCalculationMethod::Tiered => TieredStrategy.calculate(config, context),
            FeeCalculationMethod::Exponential => ExponentialStrategy.calculate(config, context),
        }
    }
}
