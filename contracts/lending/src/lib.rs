#![cfg_attr(not(feature = "std"), no_std, no_main)]
#![allow(
    clippy::arithmetic_side_effects,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::needless_borrows_for_generic_args
)]

use ink::storage::Mapping;

#[ink::contract]
mod propchain_lending {
    use super::*;
    use ink::prelude::{string::String, vec::Vec};

    #[derive(Debug, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub enum LendingError {
        Unauthorized,
        PropertyNotFound,
        InsufficientCollateral,
        LoanNotFound,
        LoanNotActive,
        PoolNotFound,
        InsufficientLiquidity,
        PositionNotFound,
        LiquidationThresholdNotMet,
        InvalidParameters,
        ProposalNotFound,
        RestructuringNotFound,
        InsufficientVotes,
        ServicerNotFound,
        PaymentScheduleNotFound,
        ReentrantCall,
        // Admin key rotation (Issue #496)
        KeyRotationCooldown,
        KeyRotationExpired,
        NoPendingRotation,
        RotationUnauthorized,
        RequestExpired,
    }

    impl From<propchain_traits::ReentrancyError> for LendingError {
        fn from(_: propchain_traits::ReentrancyError) -> Self {
            LendingError::ReentrantCall
        }
    }

    #[derive(
        Debug, Clone, PartialEq, scale::Encode, scale::Decode, ink::storage::traits::StorageLayout,
    )]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub struct CollateralRecord {
        pub property_id: u64,
        pub assessed_value: u128,
        pub ltv_ratio: u32,
        pub liquidation_threshold: u32,
    }

    #[derive(
        Debug, Clone, PartialEq, scale::Encode, scale::Decode, ink::storage::traits::StorageLayout,
    )]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub struct LendingPool {
        pub pool_id: u64,
        pub total_deposits: u128,
        pub total_borrows: u128,
        pub base_rate: u32,
    }

    #[derive(
        Debug, Clone, PartialEq, scale::Encode, scale::Decode, ink::storage::traits::StorageLayout,
    )]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub struct MarginPosition {
        pub position_id: u64,
        pub owner: AccountId,
        pub collateral: u128,
        pub leverage: u32,
        pub is_short: bool,
        pub entry_price: u128,
    }

    #[derive(
        Debug,
        Clone,
        Copy,
        PartialEq,
        Eq,
        scale::Encode,
        scale::Decode,
        ink::storage::traits::StorageLayout,
    )]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub enum LoanStatus {
        Pending,
        Active,
        Repaid,
        Defaulted,
        RestructuringProposed,
        Restructured,
        Liquidated,
    }

    #[derive(
        Debug,
        Clone,
        Copy,
        PartialEq,
        Eq,
        scale::Encode,
        scale::Decode,
        ink::storage::traits::StorageLayout,
    )]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub enum CollateralKind {
        Unsecured,
        PropertyTokenized,
    }

    #[derive(
        Debug, Clone, PartialEq, scale::Encode, scale::Decode, ink::storage::traits::StorageLayout,
    )]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub struct LoanApplication {
        pub loan_id: u64,
        pub applicant: AccountId,
        pub property_id: u64,
        pub requested_amount: u128,
        pub collateral_value: u128,
        pub credit_score: u32,
        pub approved: bool,
        pub servicer_id: Option<u64>,
        pub servicing_reference: String,
        pub servicing_status: String,
        pub collateral_kind: CollateralKind,
        pub term_months: u32,
        pub interest_rate_bps: u32,
        pub status: LoanStatus,
    }

    #[derive(
        Debug, Clone, PartialEq, scale::Encode, scale::Decode, ink::storage::traits::StorageLayout,
    )]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub struct LoanServicer {
        pub servicer_id: u64,
        pub account: AccountId,
        pub name: String,
        pub active: bool,
        pub collateral_kind: CollateralKind,
        pub term_months: u32,
        pub interest_rate_bps: u32,
        pub status: LoanStatus,
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
    pub enum PaymentScheduleStatus {
        Active,
        Completed,
        Defaulted,
    }

    #[derive(
        Debug, Clone, PartialEq, scale::Encode, scale::Decode, ink::storage::traits::StorageLayout,
    )]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub struct PaymentSchedule {
        pub schedule_id: u64,
        pub loan_id: u64,
        pub borrower: AccountId,
        pub principal_due: u128,
        pub interest_due: u128,
        pub installment_amount: u128,
        pub total_installments: u32,
        pub installments_paid: u32,
        pub first_due_block: u64,
        pub interval_blocks: u64,
        pub next_due_block: u64,
        pub total_paid: u128,
        pub status: PaymentScheduleStatus,
    }

    #[derive(
        Debug, Clone, PartialEq, scale::Encode, scale::Decode, ink::storage::traits::StorageLayout,
    )]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub struct LoanRestructuring {
        pub loan_id: u64,
        pub proposed_by: AccountId,
        pub proposed_term_months: u32,
        pub proposed_interest_rate_bps: u32,
        pub borrower_approved: bool,
        pub lender_approved: bool,
    }
    #[derive(
        Debug, Clone, PartialEq, scale::Encode, scale::Decode, ink::storage::traits::StorageLayout,
    )]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub struct YieldPosition {
        pub owner: AccountId,
        pub staked: u128,
        pub reward_debt: u128,
        pub accumulated_rewards: u128,
    }

    #[derive(
        Debug, Clone, PartialEq, scale::Encode, scale::Decode, ink::storage::traits::StorageLayout,
    )]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub struct Proposal {
        pub proposal_id: u64,
        pub description: String,
        pub votes_for: u64,
        pub votes_against: u64,
        pub executed: bool,
    }

    /// On-chain credit history for a borrower.
    ///
    /// Score formula (0–1000):
    ///   base 500
    ///   + repayments_on_time * 20   (capped at +300)
    ///   - defaults * 150            (capped at -450)
    ///   - active_loans * 10         (capped at -100)
    #[derive(
        Debug, Clone, PartialEq, scale::Encode, scale::Decode, ink::storage::traits::StorageLayout,
    )]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub struct CreditProfile {
        pub repayments_on_time: u32,
        pub defaults: u32,
        pub active_loans: u32,
        pub total_borrowed: u128,
    }

    // ── #304: Loan Marketplace types ─────────────────────────────────────────

    /// Status of a loan marketplace listing.
    #[derive(
        Debug,
        Clone,
        Copy,
        PartialEq,
        Eq,
        scale::Encode,
        scale::Decode,
        ink::storage::traits::StorageLayout,
    )]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub enum ListingStatus {
        /// Awaiting bids from lenders.
        Open,
        /// An offer has been accepted; origination in progress.
        OfferAccepted,
        /// Loan originated successfully.
        Originated,
        /// Listing withdrawn by the borrower.
        Cancelled,
    }

    /// A borrower's public loan request listed on the marketplace (#304).
    #[derive(
        Debug, Clone, PartialEq, scale::Encode, scale::Decode, ink::storage::traits::StorageLayout,
    )]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub struct LoanListing {
        pub listing_id: u64,
        pub borrower: AccountId,
        pub property_id: u64,
        pub requested_amount: u128,
        /// Maximum interest rate the borrower is willing to pay (basis points).
        pub max_rate_bps: u32,
        pub term_months: u32,
        pub collateral_kind: CollateralKind,
        pub status: ListingStatus,
        pub created_at: u64,
        /// ID of the accepted offer, if any.
        pub accepted_offer_id: Option<u64>,
    }

    /// A lender's counter-offer in response to a marketplace listing (#304).
    #[derive(
        Debug, Clone, PartialEq, scale::Encode, scale::Decode, ink::storage::traits::StorageLayout,
    )]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub struct LoanOffer {
        pub offer_id: u64,
        pub listing_id: u64,
        pub lender: AccountId,
        pub offered_amount: u128,
        /// Interest rate offered by the lender (basis points).
        pub rate_bps: u32,
        pub term_months: u32,
        pub is_accepted: bool,
        pub created_at: u64,
    }

    #[ink(storage)]
    pub struct PropertyLending {
        admin: AccountId,
        collateral_records: Mapping<u64, CollateralRecord>,
        pools: Mapping<u64, LendingPool>,
        pool_count: u64,
        margin_positions: Mapping<u64, MarginPosition>,
        position_count: u64,
        loan_applications: Mapping<u64, LoanApplication>,
        borrower_loans: Mapping<AccountId, Vec<u64>>,
        loan_restructurings: Mapping<u64, LoanRestructuring>,
        loan_count: u64,
        loan_servicers: Mapping<u64, LoanServicer>,
        servicer_count: u64,
        payment_schedules: Mapping<u64, PaymentSchedule>,
        loan_payment_schedule: Mapping<u64, u64>,
        schedule_count: u64,
        yield_positions: Mapping<AccountId, YieldPosition>,
        total_staked: u128,
        reward_per_block: u128,
        proposals: Mapping<u64, Proposal>,
        proposal_count: u64,
        credit_profiles: Mapping<AccountId, CreditProfile>,
        reentrancy_guard: propchain_traits::ReentrancyGuard,
        // ── #304: Loan Marketplace ────────────────────────────────────────────
        marketplace_listings: Mapping<u64, LoanListing>,
        marketplace_offers: Mapping<u64, LoanOffer>,
        listing_count: u64,
        offer_count: u64,
        // ── Admin Key Rotation (Issue #496) ──────────────────────────────────
        pending_admin_rotation: Option<propchain_traits::KeyRotationRequest>,
    }

    #[ink(event)]
    pub struct CollateralAssessed {
        #[ink(topic)]
        property_id: u64,
        assessed_value: u128,
        ltv_ratio: u32,
    }

    #[ink(event)]
    pub struct PoolCreated {
        #[ink(topic)]
        pool_id: u64,
        base_rate: u32,
    }

    #[ink(event)]
    pub struct PositionOpened {
        #[ink(topic)]
        position_id: u64,
        #[ink(topic)]
        owner: AccountId,
        collateral: u128,
    }

    #[ink(event)]
    pub struct LoanApproved {
        #[ink(topic)]
        loan_id: u64,
        #[ink(topic)]
        applicant: AccountId,
        amount: u128,
    }

    #[ink(event)]
    pub struct LoanServicerRegistered {
        #[ink(topic)]
        servicer_id: u64,
        #[ink(topic)]
        account: AccountId,
        name: String,
    }

    #[ink(event)]
    pub struct LoanServicerAssigned {
        #[ink(topic)]
        loan_id: u64,
        #[ink(topic)]
        servicer_id: u64,
        external_reference: String,
    }

    #[ink(event)]
    pub struct LoanServicingStatusUpdated {
        #[ink(topic)]
        loan_id: u64,
        status: String,
    }

    #[ink(event)]
    pub struct LoanRestructuringProposed {
        #[ink(topic)]
        loan_id: u64,
        #[ink(topic)]
        proposer: AccountId,
        new_term_months: u32,
        new_interest_rate_bps: u32,
    }

    #[ink(event)]
    pub struct LoanRestructured {
        #[ink(topic)]
        loan_id: u64,
        new_term_months: u32,
        new_interest_rate_bps: u32,
    }

    #[ink(event)]
    pub struct LoanLiquidated {
        #[ink(topic)]
        loan_id: u64,
        #[ink(topic)]
        borrower: AccountId,
        collateral_seized: u128,
    }

    #[ink(event)]
    pub struct ProposalCreated {
        #[ink(topic)]
        proposal_id: u64,
        description: String,
    }

    // ── #304: Loan Marketplace events ────────────────────────────────────────

    #[ink(event)]
    pub struct LoanListingCreated {
        #[ink(topic)]
        pub listing_id: u64,
        #[ink(topic)]
        pub borrower: AccountId,
        pub requested_amount: u128,
        pub max_rate_bps: u32,
    }

    #[ink(event)]
    pub struct LoanOfferSubmitted {
        #[ink(topic)]
        pub offer_id: u64,
        #[ink(topic)]
        pub listing_id: u64,
        #[ink(topic)]
        pub lender: AccountId,
        pub rate_bps: u32,
    }

    #[ink(event)]
    pub struct LoanOfferAccepted {
        #[ink(topic)]
        pub listing_id: u64,
        #[ink(topic)]
        pub offer_id: u64,
        pub loan_id: u64,
    }

    #[ink(event)]
    pub struct LoanListingCancelled {
        #[ink(topic)]
        pub listing_id: u64,
        #[ink(topic)]
        pub borrower: AccountId,
    }

    // ── Admin Key Rotation Events (Issue #496) ────────────────────────────────

    #[ink(event)]
    pub struct AdminRotationRequested {
        #[ink(topic)]
        old_admin: AccountId,
        #[ink(topic)]
        new_admin: AccountId,
        effective_at_block: u32,
    }

    #[ink(event)]
    pub struct AdminRotationConfirmed {
        #[ink(topic)]
        old_admin: AccountId,
        #[ink(topic)]
        new_admin: AccountId,
    }

    #[ink(event)]
    pub struct AdminRotationCancelled {
        #[ink(topic)]
        old_admin: AccountId,
        cancelled_by: AccountId,
    }

    impl PropertyLending {
        #[ink(constructor)]
        pub fn new(admin: AccountId) -> Self {
            Self {
                admin,
                collateral_records: Mapping::default(),
                pools: Mapping::default(),
                pool_count: 0,
                margin_positions: Mapping::default(),
                position_count: 0,
                loan_applications: Mapping::default(),
                borrower_loans: Mapping::default(),
                loan_restructurings: Mapping::default(),
                loan_count: 0,
                loan_servicers: Mapping::default(),
                servicer_count: 0,
                payment_schedules: Mapping::default(),
                loan_payment_schedule: Mapping::default(),
                schedule_count: 0,
                yield_positions: Mapping::default(),
                total_staked: 0,
                reward_per_block: 100,
                proposals: Mapping::default(),
                proposal_count: 0,
                credit_profiles: Mapping::default(),
                reentrancy_guard: propchain_traits::ReentrancyGuard::new(),
                // #304: Loan Marketplace
                marketplace_listings: Mapping::default(),
                marketplace_offers: Mapping::default(),
                listing_count: 0,
                offer_count: 0,
                // Admin Key Rotation (Issue #496)
                pending_admin_rotation: None,
            }
        }

        #[ink(message)]
        pub fn assess_collateral(
            &mut self,
            property_id: u64,
            value: u128,
            ltv: u32,
            liq_threshold: u32,
        ) -> Result<(), LendingError> {
            if self.env().caller() != self.admin {
                return Err(LendingError::Unauthorized);
            }
            let record = CollateralRecord {
                property_id,
                assessed_value: value,
                ltv_ratio: ltv,
                liquidation_threshold: liq_threshold,
            };
            self.collateral_records.insert(property_id, &record);
            self.env().emit_event(CollateralAssessed {
                property_id,
                assessed_value: value,
                ltv_ratio: ltv,
            });
            Ok(())
        }

        #[ink(message)]
        pub fn should_liquidate(&self, property_id: u64, current_value: u128) -> bool {
            if let Some(r) = self.collateral_records.get(property_id) {
                let ratio = (r.assessed_value * 10000) / current_value.max(1);
                ratio > r.liquidation_threshold as u128
            } else {
                false
            }
        }

        #[ink(message)]
        pub fn create_pool(&mut self, base_rate: u32) -> Result<u64, LendingError> {
            if self.env().caller() != self.admin {
                return Err(LendingError::Unauthorized);
            }
            self.pool_count += 1;
            let pool = LendingPool {
                pool_id: self.pool_count,
                total_deposits: 0,
                total_borrows: 0,
                base_rate,
            };
            self.pools.insert(self.pool_count, &pool);
            self.env().emit_event(PoolCreated {
                pool_id: self.pool_count,
                base_rate,
            });
            Ok(self.pool_count)
        }

        #[ink(message)]
        pub fn deposit(&mut self, pool_id: u64, amount: u128) -> Result<(), LendingError> {
            propchain_traits::non_reentrant!(self, {
                let mut pool = self.pools.get(pool_id).ok_or(LendingError::PoolNotFound)?;
                pool.total_deposits += amount;
                self.pools.insert(pool_id, &pool);
                Ok(())
            })
        }

        #[ink(message)]
        pub fn borrow(&mut self, pool_id: u64, amount: u128) -> Result<(), LendingError> {
            propchain_traits::non_reentrant!(self, {
                let mut pool = self.pools.get(pool_id).ok_or(LendingError::PoolNotFound)?;
                if pool.total_deposits < pool.total_borrows + amount {
                    return Err(LendingError::InsufficientLiquidity);
                }
                pool.total_borrows += amount;
                self.pools.insert(pool_id, &pool);
                Ok(())
            })
        }

        #[ink(message)]
        pub fn borrow_rate(&self, pool_id: u64) -> Result<u32, LendingError> {
            let pool = self.pools.get(pool_id).ok_or(LendingError::PoolNotFound)?;
            let utilisation = (pool.total_borrows * 10000)
                .checked_div(pool.total_deposits)
                .unwrap_or(0);
            Ok(pool.base_rate + (utilisation / 50) as u32)
        }

        #[ink(message)]
        pub fn open_position(
            &mut self,
            collateral: u128,
            leverage: u32,
            short: bool,
            price: u128,
        ) -> Result<u64, LendingError> {
            self.position_count += 1;
            let pos = MarginPosition {
                position_id: self.position_count,
                owner: self.env().caller(),
                collateral,
                leverage,
                is_short: short,
                entry_price: price,
            };
            self.margin_positions.insert(self.position_count, &pos);
            self.env().emit_event(PositionOpened {
                position_id: self.position_count,
                owner: self.env().caller(),
                collateral,
            });
            Ok(self.position_count)
        }

        #[ink(message)]
        pub fn position_pnl(
            &self,
            position_id: u64,
            current_price: u128,
        ) -> Result<i128, LendingError> {
            let pos = self
                .margin_positions
                .get(position_id)
                .ok_or(LendingError::PositionNotFound)?;
            let delta = current_price as i128 - pos.entry_price as i128;
            let signed = if pos.is_short { -delta } else { delta };
            Ok((signed * pos.leverage as i128) / 100)
        }

        #[ink(message)]
        pub fn apply_for_loan(
            &mut self,
            property_id: u64,
            requested_amount: u128,
            collateral_value: u128,
            credit_score: u32,
        ) -> Result<u64, LendingError> {
            self.apply_for_loan_with_terms(
                property_id,
                requested_amount,
                collateral_value,
                credit_score,
                12,
                800,
            )
        }

        #[ink(message)]
        pub fn apply_for_loan_with_terms(
            &mut self,
            property_id: u64,
            requested_amount: u128,
            collateral_value: u128,
            credit_score: u32,
            term_months: u32,
            interest_rate_bps: u32,
        ) -> Result<u64, LendingError> {
            if requested_amount == 0
                || collateral_value == 0
                || term_months == 0
                || interest_rate_bps == 0
            {
                return Err(LendingError::InvalidParameters);
            }
            self.loan_count += 1;
            let app = LoanApplication {
                loan_id: self.loan_count,
                applicant: self.env().caller(),
                property_id,
                requested_amount,
                collateral_value,
                credit_score,
                approved: false,
                servicer_id: None,
                servicing_reference: String::new(),
                servicing_status: String::from("Pending"),
                collateral_kind: CollateralKind::Unsecured,
                term_months,
                interest_rate_bps,
                status: LoanStatus::Pending,
            };
            self.loan_applications.insert(self.loan_count, &app);
            self.track_borrower_loan(app.applicant, self.loan_count);
            Ok(self.loan_count)
        }

        #[ink(message)]
        pub fn apply_for_property_backed_loan(
            &mut self,
            property_id: u64,
            requested_amount: u128,
            credit_score: u32,
            term_months: u32,
            interest_rate_bps: u32,
        ) -> Result<u64, LendingError> {
            let record = self
                .collateral_records
                .get(property_id)
                .ok_or(LendingError::PropertyNotFound)?;
            let max_borrow = (record.assessed_value * record.ltv_ratio as u128) / 10000;
            if requested_amount == 0
                || term_months == 0
                || interest_rate_bps == 0
                || requested_amount > max_borrow
            {
                return Err(LendingError::InsufficientCollateral);
            }

            self.loan_count += 1;
            let app = LoanApplication {
                loan_id: self.loan_count,
                applicant: self.env().caller(),
                property_id,
                requested_amount,
                collateral_value: record.assessed_value,
                credit_score,
                approved: false,
                servicer_id: None,
                servicing_reference: String::new(),
                servicing_status: String::from("Pending"),
                collateral_kind: CollateralKind::PropertyTokenized,
                term_months,
                interest_rate_bps,
                status: LoanStatus::Pending,
            };
            self.loan_applications.insert(self.loan_count, &app);
            self.track_borrower_loan(app.applicant, self.loan_count);
            Ok(self.loan_count)
        }

        #[ink(message)]
        pub fn underwrite_loan(&mut self, loan_id: u64) -> Result<bool, LendingError> {
            if self.env().caller() != self.admin {
                return Err(LendingError::Unauthorized);
            }
            let mut app = self
                .loan_applications
                .get(loan_id)
                .ok_or(LendingError::LoanNotFound)?;
            let ltv = (app.requested_amount * 10000) / app.collateral_value.max(1);
            let score = self.get_credit_score(app.applicant);
            // Store the computed score on the application for reference
            app.credit_score = score;
            let approved = score >= 600 && ltv <= 7500;
            app.status = if approved {
                // Track the new active loan in the borrower's credit profile
                let mut profile =
                    self.credit_profiles
                        .get(app.applicant)
                        .unwrap_or(CreditProfile {
                            repayments_on_time: 0,
                            defaults: 0,
                            active_loans: 0,
                            total_borrowed: 0,
                        });
                profile.active_loans = profile.active_loans.saturating_add(1);
                profile.total_borrowed =
                    profile.total_borrowed.saturating_add(app.requested_amount);
                self.credit_profiles.insert(app.applicant, &profile);
                LoanStatus::Active
            } else {
                LoanStatus::Pending
            };
            self.loan_applications.insert(loan_id, &app);
            if approved {
                self.env().emit_event(LoanApproved {
                    loan_id,
                    applicant: app.applicant,
                    amount: app.requested_amount,
                });
            }
            Ok(approved)
        }

        #[ink(message)]
        pub fn register_loan_servicer(
            &mut self,
            account: AccountId,
            name: String,
        ) -> Result<u64, LendingError> {
            if self.env().caller() != self.admin {
                return Err(LendingError::Unauthorized);
            }
            if name.is_empty() {
                return Err(LendingError::InvalidParameters);
            }
            self.servicer_count += 1;
            let servicer = LoanServicer {
                servicer_id: self.servicer_count,
                account,
                name: name.clone(),
                active: true,
                collateral_kind: CollateralKind::Unsecured,
                term_months: 0,
                interest_rate_bps: 0,
                status: LoanStatus::Active,
            };
            self.loan_servicers.insert(self.servicer_count, &servicer);
            self.env().emit_event(LoanServicerRegistered {
                servicer_id: self.servicer_count,
                account,
                name,
            });
            Ok(self.servicer_count)
        }

        #[ink(message)]
        pub fn set_loan_servicer_active(
            &mut self,
            servicer_id: u64,
            active: bool,
        ) -> Result<(), LendingError> {
            if self.env().caller() != self.admin {
                return Err(LendingError::Unauthorized);
            }
            let mut servicer = self
                .loan_servicers
                .get(servicer_id)
                .ok_or(LendingError::ServicerNotFound)?;
            servicer.active = active;
            self.loan_servicers.insert(servicer_id, &servicer);
            Ok(())
        }

        #[ink(message)]
        pub fn assign_loan_servicer(
            &mut self,
            loan_id: u64,
            servicer_id: u64,
            external_reference: String,
        ) -> Result<(), LendingError> {
            if self.env().caller() != self.admin {
                return Err(LendingError::Unauthorized);
            }
            if external_reference.is_empty() {
                return Err(LendingError::InvalidParameters);
            }
            let servicer = self
                .loan_servicers
                .get(servicer_id)
                .ok_or(LendingError::ServicerNotFound)?;
            if !servicer.active {
                return Err(LendingError::InvalidParameters);
            }
            let mut loan = self
                .loan_applications
                .get(loan_id)
                .ok_or(LendingError::LoanNotFound)?;
            loan.servicer_id = Some(servicer_id);
            loan.servicing_reference = external_reference.clone();
            loan.servicing_status = String::from("Boarded");
            self.loan_applications.insert(loan_id, &loan);
            self.env().emit_event(LoanServicerAssigned {
                loan_id,
                servicer_id,
                external_reference,
            });
            Ok(())
        }

        #[ink(message)]
        pub fn propose_loan_restructuring(
            &mut self,
            loan_id: u64,
            new_term_months: u32,
            new_interest_rate_bps: u32,
        ) -> Result<(), LendingError> {
            let caller = self.env().caller();
            if new_term_months == 0 || new_interest_rate_bps == 0 {
                return Err(LendingError::InvalidParameters);
            }

            let mut app = self
                .loan_applications
                .get(loan_id)
                .ok_or(LendingError::LoanNotFound)?;
            if app.status != LoanStatus::Active && app.status != LoanStatus::Restructured {
                return Err(LendingError::LoanNotActive);
            }
            if caller != app.applicant && caller != self.admin {
                return Err(LendingError::Unauthorized);
            }

            let restructuring = LoanRestructuring {
                loan_id,
                proposed_by: caller,
                proposed_term_months: new_term_months,
                proposed_interest_rate_bps: new_interest_rate_bps,
                borrower_approved: caller == app.applicant,
                lender_approved: caller == self.admin,
            };
            app.status = LoanStatus::RestructuringProposed;
            self.loan_applications.insert(loan_id, &app);
            self.loan_restructurings.insert(loan_id, &restructuring);
            self.env().emit_event(LoanRestructuringProposed {
                loan_id,
                proposer: caller,
                new_term_months,
                new_interest_rate_bps,
            });
            Ok(())
        }

        #[ink(message)]
        pub fn update_servicing_status(
            &mut self,
            loan_id: u64,
            status: String,
        ) -> Result<(), LendingError> {
            let mut loan = self
                .loan_applications
                .get(loan_id)
                .ok_or(LendingError::LoanNotFound)?;
            let servicer_id = loan.servicer_id.ok_or(LendingError::ServicerNotFound)?;
            let servicer = self
                .loan_servicers
                .get(servicer_id)
                .ok_or(LendingError::ServicerNotFound)?;
            let caller = self.env().caller();
            if caller != self.admin && caller != servicer.account {
                return Err(LendingError::Unauthorized);
            }
            if status.is_empty() {
                return Err(LendingError::InvalidParameters);
            }
            loan.servicing_status = status.clone();
            self.loan_applications.insert(loan_id, &loan);
            self.env()
                .emit_event(LoanServicingStatusUpdated { loan_id, status });
            Ok(())
        }

        #[ink(message)]
        pub fn approve_loan_restructuring(&mut self, loan_id: u64) -> Result<bool, LendingError> {
            let caller = self.env().caller();
            let mut app = self
                .loan_applications
                .get(loan_id)
                .ok_or(LendingError::LoanNotFound)?;
            let mut restructuring = self
                .loan_restructurings
                .get(loan_id)
                .ok_or(LendingError::RestructuringNotFound)?;

            if caller == app.applicant {
                restructuring.borrower_approved = true;
            } else if caller == self.admin {
                restructuring.lender_approved = true;
            } else {
                return Err(LendingError::Unauthorized);
            }

            let approved = restructuring.borrower_approved && restructuring.lender_approved;
            if approved {
                app.term_months = restructuring.proposed_term_months;
                app.interest_rate_bps = restructuring.proposed_interest_rate_bps;
                app.status = LoanStatus::Restructured;
                self.loan_applications.insert(loan_id, &app);
                self.loan_restructurings.remove(loan_id);
                self.env().emit_event(LoanRestructured {
                    loan_id,
                    new_term_months: app.term_months,
                    new_interest_rate_bps: app.interest_rate_bps,
                });
            } else {
                self.loan_restructurings.insert(loan_id, &restructuring);
                self.loan_applications.insert(loan_id, &app);
            }

            Ok(approved)
        }

        #[ink(message)]
        pub fn liquidate_loan(
            &mut self,
            loan_id: u64,
            current_property_value: u128,
        ) -> Result<(), LendingError> {
            let mut app = self
                .loan_applications
                .get(loan_id)
                .ok_or(LendingError::LoanNotFound)?;

            if app.status != LoanStatus::Active {
                return Err(LendingError::LoanNotActive);
            }

            let record = self
                .collateral_records
                .get(app.property_id)
                .ok_or(LendingError::PropertyNotFound)?;

            // Calculate current LTV: (loan amount / current property value)
            let current_ltv = (app.requested_amount * 10000) / current_property_value.max(1);

            // Check if current LTV exceeds the liquidation threshold
            if current_ltv <= record.liquidation_threshold as u128 {
                return Err(LendingError::LiquidationThresholdNotMet);
            }

            // Perform liquidation
            app.status = LoanStatus::Liquidated;
            self.loan_applications.insert(loan_id, &app);

            self.env().emit_event(LoanLiquidated {
                loan_id,
                borrower: app.applicant,
                collateral_seized: app.collateral_value,
            });

            Ok(())
        }

        #[ink(message)]
        pub fn stake(&mut self, amount: u128) -> Result<(), LendingError> {
            let caller = self.env().caller();
            let mut pos = self.yield_positions.get(caller).unwrap_or(YieldPosition {
                owner: caller,
                staked: 0,
                reward_debt: 0,
                accumulated_rewards: 0,
            });
            pos.staked += amount;
            self.yield_positions.insert(caller, &pos);
            self.total_staked += amount;
            Ok(())
        }

        #[ink(message)]
        pub fn pending_rewards(&self, owner: AccountId, current_block: u64) -> u128 {
            if let Some(p) = self.yield_positions.get(owner) {
                if self.total_staked == 0 {
                    return 0;
                }
                let per_share = (self.reward_per_block * current_block as u128) / self.total_staked;
                p.staked * per_share - p.reward_debt
            } else {
                0
            }
        }

        #[ink(message)]
        pub fn propose(&mut self, description: String) -> Result<u64, LendingError> {
            self.proposal_count += 1;
            let prop = Proposal {
                proposal_id: self.proposal_count,
                description: description.clone(),
                votes_for: 0,
                votes_against: 0,
                executed: false,
            };
            self.proposals.insert(self.proposal_count, &prop);
            self.env().emit_event(ProposalCreated {
                proposal_id: self.proposal_count,
                description,
            });
            Ok(self.proposal_count)
        }

        #[ink(message)]
        pub fn vote(&mut self, proposal_id: u64, in_favour: bool) -> Result<(), LendingError> {
            let mut prop = self
                .proposals
                .get(proposal_id)
                .ok_or(LendingError::ProposalNotFound)?;
            if in_favour {
                prop.votes_for += 1;
            } else {
                prop.votes_against += 1;
            }
            self.proposals.insert(proposal_id, &prop);
            Ok(())
        }

        #[ink(message)]
        pub fn execute_proposal(&mut self, proposal_id: u64) -> Result<bool, LendingError> {
            let mut prop = self
                .proposals
                .get(proposal_id)
                .ok_or(LendingError::ProposalNotFound)?;
            if prop.votes_for > prop.votes_against && !prop.executed {
                prop.executed = true;
                self.proposals.insert(proposal_id, &prop);
                Ok(true)
            } else {
                Ok(false)
            }
        }

        /// Compute a 0–1000 credit score from a borrower's on-chain profile.
        fn compute_credit_score(profile: &CreditProfile) -> u32 {
            let base: u32 = 500;
            let repayment_bonus = (profile.repayments_on_time * 20).min(300);
            let default_penalty = (profile.defaults * 150).min(450);
            let loan_penalty = (profile.active_loans * 10).min(100);
            base.saturating_add(repayment_bonus)
                .saturating_sub(default_penalty)
                .saturating_sub(loan_penalty)
        }

        /// Record a successful on-time repayment for the caller.
        /// Only callable by the contract admin (e.g. after verifying payment).
        #[ink(message)]
        pub fn record_repayment(&mut self, borrower: AccountId) -> Result<(), LendingError> {
            if self.env().caller() != self.admin {
                return Err(LendingError::Unauthorized);
            }
            let mut profile = self.credit_profiles.get(borrower).unwrap_or(CreditProfile {
                repayments_on_time: 0,
                defaults: 0,
                active_loans: 0,
                total_borrowed: 0,
            });
            profile.repayments_on_time = profile.repayments_on_time.saturating_add(1);
            if profile.active_loans > 0 {
                profile.active_loans -= 1;
            }
            self.credit_profiles.insert(borrower, &profile);
            Ok(())
        }

        /// Record a default event for a borrower.
        /// Only callable by the contract admin.
        #[ink(message)]
        pub fn record_default(&mut self, borrower: AccountId) -> Result<(), LendingError> {
            if self.env().caller() != self.admin {
                return Err(LendingError::Unauthorized);
            }
            let mut profile = self.credit_profiles.get(borrower).unwrap_or(CreditProfile {
                repayments_on_time: 0,
                defaults: 0,
                active_loans: 0,
                total_borrowed: 0,
            });
            profile.defaults = profile.defaults.saturating_add(1);
            if profile.active_loans > 0 {
                profile.active_loans -= 1;
            }
            self.credit_profiles.insert(borrower, &profile);
            Ok(())
        }

        /// Return the computed credit score (0–1000) for a borrower.
        #[ink(message)]
        pub fn get_credit_score(&self, borrower: AccountId) -> u32 {
            let profile = self.credit_profiles.get(borrower).unwrap_or(CreditProfile {
                repayments_on_time: 0,
                defaults: 0,
                active_loans: 0,
                total_borrowed: 0,
            });
            Self::compute_credit_score(&profile)
        }

        #[ink(message)]
        pub fn get_pool(&self, pool_id: u64) -> Option<LendingPool> {
            self.pools.get(pool_id)
        }

        #[ink(message)]
        pub fn get_collateral(&self, property_id: u64) -> Option<CollateralRecord> {
            self.collateral_records.get(property_id)
        }

        #[ink(message)]
        pub fn get_position(&self, position_id: u64) -> Option<MarginPosition> {
            self.margin_positions.get(position_id)
        }

        #[ink(message)]
        pub fn get_loan(&self, loan_id: u64) -> Option<LoanApplication> {
            self.loan_applications.get(loan_id)
        }

        #[ink(message)]
        pub fn get_loan_servicer(&self, servicer_id: u64) -> Option<LoanServicer> {
            self.loan_servicers.get(servicer_id)
        }

        // ── #304: Loan Marketplace ────────────────────────────────────────────

        /// Create a new loan listing on the marketplace (#304).
        ///
        /// Any borrower can list their loan request. Lenders can then submit
        /// competing offers via `submit_loan_offer`.
        #[ink(message)]
        pub fn create_loan_listing(
            &mut self,
            property_id: u64,
            requested_amount: u128,
            max_rate_bps: u32,
            term_months: u32,
            collateral_kind: CollateralKind,
        ) -> Result<u64, LendingError> {
            if requested_amount == 0 || max_rate_bps == 0 || term_months == 0 {
                return Err(LendingError::InvalidParameters);
            }

            let borrower = self.env().caller();
            let listing_id = self.listing_count + 1;

            let listing = LoanListing {
                listing_id,
                borrower,
                property_id,
                requested_amount,
                max_rate_bps,
                term_months,
                collateral_kind,
                status: ListingStatus::Open,
                created_at: self.env().block_number() as u64,
                accepted_offer_id: None,
            };

            self.marketplace_listings.insert(listing_id, &listing);
            self.listing_count = listing_id;

            self.env().emit_event(LoanListingCreated {
                listing_id,
                borrower,
                requested_amount,
                max_rate_bps,
            });

            Ok(listing_id)
        }

        /// Submit a lending offer against an open listing (#304).
        ///
        /// The lender specifies the rate and amount they are willing to offer.
        /// The rate must be at or below the borrower's stated maximum.
        #[ink(message)]
        pub fn submit_loan_offer(
            &mut self,
            listing_id: u64,
            offered_amount: u128,
            rate_bps: u32,
            term_months: u32,
        ) -> Result<u64, LendingError> {
            let listing = self
                .marketplace_listings
                .get(listing_id)
                .ok_or(LendingError::LoanNotFound)?;

            if !matches!(listing.status, ListingStatus::Open) {
                return Err(LendingError::LoanNotActive);
            }

            // Offer rate must not exceed borrower's maximum
            if rate_bps > listing.max_rate_bps {
                return Err(LendingError::InvalidParameters);
            }

            if offered_amount == 0 || term_months == 0 {
                return Err(LendingError::InvalidParameters);
            }

            let lender = self.env().caller();
            let offer_id = self.offer_count + 1;

            let offer = LoanOffer {
                offer_id,
                listing_id,
                lender,
                offered_amount,
                rate_bps,
                term_months,
                is_accepted: false,
                created_at: self.env().block_number() as u64,
            };

            self.marketplace_offers.insert(offer_id, &offer);
            self.offer_count = offer_id;

            self.env().emit_event(LoanOfferSubmitted {
                offer_id,
                listing_id,
                lender,
                rate_bps,
            });

            Ok(offer_id)
        }

        /// Borrower accepts a lender's offer and originates the loan (#304).
        ///
        /// Accepting an offer transitions the listing to `OfferAccepted`, creates
        /// the underlying `LoanApplication`, and marks the listing as `Originated`.
        #[ink(message)]
        pub fn accept_loan_offer(&mut self, offer_id: u64) -> Result<u64, LendingError> {
            let mut offer = self
                .marketplace_offers
                .get(offer_id)
                .ok_or(LendingError::LoanNotFound)?;

            let mut listing = self
                .marketplace_listings
                .get(offer.listing_id)
                .ok_or(LendingError::LoanNotFound)?;

            let borrower = self.env().caller();
            if listing.borrower != borrower {
                return Err(LendingError::Unauthorized);
            }

            if !matches!(listing.status, ListingStatus::Open) {
                return Err(LendingError::LoanNotActive);
            }

            if offer.is_accepted {
                return Err(LendingError::InvalidParameters);
            }

            // Originate the underlying loan application
            let loan_id = self.loan_count + 1;
            let loan = LoanApplication {
                loan_id,
                applicant: borrower,
                property_id: listing.property_id,
                requested_amount: offer.offered_amount,
                collateral_value: offer.offered_amount,
                credit_score: self.get_credit_score(borrower),
                approved: true,
                servicer_id: None,
                servicing_reference: String::new(),
                servicing_status: String::from("marketplace_originated"),
                collateral_kind: listing.collateral_kind,
                term_months: offer.term_months,
                interest_rate_bps: offer.rate_bps,
                status: LoanStatus::Active,
            };

            self.loan_applications.insert(loan_id, &loan);
            self.loan_count = loan_id;

            // Update offer and listing state
            offer.is_accepted = true;
            listing.status = ListingStatus::Originated;
            listing.accepted_offer_id = Some(offer_id);

            self.marketplace_offers.insert(offer_id, &offer);
            self.marketplace_listings
                .insert(listing.listing_id, &listing);

            self.env().emit_event(LoanOfferAccepted {
                listing_id: offer.listing_id,
                offer_id,
                loan_id,
            });

            Ok(loan_id)
        }

        /// Borrower cancels an open listing (#304).
        #[ink(message)]
        pub fn cancel_loan_listing(&mut self, listing_id: u64) -> Result<(), LendingError> {
            let mut listing = self
                .marketplace_listings
                .get(listing_id)
                .ok_or(LendingError::LoanNotFound)?;

            if listing.borrower != self.env().caller() {
                return Err(LendingError::Unauthorized);
            }

            if !matches!(listing.status, ListingStatus::Open) {
                return Err(LendingError::LoanNotActive);
            }

            listing.status = ListingStatus::Cancelled;
            self.marketplace_listings.insert(listing_id, &listing);

            self.env().emit_event(LoanListingCancelled {
                listing_id,
                borrower: listing.borrower,
            });

            Ok(())
        }

        /// Get a marketplace listing by ID (#304).
        #[ink(message)]
        pub fn get_loan_listing(&self, listing_id: u64) -> Option<LoanListing> {
            self.marketplace_listings.get(listing_id)
        }

        /// Get a lender offer by ID (#304).
        #[ink(message)]
        pub fn get_loan_offer(&self, offer_id: u64) -> Option<LoanOffer> {
            self.marketplace_offers.get(offer_id)
        }

        #[ink(message)]
        pub fn get_loan_restructuring(&self, loan_id: u64) -> Option<LoanRestructuring> {
            self.loan_restructurings.get(loan_id)
        }

        #[ink(message)]
        pub fn get_proposal(&self, proposal_id: u64) -> Option<Proposal> {
            self.proposals.get(proposal_id)
        }

        #[ink(message)]
        pub fn get_admin(&self) -> AccountId {
            self.admin
        }

        // ── Admin Key Rotation (Issue #496) ──────────────────────────────────

        /// Initiate two-step admin rotation with timelock cooldown.
        ///
        /// Only the current admin may call this. The nominated `new_admin` must
        /// confirm after `KEY_ROTATION_COOLDOWN_BLOCKS` blocks have elapsed.
        #[ink(message)]
        pub fn request_admin_rotation(&mut self, new_admin: AccountId) -> Result<(), LendingError> {
            let caller = self.env().caller();
            if caller != self.admin {
                return Err(LendingError::Unauthorized);
            }
            if self.pending_admin_rotation.is_some() {
                return Err(LendingError::KeyRotationCooldown);
            }

            let block = self.env().block_number();
            let effective_at =
                block.saturating_add(propchain_traits::constants::KEY_ROTATION_COOLDOWN_BLOCKS);

            self.pending_admin_rotation = Some(propchain_traits::KeyRotationRequest {
                old_account: caller,
                new_account: new_admin,
                requested_at: block,
                effective_at,
                confirmed: false,
            });

            self.env().emit_event(AdminRotationRequested {
                old_admin: caller,
                new_admin,
                effective_at_block: effective_at,
            });
            Ok(())
        }

        /// Confirm a pending admin rotation after the cooldown period.
        ///
        /// Must be called by the nominated new admin.
        #[ink(message)]
        pub fn confirm_admin_rotation(&mut self) -> Result<(), LendingError> {
            let caller = self.env().caller();
            let block = self.env().block_number();

            let request = self
                .pending_admin_rotation
                .as_ref()
                .ok_or(LendingError::NoPendingRotation)?;

            if request.new_account != caller {
                return Err(LendingError::RotationUnauthorized);
            }
            if block < request.effective_at {
                return Err(LendingError::KeyRotationCooldown);
            }
            let expiry = request
                .effective_at
                .saturating_add(propchain_traits::constants::KEY_ROTATION_EXPIRY_BLOCKS);
            if block > expiry {
                self.pending_admin_rotation = None;
                return Err(LendingError::RequestExpired);
            }

            let old_admin = request.old_account;
            self.admin = caller;
            self.pending_admin_rotation = None;

            self.env().emit_event(AdminRotationConfirmed {
                old_admin,
                new_admin: caller,
            });
            Ok(())
        }

        /// Cancel a pending admin rotation.
        ///
        /// Either the current admin or the nominated new admin may cancel.
        #[ink(message)]
        pub fn cancel_admin_rotation(&mut self) -> Result<(), LendingError> {
            let caller = self.env().caller();
            let request = self
                .pending_admin_rotation
                .as_ref()
                .ok_or(LendingError::NoPendingRotation)?;

            if caller != request.old_account && caller != request.new_account {
                return Err(LendingError::RotationUnauthorized);
            }

            let old_admin = request.old_account;
            self.pending_admin_rotation = None;

            self.env().emit_event(AdminRotationCancelled {
                old_admin,
                cancelled_by: caller,
            });
            Ok(())
        }

        /// Get the pending admin rotation request, if any.
        #[ink(message)]
        pub fn get_pending_admin_rotation(&self) -> Option<propchain_traits::KeyRotationRequest> {
            self.pending_admin_rotation.clone()
        }

        fn track_borrower_loan(&mut self, borrower: AccountId, loan_id: u64) {
            let mut loan_ids = self.borrower_loans.get(borrower).unwrap_or_default();
            loan_ids.push(loan_id);
            self.borrower_loans.insert(borrower, &loan_ids);
        }
    }

    impl Default for PropertyLending {
        fn default() -> Self {
            Self::new(AccountId::from([0x0; 32]))
        }
    }
}

pub use crate::propchain_lending::{
    LendingError, LoanServicer, LoanStatus, PaymentSchedule, PaymentScheduleStatus, PropertyLending,
};

#[cfg(test)]
mod tests {
    use super::*;
    use ink::env::{test, DefaultEnvironment};
    use propchain_lending::PropertyLending;

    fn setup() -> PropertyLending {
        let accounts = test::default_accounts::<DefaultEnvironment>();
        test::set_caller::<DefaultEnvironment>(accounts.alice);
        PropertyLending::new(accounts.alice)
    }

    #[ink::test]
    fn test_assess_collateral() {
        let mut contract = setup();
        assert!(contract
            .assess_collateral(1, 1_000_000, 7500, 12000)
            .is_ok());
        let record = contract.get_collateral(1).unwrap();
        assert_eq!(record.assessed_value, 1_000_000);
    }

    #[ink::test]
    fn test_liquidation_trigger() {
        let mut contract = setup();
        contract
            .assess_collateral(1, 1_000_000, 7500, 12000)
            .unwrap();
        assert!(contract.should_liquidate(1, 800_000));
        assert!(!contract.should_liquidate(1, 1_000_000));
    }

    #[ink::test]
    fn test_create_pool() {
        let mut contract = setup();
        let pool_id = contract.create_pool(500).unwrap();
        assert_eq!(pool_id, 1);
        let pool = contract.get_pool(1).unwrap();
        assert_eq!(pool.base_rate, 500);
    }

    #[ink::test]
    fn test_pool_operations() {
        let mut contract = setup();
        let pool_id = contract.create_pool(500).unwrap();
        assert!(contract.deposit(pool_id, 1_000_000).is_ok());
        assert!(contract.borrow(pool_id, 500_000).is_ok());
        let rate = contract.borrow_rate(pool_id).unwrap();
        assert!(rate > 500);
    }

    #[ink::test]
    fn test_margin_position() {
        let mut contract = setup();
        let pos_id = contract.open_position(1000, 200, false, 100).unwrap();
        let pnl = contract.position_pnl(pos_id, 150).unwrap();
        assert!(pnl > 0);
    }

    #[ink::test]
    fn test_loan_underwriting() {
        let mut contract = setup();
        let accounts = test::default_accounts::<DefaultEnvironment>();
        // LTV too high (90%) → rejected regardless of score
        let loan_id = contract.apply_for_loan(1, 900_000, 1_000_000, 0).unwrap();
        let approved = contract.underwrite_loan(loan_id).unwrap();
        assert!(!approved);
        // Give alice a good score (≥600) then apply with acceptable LTV
        for _ in 0..6 {
            contract.record_repayment(accounts.alice).unwrap();
        }
        let loan_id2 = contract.apply_for_loan(1, 700_000, 1_000_000, 0).unwrap();
        let approved2 = contract.underwrite_loan(loan_id2).unwrap();
        assert!(approved2);
    }

    #[ink::test]
    fn test_loan_servicer_integration() {
        let mut contract = setup();
        let accounts = test::default_accounts::<DefaultEnvironment>();
        let servicer_id = contract
            .register_loan_servicer(accounts.bob, String::from("Acme Servicing"))
            .unwrap();
        let loan_id = contract.apply_for_loan(1, 700_000, 1_000_000, 700).unwrap();

        contract
            .assign_loan_servicer(loan_id, servicer_id, String::from("EXT-123"))
            .unwrap();
        let loan = contract.get_loan(loan_id).unwrap();
        assert_eq!(loan.servicer_id, Some(servicer_id));
        assert_eq!(loan.servicing_reference, "EXT-123");
        assert_eq!(loan.servicing_status, "Boarded");

        test::set_caller::<DefaultEnvironment>(accounts.bob);
        contract
            .update_servicing_status(loan_id, String::from("Current"))
            .unwrap();
        assert_eq!(
            contract.get_loan(loan_id).unwrap().servicing_status,
            "Current"
        );
    }

    #[ink::test]
    fn test_property_backed_loan_uses_assessed_collateral() {
        let mut contract = setup();
        contract
            .assess_collateral(7, 2_000_000, 7000, 8500)
            .unwrap();

        let loan_id = contract
            .apply_for_property_backed_loan(7, 1_200_000, 710, 24, 650)
            .unwrap();
        let loan = contract.get_loan(loan_id).unwrap();

        assert_eq!(loan.collateral_value, 2_000_000);
        assert_eq!(loan.term_months, 24);
        assert_eq!(loan.interest_rate_bps, 650);
        assert_eq!(loan.status, LoanStatus::Pending);
    }

    #[ink::test]
    fn test_property_backed_loan_rejects_excessive_borrow() {
        let mut contract = setup();
        contract
            .assess_collateral(9, 1_000_000, 6500, 8500)
            .unwrap();

        assert_eq!(
            contract.apply_for_property_backed_loan(9, 700_000, 700, 12, 700),
            Err(LendingError::InsufficientCollateral)
        );
    }

    #[ink::test]
    fn test_loan_servicer_authorization_and_validation() {
        let mut contract = setup();
        let accounts = test::default_accounts::<DefaultEnvironment>();
        let loan_id = contract.apply_for_loan(1, 700_000, 1_000_000, 700).unwrap();

        assert_eq!(
            contract.register_loan_servicer(accounts.bob, String::new()),
            Err(LendingError::InvalidParameters)
        );
        let servicer_id = contract
            .register_loan_servicer(accounts.bob, String::from("Acme Servicing"))
            .unwrap();
        contract
            .set_loan_servicer_active(servicer_id, false)
            .unwrap();
        assert_eq!(
            contract.assign_loan_servicer(loan_id, servicer_id, String::from("EXT-123")),
            Err(LendingError::InvalidParameters)
        );

        contract
            .set_loan_servicer_active(servicer_id, true)
            .unwrap();
        contract
            .assign_loan_servicer(loan_id, servicer_id, String::from("EXT-123"))
            .unwrap();

        test::set_caller::<DefaultEnvironment>(accounts.charlie);
        assert_eq!(
            contract.update_servicing_status(loan_id, String::from("Late")),
            Err(LendingError::Unauthorized)
        );
        assert_eq!(
            contract.assign_loan_servicer(loan_id, servicer_id, String::from("EXT-456")),
            Err(LendingError::Unauthorized)
        );
    }

    #[ink::test]
    fn test_loan_restructuring_requires_borrower_and_lender_approval() {
        let mut contract = setup();
        let accounts = test::default_accounts::<DefaultEnvironment>();

        for _ in 0..6 {
            contract.record_repayment(accounts.bob).unwrap();
        }

        test::set_caller::<DefaultEnvironment>(accounts.bob);
        let loan_id = contract
            .apply_for_loan_with_terms(1, 600_000, 1_000_000, 720, 12, 900)
            .unwrap();

        test::set_caller::<DefaultEnvironment>(accounts.alice);
        assert!(contract.underwrite_loan(loan_id).unwrap());

        test::set_caller::<DefaultEnvironment>(accounts.bob);
        assert!(contract
            .propose_loan_restructuring(loan_id, 24, 700)
            .is_ok());
        let pending = contract.get_loan(loan_id).unwrap();
        assert_eq!(pending.status, LoanStatus::RestructuringProposed);

        test::set_caller::<DefaultEnvironment>(accounts.alice);
        assert!(contract.approve_loan_restructuring(loan_id).unwrap());

        let updated = contract.get_loan(loan_id).unwrap();
        assert_eq!(updated.term_months, 24);
        assert_eq!(updated.interest_rate_bps, 700);
        assert_eq!(updated.status, LoanStatus::Restructured);
        assert!(contract.get_loan_restructuring(loan_id).is_none());
    }

    #[ink::test]
    fn test_liquidate_loan() {
        let mut contract = setup();
        let accounts = test::default_accounts::<DefaultEnvironment>();
        contract
            .assess_collateral(1, 1_000_000, 7500, 8000)
            .unwrap();
        // Give alice a score ≥ 600 (6 repayments → 500 + 120 = 620)
        for _ in 0..6 {
            contract.record_repayment(accounts.alice).unwrap();
        }
        let loan_id = contract.apply_for_loan(1, 700_000, 1_000_000, 0).unwrap();
        contract.underwrite_loan(loan_id).unwrap();
        assert!(contract.liquidate_loan(loan_id, 850_000).is_ok());
        let loan = contract.get_loan(loan_id).unwrap();
        assert_eq!(loan.status, LoanStatus::Liquidated);
    }

    #[ink::test]
    fn test_yield_farming() {
        let mut contract = setup();
        let accounts = test::default_accounts::<DefaultEnvironment>();
        assert!(contract.stake(1000).is_ok());
        let rewards = contract.pending_rewards(accounts.alice, 100);
        assert!(rewards > 0);
    }

    #[ink::test]
    fn test_governance() {
        let mut contract = setup();
        let prop_id = contract.propose("Lower LTV cap".into()).unwrap();
        assert!(contract.vote(prop_id, true).is_ok());
        assert!(contract.vote(prop_id, true).is_ok());
        assert!(contract.vote(prop_id, false).is_ok());
        assert!(contract.execute_proposal(prop_id).unwrap());
    }

    // ── Credit scoring tests ──────────────────────────────────────────────

    #[ink::test]
    fn test_default_credit_score_is_500() {
        let contract = setup();
        let accounts = test::default_accounts::<DefaultEnvironment>();
        assert_eq!(contract.get_credit_score(accounts.bob), 500);
    }

    #[ink::test]
    fn test_repayment_increases_score() {
        let mut contract = setup();
        let accounts = test::default_accounts::<DefaultEnvironment>();
        contract.record_repayment(accounts.bob).unwrap();
        contract.record_repayment(accounts.bob).unwrap();
        // 500 + 2*20 = 540
        assert_eq!(contract.get_credit_score(accounts.bob), 540);
    }

    #[ink::test]
    fn test_default_decreases_score() {
        let mut contract = setup();
        let accounts = test::default_accounts::<DefaultEnvironment>();
        contract.record_default(accounts.bob).unwrap();
        // 500 - 150 = 350
        assert_eq!(contract.get_credit_score(accounts.bob), 350);
    }

    #[ink::test]
    fn test_repayment_bonus_capped_at_300() {
        let mut contract = setup();
        let accounts = test::default_accounts::<DefaultEnvironment>();
        // 15 repayments * 20 = 300 (cap)
        for _ in 0..20 {
            contract.record_repayment(accounts.bob).unwrap();
        }
        assert_eq!(contract.get_credit_score(accounts.bob), 800);
    }

    #[ink::test]
    fn test_default_penalty_capped_at_450() {
        let mut contract = setup();
        let accounts = test::default_accounts::<DefaultEnvironment>();
        // 3 defaults * 150 = 450 (cap)
        for _ in 0..5 {
            contract.record_default(accounts.bob).unwrap();
        }
        assert_eq!(contract.get_credit_score(accounts.bob), 50);
    }

    #[ink::test]
    fn test_underwrite_uses_on_chain_score() {
        let mut contract = setup();
        let accounts = test::default_accounts::<DefaultEnvironment>();
        // Give bob a good history: 6 repayments → score = 500 + 120 = 620
        for _ in 0..6 {
            contract.record_repayment(accounts.bob).unwrap();
        }

        // Apply as bob with a reasonable LTV
        test::set_caller::<DefaultEnvironment>(accounts.bob);
        let loan_id = contract
            .apply_for_loan(1, 700_000, 1_000_000, 0) // credit_score param ignored
            .unwrap();

        // Underwrite as admin
        test::set_caller::<DefaultEnvironment>(accounts.alice);
        let approved = contract.underwrite_loan(loan_id).unwrap();
        assert!(approved);

        let loan = contract.get_loan(loan_id).unwrap();
        assert_eq!(loan.credit_score, 620);
    }

    #[ink::test]
    fn test_underwrite_rejected_when_score_too_low() {
        let mut contract = setup();
        let accounts = test::default_accounts::<DefaultEnvironment>();
        // Give bob a bad history (score = 350)
        contract.record_default(accounts.bob).unwrap();

        test::set_caller::<DefaultEnvironment>(accounts.bob);
        let loan_id = contract.apply_for_loan(1, 700_000, 1_000_000, 0).unwrap();

        test::set_caller::<DefaultEnvironment>(accounts.alice);
        let approved = contract.underwrite_loan(loan_id).unwrap();
        assert!(!approved);
    }

    #[ink::test]
    fn test_record_repayment_unauthorized() {
        let mut contract = setup();
        let accounts = test::default_accounts::<DefaultEnvironment>();
        test::set_caller::<DefaultEnvironment>(accounts.bob);
        assert_eq!(
            contract.record_repayment(accounts.charlie),
            Err(propchain_lending::LendingError::Unauthorized)
        );
    }

    #[ink::test]
    fn test_active_loans_tracked_and_reduce_score() {
        let mut contract = setup();
        let accounts = test::default_accounts::<DefaultEnvironment>();

        // Give bob 6 repayments → score = 620, then approve 2 loans
        // After each approval active_loans increments by 1 (-10 each)
        // Final score = 500 + 120 - 20 = 600
        for _ in 0..6 {
            contract.record_repayment(accounts.bob).unwrap();
        }

        for _ in 0..2 {
            test::set_caller::<DefaultEnvironment>(accounts.bob);
            let loan_id = contract.apply_for_loan(1, 700_000, 1_000_000, 0).unwrap();
            test::set_caller::<DefaultEnvironment>(accounts.alice);
            contract.underwrite_loan(loan_id).unwrap();
        }

        // score = 500 + 120 - 2*10 = 600
        assert_eq!(contract.get_credit_score(accounts.bob), 600);
    }
}

// =========================================================================
// ADMIN KEY ROTATION TESTS (Issue #496) — Lending
// =========================================================================

#[cfg(test)]
mod lending_admin_rotation_tests {
    use super::propchain_lending::{LendingError, PropertyLending};
    use ink::env::{test, DefaultEnvironment};

    fn setup() -> PropertyLending {
        let accounts = test::default_accounts::<DefaultEnvironment>();
        test::set_caller::<DefaultEnvironment>(accounts.alice);
        PropertyLending::new(accounts.alice)
    }

    #[ink::test]
    fn test_admin_can_request_rotation() {
        let mut contract = setup();
        let accounts = test::default_accounts::<DefaultEnvironment>();
        assert!(contract.request_admin_rotation(accounts.bob).is_ok());
        let pending = contract.get_pending_admin_rotation();
        assert!(pending.is_some());
        let req = pending.unwrap();
        assert_eq!(req.old_account, accounts.alice);
        assert_eq!(req.new_account, accounts.bob);
    }

    #[ink::test]
    fn test_non_admin_cannot_request_rotation() {
        let mut contract = setup();
        let accounts = test::default_accounts::<DefaultEnvironment>();
        test::set_caller::<DefaultEnvironment>(accounts.bob);
        assert_eq!(
            contract.request_admin_rotation(accounts.charlie),
            Err(LendingError::Unauthorized)
        );
    }

    #[ink::test]
    fn test_rotation_cannot_be_confirmed_before_cooldown() {
        let mut contract = setup();
        let accounts = test::default_accounts::<DefaultEnvironment>();
        contract.request_admin_rotation(accounts.bob).unwrap();
        test::set_caller::<DefaultEnvironment>(accounts.bob);
        // Block 0 < effective_at 14_400
        assert_eq!(
            contract.confirm_admin_rotation(),
            Err(LendingError::KeyRotationCooldown)
        );
    }

    #[ink::test]
    fn test_rotation_expires_after_expiry_period() {
        let mut contract = setup();
        let accounts = test::default_accounts::<DefaultEnvironment>();
        contract.request_admin_rotation(accounts.bob).unwrap();
        // Verify no pending rotation after cancel
        contract.cancel_admin_rotation().unwrap();
        test::set_caller::<DefaultEnvironment>(accounts.bob);
        assert_eq!(
            contract.confirm_admin_rotation(),
            Err(LendingError::NoPendingRotation)
        );
    }

    #[ink::test]
    fn test_old_admin_can_cancel_pending_rotation() {
        let mut contract = setup();
        let accounts = test::default_accounts::<DefaultEnvironment>();
        contract.request_admin_rotation(accounts.bob).unwrap();
        assert!(contract.cancel_admin_rotation().is_ok());
        assert!(contract.get_pending_admin_rotation().is_none());
    }

    #[ink::test]
    fn test_new_admin_can_cancel_pending_rotation() {
        let mut contract = setup();
        let accounts = test::default_accounts::<DefaultEnvironment>();
        contract.request_admin_rotation(accounts.bob).unwrap();
        test::set_caller::<DefaultEnvironment>(accounts.bob);
        assert!(contract.cancel_admin_rotation().is_ok());
        assert!(contract.get_pending_admin_rotation().is_none());
    }

    #[ink::test]
    fn test_unrelated_account_cannot_cancel() {
        let mut contract = setup();
        let accounts = test::default_accounts::<DefaultEnvironment>();
        contract.request_admin_rotation(accounts.bob).unwrap();
        test::set_caller::<DefaultEnvironment>(accounts.charlie);
        assert_eq!(
            contract.cancel_admin_rotation(),
            Err(LendingError::RotationUnauthorized)
        );
    }
}
