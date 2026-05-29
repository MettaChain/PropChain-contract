#![cfg(feature = "disabled_test")]

use ink::env::test;
use ink::env::DefaultEnvironment;
use tax_compliance::{
    Jurisdiction, LegalDocumentStatus, LegalDocumentType, ReportingFrequency, TaxComplianceModule,
    TaxRule,
};

fn jurisdiction() -> Jurisdiction {
    Jurisdiction {
        code: 2002,
        country_code: *b"US",
        region_code: 4,
        locality_code: 9,
    }
}

#[ink::test]
fn legal_document_verification_updates_assessment_state() {
    let mut contract = TaxComplianceModule::new(None);
    let owner = ink::primitives::AccountId::from([0x31; 32]);
    test::set_block_timestamp::<DefaultEnvironment>(2_000);

    contract
        .configure_tax_rule(
            jurisdiction(),
            TaxRule {
                rate_basis_points: 250,
                fixed_charge: 0,
                exemption_amount: 0,
                payment_due_period: 1_000,
                reporting_frequency: ReportingFrequency::Annual,
                penalty_basis_points: 100,
                requires_reporting: false,
                requires_legal_documents: true,
                active: true,
            },
        )
        .expect("rule");
    contract
        .set_property_assessment(2, jurisdiction(), owner, 50_000, 0)
        .expect("assessment");
    contract
        .upsert_legal_document(
            2,
            jurisdiction(),
            LegalDocumentType::TitleDeed,
            [2u8; 32],
            100,
            3_000,
            true,
        )
        .expect("document");

    let document = contract
        .get_legal_document(2, jurisdiction().code, LegalDocumentType::TitleDeed)
        .expect("stored");
    let assessment = contract
        .get_property_assessment(2, jurisdiction().code)
        .expect("assessment");

    assert_eq!(document.status, LegalDocumentStatus::Verified);
    assert!(assessment.legal_documents_verified);
}
