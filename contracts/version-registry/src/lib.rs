#![cfg_attr(not(feature = "std"), no_std)]

use ink::storage::Mapping;
use propchain_traits::{ContractError, ErrorCategory, ReentrancyError};
#[cfg(not(feature = "std"))]
use scale_info::prelude::{string::String, vec::Vec};

#[ink::contract]
mod version_registry {
    use super::*;

    #[derive(Debug, Clone, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(
        feature = "std",
        derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
    )]
    pub struct DeploymentRecord {
        pub contract_name: String,
        pub version: u32,
        pub code_hash: [u8; 32],
        pub deployed_at: u64,
        pub deployer: AccountId,
    }

    #[allow(dead_code)]
    #[derive(Debug, Clone, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub struct DeploymentHistory {
        pub name: String,
        pub deployments: Vec<DeploymentRecord>,
    }

    #[derive(Debug, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub enum Error {
        Unauthorized,
        NameNotFound,
        VersionAlreadyExists,
        InvalidVersion,
    }

    impl core::fmt::Display for Error {
        fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
            match self {
                Error::Unauthorized => write!(f, "Caller is not authorized"),
                Error::NameNotFound => write!(f, "Name not found in registry"),
                Error::VersionAlreadyExists => {
                    write!(f, "Version already exists for this name")
                }
                Error::InvalidVersion => write!(f, "Invalid version number"),
            }
        }
    }

    impl ContractError for Error {
        fn error_code(&self) -> u32 {
            match self {
                Error::Unauthorized => 13001,
                Error::NameNotFound => 13002,
                Error::VersionAlreadyExists => 13003,
                Error::InvalidVersion => 13004,
            }
        }

        fn error_description(&self) -> &'static str {
            match self {
                Error::Unauthorized => {
                    "Caller does not have permission to perform this operation"
                }
                Error::NameNotFound => {
                    "The specified name is not registered in the version registry"
                }
                Error::VersionAlreadyExists => {
                    "A deployment with this version already exists for the specified name"
                }
                Error::InvalidVersion => "The provided version number is invalid",
            }
        }

        fn error_category(&self) -> ErrorCategory {
            ErrorCategory::Common
        }
    }

    impl From<ReentrancyError> for Error {
        fn from(_: ReentrancyError) -> Self {
            Error::Unauthorized
        }
    }

    #[ink(event)]
    pub struct ContractDeployed {
        #[ink(topic)]
        pub contract_name: String,
        pub version: u32,
        pub code_hash: [u8; 32],
        pub deployed_at: u64,
        #[ink(topic)]
        pub deployer: AccountId,
    }

    #[ink(storage)]
    pub struct VersionRegistry {
        admin: AccountId,
        deployments: Mapping<(String, u32), DeploymentRecord>,
        latest_versions: Mapping<String, u32>,
        name_count: u32,
        next_version: Mapping<String, u32>,
        name_index: Mapping<u32, String>,
    }

    impl VersionRegistry {
        #[ink(constructor)]
        pub fn new() -> Self {
            Self {
                admin: Self::env().caller(),
                deployments: Mapping::default(),
                latest_versions: Mapping::default(),
                name_count: 0,
                next_version: Mapping::default(),
                name_index: Mapping::default(),
            }
        }

        fn ensure_admin(&self) -> Result<(), Error> {
            if self.env().caller() != self.admin {
                return Err(Error::Unauthorized);
            }
            Ok(())
        }

        #[ink(message)]
        pub fn register_deployment(
            &mut self,
            name: String,
            code_hash: [u8; 32],
        ) -> Result<u32, Error> {
            self.ensure_admin()?;

            let version = self.next_version.get(&name).unwrap_or(1);

            if self
                .deployments
                .get(&(name.clone(), version))
                .is_some()
            {
                return Err(Error::VersionAlreadyExists);
            }

            let caller = self.env().caller();
            let now = self.env().block_timestamp();
            let is_new_name = self.latest_versions.get(&name).is_none();

            let record = DeploymentRecord {
                contract_name: name.clone(),
                version,
                code_hash,
                deployed_at: now,
                deployer: caller,
            };

            self.deployments
                .insert(&(name.clone(), version), &record);
            self.latest_versions.insert(&name, &version);
            self.next_version.insert(&name, &(version + 1));

            if is_new_name {
                self.name_count += 1;
                self.name_index.insert(&self.name_count, &name.clone());
            }

            self.env().emit_event(ContractDeployed {
                contract_name: name,
                version,
                code_hash,
                deployed_at: now,
                deployer: caller,
            });

            Ok(version)
        }

        #[ink(message)]
        pub fn register_deployment_with_version(
            &mut self,
            name: String,
            version: u32,
            code_hash: [u8; 32],
        ) -> Result<(), Error> {
            self.ensure_admin()?;

            if version < 1 {
                return Err(Error::InvalidVersion);
            }

            if self
                .deployments
                .get(&(name.clone(), version))
                .is_some()
            {
                return Err(Error::VersionAlreadyExists);
            }

            let caller = self.env().caller();
            let now = self.env().block_timestamp();
            let is_new_name = self.latest_versions.get(&name).is_none();

            let record = DeploymentRecord {
                contract_name: name.clone(),
                version,
                code_hash,
                deployed_at: now,
                deployer: caller,
            };

            self.deployments
                .insert(&(name.clone(), version), &record);

            let current_latest = self.latest_versions.get(&name).unwrap_or(0);
            if version > current_latest {
                self.latest_versions.insert(&name, &version);
            }

            let next_ver = self.next_version.get(&name).unwrap_or(1);
            if version >= next_ver {
                self.next_version.insert(&name, &(version + 1));
            }

            if is_new_name {
                self.name_count += 1;
                self.name_index.insert(&self.name_count, &name.clone());
            }

            self.env().emit_event(ContractDeployed {
                contract_name: name,
                version,
                code_hash,
                deployed_at: now,
                deployer: caller,
            });

            Ok(())
        }

        #[ink(message)]
        pub fn get_latest_version(&self, name: String) -> Option<u32> {
            self.latest_versions.get(&name)
        }

        #[ink(message)]
        pub fn get_deployment(
            &self,
            name: String,
            version: u32,
        ) -> Option<DeploymentRecord> {
            self.deployments.get(&(name, version))
        }

        #[ink(message)]
        pub fn get_deployment_history(&self, name: String) -> Vec<DeploymentRecord> {
            let latest = match self.latest_versions.get(&name) {
                Some(v) => v,
                None => return Vec::new(),
            };

            let mut history = Vec::new();
            for v in 1..=latest {
                if let Some(record) = self.deployments.get(&(name.clone(), v)) {
                    history.push(record);
                }
            }
            history
        }

        #[ink(message)]
        pub fn get_all_names(&self) -> Vec<String> {
            let mut names = Vec::with_capacity(self.name_count as usize);
            for i in 1..=self.name_count {
                if let Some(name) = self.name_index.get(&i) {
                    names.push(name);
                }
            }
            names
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use ink::env::test;

        fn default_registry() -> VersionRegistry {
            VersionRegistry::new()
        }

        #[ink::test]
        fn test_register_and_query() {
            let mut registry = default_registry();
            let hash = [1u8; 32];
            let version = registry
                .register_deployment("test_contract".into(), hash)
                .unwrap();
            assert_eq!(version, 1);

            let record = registry
                .get_deployment("test_contract".into(), 1)
                .unwrap();
            assert_eq!(record.version, 1);
            assert_eq!(record.code_hash, hash);
            assert_eq!(record.contract_name, "test_contract");
        }

        #[ink::test]
        fn test_latest_version_returns_highest() {
            let mut registry = default_registry();

            registry
                .register_deployment("test".into(), [1u8; 32])
                .unwrap();
            assert_eq!(registry.get_latest_version("test".into()), Some(1));

            registry
                .register_deployment("test".into(), [2u8; 32])
                .unwrap();
            assert_eq!(registry.get_latest_version("test".into()), Some(2));
        }

        #[ink::test]
        fn test_deployment_history_returns_all() {
            let mut registry = default_registry();
            let hashes = [[1u8; 32], [2u8; 32], [3u8; 32]];

            for hash in &hashes {
                registry
                    .register_deployment("test".into(), *hash)
                    .unwrap();
            }

            let history = registry.get_deployment_history("test".into());
            assert_eq!(history.len(), 3);
            assert_eq!(history[0].version, 1);
            assert_eq!(history[1].version, 2);
            assert_eq!(history[2].version, 3);
        }

        #[ink::test]
        fn test_unauthorized_caller_fails() {
            let mut registry = default_registry();
            let accounts = test::default_accounts::<ink::env::DefaultEnvironment>();
            test::set_caller::<ink::env::DefaultEnvironment>(accounts.bob);

            let result =
                registry.register_deployment("test".into(), [1u8; 32]);
            assert_eq!(result, Err(Error::Unauthorized));
        }

        #[ink::test]
        fn test_duplicate_version_fails() {
            let mut registry = default_registry();

            registry
                .register_deployment_with_version("test".into(), 1, [1u8; 32])
                .unwrap();
            let result = registry
                .register_deployment_with_version("test".into(), 1, [2u8; 32]);
            assert_eq!(result, Err(Error::VersionAlreadyExists));
        }

        #[ink::test]
        fn test_auto_increment_skips_used_versions() {
            let mut registry = default_registry();

            registry
                .register_deployment_with_version("test".into(), 5, [1u8; 32])
                .unwrap();
            let version = registry
                .register_deployment("test".into(), [2u8; 32])
                .unwrap();
            assert_eq!(version, 6);
        }

        #[ink::test]
        fn test_get_all_names() {
            let mut registry = default_registry();

            registry
                .register_deployment("alpha".into(), [1u8; 32])
                .unwrap();
            registry
                .register_deployment("beta".into(), [2u8; 32])
                .unwrap();

            let names = registry.get_all_names();
            assert_eq!(names.len(), 2);
            assert!(names.contains(&"alpha".into()));
            assert!(names.contains(&"beta".into()));
        }

        #[ink::test]
        fn test_get_nonexistent_deployment() {
            let registry = default_registry();
            assert!(registry
                .get_deployment("nonexistent".into(), 1)
                .is_none());
            assert!(registry
                .get_latest_version("nonexistent".into())
                .is_none());
        }
    }
}
