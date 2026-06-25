#![cfg_attr(not(feature = "std"), no_std, no_main)]

#[ink::contract]
mod slashing_coordinator {
    use ink::env::{
        call::{build_call, ExecutionInput},
        DefaultEnvironment,
    };
    use ink::prelude::string::String;
    use propchain_traits::{ContractType, Equivocation};

    #[ink(storage)]
    pub struct SlashingCoordinator {
        staking_contract: AccountId,
        oracle_contract: AccountId,
        reentrancy_guard: propchain_traits::ReentrancyGuard,
    }

    impl SlashingCoordinator {
        #[ink(constructor)]
        pub fn new(staking_contract: AccountId, oracle_contract: AccountId) -> Self {
            Self {
                staking_contract,
                oracle_contract,
                reentrancy_guard: propchain_traits::ReentrancyGuard::new(),
            }
        }

        #[ink(message)]
        pub fn on_equivocation(&mut self, equivocation: Equivocation) {
            propchain_traits::non_reentrant!(self, {
                match equivocation.contract_type {
                    ContractType::Staking => {
                        let _ = build_call::<DefaultEnvironment>()
                            .call(self.staking_contract)
                            .gas_limit(0)
                            .exec_input(
                                ExecutionInput::new(ink::env::call::Selector::new(
                                    "slash_validator".into(),
                                ))
                                .push_arg(equivocation.operator),
                            )
                            .returns::<()>()
                            .try_invoke();
                    }
                    ContractType::Oracle => {
                        let _ = build_call::<DefaultEnvironment>()
                            .call(self.oracle_contract)
                            .gas_limit(0)
                            .exec_input(
                                ExecutionInput::new(ink::env::call::Selector::new(
                                    "slash_source".into(),
                                ))
                                .push_arg(equivocation.operator.to_string())
                                .push_arg("High".to_string())
                                .push_arg("Equivocation".to_string()),
                            )
                            .returns::<()>()
                            .try_invoke();
                    }
                }
            });
        }
    }
}