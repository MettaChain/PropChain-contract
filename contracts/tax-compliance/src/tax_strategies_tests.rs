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
            reporting_frequency: crate::ReportingFrequency::Annual,
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
            reporting_frequency: crate::ReportingFrequency::Annual,
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
            reporting_frequency: crate::ReportingFrequency::Annual,
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

    #[test]
    fn test_strategy_type_enum() {
        // Ensure all strategy types are properly defined
        assert_eq!(StrategyType::TimingOptimization as u8, 0);
        assert_eq!(StrategyType::PropertyTransfer as u8, 1);
        assert_eq!(StrategyType::PortfolioRebalancing as u8, 2);
        assert_eq!(StrategyType::EntityStructuring as u8, 3);
        assert_eq!(StrategyType::InstallmentStructuring as u8, 4);
    }
}
