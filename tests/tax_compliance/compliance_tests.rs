#![cfg(feature = "disabled_test")]

use compliance_registry::{ComplianceRegistry, TaxComplianceStatus, TaxJurisdictionRecord};

#[ink::test]
fn registry_keeps_tax_history_for_multi_jurisdiction_sync() {
    let mut registry = ComplianceRegistry::new();
    let account = ink::primitives::AccountId::from([0x41; 32]);

    registry
        .configure_tax_jurisdiction(TaxJurisdictionRecord {
            jurisdiction_code: 3001,
            reporting_cycle_days: 90,
            requires_legal_clearance: true,
            authority_hash: [4u8; 32],
        })
        .expect("jurisdiction");
    registry
        .update_tax_compliance_status(
            account,
            TaxComplianceStatus {
                jurisdiction_code: 3001,
                reporting_period: 11,
                last_checked_at: 20,
                last_payment_at: 10,
                outstanding_tax: 0,
                reporting_submitted: true,
                legal_documents_verified: true,
                clearance_expiry: 100,
                violation_count: 0,
            },
        )
        .expect("sync");

    let records = registry.get_tax_records(account, 10);
    let audit = registry.get_tax_audit_trail(account, 10);

    assert_eq!(records.len(), 1);
    assert_eq!(audit.len(), 1);
    assert_eq!(records[0].jurisdiction_code, 3001);
    assert_eq!(audit[0].reporting_period, 11);
}
