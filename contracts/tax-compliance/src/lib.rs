#![cfg_attr(not(feature = "std"), no_std, no_main)]

use ink::prelude::vec::Vec;
use ink::storage::Mapping;
use propchain_contracts::{non_reentrant, ReentrancyError, ReentrancyGuard};
use propchain_traits::ComplianceChecker;
use propchain_traits::*;

#[ink::contract]
mod tax_compliance {
    use super::*;

    const BASIS_POINTS_DENOMINATOR: Balance = 10_000;

    #[derive(Debug, Clone, Copy, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(
        feature = "std",
        derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
    )]
    pub struct Jurisdiction {
        pub code: u32,
        pub country_code: [u8; 2],
        pub region_code: u16,
        pub locality_code: u16,
    }

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
    pub struct TaxRule {
        pub rate_basis_points: u32,
        pub fixed_charge: Balance,
        pub exemption_amount: Balance,
        pub payment_due_period: u64,
        pub reporting_frequency: ReportingFrequency,
        pub penalty_basis_points: u32,
        pub requires_reporting: bool,
        pub requires_legal_documents: bool,
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
        pub status: TaxStatus,
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

    #[ink(storage)]
    pub struct TaxComplianceModule {
        admin: AccountId,
        compliance_registry: Option<AccountId>,
        reentrancy_guard: ReentrancyGuard,
        tax_rules: Mapping<u32, TaxRule>,
        property_assessments: Mapping<(u64, u32), PropertyAssessment>,
        #[allow(clippy::type_complexity)]
        tax_records: Mapping<(u64, u32, u64), TaxRecord>,
        latest_reporting_period: Mapping<(u64, u32), u64>,
        audit_logs: Mapping<(u64, u64), AuditEntry>,
        audit_log_count: Mapping<u64, u64>,
    }

    impl TaxComplianceModule {
        #[ink(constructor)]
        pub fn new(compliance_registry: Option<AccountId>) -> Self {
            Self {
                admin: Self::env().caller(),
                compliance_registry,
                reentrancy_guard: ReentrancyGuard::new(),
                tax_rules: Mapping::default(),
                property_assessments: Mapping::default(),
                tax_records: Mapping::default(),
                latest_reporting_period: Mapping::default(),
                audit_logs: Mapping::default(),
                audit_log_count: Mapping::default(),
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
                let tax_due = base_tax.saturating_add(rule.fixed_charge);
                let mut record = TaxRecord {
                    property_id,
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
                    .insert((property_id, jurisdiction.code), &reporting_period);

                self.log_audit(
                    property_id,
                    jurisdiction.code,
                    reporting_period,
                    AuditAction::TaxCalculated,
                    tax_due,
                    [0u8; 32],
                );
                self.env().emit_event(TaxCalculated {
                    property_id,
                    jurisdiction_code: jurisdiction.code,
                    reporting_period,
                    tax_due,
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

            let record = contract.calculate_tax(7, jurisdiction()).expect("tax");
            assert_eq!(record.taxable_value, 185_000);
            assert_eq!(record.tax_due, 5_625);
            assert_eq!(record.status, TaxStatus::Assessed);
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

            let record = contract.calculate_tax(8, jurisdiction()).expect("tax");
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
            let record = contract.calculate_tax(9, jurisdiction()).expect("tax");
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
    }
}
