#![cfg_attr(not(feature = "std"), no_std, no_main)]

use ink::prelude::vec::Vec;
use ink::storage::Mapping;
use propchain_contracts::{non_reentrant, ReentrancyError, ReentrancyGuard};
use propchain_traits::ComplianceChecker;
use propchain_traits::*;

#[ink::contract]
mod tax_compliance {
    use super::*;
    mod tax_engine;
    mod jurisdiction_presets;

    const BASIS_POINTS_DENOMINATOR: Balance = 10_000;


    #[derive(Debug, Clone, Copy, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(
        feature = "std",
        derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
    )]
    pub enum ReportingFrequency {
        Monthly,
        Quarterly,
        Annual,
    }

    impl ReportingFrequency {
        fn period_millis(&self) -> u64 {
            match self {
                Self::Monthly => 30 * 24 * 60 * 60 * 1000,
                Self::Quarterly => 90 * 24 * 60 * 60 * 1000,
                Self::Annual => 365 * 24 * 60 * 60 * 1000,
            }
        }
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(
        feature = "std",
        derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
    )]
    pub enum RegionType {
        US,
        EU,
        Asia,
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(
        feature = "std",
        derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
    )]
    pub struct JurisdictionProfile {
        pub surcharge_basis_points: u32,
        pub early_payment_discount_basis_points: u32,
        pub late_payment_grace_period: u64,
        pub optimization_window: u64,
        pub requires_digital_stamp: bool,
        pub authority_hash: [u8; 32],
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(
        feature = "std",
        derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
    )]
    pub struct TaxBreakdown {
        pub taxable_value: Balance,
        pub base_tax: Balance,
        pub fixed_charge: Balance,
        pub surcharge_amount: Balance,
        pub discount_amount: Balance,
        pub penalty_amount: Balance,
        pub total_due: Balance,
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(
        feature = "std",
        derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
    )]
    pub struct OptimizationPlan {
        pub estimated_savings: Balance,
        pub recommended_installments: u32,
        pub should_prepay: bool,
        pub review_exemption: bool,
        pub supporting_reference: [u8; 32],
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(
        feature = "std",
        derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
    )]
    pub struct PaymentReceipt {
        pub property_id: u64,
        pub jurisdiction_code: u32,
        pub reporting_period: u64,
        pub payment_reference: [u8; 32],
        pub amount_paid: Balance,
        pub outstanding_balance: Balance,
        pub settled_at: Timestamp,
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(
        feature = "std",
        derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
    )]
    pub enum LegalDocumentType {
        TitleDeed,
        TaxClearance,
        OwnershipTransfer,
        Mortgage,
        Other,
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(
        feature = "std",
        derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
    )]
    pub enum LegalDocumentStatus {
        Pending,
        Verified,
        Rejected,
        Expired,
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(
        feature = "std",
        derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
    )]
    pub enum ComplianceAlertType {
        RegistryNonCompliant,
        TaxOverdue,
        PaymentDueSoon,
        ReportingMissing,
        LegalDocumentsMissing,
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(
        feature = "std",
        derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
    )]
    pub enum ComplianceAlertLevel {
        Info,
        Warning,
        Critical,
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(
        feature = "std",
        derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
    )]
    pub struct ComplianceAlert {
        pub property_id: u64,
        pub jurisdiction_code: u32,
        pub reporting_period: u64,
        pub alert_type: ComplianceAlertType,
        pub level: ComplianceAlertLevel,
        pub outstanding_tax: Balance,
        pub due_at: Timestamp,
        pub triggered_at: Timestamp,
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(
        feature = "std",
        derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
    )]
    pub struct TaxRule {
        pub rate_basis_points: u32,
        pub fixed_charge: Balance,
        pub exemption_amount: Balance,
        pub payment_due_period: u64,
        pub reporting_frequency: ReportingFrequency,
        pub penalty_basis_points: u32,
        pub requires_reporting: bool,
        pub requires_legal_documents: bool,
        pub withholding_rate_basis_points: u32,
        pub tax_collector: AccountId,
        pub active: bool,
    }

    /// A bilateral tax treaty between two jurisdictions that reduces the effective
    /// tax rate for cross-border transactions.
    /// `reduction_basis_points` is the percentage-point reduction applied to the
    /// computed `tax_due` (e.g. 2000 = 20 % reduction).
    #[derive(Debug, Clone, Copy, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(
        feature = "std",
        derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
    )]
    pub struct TaxTreaty {
        /// Jurisdiction code of the first party (source country).
        pub jurisdiction_a: u32,
        /// Jurisdiction code of the second party (residence country).
        pub jurisdiction_b: u32,
        /// Reduction applied to the computed tax, in basis points (max 10 000).
        pub reduction_basis_points: u32,
        /// Whether this treaty is currently active.
        pub active: bool,
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(
        feature = "std",
        derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
    )]
    pub struct PropertyAssessment {
        pub owner: AccountId,
        pub assessed_value: Balance,
        pub exemption_override: Balance,
        pub last_assessed_at: Timestamp,
        pub legal_documents_verified: bool,
        pub reporting_submitted: bool,
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(
        feature = "std",
        derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
    )]
    pub enum TaxStatus {
        Assessed,
        PartiallyPaid,
        Paid,
        Overdue,
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(
        feature = "std",
        derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
    )]
    pub struct TaxRecord {
        pub property_id: u64,
        pub jurisdiction_code: u32,
        pub reporting_period: u64,
        pub assessed_value: Balance,
        pub taxable_value: Balance,
        pub tax_due: Balance,
        pub paid_amount: Balance,
        pub penalty_amount: Balance,
        pub discount_amount: Balance,
        pub due_at: Timestamp,
        pub last_payment_at: Timestamp,
        pub status: TaxStatus,
        pub payment_reference: [u8; 32],
        pub report_hash: [u8; 32],
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(
        feature = "std",
        derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
    )]
    pub enum AuditAction {
        RuleConfigured,
        AssessmentUpdated,
        TaxCalculated,
        TaxPaid,
        ReportingSubmitted,
        LegalDocumentUpdated,
        ComplianceChecked,
        ComplianceViolation,
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(
        feature = "std",
        derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
    )]
    pub struct AuditEntry {
        pub action: AuditAction,
        pub property_id: u64,
        pub jurisdiction_code: u32,
        pub reporting_period: u64,
        pub actor: AccountId,
        pub timestamp: Timestamp,
        pub amount: Balance,
        pub reference_hash: [u8; 32],
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(
        feature = "std",
        derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
    )]
    pub struct ComplianceSnapshot {
        pub property_id: u64,
        pub jurisdiction_code: u32,
        pub reporting_period: u64,
        pub registry_compliant: bool,
        pub tax_current: bool,
        pub outstanding_tax: Balance,
        pub reporting_submitted: bool,
        pub legal_documents_verified: bool,
        pub active_alerts: u32,
        pub status: TaxStatus,
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(
        feature = "std",
        derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
    )]
    pub enum TaxLossHarvestingOpportunityKind {
        Reassessment,
        ExemptionReview,
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(
        feature = "std",
        derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
    )]
    pub struct TaxLossHarvestingOpportunity {
        pub property_id: u64,
        pub jurisdiction_code: u32,
        pub reporting_period: u64,
        pub kind: TaxLossHarvestingOpportunityKind,
        pub estimated_savings: Balance,
        pub current_tax_due: Balance,
        pub revised_tax_due: Balance,
    }

    #[derive(Debug, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub enum Error {
        Unauthorized,
        RuleNotFound,
        AssessmentNotFound,
        RecordNotFound,
        InactiveRule,
        InvalidRate,
        ReentrantCall,
        TreatyNotFound,
    }

    impl From<ReentrancyError> for Error {
        fn from(_: ReentrancyError) -> Self {
            Error::ReentrantCall
        }
    }

    impl core::fmt::Display for Error {
        fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
            match self {
                Self::Unauthorized => write!(f, "Caller is not authorized"),
                Self::RuleNotFound => write!(f, "Tax rule not found"),
                Self::AssessmentNotFound => write!(f, "Property assessment not found"),
                Self::RecordNotFound => write!(f, "Tax record not found"),
                Self::InactiveRule => write!(f, "Tax rule is inactive"),
                Self::InvalidRate => write!(f, "Tax configuration is invalid"),
                Self::ReentrantCall => write!(f, "Reentrant call"),
                Self::TreatyNotFound => write!(f, "Tax treaty not found"),
            }
        }
    }

    impl ContractError for Error {
        fn error_code(&self) -> u32 {
            match self {
                Self::Unauthorized => {
                    propchain_traits::errors::compliance_codes::COMPLIANCE_UNAUTHORIZED
                }
                Self::RuleNotFound => {
                    propchain_traits::errors::compliance_codes::COMPLIANCE_CHECK_FAILED
                }
                Self::AssessmentNotFound => {
                    propchain_traits::errors::compliance_codes::COMPLIANCE_CHECK_FAILED
                }
                Self::RecordNotFound => {
                    propchain_traits::errors::compliance_codes::COMPLIANCE_CHECK_FAILED
                }
                Self::InactiveRule => {
                    propchain_traits::errors::compliance_codes::COMPLIANCE_CHECK_FAILED
                }
                Self::InvalidRate => {
                    propchain_traits::errors::compliance_codes::COMPLIANCE_CHECK_FAILED
                }
                Self::ReentrantCall => propchain_traits::errors::compliance_codes::REENTRANT_CALL,
                Self::TreatyNotFound => {
                    propchain_traits::errors::compliance_codes::COMPLIANCE_CHECK_FAILED
                }
            }
        }

        fn error_description(&self) -> &'static str {
            match self {
                Self::Unauthorized => {
                    "Caller does not have permission to manage tax compliance state"
                }
                Self::RuleNotFound => "No tax rule was configured for the requested jurisdiction",
                Self::AssessmentNotFound => {
                    "No property assessment is available for the requested jurisdiction"
                }
                Self::RecordNotFound => "No tax record exists for the requested reporting period",
                Self::InactiveRule => "The tax rule for the requested jurisdiction is inactive",
                Self::InvalidRate => {
                    "The configured tax rate exceeds the supported deterministic bounds"
                }
                Self::ReentrantCall => "Reentrancy guard detected a reentrant call",
                Self::TreatyNotFound => "No tax treaty was configured for the requested jurisdiction pair",
            }
        }

        fn error_category(&self) -> ErrorCategory {
            ErrorCategory::Compliance
        }
    }

    pub type Result<T> = core::result::Result<T, Error>;

    #[ink(event)]
    pub struct TaxCalculated {
        #[ink(topic)]
        property_id: u64,
        #[ink(topic)]
        jurisdiction_code: u32,
        reporting_period: u64,
        tax_due: Balance,
    }

    #[ink(event)]
    pub struct TaxPaid {
        #[ink(topic)]
        property_id: u64,
        #[ink(topic)]
        jurisdiction_code: u32,
        reporting_period: u64,
        amount: Balance,
        outstanding_tax: Balance,
    }

    #[ink(event)]
    pub struct ComplianceViolation {
        #[ink(topic)]
        property_id: u64,
        #[ink(topic)]
        jurisdiction_code: u32,
        reporting_period: u64,
        outstanding_tax: Balance,
        registry_compliant: bool,
    }

    #[ink(event)]
    pub struct ReportingHookTriggered {
        #[ink(topic)]
        property_id: u64,
        #[ink(topic)]
        jurisdiction_code: u32,
        reporting_period: u64,
        report_hash: [u8; 32],
    }

    #[ink(event)]
    pub struct LegalDocumentHookTriggered {
        #[ink(topic)]
        property_id: u64,
        #[ink(topic)]
        jurisdiction_code: u32,
        document_hash: [u8; 32],
        verified: bool,
    }

#[ink(event)]
pub struct ComplianceRegistrySyncRequested {
    #[ink(topic)]
    property_id: u64,
    #[ink(topic)]
    jurisdiction_code: u32,
    reporting_period: u64,
    outstanding_tax: Balance,
    legal_documents_verified: bool,
    reporting_submitted: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, scale::Encode, scale::Decode)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub enum DeadlineAlertLevel {
    Approaching,
    Urgent,
}

#[ink(event)]
pub struct TaxDeadlineApproaching {
    #[ink(topic)]
    property_id: u64,
    #[ink(topic)]
    jurisdiction_code: u32,
    reporting_period: u64,
    due_at: Timestamp,
    days_remaining: u16,
    alert_level: DeadlineAlertLevel,
}

#[ink(event)]
pub struct TaxDeadlineNotification {
    #[ink(topic)]
    property_id: u64,
    #[ink(topic)]
    jurisdiction_code: u32,
    reporting_period: u64,
    due_at: Timestamp,
    days_remaining: u16,
}

    #[ink(event)]
    pub struct TaxDocumentUploaded {
        #[ink(topic)]
        property_id: u64,
        #[ink(topic)]
        jurisdiction_code: u32,
        reporting_period: u64,
        document_index: u64,
        document_type: DocumentType,
        ipfs_hash: [u8; 32],
        uploaded_by: AccountId,
    }

    #[ink(event)]
    pub struct TaxWithheld {
        #[ink(topic)]
        pub property_id: u64,
        #[ink(topic)]
        pub jurisdiction_code: u32,
        pub amount: Balance,
        pub collector: AccountId,
    }

    #[ink(event)]
    pub struct TaxDocumentVerified {
        #[ink(topic)]
        property_id: u64,
        #[ink(topic)]
        jurisdiction_code: u32,
        reporting_period: u64,
        document_index: u64,
        verified_by: AccountId,
    }

    #[ink(event)]
    pub struct TaxAdvisorRegistered {
        #[ink(topic)]
        advisor_id: AccountId,
        license_number: [u8; 32],
        jurisdiction_codes: Vec<u32>,
    }

    #[ink(event)]
    pub struct TaxAdvisorAssigned {
        #[ink(topic)]
        advisor_id: AccountId,
        #[ink(topic)]
        property_id: u64,
    }

    /// Emitted when a tax treaty is created or updated
    #[ink(event)]
    pub struct TaxTreatyConfigured {
        #[ink(topic)]
        jurisdiction_a: u32,
        #[ink(topic)]
        jurisdiction_b: u32,
        reduction_basis_points: u32,
        active: bool,
    }

    #[derive(Debug, Clone, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(
        feature = "std",
        derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
    )]
    pub struct TaxDocument {
        pub property_id: u64,
        pub jurisdiction_code: u32,
        pub reporting_period: u64,
        pub document_type: DocumentType,
        pub ipfs_hash: [u8; 32],
        pub uploaded_by: AccountId,
        pub uploaded_at: Timestamp,
        pub verified: bool,
        pub verified_by: Option<AccountId>,
        pub verified_at: Option<Timestamp>,
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(
        feature = "std",
        derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
    )]
    pub enum DocumentType {
        TaxReturn,
        PaymentReceipt,
        AssessmentReport,
        ExemptionCertificate,
        ComplianceReport,
        Other,
    }

    #[derive(Debug, Clone, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(
        feature = "std",
        derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
    )]
    pub struct TaxAdvisor {
        pub advisor_id: AccountId,
        pub name: [u8; 64],
        pub license_number: [u8; 32],
        pub jurisdiction_codes: Vec<u32>,
        pub is_active: bool,
        pub registered_at: Timestamp,
    }

    #[ink(storage)]
    pub struct TaxComplianceModule {
        admin: AccountId,
        compliance_registry: Option<AccountId>,
        reentrancy_guard: ReentrancyGuard,
        tax_rules: Mapping<u32, TaxRule>,
        jurisdiction_profiles: Mapping<u32, JurisdictionProfile>,
        property_assessments: Mapping<(u64, u32), PropertyAssessment>,
        #[allow(clippy::type_complexity)]
        tax_records: Mapping<(u64, u32, u64), TaxRecord>,
        latest_reporting_period: Mapping<(u64, u32), u64>,
        audit_logs: Mapping<(u64, u64), AuditEntry>,
        audit_log_count: Mapping<u64, u64>,
        tax_documents: Mapping<(u64, u32, u64, u64), TaxDocument>,
        tax_document_count: Mapping<(u64, u32, u64), u64>,
        tax_advisors: Mapping<AccountId, TaxAdvisor>,
        advisor_property_assignments: Mapping<(AccountId, u64), bool>,
        /// Tax treaties keyed by (min(a,b), max(a,b)) for canonical ordering
        tax_treaties: Mapping<(u32, u32), TaxTreaty>,
    }

    impl TaxComplianceModule {
        #[ink(constructor)]
        pub fn new(compliance_registry: Option<AccountId>) -> Self {
            Self {
                admin: Self::env().caller(),
                compliance_registry,
                reentrancy_guard: ReentrancyGuard::new(),
                tax_rules: Mapping::default(),
                jurisdiction_profiles: Mapping::default(),
                property_assessments: Mapping::default(),
                tax_records: Mapping::default(),
                latest_reporting_period: Mapping::default(),
                audit_logs: Mapping::default(),
                audit_log_count: Mapping::default(),
                tax_documents: Mapping::default(),
                tax_document_count: Mapping::default(),
                tax_advisors: Mapping::default(),
                advisor_property_assignments: Mapping::default(),
                tax_treaties: Mapping::default(),
            }
        }

        #[ink(message)]
        pub fn set_compliance_registry(&mut self, registry: Option<AccountId>) -> Result<()> {
            self.ensure_admin()?;
            self.compliance_registry = registry;
            Ok(())
        }

        #[ink(message)]
        pub fn configure_tax_rule(
            &mut self,
            jurisdiction: Jurisdiction,
            rule: TaxRule,
        ) -> Result<()> {
            self.ensure_admin()?;
            if rule.rate_basis_points > BASIS_POINTS_DENOMINATOR as u32 {
                return Err(Error::InvalidRate);
            }
            self.tax_rules.insert(jurisdiction.code, &rule);
            self.log_audit(
                0,
                jurisdiction.code,
                0,
                AuditAction::RuleConfigured,
                0,
                [0u8; 32],
            );
            Ok(())
        }

        #[ink(message)]
        pub fn configure_jurisdiction_profile(
            &mut self,
            jurisdiction: Jurisdiction,
            profile: JurisdictionProfile,
        ) -> Result<()> {
            self.ensure_admin()?;
            self.jurisdiction_profiles.insert(jurisdiction.code, &profile);
            self.log_audit(
                0,
                jurisdiction.code,
                0,
                AuditAction::RuleConfigured,
                0,
                profile.authority_hash,
            );
            Ok(())
        }

        #[ink(message)]
        pub fn initialize_jurisdiction_presets(&mut self, region: RegionType) -> Result<()> {
            self.ensure_admin()?;

            match region {
                RegionType::US => {
                    let jurisdiction = jurisdiction_presets::jurisdiction_from_country(b"US");
                    self.tax_rules.insert(jurisdiction.code, &jurisdiction_presets::us_federal_rule());
                    self.jurisdiction_profiles.insert(jurisdiction.code, &jurisdiction_presets::us_federal_profile());
                }
                RegionType::EU => {
                    let jurisdiction = jurisdiction_presets::jurisdiction_from_country(b"DE");
                    self.tax_rules.insert(jurisdiction.code, &jurisdiction_presets::eu_standard_rule());
                    self.jurisdiction_profiles.insert(jurisdiction.code, &jurisdiction_presets::eu_standard_profile());
                }
                RegionType::Asia => {
                    let jurisdiction = jurisdiction_presets::jurisdiction_from_country(b"SG");
                    self.tax_rules.insert(jurisdiction.code, &jurisdiction_presets::asia_standard_rule());
                    self.jurisdiction_profiles.insert(jurisdiction.code, &jurisdiction_presets::asia_standard_profile());
                }
            }

            Ok(())
        }

        #[ink(message)]
        pub fn set_property_assessment(
            &mut self,
            property_id: u64,
            jurisdiction: Jurisdiction,
            owner: AccountId,
            assessed_value: Balance,
            exemption_override: Balance,
        ) -> Result<()> {
            self.ensure_admin()?;
            let assessment = PropertyAssessment {
                owner,
                assessed_value,
                exemption_override,
                last_assessed_at: self.env().block_timestamp(),
                legal_documents_verified: false,
                reporting_submitted: false,
            };
            self.property_assessments
                .insert((property_id, jurisdiction.code), &assessment);
            self.log_audit(
                property_id,
                jurisdiction.code,
                0,
                AuditAction::AssessmentUpdated,
                assessed_value,
                [0u8; 32],
            );
            Ok(())
        }

        #[ink(message)]
        pub fn calculate_tax(
            &mut self,
            property_id: u64,
            jurisdiction: Jurisdiction,
            residence_jurisdiction_code: Option<u32>,
        ) -> Result<TaxRecord> {
            non_reentrant!(self, {
                self.ensure_admin()?;
                let now = self.env().block_timestamp();
                let rule = self.get_active_rule(jurisdiction.code)?;
                let assessment = self
                    .property_assessments
                    .get((property_id, jurisdiction.code))
                    .ok_or(Error::AssessmentNotFound)?;
                let reporting_period = self.reporting_period(now, rule.reporting_frequency);
                let existing =
                    self.tax_records
                        .get((property_id, jurisdiction.code, reporting_period));
                let combined_exemption = rule
                    .exemption_amount
                    .saturating_add(assessment.exemption_override);
                let taxable_value = assessment.assessed_value.saturating_sub(combined_exemption);
                let base_tax = taxable_value.saturating_mul(rule.rate_basis_points as Balance)
                    / BASIS_POINTS_DENOMINATOR;
                let gross_tax = base_tax.saturating_add(rule.fixed_charge);
                // Apply treaty reduction if a residence jurisdiction is provided and an
                // active treaty exists between the two jurisdictions.
                let treaty_reduction = residence_jurisdiction_code
                    .and_then(|res| {
                        self.tax_treaties
                            .get(Self::treaty_key(jurisdiction.code, res))
                    })
                    .filter(|t| t.active)
                    .map(|t| {
                        gross_tax.saturating_mul(t.reduction_basis_points as Balance)
                            / BASIS_POINTS_DENOMINATOR
                    })
                    .unwrap_or(0);
                let tax_due = gross_tax.saturating_sub(treaty_reduction);
                let mut record = TaxRecord {                    property_id,
                    jurisdiction_code: jurisdiction.code,
                    reporting_period,
                    assessed_value: assessment.assessed_value,
                    taxable_value,
                    tax_due,
                    paid_amount: existing
                        .map(|value: TaxRecord| value.paid_amount)
                        .unwrap_or(0),
                    due_at: now.saturating_add(rule.payment_due_period),
                    last_payment_at: existing
                        .map(|value: TaxRecord| value.last_payment_at)
                        .unwrap_or(0),
                    status: TaxStatus::Assessed,
                    payment_reference: existing
                        .map(|value: TaxRecord| value.payment_reference)
                        .unwrap_or([0u8; 32]),
                    report_hash: existing
                        .map(|value: TaxRecord| value.report_hash)
                        .unwrap_or([0u8; 32]),
                };
                record.status = self.resolve_status(&record, now);
                self.tax_records
                    .insert((property_id, jurisdiction.code, reporting_period), &record);
                self.latest_reporting_period
                    .insert((property_id, jurisdiction.code), &record.reporting_period);

                self.log_audit(
                    property_id,
                    jurisdiction.code,
                    record.reporting_period,
                    AuditAction::TaxCalculated,
                    record.tax_due,
                    [0u8; 32],
                );
                self.env().emit_event(TaxCalculated {
                    property_id,
                    jurisdiction_code: jurisdiction.code,
                    reporting_period: record.reporting_period,
                    tax_due: record.tax_due,
                });

                // Emit tax deadline notification if approaching
                if let Some(days) = crate::tax_engine::days_until_due(now, record.due_at) {
                    if days <= 30 {
                        let alert_level = if days <= 7 { DeadlineAlertLevel::Urgent } else { DeadlineAlertLevel::Approaching };
                        self.env().emit_event(TaxDeadlineApproaching {
                            property_id,
                            jurisdiction_code: jurisdiction.code,
                            reporting_period,
                            due_at: record.due_at,
                            days_remaining: days,
                            alert_level,
                        });
                        self.env().emit_event(TaxDeadlineNotification {
                            property_id,
                            jurisdiction_code: jurisdiction.code,
                            reporting_period,
                            due_at: record.due_at,
                            days_remaining: days,
                        });
                    }
                }

                let snapshot = self.build_snapshot(
                    property_id,
                    jurisdiction.code,
                    &rule,
                    &assessment,
                    Some(record),
                );
                self.emit_registry_sync_requested(snapshot);

                Ok(record)
            })
        }

        #[ink(message)]
        pub fn record_tax_payment(
            &mut self,
            property_id: u64,
            jurisdiction: Jurisdiction,
            reporting_period: u64,
            amount: Balance,
            payment_reference: [u8; 32],
        ) -> Result<TaxRecord> {
            non_reentrant!(self, {
                self.ensure_admin()?;
                let now = self.env().block_timestamp();
                let rule = self.get_active_rule(jurisdiction.code)?;
                let assessment = self
                    .property_assessments
                    .get((property_id, jurisdiction.code))
                    .ok_or(Error::AssessmentNotFound)?;
                let mut record = self
                    .tax_records
                    .get((property_id, jurisdiction.code, reporting_period))
                    .ok_or(Error::RecordNotFound)?;
                record.paid_amount = record.paid_amount.saturating_add(amount);
                record.last_payment_at = now;
                record.payment_reference = payment_reference;
                record.status = self.resolve_status(&record, now);

                self.tax_records
                    .insert((property_id, jurisdiction.code, reporting_period), &record);
                self.log_audit(
                    property_id,
                    jurisdiction.code,
                    reporting_period,
                    AuditAction::TaxPaid,
                    amount,
                    payment_reference,
                );
                self.env().emit_event(TaxPaid {
                    property_id,
                    jurisdiction_code: jurisdiction.code,
                    reporting_period,
                    amount,
                    outstanding_tax: self.outstanding_tax(&record),
                });

                let snapshot = self.build_snapshot(
                    property_id,
                    jurisdiction.code,
                    &rule,
                    &assessment,
                    Some(record),
                );
                self.emit_registry_sync_requested(snapshot);

                Ok(record)
            })
        }

        #[ink(message)]
        pub fn record_reporting_submission(
            &mut self,
            property_id: u64,
            jurisdiction: Jurisdiction,
            reporting_period: u64,
            report_hash: [u8; 32],
        ) -> Result<()> {
            non_reentrant!(self, {
                self.ensure_admin()?;
                let now = self.env().block_timestamp();
                let rule = self.get_active_rule(jurisdiction.code)?;
                let mut assessment = self
                    .property_assessments
                    .get((property_id, jurisdiction.code))
                    .ok_or(Error::AssessmentNotFound)?;
                assessment.reporting_submitted = true;
                self.property_assessments
                    .insert((property_id, jurisdiction.code), &assessment);

                let mut record = self
                    .tax_records
                    .get((property_id, jurisdiction.code, reporting_period))
                    .ok_or(Error::RecordNotFound)?;
                record.report_hash = report_hash;
                record.status = self.resolve_status(&record, now);
                self.tax_records
                    .insert((property_id, jurisdiction.code, reporting_period), &record);

                self.log_audit(
                    property_id,
                    jurisdiction.code,
                    reporting_period,
                    AuditAction::ReportingSubmitted,
                    0,
                    report_hash,
                );
                self.env().emit_event(ReportingHookTriggered {
                    property_id,
                    jurisdiction_code: jurisdiction.code,
                    reporting_period,
                    report_hash,
                });

                let snapshot = self.build_snapshot(
                    property_id,
                    jurisdiction.code,
                    &rule,
                    &assessment,
                    Some(record),
                );
                self.emit_registry_sync_requested(snapshot);

                Ok(())
            })
        }

        #[ink(message)]
        pub fn record_legal_document(
            &mut self,
            property_id: u64,
            jurisdiction: Jurisdiction,
            document_hash: [u8; 32],
            verified: bool,
        ) -> Result<()> {
            non_reentrant!(self, {
                self.ensure_admin()?;
                let now = self.env().block_timestamp();
                let rule = self.get_active_rule(jurisdiction.code)?;
                let mut assessment = self
                    .property_assessments
                    .get((property_id, jurisdiction.code))
                    .ok_or(Error::AssessmentNotFound)?;
                assessment.legal_documents_verified = verified;
                self.property_assessments
                    .insert((property_id, jurisdiction.code), &assessment);

                let reporting_period = self
                    .latest_reporting_period
                    .get((property_id, jurisdiction.code))
                    .unwrap_or(self.reporting_period(now, rule.reporting_frequency));
                let record =
                    self.tax_records
                        .get((property_id, jurisdiction.code, reporting_period));

                self.log_audit(
                    property_id,
                    jurisdiction.code,
                    reporting_period,
                    AuditAction::LegalDocumentUpdated,
                    0,
                    document_hash,
                );
                self.env().emit_event(LegalDocumentHookTriggered {
                    property_id,
                    jurisdiction_code: jurisdiction.code,
                    document_hash,
                    verified,
                });

                let snapshot =
                    self.build_snapshot(property_id, jurisdiction.code, &rule, &assessment, record);
                self.emit_registry_sync_requested(snapshot);

                Ok(())
            })
        }

        #[ink(message)]
        pub fn check_compliance(
            &mut self,
            property_id: u64,
            jurisdiction: Jurisdiction,
        ) -> Result<ComplianceSnapshot> {
            self.ensure_admin()?;
            let now = self.env().block_timestamp();
            let rule = self.get_active_rule(jurisdiction.code)?;
            let assessment = self
                .property_assessments
                .get((property_id, jurisdiction.code))
                .ok_or(Error::AssessmentNotFound)?;
            let reporting_period = self
                .latest_reporting_period
                .get((property_id, jurisdiction.code))
                .unwrap_or(self.reporting_period(now, rule.reporting_frequency));
            let record = self
                .tax_records
                .get((property_id, jurisdiction.code, reporting_period));

            non_reentrant!(self, {
                let snapshot =
                    self.build_snapshot(property_id, jurisdiction.code, &rule, &assessment, record);

                // Emit tax deadline notification if approaching during compliance check
                if let Some(record) = record {
                    if let Some(days) = crate::tax_engine::days_until_due(now, record.due_at) {
                        if days <= 30 {
                            let alert_level = if days <= 7 { DeadlineAlertLevel::Urgent } else { DeadlineAlertLevel::Approaching };
                            self.env().emit_event(TaxDeadlineApproaching {
                                property_id,
                                jurisdiction_code: jurisdiction.code,
                                reporting_period: record.reporting_period,
                                due_at: record.due_at,
                                days_remaining: days,
                                alert_level,
                            });
                            self.env().emit_event(TaxDeadlineNotification {
                                property_id,
                                jurisdiction_code: jurisdiction.code,
                                reporting_period: record.reporting_period,
                                due_at: record.due_at,
                                days_remaining: days,
                            });
                        }
                    }
                }

                let mut outstanding_ref = [0u8; 32];
                outstanding_ref[16..].copy_from_slice(&snapshot.outstanding_tax.to_be_bytes());

                self.log_audit(
                    property_id,
                    jurisdiction.code,
                    snapshot.reporting_period,
                    AuditAction::ComplianceChecked,
                    snapshot.outstanding_tax,
                    outstanding_ref,
                );

                if !snapshot.registry_compliant || snapshot.outstanding_tax > 0 {
                    self.env().emit_event(ComplianceViolation {
                        property_id,
                        jurisdiction_code: jurisdiction.code,
                        reporting_period: snapshot.reporting_period,
                        outstanding_tax: snapshot.outstanding_tax,
                        registry_compliant: snapshot.registry_compliant,
                    });
                    self.log_audit(
                        property_id,
                        jurisdiction.code,
                        snapshot.reporting_period,
                        AuditAction::ComplianceViolation,
                        snapshot.outstanding_tax,
                        outstanding_ref,
                    );
                }

                Ok(snapshot)
            })
        }

        #[ink(message)]
        pub fn get_tax_rule(&self, jurisdiction_code: u32) -> Option<TaxRule> {
            self.tax_rules.get(jurisdiction_code)
        }

        /// Create or update a tax treaty between two jurisdictions.
        /// `reduction_basis_points` must not exceed 10 000 (100 %).
        #[ink(message)]
        pub fn set_tax_treaty(
            &mut self,
            jurisdiction_a: u32,
            jurisdiction_b: u32,
            reduction_basis_points: u32,
            active: bool,
        ) -> Result<()> {
            self.ensure_admin()?;
            if reduction_basis_points > BASIS_POINTS_DENOMINATOR as u32 {
                return Err(Error::InvalidRate);
            }
            let key = Self::treaty_key(jurisdiction_a, jurisdiction_b);
            let treaty = TaxTreaty {
                jurisdiction_a,
                jurisdiction_b,
                reduction_basis_points,
                active,
            };
            self.tax_treaties.insert(key, &treaty);
            self.env().emit_event(TaxTreatyConfigured {
                jurisdiction_a,
                jurisdiction_b,
                reduction_basis_points,
                active,
            });
            Ok(())
        }

        /// Retrieve the treaty between two jurisdictions, if one exists.
        #[ink(message)]
        pub fn get_tax_treaty(
            &self,
            jurisdiction_a: u32,
            jurisdiction_b: u32,
        ) -> Option<TaxTreaty> {
            self.tax_treaties
                .get(Self::treaty_key(jurisdiction_a, jurisdiction_b))
        }

        #[ink(message)]
        pub fn get_jurisdiction_profile(&self, jurisdiction_code: u32) -> Option<JurisdictionProfile> {
            self.jurisdiction_profiles.get(jurisdiction_code)
        }

        #[ink(message)]
        pub fn calculate_tax_breakdown(
            &self,
            property_id: u64,
            jurisdiction_code: u32,
            reporting_period: u64,
        ) -> Result<TaxBreakdown> {
            let rule = self.get_active_rule(jurisdiction_code)?;
            let assessment = self
                .property_assessments
                .get((property_id, jurisdiction_code))
                .ok_or(Error::AssessmentNotFound)?;
            let record = self
                .tax_records
                .get((property_id, jurisdiction_code, reporting_period))
                .ok_or(Error::RecordNotFound)?;
            let profile = self.jurisdiction_profiles.get(jurisdiction_code);
            let now = self.env().block_timestamp();

            Ok(tax_engine::build_breakdown(rule, profile, assessment, record, now))
        }

        #[ink(message)]
        pub fn get_property_assessment(
            &self,
            property_id: u64,
            jurisdiction_code: u32,
        ) -> Option<PropertyAssessment> {
            self.property_assessments
                .get((property_id, jurisdiction_code))
        }

        #[ink(message)]
        pub fn get_tax_record(
            &self,
            property_id: u64,
            jurisdiction_code: u32,
            reporting_period: u64,
        ) -> Option<TaxRecord> {
            self.tax_records
                .get((property_id, jurisdiction_code, reporting_period))
        }

        #[ink(message)]
        pub fn get_tax_loss_harvesting_opportunities(
            &self,
            property_id: u64,
            jurisdiction: Jurisdiction,
        ) -> Result<Vec<TaxLossHarvestingOpportunity>> {
            let now = self.env().block_timestamp();
            let rule = self.get_active_rule(jurisdiction.code)?;
            let assessment = self
                .property_assessments
                .get((property_id, jurisdiction.code))
                .ok_or(Error::AssessmentNotFound)?;
            let reporting_period = self
                .latest_reporting_period
                .get((property_id, jurisdiction.code))
                .unwrap_or(self.reporting_period(now, rule.reporting_frequency));
            let current_tax_due = self.estimate_tax_due(&rule, &assessment);
            let current_taxable_value = self.taxable_value(&rule, &assessment);
            let mut opportunities = Vec::new();

            if let Some(record) = self
                .tax_records
                .get((property_id, jurisdiction.code, reporting_period))
            {
                let previous_base_due = self
                    .base_tax_due(record.taxable_value, rule.rate_basis_points)
                    .saturating_add(rule.fixed_charge);
                let revised_base_due = current_tax_due;

                if assessment.assessed_value < record.assessed_value
                    || current_taxable_value < record.taxable_value
                {
                    let estimated_savings = previous_base_due.saturating_sub(revised_base_due);
                    if estimated_savings > 0 {
                        opportunities.push(TaxLossHarvestingOpportunity {
                            property_id,
                            jurisdiction_code: jurisdiction.code,
                            reporting_period,
                            kind: TaxLossHarvestingOpportunityKind::Reassessment,
                            estimated_savings,
                            current_tax_due: previous_base_due,
                            revised_tax_due: revised_base_due,
                        });
                    }
                }
            }

            let exemption_threshold = assessment.assessed_value / 20;
            if assessment.exemption_override < exemption_threshold {
                let revised_taxable_value = assessment.assessed_value.saturating_sub(
                    rule.exemption_amount
                        .saturating_add(exemption_threshold),
                );
                let revised_tax_due = self
                    .base_tax_due(revised_taxable_value, rule.rate_basis_points)
                    .saturating_add(rule.fixed_charge);
                let estimated_savings = current_tax_due.saturating_sub(revised_tax_due);

                if estimated_savings > 0 {
                    opportunities.push(TaxLossHarvestingOpportunity {
                        property_id,
                        jurisdiction_code: jurisdiction.code,
                        reporting_period,
                        kind: TaxLossHarvestingOpportunityKind::ExemptionReview,
                        estimated_savings,
                        current_tax_due,
                        revised_tax_due,
                    });
                }
            }

            opportunities.sort_by(|left, right| right.estimated_savings.cmp(&left.estimated_savings));
            Ok(opportunities)
        }

        #[ink(message)]
        pub fn get_audit_trail(&self, property_id: u64, limit: u64) -> Vec<AuditEntry> {
            let count = self.audit_log_count.get(property_id).unwrap_or(0);
            let start = count.saturating_sub(limit);
            let mut entries = Vec::new();
            for index in start..count {
                if let Some(entry) = self.audit_logs.get((property_id, index)) {
                    entries.push(entry);
                }
            }
            entries
        }

        fn ensure_admin(&self) -> Result<()> {
            if self.env().caller() != self.admin {
                return Err(Error::Unauthorized);
            }
            Ok(())
        }

        /// Canonical key for a treaty: (min, max) so order of arguments doesn't matter.
        fn treaty_key(a: u32, b: u32) -> (u32, u32) {
            if a <= b { (a, b) } else { (b, a) }
        }

        fn get_active_rule(&self, jurisdiction_code: u32) -> Result<TaxRule> {
            match self.tax_rules.get(jurisdiction_code) {
                Some(rule) if rule.active => Ok(rule),
                Some(_) => Err(Error::InactiveRule),
                None => Err(Error::RuleNotFound),
            }
        }

        fn reporting_period(&self, now: Timestamp, frequency: ReportingFrequency) -> u64 {
            now / frequency.period_millis()
        }

        fn resolve_status(&self, record: &TaxRecord, now: Timestamp) -> TaxStatus {
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

        fn outstanding_tax(&self, record: &TaxRecord) -> Balance {
            record.tax_due.saturating_sub(record.paid_amount)
        }

        fn taxable_value(&self, rule: &TaxRule, assessment: &PropertyAssessment) -> Balance {
            assessment
                .assessed_value
                .saturating_sub(rule.exemption_amount.saturating_add(assessment.exemption_override))
        }

        fn base_tax_due(&self, taxable_value: Balance, rate_basis_points: u32) -> Balance {
            taxable_value.saturating_mul(rate_basis_points as Balance) / BASIS_POINTS_DENOMINATOR
        }

        fn estimate_tax_due(&self, rule: &TaxRule, assessment: &PropertyAssessment) -> Balance {
            self.base_tax_due(self.taxable_value(rule, assessment), rule.rate_basis_points)
                .saturating_add(rule.fixed_charge)
        }

        fn registry_compliant(&self, owner: AccountId) -> bool {
            match self.compliance_registry {
                Some(registry) => {
                    use ink::env::call::FromAccountId;
                    let checker: ink::contract_ref!(ComplianceChecker) =
                        FromAccountId::from_account_id(registry);
                    checker.is_compliant(owner)
                }
                None => true,
            }
        }

        fn build_snapshot(
            &self,
            property_id: u64,
            jurisdiction_code: u32,
            rule: &TaxRule,
            assessment: &PropertyAssessment,
            record: Option<TaxRecord>,
        ) -> ComplianceSnapshot {
            let outstanding_tax = record
                .map(|value| self.outstanding_tax(&value))
                .unwrap_or_default();
            let status = record
                .map(|value| value.status)
                .unwrap_or(TaxStatus::Assessed);
            let reporting_period = record
                .map(|value| value.reporting_period)
                .unwrap_or_default();
            let tax_current = record
                .map(|value| {
                    value.paid_amount >= value.tax_due
                        && (!rule.requires_legal_documents || assessment.legal_documents_verified)
                        && (!rule.requires_reporting || assessment.reporting_submitted)
                })
                .unwrap_or(false);

            ComplianceSnapshot {
                property_id,
                jurisdiction_code,
                reporting_period,
                registry_compliant: self.registry_compliant(assessment.owner),
                tax_current,
                outstanding_tax,
                reporting_submitted: assessment.reporting_submitted,
                legal_documents_verified: assessment.legal_documents_verified,
                active_alerts: 0,
                status,
            }
        }

        fn emit_registry_sync_requested(&self, snapshot: ComplianceSnapshot) {
            self.env().emit_event(ComplianceRegistrySyncRequested {
                property_id: snapshot.property_id,
                jurisdiction_code: snapshot.jurisdiction_code,
                reporting_period: snapshot.reporting_period,
                outstanding_tax: snapshot.outstanding_tax,
                legal_documents_verified: snapshot.legal_documents_verified,
                reporting_submitted: snapshot.reporting_submitted,
            });
        }

        fn log_audit(
            &mut self,
            property_id: u64,
            jurisdiction_code: u32,
            reporting_period: u64,
            action: AuditAction,
            amount: Balance,
            reference_hash: [u8; 32],
        ) {
            let count = self.audit_log_count.get(property_id).unwrap_or(0);
            let entry = AuditEntry {
                action,
                property_id,
                jurisdiction_code,
                reporting_period,
                actor: self.env().caller(),
                timestamp: self.env().block_timestamp(),
                amount,
                reference_hash,
            };
            self.audit_logs.insert((property_id, count), &entry);
            self.audit_log_count.insert(property_id, &(count + 1));
        }

        // ===== Tax Document Storage (IPFS) - Issue #264 =====

        #[ink(message)]
        pub fn upload_tax_document(
            &mut self,
            property_id: u64,
            jurisdiction_code: u32,
            reporting_period: u64,
            document_type: DocumentType,
            ipfs_hash: [u8; 32],
        ) -> Result<()> {
            non_reentrant!(self, {
                self.ensure_admin()?;
                let now = self.env().block_timestamp();
                let caller = self.env().caller();

                let count = self
                    .tax_document_count
                    .get((property_id, jurisdiction_code, reporting_period))
                    .unwrap_or(0);

                let document = TaxDocument {
                    property_id,
                    jurisdiction_code,
                    reporting_period,
                    document_type,
                    ipfs_hash,
                    uploaded_by: caller,
                    uploaded_at: now,
                    verified: false,
                    verified_by: None,
                    verified_at: None,
                };

                self.tax_documents.insert(
                    (property_id, jurisdiction_code, reporting_period, count),
                    &document,
                );
                self.tax_document_count.insert(
                    (property_id, jurisdiction_code, reporting_period),
                    &(count + 1),
                );

                self.env().emit_event(TaxDocumentUploaded {
                    property_id,
                    jurisdiction_code,
                    reporting_period,
                    document_index: count,
                    document_type,
                    ipfs_hash,
                    uploaded_by: caller,
                });

                self.log_audit(
                    property_id,
                    jurisdiction_code,
                    reporting_period,
                    AuditAction::LegalDocumentUpdated,
                    0,
                    ipfs_hash,
                );

                Ok(())
            })
        }

        #[ink(message)]
        pub fn verify_tax_document(
            &mut self,
            property_id: u64,
            jurisdiction_code: u32,
            reporting_period: u64,
            document_index: u64,
        ) -> Result<()> {
            non_reentrant!(self, {
                self.ensure_admin()?;
                let now = self.env().block_timestamp();
                let caller = self.env().caller();

                let key = (
                    property_id,
                    jurisdiction_code,
                    reporting_period,
                    document_index,
                );
                let mut document = self.tax_documents.get(key).ok_or(Error::RecordNotFound)?;

                document.verified = true;
                document.verified_by = Some(caller);
                document.verified_at = Some(now);

                self.tax_documents.insert(key, &document);

                self.env().emit_event(TaxDocumentVerified {
                    property_id,
                    jurisdiction_code,
                    reporting_period,
                    document_index,
                    verified_by: caller,
                });

                self.log_audit(
                    property_id,
                    jurisdiction_code,
                    reporting_period,
                    AuditAction::LegalDocumentUpdated,
                    0,
                    document.ipfs_hash,
                );

                Ok(())
            })
        }

        #[ink(message)]
        pub fn get_tax_documents(
            &self,
            property_id: u64,
            jurisdiction_code: u32,
            reporting_period: u64,
        ) -> Vec<TaxDocument> {
            let count = self
                .tax_document_count
                .get((property_id, jurisdiction_code, reporting_period))
                .unwrap_or(0);
            let mut documents = Vec::new();
            for i in 0..count {
                if let Some(doc) =
                    self.tax_documents
                        .get((property_id, jurisdiction_code, reporting_period, i))
                {
                    documents.push(doc);
                }
            }
            documents
        }

        #[ink(message)]
        pub fn get_tax_document(
            &self,
            property_id: u64,
            jurisdiction_code: u32,
            reporting_period: u64,
            document_index: u64,
        ) -> Option<TaxDocument> {
            self.tax_documents.get((
                property_id,
                jurisdiction_code,
                reporting_period,
                document_index,
            ))
        }

        // ===== Tax Advisor Integration - Issue #265 =====

        #[ink(message)]
        pub fn register_tax_advisor(
            &mut self,
            advisor_id: AccountId,
            name: [u8; 64],
            license_number: [u8; 32],
            jurisdiction_codes: Vec<u32>,
        ) -> Result<()> {
            self.ensure_admin()?;
            let now = self.env().block_timestamp();

            let advisor = TaxAdvisor {
                advisor_id,
                name,
                license_number,
                jurisdiction_codes,
                is_active: true,
                registered_at: now,
            };

            self.tax_advisors.insert(&advisor_id, &advisor);

            self.env().emit_event(TaxAdvisorRegistered {
                advisor_id,
                license_number,
                jurisdiction_codes,
            });

            self.log_audit(0, 0, 0, AuditAction::RuleConfigured, 0, license_number);

            Ok(())
        }

        #[ink(message)]
        pub fn deactivate_tax_advisor(&mut self, advisor_id: AccountId) -> Result<()> {
            self.ensure_admin()?;

            let mut advisor = self
                .tax_advisors
                .get(&advisor_id)
                .ok_or(Error::Unauthorized)?;

            advisor.is_active = false;
            self.tax_advisors.insert(&advisor_id, &advisor);

            Ok(())
        }

        #[ink(message)]
        pub fn assign_advisor_to_property(
            &mut self,
            advisor_id: AccountId,
            property_id: u64,
        ) -> Result<()> {
            self.ensure_admin()?;

            let advisor = self
                .tax_advisors
                .get(&advisor_id)
                .ok_or(Error::Unauthorized)?;

            if !advisor.is_active {
                return Err(Error::InactiveRule);
            }

            self.advisor_property_assignments
                .insert((&advisor_id, property_id), &true);

            self.env().emit_event(TaxAdvisorAssigned {
                advisor_id,
                property_id,
            });

            self.log_audit(
                property_id,
                0,
                0,
                AuditAction::AssessmentUpdated,
                0,
                [0u8; 32],
            );

            Ok(())
        }

        #[ink(message)]
        pub fn remove_advisor_from_property(
            &mut self,
            advisor_id: AccountId,
            property_id: u64,
        ) -> Result<()> {
            self.ensure_admin()?;

            self.advisor_property_assignments
                .insert((&advisor_id, property_id), &false);

            Ok(())
        }

        #[ink(message)]
        pub fn get_tax_advisor(&self, advisor_id: AccountId) -> Option<TaxAdvisor> {
            self.tax_advisors.get(&advisor_id)
        }

        #[ink(message)]
        pub fn is_advisor_assigned(&self, advisor_id: AccountId, property_id: u64) -> bool {
            self.advisor_property_assignments
                .get((&advisor_id, property_id))
                .unwrap_or(false)
        }

        #[ink(message)]
        pub fn get_property_advisors(&self, property_id: u64) -> Vec<TaxAdvisor> {
            let mut advisors = Vec::new();
            // Note: In production, you'd want to maintain an index of advisor_ids
            // For now, this is a placeholder that would need iteration optimization
            advisors
        }
    }

    impl TaxWithholder for TaxComplianceModule {
        #[ink(message)]
        fn withhold_tax(
            &mut self,
            property_id: u64,
            jurisdiction: Jurisdiction,
            transaction_amount: u128,
        ) -> (u128, AccountId) {
            let rule = match self.get_active_rule(jurisdiction.code) {
                Ok(r) => r,
                Err(_) => return (0, AccountId::from([0x00; 32])),
            };

            if rule.withholding_rate_basis_points == 0 {
                return (0, rule.tax_collector);
            }

            let withheld_amount = (transaction_amount
                .saturating_mul(rule.withholding_rate_basis_points as u128))
                / BASIS_POINTS_DENOMINATOR as u128;

            if withheld_amount > 0 {
                let now = self.env().block_timestamp();
                let period = self.reporting_period(now, rule.reporting_frequency);

                self.env().emit_event(TaxWithheld {
                    property_id,
                    jurisdiction_code: jurisdiction.code,
                    amount: withheld_amount,
                    collector: rule.tax_collector,
                });

                self.log_audit(
                    property_id,
                    jurisdiction.code,
                    period,
                    AuditAction::TaxPaid,
                    withheld_amount,
                    [0u8; 32],
                );
            }

            (withheld_amount, rule.tax_collector)
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        fn jurisdiction() -> Jurisdiction {
            Jurisdiction {
                code: 1001,
                country_code: *b"US",
                region_code: 12,
                locality_code: 34,
            }
        }

        fn rule() -> TaxRule {
            TaxRule {
                rate_basis_points: 250,
                fixed_charge: 1_000,
                exemption_amount: 10_000,
                payment_due_period: 30 * 24 * 60 * 60 * 1000,
                reporting_frequency: ReportingFrequency::Annual,
                penalty_basis_points: 500,
                requires_reporting: true,
                requires_legal_documents: true,
                withholding_rate_basis_points: 500, // 5%
                tax_collector: AccountId::from([0x01; 32]),
                active: true,
            }
        }

        #[ink::test]
        fn calculate_tax_uses_jurisdiction_rule() {
            let mut contract = TaxComplianceModule::new(None);
            let owner = AccountId::from([0x02; 32]);

            contract
                .configure_tax_rule(jurisdiction(), rule())
                .expect("rule");
            contract
                .set_property_assessment(7, jurisdiction(), owner, 200_000, 5_000)
                .expect("assessment");

            let record = contract.calculate_tax(7, jurisdiction(), None).expect("tax");
            assert_eq!(record.taxable_value, 185_000);
            assert_eq!(record.tax_due, 5_625);
            assert_eq!(record.status, TaxStatus::Assessed);
        }

        #[ink::test]
        fn tax_loss_harvesting_recommends_reassessment_after_value_drop() {
            let mut contract = TaxComplianceModule::new(None);
            let owner = AccountId::from([0x07; 32]);

            contract
                .configure_tax_rule(jurisdiction(), rule())
                .expect("rule");
            contract
                .set_property_assessment(11, jurisdiction(), owner, 240_000, 0)
                .expect("assessment");

            let initial_record = contract.calculate_tax(11, jurisdiction()).expect("tax");

            contract
                .set_property_assessment(11, jurisdiction(), owner, 180_000, 0)
                .expect("updated assessment");

            let opportunities = contract
                .get_tax_loss_harvesting_opportunities(11, jurisdiction())
                .expect("opportunities");

            let reassessment = opportunities
                .iter()
                .find(|opportunity| {
                    opportunity.kind == TaxLossHarvestingOpportunityKind::Reassessment
                })
                .expect("reassessment opportunity");

            assert!(reassessment.estimated_savings > 0);
            assert!(reassessment.current_tax_due >= reassessment.revised_tax_due);
            assert_eq!(reassessment.reporting_period, initial_record.reporting_period);
        }

        #[ink::test]
        fn compliance_requires_payment_reporting_and_documents() {
            let mut contract = TaxComplianceModule::new(None);
            let owner = AccountId::from([0x03; 32]);

            contract
                .configure_tax_rule(jurisdiction(), rule())
                .expect("rule");
            contract
                .set_property_assessment(8, jurisdiction(), owner, 120_000, 0)
                .expect("assessment");

            let record = contract.calculate_tax(8, jurisdiction(), None).expect("tax");
            let initial = contract
                .check_compliance(8, jurisdiction())
                .expect("compliance");
            assert!(!initial.tax_current);
            assert_eq!(initial.outstanding_tax, record.tax_due);

            contract
                .record_tax_payment(
                    8,
                    jurisdiction(),
                    record.reporting_period,
                    record.tax_due,
                    [9u8; 32],
                )
                .expect("payment");
            contract
                .record_reporting_submission(8, jurisdiction(), record.reporting_period, [7u8; 32])
                .expect("report");
            contract
                .record_legal_document(8, jurisdiction(), [8u8; 32], true)
                .expect("document");

            let final_snapshot = contract
                .check_compliance(8, jurisdiction())
                .expect("compliance after hooks");
            assert!(final_snapshot.tax_current);
            assert_eq!(final_snapshot.outstanding_tax, 0);
            assert!(final_snapshot.reporting_submitted);
            assert!(final_snapshot.legal_documents_verified);
        }

        #[ink::test]
        fn audit_trail_captures_tax_lifecycle() {
            let mut contract = TaxComplianceModule::new(None);
            let owner = AccountId::from([0x04; 32]);

            contract
                .configure_tax_rule(jurisdiction(), rule())
                .expect("rule");
            contract
                .set_property_assessment(9, jurisdiction(), owner, 100_000, 0)
                .expect("assessment");
            let record = contract.calculate_tax(9, jurisdiction(), None).expect("tax");
            contract
                .record_tax_payment(
                    9,
                    jurisdiction(),
                    record.reporting_period,
                    record.tax_due / 2,
                    [5u8; 32],
                )
                .expect("payment");

            let logs = contract.get_audit_trail(9, 10);
            assert_eq!(logs.len(), 3);
            assert_eq!(logs[0].action, AuditAction::AssessmentUpdated);
            assert_eq!(logs[1].action, AuditAction::TaxCalculated);
            assert_eq!(logs[2].action, AuditAction::TaxPaid);
        }

        #[ink::test]
        fn test_upload_and_verify_tax_document() {
            let mut contract = TaxComplianceModule::new(None);
            let owner = AccountId::from([0x05; 32]);

            contract
                .configure_tax_rule(jurisdiction(), rule())
                .expect("rule");
            contract
                .set_property_assessment(10, jurisdiction(), owner, 150_000, 0)
                .expect("assessment");

            // Upload a tax document
            let ipfs_hash = [0xAB; 32];
            contract
                .upload_tax_document(10, 1001, 0, DocumentType::TaxReturn, ipfs_hash)
                .expect("upload");

            // Verify document was uploaded
            let documents = contract.get_tax_documents(10, 1001, 0);
            assert_eq!(documents.len(), 1);
            assert_eq!(documents[0].ipfs_hash, ipfs_hash);
            assert_eq!(documents[0].document_type, DocumentType::TaxReturn);
            assert!(!documents[0].verified);

            // Verify the document
            contract
                .verify_tax_document(10, 1001, 0, 0)
                .expect("verify");

            let documents = contract.get_tax_documents(10, 1001, 0);
            assert!(documents[0].verified);
            assert!(documents[0].verified_by.is_some());
        }

        #[ink::test]
        fn test_register_and_assign_tax_advisor() {
            let mut contract = TaxComplianceModule::new(None);
            let advisor_id = AccountId::from([0x06; 32]);

            // Register a tax advisor
            let name = [0x41; 64];
            let license = [0x42; 32];
            let jurisdictions = vec![1001, 1002];

            contract
                .register_tax_advisor(advisor_id, name, license, jurisdictions.clone())
                .expect("register");

            // Verify advisor was registered
            let advisor = contract.get_tax_advisor(advisor_id);
            assert!(advisor.is_some());
            let advisor = advisor.unwrap();
            assert!(advisor.is_active);
            assert_eq!(advisor.jurisdiction_codes, jurisdictions);

            // Assign advisor to property
            contract
                .assign_advisor_to_property(advisor_id, 15)
                .expect("assign");

            assert!(contract.is_advisor_assigned(advisor_id, 15));

            // Remove advisor from property
            contract
                .remove_advisor_from_property(advisor_id, 15)
                .expect("remove");

            assert!(!contract.is_advisor_assigned(advisor_id, 15));
        }

        // ── Tax treaty tests (#267) ──────────────────────────────────────────

        fn residence_jurisdiction() -> Jurisdiction {
            Jurisdiction {
                code: 2001,
                country_code: *b"DE",
                region_code: 0,
                locality_code: 0,
            }
        }

        #[ink::test]
        fn set_and_get_treaty() {
            let mut contract = TaxComplianceModule::new(None);
            contract
                .set_tax_treaty(1001, 2001, 2000, true)
                .expect("set treaty");
            let treaty = contract.get_tax_treaty(1001, 2001).expect("get treaty");
            assert_eq!(treaty.reduction_basis_points, 2000);
            assert!(treaty.active);
            // Canonical key: same result regardless of argument order
            assert_eq!(
                contract.get_tax_treaty(2001, 1001),
                Some(treaty)
            );
        }

        #[ink::test]
        fn treaty_reduces_tax_due() {
            let mut contract = TaxComplianceModule::new(None);
            let owner = AccountId::from([0x10; 32]);

            contract.configure_tax_rule(jurisdiction(), rule()).expect("rule");
            contract
                .set_property_assessment(20, jurisdiction(), owner, 200_000, 5_000)
                .expect("assessment");

            // Without treaty
            let record_no_treaty = contract
                .calculate_tax(20, jurisdiction(), None)
                .expect("tax no treaty");

            // Set a 20 % reduction treaty
            contract
                .set_tax_treaty(jurisdiction().code, residence_jurisdiction().code, 2000, true)
                .expect("treaty");

            let record_with_treaty = contract
                .calculate_tax(20, jurisdiction(), Some(residence_jurisdiction().code))
                .expect("tax with treaty");

            // tax_due should be 20 % less
            let expected = record_no_treaty
                .tax_due
                .saturating_mul(8000)
                / 10_000;
            assert_eq!(record_with_treaty.tax_due, expected);
            assert!(record_with_treaty.tax_due < record_no_treaty.tax_due);
        }

        #[ink::test]
        fn inactive_treaty_has_no_effect() {
            let mut contract = TaxComplianceModule::new(None);
            let owner = AccountId::from([0x11; 32]);

            contract.configure_tax_rule(jurisdiction(), rule()).expect("rule");
            contract
                .set_property_assessment(21, jurisdiction(), owner, 200_000, 0)
                .expect("assessment");

            // Inactive treaty
            contract
                .set_tax_treaty(jurisdiction().code, residence_jurisdiction().code, 3000, false)
                .expect("treaty");

            let record_no_treaty = contract
                .calculate_tax(21, jurisdiction(), None)
                .expect("no treaty");
            let record_inactive = contract
                .calculate_tax(21, jurisdiction(), Some(residence_jurisdiction().code))
                .expect("inactive treaty");

            assert_eq!(record_no_treaty.tax_due, record_inactive.tax_due);
        }

        #[ink::test]
        fn set_treaty_rejects_rate_over_100_percent() {
            let mut contract = TaxComplianceModule::new(None);
            assert_eq!(
                contract.set_tax_treaty(1001, 2001, 10_001, true),
                Err(Error::InvalidRate)
            );
        }

        #[ink::test]
        fn no_treaty_returns_none() {
            let contract = TaxComplianceModule::new(None);
            assert!(contract.get_tax_treaty(1001, 9999).is_none());
        }
    }
}
