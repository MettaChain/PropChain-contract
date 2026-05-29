#![cfg(feature = "disabled_test")]

use ink::env::test;
use ink::env::DefaultEnvironment;
use tax_compliance::{
    Jurisdiction, JurisdictionProfile, ReportingFrequency, TaxComplianceModule, TaxRule,
};

fn us_jurisdiction() -> Jurisdiction {
    Jurisdiction {
        code: 1001,
        country_code: *b"US",
        region_code: 6,  // California
        locality_code: 37, // Los Angeles
    }
}

fn eu_jurisdiction() -> Jurisdiction {
    Jurisdiction {
        code: 2001,
        country_code: *b"DE",
        region_code: 2,  // Bavaria
        locality_code: 16, // Munich
    }
}

fn asia_jurisdiction() -> Jurisdiction {
    Jurisdiction {
        code: 3001,
        country_code: *b"SG",
        region_code: 0,
        locality_code: 0,
    }
}

#[ink::test]
fn us_property_tax_calculation() {
    let mut contract = TaxComplianceModule::new(None);
    let owner = ink::primitives::AccountId::from([0x10; 32]);
    test::set_block_timestamp::<DefaultEnvironment>(100);

    // Configure US tax rule
    contract
        .configure_tax_rule(
            us_jurisdiction(),
            TaxRule {
                rate_basis_points: 300, // 3%
                fixed_charge: 500,
                exemption_amount: 50_000,
                payment_due_period: 90 * 24 * 60 * 60 * 1000,
                reporting_frequency: ReportingFrequency::Annual,
                penalty_basis_points: 500,
                requires_reporting: true,
                requires_legal_documents: true,
                active: true,
            },
        )
        .expect("rule");

    // Configure US profile
    contract
        .configure_jurisdiction_profile(
            us_jurisdiction(),
            JurisdictionProfile {
                surcharge_basis_points: 100, // 1% local surcharge
                early_payment_discount_basis_points: 150,
                late_payment_grace_period: 30 * 24 * 60 * 60 * 1000,
                optimization_window: 10_000,
                requires_digital_stamp: false,
                authority_hash: [1u8; 32],
            },
        )
        .expect("profile");

    contract
        .set_property_assessment(100, us_jurisdiction(), owner, 500_000, 0)
        .expect("assessment");

    let record = contract.calculate_tax(100, us_jurisdiction()).expect("tax");
    let breakdown = contract
        .calculate_tax_breakdown(100, 1001, record.reporting_period)
        .expect("breakdown");

    // Taxable value: 500,000 - 50,000 = 450,000
    assert_eq!(record.taxable_value, 450_000);
    // Base tax: 450,000 * 3% = 13,500
    assert_eq!(breakdown.base_tax, 13_500);
    // Surcharge: 13,500 * 1% = 135
    assert_eq!(breakdown.surcharge_amount, 135);
}

#[ink::test]
fn eu_property_tax_calculation() {
    let mut contract = TaxComplianceModule::new(None);
    let owner = ink::primitives::AccountId::from([0x20; 32]);
    test::set_block_timestamp::<DefaultEnvironment>(100);

    contract
        .configure_tax_rule(
            eu_jurisdiction(),
            TaxRule {
                rate_basis_points: 200, // 2%
                fixed_charge: 200,
                exemption_amount: 30_000,
                payment_due_period: 60 * 24 * 60 * 60 * 1000,
                reporting_frequency: ReportingFrequency::Annual,
                penalty_basis_points: 400,
                requires_reporting: true,
                requires_legal_documents: true,
                active: true,
            },
        )
        .expect("rule");

    contract
        .configure_jurisdiction_profile(
            eu_jurisdiction(),
            JurisdictionProfile {
                surcharge_basis_points: 50, // 0.5%
                early_payment_discount_basis_points: 200, // 2%
                late_payment_grace_period: 15 * 24 * 60 * 60 * 1000,
                optimization_window: 10_000,
                requires_digital_stamp: true,
                authority_hash: [2u8; 32],
            },
        )
        .expect("profile");

    contract
        .set_property_assessment(200, eu_jurisdiction(), owner, 350_000, 0)
        .expect("assessment");

    let record = contract.calculate_tax(200, eu_jurisdiction()).expect("tax");
    let breakdown = contract
        .calculate_tax_breakdown(200, 2001, record.reporting_period)
        .expect("breakdown");

    // Taxable value: 350,000 - 30,000 = 320,000
    assert_eq!(record.taxable_value, 320_000);
    // Base tax: 320,000 * 2% = 6,400
    assert_eq!(breakdown.base_tax, 6_400);
    // Surcharge: 6,400 * 0.5% = 32
    assert_eq!(breakdown.surcharge_amount, 32);
}

#[ink::test]
fn asia_property_tax_calculation() {
    let mut contract = TaxComplianceModule::new(None);
    let owner = ink::primitives::AccountId::from([0x30; 32]);
    test::set_block_timestamp::<DefaultEnvironment>(100);

    contract
        .configure_tax_rule(
            asia_jurisdiction(),
            TaxRule {
                rate_basis_points: 400, // 4%
                fixed_charge: 300,
                exemption_amount: 20_000,
                payment_due_period: 60 * 24 * 60 * 60 * 1000,
                reporting_frequency: ReportingFrequency::Annual,
                penalty_basis_points: 600,
                requires_reporting: true,
                requires_legal_documents: true,
                active: true,
            },
        )
        .expect("rule");

    contract
        .configure_jurisdiction_profile(
            asia_jurisdiction(),
            JurisdictionProfile {
                surcharge_basis_points: 150, // 1.5%
                early_payment_discount_basis_points: 100, // 1%
                late_payment_grace_period: 20 * 24 * 60 * 60 * 1000,
                optimization_window: 10_000,
                requires_digital_stamp: true,
                authority_hash: [3u8; 32],
            },
        )
        .expect("profile");

    contract
        .set_property_assessment(300, asia_jurisdiction(), owner, 800_000, 0)
        .expect("assessment");

    let record = contract.calculate_tax(300, asia_jurisdiction()).expect("tax");
    let breakdown = contract
        .calculate_tax_breakdown(300, 3001, record.reporting_period)
        .expect("breakdown");

    // Taxable value: 800,000 - 20,000 = 780,000
    assert_eq!(record.taxable_value, 780_000);
    // Base tax: 780,000 * 4% = 31,200
    assert_eq!(breakdown.base_tax, 31_200);
    // Surcharge: 31,200 * 1.5% = 468
    assert_eq!(breakdown.surcharge_amount, 468);
}
