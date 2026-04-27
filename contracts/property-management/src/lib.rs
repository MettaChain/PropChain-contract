#![cfg_attr(not(feature = "std"), no_std)]
#![allow(unexpected_cfgs)]

use ink::prelude::string::String;
use ink::storage::Mapping;
use propchain_traits::{non_reentrant, ComplianceChecker, ReentrancyError, ReentrancyGuard};

#[ink::contract]
mod property_management {
    use super::*;

    pub type TokenId = u64;

    #[derive(Debug, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub enum Error {
        Unauthorized,
        NotFound,
        InvalidAmount,
        LeaseNotActive,
        NotTenant,
        NotLandlordOrManager,
        InvalidFee,
        ScreeningNotFound,
        MaintenanceNotFound,
        ExpenseNotFound,
        DisputeNotFound,
        InvalidStatus,
        ComplianceViolation,
        NotCompliant,
        InspectionNotFound,
        TransferFailed,
        RespondentMismatch,
        ReentrantCall,
    }

    impl From<ReentrancyError> for Error {
        fn from(_: ReentrancyError) -> Self {
            Error::ReentrantCall
        }
    }

    #[derive(
        Debug,
        Clone,
        PartialEq,
        Eq,
        scale::Encode,
        scale::Decode,
        ink::storage::traits::StorageLayout,
    )]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub enum LeaseStatus {
        Active,
        Ended,
        Defaulted,
    }

    #[derive(
        Debug,
        Clone,
        PartialEq,
        Eq,
        scale::Encode,
        scale::Decode,
        ink::storage::traits::StorageLayout,
    )]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub struct Lease {
        pub id: u64,
        pub token_id: TokenId,
        pub tenant: AccountId,
        pub landlord: AccountId,
        pub rent_per_period: Balance,
        pub period_secs: u64,
        pub next_due: u64,
        pub management_fee_bps: u16,
        pub security_deposit: Balance,
        pub status: LeaseStatus,
        pub created_at: u64,
    }

    #[derive(
        Debug,
        Clone,
        PartialEq,
        Eq,
        scale::Encode,
        scale::Decode,
        ink::storage::traits::StorageLayout,
    )]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub enum MaintenanceStatus {
        Submitted,
        Acknowledged,
        InProgress,
        Resolved,
        Cancelled,
    }

    #[derive(
        Debug,
        Clone,
        PartialEq,
        Eq,
        scale::Encode,
        scale::Decode,
        ink::storage::traits::StorageLayout,
    )]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub struct MaintenanceRequest {
        pub id: u64,
        pub token_id: TokenId,
        pub requester: AccountId,
        pub title: String,
        pub description_hash: Hash,
        pub status: MaintenanceStatus,
        pub assigned_to: Option<AccountId>,
        pub resolution_hash: Option<Hash>,
        pub created_at: u64,
        pub updated_at: u64,
    }

    #[derive(
        Debug,
        Clone,
        PartialEq,
        Eq,
        scale::Encode,
        scale::Decode,
        ink::storage::traits::StorageLayout,
    )]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub enum ScreeningStatus {
        Pending,
        Approved,
        Rejected,
    }

    #[derive(
        Debug,
        Clone,
        PartialEq,
        Eq,
        scale::Encode,
        scale::Decode,
        ink::storage::traits::StorageLayout,
    )]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub struct TenantScreening {
        pub id: u64,
        pub token_id: TokenId,
        pub applicant: AccountId,
        pub application_hash: Hash,
        pub credit_tier: u8,
        pub income_ratio_bps: u16,
        pub status: ScreeningStatus,
        pub reviewer: Option<AccountId>,
        pub reviewed_at: Option<u64>,
        pub created_at: u64,
    }

    #[derive(
        Debug,
        Clone,
        PartialEq,
        Eq,
        scale::Encode,
        scale::Decode,
        ink::storage::traits::StorageLayout,
    )]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub enum ExpenseStatus {
        Recorded,
        Paid,
    }

    #[derive(
        Debug,
        Clone,
        PartialEq,
        Eq,
        scale::Encode,
        scale::Decode,
        ink::storage::traits::StorageLayout,
    )]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub struct Expense {
        pub id: u64,
        pub token_id: TokenId,
        pub category: String,
        pub amount: Balance,
        pub vendor: AccountId,
        pub description_hash: Hash,
        pub status: ExpenseStatus,
        pub recorded_by: AccountId,
        pub paid_at: Option<u64>,
    }

    #[derive(
        Debug,
        Clone,
        PartialEq,
        Eq,
        scale::Encode,
        scale::Decode,
        ink::storage::traits::StorageLayout,
    )]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub enum InspectionStatus {
        Scheduled,
        Completed,
        Cancelled,
    }

    #[derive(
        Debug,
        Clone,
        PartialEq,
        Eq,
        scale::Encode,
        scale::Decode,
        ink::storage::traits::StorageLayout,
    )]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub struct Inspection {
        pub id: u64,
        pub token_id: TokenId,
        pub inspector: AccountId,
        pub scheduled_ts: u64,
        pub report_hash: Option<Hash>,
        pub passed: Option<bool>,
        pub status: InspectionStatus,
        pub created_at: u64,
        pub completed_at: Option<u64>,
    }

    #[derive(
        Debug,
        Clone,
        PartialEq,
        Eq,
        scale::Encode,
        scale::Decode,
        ink::storage::traits::StorageLayout,
    )]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub struct JurisdictionCompliance {
        pub jurisdiction_code: String,
        pub max_security_deposit_bps: u16,
        pub min_notice_period_days: u16,
        pub late_fee_cap_bps: u16,
        pub last_audit_ts: u64,
        pub compliant: bool,
    }

    #[derive(
        Debug,
        Clone,
        PartialEq,
        Eq,
        scale::Encode,
        scale::Decode,
        ink::storage::traits::StorageLayout,
    )]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub enum DisputeStatus {
        Open,
        AwaitingCounterparty,
        ResolvedInitiator,
        ResolvedRespondent,
        Split,
        Cancelled,
    }

    #[derive(
        Debug,
        Clone,
        PartialEq,
        Eq,
        scale::Encode,
        scale::Decode,
        ink::storage::traits::StorageLayout,
    )]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub struct DisputeCase {
        pub id: u64,
        pub token_id: TokenId,
        pub initiator: AccountId,
        pub respondent: AccountId,
        pub reason_hash: Hash,
        pub initiator_stake: Balance,
        pub respondent_stake: Balance,
        pub status: DisputeStatus,
        pub created_at: u64,
    }

    #[derive(
        Debug,
        Clone,
        PartialEq,
        Eq,
        scale::Encode,
        scale::Decode,
        ink::storage::traits::StorageLayout,
    )]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub struct PropertyAnalytics {
        pub rent_collected: Balance,
        pub maintenance_open: u32,
        pub maintenance_resolved: u32,
        pub expense_total: Balance,
        pub inspection_count: u32,
        pub dispute_count: u32,
        pub screening_approved: u32,
    }

    #[derive(
        Debug,
        Clone,
        PartialEq,
        Eq,
        scale::Encode,
        scale::Decode,
        ink::storage::traits::StorageLayout,
    )]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub struct ManagementDashboard {
        pub total_rent_collected: Balance,
        pub open_maintenance: u32,
        pub active_leases: u32,
        pub open_disputes: u32,
        pub pending_screenings: u32,
    }

    #[ink(storage)]
    pub struct PropertyManagement {
        admin: AccountId,
        managers: Mapping<AccountId, bool>,
        compliance_registry: Option<AccountId>,
        fee_beneficiary: AccountId,
        reentrancy_guard: ReentrancyGuard,
        lease_counter: u64,
        leases: Mapping<u64, Lease>,
        maintenance_counter: u64,
        maintenance: Mapping<u64, MaintenanceRequest>,
        screening_counter: u64,
        screenings: Mapping<u64, TenantScreening>,
        expense_counter: u64,
        expenses: Mapping<u64, Expense>,
        inspection_counter: u64,
        inspections: Mapping<u64, Inspection>,
        dispute_counter: u64,
        disputes: Mapping<u64, DisputeCase>,
        legal_by_token: Mapping<TokenId, JurisdictionCompliance>,
        analytics_by_token: Mapping<TokenId, PropertyAnalytics>,
        operating_float: Balance,
        /// Native balance locked in active dispute escrows (excluded from operating payouts).
        dispute_escrow_locked: Balance,
        global_open_maintenance: u32,
        global_active_leases: u32,
        global_open_disputes: u32,
        global_pending_screenings: u32,
        total_rent_collected: Balance,
    }

    #[ink(event)]
    pub struct LeaseCreated {
        #[ink(topic)]
        pub lease_id: u64,
        #[ink(topic)]
        pub token_id: TokenId,
        #[ink(topic)]
        pub tenant: AccountId,
        pub rent_per_period: Balance,
    }

    #[ink(event)]
    pub struct RentPaid {
        #[ink(topic)]
        pub lease_id: u64,
        #[ink(topic)]
        pub tenant: AccountId,
        pub landlord_share: Balance,
        pub fee_share: Balance,
    }

    #[ink(event)]
    pub struct MaintenanceUpdated {
        #[ink(topic)]
        pub request_id: u64,
        #[ink(topic)]
        pub token_id: TokenId,
        pub status: MaintenanceStatus,
    }

    #[ink(event)]
    pub struct ScreeningReviewed {
        #[ink(topic)]
        pub screening_id: u64,
        pub status: ScreeningStatus,
    }

    #[ink(event)]
    pub struct ExpenseRecorded {
        #[ink(topic)]
        pub expense_id: u64,
        #[ink(topic)]
        pub token_id: TokenId,
        pub amount: Balance,
    }

    #[ink(event)]
    pub struct ExpensePaid {
        #[ink(topic)]
        pub expense_id: u64,
        pub vendor: AccountId,
    }

    #[ink(event)]
    pub struct InspectionCompleted {
        #[ink(topic)]
        pub inspection_id: u64,
        pub passed: bool,
    }

    #[ink(event)]
    pub struct DisputeResolved {
        #[ink(topic)]
        pub dispute_id: u64,
        pub status: DisputeStatus,
    }

    #[ink(event)]
    pub struct ComplianceUpdated {
        #[ink(topic)]
        pub token_id: TokenId,
        pub compliant: bool,
    }

    impl PropertyManagement {
        #[ink(constructor)]
        pub fn new() -> Self {
            let caller = Self::env().caller();
            Self {
                admin: caller,
                managers: Mapping::default(),
                compliance_registry: None,
                fee_beneficiary: caller,
                reentrancy_guard: ReentrancyGuard::new(),
                lease_counter: 0,
                leases: Mapping::default(),
                maintenance_counter: 0,
                maintenance: Mapping::default(),
                screening_counter: 0,
                screenings: Mapping::default(),
                expense_counter: 0,
                expenses: Mapping::default(),
                inspection_counter: 0,
                inspections: Mapping::default(),
                dispute_counter: 0,
                disputes: Mapping::default(),
                legal_by_token: Mapping::default(),
                analytics_by_token: Mapping::default(),
                operating_float: 0,
                dispute_escrow_locked: 0,
                global_open_maintenance: 0,
                global_active_leases: 0,
                global_open_disputes: 0,
                global_pending_screenings: 0,
                total_rent_collected: 0,
            }
        }

        #[ink(message)]
        pub fn admin(&self) -> AccountId {
            self.admin
        }

        #[ink(message)]
        pub fn set_fee_beneficiary(&mut self, beneficiary: AccountId) -> Result<(), Error> {
            self.ensure_admin()?;
            self.fee_beneficiary = beneficiary;
            Ok(())
        }

        #[ink(message)]
        pub fn set_compliance_registry(
            &mut self,
            registry: Option<AccountId>,
        ) -> Result<(), Error> {
            self.ensure_admin()?;
            self.compliance_registry = registry;
            Ok(())
        }

        #[ink(message)]
        pub fn add_manager(&mut self, account: AccountId) -> Result<(), Error> {
            self.ensure_admin()?;
            self.managers.insert(account, &true);
            Ok(())
        }

        #[ink(message)]
        pub fn remove_manager(&mut self, account: AccountId) -> Result<(), Error> {
            self.ensure_admin()?;
            self.managers.insert(account, &false);
            Ok(())
        }

        #[ink(message)]
        pub fn is_manager(&self, account: AccountId) -> bool {
            self.managers.get(account).unwrap_or(false)
        }

        /// Landlord-tenant legal parameters for automated compliance checks (caps, notice periods).
        #[ink(message)]
        pub fn set_jurisdiction_compliance(
            &mut self,
            token_id: TokenId,
            cfg: JurisdictionCompliance,
        ) -> Result<(), Error> {
            let caller = self.env().caller();
            if caller != self.admin && !self.is_manager(caller) {
                return Err(Error::Unauthorized);
            }
            self.legal_by_token.insert(token_id, &cfg);
            self.env().emit_event(ComplianceUpdated {
                token_id,
                compliant: cfg.compliant,
            });
            Ok(())
        }

        #[ink(message)]
        pub fn get_jurisdiction_compliance(
            &self,
            token_id: TokenId,
        ) -> Option<JurisdictionCompliance> {
            self.legal_by_token.get(token_id)
        }

        /// Create a lease; enforces security-deposit cap vs rent when jurisdiction config exists.
        #[ink(message)]
        #[allow(clippy::too_many_arguments)]
        pub fn create_lease(
            &mut self,
            token_id: TokenId,
            tenant: AccountId,
            landlord: AccountId,
            rent_per_period: Balance,
            period_secs: u64,
            management_fee_bps: u16,
            security_deposit: Balance,
            first_due: u64,
        ) -> Result<u64, Error> {
            non_reentrant!(self, {
                let caller = self.env().caller();
                if caller != landlord && caller != self.admin && !self.is_manager(caller) {
                    return Err(Error::NotLandlordOrManager);
                }
                if rent_per_period == 0 || period_secs == 0 {
                    return Err(Error::InvalidAmount);
                }
                if management_fee_bps > 10_000 {
                    return Err(Error::InvalidFee);
                }
                self.require_compliant(tenant)?;
                if let Some(legal) = self.legal_by_token.get(token_id) {
                    let periods_per_year: u128 =
                        (365u128 * 86_400) / u128::from(period_secs.max(1));
                    let annual = rent_per_period
                        .saturating_mul(periods_per_year)
                        .max(rent_per_period);
                    let max_dep =
                        annual.saturating_mul(legal.max_security_deposit_bps as u128) / 10_000;
                    if security_deposit > max_dep {
                        return Err(Error::ComplianceViolation);
                    }
                }

                self.lease_counter += 1;
                let id = self.lease_counter;
                let lease = Lease {
                    id,
                    token_id,
                    tenant,
                    landlord,
                    rent_per_period,
                    period_secs,
                    next_due: first_due,
                    management_fee_bps,
                    security_deposit,
                    status: LeaseStatus::Active,
                    created_at: self.env().block_timestamp(),
                };
                self.leases.insert(id, &lease);
                self.global_active_leases = self.global_active_leases.saturating_add(1);
                self.env().emit_event(LeaseCreated {
                    lease_id: id,
                    token_id,
                    tenant,
                    rent_per_period,
                });
                Ok(id)
            })
        }

        #[ink(message)]
        pub fn get_lease(&self, lease_id: u64) -> Option<Lease> {
            self.leases.get(lease_id)
        }

        /// Tenant pays rent; splits to landlord and fee beneficiary (management fee).
        #[ink(message, payable)]
        pub fn pay_rent(&mut self, lease_id: u64) -> Result<(), Error> {
            non_reentrant!(self, {
                let mut lease = self.leases.get(lease_id).ok_or(Error::NotFound)?;
                if lease.status != LeaseStatus::Active {
                    return Err(Error::LeaseNotActive);
                }
                let caller = self.env().caller();
                if caller != lease.tenant {
                    return Err(Error::NotTenant);
                }
                let paid = self.env().transferred_value();
                if paid != lease.rent_per_period {
                    return Err(Error::InvalidAmount);
                }
                let fee = paid.saturating_mul(lease.management_fee_bps as u128) / 10_000;
                let to_landlord = paid.saturating_sub(fee);
                self.env()
                    .transfer(lease.landlord, to_landlord)
                    .map_err(|_| Error::TransferFailed)?;
                if fee > 0 {
                    self.env()
                        .transfer(self.fee_beneficiary, fee)
                        .map_err(|_| Error::TransferFailed)?;
                }
                lease.next_due = lease.next_due.saturating_add(lease.period_secs);
                self.leases.insert(lease_id, &lease);
                let mut a = self.analytics_for(lease.token_id);
                a.rent_collected = a.rent_collected.saturating_add(paid);
                self.analytics_by_token.insert(lease.token_id, &a);
                self.total_rent_collected = self.total_rent_collected.saturating_add(paid);
                self.env().emit_event(RentPaid {
                    lease_id,
                    tenant: caller,
                    landlord_share: to_landlord,
                    fee_share: fee,
                });
                Ok(())
            })
        }

        #[ink(message)]
        pub fn end_lease(&mut self, lease_id: u64) -> Result<(), Error> {
            let mut lease = self.leases.get(lease_id).ok_or(Error::NotFound)?;
            let caller = self.env().caller();
            if caller != lease.landlord && caller != self.admin && !self.is_manager(caller) {
                return Err(Error::NotLandlordOrManager);
            }
            if lease.status != LeaseStatus::Active {
                return Err(Error::InvalidStatus);
            }
            lease.status = LeaseStatus::Ended;
            self.leases.insert(lease_id, &lease);
            self.global_active_leases = self.global_active_leases.saturating_sub(1);
            Ok(())
        }

        #[ink(message)]
        pub fn submit_maintenance_request(
            &mut self,
            token_id: TokenId,
            title: String,
            description_hash: Hash,
        ) -> Result<u64, Error> {
            non_reentrant!(self, {
                let caller = self.env().caller();
                self.require_compliant(caller)?;
                self.maintenance_counter += 1;
                let id = self.maintenance_counter;
                let now = self.env().block_timestamp();
                let req = MaintenanceRequest {
                    id,
                    token_id,
                    requester: caller,
                    title,
                    description_hash,
                    status: MaintenanceStatus::Submitted,
                    assigned_to: None,
                    resolution_hash: None,
                    created_at: now,
                    updated_at: now,
                };
                self.maintenance.insert(id, &req);
                let mut a = self.analytics_for(token_id);
                a.maintenance_open = a.maintenance_open.saturating_add(1);
                self.analytics_by_token.insert(token_id, &a);
                self.global_open_maintenance = self.global_open_maintenance.saturating_add(1);
                self.env().emit_event(MaintenanceUpdated {
                    request_id: id,
                    token_id,
                    status: MaintenanceStatus::Submitted,
                });
                Ok(id)
            })
        }

        #[ink(message)]
        pub fn update_maintenance_status(
            &mut self,
            request_id: u64,
            status: MaintenanceStatus,
            assigned_to: Option<AccountId>,
        ) -> Result<(), Error> {
            self.ensure_manager_or_admin()?;
            let mut req = self
                .maintenance
                .get(request_id)
                .ok_or(Error::MaintenanceNotFound)?;
            let was_open = matches!(
                req.status,
                MaintenanceStatus::Submitted
                    | MaintenanceStatus::Acknowledged
                    | MaintenanceStatus::InProgress
            );
            let now_open = matches!(
                status,
                MaintenanceStatus::Submitted
                    | MaintenanceStatus::Acknowledged
                    | MaintenanceStatus::InProgress
            );
            req.status = status;
            req.assigned_to = assigned_to;
            req.updated_at = self.env().block_timestamp();
            self.maintenance.insert(request_id, &req);
            if was_open && !now_open {
                let mut a = self.analytics_for(req.token_id);
                a.maintenance_open = a.maintenance_open.saturating_sub(1);
                if matches!(req.status, MaintenanceStatus::Resolved) {
                    a.maintenance_resolved = a.maintenance_resolved.saturating_add(1);
                }
                self.analytics_by_token.insert(req.token_id, &a);
                self.global_open_maintenance = self.global_open_maintenance.saturating_sub(1);
            }
            self.env().emit_event(MaintenanceUpdated {
                request_id,
                token_id: req.token_id,
                status: req.status.clone(),
            });
            Ok(())
        }

        #[ink(message)]
        pub fn resolve_maintenance(
            &mut self,
            request_id: u64,
            resolution_hash: Hash,
        ) -> Result<(), Error> {
            self.ensure_manager_or_admin()?;
            let mut req = self
                .maintenance
                .get(request_id)
                .ok_or(Error::MaintenanceNotFound)?;
            let was_open = matches!(
                req.status,
                MaintenanceStatus::Submitted
                    | MaintenanceStatus::Acknowledged
                    | MaintenanceStatus::InProgress
            );
            req.status = MaintenanceStatus::Resolved;
            req.resolution_hash = Some(resolution_hash);
            req.updated_at = self.env().block_timestamp();
            self.maintenance.insert(request_id, &req);
            if was_open {
                let mut a = self.analytics_for(req.token_id);
                a.maintenance_open = a.maintenance_open.saturating_sub(1);
                a.maintenance_resolved = a.maintenance_resolved.saturating_add(1);
                self.analytics_by_token.insert(req.token_id, &a);
                self.global_open_maintenance = self.global_open_maintenance.saturating_sub(1);
            }
            self.env().emit_event(MaintenanceUpdated {
                request_id,
                token_id: req.token_id,
                status: MaintenanceStatus::Resolved,
            });
            Ok(())
        }

        #[ink(message)]
        pub fn get_maintenance(&self, request_id: u64) -> Option<MaintenanceRequest> {
            self.maintenance.get(request_id)
        }

        /// Tenant screening application (off-chain data committed via `application_hash`).
        #[ink(message)]
        pub fn submit_screening_application(
            &mut self,
            token_id: TokenId,
            application_hash: Hash,
            credit_tier: u8,
            income_ratio_bps: u16,
        ) -> Result<u64, Error> {
            non_reentrant!(self, {
                let caller = self.env().caller();
                self.require_compliant(caller)?;
                self.screening_counter += 1;
                let id = self.screening_counter;
                let s = TenantScreening {
                    id,
                    token_id,
                    applicant: caller,
                    application_hash,
                    credit_tier,
                    income_ratio_bps,
                    status: ScreeningStatus::Pending,
                    reviewer: None,
                    reviewed_at: None,
                    created_at: self.env().block_timestamp(),
                };
                self.screenings.insert(id, &s);
                self.global_pending_screenings = self.global_pending_screenings.saturating_add(1);
                Ok(id)
            })
        }

        #[ink(message)]
        pub fn review_screening(&mut self, screening_id: u64, approve: bool) -> Result<(), Error> {
            self.ensure_manager_or_admin()?;
            let mut s = self
                .screenings
                .get(screening_id)
                .ok_or(Error::ScreeningNotFound)?;
            if s.status != ScreeningStatus::Pending {
                return Err(Error::InvalidStatus);
            }
            let caller = self.env().caller();
            s.status = if approve {
                ScreeningStatus::Approved
            } else {
                ScreeningStatus::Rejected
            };
            s.reviewer = Some(caller);
            s.reviewed_at = Some(self.env().block_timestamp());
            self.screenings.insert(screening_id, &s);
            self.global_pending_screenings = self.global_pending_screenings.saturating_sub(1);
            if approve {
                let mut a = self.analytics_for(s.token_id);
                a.screening_approved = a.screening_approved.saturating_add(1);
                self.analytics_by_token.insert(s.token_id, &a);
            }
            self.env().emit_event(ScreeningReviewed {
                screening_id,
                status: s.status.clone(),
            });
            Ok(())
        }

        #[ink(message)]
        pub fn get_screening(&self, screening_id: u64) -> Option<TenantScreening> {
            self.screenings.get(screening_id)
        }

        #[ink(message)]
        pub fn record_expense(
            &mut self,
            token_id: TokenId,
            category: String,
            amount: Balance,
            vendor: AccountId,
            description_hash: Hash,
        ) -> Result<u64, Error> {
            self.ensure_manager_or_admin()?;
            if amount == 0 {
                return Err(Error::InvalidAmount);
            }
            self.expense_counter += 1;
            let id = self.expense_counter;
            let e = Expense {
                id,
                token_id,
                category,
                amount,
                vendor,
                description_hash,
                status: ExpenseStatus::Recorded,
                recorded_by: self.env().caller(),
                paid_at: None,
            };
            self.expenses.insert(id, &e);
            let mut a = self.analytics_for(token_id);
            a.expense_total = a.expense_total.saturating_add(amount);
            self.analytics_by_token.insert(token_id, &a);
            self.env().emit_event(ExpenseRecorded {
                expense_id: id,
                token_id,
                amount,
            });
            Ok(id)
        }

        /// Pay a recorded expense to the vendor from the contract operating float (automated payout).
        #[ink(message)]
        pub fn pay_expense(&mut self, expense_id: u64) -> Result<(), Error> {
            non_reentrant!(self, {
                self.ensure_manager_or_admin()?;
                let mut e = self
                    .expenses
                    .get(expense_id)
                    .ok_or(Error::ExpenseNotFound)?;
                if e.status != ExpenseStatus::Recorded {
                    return Err(Error::InvalidStatus);
                }
                if self.operating_float < e.amount {
                    return Err(Error::InvalidAmount);
                }
                let spendable_native = self
                    .env()
                    .balance()
                    .saturating_sub(self.dispute_escrow_locked);
                if spendable_native < e.amount {
                    return Err(Error::InvalidAmount);
                }
                self.operating_float = self.operating_float.saturating_sub(e.amount);
                self.env()
                    .transfer(e.vendor, e.amount)
                    .map_err(|_| Error::TransferFailed)?;
                e.status = ExpenseStatus::Paid;
                e.paid_at = Some(self.env().block_timestamp());
                self.expenses.insert(expense_id, &e);
                self.env().emit_event(ExpensePaid {
                    expense_id,
                    vendor: e.vendor,
                });
                Ok(())
            })
        }

        #[ink(message, payable)]
        pub fn fund_operating_float(&mut self) -> Result<(), Error> {
            let v = self.env().transferred_value();
            self.operating_float = self.operating_float.saturating_add(v);
            Ok(())
        }

        #[ink(message)]
        pub fn operating_float_balance(&self) -> Balance {
            self.operating_float
        }

        #[ink(message)]
        pub fn dispute_escrow_locked_balance(&self) -> Balance {
            self.dispute_escrow_locked
        }

        #[ink(message)]
        pub fn get_expense(&self, expense_id: u64) -> Option<Expense> {
            self.expenses.get(expense_id)
        }

        #[ink(message)]
        pub fn schedule_inspection(
            &mut self,
            token_id: TokenId,
            inspector: AccountId,
            scheduled_ts: u64,
        ) -> Result<u64, Error> {
            self.ensure_manager_or_admin()?;
            self.inspection_counter += 1;
            let id = self.inspection_counter;
            let ins = Inspection {
                id,
                token_id,
                inspector,
                scheduled_ts,
                report_hash: None,
                passed: None,
                status: InspectionStatus::Scheduled,
                created_at: self.env().block_timestamp(),
                completed_at: None,
            };
            self.inspections.insert(id, &ins);
            Ok(id)
        }

        #[ink(message)]
        pub fn complete_inspection(
            &mut self,
            inspection_id: u64,
            report_hash: Hash,
            passed: bool,
        ) -> Result<(), Error> {
            let caller = self.env().caller();
            let mut ins = self
                .inspections
                .get(inspection_id)
                .ok_or(Error::InspectionNotFound)?;
            if caller != ins.inspector && caller != self.admin && !self.is_manager(caller) {
                return Err(Error::Unauthorized);
            }
            if ins.status != InspectionStatus::Scheduled {
                return Err(Error::InvalidStatus);
            }
            ins.status = InspectionStatus::Completed;
            ins.report_hash = Some(report_hash);
            ins.passed = Some(passed);
            ins.completed_at = Some(self.env().block_timestamp());
            self.inspections.insert(inspection_id, &ins);
            let mut a = self.analytics_for(ins.token_id);
            a.inspection_count = a.inspection_count.saturating_add(1);
            self.analytics_by_token.insert(ins.token_id, &a);
            self.env().emit_event(InspectionCompleted {
                inspection_id,
                passed,
            });
            Ok(())
        }

        #[ink(message)]
        pub fn get_inspection(&self, inspection_id: u64) -> Option<Inspection> {
            self.inspections.get(inspection_id)
        }

        /// Open a dispute escrow: initiator locks native stake; respondent must match stake.
        #[ink(message, payable)]
        pub fn open_dispute(
            &mut self,
            token_id: TokenId,
            respondent: AccountId,
            reason_hash: Hash,
        ) -> Result<u64, Error> {
            non_reentrant!(self, {
                let caller = self.env().caller();
                self.require_compliant(caller)?;
                let stake = self.env().transferred_value();
                if stake == 0 {
                    return Err(Error::InvalidAmount);
                }
                self.dispute_counter += 1;
                let id = self.dispute_counter;
                let d = DisputeCase {
                    id,
                    token_id,
                    initiator: caller,
                    respondent,
                    reason_hash,
                    initiator_stake: stake,
                    respondent_stake: 0,
                    status: DisputeStatus::AwaitingCounterparty,
                    created_at: self.env().block_timestamp(),
                };
                self.disputes.insert(id, &d);
                self.dispute_escrow_locked = self.dispute_escrow_locked.saturating_add(stake);
                self.global_open_disputes = self.global_open_disputes.saturating_add(1);
                let mut a = self.analytics_for(token_id);
                a.dispute_count = a.dispute_count.saturating_add(1);
                self.analytics_by_token.insert(token_id, &a);
                Ok(id)
            })
        }

        #[ink(message, payable)]
        pub fn counterparty_stake_dispute(&mut self, dispute_id: u64) -> Result<(), Error> {
            let mut d = self
                .disputes
                .get(dispute_id)
                .ok_or(Error::DisputeNotFound)?;
            let caller = self.env().caller();
            if caller != d.respondent {
                return Err(Error::RespondentMismatch);
            }
            if d.status != DisputeStatus::AwaitingCounterparty {
                return Err(Error::InvalidStatus);
            }
            let v = self.env().transferred_value();
            if v != d.initiator_stake {
                return Err(Error::InvalidAmount);
            }
            d.respondent_stake = v;
            d.status = DisputeStatus::Open;
            self.dispute_escrow_locked = self.dispute_escrow_locked.saturating_add(v);
            self.disputes.insert(dispute_id, &d);
            Ok(())
        }

        /// Arbitrator (admin) resolves: release all to initiator, respondent, or split 50/50.
        #[ink(message)]
        pub fn resolve_dispute(
            &mut self,
            dispute_id: u64,
            release_to_initiator: Option<bool>,
        ) -> Result<(), Error> {
            non_reentrant!(self, {
                self.ensure_admin()?;
                let d = self
                    .disputes
                    .get(dispute_id)
                    .ok_or(Error::DisputeNotFound)?;
                if d.status != DisputeStatus::Open {
                    return Err(Error::InvalidStatus);
                }
                let total = d.initiator_stake.saturating_add(d.respondent_stake);
                match release_to_initiator {
                    Some(true) => {
                        self.env()
                            .transfer(d.initiator, total)
                            .map_err(|_| Error::TransferFailed)?;
                        self.finish_dispute(dispute_id, DisputeStatus::ResolvedInitiator)?;
                    }
                    Some(false) => {
                        self.env()
                            .transfer(d.respondent, total)
                            .map_err(|_| Error::TransferFailed)?;
                        self.finish_dispute(dispute_id, DisputeStatus::ResolvedRespondent)?;
                    }
                    None => {
                        let half = total / 2;
                        let rem = total.saturating_sub(half.saturating_mul(2));
                        self.env()
                            .transfer(d.initiator, half.saturating_add(rem))
                            .map_err(|_| Error::TransferFailed)?;
                        self.env()
                            .transfer(d.respondent, half)
                            .map_err(|_| Error::TransferFailed)?;
                        self.finish_dispute(dispute_id, DisputeStatus::Split)?;
                    }
                }
                Ok(())
            })
        }

        #[ink(message)]
        pub fn get_dispute(&self, dispute_id: u64) -> Option<DisputeCase> {
            self.disputes.get(dispute_id)
        }

        #[ink(message)]
        pub fn get_property_analytics(&self, token_id: TokenId) -> PropertyAnalytics {
            self.analytics_for(token_id)
        }

        /// Aggregated dashboard for property managers.
        #[ink(message)]
        pub fn get_management_dashboard(&self) -> ManagementDashboard {
            ManagementDashboard {
                total_rent_collected: self.total_rent_collected,
                open_maintenance: self.global_open_maintenance,
                active_leases: self.global_active_leases,
                open_disputes: self.global_open_disputes,
                pending_screenings: self.global_pending_screenings,
            }
        }

        fn finish_dispute(&mut self, dispute_id: u64, status: DisputeStatus) -> Result<(), Error> {
            let mut d = self
                .disputes
                .get(dispute_id)
                .ok_or(Error::DisputeNotFound)?;
            let released = d.initiator_stake.saturating_add(d.respondent_stake);
            self.dispute_escrow_locked = self.dispute_escrow_locked.saturating_sub(released);
            d.status = status.clone();
            self.disputes.insert(dispute_id, &d);
            self.global_open_disputes = self.global_open_disputes.saturating_sub(1);
            self.env()
                .emit_event(DisputeResolved { dispute_id, status });
            Ok(())
        }

        fn analytics_for(&self, token_id: TokenId) -> PropertyAnalytics {
            self.analytics_by_token
                .get(token_id)
                .unwrap_or(PropertyAnalytics {
                    rent_collected: 0,
                    maintenance_open: 0,
                    maintenance_resolved: 0,
                    expense_total: 0,
                    inspection_count: 0,
                    dispute_count: 0,
                    screening_approved: 0,
                })
        }

        fn ensure_admin(&self) -> Result<(), Error> {
            if self.env().caller() != self.admin {
                return Err(Error::Unauthorized);
            }
            Ok(())
        }

        fn ensure_manager_or_admin(&self) -> Result<(), Error> {
            let c = self.env().caller();
            if c != self.admin && !self.is_manager(c) {
                return Err(Error::Unauthorized);
            }
            Ok(())
        }

        fn require_compliant(&self, account: AccountId) -> Result<(), Error> {
            if let Some(registry) = self.compliance_registry {
                use ink::env::call::FromAccountId;
                let checker: ink::contract_ref!(ComplianceChecker) =
                    FromAccountId::from_account_id(registry);
                if !checker.is_compliant(account) {
                    return Err(Error::NotCompliant);
                }
            }
            Ok(())
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use ink::env::{test, DefaultEnvironment};

        fn setup() -> PropertyManagement {
            PropertyManagement::new()
        }

        #[ink::test]
        fn create_lease_and_pay_rent_distributes() {
            let accounts = test::default_accounts::<DefaultEnvironment>();
            test::set_caller::<DefaultEnvironment>(accounts.alice);
            let mut pm = setup();
            let lease_id = pm
                .create_lease(1, accounts.bob, accounts.alice, 1000, 86_400, 500, 0, 0)
                .expect("lease");
            test::set_caller::<DefaultEnvironment>(accounts.bob);
            test::set_value_transferred::<DefaultEnvironment>(1000);
            pm.pay_rent(lease_id).expect("pay rent");
            let lease = pm.get_lease(lease_id).expect("lease exists");
            assert_eq!(lease.next_due, 86_400);
            let dash = pm.get_management_dashboard();
            assert_eq!(dash.total_rent_collected, 1000);
        }

        #[ink::test]
        fn maintenance_workflow() {
            let accounts = test::default_accounts::<DefaultEnvironment>();
            test::set_caller::<DefaultEnvironment>(accounts.alice);
            let mut pm = setup();
            pm.add_manager(accounts.charlie).ok();
            test::set_caller::<DefaultEnvironment>(accounts.bob);
            let mid = pm
                .submit_maintenance_request(7, String::from("Leak"), Hash::from([1u8; 32]))
                .expect("m");
            test::set_caller::<DefaultEnvironment>(accounts.charlie);
            pm.update_maintenance_status(
                mid,
                MaintenanceStatus::InProgress,
                Some(accounts.charlie),
            )
            .expect("upd");
            pm.resolve_maintenance(mid, Hash::from([2u8; 32]))
                .expect("res");
            let m = pm.get_maintenance(mid).expect("get");
            assert_eq!(m.status, MaintenanceStatus::Resolved);
        }

        #[ink::test]
        fn screening_review() {
            let accounts = test::default_accounts::<DefaultEnvironment>();
            test::set_caller::<DefaultEnvironment>(accounts.alice);
            let mut pm = setup();
            pm.add_manager(accounts.alice).ok();
            test::set_caller::<DefaultEnvironment>(accounts.bob);
            let sid = pm
                .submit_screening_application(1, Hash::from([3u8; 32]), 2, 3000)
                .expect("s");
            test::set_caller::<DefaultEnvironment>(accounts.alice);
            pm.review_screening(sid, true).expect("review");
            let s = pm.get_screening(sid).expect("get");
            assert_eq!(s.status, ScreeningStatus::Approved);
        }

        #[ink::test]
        fn expense_pay_from_float() {
            let accounts = test::default_accounts::<DefaultEnvironment>();
            test::set_caller::<DefaultEnvironment>(accounts.alice);
            let mut pm = setup();
            test::set_value_transferred::<DefaultEnvironment>(5000);
            pm.fund_operating_float().expect("fund");
            let eid = pm
                .record_expense(
                    1,
                    String::from("repair"),
                    500,
                    accounts.bob,
                    Hash::from([4u8; 32]),
                )
                .expect("exp");
            pm.pay_expense(eid).expect("pay");
            assert_eq!(pm.operating_float_balance(), 4500);
            let e = pm.get_expense(eid).expect("e");
            assert_eq!(e.status, ExpenseStatus::Paid);
        }

        #[ink::test]
        fn dispute_escrow_split() {
            let accounts = test::default_accounts::<DefaultEnvironment>();
            test::set_caller::<DefaultEnvironment>(accounts.alice);
            let mut pm = setup();
            test::set_value_transferred::<DefaultEnvironment>(100);
            let did = pm
                .open_dispute(1, accounts.bob, Hash::from([5u8; 32]))
                .expect("open");
            test::set_caller::<DefaultEnvironment>(accounts.bob);
            test::set_value_transferred::<DefaultEnvironment>(100);
            pm.counterparty_stake_dispute(did).expect("counter");
            test::set_caller::<DefaultEnvironment>(accounts.alice);
            pm.resolve_dispute(did, None).expect("split");
            let d = pm.get_dispute(did).expect("d");
            assert_eq!(d.status, DisputeStatus::Split);
        }

        #[ink::test]
        fn compliance_blocks_overlarge_deposit() {
            let accounts = test::default_accounts::<DefaultEnvironment>();
            test::set_caller::<DefaultEnvironment>(accounts.alice);
            let mut pm = setup();
            pm.set_jurisdiction_compliance(
                9,
                JurisdictionCompliance {
                    jurisdiction_code: String::from("US-CA"),
                    max_security_deposit_bps: 1000,
                    min_notice_period_days: 30,
                    late_fee_cap_bps: 500,
                    last_audit_ts: 0,
                    compliant: true,
                },
            )
            .expect("legal");
            // 10% of implied annual (1000 * 365) = 36_500 cap
            let r = pm.create_lease(9, accounts.bob, accounts.alice, 1000, 86_400, 0, 40_000, 0);
            assert_eq!(r, Err(Error::ComplianceViolation));
        }

        #[ink::test]
        fn full_management_workflow() {
            let accounts = test::default_accounts::<DefaultEnvironment>();
            test::set_caller::<DefaultEnvironment>(accounts.alice);
            let mut pm = setup();
            pm.add_manager(accounts.charlie).expect("manager");
            pm.set_jurisdiction_compliance(
                1,
                JurisdictionCompliance {
                    jurisdiction_code: String::from("US"),
                    max_security_deposit_bps: 2000,
                    min_notice_period_days: 30,
                    late_fee_cap_bps: 500,
                    last_audit_ts: 0,
                    compliant: true,
                },
            )
            .expect("compliance");
            let lease_id = pm
                .create_lease(1, accounts.bob, accounts.alice, 2000, 86_400, 250, 100, 0)
                .expect("lease");
            test::set_caller::<DefaultEnvironment>(accounts.bob);
            test::set_value_transferred::<DefaultEnvironment>(2000);
            pm.pay_rent(lease_id).expect("rent");
            test::set_caller::<DefaultEnvironment>(accounts.bob);
            let mid = pm
                .submit_maintenance_request(1, String::from("HVAC"), Hash::from([9u8; 32]))
                .expect("maint");
            test::set_caller::<DefaultEnvironment>(accounts.charlie);
            pm.update_maintenance_status(
                mid,
                MaintenanceStatus::Acknowledged,
                Some(accounts.charlie),
            )
            .expect("ack");
            pm.resolve_maintenance(mid, Hash::from([8u8; 32]))
                .expect("resolve");
            test::set_caller::<DefaultEnvironment>(accounts.bob);
            let sid = pm
                .submit_screening_application(1, Hash::from([7u8; 32]), 3, 2800)
                .expect("screen");
            test::set_caller::<DefaultEnvironment>(accounts.charlie);
            pm.review_screening(sid, true).expect("approve");
            test::set_caller::<DefaultEnvironment>(accounts.alice);
            test::set_value_transferred::<DefaultEnvironment>(10_000);
            pm.fund_operating_float().expect("fund");
            test::set_caller::<DefaultEnvironment>(accounts.charlie);
            let eid = pm
                .record_expense(
                    1,
                    String::from("plumbing"),
                    800,
                    accounts.bob,
                    Hash::from([6u8; 32]),
                )
                .expect("exp");
            pm.pay_expense(eid).expect("pay exp");
            test::set_caller::<DefaultEnvironment>(accounts.charlie);
            let iid = pm
                .schedule_inspection(1, accounts.charlie, 1_700_000_000)
                .expect("sched");
            pm.complete_inspection(iid, Hash::from([5u8; 32]), true)
                .expect("insp");
            test::set_caller::<DefaultEnvironment>(accounts.bob);
            test::set_value_transferred::<DefaultEnvironment>(50);
            let did = pm
                .open_dispute(1, accounts.alice, Hash::from([4u8; 32]))
                .expect("dispute");
            test::set_caller::<DefaultEnvironment>(accounts.alice);
            test::set_value_transferred::<DefaultEnvironment>(50);
            pm.counterparty_stake_dispute(did).expect("stake");
            test::set_caller::<DefaultEnvironment>(accounts.alice);
            pm.resolve_dispute(did, Some(true))
                .expect("resolve dispute");
            assert_eq!(
                pm.get_dispute(did).expect("d").status,
                DisputeStatus::ResolvedInitiator
            );
            let dash = pm.get_management_dashboard();
            assert!(dash.total_rent_collected >= 2000);
            let a = pm.get_property_analytics(1);
            assert!(a.rent_collected >= 2000);
            assert!(a.maintenance_resolved >= 1);
        }
    }
}
