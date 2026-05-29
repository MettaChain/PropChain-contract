use crate::{
    Balance, JurisdictionProfile, PropertyAssessment, TaxBreakdown, TaxRecord, TaxRule, TaxStatus,
    Timestamp, BASIS_POINTS_DENOMINATOR,
};

pub(crate) fn calculate_tax_record(
    property_id: u64,
    jurisdiction_code: u32,
    rule: TaxRule,
    profile: Option<JurisdictionProfile>,
    assessment: PropertyAssessment,
    existing: Option<TaxRecord>,
    now: Timestamp,
) -> (TaxRecord, TaxBreakdown) {
    let reporting_period = now / rule.reporting_frequency.period_millis();
    let combined_exemption = rule
        .exemption_amount
        .saturating_add(assessment.exemption_override);
    let taxable_value = assessment.assessed_value.saturating_sub(combined_exemption);
    let base_tax =
        taxable_value.saturating_mul(rule.rate_basis_points as Balance) / BASIS_POINTS_DENOMINATOR;
    let surcharge_amount = profile
        .map(|item| {
            base_tax.saturating_mul(item.surcharge_basis_points as Balance)
                / BASIS_POINTS_DENOMINATOR
        })
        .unwrap_or(0);
    let discount_amount = profile
        .filter(|item| {
            now <= assessment
                .last_assessed_at
                .saturating_add(item.optimization_window)
        })
        .map(|item| {
            base_tax.saturating_mul(item.early_payment_discount_basis_points as Balance)
                / BASIS_POINTS_DENOMINATOR
        })
        .unwrap_or(0);
    let previous_due = existing.map(|item| item.tax_due).unwrap_or(0);
    let previous_paid = existing.map(|item| item.paid_amount).unwrap_or(0);
    let due_at = existing
        .map(|item| item.due_at)
        .unwrap_or(now.saturating_add(rule.payment_due_period));
    let outstanding_previous = previous_due.saturating_sub(previous_paid);
    let penalty_amount = if outstanding_previous > 0 && now > due_at {
        outstanding_previous.saturating_mul(rule.penalty_basis_points as Balance)
            / BASIS_POINTS_DENOMINATOR
    } else {
        0
    };
    let total_due = base_tax
        .saturating_add(rule.fixed_charge)
        .saturating_add(surcharge_amount)
        .saturating_add(penalty_amount)
        .saturating_sub(discount_amount);
    let mut record = TaxRecord {
        property_id,
        jurisdiction_code,
        reporting_period,
        assessed_value: assessment.assessed_value,
        taxable_value,
        tax_due: total_due,
        paid_amount: previous_paid,
        penalty_amount,
        discount_amount,
        due_at,
        last_payment_at: existing.map(|item| item.last_payment_at).unwrap_or(0),
        status: TaxStatus::Assessed,
        payment_reference: existing
            .map(|item| item.payment_reference)
            .unwrap_or([0u8; 32]),
        report_hash: existing.map(|item| item.report_hash).unwrap_or([0u8; 32]),
    };
    record.status = resolve_status(record, now);

    (
        record,
        TaxBreakdown {
            taxable_value,
            base_tax,
            fixed_charge: rule.fixed_charge,
            surcharge_amount,
            discount_amount,
            penalty_amount,
            total_due,
        },
    )
}

pub(crate) fn build_breakdown(
    rule: TaxRule,
    profile: Option<JurisdictionProfile>,
    assessment: PropertyAssessment,
    record: TaxRecord,
    now: Timestamp,
) -> TaxBreakdown {
    let (_, breakdown) = calculate_tax_record(
        record.property_id,
        record.jurisdiction_code,
        rule,
        profile,
        assessment,
        Some(record),
        now,
    );
    breakdown
}

pub(crate) fn resolve_status(record: TaxRecord, now: Timestamp) -> TaxStatus {
    if record.paid_amount >= record.tax_due {
        TaxStatus::Paid
    } else if now > record.due_at {
        TaxStatus::Overdue
    } else if record.paid_amount > 0 {
        TaxStatus::PartiallyPaid
    } else {
        TaxStatus::Assessed
    }
}

pub(crate) fn days_until_due(now: Timestamp, due_at: Timestamp) -> Option<u16> {
    if due_at <= now {
        return None;
    }
    let millis_per_day = 24 * 60 * 60 * 1000u64;
    let days = ((due_at - now) / millis_per_day) as u16;
    Some(days)
}
