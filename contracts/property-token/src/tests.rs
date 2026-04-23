// Unit tests for the PropertyToken contract (Issue #101 - extracted from lib.rs)

#[cfg(test)]
mod tests {
    use super::*;
    use ink::env::{test, DefaultEnvironment};

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
}
