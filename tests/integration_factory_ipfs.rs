/// # Integration Tests: Contract Factory + IPFS Metadata Registry
/// (Issue #1006 factory, Issue #1007 ipfs-metadata)
///
/// Factory coverage verifies the code-hash registry and deployment guard
/// surface:
///   set_code_hash -> get_code_hash round-trip
///   deploy_contract without a registered code hash is rejected (CodeHashNotSet)
///   admin rotation locks out the old admin
///
/// Note on real deployments: the off-chain `#[ink::test]` environment cannot
/// upload/instantiate child contract code, so genuine instantiation is out of
/// scope here; the builder stub inside the factory crate records the registry
/// entry without a chain-level instantiate call. The security-relevant path
/// (CodeHashNotSet rejection) IS covered.
///
/// IPFS metadata coverage focuses on the documented access-escalation path of
/// `validate_and_register_metadata` (the registering caller receives
/// `AccessLevel::Admin` for that property), plus CID/metadata validation and
/// unauthorized-access rejections.
///
/// Acceptance criteria tested:
///   check Admin registers a code hash per contract type and reads it back
///   check Non-admin cannot set code hashes or rotate the admin
///   check deploy_contract rejects unknown contract types with CodeHashNotSet
///   check Successful registration records deployment id, deployer, code hash
///   change_admin: old admin rejected afterwards, new admin succeeds
///   check Registering valid metadata stores it and escalates caller to property Admin
///   check Invalid CIDs are rejected by validate_ipfs_cid
///   check Invalid metadata (missing fields / bounds) is rejected by validate_metadata
///   check Unauthorized accounts cannot register documents or verify content
#[cfg(test)]
#[allow(clippy::module_inception)]
mod integration_factory_ipfs {
    // Contract factory
    use ink::env::{test, DefaultEnvironment};
    use ink::primitives::Hash;
    // IPFS metadata registry
    use ipfs_metadata::ipfs_metadata::{
        AccessLevel, DocumentType, Error as IpfsError, IpfsMetadataRegistry, PropertyMetadata,
        ValidationRules,
    };
    use propchain_factory::contract_factory::{
        ContractFactory, ContractType, DeploymentConfig, Error as FactoryError,
    };

    fn hash(byte: u8) -> Hash {
        Hash::from([byte; 32])
    }

    // ── Issue #1006: Contract Factory ────────────────────────────────────────

    fn setup_factory() -> ContractFactory {
        let accounts = test::default_accounts::<DefaultEnvironment>();
        test::set_caller::<DefaultEnvironment>(accounts.alice);
        ContractFactory::new()
    }

    /// Admin can register a code hash per type and read it back.
    #[ink::test]
    fn test_set_and_get_code_hash_roundtrip() {
        let accounts = test::default_accounts::<DefaultEnvironment>();
        let mut factory = setup_factory();

        factory
            .set_code_hash(ContractType::PropertyToken, hash(0xA1))
            .expect("Admin should register a code hash");
        factory
            .set_code_hash(ContractType::Escrow, hash(0xB2))
            .expect("Admin should register a second code hash");

        assert_eq!(
            factory.get_code_hash(ContractType::PropertyToken),
            Some(hash(0xA1))
        );
        assert_eq!(
            factory.get_code_hash(ContractType::Escrow),
            Some(hash(0xB2))
        );
        assert_eq!(
            factory.get_code_hash(ContractType::Oracle),
            None,
            "Unregistered types must return None"
        );
        assert_eq!(factory.admin(), accounts.alice);
    }

    /// Non-admin cannot set code hashes.
    #[ink::test]
    fn test_non_admin_cannot_set_code_hash() {
        let accounts = test::default_accounts::<DefaultEnvironment>();
        let mut factory = setup_factory();

        test::set_caller::<DefaultEnvironment>(accounts.bob);
        assert_eq!(
            factory.set_code_hash(ContractType::Oracle, hash(0x01)),
            Err(FactoryError::Unauthorized),
            "Only the admin may register code hashes"
        );
        assert_eq!(factory.get_code_hash(ContractType::Oracle), None);
    }

    /// Deploying an unregistered contract type must be rejected (guard path).
    #[ink::test]
    fn test_deploy_without_code_hash_rejected() {
        let accounts = test::default_accounts::<DefaultEnvironment>();
        let mut factory = setup_factory();

        let config = DeploymentConfig {
            contract_type: ContractType::Fractional,
            salt: [0u8; 32],
            init_params: Vec::new(),
        };

        assert_eq!(
            factory.deploy_contract(config, String::from("1.0.0")),
            Err(FactoryError::CodeHashNotSet),
            "Deployment without a registered code hash must be rejected"
        );
        assert_eq!(factory.get_deployment_count(), 0);
        assert!(factory.get_deployer_contracts(accounts.alice).is_empty());
    }

    /// With a code hash registered, the deployment is recorded in the registry:
    /// counter, deployer list, and stored record all update consistently.
    #[ink::test]
    fn test_deployment_registry_recorded_after_registration() {
        let accounts = test::default_accounts::<DefaultEnvironment>();
        let mut factory = setup_factory();

        factory
            .set_code_hash(ContractType::Fractional, hash(0xCD))
            .expect("Code hash registration should succeed");

        let config = DeploymentConfig {
            contract_type: ContractType::Fractional,
            salt: [7u8; 32],
            init_params: Vec::new(),
        };
        factory
            .deploy_contract(config, String::from("0.1.0"))
            .expect("Registered type should deploy");

        assert_eq!(factory.get_deployment_count(), 1);

        let deployer_ids = factory.get_deployer_contracts(accounts.alice);
        assert_eq!(deployer_ids, vec![0], "First deployment id should be 0");

        let record = factory
            .get_deployment(0)
            .expect("Deployment should be stored");
        assert_eq!(record.contract_type, ContractType::Fractional);
        assert_eq!(record.deployer, accounts.alice);
        assert_eq!(record.code_hash, hash(0xCD));
        assert_eq!(record.version, String::from("0.1.0"));

        assert!(factory.get_deployment(1).is_none());
    }

    /// Two-step admin handover: old admin locked out, new admin operational.
    #[ink::test]
    fn test_change_admin_locks_out_old_admin() {
        let accounts = test::default_accounts::<DefaultEnvironment>();
        let mut factory = setup_factory();

        // Non-admin cannot rotate the admin
        test::set_caller::<DefaultEnvironment>(accounts.charlie);
        assert_eq!(
            factory.change_admin(accounts.charlie),
            Err(FactoryError::Unauthorized),
            "Non-admin must not rotate the admin"
        );

        // Legitimate rotation
        test::set_caller::<DefaultEnvironment>(accounts.alice);
        factory
            .change_admin(accounts.bob)
            .expect("Admin should rotate to bob");
        assert_eq!(factory.admin(), accounts.bob);

        // Old admin is now rejected...
        test::set_caller::<DefaultEnvironment>(accounts.alice);
        assert_eq!(
            factory.set_code_hash(ContractType::Bridge, hash(0x02)),
            Err(FactoryError::Unauthorized),
            "Old admin must lose privileges after rotation"
        );

        // ...while the new admin succeeds
        test::set_caller::<DefaultEnvironment>(accounts.bob);
        factory
            .set_code_hash(ContractType::Bridge, hash(0x03))
            .expect("New admin should hold full privileges");
        assert_eq!(
            factory.get_code_hash(ContractType::Bridge),
            Some(hash(0x03))
        );
    }

    // ── Issue #1007: IPFS Metadata Registry ─────────────────────────────────

    fn default_rules() -> ValidationRules {
        ValidationRules {
            max_location_length: 200,
            min_size: 10,
            max_size: 1_000_000,
            max_legal_description_length: 1_000,
            min_valuation: 100,
            max_file_size: 1_000_000,
            allowed_mime_types: Vec::new(),
            max_documents_per_property: 10,
            max_pinned_size_per_property: 5_000_000,
        }
    }

    fn valid_cid_v0() -> String {
        format!("Qm{}", "a".repeat(44)) // CIDv0: "Qm" prefix + exactly 46 chars total
    }

    fn valid_metadata(cid: Option<String>) -> PropertyMetadata {
        PropertyMetadata {
            location: String::from("12 Integration Lane, Lagos"),
            size: 500,
            legal_description: String::from("Factory-ipfs integration test property"),
            valuation: 750_000,
            documents_ipfs_cid: cid.clone(),
            images_ipfs_cid: None,
            legal_docs_ipfs_cid: cid,
            created_at: 1_000,
            content_hash: Hash::from([0x77u8; 32]),
            is_encrypted: false,
        }
    }

    /// Registering valid metadata stores it under the property id.
    #[ink::test]
    fn test_validate_and_register_stores_metadata() {
        let mut registry = IpfsMetadataRegistry::new_with_rules(default_rules());

        let meta = valid_metadata(Some(valid_cid_v0()));
        registry
            .validate_and_register_metadata(1, meta.clone())
            .expect("Valid metadata should be accepted");

        let stored = registry.get_metadata(1).expect("Metadata should be stored");
        assert_eq!(stored.location, meta.location);
        assert_eq!(stored.size, meta.size);
        assert_eq!(stored.valuation, meta.valuation);
        assert_eq!(
            stored.documents_ipfs_cid.as_deref(),
            Some(valid_cid_v0().as_str())
        );
    }

    /// Access-escalation path: any account that successfully registers a
    /// property is escalated to AccessLevel::Admin *for that property*, which
    /// is observable because they can then grant/revoke access on it even
    /// though they are not the contract admin.
    #[ink::test]
    fn test_registration_gating_and_admin_bootstrap() {
        let accounts = test::default_accounts::<DefaultEnvironment>();
        let mut registry = IpfsMetadataRegistry::new_with_rules(default_rules());

        // Since the #966 hardening, an unauthenticated caller cannot register
        // metadata for a fresh property and gains no access by trying.
        test::set_caller::<DefaultEnvironment>(accounts.bob);
        assert_eq!(
            registry.validate_and_register_metadata(42, valid_metadata(None)),
            Err(IpfsError::Unauthorized)
        );
        // ...and he cannot register documents against it either.
        assert_eq!(
            registry.register_ipfs_document(
                42,
                format!("b{}", "c".repeat(52)),
                DocumentType::Deed,
                Hash::from([0x33u8; 32]),
                1_024,
                String::from("application/pdf"),
                false,
            ),
            Err(IpfsError::Unauthorized)
        );

        // The contract admin bootstraps property 42 and receives persistent
        // property-level Admin.
        test::set_caller::<DefaultEnvironment>(accounts.alice);
        registry
            .validate_and_register_metadata(42, valid_metadata(None))
            .expect("admin bootstraps the property");

        // Behavioural proof of the Admin grant: only the admin may delegate.
        registry
            .grant_access(42, accounts.charlie, AccessLevel::Write)
            .expect("property admin delegates Write access");
        let doc_id = registry
            .register_ipfs_document(
                42,
                format!("b{}", "c".repeat(52)),
                DocumentType::Deed,
                Hash::from([0x33u8; 32]),
                1_024,
                String::from("application/pdf"),
                false,
            )
            .expect("Granted Write access must allow document registration");
        assert_eq!(doc_id, 1);

        // Neither the rejected registrant nor the delegated writer can
        // escalate: granting access stays admin-only.
        test::set_caller::<DefaultEnvironment>(accounts.charlie);
        assert_eq!(
            registry.grant_access(42, accounts.bob, AccessLevel::Read),
            Err(IpfsError::Unauthorized)
        );
    }

    /// CID validation matrix: empty, wrong-prefix, too-short v1 and malformed
    /// v0 are rejected; well-formed v0/v1 pass.
    #[ink::test]
    fn test_ipfs_cid_validation_matrix() {
        let registry = IpfsMetadataRegistry::new();

        assert_eq!(
            registry.validate_ipfs_cid(String::new()),
            Err(IpfsError::InvalidIpfsCid),
            "Empty CID must be rejected"
        );
        assert_eq!(
            registry.validate_ipfs_cid(String::from("not-a-cid")),
            Err(IpfsError::InvalidIpfsCid),
            "Wrong prefix must be rejected"
        );
        assert_eq!(
            registry.validate_ipfs_cid(String::from("b123")),
            Err(IpfsError::InvalidIpfsCid),
            "Too-short CIDv1 must be rejected"
        );
        assert_eq!(
            registry.validate_ipfs_cid(String::from("Qmshort")),
            Err(IpfsError::InvalidIpfsCid),
            "Malformed CIDv0 length must be rejected"
        );

        registry
            .validate_ipfs_cid(valid_cid_v0())
            .expect("Well-formed CIDv0 should pass");
        registry
            .validate_ipfs_cid(format!("b{}", "a".repeat(52)))
            .expect("Well-formed CIDv1 should pass");
    }

    /// Metadata validation matrix: missing required fields, out-of-bounds
    /// sizes/valuations and invalid embedded CIDs are all rejected.
    #[ink::test]
    fn test_metadata_validation_rejects_invalid_input() {
        let mut registry = IpfsMetadataRegistry::new_with_rules(default_rules());

        // Missing location -> RequiredFieldMissing
        let mut no_location = valid_metadata(None);
        no_location.location = String::new();
        assert_eq!(
            registry.validate_metadata(no_location.clone()),
            Err(IpfsError::RequiredFieldMissing)
        );

        // Missing legal description -> RequiredFieldMissing
        let mut no_legal = valid_metadata(None);
        no_legal.legal_description = String::new();
        assert_eq!(
            registry.validate_metadata(no_legal.clone()),
            Err(IpfsError::RequiredFieldMissing)
        );

        // Size below minimum -> DataTypeMismatch
        let mut tiny = valid_metadata(None);
        tiny.size = 5;
        assert_eq!(
            registry.validate_metadata(tiny.clone()),
            Err(IpfsError::DataTypeMismatch)
        );

        // Valuation below minimum -> DataTypeMismatch
        let mut cheap = valid_metadata(None);
        cheap.valuation = 0;
        assert_eq!(
            registry.validate_metadata(cheap.clone()),
            Err(IpfsError::DataTypeMismatch)
        );

        // Location exceeds max length -> SizeLimitExceeded
        let mut sprawling = valid_metadata(None);
        sprawling.location = "x".repeat(201);
        assert_eq!(
            registry.validate_metadata(sprawling.clone()),
            Err(IpfsError::SizeLimitExceeded)
        );

        // Embedded invalid CID propagates from validate_ipfs_cid
        let bad_cid_meta = valid_metadata(Some(String::from("definitely-not-a-cid")));
        assert_eq!(
            registry.validate_metadata(bad_cid_meta.clone()),
            Err(IpfsError::InvalidIpfsCid)
        );

        // None of the rejected variants may have been persisted
        assert_eq!(
            registry.validate_and_register_metadata(9, bad_cid_meta),
            Err(IpfsError::InvalidIpfsCid),
            "validate_and_register_metadata must reject invalid CIDs"
        );
        assert!(
            registry.get_metadata(9).is_none(),
            "Rejected metadata must not be stored"
        );
    }

    /// Unauthorized access attempts are rejected:
    /// strangers cannot attach documents to properties they hold no rights on,
    /// nor verify content hashes of documents they cannot read.
    #[ink::test]
    fn test_unauthorized_document_and_verification_attempts_rejected() {
        let accounts = test::default_accounts::<DefaultEnvironment>();
        test::set_caller::<DefaultEnvironment>(accounts.alice);
        let mut registry = IpfsMetadataRegistry::new_with_rules(default_rules());

        // Admin (alice, constructor caller) owns property 1 and uploads a deed
        registry
            .register_ipfs_document(
                1,
                valid_cid_v0(),
                DocumentType::Deed,
                Hash::from([0x44u8; 32]),
                2_048,
                String::from("application/pdf"),
                false,
            )
            .expect("Admin should register a document");

        // Stranger (bob) cannot register documents on property 1
        test::set_caller::<DefaultEnvironment>(accounts.bob);
        assert_eq!(
            registry.register_ipfs_document(
                1,
                format!("b{}", "d".repeat(52)),
                DocumentType::Title,
                Hash::from([0x55u8; 32]),
                1_024,
                String::from("application/pdf"),
                false,
            ),
            Err(IpfsError::Unauthorized),
            "Accounts without access must not attach documents"
        );

        // Stranger cannot verify the content hash either (no read access)
        assert_eq!(
            registry.verify_content_hash(1, Hash::from([0x44u8; 32])),
            Err(IpfsError::Unauthorized),
            "Accounts without access must not verify document contents"
        );

        // Unknown document id surfaces DocumentNotFound for authorized callers
        test::set_caller::<DefaultEnvironment>(accounts.alice);
        assert_eq!(
            registry.verify_content_hash(999, Hash::from([0x44u8; 32])),
            Err(IpfsError::DocumentNotFound),
            "Unknown document ids must be reported as such"
        );

        // Wrong hash verification fails with ContentHashMismatch
        assert_eq!(
            registry.verify_content_hash(1, Hash::from([0xFFu8; 32])),
            Err(IpfsError::ContentHashMismatch),
            "A mismatching content hash must be rejected"
        );

        // The legitimate hash still verifies
        assert_eq!(
            registry.verify_content_hash(1, Hash::from([0x44u8; 32])),
            Ok(true),
            "The recorded content hash must verify"
        );
    }
}
