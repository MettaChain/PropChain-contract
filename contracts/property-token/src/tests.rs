// Unit tests for the PropertyToken contract (Issue #101 - extracted from lib.rs)

// ============================================================================
// ERC-721 conformance test suite (Issue #561)
// Exercises the full ERC-721 surface and verifies behaviour against the spec.
// ============================================================================
#[cfg(test)]
mod erc721_conformance {
    use super::*;
    use ink::env::{test, DefaultEnvironment};

    fn sample_metadata() -> PropertyMetadata {
        PropertyMetadata {
            location: String::from("1 Blockchain Ave"),
            size: 500,
            legal_description: String::from("Conformance test property"),
            valuation: 1_000_000,
            documents_url: String::from("ipfs://QmConformance"),
        }
    }

    fn setup() -> (PropertyToken, TokenId) {
        let mut contract = PropertyToken::new();
        let accounts = test::default_accounts::<DefaultEnvironment>();
        test::set_caller::<DefaultEnvironment>(accounts.alice);
        let token_id = contract
            .register_property_with_token(sample_metadata())
            .expect("setup mint failed");
        (contract, token_id)
    }

    // ── name ────────────────────────────────────────────────────────────────
    #[ink::test]
    fn conformance_name_returns_string() {
        let (contract, _) = setup();
        let n = contract.name();
        assert!(!n.is_empty(), "name() must not be empty");
    }

    // ── symbol ──────────────────────────────────────────────────────────────
    #[ink::test]
    fn conformance_symbol_returns_string() {
        let (contract, _) = setup();
        let s = contract.symbol();
        assert!(!s.is_empty(), "symbol() must not be empty");
    }

    // ── balanceOf ───────────────────────────────────────────────────────────
    #[ink::test]
    fn conformance_balance_of_owner_is_one() {
        let (contract, _) = setup();
        let accounts = test::default_accounts::<DefaultEnvironment>();
        assert_eq!(contract.balance_of(accounts.alice), 1);
    }

    #[ink::test]
    fn conformance_balance_of_non_owner_is_zero() {
        let (contract, _) = setup();
        let accounts = test::default_accounts::<DefaultEnvironment>();
        assert_eq!(contract.balance_of(accounts.bob), 0);
    }

    // ── ownerOf ─────────────────────────────────────────────────────────────
    #[ink::test]
    fn conformance_owner_of_minted_token() {
        let (contract, token_id) = setup();
        let accounts = test::default_accounts::<DefaultEnvironment>();
        assert_eq!(contract.owner_of(token_id), Some(accounts.alice));
    }

    #[ink::test]
    fn conformance_owner_of_nonexistent_is_none() {
        let (contract, _) = setup();
        assert_eq!(contract.owner_of(999), None);
    }

    // ── approve / getApproved ───────────────────────────────────────────────
    #[ink::test]
    fn conformance_approve_and_get_approved() {
        let (mut contract, token_id) = setup();
        let accounts = test::default_accounts::<DefaultEnvironment>();
        test::set_caller::<DefaultEnvironment>(accounts.alice);

        assert_eq!(contract.get_approved(token_id), None);
        contract.approve(accounts.bob, token_id).unwrap();
        assert_eq!(contract.get_approved(token_id), Some(accounts.bob));
    }

    #[ink::test]
    fn conformance_approve_nonexistent_token_fails() {
        let (mut contract, _) = setup();
        let accounts = test::default_accounts::<DefaultEnvironment>();
        test::set_caller::<DefaultEnvironment>(accounts.alice);
        assert_eq!(
            contract.approve(accounts.bob, 999),
            Err(Error::TokenNotFound)
        );
    }

    #[ink::test]
    fn conformance_approve_by_non_owner_fails() {
        let (mut contract, token_id) = setup();
        let accounts = test::default_accounts::<DefaultEnvironment>();
        test::set_caller::<DefaultEnvironment>(accounts.bob);
        assert_eq!(
            contract.approve(accounts.charlie, token_id),
            Err(Error::Unauthorized)
        );
    }

    // ── setApprovalForAll / isApprovedForAll ────────────────────────────────
    #[ink::test]
    fn conformance_set_approval_for_all() {
        let (mut contract, _) = setup();
        let accounts = test::default_accounts::<DefaultEnvironment>();
        test::set_caller::<DefaultEnvironment>(accounts.alice);

        assert!(!contract.is_approved_for_all(accounts.alice, accounts.bob));
        contract.set_approval_for_all(accounts.bob, true).unwrap();
        assert!(contract.is_approved_for_all(accounts.alice, accounts.bob));
    }

    #[ink::test]
    fn conformance_revoke_approval_for_all() {
        let (mut contract, _) = setup();
        let accounts = test::default_accounts::<DefaultEnvironment>();
        test::set_caller::<DefaultEnvironment>(accounts.alice);

        contract.set_approval_for_all(accounts.bob, true).unwrap();
        contract.set_approval_for_all(accounts.bob, false).unwrap();
        assert!(!contract.is_approved_for_all(accounts.alice, accounts.bob));
    }

    // ── transferFrom ────────────────────────────────────────────────────────
    #[ink::test]
    fn conformance_owner_can_transfer_from() {
        let (mut contract, token_id) = setup();
        let accounts = test::default_accounts::<DefaultEnvironment>();
        test::set_caller::<DefaultEnvironment>(accounts.alice);

        contract
            .transfer_from(accounts.alice, accounts.bob, token_id)
            .unwrap();

        assert_eq!(contract.owner_of(token_id), Some(accounts.bob));
        assert_eq!(contract.balance_of(accounts.alice), 0);
        assert_eq!(contract.balance_of(accounts.bob), 1);
    }

    #[ink::test]
    fn conformance_approved_can_transfer_from() {
        let (mut contract, token_id) = setup();
        let accounts = test::default_accounts::<DefaultEnvironment>();
        test::set_caller::<DefaultEnvironment>(accounts.alice);
        contract.approve(accounts.bob, token_id).unwrap();

        test::set_caller::<DefaultEnvironment>(accounts.bob);
        contract
            .transfer_from(accounts.alice, accounts.bob, token_id)
            .unwrap();
        assert_eq!(contract.owner_of(token_id), Some(accounts.bob));
    }

    #[ink::test]
    fn conformance_operator_can_transfer_from() {
        let (mut contract, token_id) = setup();
        let accounts = test::default_accounts::<DefaultEnvironment>();
        test::set_caller::<DefaultEnvironment>(accounts.alice);
        contract.set_approval_for_all(accounts.bob, true).unwrap();

        test::set_caller::<DefaultEnvironment>(accounts.bob);
        contract
            .transfer_from(accounts.alice, accounts.bob, token_id)
            .unwrap();
        assert_eq!(contract.owner_of(token_id), Some(accounts.bob));
    }

    #[ink::test]
    fn conformance_transfer_clears_approval() {
        let (mut contract, token_id) = setup();
        let accounts = test::default_accounts::<DefaultEnvironment>();
        test::set_caller::<DefaultEnvironment>(accounts.alice);
        contract.approve(accounts.bob, token_id).unwrap();
        contract
            .transfer_from(accounts.alice, accounts.bob, token_id)
            .unwrap();
        assert_eq!(contract.get_approved(token_id), None);
    }

    #[ink::test]
    fn conformance_unauthorized_transfer_fails() {
        let (mut contract, token_id) = setup();
        let accounts = test::default_accounts::<DefaultEnvironment>();
        test::set_caller::<DefaultEnvironment>(accounts.bob);
        assert_eq!(
            contract.transfer_from(accounts.alice, accounts.bob, token_id),
            Err(Error::Unauthorized)
        );
    }

    #[ink::test]
    fn conformance_transfer_to_zero_address_fails() {
        let (mut contract, token_id) = setup();
        let accounts = test::default_accounts::<DefaultEnvironment>();
        test::set_caller::<DefaultEnvironment>(accounts.alice);

        let zero_address = AccountId::from([0x0; 32]);
        let result = contract.transfer_from(accounts.alice, zero_address, token_id);
        assert_eq!(result, Err(Error::InvalidRecipient));
    }

    // ── safeTransferFrom ────────────────────────────────────────────────────
    #[ink::test]
    fn conformance_safe_transfer_from_owner() {
        let (mut contract, token_id) = setup();
        let accounts = test::default_accounts::<DefaultEnvironment>();
        test::set_caller::<DefaultEnvironment>(accounts.alice);

        contract
            .safe_transfer_from(accounts.alice, accounts.bob, token_id)
            .unwrap();
        assert_eq!(contract.owner_of(token_id), Some(accounts.bob));
    }

    #[ink::test]
    fn conformance_safe_transfer_from_unauthorized_fails() {
        let (mut contract, token_id) = setup();
        let accounts = test::default_accounts::<DefaultEnvironment>();
        test::set_caller::<DefaultEnvironment>(accounts.bob);
        assert_eq!(
            contract.safe_transfer_from(accounts.alice, accounts.bob, token_id),
            Err(Error::Unauthorized)
        );
    }

    // ── conformance report ──────────────────────────────────────────────────
    #[ink::test]
    fn conformance_report_is_valid_json_structure() {
        let (contract, _) = setup();
        let report = contract.erc721_conformance_report();
        assert!(report.contains("\"ERC-721\""), "report must identify standard");
        assert!(report.contains("\"implemented\": 10"), "all 10 functions must be implemented");
        assert!(report.contains("\"compliance_level\": \"full\""));
        assert!(report.contains("\"balanceOf\""));
        assert!(report.contains("\"ownerOf\""));
        assert!(report.contains("\"approve\""));
        assert!(report.contains("\"getApproved\""));
        assert!(report.contains("\"setApprovalForAll\""));
        assert!(report.contains("\"isApprovedForAll\""));
        assert!(report.contains("\"transferFrom\""));
        assert!(report.contains("\"safeTransferFrom\""));
        assert!(report.contains("\"name\""));
        assert!(report.contains("\"symbol\""));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ink::env::{test, DefaultEnvironment};

    // =========================================================================
    // Issue #557 – metadata versioning tests
    // =========================================================================

    fn make_metadata(location: &str, valuation: u128) -> PropertyMetadata {
        PropertyMetadata {
            location: String::from(location),
            size: 1000,
            legal_description: String::from("Test property"),
            valuation,
            documents_url: String::from("ipfs://docs"),
        }
    }

    #[ink::test]
    fn test_metadata_version_count_starts_at_zero() {
        let mut contract = PropertyToken::new();
        let accounts = test::default_accounts::<DefaultEnvironment>();
        test::set_caller::<DefaultEnvironment>(accounts.alice);

        let token_id = contract
            .register_property_with_token(make_metadata("123 Main St", 500_000))
            .unwrap();

        assert_eq!(contract.metadata_version_count(token_id), 0);
    }

    #[ink::test]
    fn test_first_update_creates_version_one() {
        let mut contract = PropertyToken::new();
        let accounts = test::default_accounts::<DefaultEnvironment>();
        test::set_caller::<DefaultEnvironment>(accounts.alice);

        let token_id = contract
            .register_property_with_token(make_metadata("123 Main St", 500_000))
            .unwrap();

        contract
            .update_property_metadata(token_id, make_metadata("123 Main St Updated", 600_000))
            .unwrap();

        assert_eq!(contract.metadata_version_count(token_id), 1);
    }

    #[ink::test]
    fn test_get_metadata_at_returns_snapshot() {
        let mut contract = PropertyToken::new();
        let accounts = test::default_accounts::<DefaultEnvironment>();
        test::set_caller::<DefaultEnvironment>(accounts.alice);

        let original = make_metadata("Original Location", 100_000);
        let token_id = contract
            .register_property_with_token(original.clone())
            .unwrap();

        let updated = make_metadata("Updated Location", 200_000);
        contract
            .update_property_metadata(token_id, updated.clone())
            .unwrap();

        let v1 = contract.get_metadata_at(token_id, 1).expect("version 1 must exist");
        assert_eq!(v1.version_number, 1);
        assert_eq!(v1.metadata.location, original.location);
        assert_eq!(v1.metadata.valuation, original.valuation);
    }

    #[ink::test]
    fn test_multiple_versions_tracked() {
        let mut contract = PropertyToken::new();
        let accounts = test::default_accounts::<DefaultEnvironment>();
        test::set_caller::<DefaultEnvironment>(accounts.alice);

        let token_id = contract
            .register_property_with_token(make_metadata("v0", 100_000))
            .unwrap();

        contract
            .update_property_metadata(token_id, make_metadata("v1", 200_000))
            .unwrap();
        contract
            .update_property_metadata(token_id, make_metadata("v2", 300_000))
            .unwrap();
        contract
            .update_property_metadata(token_id, make_metadata("v3", 400_000))
            .unwrap();

        assert_eq!(contract.metadata_version_count(token_id), 3);

        let v1 = contract.get_metadata_at(token_id, 1).unwrap();
        let v2 = contract.get_metadata_at(token_id, 2).unwrap();
        let v3 = contract.get_metadata_at(token_id, 3).unwrap();
        assert_eq!(v1.metadata.valuation, 100_000);
        assert_eq!(v2.metadata.valuation, 200_000);
        assert_eq!(v3.metadata.valuation, 300_000);
    }

    #[ink::test]
    fn test_get_metadata_at_version_zero_returns_none() {
        let mut contract = PropertyToken::new();
        let accounts = test::default_accounts::<DefaultEnvironment>();
        test::set_caller::<DefaultEnvironment>(accounts.alice);

        let token_id = contract
            .register_property_with_token(make_metadata("Test", 100_000))
            .unwrap();

        assert_eq!(contract.get_metadata_at(token_id, 0), None);
    }

    #[ink::test]
    fn test_get_metadata_at_nonexistent_version_returns_none() {
        let mut contract = PropertyToken::new();
        let accounts = test::default_accounts::<DefaultEnvironment>();
        test::set_caller::<DefaultEnvironment>(accounts.alice);

        let token_id = contract
            .register_property_with_token(make_metadata("Test", 100_000))
            .unwrap();

        assert_eq!(contract.get_metadata_at(token_id, 5), None);
    }

    #[ink::test]
    fn test_metadata_update_unauthorized() {
        let mut contract = PropertyToken::new();
        let accounts = test::default_accounts::<DefaultEnvironment>();
        test::set_caller::<DefaultEnvironment>(accounts.alice);

        let token_id = contract
            .register_property_with_token(make_metadata("Test", 100_000))
            .unwrap();

        test::set_caller::<DefaultEnvironment>(accounts.bob);
        let result =
            contract.update_property_metadata(token_id, make_metadata("Hacked", 999_999));
        assert_eq!(result, Err(Error::Unauthorized));
        assert_eq!(contract.metadata_version_count(token_id), 0);
    }

    // =========================================================================
    // Issue #556 – mint_batch tests
    // =========================================================================

    #[ink::test]
    fn test_mint_batch_basic() {
        let mut contract = setup_contract();
        let accounts = test::default_accounts::<DefaultEnvironment>();
        test::set_caller::<DefaultEnvironment>(accounts.alice);

        let uris = vec![
            String::from("ipfs://QmProp1"),
            String::from("ipfs://QmProp2"),
            String::from("ipfs://QmProp3"),
        ];
        let result = contract.mint_batch(accounts.alice, uris);
        assert!(result.is_ok());
        let ids = result.unwrap();
        assert_eq!(ids.len(), 3);
        assert_eq!(contract.total_supply(), 3);
    }

    #[ink::test]
    fn test_mint_batch_assigns_correct_owner() {
        let mut contract = setup_contract();
        let accounts = test::default_accounts::<DefaultEnvironment>();
        test::set_caller::<DefaultEnvironment>(accounts.alice);

        let uris = vec![String::from("ipfs://QmA"), String::from("ipfs://QmB")];
        let ids = contract.mint_batch(accounts.alice, uris).unwrap();

        for id in &ids {
            assert_eq!(contract.owner_of(*id), Some(accounts.alice));
        }
        assert_eq!(contract.balance_of(accounts.alice), 2);
    }

    #[ink::test]
    fn test_mint_batch_stores_uris() {
        let mut contract = setup_contract();
        let accounts = test::default_accounts::<DefaultEnvironment>();
        test::set_caller::<DefaultEnvironment>(accounts.alice);

        let uris = vec![String::from("ipfs://QmX"), String::from("ipfs://QmY")];
        let ids = contract.mint_batch(accounts.alice, uris.clone()).unwrap();

        for (idx, id) in ids.iter().enumerate() {
            let stored = contract.token_uris.get(id);
            assert_eq!(stored, Some(uris[idx].clone()));
        }
    }

    #[ink::test]
    fn test_mint_batch_empty_fails() {
        let mut contract = setup_contract();
        let accounts = test::default_accounts::<DefaultEnvironment>();
        test::set_caller::<DefaultEnvironment>(accounts.alice);

        let result = contract.mint_batch(accounts.alice, vec![]);
        assert_eq!(result, Err(Error::InvalidAmount));
    }

    #[ink::test]
    fn test_mint_batch_unauthorized_fails() {
        let mut contract = setup_contract();
        let accounts = test::default_accounts::<DefaultEnvironment>();

        // Bob tries to mint tokens owned by Alice – caller must be owner or admin
        test::set_caller::<DefaultEnvironment>(accounts.bob);
        let result = contract.mint_batch(
            accounts.alice,
            vec![String::from("ipfs://QmUnauth")],
        );
        assert_eq!(result, Err(Error::Unauthorized));
    }

    #[ink::test]
    fn test_mint_batch_token_ids_sequential() {
        let mut contract = setup_contract();
        let accounts = test::default_accounts::<DefaultEnvironment>();
        test::set_caller::<DefaultEnvironment>(accounts.alice);

        let uris = vec![
            String::from("ipfs://QmSeq1"),
            String::from("ipfs://QmSeq2"),
        ];
        let ids = contract.mint_batch(accounts.alice, uris).unwrap();
        assert_eq!(ids[0] + 1, ids[1], "Token IDs must be sequential");
    }

    fn setup_contract() -> PropertyToken {
        PropertyToken::new()
    }

    #[ink::test]
    fn test_constructor_works() {
        let contract = setup_contract();
        assert_eq!(contract.total_supply(), 0);
        assert_eq!(contract.current_token_id(), 0);
    }

    #[ink::test]
    fn test_register_property_with_token() {
        let mut contract = setup_contract();

        let metadata = PropertyMetadata {
            location: String::from("123 Main St"),
            size: 1000,
            legal_description: String::from("Sample property"),
            valuation: 500000,
            documents_url: String::from("ipfs://sample-docs"),
        };

        let result = contract.register_property_with_token(metadata.clone());
        assert!(result.is_ok());

        let token_id = result.expect("Token registration should succeed in test");
        assert_eq!(token_id, 1);
        assert_eq!(contract.total_supply(), 1);
    }

    #[ink::test]
    fn test_balance_of() {
        let mut contract = setup_contract();

        let metadata = PropertyMetadata {
            location: String::from("123 Main St"),
            size: 1000,
            legal_description: String::from("Sample property"),
            valuation: 500000,
            documents_url: String::from("ipfs://sample-docs"),
        };

        let _token_id = contract
            .register_property_with_token(metadata)
            .expect("Token registration should succeed in test");
        let _caller = AccountId::from([1u8; 32]);

        // Set up mock caller for the test
        let accounts = test::default_accounts::<DefaultEnvironment>();
        test::set_caller::<DefaultEnvironment>(accounts.alice);

        assert_eq!(contract.balance_of(accounts.alice), 1);
    }

    #[ink::test]
    fn test_attach_legal_document() {
        let mut contract = setup_contract();

        let metadata = PropertyMetadata {
            location: String::from("123 Main St"),
            size: 1000,
            legal_description: String::from("Sample property"),
            valuation: 500000,
            documents_url: String::from("ipfs://sample-docs"),
        };

        let token_id = contract
            .register_property_with_token(metadata)
            .expect("Token registration should succeed in test");

        let accounts = test::default_accounts::<DefaultEnvironment>();
        test::set_caller::<DefaultEnvironment>(accounts.alice);

        let doc_hash = Hash::from([1u8; 32]);
        let doc_type = String::from("Deed");

        let result = contract.attach_legal_document(token_id, doc_hash, doc_type);
        assert!(result.is_ok());
    }

    #[ink::test]
    fn test_verify_compliance() {
        let mut contract = setup_contract();

        let metadata = PropertyMetadata {
            location: String::from("123 Main St"),
            size: 1000,
            legal_description: String::from("Sample property"),
            valuation: 500000,
            documents_url: String::from("ipfs://sample-docs"),
        };

        let token_id = contract
            .register_property_with_token(metadata)
            .expect("Token registration should succeed in test");

        let _accounts = test::default_accounts::<DefaultEnvironment>();
        test::set_caller::<DefaultEnvironment>(contract.admin());

        let result = contract.verify_compliance(token_id, true);
        assert!(result.is_ok());

        let compliance_info = contract
            .compliance_flags
            .get(&token_id)
            .expect("Compliance info should exist after verification");
        assert!(compliance_info.verified);
    }

    // ============================================================================
    // EDGE CASE TESTS
    // ============================================================================

    #[ink::test]
    fn test_transfer_from_nonexistent_token() {
        let mut contract = setup_contract();
        let accounts = test::default_accounts::<DefaultEnvironment>();

        let result = contract.transfer_from(accounts.alice, accounts.bob, 999);
        assert_eq!(result, Err(Error::TokenNotFound));
    }

    #[ink::test]
    fn test_transfer_from_unauthorized_caller() {
        let mut contract = setup_contract();
        let accounts = test::default_accounts::<DefaultEnvironment>();
        test::set_caller::<DefaultEnvironment>(accounts.alice);

        let metadata = PropertyMetadata {
            location: String::from("123 Main St"),
            size: 1000,
            legal_description: String::from("Sample property"),
            valuation: 500000,
            documents_url: String::from("ipfs://sample-docs"),
        };

        let token_id = contract
            .register_property_with_token(metadata)
            .expect("Token registration should succeed in test");

        // Bob tries to transfer Alice's token without approval
        test::set_caller::<DefaultEnvironment>(accounts.bob);
        let result = contract.transfer_from(accounts.alice, accounts.bob, token_id);
        assert_eq!(result, Err(Error::Unauthorized));
    }

    #[ink::test]
    fn test_approve_nonexistent_token() {
        let mut contract = setup_contract();
        let accounts = test::default_accounts::<DefaultEnvironment>();

        let result = contract.approve(accounts.bob, 999);
        assert_eq!(result, Err(Error::TokenNotFound));
    }

    #[ink::test]
    fn test_approve_unauthorized_caller() {
        let mut contract = setup_contract();
        let accounts = test::default_accounts::<DefaultEnvironment>();
        test::set_caller::<DefaultEnvironment>(accounts.alice);

        let metadata = PropertyMetadata {
            location: String::from("123 Main St"),
            size: 1000,
            legal_description: String::from("Sample property"),
            valuation: 500000,
            documents_url: String::from("ipfs://sample-docs"),
        };

        let token_id = contract
            .register_property_with_token(metadata)
            .expect("Token registration should succeed in test");

        // Bob tries to approve without being owner or operator
        test::set_caller::<DefaultEnvironment>(accounts.bob);
        let result = contract.approve(accounts.charlie, token_id);
        assert_eq!(result, Err(Error::Unauthorized));
    }

    #[ink::test]
    fn test_owner_of_nonexistent_token() {
        let contract = setup_contract();

        assert_eq!(contract.owner_of(0), None);
        assert_eq!(contract.owner_of(1), None);
        assert_eq!(contract.owner_of(u64::MAX), None);
    }

    #[ink::test]
    fn test_balance_of_nonexistent_account() {
        let contract = setup_contract();
        let nonexistent = AccountId::from([0xFF; 32]);

        assert_eq!(contract.balance_of(nonexistent), 0);
    }

    #[ink::test]
    fn test_attach_document_to_nonexistent_token() {
        let mut contract = setup_contract();
        let doc_hash = Hash::from([1u8; 32]);

        let result = contract.attach_legal_document(999, doc_hash, "Deed".to_string());
        assert_eq!(result, Err(Error::TokenNotFound));
    }

    #[ink::test]
    fn test_attach_document_unauthorized() {
        let mut contract = setup_contract();
        let accounts = test::default_accounts::<DefaultEnvironment>();
        test::set_caller::<DefaultEnvironment>(accounts.alice);

        let metadata = PropertyMetadata {
            location: String::from("123 Main St"),
            size: 1000,
            legal_description: String::from("Sample property"),
            valuation: 500000,
            documents_url: String::from("ipfs://sample-docs"),
        };

        let token_id = contract
            .register_property_with_token(metadata)
            .expect("Token registration should succeed in test");

        // Bob tries to attach document
        test::set_caller::<DefaultEnvironment>(accounts.bob);
        let doc_hash = Hash::from([1u8; 32]);
        let result = contract.attach_legal_document(token_id, doc_hash, "Deed".to_string());
        assert_eq!(result, Err(Error::Unauthorized));
    }

    #[ink::test]
    fn test_verify_compliance_nonexistent_token() {
        let mut contract = setup_contract();
        let accounts = test::default_accounts::<DefaultEnvironment>();
        test::set_caller::<DefaultEnvironment>(accounts.alice);

        let result = contract.verify_compliance(999, true);
        assert_eq!(result, Err(Error::TokenNotFound));
    }

    #[ink::test]
    fn test_initiate_bridge_invalid_chain() {
        let mut contract = setup_contract();
        let accounts = test::default_accounts::<DefaultEnvironment>();
        test::set_caller::<DefaultEnvironment>(accounts.alice);

        let metadata = PropertyMetadata {
            location: String::from("123 Main St"),
            size: 1000,
            legal_description: String::from("Sample property"),
            valuation: 500000,
            documents_url: String::from("ipfs://sample-docs"),
        };

        let token_id = contract
            .register_property_with_token(metadata)
            .expect("Token registration should succeed in test");

        // Try to bridge to unsupported chain
        let result = contract.initiate_bridge_multisig(
            token_id,
            999, // Invalid chain ID
            accounts.bob,
            2,    // required_signatures
            None, // timeout_blocks
        );

        assert_eq!(result, Err(Error::InvalidChain));
    }

    #[ink::test]
    fn test_initiate_bridge_nonexistent_token() {
        let mut contract = setup_contract();
        let accounts = test::default_accounts::<DefaultEnvironment>();

        let result = contract.initiate_bridge_multisig(
            999,          // nonexistent token_id
            2,            // destination_chain
            accounts.bob, // recipient
            2,            // required_signatures
            None,         // timeout_blocks
        );

        assert_eq!(result, Err(Error::TokenNotFound));
    }

    #[ink::test]
    fn test_sign_bridge_request_nonexistent() {
        let mut contract = setup_contract();
        let _accounts = test::default_accounts::<DefaultEnvironment>();

        let result = contract.sign_bridge_request(999, true);
        assert_eq!(result, Err(Error::InvalidRequest));
    }

    #[ink::test]
    fn test_register_multiple_properties_increments_ids() {
        let mut contract = setup_contract();
        let accounts = test::default_accounts::<DefaultEnvironment>();
        test::set_caller::<DefaultEnvironment>(accounts.alice);

        for i in 1..=10 {
            let metadata = PropertyMetadata {
                location: format!("Property {}", i),
                size: 1000 + i,
                legal_description: format!("Description {}", i),
                valuation: 100_000 + (i as u128 * 1000),
                documents_url: format!("ipfs://prop{}", i),
            };

            let token_id = contract
                .register_property_with_token(metadata)
                .expect("Token registration should succeed in test");
            assert_eq!(token_id, i);
            assert_eq!(contract.total_supply(), i);
        }
    }

    #[ink::test]
    fn test_transfer_preserves_total_supply() {
        let mut contract = setup_contract();
        let accounts = test::default_accounts::<DefaultEnvironment>();
        test::set_caller::<DefaultEnvironment>(accounts.alice);

        let metadata = PropertyMetadata {
            location: String::from("123 Main St"),
            size: 1000,
            legal_description: String::from("Sample property"),
            valuation: 500000,
            documents_url: String::from("ipfs://sample-docs"),
        };

        let token_id = contract
            .register_property_with_token(metadata)
            .expect("Token registration should succeed in test");

        let initial_supply = contract.total_supply();

        contract
            .transfer_from(accounts.alice, accounts.bob, token_id)
            .expect("Transfer should succeed");

        // Total supply should remain constant
        assert_eq!(contract.total_supply(), initial_supply);
    }

    #[ink::test]
    fn test_balance_of_batch_empty_vectors() {
        let contract = setup_contract();

        let result = contract.balance_of_batch(Vec::new(), Vec::new());
        assert_eq!(result, Vec::<u128>::new());
    }

    #[ink::test]
    fn test_get_error_count_nonexistent() {
        let contract = setup_contract();
        let accounts = test::default_accounts::<DefaultEnvironment>();

        let count = contract.get_error_count(accounts.alice, "NONEXISTENT".to_string());
        assert_eq!(count, 0);
    }

    #[ink::test]
    fn test_get_error_rate_nonexistent() {
        let contract = setup_contract();

        let rate = contract.get_error_rate("NONEXISTENT".to_string());
        assert_eq!(rate, 0);
    }

    #[ink::test]
    fn test_get_recent_errors_unauthorized() {
        let contract = setup_contract();
        let accounts = test::default_accounts::<DefaultEnvironment>();

        // Non-admin tries to get errors
        test::set_caller::<DefaultEnvironment>(accounts.bob);
        let errors = contract.get_recent_errors(10);
        assert_eq!(errors, Vec::new());
    }

    #[ink::test]
    fn test_property_management_linkage() {
        let mut contract = setup_contract();
        let accounts = test::default_accounts::<DefaultEnvironment>();
        test::set_caller::<DefaultEnvironment>(accounts.alice);

        let metadata = PropertyMetadata {
            location: String::from("123 Main St"),
            size: 1000,
            legal_description: String::from("Sample property"),
            valuation: 500000,
            documents_url: String::from("ipfs://sample-docs"),
        };
        let token_id = contract
            .register_property_with_token(metadata)
            .expect("register");

        test::set_caller::<DefaultEnvironment>(contract.admin());
        contract
            .set_property_management_contract(Some(accounts.charlie))
            .expect("set pm contract");
        assert_eq!(
            contract.get_property_management_contract(),
            Some(accounts.charlie)
        );

        test::set_caller::<DefaultEnvironment>(accounts.alice);
        contract
            .assign_management_agent(token_id, accounts.bob)
            .expect("agent");
        assert_eq!(contract.get_management_agent(token_id), Some(accounts.bob));

        contract.clear_management_agent(token_id).expect("clear");
        assert_eq!(contract.get_management_agent(token_id), None);
    }

    #[ink::test]
    fn test_distribute_rental_income_by_management_agent() {
        let mut contract = setup_contract();
        let accounts = test::default_accounts::<DefaultEnvironment>();
        test::set_caller::<DefaultEnvironment>(accounts.alice);

        let metadata = PropertyMetadata {
            location: String::from("456 Rental Rd"),
            size: 900,
            legal_description: String::from("Rental Property"),
            valuation: 800_000,
            documents_url: String::from("ipfs://rental-docs"),
        };
        let token_id = contract
            .register_property_with_token(metadata)
            .expect("register");

        assert!(contract.issue_shares(token_id, accounts.alice, 1_000).is_ok());
        assert!(contract.transfer_shares(accounts.alice, accounts.bob, token_id, 500).is_ok());

        contract.assign_management_agent(token_id, accounts.charlie).expect("assign agent");
        test::set_caller::<DefaultEnvironment>(accounts.charlie);
        test::set_value_transferred::<DefaultEnvironment>(10_000);
        assert!(contract.distribute_rental_income(token_id).is_ok());

        test::set_caller::<DefaultEnvironment>(accounts.bob);
        let withdrawn = contract.withdraw_dividends(token_id).unwrap();
        assert!(withdrawn > 0);
    }

    // ── Staking tests (Issue #197) ─────────────────────────────────────────

    fn setup_token_with_shares(amount: u128) -> (PropertyToken, TokenId) {
        let mut contract = setup_contract();
        let accounts = test::default_accounts::<DefaultEnvironment>();
        let metadata = PropertyMetadata {
            location: String::from("1 Stake St"),
            size: 500,
            legal_description: String::from("Staking test property"),
            valuation: 100000,
            documents_url: String::from("ipfs://stake"),
        };
        let token_id = contract
            .register_property_with_token(metadata)
            .expect("register");
        contract
            .issue_shares(token_id, accounts.alice, amount)
            .expect("issue shares");
        (contract, token_id)
    }

    #[ink::test]
    fn test_stake_shares_success() {
        // register_property_with_token seeds balance with 1; issue 999 more → 1000 total
        let (mut contract, token_id) = setup_token_with_shares(999);
        let accounts = test::default_accounts::<DefaultEnvironment>();
        test::set_caller::<DefaultEnvironment>(accounts.alice);

        contract
            .stake_shares(token_id, 500, LockPeriod::Flexible)
            .expect("stake");

        let stake = contract
            .get_share_stake(accounts.alice, token_id)
            .expect("stake record");
        assert_eq!(stake.amount, 500);
        assert_eq!(contract.share_balance_of(accounts.alice, token_id), 500);
    }

    #[ink::test]
    fn test_stake_zero_amount_fails() {
        let (mut contract, token_id) = setup_token_with_shares(1000);
        let accounts = test::default_accounts::<DefaultEnvironment>();
        test::set_caller::<DefaultEnvironment>(accounts.alice);

        let result = contract.stake_shares(token_id, 0, LockPeriod::Flexible);
        assert_eq!(result, Err(Error::InvalidAmount));
    }

    #[ink::test]
    fn test_stake_insufficient_balance_fails() {
        let (mut contract, token_id) = setup_token_with_shares(100);
        let accounts = test::default_accounts::<DefaultEnvironment>();
        test::set_caller::<DefaultEnvironment>(accounts.alice);

        let result = contract.stake_shares(token_id, 200, LockPeriod::Flexible);
        assert_eq!(result, Err(Error::InsufficientBalance));
    }

    #[ink::test]
    fn test_batch_transfer_success() {
        let mut contract = setup_contract();
        let accounts = test::default_accounts::<DefaultEnvironment>();
        test::set_caller::<DefaultEnvironment>(accounts.alice);

        // Mint shares for two tokens
        let token_id_1: TokenId = 1;
        let token_id_2: TokenId = 2;
        contract.balances.insert((&accounts.alice, &token_id_1), &100u128);
        contract.balances.insert((&accounts.alice, &token_id_2), &200u128);

        let result = contract.safe_batch_transfer_from(
            accounts.alice,
            accounts.bob,
            vec![token_id_1, token_id_2],
            vec![50u128, 100u128],
            vec![],
        );
        assert!(result.is_ok());
        assert_eq!(contract.balances.get((&accounts.alice, &token_id_1)).unwrap_or(0), 50);
        assert_eq!(contract.balances.get((&accounts.bob, &token_id_2)).unwrap_or(0), 100);
    }

    #[ink::test]
    fn test_batch_transfer_length_mismatch() {
        let mut contract = setup_contract();
        let accounts = test::default_accounts::<DefaultEnvironment>();
        test::set_caller::<DefaultEnvironment>(accounts.alice);

        let result = contract.safe_batch_transfer_from(
            accounts.alice,
            accounts.bob,
            vec![1u64, 2u64],
            vec![10u128],  // mismatched length
            vec![],
        );
        assert_eq!(result, Err(Error::LengthMismatch));
    }

    #[ink::test]
    fn test_batch_transfer_insufficient_balance() {
        let mut contract = setup_contract();
        let accounts = test::default_accounts::<DefaultEnvironment>();
        test::set_caller::<DefaultEnvironment>(accounts.alice);

        contract.balances.insert((&accounts.alice, &1u64), &10u128);

        let result = contract.safe_batch_transfer_from(
            accounts.alice,
            accounts.bob,
            vec![1u64],
            vec![999u128],  // more than balance
            vec![],
        );
        assert_eq!(result, Err(Error::InsufficientBalance));
    }

    #[ink::test]
    fn test_stake_already_staked_fails() {
        let (mut contract, token_id) = setup_token_with_shares(1000);
        let accounts = test::default_accounts::<DefaultEnvironment>();
        test::set_caller::<DefaultEnvironment>(accounts.alice);

        contract
            .stake_shares(token_id, 500, LockPeriod::Flexible)
            .expect("first stake");
        let result = contract.stake_shares(token_id, 100, LockPeriod::Flexible);
        assert_eq!(result, Err(Error::AlreadyStaked));
    }

    #[ink::test]
    fn test_unstake_lock_active_fails() {
        let (mut contract, token_id) = setup_token_with_shares(1000);
        let accounts = test::default_accounts::<DefaultEnvironment>();
        test::set_caller::<DefaultEnvironment>(accounts.alice);

        contract
            .stake_shares(token_id, 500, LockPeriod::ThirtyDays)
            .expect("stake");
        let result = contract.unstake_shares(token_id);
        assert_eq!(result, Err(Error::LockActive));
    }

    #[ink::test]
    fn test_unstake_flexible_succeeds() {
        // register_property_with_token seeds balance with 1; issue 999 more → 1000 total
        let (mut contract, token_id) = setup_token_with_shares(999);
        let accounts = test::default_accounts::<DefaultEnvironment>();
        test::set_caller::<DefaultEnvironment>(accounts.alice);

        contract
            .stake_shares(token_id, 600, LockPeriod::Flexible)
            .expect("stake");
        contract.unstake_shares(token_id).expect("unstake");

        assert!(contract.get_share_stake(accounts.alice, token_id).is_none());
        assert_eq!(contract.share_balance_of(accounts.alice, token_id), 1000);
    }

    #[ink::test]
    fn test_unstake_not_staked_fails() {
        let (mut contract, token_id) = setup_token_with_shares(1000);
        let accounts = test::default_accounts::<DefaultEnvironment>();
        test::set_caller::<DefaultEnvironment>(accounts.alice);

        let result = contract.unstake_shares(token_id);
        assert_eq!(result, Err(Error::StakeNotFound));
    }

    #[ink::test]
    fn test_claim_rewards_no_stake_fails() {
        let (mut contract, token_id) = setup_token_with_shares(1000);
        let accounts = test::default_accounts::<DefaultEnvironment>();
        test::set_caller::<DefaultEnvironment>(accounts.alice);

        let result = contract.claim_stake_rewards(token_id);
        assert_eq!(result, Err(Error::StakeNotFound));
    }

    #[ink::test]
    fn test_governance_weight_non_staker() {
        // register_property_with_token seeds balance with 1; issue 799 more → 800 total
        let (contract, token_id) = setup_token_with_shares(799);
        let accounts = test::default_accounts::<DefaultEnvironment>();

        let weight = contract.get_governance_weight(accounts.alice, token_id);
        assert_eq!(weight, 800);
    }

    #[ink::test]
    fn test_governance_weight_staker_boosted() {
        let (mut contract, token_id) = setup_token_with_shares(1000);
        let accounts = test::default_accounts::<DefaultEnvironment>();
        test::set_caller::<DefaultEnvironment>(accounts.alice);

        contract
            .stake_shares(token_id, 1000, LockPeriod::OneYear)
            .expect("stake");

        // MULTIPLIER_1_YEAR = 150 (1.5×); 1000 * 150 / 100 = 1500
        let weight = contract.get_governance_weight(accounts.alice, token_id);
        assert_eq!(weight, 1500);
    }

    #[ink::test]
    fn test_vote_uses_boosted_weight() {
        let (mut contract, token_id) = setup_token_with_shares(1000);
        let accounts = test::default_accounts::<DefaultEnvironment>();
        test::set_caller::<DefaultEnvironment>(accounts.alice);

        contract
            .stake_shares(token_id, 1000, LockPeriod::OneYear)
            .expect("stake");

        // Quorum of 1200 — unreachable with raw balance (1000) but met with 1.5× boost (1500)
        let proposal_id = contract
            .create_proposal(token_id, 1200, Hash::from([9u8; 32]))
            .expect("proposal");
        contract.vote(token_id, proposal_id, true).expect("vote");

        let proposal = contract
            .get_proposal(token_id, proposal_id)
            .expect("proposal exists");
        assert!(
            proposal.for_votes >= 1200,
            "boosted votes should meet quorum"
        );
    }

    #[ink::test]
    fn test_batch_transfer_unauthorized() {
        let mut contract = setup_contract();
        let accounts = test::default_accounts::<DefaultEnvironment>();
        test::set_caller::<DefaultEnvironment>(accounts.bob); // bob tries to move alice's tokens

        let result = contract.safe_batch_transfer_from(
            accounts.alice,
            accounts.bob,
            vec![1u64],
            vec![10u128],
            vec![],
        );
        assert_eq!(result, Err(Error::Unauthorized));
    }

    #[ink::test]
    fn test_batch_transfer_size_exceeded() {
        let mut contract = setup_contract();
        let accounts = test::default_accounts::<DefaultEnvironment>();
        test::set_caller::<DefaultEnvironment>(accounts.alice);

        // max_batch_size is 50. Let's create 51 items.
        let ids: Vec<TokenId> = (1..=51).collect();
        let amounts: Vec<u128> = vec![1; 51];

        let result = contract.safe_batch_transfer_from(
            accounts.alice,
            accounts.bob,
            ids,
            amounts,
            vec![],
        );
        assert_eq!(result, Err(Error::BatchSizeExceeded));
    }

    #[ink::test]
    fn test_batch_transfer_size_exact_max() {
        let mut contract = setup_contract();
        let accounts = test::default_accounts::<DefaultEnvironment>();
        test::set_caller::<DefaultEnvironment>(accounts.alice);

        // max_batch_size is 50. Let's create exactly 50 items.
        let ids: Vec<TokenId> = (1..=50).collect();
        let amounts: Vec<u128> = vec![1; 50];

        // Seed balances
        for &id in &ids {
            contract.balances.insert((&accounts.alice, &id), &10u128);
        }

        let result = contract.safe_batch_transfer_from(
            accounts.alice,
            accounts.bob,
            ids,
            amounts,
            vec![],
        );
        // It might return other compliance or kyc errors, but not BatchSizeExceeded.
        assert_ne!(result, Err(Error::BatchSizeExceeded));
    }

    #[ink::test]
    fn test_batch_transfer_exact_balance() {
        let mut contract = setup_contract();
        let accounts = test::default_accounts::<DefaultEnvironment>();
        test::set_caller::<DefaultEnvironment>(accounts.alice);

        let token_id: TokenId = 1;
        contract.balances.insert((&accounts.alice, &token_id), &100u128);

        let result = contract.safe_batch_transfer_from(
            accounts.alice,
            accounts.bob,
            vec![token_id],
            vec![100u128],
            vec![],
        );
        assert!(result.is_ok());
        assert_eq!(contract.balances.get((&accounts.alice, &token_id)).unwrap_or(0), 0);
        assert_eq!(contract.balances.get((&accounts.bob, &token_id)).unwrap_or(0), 100);
    }

    #[ink::test]
    fn test_issue_shares_by_owner_not_admin() {
        let mut contract = setup_contract();
        let accounts = test::default_accounts::<DefaultEnvironment>();
        
        let metadata = PropertyMetadata {
            location: String::from("123 Main St"),
            size: 1000,
            legal_description: String::from("Sample property"),
            valuation: 500000,
            documents_url: String::from("ipfs://sample-docs"),
        };

        // Deployer (admin) registers property, making alice the owner of token 1
        test::set_caller::<DefaultEnvironment>(contract.admin());
        let token_id = contract.register_property_with_token(metadata).unwrap();
        contract.token_owner.insert(token_id, &accounts.alice);

        // Alice (owner, not admin) issues shares
        test::set_caller::<DefaultEnvironment>(accounts.alice);
        let result = contract.issue_shares(token_id, accounts.bob, 500);
        assert!(result.is_ok());
        assert_eq!(contract.share_balance_of(accounts.bob, token_id), 500);
    }

    #[ink::test]
    fn test_issue_shares_unauthorized() {
        let mut contract = setup_contract();
        let accounts = test::default_accounts::<DefaultEnvironment>();
        
        let metadata = PropertyMetadata {
            location: String::from("123 Main St"),
            size: 1000,
            legal_description: String::from("Sample property"),
            valuation: 500000,
            documents_url: String::from("ipfs://sample-docs"),
        };

        test::set_caller::<DefaultEnvironment>(contract.admin());
        let token_id = contract.register_property_with_token(metadata).unwrap();

        // Bob (not admin, not owner) tries to issue shares
        test::set_caller::<DefaultEnvironment>(accounts.bob);
        let result = contract.issue_shares(token_id, accounts.charlie, 500);
        assert_eq!(result, Err(Error::Unauthorized));
    }

    #[ink::test]
    fn test_redeem_shares_success() {
        let (mut contract, token_id) = setup_token_with_shares(1000);
        let accounts = test::default_accounts::<DefaultEnvironment>();
        
        test::set_caller::<DefaultEnvironment>(accounts.alice);
        let result = contract.redeem_shares(token_id, accounts.alice, 400);
        assert!(result.is_ok());
        assert_eq!(contract.share_balance_of(accounts.alice, token_id), 601);
    }

    #[ink::test]
    fn test_redeem_shares_approved_operator() {
        let (mut contract, token_id) = setup_token_with_shares(1000);
        let accounts = test::default_accounts::<DefaultEnvironment>();
        
        test::set_caller::<DefaultEnvironment>(accounts.alice);
        contract.set_approval_for_all(accounts.bob, true).unwrap();

        test::set_caller::<DefaultEnvironment>(accounts.bob);
        let result = contract.redeem_shares(token_id, accounts.alice, 400);
        assert!(result.is_ok());
        assert_eq!(contract.share_balance_of(accounts.alice, token_id), 601);
    }

    #[ink::test]
    fn test_redeem_shares_unauthorized() {
        let (mut contract, token_id) = setup_token_with_shares(1000);
        let accounts = test::default_accounts::<DefaultEnvironment>();
        
        test::set_caller::<DefaultEnvironment>(accounts.bob);
        let result = contract.redeem_shares(token_id, accounts.alice, 400);
        assert_eq!(result, Err(Error::Unauthorized));
    }

    #[ink::test]
    fn test_redeem_shares_zero_amount() {
        let (mut contract, token_id) = setup_token_with_shares(1000);
        let accounts = test::default_accounts::<DefaultEnvironment>();
        
        test::set_caller::<DefaultEnvironment>(accounts.alice);
        let result = contract.redeem_shares(token_id, accounts.alice, 0);
        assert_eq!(result, Err(Error::InvalidAmount));
    }

    #[ink::test]
    fn test_redeem_shares_insufficient_balance() {
        let (mut contract, token_id) = setup_token_with_shares(100);
        let accounts = test::default_accounts::<DefaultEnvironment>();
        
        test::set_caller::<DefaultEnvironment>(accounts.alice);
        let result = contract.redeem_shares(token_id, accounts.alice, 102); // 100 + 1 = 101, so 102 is insufficient
        assert_eq!(result, Err(Error::InsufficientBalance));
    }

    #[ink::test]
    fn test_redeem_shares_exact_balance() {
        let (mut contract, token_id) = setup_token_with_shares(100);
        let accounts = test::default_accounts::<DefaultEnvironment>();
        
        test::set_caller::<DefaultEnvironment>(accounts.alice);
        let result = contract.redeem_shares(token_id, accounts.alice, 101); // 100 + 1 = 101
        assert!(result.is_ok());
        assert_eq!(contract.share_balance_of(accounts.alice, token_id), 0);
    }

    #[ink::test]
    fn test_deposit_dividends_success() {
        let (mut contract, token_id) = setup_token_with_shares(1000);
        let accounts = test::default_accounts::<DefaultEnvironment>();
        
        test::set_caller::<DefaultEnvironment>(accounts.alice);
        test::set_value_transferred::<DefaultEnvironment>(10_000);
        let result = contract.deposit_dividends(token_id);
        assert!(result.is_ok());
        
        let dividends = contract.dividends_per_share.get(token_id).unwrap_or(0);
        assert!(dividends > 0);
    }

    #[ink::test]
    fn test_deposit_dividends_zero_amount() {
        let (mut contract, token_id) = setup_token_with_shares(1000);
        let accounts = test::default_accounts::<DefaultEnvironment>();
        
        test::set_caller::<DefaultEnvironment>(accounts.alice);
        test::set_value_transferred::<DefaultEnvironment>(0);
        let result = contract.deposit_dividends(token_id);
        assert_eq!(result, Err(Error::InvalidAmount));
    }

    #[ink::test]
    fn test_deposit_dividends_no_shares() {
        let mut contract = setup_contract();
        let metadata = PropertyMetadata {
            location: String::from("123 Main St"),
            size: 1000,
            legal_description: String::from("Sample property"),
            valuation: 500000,
            documents_url: String::from("ipfs://sample-docs"),
        };
        let token_id = contract.register_property_with_token(metadata).unwrap();
        
        test::set_caller::<DefaultEnvironment>(contract.admin());
        test::set_value_transferred::<DefaultEnvironment>(10_000);
        let result = contract.deposit_dividends(token_id);
        assert_eq!(result, Err(Error::InvalidRequest));
    }

    #[ink::test]
    fn test_distribute_rental_income_unauthorized() {
        let (mut contract, token_id) = setup_token_with_shares(1000);
        let accounts = test::default_accounts::<DefaultEnvironment>();
        
        test::set_caller::<DefaultEnvironment>(accounts.bob);
        test::set_value_transferred::<DefaultEnvironment>(10_000);
        let result = contract.distribute_rental_income(token_id);
        assert_eq!(result, Err(Error::Unauthorized));
    }

    #[ink::test]
    fn test_withdraw_dividends_exact_amount() {
        let (mut contract, token_id) = setup_token_with_shares(1000);
        let accounts = test::default_accounts::<DefaultEnvironment>();

        test::set_caller::<DefaultEnvironment>(contract.admin());
        test::set_value_transferred::<DefaultEnvironment>(10_000);
        contract.distribute_rental_income(token_id).unwrap();

        test::set_caller::<DefaultEnvironment>(accounts.alice);
        let withdrawn = contract.withdraw_dividends(token_id).unwrap();
        assert_eq!(withdrawn, 10_010);
    }

    #[ink::test]
    fn test_place_ask_success() {
        let (mut contract, token_id) = setup_token_with_shares(1000);
        let accounts = test::default_accounts::<DefaultEnvironment>();

        test::set_caller::<DefaultEnvironment>(accounts.alice);
        let result = contract.place_ask(token_id, 10, 400);
        assert!(result.is_ok());

        assert_eq!(contract.share_balance_of(accounts.alice, token_id), 601);
        assert_eq!(contract.escrowed_shares.get((token_id, accounts.alice)).unwrap_or(0), 400);
        
        let ask = contract.asks.get((token_id, accounts.alice)).unwrap();
        assert_eq!(ask.amount, 400);
        assert_eq!(ask.price_per_share, 10);
    }

    #[ink::test]
    fn test_place_ask_invalid_price() {
        let (mut contract, token_id) = setup_token_with_shares(1000);
        let accounts = test::default_accounts::<DefaultEnvironment>();

        test::set_caller::<DefaultEnvironment>(accounts.alice);
        let result = contract.place_ask(token_id, 0, 400);
        assert_eq!(result, Err(Error::InvalidAmount));
    }

    #[ink::test]
    fn test_place_ask_invalid_amount() {
        let (mut contract, token_id) = setup_token_with_shares(1000);
        let accounts = test::default_accounts::<DefaultEnvironment>();

        test::set_caller::<DefaultEnvironment>(accounts.alice);
        let result = contract.place_ask(token_id, 10, 0);
        assert_eq!(result, Err(Error::InvalidAmount));
    }

    #[ink::test]
    fn test_place_ask_insufficient_balance() {
        let (mut contract, token_id) = setup_token_with_shares(100);
        let accounts = test::default_accounts::<DefaultEnvironment>();

        test::set_caller::<DefaultEnvironment>(accounts.alice);
        let result = contract.place_ask(token_id, 10, 102); // 100 + 1 = 101
        assert_eq!(result, Err(Error::InsufficientBalance));
    }

    #[ink::test]
    fn test_place_ask_exact_balance() {
        let (mut contract, token_id) = setup_token_with_shares(100);
        let accounts = test::default_accounts::<DefaultEnvironment>();

        test::set_caller::<DefaultEnvironment>(accounts.alice);
        let result = contract.place_ask(token_id, 10, 101); // 100 + 1 = 101
        assert!(result.is_ok());
        assert_eq!(contract.share_balance_of(accounts.alice, token_id), 0);
        assert_eq!(contract.escrowed_shares.get((token_id, accounts.alice)).unwrap_or(0), 101);
    }
}

// =========================================================================
// Operator Management Tests (Issue #559)
// =========================================================================

#[cfg(test)]
mod operator_management_tests {
    use super::*;
    use ink::env::{test, DefaultEnvironment};

    fn mint_token(contract: &mut PropertyToken, owner: ink::primitives::AccountId) -> TokenId {
        test::set_caller::<DefaultEnvironment>(owner);
        let metadata = PropertyMetadata {
            location: String::from("100 Operator St"),
            size: 500,
            legal_description: String::from("Operator test property"),
            valuation: 200000,
            documents_url: String::from("ipfs://op-docs"),
        };
        contract.register_property_with_token(metadata).expect("mint failed")
    }

    #[ink::test]
    fn test_approve_by_owner_succeeds() {
        let mut contract = PropertyToken::new();
        let accounts = test::default_accounts::<DefaultEnvironment>();
        let token_id = mint_token(&mut contract, accounts.alice);

        test::set_caller::<DefaultEnvironment>(accounts.alice);
        let result = contract.approve(accounts.bob, token_id);
        assert!(result.is_ok(), "Owner should be able to approve a spender");

        let approved = contract.get_approved(token_id);
        assert_eq!(approved, Some(accounts.bob));
    }

    #[ink::test]
    fn test_approve_by_non_owner_fails() {
        let mut contract = PropertyToken::new();
        let accounts = test::default_accounts::<DefaultEnvironment>();
        let token_id = mint_token(&mut contract, accounts.alice);

        test::set_caller::<DefaultEnvironment>(accounts.charlie);
        let result = contract.approve(accounts.bob, token_id);
        assert_eq!(result, Err(Error::Unauthorized));
    }

    #[ink::test]
    fn test_approve_nonexistent_token_fails() {
        let mut contract = PropertyToken::new();
        let accounts = test::default_accounts::<DefaultEnvironment>();

        test::set_caller::<DefaultEnvironment>(accounts.alice);
        let result = contract.approve(accounts.bob, 9999);
        assert_eq!(result, Err(Error::TokenNotFound));
    }

    #[ink::test]
    fn test_approve_clears_previous_approval() {
        let mut contract = PropertyToken::new();
        let accounts = test::default_accounts::<DefaultEnvironment>();
        let token_id = mint_token(&mut contract, accounts.alice);

        test::set_caller::<DefaultEnvironment>(accounts.alice);
        contract.approve(accounts.bob, token_id).unwrap();
        assert_eq!(contract.get_approved(token_id), Some(accounts.bob));

        contract.approve(accounts.charlie, token_id).unwrap();
        assert_eq!(contract.get_approved(token_id), Some(accounts.charlie));
    }

    #[ink::test]
    fn test_approved_operator_can_approve_on_behalf_of_owner() {
        let mut contract = PropertyToken::new();
        let accounts = test::default_accounts::<DefaultEnvironment>();
        let token_id = mint_token(&mut contract, accounts.alice);

        test::set_caller::<DefaultEnvironment>(accounts.alice);
        contract.set_approval_for_all(accounts.bob, true).unwrap();

        test::set_caller::<DefaultEnvironment>(accounts.bob);
        let result = contract.approve(accounts.charlie, token_id);
        assert!(result.is_ok(), "Operator should be able to approve on behalf of owner");
        assert_eq!(contract.get_approved(token_id), Some(accounts.charlie));
    }

    #[ink::test]
    fn test_set_approval_for_all_grants_operator() {
        let mut contract = PropertyToken::new();
        let accounts = test::default_accounts::<DefaultEnvironment>();

        test::set_caller::<DefaultEnvironment>(accounts.alice);
        contract.set_approval_for_all(accounts.bob, true).unwrap();

        assert!(contract.is_approved_for_all(accounts.alice, accounts.bob));
    }

    #[ink::test]
    fn test_set_approval_for_all_revokes_operator() {
        let mut contract = PropertyToken::new();
        let accounts = test::default_accounts::<DefaultEnvironment>();

        test::set_caller::<DefaultEnvironment>(accounts.alice);
        contract.set_approval_for_all(accounts.bob, true).unwrap();
        assert!(contract.is_approved_for_all(accounts.alice, accounts.bob));

        contract.set_approval_for_all(accounts.bob, false).unwrap();
        assert!(!contract.is_approved_for_all(accounts.alice, accounts.bob));
    }

    #[ink::test]
    fn test_is_approved_for_all_returns_false_by_default() {
        let contract = PropertyToken::new();
        let accounts = test::default_accounts::<DefaultEnvironment>();

        assert!(!contract.is_approved_for_all(accounts.alice, accounts.bob));
        assert!(!contract.is_approved_for_all(accounts.bob, accounts.alice));
    }

    #[ink::test]
    fn test_get_approved_returns_none_for_no_approval() {
        let mut contract = PropertyToken::new();
        let accounts = test::default_accounts::<DefaultEnvironment>();
        let token_id = mint_token(&mut contract, accounts.alice);

        assert_eq!(contract.get_approved(token_id), None);
    }

    #[ink::test]
    fn test_multiple_operators_per_owner() {
        let mut contract = PropertyToken::new();
        let accounts = test::default_accounts::<DefaultEnvironment>();

        test::set_caller::<DefaultEnvironment>(accounts.alice);
        contract.set_approval_for_all(accounts.bob, true).unwrap();
        contract.set_approval_for_all(accounts.charlie, true).unwrap();

        assert!(contract.is_approved_for_all(accounts.alice, accounts.bob));
        assert!(contract.is_approved_for_all(accounts.alice, accounts.charlie));
    }

    #[ink::test]
    fn test_operator_approval_is_per_owner() {
        let mut contract = PropertyToken::new();
        let accounts = test::default_accounts::<DefaultEnvironment>();

        test::set_caller::<DefaultEnvironment>(accounts.alice);
        contract.set_approval_for_all(accounts.bob, true).unwrap();

        assert!(!contract.is_approved_for_all(accounts.charlie, accounts.bob));
    }

    #[ink::test]
    fn test_token_approval_does_not_grant_all_tokens() {
        let mut contract = PropertyToken::new();
        let accounts = test::default_accounts::<DefaultEnvironment>();
        let token_id_1 = mint_token(&mut contract, accounts.alice);
        let token_id_2 = mint_token(&mut contract, accounts.alice);

        test::set_caller::<DefaultEnvironment>(accounts.alice);
        contract.approve(accounts.bob, token_id_1).unwrap();

        assert_eq!(contract.get_approved(token_id_1), Some(accounts.bob));
        assert_eq!(contract.get_approved(token_id_2), None);
        assert!(!contract.is_approved_for_all(accounts.alice, accounts.bob));
    }
}