use crate::{
    payments, Balance, JurisdictionProfile, OptimizationPlan, PropertyAssessment, TaxRecord,
    TaxRule, Timestamp,
};

pub(crate) fn recommend_plan(
    rule: TaxRule,
    profile: Option<JurisdictionProfile>,
    assessment: PropertyAssessment,
    record: Option<TaxRecord>,
    now: Timestamp,
) -> OptimizationPlan {
    let outstanding = record
        .as_ref()
        .map(payments::outstanding_tax)
        .unwrap_or_default();
    let review_exemption = assessment.exemption_override < (assessment.assessed_value / 20);
    let estimated_discount = profile
        .filter(|item| {
            now <= assessment
                .last_assessed_at
                .saturating_add(item.optimization_window)
        })
        .map(|item| {
            assessment
                .assessed_value
                .saturating_mul(item.early_payment_discount_basis_points as Balance)
                / 10_000
        })
        .unwrap_or(0);
    let estimated_savings = estimated_discount
        .saturating_add(outstanding.saturating_mul(rule.penalty_basis_points as Balance) / 10_000);

    OptimizationPlan {
        estimated_savings,
        recommended_installments: if outstanding > 0 { 2 } else { 1 },
        should_prepay: profile
            .map(|item| item.early_payment_discount_basis_points > 0)
            .unwrap_or(false),
        review_exemption,
        supporting_reference: profile.map(|item| item.authority_hash).unwrap_or([0u8; 32]),
    }
}
