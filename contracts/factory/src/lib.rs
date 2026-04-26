#![cfg_attr(not(feature = "std"), no_std)]

use ink::prelude::string::String;
use ink::prelude::vec::Vec;

pub mod templates;
pub mod builder;

#[cfg(test)]
mod tests;

#[ink::contract]
pub mod contract_factory {
    use super::*;

    /// Contract types that can be deployed
    #[derive(Debug, Clone, Copy, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub enum ContractType {
        PropertyToken,
        Escrow,
        Oracle,
        Bridge,
        Insurance,
        Governance,
        Dex,
        Lending,
        Crowdfunding,
        Fractional,
    }

    /// Deployment configuration
    #[derive(Debug, Clone, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub struct DeploymentConfig {
        pub contract_type: ContractType,
        pub salt: [u8; 32],
        pub init_params: Vec<u8>,
    }

    /// Deployed contract information
    #[derive(Debug, Clone, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout))]
    pub struct DeployedContract {
        pub contract_type: ContractType,
        pub address: AccountId,
        pub deployer: AccountId,
        pub deployed_at: u64,
        pub code_hash: Hash,
        pub version: String,
    }

    /// Factory errors
    #[derive(Debug, Clone, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub enum Error {
        Unauthorized,
        InvalidContractType,
        DeploymentFailed,
        CodeHashNotSet,
        ContractNotFound,
        InvalidParameters,
    }

    /// Contract Factory storage
    #[ink(storage)]
    pub struct ContractFactory {
        /// Factory admin
        admin: AccountId,
        /// Mapping from contract type to code hash
        code_hashes: ink::storage::Mapping<ContractType, Hash>,
        /// Deployed contracts registry
        deployed_contracts: ink::storage::Mapping<u64, DeployedContract>,
        /// Deployment counter
        deployment_count: u64,
        /// Mapping from deployer to their deployed contracts
        deployer_contracts: ink::storage::Mapping<AccountId, Vec<u64>>,
    }

    /// Events
    #[ink(event)]
    pub struct ContractDeployed {
        #[ink(topic)]
        deployment_id: u64,
        #[ink(topic)]
        contract_type: ContractType,
        #[ink(topic)]
        deployer: AccountId,
        contract_address: AccountId,
        timestamp: u64,
    }

    #[ink(event)]
    pub struct CodeHashUpdated {
        #[ink(topic)]
        contract_type: ContractType,
        #[ink(topic)]
        updated_by: AccountId,
        old_hash: Option<Hash>,
        new_hash: Hash,
        timestamp: u64,
    }

    impl ContractFactory {
        /// Creates a new factory instance
        #[ink(constructor)]
        pub fn new() -> Self {
            Self {
                admin: Self::env().caller(),
                code_hashes: ink::storage::Mapping::default(),
                deployed_contracts: ink::storage::Mapping::default(),
                deployment_count: 0,
                deployer_contracts: ink::storage::Mapping::default(),
            }
        }

        /// Sets the code hash for a contract type (admin only)
        #[ink(message)]
        pub fn set_code_hash(
            &mut self,
            contract_type: ContractType,
            code_hash: Hash,
        ) -> Result<(), Error> {
            self.ensure_admin()?;

            let old_hash = self.code_hashes.get(&contract_type);
            self.code_hashes.insert(&contract_type, &code_hash);

            self.env().emit_event(CodeHashUpdated {
                contract_type,
                updated_by: self.env().caller(),
                old_hash,
                new_hash: code_hash,
                timestamp: self.env().block_timestamp(),
            });

            Ok(())
        }

        /// Gets the code hash for a contract type
        #[ink(message)]
        pub fn get_code_hash(&self, contract_type: ContractType) -> Option<Hash> {
            self.code_hashes.get(&contract_type)
        }

        /// Deploys a new contract instance
        #[ink(message, payable)]
        pub fn deploy_contract(
            &mut self,
            config: DeploymentConfig,
            version: String,
        ) -> Result<AccountId, Error> {
            let code_hash = self
                .code_hashes
                .get(&config.contract_type)
                .ok_or(Error::CodeHashNotSet)?;

            // Build the create parameters
            let create_params = ink::env::call::build_create::<ink::env::DefaultEnvironment>()
                .code_hash(code_hash)
                .gas_limit(0)
                .endowment(self.env().transferred_value())
                .exec_input(
                    ink::env::call::ExecutionInput::new(ink::env::call::Selector::new(
                        ink::selector_bytes!("new"),
                    ))
                )
                .salt_bytes(&config.salt)
                .returns::<AccountId>()
                .params();

            // Deploy contract using instantiate_contract
            let contract_address = self
                .env()
                .instantiate_contract(&create_params)
                .map_err(|_| Error::DeploymentFailed)?;

            // Record deployment
            let deployment_id = self.deployment_count;
            let deployer = self.env().caller();
            let deployed_at = self.env().block_timestamp();

            let deployed_contract = DeployedContract {
                contract_type: config.contract_type,
                address: contract_address,
                deployer,
                deployed_at,
                code_hash,
                version,
            };

            self.deployed_contracts
                .insert(&deployment_id, &deployed_contract);
            self.deployment_count += 1;

            // Update deployer's contract list
            let mut deployer_list = self
                .deployer_contracts
                .get(&deployer)
                .unwrap_or_default();
            deployer_list.push(deployment_id);
            self.deployer_contracts.insert(&deployer, &deployer_list);

            self.env().emit_event(ContractDeployed {
                deployment_id,
                contract_type: config.contract_type,
                deployer,
                contract_address,
                timestamp: deployed_at,
            });

            Ok(contract_address)
        }

        /// Gets deployment information by ID
        #[ink(message)]
        pub fn get_deployment(&self, deployment_id: u64) -> Option<DeployedContract> {
            self.deployed_contracts.get(&deployment_id)
        }

        /// Gets all deployments by a deployer
        #[ink(message)]
        pub fn get_deployer_contracts(&self, deployer: AccountId) -> Vec<u64> {
            self.deployer_contracts.get(&deployer).unwrap_or_default()
        }

        /// Gets total deployment count
        #[ink(message)]
        pub fn get_deployment_count(&self) -> u64 {
            self.deployment_count
        }

        /// Gets the factory admin
        #[ink(message)]
        pub fn admin(&self) -> AccountId {
            self.admin
        }

        /// Changes the admin (admin only)
        #[ink(message)]
        pub fn change_admin(&mut self, new_admin: AccountId) -> Result<(), Error> {
            self.ensure_admin()?;
            self.admin = new_admin;
            Ok(())
        }

        // Helper functions
        fn ensure_admin(&self) -> Result<(), Error> {
            if self.env().caller() != self.admin {
                return Err(Error::Unauthorized);
            }
            Ok(())
        }
    }
}
