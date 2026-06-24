/// Tax Optimization Strategies Module
/// Provides suggestions for tax-efficient transaction structuring

/// Different transaction structuring strategies for tax efficiency
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(
    feature = "std",
    derive(ink::storage::traits::StorageLayout, scale_info::TypeInfo)
)]
pub enum StrategyType {
    /// Timing strategy to optimize payment schedules
    TimingOptimization,
    /// Property transfer optimization across entities
    PropertyTransfer,
    /// Portfolio rebalancing for tax efficiency
    PortfolioRebalancing,
    /// Entity structure optimization
    EntityStructuring,
    /// Installment-based transaction structuring
    InstallmentStructuring,
    /// Cross-border transaction optimization
    CrossBorderOptimization,
    /// Loss harvesting strategy
    LossHarvesting,
}

/// Represents a tax optimization strategy recommendation
#[derive(Debug, Clone, Copy, PartialEq, Eq, scale::Encode, scale::Decode)]
#[cfg_attr(
    feature = "std",
    derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
)]
pub struct TaxStrategy {
    /// Type of optimization strategy
    pub strategy_type: u8, // 0: Timing, 1: Transfer, 2: Portfolio, 3: Entity, 4: Installment, 5: CrossBorder, 6: LossHarvesting
    /// Estimated tax savings in basis points (e.g., 500 = 5%)
    pub estimated_savings_basis_points: u32,
    /// Absolute estimated tax savings
    pub estimated_savings_amount: Balance,
    /// Implementation complexity score (1-10)
    pub complexity_score: u8,
    /// Risk level (1-10)
    pub risk_level: u8,
    /// Recommended implementation period (milliseconds)
    pub implementation_period: u64,
    /// Whether strategy is applicable to current transaction
    pub is_applicable: bool,
    /// Confidence score (0-100)
    pub confidence_score: u8,
}

/// Timing-based optimization strategy
#[derive(Debug, Clone, Copy, PartialEq, Eq, scale::Encode, scale::Decode)]
#[cfg_attr(
    feature = "std",
    derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
)]
pub struct TimingStrategy {
    /// Optimal time to execute transaction (milliseconds from now)
    pub optimal_timing_offset: u64,
    /// Number of installments to split payment
    pub recommended_installments: u32,
    /// Estimated savings from timing optimization
    pub estimated_savings: Balance,
    /// Early payment benefit
    pub early_payment_benefit: Balance,
    /// Penalty avoidance through timing
    pub penalty_avoidance: Balance,
}

/// Property transfer optimization strategy
#[derive(Debug, Clone, Copy, PartialEq, Eq, scale::Encode, scale::Decode)]
#[cfg_attr(
    feature = "std",
    derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
)]
pub struct TransferStrategy {
    /// Whether to structure as multi-step transfer
    pub multi_step_transfer: bool,
    /// Intermediate entity recommendation
    pub use_intermediate_entity: bool,
    /// Recommended holding period (milliseconds)
    pub holding_period: u64,
    /// Tax basis adjustment opportunity
    pub basis_adjustment_opportunity: Balance,
    /// Estimated transfer tax savings
    pub estimated_savings: Balance,
}

/// Portfolio rebalancing strategy
#[derive(Debug, Clone, Copy, PartialEq, Eq, scale::Encode, scale::Decode)]
#[cfg_attr(
    feature = "std",
    derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
)]
pub struct PortfolioStrategy {
    /// Total portfolio value being optimized
    pub portfolio_value: Balance,
    /// Number of properties in portfolio
    pub property_count: u32,
    /// Estimated rebalancing savings
    pub estimated_savings: Balance,
    /// Recommended number of concurrent transactions
    pub max_concurrent_transactions: u32,
    /// Tax-loss harvesting opportunity
    pub harvesting_opportunity: Balance,
}

/// Entity structure optimization strategy
#[derive(Debug, Clone, Copy, PartialEq, Eq, scale::Encode, scale::Decode)]
#[cfg_attr(
    feature = "std",
    derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
)]
pub struct EntityStrategy {
    /// Recommended entity type (0: Individual, 1: LLC, 2: Corporation, 3: Trust, 4: Partnership)
    pub recommended_entity_type: u8,
    /// Tax rate under recommended entity
    pub entity_tax_rate_basis_points: u32,
    /// Current tax rate
    pub current_tax_rate_basis_points: u32,
    /// Estimated annual savings
    pub estimated_annual_savings: Balance,
    /// Setup cost for restructuring
    pub restructuring_cost: Balance,
}

/// Installment-based transaction strategy
#[derive(Debug, Clone, Copy, PartialEq, Eq, scale::Encode, scale::Decode)]
#[cfg_attr(
    feature = "std",
    derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
)]
pub struct InstallmentStrategy {
    /// Number of recommended installments
    pub installment_count: u32,
    /// Amount per installment
    pub amount_per_installment: Balance,
    /// Spacing between installments (milliseconds)
    pub installment_spacing: u64,
    /// Interest or fees for installment structure
    pub total_fees: Balance,
    /// Tax deferral benefit
    pub deferral_benefit: Balance,
}

/// Cross-border transaction strategy
#[derive(Debug, Clone, Copy, PartialEq, Eq, scale::Encode, scale::Decode)]
#[cfg_attr(
    feature = "std",
    derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
)]
pub struct CrossBorderStrategy {
    /// Source jurisdiction code
    pub source_jurisdiction: u32,
    /// Target jurisdiction code
    pub target_jurisdiction: u32,
    /// Current combined tax rate (basis points)
    pub current_combined_rate: u32,
    /// Optimized combined tax rate (basis points)
    pub optimized_combined_rate: u32,
    /// Treaty-based savings
    pub treaty_savings: Balance,
    /// Transfer pricing opportunity
    pub transfer_pricing_opportunity: Balance,
}

/// Comprehensive tax optimization analysis
#[derive(Debug, Clone, Copy, PartialEq, Eq, scale::Encode, scale::Decode)]
#[cfg_attr(
    feature = "std",
    derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
)]
pub struct OptimizationAnalysis {
    /// Total estimated tax savings
    pub total_savings: Balance,
    /// Number of applicable strategies
    pub applicable_strategies: u32,
    /// Primary recommended strategy (strategy type index)
    pub primary_strategy: u8,
    /// Combined complexity across all recommended strategies
    pub combined_complexity: u8,
    /// Overall risk assessment (1-10)
    pub overall_risk: u8,
    /// Implementation priority (1-10, higher = more urgent)
    pub priority_score: u8,
}

/// Calculates timing-based optimization strategy
pub(crate) fn calculate_timing_strategy(
    rule: TaxRule,
    profile: Option<JurisdictionProfile>,
    record: Option<TaxRecord>,
    now: Timestamp,
) -> TimingStrategy {
    let early_payment_benefit = profile
        .map(|p| {
            let base_tax = (1_000_000 * p.early_payment_discount_basis_points as Balance) / BASIS_POINTS_DENOMINATOR;
            base_tax
        })
        .unwrap_or(0);

    let penalty_avoidance = record
        .map(|r| {
            if r.status != TaxStatus::Paid {
                (r.tax_due * rule.penalty_basis_points as Balance) / BASIS_POINTS_DENOMINATOR
            } else {
                0
            }
        })
        .unwrap_or(0);

    let optimization_window = profile.map(|p| p.optimization_window).unwrap_or(30 * 24 * 60 * 60 * 1000);
    let installment_count = if penalty_avoidance > 0 { 2 } else { 1 };

    TimingStrategy {
        optimal_timing_offset: optimization_window / 2,
        recommended_installments: installment_count,
        estimated_savings: early_payment_benefit.saturating_add(penalty_avoidance),
        early_payment_benefit,
        penalty_avoidance,
    }
}

/// Calculates property transfer optimization strategy
pub(crate) fn calculate_transfer_strategy(
    assessment: PropertyAssessment,
    rule: TaxRule,
) -> TransferStrategy {
    let basis_adjustment = assessment.assessed_value / 10;
    let transfer_tax_savings = (basis_adjustment * rule.rate_basis_points as Balance) / BASIS_POINTS_DENOMINATOR;

    TransferStrategy {
        multi_step_transfer: assessment.assessed_value > 1_000_000,
        use_intermediate_entity: assessment.assessed_value > 500_000,
        holding_period: 365 * 24 * 60 * 60 * 1000, // 1 year
        basis_adjustment_opportunity: basis_adjustment,
        estimated_savings: transfer_tax_savings,
    }
}

/// Calculates portfolio rebalancing strategy
pub(crate) fn calculate_portfolio_strategy(
    total_value: Balance,
    property_count: u32,
    harvesting_opportunity: Balance,
) -> PortfolioStrategy {
    let rebalancing_savings = (total_value * 100) / BASIS_POINTS_DENOMINATOR; // ~1% savings
    let max_concurrent = (property_count + 4) / 5; // 1 transaction per 5 properties

    PortfolioStrategy {
        portfolio_value: total_value,
        property_count,
        estimated_savings: rebalancing_savings,
        max_concurrent_transactions: core::cmp::max(1, max_concurrent),
        harvesting_opportunity,
    }
}

/// Calculates entity structure optimization strategy
pub(crate) fn calculate_entity_strategy(
    current_tax_rate: u32,
    assessment_value: Balance,
) -> EntityStrategy {
    // LLC typically has ~20% lower tax rate than individual
    let recommended_rate = (current_tax_rate * 80) / 100;
    let annual_tax_base = (assessment_value * current_tax_rate as Balance) / BASIS_POINTS_DENOMINATOR;
    let optimized_tax = (assessment_value * recommended_rate as Balance) / BASIS_POINTS_DENOMINATOR;
    let savings = annual_tax_base.saturating_sub(optimized_tax);
    let restructuring_cost = assessment_value / 100; // ~1% of property value

    EntityStrategy {
        recommended_entity_type: 1, // LLC
        entity_tax_rate_basis_points: recommended_rate,
        current_tax_rate_basis_points: current_tax_rate,
        estimated_annual_savings: savings,
        restructuring_cost,
    }
}

/// Calculates installment strategy for transaction structuring
pub(crate) fn calculate_installment_strategy(
    total_amount: Balance,
) -> InstallmentStrategy {
    let installment_count = if total_amount > 1_000_000 { 4 } else if total_amount > 500_000 { 3 } else { 2 };
    let amount_per_installment = total_amount / installment_count as Balance;
    let spacing = 90 * 24 * 60 * 60 * 1000; // 90 days
    let fees = (total_amount * 50) / BASIS_POINTS_DENOMINATOR; // 0.5% fees
    let deferral_benefit = (total_amount * 200) / BASIS_POINTS_DENOMINATOR; // 2% deferral benefit

    InstallmentStrategy {
        installment_count,
        amount_per_installment,
        installment_spacing: spacing,
        total_fees: fees,
        deferral_benefit,
    }
}

/// Calculates cross-border transaction strategy
pub(crate) fn calculate_cross_border_strategy(
    source_jurisdiction: u32,
    target_jurisdiction: u32,
    source_rate: u32,
    target_rate: u32,
    transaction_value: Balance,
) -> CrossBorderStrategy {
    let current_combined = source_rate.saturating_add(target_rate) / 2;
    // Treaty typically reduces by 20-30%
    let optimized_combined = (current_combined * 75) / 100;
    let rate_reduction = current_combined.saturating_sub(optimized_combined);
    let treaty_savings = (transaction_value * rate_reduction as Balance) / BASIS_POINTS_DENOMINATOR;
    
    // Transfer pricing opportunity ~5% of transaction value
    let transfer_pricing = (transaction_value * 500) / BASIS_POINTS_DENOMINATOR;

    CrossBorderStrategy {
        source_jurisdiction,
        target_jurisdiction,
        current_combined_rate: current_combined,
        optimized_combined_rate: optimized_combined,
        treaty_savings,
        transfer_pricing_opportunity: transfer_pricing,
    }
}

/// Analyzes all applicable strategies and provides comprehensive recommendation
pub(crate) fn analyze_strategies(
    rule: TaxRule,
    profile: Option<JurisdictionProfile>,
    assessment: PropertyAssessment,
    record: Option<TaxRecord>,
    portfolio_value: Balance,
    property_count: u32,
    now: Timestamp,
) -> OptimizationAnalysis {
    // Calculate individual strategies
    let timing = calculate_timing_strategy(rule, profile, record, now);
    let transfer = calculate_transfer_strategy(assessment, rule);
    let portfolio = calculate_portfolio_strategy(portfolio_value, property_count, 0);
    let entity = calculate_entity_strategy(rule.rate_basis_points, assessment.assessed_value);

    // Calculate total savings
    let total_savings = timing.estimated_savings
        .saturating_add(transfer.estimated_savings)
        .saturating_add(portfolio.estimated_savings)
        .saturating_add(entity.estimated_annual_savings);

    // Count applicable strategies
    let mut applicable_strategies = 0;
    let mut primary_strategy = 0;
    let mut max_savings = 0;

    // Check timing strategy
    if timing.estimated_savings > 0 {
        applicable_strategies += 1;
        if timing.estimated_savings > max_savings {
            primary_strategy = 0;
            max_savings = timing.estimated_savings;
        }
    }

    // Check transfer strategy
    if transfer.estimated_savings > 0 {
        applicable_strategies += 1;
        if transfer.estimated_savings > max_savings {
            primary_strategy = 1;
            max_savings = transfer.estimated_savings;
        }
    }

    // Check portfolio strategy
    if portfolio.estimated_savings > 0 {
        applicable_strategies += 1;
        if portfolio.estimated_savings > max_savings {
            primary_strategy = 2;
            max_savings = portfolio.estimated_savings;
        }
    }

    // Check entity strategy
    if entity.estimated_annual_savings > 0 {
        applicable_strategies += 1;
        if entity.estimated_annual_savings > max_savings {
            primary_strategy = 3;
            max_savings = entity.estimated_annual_savings;
        }
    }

    let complexity = core::cmp::min(10, (applicable_strategies as u8) * 2);
    let risk = if profile.is_some() { 4 } else { 6 }; // Higher risk without jurisdiction profile
    let priority = if timing.penalty_avoidance > 0 { 10 } else if applicable_strategies > 2 { 8 } else { 5 };

    OptimizationAnalysis {
        total_savings,
        applicable_strategies,
        primary_strategy,
        combined_complexity: complexity,
        overall_risk: risk,
        priority_score: priority,
    }
}

/// Builds a comprehensive tax strategy recommendation
pub(crate) fn build_tax_strategy(
    strategy_type: u8,
    savings_amount: Balance,
    savings_basis_points: u32,
    complexity: u8,
    risk: u8,
    period: u64,
    is_applicable: bool,
    confidence: u8,
) -> TaxStrategy {
    TaxStrategy {
        strategy_type,
        estimated_savings_basis_points: savings_basis_points,
        estimated_savings_amount: savings_amount,
        complexity_score: complexity,
        risk_level: risk,
        implementation_period: period,
        is_applicable,
        confidence_score: confidence,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_timing_strategy_calculation() {
        let rule = TaxRule {
            rate_basis_points: 300,
            fixed_charge: 1_000,
            exemption_amount: 50_000,
            payment_due_period: 30 * 24 * 60 * 60 * 1000,
            reporting_frequency: ReportingFrequency::Annual,
            penalty_basis_points: 500,
            requires_reporting: false,
            requires_legal_documents: false,
            withholding_rate_basis_points: 0,
            tax_collector: [0x00; 32].into(),
            active: true,
        };

        let profile = Some(JurisdictionProfile {
            surcharge_basis_points: 100,
            early_payment_discount_basis_points: 150,
            late_payment_grace_period: 30 * 24 * 60 * 60 * 1000,
            optimization_window: 90 * 24 * 60 * 60 * 1000,
            requires_digital_stamp: false,
            authority_hash: [0x01; 32],
        });

        let now = 1000000;
        let strategy = calculate_timing_strategy(rule, profile, None, now);

        assert!(strategy.early_payment_benefit > 0);
        assert_eq!(strategy.recommended_installments, 1);
    }

    #[test]
    fn test_transfer_strategy_calculation() {
        let rule = TaxRule {
            rate_basis_points: 300,
            fixed_charge: 1_000,
            exemption_amount: 50_000,
            payment_due_period: 30 * 24 * 60 * 60 * 1000,
            reporting_frequency: ReportingFrequency::Annual,
            penalty_basis_points: 500,
            requires_reporting: false,
            requires_legal_documents: false,
            withholding_rate_basis_points: 0,
            tax_collector: [0x00; 32].into(),
            active: true,
        };

        let assessment = PropertyAssessment {
            owner: [0x01; 32].into(),
            assessed_value: 2_000_000,
            exemption_override: 0,
            last_assessed_at: 1000000,
            legal_documents_verified: true,
            reporting_submitted: true,
        };

        let strategy = calculate_transfer_strategy(assessment, rule);

        assert!(strategy.estimated_savings > 0);
        assert!(strategy.use_intermediate_entity); // Value > 500k
        assert!(strategy.multi_step_transfer); // Value > 1M
    }

    #[test]
    fn test_portfolio_strategy_calculation() {
        let total_value = 5_000_000;
        let property_count = 10;
        let harvesting_opportunity = 100_000;

        let strategy = calculate_portfolio_strategy(total_value, property_count, harvesting_opportunity);

        assert_eq!(strategy.portfolio_value, total_value);
        assert_eq!(strategy.property_count, property_count);
        assert!(strategy.estimated_savings > 0);
        assert_eq!(strategy.max_concurrent_transactions, 2); // (10 + 4) / 5 = 2
    }

    #[test]
    fn test_entity_strategy_calculation() {
        let current_rate = 300; // 3%
        let assessment_value = 1_000_000;

        let strategy = calculate_entity_strategy(current_rate, assessment_value);

        assert_eq!(strategy.recommended_entity_type, 1); // LLC
        assert!(strategy.entity_tax_rate_basis_points < strategy.current_tax_rate_basis_points);
        assert!(strategy.estimated_annual_savings > 0);
        assert!(strategy.restructuring_cost > 0);
    }

    #[test]
    fn test_installment_strategy_large_transaction() {
        let amount = 2_000_000;
        let strategy = calculate_installment_strategy(amount);

        assert_eq!(strategy.installment_count, 4);
        assert_eq!(strategy.amount_per_installment, 500_000);
        assert!(strategy.total_fees > 0);
        assert!(strategy.deferral_benefit > 0);
    }

    #[test]
    fn test_installment_strategy_medium_transaction() {
        let amount = 700_000;
        let strategy = calculate_installment_strategy(amount);

        assert_eq!(strategy.installment_count, 3);
        assert_eq!(strategy.amount_per_installment, 700_000 / 3);
    }

    #[test]
    fn test_installment_strategy_small_transaction() {
        let amount = 300_000;
        let strategy = calculate_installment_strategy(amount);

        assert_eq!(strategy.installment_count, 2);
        assert_eq!(strategy.amount_per_installment, 150_000);
    }

    #[test]
    fn test_cross_border_strategy_calculation() {
        let source_jurisdiction = 1001; // US
        let target_jurisdiction = 2001; // EU
        let source_rate = 300;
        let target_rate = 200;
        let transaction_value = 1_000_000;

        let strategy = calculate_cross_border_strategy(
            source_jurisdiction,
            target_jurisdiction,
            source_rate,
            target_rate,
            transaction_value,
        );

        assert_eq!(strategy.source_jurisdiction, source_jurisdiction);
        assert_eq!(strategy.target_jurisdiction, target_jurisdiction);
        assert!(strategy.treaty_savings > 0);
        assert!(strategy.transfer_pricing_opportunity > 0);
        assert!(strategy.optimized_combined_rate < strategy.current_combined_rate);
    }

    #[test]
    fn test_strategy_analysis() {
        let rule = TaxRule {
            rate_basis_points: 300,
            fixed_charge: 1_000,
            exemption_amount: 50_000,
            payment_due_period: 30 * 24 * 60 * 60 * 1000,
            reporting_frequency: ReportingFrequency::Annual,
            penalty_basis_points: 500,
            requires_reporting: false,
            requires_legal_documents: false,
            withholding_rate_basis_points: 0,
            tax_collector: [0x00; 32].into(),
            active: true,
        };

        let profile = Some(JurisdictionProfile {
            surcharge_basis_points: 100,
            early_payment_discount_basis_points: 150,
            late_payment_grace_period: 30 * 24 * 60 * 60 * 1000,
            optimization_window: 90 * 24 * 60 * 60 * 1000,
            requires_digital_stamp: false,
            authority_hash: [0x01; 32],
        });

        let assessment = PropertyAssessment {
            owner: [0x01; 32].into(),
            assessed_value: 1_000_000,
            exemption_override: 0,
            last_assessed_at: 1000000,
            legal_documents_verified: true,
            reporting_submitted: true,
        };

        let now = 1000000;
        let portfolio_value = 5_000_000;
        let property_count = 5;

        let analysis = analyze_strategies(
            rule,
            profile,
            assessment,
            None,
            portfolio_value,
            property_count,
            now,
        );

        assert!(analysis.total_savings > 0);
        assert!(analysis.applicable_strategies > 0);
        assert!(analysis.combined_complexity > 0);
        assert!(analysis.priority_score > 0);
    }

    #[test]
    fn test_build_tax_strategy() {
        let strategy = build_tax_strategy(
            0, // Timing
            50_000,
            500,
            3,
            4,
            90 * 24 * 60 * 60 * 1000,
            true,
            85,
        );

        assert_eq!(strategy.strategy_type, 0);
        assert_eq!(strategy.estimated_savings_amount, 50_000);
        assert_eq!(strategy.estimated_savings_basis_points, 500);
        assert_eq!(strategy.complexity_score, 3);
        assert_eq!(strategy.risk_level, 4);
        assert!(strategy.is_applicable);
        assert_eq!(strategy.confidence_score, 85);
    }
}
