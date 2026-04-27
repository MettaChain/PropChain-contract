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
    use ink::prelude::string::String;

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
        ReentrantCall,
    }

    #[derive(
        Debug, Clone, PartialEq, scale::Encode, scale::Decode, ink::storage::traits::StorageLayout,
    )]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub enum LoanStatus {
        Pending,
        Active,
        Repaid,
        Defaulted,
        Liquidated,
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
        pub collateral_kind: CollateralKind,
        pub term_months: u32,
        pub interest_rate_bps: u32,
        pub status: LoanStatus,
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

    #[ink(storage)]
    pub struct PropertyLending {
        admin: AccountId,
        collateral_records: Mapping<u64, CollateralRecord>,
        pools: Mapping<u64, LendingPool>,
        pool_count: u64,
        margin_positions: Mapping<u64, MarginPosition>,
        position_count: u64,
        loan_applications: Mapping<u64, LoanApplication>,
        loan_restructurings: Mapping<u64, LoanRestructuring>,
        loan_count: u64,
        yield_positions: Mapping<AccountId, YieldPosition>,
        total_staked: u128,
        reward_per_block: u128,
        proposals: Mapping<u64, Proposal>,
        proposal_count: u64,
        credit_profiles: Mapping<AccountId, CreditProfile>,
        reentrancy_guard: propchain_traits::ReentrancyGuard,
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
                loan_restructurings: Mapping::default(),
                loan_count: 0,
                yield_positions: Mapping::default(),
                total_staked: 0,
                reward_per_block: 100,
                proposals: Mapping::default(),
                proposal_count: 0,
                credit_profiles: Mapping::default(),
                reentrancy_guard: propchain_traits::ReentrancyGuard::new(),
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
                collateral_kind: CollateralKind::Unsecured,
                term_months,
                interest_rate_bps,
                status: LoanStatus::Pending,
            };
            self.loan_applications.insert(self.loan_count, &app);
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
                collateral_kind: CollateralKind::PropertyTokenized,
                term_months,
                interest_rate_bps,
                status: LoanStatus::Pending,
            };
            self.loan_applications.insert(self.loan_count, &app);
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
    }

    impl Default for PropertyLending {
        fn default() -> Self {
            Self::new(AccountId::from([0x0; 32]))
        }
    }
}

pub use crate::propchain_lending::{LendingError, LoanStatus, PropertyLending};

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
    fn test_loan_restructuring_requires_borrower_and_lender_approval() {
        let mut contract = setup();
        let accounts = test::default_accounts::<DefaultEnvironment>();

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
        let loan_id = contract
            .apply_for_loan(1, 700_000, 1_000_000, 0)
            .unwrap();

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
