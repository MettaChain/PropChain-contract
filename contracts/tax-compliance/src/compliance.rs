use crate::{
    payments, ComplianceAlert, ComplianceAlertLevel, ComplianceAlertType, ComplianceSnapshot,
    PropertyAssessment, TaxRecord, TaxRule, TaxStatus, Timestamp,
};
use ink::prelude::vec::Vec;

pub(crate) fn build_snapshot(
    property_id: u64,
    jurisdiction_code: u32,
    rule: TaxRule,
    assessment: PropertyAssessment,
    record: Option<TaxRecord>,
    registry_compliant: bool,
    active_alerts: u32,
) -> ComplianceSnapshot {
    let outstanding_tax = record
        .map(|item| payments::outstanding_tax(&item))
        .unwrap_or_default();
    let status = record
        .map(|item| item.status)
        .unwrap_or(TaxStatus::Assessed);
    let reporting_period = record.map(|item| item.reporting_period).unwrap_or_default();
    let tax_current = record
        .map(|item| {
            item.paid_amount >= item.tax_due
                && (!rule.requires_legal_documents || assessment.legal_documents_verified)
                && (!rule.requires_reporting || assessment.reporting_submitted)
        })
        .unwrap_or(false);

    ComplianceSnapshot {
        property_id,
        jurisdiction_code,
        reporting_period,
        registry_compliant,
        tax_current,
        outstanding_tax,
        reporting_submitted: assessment.reporting_submitted,
        legal_documents_verified: assessment.legal_documents_verified,
        active_alerts,
        status,
    }
}

pub(crate) fn generate_alerts(
    property_id: u64,
    jurisdiction_code: u32,
    rule: TaxRule,
    assessment: PropertyAssessment,
    record: Option<TaxRecord>,
    registry_compliant: bool,
    now: Timestamp,
) -> Vec<ComplianceAlert> {
    let mut alerts = Vec::new();

    if !registry_compliant {
        alerts.push(ComplianceAlert {
            property_id,
            jurisdiction_code,
            reporting_period: record.map(|item| item.reporting_period).unwrap_or_default(),
            alert_type: ComplianceAlertType::RegistryNonCompliant,
            level: ComplianceAlertLevel::Critical,
            outstanding_tax: record
                .map(|item| payments::outstanding_tax(&item))
                .unwrap_or_default(),
            due_at: record.map(|item| item.due_at).unwrap_or_default(),
            triggered_at: now,
        });
    }

    if let Some(item) = record {
        let outstanding = payments::outstanding_tax(&item);
        if outstanding > 0 {
            let (alert_type, level) = if now > item.due_at {
                (
                    ComplianceAlertType::TaxOverdue,
                    ComplianceAlertLevel::Critical,
                )
            } else {
                (
                    ComplianceAlertType::PaymentDueSoon,
                    ComplianceAlertLevel::Warning,
                )
            };
            alerts.push(ComplianceAlert {
                property_id,
                jurisdiction_code,
                reporting_period: item.reporting_period,
                alert_type,
                level,
                outstanding_tax: outstanding,
                due_at: item.due_at,
                triggered_at: now,
            });
        }

        if rule.requires_reporting && !assessment.reporting_submitted {
            alerts.push(ComplianceAlert {
                property_id,
                jurisdiction_code,
                reporting_period: item.reporting_period,
                alert_type: ComplianceAlertType::ReportingMissing,
                level: ComplianceAlertLevel::Warning,
                outstanding_tax: outstanding,
                due_at: item.due_at,
                triggered_at: now,
            });
        }

        if rule.requires_legal_documents && !assessment.legal_documents_verified {
            alerts.push(ComplianceAlert {
                property_id,
                jurisdiction_code,
                reporting_period: item.reporting_period,
                alert_type: ComplianceAlertType::LegalDocumentsMissing,
                level: ComplianceAlertLevel::Critical,
                outstanding_tax: outstanding,
                due_at: item.due_at,
                triggered_at: now,
            });
        }
    }

    alerts
}
