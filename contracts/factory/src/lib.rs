#![allow(clippy::clone_on_copy)] // fires inside ink! generated storage code
#![cfg_attr(not(feature = "std"), no_std)]

use ink::prelude::string::String;
use ink::prelude::vec::Vec;

pub mod builder;
pub mod templates;

#[cfg(test)]
mod tests;

#[ink::contract]
pub mod contract_factory {
    use super::*;

    /// Largest `init_params` payload accepted by `deploy_contract`, in bytes.
    ///
    /// `init_params` is the SCALE-encoded constructor argument list that gets
    /// forwarded to the target contract. Nothing about it is bounded on the way
    /// in, so an oversized payload is copied into the transaction, re-encoded
    /// for the cross-contract call, and then written into the deployment
    /// record's footprint — all before anything rejects it. 4 KiB is far above
    /// any realistic ink! constructor signature (the largest templates in
    /// `templates.rs` encode to tens of bytes) while still bounding the work a
    /// single call can force.
    pub const MAX_INIT_PARAMS_LEN: u32 = 4_096;

    /// Largest `version` label accepted by `deploy_contract`, in bytes.
    ///
    /// The version is stored verbatim in `DeployedContract` and returned by
    /// `get_deployment`, so an unbounded label is unbounded permanent storage
    /// paid for by the factory. 64 bytes comfortably fits a semver string, a
    /// git SHA, or a `v1.2.3-rc1` style tag.
    pub const MAX_VERSION_LEN: u32 = 64;

    /// Contract types that can be deployed
    #[derive(Debug, Clone, Copy, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(
        feature = "std",
        derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
    )]
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
    #[cfg_attr(
        feature = "std",
        derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
    )]
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
        /// `init_params` was empty or longer than `MAX_INIT_PARAMS_LEN`.
        ///
        /// Previously never constructed: `builder::build_contract` accepted
        /// any payload and returned `Ok`, so a caller had no way to learn that
        /// its parameters were unusable.
        InvalidParameters,
        /// Native tokens were attached to `deploy_contract` but deployment
        /// fees are not supported. Send zero value.
        UnexpectedValue,
        /// `version` was empty or longer than `MAX_VERSION_LEN`.
        ///
        /// Distinct from `InvalidParameters` so a caller can tell which half of
        /// the request was rejected. Appended last so no existing discriminant
        /// moves.
        InvalidVersion,
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
        #[allow(clippy::new_without_default)]
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

            let old_hash = self.code_hashes.get(contract_type);
            self.code_hashes.insert(contract_type, &code_hash);

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
            self.code_hashes.get(contract_type)
        }

        /// Deploys a new contract instance.
        ///
        /// `config.init_params` must be the SCALE-encoded constructor argument
        /// list for `config.contract_type`: non-empty and no longer than
        /// `MAX_INIT_PARAMS_LEN`. `version` must be non-empty and no longer
        /// than `MAX_VERSION_LEN`. See `validate_deployment_request` for why
        /// the empty case is rejected.
        ///
        /// Attaching native tokens is not supported; deployment is free.
        /// Send zero value with this call.
        #[ink(message, payable)]
        pub fn deploy_contract(
            &mut self,
            config: DeploymentConfig,
            version: String,
        ) -> Result<AccountId, Error> {
            if self.env().transferred_value() > 0 {
                return Err(Error::UnexpectedValue);
            }

            // Validate the request before touching storage or the builder, so
            // a malformed call is rejected on its own terms rather than being
            // reported as a missing code hash or a deployment failure.
            Self::validate_deployment_request(&config, &version)?;

            let code_hash = self
                .code_hashes
                .get(config.contract_type)
                .ok_or(Error::CodeHashNotSet)?;

            let deployer = self.env().caller();

            let contract = builder::build_contract(
                config.contract_type,
                config.init_params,
                Some(config.salt),
            )
            .map_err(|_| Error::DeploymentFailed)?;

            let contract_address = contract.address;

            // Record deployment
            let deployment_id = self.deployment_count;
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
                .insert(deployment_id, &deployed_contract);
            self.deployment_count += 1;

            // Update deployer's contract list
            let mut deployer_list = self.deployer_contracts.get(deployer).unwrap_or_default();
            deployer_list.push(deployment_id);
            self.deployer_contracts.insert(deployer, &deployer_list);

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
            self.deployed_contracts.get(deployment_id)
        }

        /// Gets all deployments by a deployer
        #[ink(message)]
        pub fn get_deployer_contracts(&self, deployer: AccountId) -> Vec<u64> {
            self.deployer_contracts.get(deployer).unwrap_or_default()
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

        /// Reject a deployment request whose `init_params` or `version` cannot
        /// describe a real deployment.
        ///
        /// Empty `init_params` is rejected. `init_params` is the SCALE-encoded
        /// constructor argument list for the target contract, and
        /// `builder::build_contract` ignores it entirely and returns `Ok`, so
        /// an empty payload is currently accepted and recorded as a successful
        /// deployment. If the target's constructor expects arguments, that
        /// deployment is un-callable; the factory is the only place that can
        /// notice, because by the time a real instantiation runs, the params
        /// are already baked into the record. No `DeploymentTemplate` in
        /// `templates.rs` encodes to an empty payload, so this matches the
        /// intended calling convention rather than inventing one.
        ///
        /// The factory only holds a `Hash` for each contract type, not its
        /// constructor metadata, so it genuinely cannot type-check the payload
        /// against the expected ABI. These are the checks that are correct
        /// regardless of ABI, and the length bounds are what keep a single
        /// call from forcing unbounded copying and permanent storage growth.
        ///
        /// The limits themselves are readable on-chain via
        /// [`Self::deployment_limits`] so a client can check a request before
        /// submitting it.
        fn validate_deployment_request(
            config: &DeploymentConfig,
            version: &str,
        ) -> Result<(), Error> {
            if config.init_params.is_empty() {
                return Err(Error::InvalidParameters);
            }
            if config.init_params.len() > MAX_INIT_PARAMS_LEN as usize {
                return Err(Error::InvalidParameters);
            }
            if version.is_empty() {
                return Err(Error::InvalidVersion);
            }
            if version.len() > MAX_VERSION_LEN as usize {
                return Err(Error::InvalidVersion);
            }
            Ok(())
        }

        /// The limits `validate_deployment_request` enforces, so a
        /// client can check a request before submitting it.
        #[ink(message)]
        pub fn deployment_limits(&self) -> (u32, u32) {
            (MAX_INIT_PARAMS_LEN, MAX_VERSION_LEN)
        }
    }
}
