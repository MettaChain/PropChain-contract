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
        PoolNotFound,
        InsufficientLiquidity,
        PositionNotFound,
        LiquidationThresholdNotMet,
        InvalidParameters,
        ProposalNotFound,
        InsufficientVotes,
        ReentrantCall,
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
        pub status: LoanStatus,
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

    #[ink(storage)]
    pub struct PropertyLending {
        admin: AccountId,
        collateral_records: Mapping<u64, CollateralRecord>,
        pools: Mapping<u64, LendingPool>,
        pool_count: u64,
        margin_positions: Mapping<u64, MarginPosition>,
        position_count: u64,
        loan_applications: Mapping<u64, LoanApplication>,
        loan_count: u64,
        yield_positions: Mapping<AccountId, YieldPosition>,
        total_staked: u128,
        reward_per_block: u128,
        proposals: Mapping<u64, Proposal>,
        proposal_count: u64,
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
                loan_count: 0,
                yield_positions: Mapping::default(),
                total_staked: 0,
                reward_per_block: 100,
                proposals: Mapping::default(),
                proposal_count: 0,
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
            self.loan_count += 1;
            let app = LoanApplication {
                loan_id: self.loan_count,
                applicant: self.env().caller(),
                property_id,
                requested_amount,
                collateral_value,
                credit_score,
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
            let approved = app.credit_score >= 600 && ltv <= 7500;
            app.status = if approved {
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
        let loan_id = contract.apply_for_loan(1, 900_000, 1_000_000, 700).unwrap();
        let approved = contract.underwrite_loan(loan_id).unwrap();
        assert!(!approved);
        let loan_id2 = contract.apply_for_loan(1, 700_000, 1_000_000, 700).unwrap();
        let approved2 = contract.underwrite_loan(loan_id2).unwrap();
        assert!(approved2);
    }

    #[ink::test]
    fn test_liquidate_loan() {
        let mut contract = setup();
        contract
            .assess_collateral(1, 1_000_000, 7500, 8000)
            .unwrap();
        let loan_id = contract.apply_for_loan(1, 700_000, 1_000_000, 700).unwrap();
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
}
