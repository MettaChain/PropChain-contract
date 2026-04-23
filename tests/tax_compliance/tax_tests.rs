#![cfg(feature = "disabled_test")]

use ink::env::test;
use ink::env::DefaultEnvironment;
use tax_compliance::{
    Jurisdiction, JurisdictionProfile, ReportingFrequency, TaxComplianceModule, TaxRule,
};

fn jurisdiction() -> Jurisdiction {
    Jurisdiction {
        code: 2001,
        country_code: *b"NG",
        region_code: 10,
        locality_code: 22,
    }
}

fn rule() -> TaxRule {
    TaxRule {
        rate_basis_points: 300,
        fixed_charge: 500,
        exemption_amount: 5_000,
        payment_due_period: 30 * 24 * 60 * 60 * 1_000,
        reporting_frequency: ReportingFrequency::Annual,
        penalty_basis_points: 200,
        requires_reporting: true,
        requires_legal_documents: true,
        active: true,
    }
}

#[ink::test]
fn tax_engine_applies_profile_adjustments() {
    let mut contract = TaxComplianceModule::new(None);
    let owner = ink::primitives::AccountId::from([0x21; 32]);
    test::set_block_timestamp::<DefaultEnvironment>(100);

    contract
        .configure_jurisdiction_profile(
            jurisdiction(),
            JurisdictionProfile {
                surcharge_basis_points: 100,
                early_payment_discount_basis_points: 150,
                late_payment_grace_period: 0,
                optimization_window: 10_000,
                requires_digital_stamp: true,
                authority_hash: [1u8; 32],
            },
        )
        .expect("profile");
    contract
        .configure_tax_rule(jurisdiction(), rule())
        .expect("rule");
    contract
        .set_property_assessment(1, jurisdiction(), owner, 100_000, 0)
        .expect("assessment");

    let record = contract.calculate_tax(1, jurisdiction()).expect("tax");
    let breakdown = contract
        .calculate_tax_breakdown(1, jurisdiction().code, record.reporting_period)
        .expect("breakdown");

    assert_eq!(breakdown.base_tax, 2_850);
    assert_eq!(breakdown.surcharge_amount, 28);
    assert_eq!(record.tax_due, 3_336);
}
