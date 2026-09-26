use ink::env::test;
use ink::prelude::string::String;
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

/// A minimal well-formed escrow deployment request.
///
/// `init_params` is non-empty because `deploy_contract` now rejects an empty
/// constructor argument list (Issue #1175). The payload is opaque here — the
/// factory holds only a `Hash` for each type, not its constructor ABI — but it
/// stands in for what a `DeploymentTemplate::try_encode_params` call would
/// produce on a real deployment.
fn escrow_config(salt_byte: u8) -> DeploymentConfig {
    DeploymentConfig {
        contract_type: ContractType::Escrow,
        salt: [salt_byte; 32],
        init_params: vec![0x01, 0x02, 0x03, 0x04],
    }
}

/// A deployment request with caller-chosen `init_params` and `version`.
fn config_with(contract_type: ContractType, init_params: Vec<u8>, version: &str) -> (DeploymentConfig, String) {
    (
        DeploymentConfig {
            contract_type,
            salt: [9u8; 32],
            init_params,
        },
        version.into(),
    )
}

/// A factory with a code hash registered for `contract_type`.
fn factory_with_code_hash(contract_type: ContractType) -> ContractFactory {
    let mut factory = ContractFactory::new();
    let code_hash: Hash = [7u8; 32].into();
    factory.set_code_hash(contract_type, code_hash).unwrap();
    factory
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

// ── Deployment request validation (Issue #1175) ─────────────────────────────

#[ink::test]
fn test_deploy_rejects_empty_init_params() {
    let mut factory = factory_with_code_hash(ContractType::Escrow);
    let (config, version) = config_with(ContractType::Escrow, Vec::new(), "1.0.0");

    assert_eq!(
        factory.deploy_contract(config, version),
        Err(Error::InvalidParameters)
    );
}

#[ink::test]
fn test_deploy_rejects_oversized_init_params() {
    let mut factory = factory_with_code_hash(ContractType::Escrow);
    let (config, version) = config_with(
        ContractType::Escrow,
        vec![0u8; MAX_INIT_PARAMS_LEN as usize + 1],
        "1.0.0",
    );

    assert_eq!(
        factory.deploy_contract(config, version),
        Err(Error::InvalidParameters)
    );
}

/// The boundary is inclusive: exactly at the limit must still be accepted.
#[ink::test]
fn test_deploy_accepts_init_params_at_the_limit() {
    let mut factory = factory_with_code_hash(ContractType::Escrow);
    let (config, version) = config_with(
        ContractType::Escrow,
        vec![0u8; MAX_INIT_PARAMS_LEN as usize],
        "1.0.0",
    );

    assert!(factory.deploy_contract(config, version).is_ok());
    assert_eq!(factory.get_deployment_count(), 1);
}

#[ink::test]
fn test_deploy_rejects_empty_version() {
    let mut factory = factory_with_code_hash(ContractType::Escrow);
    let (config, version) = config_with(ContractType::Escrow, vec![1, 2, 3], "");

    assert_eq!(
        factory.deploy_contract(config, version),
        Err(Error::InvalidVersion)
    );
}

#[ink::test]
fn test_deploy_rejects_oversized_version() {
    let mut factory = factory_with_code_hash(ContractType::Escrow);
    let long_version = "v".repeat(MAX_VERSION_LEN as usize + 1);
    let (config, version) = config_with(ContractType::Escrow, vec![1, 2, 3], &long_version);

    assert_eq!(
        factory.deploy_contract(config, version),
        Err(Error::InvalidVersion)
    );
}

#[ink::test]
fn test_deploy_accepts_version_at_the_limit() {
    let mut factory = factory_with_code_hash(ContractType::Escrow);
    let at_limit = "v".repeat(MAX_VERSION_LEN as usize);
    let (config, version) = config_with(ContractType::Escrow, vec![1, 2, 3], &at_limit);

    assert!(factory.deploy_contract(config, version).is_ok());
    assert_eq!(factory.get_deployment(0).unwrap().version, at_limit);
}

/// A one-byte overage is rejected, not just a wildly oversized value.
#[ink::test]
fn test_validation_boundaries_are_exact() {
    let factory = ContractFactory::new();

    let (too_long_params, ok_version) = config_with(
        ContractType::Escrow,
        vec![0u8; MAX_INIT_PARAMS_LEN as usize + 1],
        "1.0.0",
    );
    assert_eq!(
        ContractFactory::validate_deployment_request(&too_long_params, &ok_version),
        Err(Error::InvalidParameters)
    );

    let (at_limit_params, ok_version) = config_with(
        ContractType::Escrow,
        vec![0u8; MAX_INIT_PARAMS_LEN as usize],
        "1.0.0",
    );
    assert!(ContractFactory::validate_deployment_request(&at_limit_params, &ok_version).is_ok());

    let (params, too_long_version) = config_with(
        ContractType::Escrow,
        vec![1, 2, 3],
        &"v".repeat(MAX_VERSION_LEN as usize + 1),
    );
    assert_eq!(
        ContractFactory::validate_deployment_request(&params, &too_long_version),
        Err(Error::InvalidVersion)
    );

    let (params, at_limit_version) = config_with(
        ContractType::Escrow,
        vec![1, 2, 3],
        &"v".repeat(MAX_VERSION_LEN as usize),
    );
    assert!(ContractFactory::validate_deployment_request(&params, &at_limit_version).is_ok());
}

/// `init_params` is reported before `version`, so a caller fixing one problem
/// at a time is told about the first one deterministically.
#[ink::test]
fn test_init_params_are_validated_before_version() {
    let factory = ContractFactory::new();
    let (config, version) = config_with(ContractType::Escrow, Vec::new(), "");
    assert_eq!(
        ContractFactory::validate_deployment_request(&config, &version),
        Err(Error::InvalidParameters)
    );
}

/// A malformed request is rejected on its own terms, not reported as a missing
/// code hash — otherwise the caller would go looking for a code-hash problem
/// that does not exist.
#[ink::test]
fn test_validation_precedes_the_code_hash_check() {
    // No code hash registered for Escrow.
    let mut factory = ContractFactory::new();
    let (config, version) = config_with(ContractType::Escrow, Vec::new(), "1.0.0");

    assert_eq!(
        factory.deploy_contract(config, version),
        Err(Error::InvalidParameters)
    );
}

/// A well-formed request with no code hash still reports the code hash, so the
/// new validation did not swallow the existing guard.
#[ink::test]
fn test_well_formed_request_still_reports_the_missing_code_hash() {
    let mut factory = ContractFactory::new();
    let (config, version) = config_with(ContractType::Escrow, vec![1, 2, 3], "1.0.0");

    assert_eq!(
        factory.deploy_contract(config, version),
        Err(Error::CodeHashNotSet)
    );
}

#[ink::test]
fn test_failed_validation_records_nothing() {
    let mut factory = factory_with_code_hash(ContractType::Escrow);
    let accounts = test::default_accounts::<ink::env::DefaultEnvironment>();

    for (params, version) in [
        (Vec::new(), "1.0.0".to_string()),
        (vec![0u8; MAX_INIT_PARAMS_LEN as usize + 1], "1.0.0".to_string()),
        (vec![1, 2, 3], String::new()),
        (vec![1, 2, 3], "v".repeat(MAX_VERSION_LEN as usize + 1)),
    ] {
        let (config, version) = config_with(ContractType::Escrow, params, &version);
        assert!(factory.deploy_contract(config, version).is_err());
    }

    assert_eq!(factory.get_deployment_count(), 0);
    assert!(factory.get_deployer_contracts(accounts.alice).is_empty());
    assert!(factory.get_deployment(0).is_none());
}

#[ink::test]
fn test_deployment_limits_are_sane() {
    let factory = ContractFactory::new();
    let (max_params, max_version) = factory.deployment_limits();

    assert!(max_params > 0);
    assert!(max_version > 0);
    // A constructor argument list must fit inside the version bound's scale by
    // a wide margin; if these ever invert, the limits are misconfigured.
    assert!(max_params > max_version);
}

#[ink::test]
fn test_version_is_recorded_verbatim() {
    let mut factory = factory_with_code_hash(ContractType::Escrow);
    let (config, version) = config_with(ContractType::Escrow, vec![1, 2, 3], "v2.0.0-rc1+build");
    factory.deploy_contract(config, version).unwrap();

    assert_eq!(factory.get_deployment(0).unwrap().version, "v2.0.0-rc1+build");
}
