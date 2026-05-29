use crate::{Jurisdiction, JurisdictionProfile, ReportingFrequency, TaxRule};

/// US Federal tax rule configuration
/// - Property tax rate: ~3% (varies by state)
/// - Homestead exemption: $50,000
/// - Annual reporting
/// - 90-day payment window
pub fn us_federal_rule() -> TaxRule {
    TaxRule {
        rate_basis_points: 300, // 3% property tax
        fixed_charge: 500,
        exemption_amount: 50_000, // Homestead exemption
        payment_due_period: 90 * 24 * 60 * 60 * 1000, // 90 days
        reporting_frequency: ReportingFrequency::Annual,
        penalty_basis_points: 500, // 5% penalty
        requires_reporting: true,
        requires_legal_documents: true,
        active: true,
    }
}

/// US jurisdiction profile
/// - Local surcharge: 1%
/// - Early payment discount: 1.5%
/// - 30-day grace period
pub fn us_federal_profile() -> JurisdictionProfile {
    JurisdictionProfile {
        surcharge_basis_points: 100, // 1% local surcharge
        early_payment_discount_basis_points: 150, // 1.5% early payment discount
        late_payment_grace_period: 30 * 24 * 60 * 60 * 1000, // 30 days grace
        optimization_window: 60 * 24 * 60 * 60 * 1000, // 60 days for early payment
        requires_digital_stamp: false,
        authority_hash: [0u8; 32],
    }
}

/// EU Standard tax rule configuration
/// - Property tax rate: ~2% (varies by country)
/// - Standard exemption: €30,000
/// - Annual reporting
/// - 60-day payment window (stricter)
pub fn eu_standard_rule() -> TaxRule {
    TaxRule {
        rate_basis_points: 200, // 2% property tax (varies by country)
        fixed_charge: 200,
        exemption_amount: 30_000,
        payment_due_period: 60 * 24 * 60 * 60 * 1000, // 60 days
        reporting_frequency: ReportingFrequency::Annual,
        penalty_basis_points: 400, // 4% penalty
        requires_reporting: true,
        requires_legal_documents: true,
        active: true,
    }
}

/// EU jurisdiction profile
/// - Municipal surcharge: 0.5%
/// - Early payment discount: 2%
/// - 15-day grace period (stricter)
/// - Digital stamp required (GDPR compliance)
pub fn eu_standard_profile() -> JurisdictionProfile {
    JurisdictionProfile {
        surcharge_basis_points: 50, // 0.5% municipal surcharge
        early_payment_discount_basis_points: 200, // 2% GDPR-compliant early payment
        late_payment_grace_period: 15 * 24 * 60 * 60 * 1000, // 15 days (stricter)
        optimization_window: 45 * 24 * 60 * 60 * 1000,
        requires_digital_stamp: true, // EU digital compliance
        authority_hash: [0u8; 32],
    }
}

/// Asia Standard tax rule configuration
/// - Property tax rate: ~4% (varies: Singapore 0-4%, Malaysia 0.5-2%, etc.)
/// - Standard exemption: $20,000
/// - Annual reporting
/// - 60-day payment window
pub fn asia_standard_rule() -> TaxRule {
    TaxRule {
        rate_basis_points: 400, // 4% (varies by country)
        fixed_charge: 300,
        exemption_amount: 20_000,
        payment_due_period: 60 * 24 * 60 * 60 * 1000,
        reporting_frequency: ReportingFrequency::Annual,
        penalty_basis_points: 600, // 6% penalty (stricter enforcement)
        requires_reporting: true,
        requires_legal_documents: true,
        active: true,
    }
}

/// Asia jurisdiction profile
/// - Local development charge: 1.5%
/// - Early payment discount: 1%
/// - 20-day grace period
/// - Digital stamp required
pub fn asia_standard_profile() -> JurisdictionProfile {
    JurisdictionProfile {
        surcharge_basis_points: 150, // 1.5% local development charge
        early_payment_discount_basis_points: 100, // 1% early payment
        late_payment_grace_period: 20 * 24 * 60 * 60 * 1000,
        optimization_window: 50 * 24 * 60 * 60 * 1000,
        requires_digital_stamp: true,
        authority_hash: [0u8; 32],
    }
}

/// Helper function to get jurisdiction code from country code
pub fn jurisdiction_from_country(country: &[u8; 2]) -> Jurisdiction {
    match country {
        b"US" => Jurisdiction {
            code: 1001,
            country_code: *b"US",
            region_code: 0,
            locality_code: 0,
        },
        b"DE" | b"FR" | b"IT" | b"ES" | b"NL" => Jurisdiction {
            // EU countries
            code: 2001,
            country_code: *country,
            region_code: 0,
            locality_code: 0,
        },
        b"SG" | b"MY" | b"TH" | b"JP" | b"KR" => Jurisdiction {
            // Asian countries
            code: 3001,
            country_code: *country,
            region_code: 0,
            locality_code: 0,
        },
        _ => Jurisdiction {
            code: 9999,
            country_code: *country,
            region_code: 0,
            locality_code: 0,
        },
    }
}
