use ink::env::test;
use ink::prelude::vec::Vec;
use ink::primitives::{AccountId, Hash};

use crate::contract_factory::*;

#[ink::test]
fn test_factory_initialization() {
    let factory = ContractFactory::new();
    let accounts = test::default_accounts::<ink::env::DefaultEnvironment>();

    assert_eq!(factory.admin(), accounts.alice);
    assert_eq!(factory.get_deployment_count(), 0);
}

#[ink::test]
fn test_set_code_hash() {
    let mut factory = ContractFactory::new();
    let code_hash: Hash = [1u8; 32].into();

    let result = factory.set_code_hash(ContractType::PropertyToken, code_hash);
    assert!(result.is_ok());

    let retrieved = factory.get_code_hash(ContractType::PropertyToken);
    assert_eq!(retrieved, Some(code_hash));
}

#[ink::test]
fn test_unauthorized_set_code_hash() {
    let mut factory = ContractFactory::new();
    let accounts = test::default_accounts::<ink::env::DefaultEnvironment>();

    // Change caller to non-admin
    test::set_caller::<ink::env::DefaultEnvironment>(accounts.bob);

    let code_hash: Hash = [1u8; 32].into();
    let result = factory.set_code_hash(ContractType::PropertyToken, code_hash);

    assert_eq!(result, Err(Error::Unauthorized));
}

#[ink::test]
fn test_change_admin() {
    let mut factory = ContractFactory::new();
    let accounts = test::default_accounts::<ink::env::DefaultEnvironment>();

    let result = factory.change_admin(accounts.bob);
    assert!(result.is_ok());
    assert_eq!(factory.admin(), accounts.bob);
}

#[ink::test]
fn test_get_deployer_contracts_empty() {
    let factory = ContractFactory::new();
    let accounts = test::default_accounts::<ink::env::DefaultEnvironment>();

    let contracts = factory.get_deployer_contracts(accounts.alice);
    assert_eq!(contracts.len(), 0);
}

// ── Deployment & admin-transfer tests (Issue #1019) ─────────────────────────

fn escrow_config(salt_byte: u8) -> DeploymentConfig {
    DeploymentConfig {
        contract_type: ContractType::Escrow,
        salt: [salt_byte; 32],
        init_params: Vec::new(),
    }
}

#[ink::test]
fn test_deploy_without_code_hash_fails() {
    let mut factory = ContractFactory::new();

    // No set_code_hash was performed for ContractType::Escrow, so the guard
    // path must reject the deployment before any instantiation is attempted.
    let result = factory.deploy_contract(escrow_config(7), "1.0.0".into());
    assert_eq!(result, Err(Error::CodeHashNotSet));

    // Nothing may have been recorded by the failed deployment attempt.
    assert_eq!(factory.get_deployment_count(), 0);
    let accounts = test::default_accounts::<ink::env::DefaultEnvironment>();
    assert!(factory.get_deployer_contracts(accounts.alice).is_empty());
}

#[ink::test]
fn test_change_admin_revokes_old_admin() {
    let mut factory = ContractFactory::new();
    let accounts = test::default_accounts::<ink::env::DefaultEnvironment>();

    // Admin (default caller Alice) transfers admin rights to Bob
    factory.change_admin(accounts.bob).unwrap();
    assert_eq!(factory.admin(), accounts.bob);

    // Old admin is now rejected on the admin-only set_code_hash operation
    test::set_caller::<ink::env::DefaultEnvironment>(accounts.alice);
    let old_hash: Hash = [2u8; 32].into();
    assert_eq!(
        factory.set_code_hash(ContractType::Dex, old_hash),
        Err(Error::Unauthorized)
    );

    // New admin can perform the same operation successfully
    test::set_caller::<ink::env::DefaultEnvironment>(accounts.bob);
    factory.set_code_hash(ContractType::Dex, old_hash).unwrap();
    assert_eq!(factory.get_code_hash(ContractType::Dex), Some(old_hash));
}

#[ink::test]
fn test_change_admin_by_non_admin_fails() {
    let mut factory = ContractFactory::new();
    let accounts = test::default_accounts::<ink::env::DefaultEnvironment>();

    // Bob (not admin) tries to hand admin rights to Charlie
    test::set_caller::<ink::env::DefaultEnvironment>(accounts.bob);
    assert_eq!(
        factory.change_admin(accounts.charlie),
        Err(Error::Unauthorized)
    );

    // Admin must be unchanged and still able to perform admin operations
    assert_eq!(factory.admin(), accounts.alice);
    let code_hash: Hash = [3u8; 32].into();
    test::set_caller::<ink::env::DefaultEnvironment>(accounts.alice);
    assert!(factory
        .set_code_hash(ContractType::Governance, code_hash)
        .is_ok());
}

#[ink::test]
fn test_deployment_getters_reflect_recorded_deployments() {
    // NOTE: real cross-contract instantiation needs a live on-chain code hash,
    // which a unit environment cannot provide. `builder::build_contract` is a
    // stub that always succeeds with a synthetic zero address, so deployment
    // *recording* can still be exercised end-to-end here; only the resulting
    // contract address is synthetic.
    let mut factory = ContractFactory::new();
    let accounts = test::default_accounts::<ink::env::DefaultEnvironment>();

    let escrow_hash: Hash = [4u8; 32].into();
    factory
        .set_code_hash(ContractType::Escrow, escrow_hash)
        .unwrap();

    // First deployment: recorded under id 0 for Alice
    let address_a = factory
        .deploy_contract(escrow_config(1), "1.0.0".into())
        .unwrap();
    assert_eq!(address_a, AccountId::from([0u8; 32])); // stub builder address

    assert_eq!(factory.get_deployment_count(), 1);
    assert_eq!(factory.get_deployer_contracts(accounts.alice), vec![0]);

    let record = factory.get_deployment(0).expect("deployment 0 recorded");
    assert_eq!(record.contract_type, ContractType::Escrow);
    assert_eq!(record.deployer, accounts.alice);
    assert_eq!(record.code_hash, escrow_hash);
    assert_eq!(record.version, "1.0.0");
    assert_eq!(record.address, address_a);

    // Second deployment from a different deployer lands under id 1
    test::set_caller::<ink::env::DefaultEnvironment>(accounts.bob);
    factory
        .deploy_contract(escrow_config(2), "1.0.1".into())
        .unwrap();

    assert_eq!(factory.get_deployment_count(), 2);
    assert_eq!(factory.get_deployer_contracts(accounts.bob), vec![1]);
    assert_eq!(factory.get_deployer_contracts(accounts.alice), vec![0]);
    assert_eq!(factory.get_deployment(1).unwrap().version, "1.0.1");

    // Unknown ids and deployers resolve to their empty states
    assert!(factory.get_deployment(999).is_none());
    assert!(factory.get_deployer_contracts(accounts.charlie).is_empty());
}
