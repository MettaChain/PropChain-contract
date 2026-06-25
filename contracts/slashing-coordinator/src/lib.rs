#![cfg_attr(not(feature = "std"), no_std)]

use ink::prelude::vec::Vec;
use propchain_traits::{Equivocation, ContractType};

#[ink::contract]
pub mod slashing_coordinator {
    use super::*;

    #[ink(storage)]
    pub struct SlashingCoordinator {
        staking_contract: AccountId,
        oracle_contract: AccountId,
    }

    impl SlashingCoordinator {
        #[ink(constructor)]
        pub fn new(staking_contract: AccountId, oracle_contract: AccountId) -> Self {
            Self {
                staking_contract,
                oracle_contract,
            }
        }

        #[ink(message)]
        pub fn on_equivocation(&mut self, equivocation: Equivocation) {
            match equivocation.contract_type {
                ContractType::Staking => {
                    // Call the staking contract's slashing function
                }
                ContractType::Oracle => {
                    // Call the oracle contract's slashing function
                }
            }
        }
    }
}