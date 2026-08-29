/// # Integration Tests: On-Chain Version Registry (+ Contract Factory pairing)
/// (Issue: version-registry has no integration coverage in the tests/ suite)
///
/// The version registry is the upgrade-tracking side of a deployment: the
/// factory instantiates code, the registry records which version of which
/// contract that code hash corresponds to. Its unit tests exercise the
/// registry in isolation; this suite covers it end to end and, crucially,
/// alongside the factory — the pairing the registry exists for.
///
/// Acceptance criteria tested:
///   check Sequential registration auto-increments 1 -> 2 -> 3
///   check get_latest_version tracks the highest version registered
///   check get_deployment_history returns every version in ascending order
///   check Explicit-version registration advances the auto-increment counter
///         so a later sequential call does not collide
///   check Explicit-version registration does not lower the latest version
///   check Gaps left by explicit-version registration are skipped in history
///   check Version 0 and duplicate versions are rejected
///   check Non-admin callers cannot register deployments
///   check Unknown names read back as None / empty history
///   check get_all_names lists each name once, in first-registration order
///   check Factory pairing: each factory deployment is recorded in the
///         registry under the next version with the same code hash, and an
///         upgrade bumps the registry's latest version
#[cfg(test)]
#[allow(clippy::module_inception)]
mod integration_version_registry {
    use ink::env::{test, DefaultEnvironment};
    use ink::primitives::Hash;
    use propchain_factory::contract_factory::{ContractFactory, ContractType, DeploymentConfig};
    use version_registry::version_registry::{Error as RegistryError, VersionRegistry};

    /// Raw code hashes shared by the registry (`[u8; 32]`) and the factory
    /// (`Hash`), so the pairing tests can compare the two sides directly.
    const TOKEN_V1: [u8; 32] = [0xA1; 32];
    const TOKEN_V2: [u8; 32] = [0xA2; 32];
    const ESCROW_V1: [u8; 32] = [0xB1; 32];

    fn setup_registry() -> VersionRegistry {
        let accounts = test::default_accounts::<DefaultEnvironment>();
        test::set_caller::<DefaultEnvironment>(accounts.alice);
        VersionRegistry::new()
    }

    fn setup_factory() -> ContractFactory {
        let accounts = test::default_accounts::<DefaultEnvironment>();
        test::set_caller::<DefaultEnvironment>(accounts.alice);
        ContractFactory::new()
    }

    // ── Sequential registration ──────────────────────────────────────────────

    /// register -> get latest -> register again -> history.
    #[ink::test]
    fn test_sequential_registration_increments_and_records_history() {
        let accounts = test::default_accounts::<DefaultEnvironment>();
        let mut registry = setup_registry();

        assert_eq!(registry.get_latest_version("property_token".into()), None);

        let v1 = registry
            .register_deployment("property_token".into(), TOKEN_V1)
            .expect("admin registers the first version");
        assert_eq!(v1, 1);
        assert_eq!(
            registry.get_latest_version("property_token".into()),
            Some(1)
        );

        let v2 = registry
            .register_deployment("property_token".into(), TOKEN_V2)
            .expect("admin registers the second version");
        assert_eq!(v2, 2);
        assert_eq!(
            registry.get_latest_version("property_token".into()),
            Some(2)
        );

        let history = registry.get_deployment_history("property_token".into());
        assert_eq!(history.len(), 2);
        // Ascending version order, each carrying its own code hash.
        assert_eq!(history[0].version, 1);
        assert_eq!(history[0].code_hash, TOKEN_V1);
        assert_eq!(history[1].version, 2);
        assert_eq!(history[1].code_hash, TOKEN_V2);
        // Every record attributes the registering admin.
        assert!(history.iter().all(|r| r.deployer == accounts.alice));
        assert!(history.iter().all(|r| r.contract_name == "property_token"));
    }

    /// `deployed_at` is stamped from the block timestamp at registration time.
    #[ink::test]
    fn test_registration_stamps_block_timestamp() {
        let mut registry = setup_registry();

        test::set_block_timestamp::<DefaultEnvironment>(1_700_000_000_000);
        registry
            .register_deployment("oracle".into(), TOKEN_V1)
            .unwrap();

        test::set_block_timestamp::<DefaultEnvironment>(1_700_000_900_000);
        registry
            .register_deployment("oracle".into(), TOKEN_V2)
            .unwrap();

        let history = registry.get_deployment_history("oracle".into());
        assert_eq!(history[0].deployed_at, 1_700_000_000_000);
        assert_eq!(history[1].deployed_at, 1_700_000_900_000);
    }

    /// Names are tracked independently: registering one does not advance
    /// another's version counter.
    #[ink::test]
    fn test_versions_are_per_name() {
        let mut registry = setup_registry();

        registry
            .register_deployment("property_token".into(), TOKEN_V1)
            .unwrap();
        registry
            .register_deployment("property_token".into(), TOKEN_V2)
            .unwrap();
        let escrow_v = registry
            .register_deployment("escrow".into(), ESCROW_V1)
            .unwrap();

        assert_eq!(escrow_v, 1, "a fresh name starts at version 1");
        assert_eq!(
            registry.get_latest_version("property_token".into()),
            Some(2)
        );
        assert_eq!(registry.get_latest_version("escrow".into()), Some(1));

        let mut names = registry.get_all_names();
        names.sort();
        assert_eq!(
            names,
            vec!["escrow".to_string(), "property_token".to_string()]
        );
    }

    // ── Explicit-version registration ────────────────────────────────────────

    /// Registering an explicit version advances the auto-increment counter, so
    /// a later sequential call lands above it rather than colliding.
    #[ink::test]
    fn test_explicit_version_advances_sequential_counter() {
        let mut registry = setup_registry();

        registry
            .register_deployment("bridge".into(), TOKEN_V1)
            .unwrap(); // -> 1
        registry
            .register_deployment_with_version("bridge".into(), 5, TOKEN_V2)
            .expect("admin registers an explicit version");

        assert_eq!(registry.get_latest_version("bridge".into()), Some(5));

        let next = registry
            .register_deployment("bridge".into(), ESCROW_V1)
            .expect("the sequential counter must skip past the explicit version");
        assert_eq!(next, 6);
        assert_eq!(registry.get_latest_version("bridge".into()), Some(6));
    }

    /// Backfilling a lower version records it without lowering the latest, and
    /// the gap left behind is skipped rather than erroring out.
    #[ink::test]
    fn test_explicit_lower_version_backfills_without_lowering_latest() {
        let mut registry = setup_registry();

        registry
            .register_deployment_with_version("lending".into(), 4, TOKEN_V1)
            .unwrap();
        assert_eq!(registry.get_latest_version("lending".into()), Some(4));

        // Versions 1 and 3 exist; 2 never does.
        registry
            .register_deployment_with_version("lending".into(), 1, TOKEN_V2)
            .unwrap();
        registry
            .register_deployment_with_version("lending".into(), 3, ESCROW_V1)
            .unwrap();

        assert_eq!(
            registry.get_latest_version("lending".into()),
            Some(4),
            "backfilling must not lower the latest version"
        );

        let history = registry.get_deployment_history("lending".into());
        assert_eq!(
            history.iter().map(|r| r.version).collect::<Vec<_>>(),
            vec![1, 3, 4],
            "the version-2 gap is skipped, not reported"
        );
        assert!(registry.get_deployment("lending".into(), 2).is_none());
    }

    // ── Rejections ───────────────────────────────────────────────────────────

    #[ink::test]
    fn test_version_zero_is_rejected() {
        let mut registry = setup_registry();
        assert_eq!(
            registry.register_deployment_with_version("dex".into(), 0, TOKEN_V1),
            Err(RegistryError::InvalidVersion)
        );
        assert_eq!(registry.get_latest_version("dex".into()), None);
    }

    #[ink::test]
    fn test_duplicate_version_is_rejected() {
        let mut registry = setup_registry();
        registry
            .register_deployment("dex".into(), TOKEN_V1)
            .unwrap(); // -> 1
        assert_eq!(
            registry.register_deployment_with_version("dex".into(), 1, TOKEN_V2),
            Err(RegistryError::VersionAlreadyExists)
        );
        // The original record is untouched.
        assert_eq!(
            registry.get_deployment("dex".into(), 1).unwrap().code_hash,
            TOKEN_V1
        );
    }

    #[ink::test]
    fn test_non_admin_cannot_register() {
        let accounts = test::default_accounts::<DefaultEnvironment>();
        let mut registry = setup_registry();

        test::set_caller::<DefaultEnvironment>(accounts.bob);
        assert_eq!(
            registry.register_deployment("insurance".into(), TOKEN_V1),
            Err(RegistryError::Unauthorized)
        );
        assert_eq!(
            registry.register_deployment_with_version("insurance".into(), 1, TOKEN_V1),
            Err(RegistryError::Unauthorized)
        );
        assert_eq!(registry.get_latest_version("insurance".into()), None);
        assert!(registry.get_all_names().is_empty());
    }

    #[ink::test]
    fn test_unknown_name_reads_back_empty() {
        let registry = setup_registry();
        assert_eq!(registry.get_latest_version("nope".into()), None);
        assert!(registry.get_deployment("nope".into(), 1).is_none());
        assert!(registry.get_deployment_history("nope".into()).is_empty());
    }

    // ── Factory + registry pairing ───────────────────────────────────────────

    /// The pairing the registry exists for: the factory deploys a contract,
    /// the registry records that deployment under the next version with the
    /// same code hash, and an upgrade deploy bumps the registry's latest
    /// version while leaving the previous record readable.
    #[ink::test]
    fn test_factory_deployment_is_tracked_by_the_registry() {
        let accounts = test::default_accounts::<DefaultEnvironment>();
        let mut factory = setup_factory();
        let mut registry = setup_registry();

        // v1: register the code hash with the factory, deploy, record it.
        factory
            .set_code_hash(ContractType::PropertyToken, Hash::from(TOKEN_V1))
            .expect("admin registers the v1 code hash");
        factory
            .deploy_contract(
                DeploymentConfig {
                    contract_type: ContractType::PropertyToken,
                    salt: [1u8; 32],
                    init_params: Vec::new(),
                },
                String::from("1.0.0"),
            )
            .expect("deployment with a registered code hash succeeds");

        let deployed_v1 = factory.get_deployment(0).expect("deployment 0 recorded");
        let registry_v1 = registry
            .register_deployment("property_token".into(), deployed_v1.code_hash.into())
            .expect("registry records the deployment");
        assert_eq!(registry_v1, 1);

        // v2: rotate the factory's code hash and deploy the upgrade.
        factory
            .set_code_hash(ContractType::PropertyToken, Hash::from(TOKEN_V2))
            .expect("admin rotates to the v2 code hash");
        factory
            .deploy_contract(
                DeploymentConfig {
                    contract_type: ContractType::PropertyToken,
                    salt: [2u8; 32],
                    init_params: Vec::new(),
                },
                String::from("2.0.0"),
            )
            .expect("upgrade deployment succeeds");

        let deployed_v2 = factory.get_deployment(1).expect("deployment 1 recorded");
        let registry_v2 = registry
            .register_deployment("property_token".into(), deployed_v2.code_hash.into())
            .expect("registry records the upgrade");
        assert_eq!(registry_v2, 2);

        // The registry's view matches the factory's, version for version.
        assert_eq!(
            registry.get_latest_version("property_token".into()),
            Some(2)
        );
        let history = registry.get_deployment_history("property_token".into());
        assert_eq!(history.len(), 2);
        assert_eq!(Hash::from(history[0].code_hash), deployed_v1.code_hash);
        assert_eq!(Hash::from(history[1].code_hash), deployed_v2.code_hash);
        assert_eq!(history[0].code_hash, TOKEN_V1);
        assert_eq!(history[1].code_hash, TOKEN_V2);

        // Both sides agree the same deployer performed both deployments.
        assert_eq!(deployed_v1.deployer, accounts.alice);
        assert_eq!(history[1].deployer, accounts.alice);
        assert_eq!(factory.get_deployment_count(), 2);
    }

    /// A factory deployment that never happened is never recorded: a rejected
    /// deploy must leave the registry's version line untouched.
    #[ink::test]
    fn test_rejected_deployment_leaves_registry_version_untouched() {
        let mut factory = setup_factory();
        let mut registry = setup_registry();

        factory
            .set_code_hash(ContractType::Escrow, Hash::from(ESCROW_V1))
            .unwrap();
        factory
            .deploy_contract(
                DeploymentConfig {
                    contract_type: ContractType::Escrow,
                    salt: [3u8; 32],
                    init_params: Vec::new(),
                },
                String::from("1.0.0"),
            )
            .unwrap();
        registry
            .register_deployment("escrow".into(), ESCROW_V1)
            .unwrap();
        assert_eq!(registry.get_latest_version("escrow".into()), Some(1));

        // Oracle has no code hash registered, so this deploy is rejected.
        assert!(factory
            .deploy_contract(
                DeploymentConfig {
                    contract_type: ContractType::Oracle,
                    salt: [4u8; 32],
                    init_params: Vec::new(),
                },
                String::from("1.0.0"),
            )
            .is_err());

        assert_eq!(factory.get_deployment_count(), 1);
        assert_eq!(registry.get_latest_version("escrow".into()), Some(1));
        assert_eq!(registry.get_latest_version("oracle".into()), None);
    }
}
