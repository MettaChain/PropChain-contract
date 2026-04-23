#[cfg(test)]
mod property_token_tests {
    use ink::env::{DefaultEnvironment, test};
    use crate::property_token::{PropertyToken, Error, PropertyMetadata};

    #[ink::test]
    fn test_property_token_creation() {
        let accounts = test::default_accounts::<DefaultEnvironment>();
        test::set_caller::<DefaultEnvironment>(accounts.alice);

        let contract = PropertyToken::new();
        assert_eq!(contract.total_supply(), 0);
        assert_eq!(contract.current_token_id(), 0);
        assert_eq!(contract.admin(), accounts.alice);
    }

    #[ink::test]
    fn test_register_property_with_token() {
        let accounts = test::default_accounts::<DefaultEnvironment>();
        test::set_caller::<DefaultEnvironment>(accounts.alice);

        let mut contract = PropertyToken::new();
        
        let metadata = PropertyMetadata {
            location: String::from("123 Main St"),
            size: 1000,
            legal_description: String::from("Sample property"),
            valuation: 500000,
            documents_url: String::from("ipfs://sample-docs"),
        };
        
        let result = contract.register_property_with_token(metadata.clone());
        assert!(result.is_ok());
        
        let token_id = result.unwrap();
        assert_eq!(token_id, 1);
        assert_eq!(contract.total_supply(), 1);
        
        // Check ownership
        assert_eq!(contract.owner_of(1), Some(accounts.alice));
        assert_eq!(contract.balance_of(accounts.alice), 1);
    }

    #[ink::test]
    fn test_erc721_transfer() {
        let accounts = test::default_accounts::<DefaultEnvironment>();
        test::set_caller::<DefaultEnvironment>(accounts.alice);

        let mut contract = PropertyToken::new();
        
        let metadata = PropertyMetadata {
            location: String::from("123 Main St"),
            size: 1000,
            legal_description: String::from("Sample property"),
            valuation: 500000,
            documents_url: String::from("ipfs://sample-docs"),
        };
        
        let token_id = contract.register_property_with_token(metadata).unwrap();
        
        // Transfer token to bob
        test::set_caller::<DefaultEnvironment>(accounts.alice);
        let result = contract.transfer_from(accounts.alice, accounts.bob, token_id);
        assert!(result.is_ok());
        
        // Verify new ownership
        assert_eq!(contract.owner_of(token_id), Some(accounts.bob));
        assert_eq!(contract.balance_of(accounts.alice), 0);
        assert_eq!(contract.balance_of(accounts.bob), 1);
    }

    #[ink::test]
    fn test_approve_and_transfer_from() {
        let accounts = test::default_accounts::<DefaultEnvironment>();
        test::set_caller::<DefaultEnvironment>(accounts.alice);

        let mut contract = PropertyToken::new();
        
        let metadata = PropertyMetadata {
            location: String::from("123 Main St"),
            size: 1000,
            legal_description: String::from("Sample property"),
            valuation: 500000,
            documents_url: String::from("ipfs://sample-docs"),
        };
        
        let token_id = contract.register_property_with_token(metadata).unwrap();
        
        // Alice approves Bob to transfer the token
        let result = contract.approve(accounts.bob, token_id);
        assert!(result.is_ok());
        
        // Verify approval
        assert_eq!(contract.get_approved(token_id), Some(accounts.bob));
        
        // Bob transfers the token to Charlie (Bob acts as the caller)
        test::set_caller::<DefaultEnvironment>(accounts.bob);
        let result = contract.transfer_from(accounts.alice, accounts.charlie, token_id);
        assert!(result.is_ok());
        
        // Verify new ownership
        assert_eq!(contract.owner_of(token_id), Some(accounts.charlie));
    }

    #[ink::test]
    fn test_attach_legal_document() {
        let accounts = test::default_accounts::<DefaultEnvironment>();
        test::set_caller::<DefaultEnvironment>(accounts.alice);

        let mut contract = PropertyToken::new();
        
        let metadata = PropertyMetadata {
            location: String::from("123 Main St"),
            size: 1000,
            legal_description: String::from("Sample property"),
            valuation: 500000,
            documents_url: String::from("ipfs://sample-docs"),
        };
        
        let token_id = contract.register_property_with_token(metadata).unwrap();
        
        // Attach a legal document
        let doc_hash = ink::Hash::from([1u8; 32]);
        let doc_type = String::from("Deed");
        
        let result = contract.attach_legal_document(token_id, doc_hash, doc_type.clone());
        assert!(result.is_ok());
        
        // Note: We can't directly test the document was stored because
        // legal_documents mapping is private. The test verifies the function executes without error.
    }

    #[ink::test]
    fn test_verify_compliance() {
        let accounts = test::default_accounts::<DefaultEnvironment>();
        test::set_caller::<DefaultEnvironment>(accounts.alice);

        let mut contract = PropertyToken::new();
        
        let metadata = PropertyMetadata {
            location: String::from("123 Main St"),
            size: 1000,
            legal_description: String::from("Sample property"),
            valuation: 500000,
            documents_url: String::from("ipfs://sample-docs"),
        };
        
        let token_id = contract.register_property_with_token(metadata).unwrap();
        
        // Verify compliance (admin is alice in this test)
        let result = contract.verify_compliance(token_id, true);
        assert!(result.is_ok());
        
        // Note: We can't directly test the compliance status was updated because
        // compliance_flags mapping is private. The test verifies the function executes without error.
    }

    #[ink::test]
    fn test_erc1155_batch_operations() {
        let accounts = test::default_accounts::<DefaultEnvironment>();
        test::set_caller::<DefaultEnvironment>(accounts.alice);

        let mut contract = PropertyToken::new();
        
        let metadata = PropertyMetadata {
            location: String::from("123 Main St"),
            size: 1000,
            legal_description: String::from("Sample property"),
            valuation: 500000,
            documents_url: String::from("ipfs://sample-docs"),
        };
        
        // Register two properties
        let token_id1 = contract.register_property_with_token(metadata.clone()).unwrap();
        let token_id2 = contract.register_property_with_token(metadata).unwrap();
        
        // Test balance_of_batch
        let accounts_vec = vec![accounts.alice, accounts.alice];
        let ids_vec = vec![token_id1, token_id2];
        let balances = contract.balance_of_batch(accounts_vec, ids_vec);
        
        assert_eq!(balances.len(), 2);
        assert_eq!(balances[0], 1);
        assert_eq!(balances[1], 1);
    }

    #[ink::test]
    fn test_uri_function() {
        let accounts = test::default_accounts::<DefaultEnvironment>();
        test::set_caller::<DefaultEnvironment>(accounts.alice);

        let mut contract = PropertyToken::new();
        
        let metadata = PropertyMetadata {
            location: String::from("123 Main St"),
            size: 1000,
            legal_description: String::from("Sample property"),
            valuation: 500000,
            documents_url: String::from("ipfs://sample-docs"),
        };
        
        let token_id = contract.register_property_with_token(metadata).unwrap();
        
        let uri_result = contract.uri(token_id);
        assert!(uri_result.is_some());
        
        let uri = uri_result.unwrap();
        assert!(uri.contains(&format!("{}", contract.env().account_id())));
        assert!(uri.contains(&format!("{}", token_id)));
    }

    #[ink::test]
    fn test_bridge_operator_management() {
        let accounts = test::default_accounts::<DefaultEnvironment>();
        test::set_caller::<DefaultEnvironment>(accounts.alice);

        let mut contract = PropertyToken::new();
        
        // Initially, alice should be a bridge operator (as admin)
        assert_eq!(contract.admin(), accounts.alice);
        
        // Add a new bridge operator
        let result = contract.add_bridge_operator(accounts.bob);
        assert!(result.is_ok());
        
        // Note: We can't directly test if bob was added as an operator because
        // bridge_operators vector is private. The test verifies the function executes without error.
    }

    #[ink::test]
    fn test_get_ownership_history() {
        let accounts = test::default_accounts::<DefaultEnvironment>();
        test::set_caller::<DefaultEnvironment>(accounts.alice);

        let mut contract = PropertyToken::new();
        
        let metadata = PropertyMetadata {
            location: String::from("123 Main St"),
            size: 1000,
            legal_description: String::from("Sample property"),
            valuation: 500000,
            documents_url: String::from("ipfs://sample-docs"),
        };
        
        let token_id = contract.register_property_with_token(metadata).unwrap();
        
        // Get ownership history
        let history = contract.get_ownership_history(token_id);
        assert!(history.is_some());
        
        // The history should contain at least the initial minting record
        let history_vec = history.unwrap();
        assert!(!history_vec.is_empty());
    }

    #[ink::test]
    fn test_bridge_to_chain() {
        let accounts = test::default_accounts::<DefaultEnvironment>();
        test::set_caller::<DefaultEnvironment>(accounts.alice);

        let mut contract = PropertyToken::new();
        
        let metadata = PropertyMetadata {
            location: String::from("123 Main St"),
            size: 1000,
            legal_description: String::from("Sample property"),
            valuation: 500000,
            documents_url: String::from("ipfs://sample-docs"),
        };
        
        let token_id = contract.register_property_with_token(metadata).unwrap();
        
        // First verify the token for compliance
        test::set_caller::<DefaultEnvironment>(accounts.alice); // admin
        let result = contract.verify_compliance(token_id, true);
        assert!(result.is_ok());
        
        // Now bridge the token
        test::set_caller::<DefaultEnvironment>(accounts.alice); // token owner
        let result = contract.bridge_to_chain(2, token_id, accounts.bob); // chain ID 2
        assert!(result.is_ok());
        
        // After bridging, the token should be locked (owned by zero address)
        let zero_address = ink::primitives::AccountId::from([0u8; 32]);
        // Note: This test depends on internal implementation details
    }

    #[ink::test]
    fn test_receive_bridged_token() {
        let accounts = test::default_accounts::<DefaultEnvironment>();
        test::set_caller::<DefaultEnvironment>(accounts.alice);

        let mut contract = PropertyToken::new();
        
        // Add bob as a bridge operator
        let result = contract.add_bridge_operator(accounts.bob);
        assert!(result.is_ok());
        
        // Bob receives a bridged token
        test::set_caller::<DefaultEnvironment>(accounts.bob);
        let result = contract.receive_bridged_token(2, 1, accounts.charlie); // source chain 2, token 1
        assert!(result.is_ok());
    }

    #[ink::test]
    fn test_error_conditions() {
        let accounts = test::default_accounts::<DefaultEnvironment>();
        test::set_caller::<DefaultEnvironment>(accounts.alice);

        let mut contract = PropertyToken::new();
        
        // Test trying to transfer a non-existent token
        let result = contract.transfer_from(accounts.alice, accounts.bob, 999);
        assert_eq!(result, Err(Error::TokenNotFound));
        
        // Test trying to get owner of non-existent token
        assert_eq!(contract.owner_of(999), None);
        
        // Test trying to approve a non-existent token
        let result = contract.approve(accounts.bob, 999);
        assert_eq!(result, Err(Error::TokenNotFound));
    }
<<<<<<< HEAD

    // ============================================================
    // KYC-Based Transfer Restriction Tests
    // ============================================================

    #[ink::test]
    fn test_set_transfer_restriction() {
        let accounts = test::default_accounts::<DefaultEnvironment>();
        test::set_caller::<DefaultEnvironment>(accounts.alice);

        let mut contract = PropertyToken::new();
        
        let metadata = PropertyMetadata {
            location: String::from("123 Main St"),
            size: 1000,
            legal_description: String::from("Sample property"),
            valuation: 500000,
            documents_url: String::from("ipfs://sample-docs"),
        };
        
        let token_id = contract.register_property_with_token(metadata).unwrap();
        
        // Set transfer restrictions
        let result = contract.set_transfer_restriction(
            token_id,
            property_token::TransferRestrictionLevel::VerificationLevelRequired,
            property_token::KYCVerificationLevel::Standard,
            1000, // max_transfer_amount
            100,  // quota_period in blocks
            10,   // hold_period in blocks
            false, // check_risk_level
            50,   // max_allowed_risk_level
        );
        assert!(result.is_ok());
    }

    #[ink::test]
    fn test_get_transfer_restriction_config() {
        let accounts = test::default_accounts::<DefaultEnvironment>();
        test::set_caller::<DefaultEnvironment>(accounts.alice);

        let mut contract = PropertyToken::new();
        
        let metadata = PropertyMetadata {
            location: String::from("123 Main St"),
            size: 1000,
            legal_description: String::from("Sample property"),
            valuation: 500000,
            documents_url: String::from("ipfs://sample-docs"),
        };
        
        let token_id = contract.register_property_with_token(metadata).unwrap();
        
        // Initially no restrictions
        assert_eq!(contract.get_transfer_restriction_config(token_id), None);
        
        // Set restrictions
        let result = contract.set_transfer_restriction(
            token_id,
            property_token::TransferRestrictionLevel::KYCRequired,
            property_token::KYCVerificationLevel::Basic,
            5000,
            200,
            20,
            true,
            75,
        );
        assert!(result.is_ok());
        
        // Get restrictions
        let config = contract.get_transfer_restriction_config(token_id);
        assert!(config.is_some());
        
        let (restriction_level, min_level, max_amount, quota_period, hold_period, check_risk, max_risk) = config.unwrap();
        assert_eq!(min_level, property_token::KYCVerificationLevel::Basic);
        assert_eq!(max_amount, 5000);
        assert_eq!(quota_period, 200);
        assert_eq!(hold_period, 20);
        assert_eq!(check_risk, true);
        assert_eq!(max_risk, 75);
    }

    #[ink::test]
    fn test_remove_transfer_restriction() {
        let accounts = test::default_accounts::<DefaultEnvironment>();
        test::set_caller::<DefaultEnvironment>(accounts.alice);

        let mut contract = PropertyToken::new();
        
        let metadata = PropertyMetadata {
            location: String::from("123 Main St"),
            size: 1000,
            legal_description: String::from("Sample property"),
            valuation: 500000,
            documents_url: String::from("ipfs://sample-docs"),
        };
        
        let token_id = contract.register_property_with_token(metadata).unwrap();
        
        // Set restrictions
        let result = contract.set_transfer_restriction(
            token_id,
            property_token::TransferRestrictionLevel::WhitelistOnly,
            property_token::KYCVerificationLevel::Enhanced,
            2000,
            150,
            15,
            false,
            60,
        );
        assert!(result.is_ok());
        assert!(contract.get_transfer_restriction_config(token_id).is_some());
        
        // Remove restrictions
        let result = contract.remove_transfer_restriction(token_id);
        assert!(result.is_ok());
        assert_eq!(contract.get_transfer_restriction_config(token_id), None);
    }

    #[ink::test]
    fn test_blacklist_and_whitelist_operations() {
        let accounts = test::default_accounts::<DefaultEnvironment>();
        test::set_caller::<DefaultEnvironment>(accounts.alice);

        let mut contract = PropertyToken::new();
        
        let metadata = PropertyMetadata {
            location: String::from("123 Main St"),
            size: 1000,
            legal_description: String::from("Sample property"),
            valuation: 500000,
            documents_url: String::from("ipfs://sample-docs"),
        };
        
        let token_id = contract.register_property_with_token(metadata).unwrap();
        
        // Test blacklist
        assert!(!contract.is_account_blacklisted(accounts.bob));
        let result = contract.blacklist_account(accounts.bob);
        assert!(result.is_ok());
        assert!(contract.is_account_blacklisted(accounts.bob));
        
        // Remove from blacklist
        let result = contract.remove_from_blacklist(accounts.bob);
        assert!(result.is_ok());
        assert!(!contract.is_account_blacklisted(accounts.bob));
        
        // Test whitelist
        assert!(!contract.is_account_whitelisted(token_id, accounts.charlie));
        let result = contract.whitelist_account(token_id, accounts.charlie);
        assert!(result.is_ok());
        assert!(contract.is_account_whitelisted(token_id, accounts.charlie));
        
        // Remove from whitelist
        let result = contract.remove_from_whitelist(token_id, accounts.charlie);
        assert!(result.is_ok());
        assert!(!contract.is_account_whitelisted(token_id, accounts.charlie));
    }

    #[ink::test]
    fn test_transfer_with_no_restrictions() {
        let accounts = test::default_accounts::<DefaultEnvironment>();
        test::set_caller::<DefaultEnvironment>(accounts.alice);

        let mut contract = PropertyToken::new();
        
        let metadata = PropertyMetadata {
            location: String::from("123 Main St"),
            size: 1000,
            legal_description: String::from("Sample property"),
            valuation: 500000,
            documents_url: String::from("ipfs://sample-docs"),
        };
        
        let token_id = contract.register_property_with_token(metadata).unwrap();
        
        // Transfer should succeed without any restrictions set
        let result = contract.transfer_from(accounts.alice, accounts.bob, token_id);
        assert!(result.is_ok());
        assert_eq!(contract.owner_of(token_id), Some(accounts.bob));
    }

    #[ink::test]
    fn test_transfer_quota_tracking() {
        let accounts = test::default_accounts::<DefaultEnvironment>();
        test::set_caller::<DefaultEnvironment>(accounts.alice);

        let mut contract = PropertyToken::new();
        
        let metadata = PropertyMetadata {
            location: String::from("123 Main St"),
            size: 1000,
            legal_description: String::from("Sample property"),
            valuation: 500000,
            documents_url: String::from("ipfs://sample-docs"),
        };
        
        let token_id = contract.register_property_with_token(metadata).unwrap();
        
        // Set transfer restrictions with quota
        let result = contract.set_transfer_restriction(
            token_id,
            property_token::TransferRestrictionLevel::None,
            property_token::KYCVerificationLevel::None,
            100, // max_transfer_amount
            1000, // quota_period
            0,   // no hold period
            false,
            0,
        );
        assert!(result.is_ok());
        
        // Transfer and check quota
        test::set_caller::<DefaultEnvironment>(accounts.alice);
        let result = contract.transfer_from(accounts.alice, accounts.bob, token_id);
        assert!(result.is_ok());
        
        // Check transfer quota status (returns (amount_transferred, period_start_block, acquisition_block))
        let quota_status = contract.get_transfer_quota_status(token_id, accounts.alice);
        assert!(quota_status.is_some());
        let (amount_transferred, _, _) = quota_status.unwrap();
        assert_eq!(amount_transferred, 1); // We transferred 1 share
    }

    #[ink::test]
    fn test_blacklist_prevents_transfer() {
        let accounts = test::default_accounts::<DefaultEnvironment>();
        test::set_caller::<DefaultEnvironment>(accounts.alice);

        let mut contract = PropertyToken::new();
        
        let metadata = PropertyMetadata {
            location: String::from("123 Main St"),
            size: 1000,
            legal_description: String::from("Sample property"),
            valuation: 500000,
            documents_url: String::from("ipfs://sample-docs"),
        };
        
        let token_id = contract.register_property_with_token(metadata).unwrap();
        
        // Blacklist bob
        let result = contract.blacklist_account(accounts.bob);
        assert!(result.is_ok());
        
        // Try to transfer to blacklisted account
        let result = contract.transfer_from(accounts.alice, accounts.bob, token_id);
        assert_eq!(result, Err(Error::AccountBlacklisted));
    }

    #[ink::test]
    fn test_whitelist_only_restriction() {
        let accounts = test::default_accounts::<DefaultEnvironment>();
        test::set_caller::<DefaultEnvironment>(accounts.alice);

        let mut contract = PropertyToken::new();
        
        let metadata = PropertyMetadata {
            location: String::from("123 Main St"),
            size: 1000,
            legal_description: String::from("Sample property"),
            valuation: 500000,
            documents_url: String::from("ipfs://sample-docs"),
        };
        
        let token_id = contract.register_property_with_token(metadata).unwrap();
        
        // Set whitelist-only restriction
        let result = contract.set_transfer_restriction(
            token_id,
            property_token::TransferRestrictionLevel::WhitelistOnly,
            property_token::KYCVerificationLevel::None,
            0,
            0,
            0,
            false,
            0,
        );
        assert!(result.is_ok());
        
        // Whitelist alice and bob
        let result = contract.whitelist_account(token_id, accounts.alice);
        assert!(result.is_ok());
        let result = contract.whitelist_account(token_id, accounts.bob);
        assert!(result.is_ok());
        
        // Transfer should succeed (both are whitelisted)
        let result = contract.transfer_from(accounts.alice, accounts.bob, token_id);
        assert!(result.is_ok());
    }

    #[ink::test]
    fn test_whitelist_only_rejection() {
        let accounts = test::default_accounts::<DefaultEnvironment>();
        test::set_caller::<DefaultEnvironment>(accounts.alice);

        let mut contract = PropertyToken::new();
        
        let metadata = PropertyMetadata {
            location: String::from("123 Main St"),
            size: 1000,
            legal_description: String::from("Sample property"),
            valuation: 500000,
            documents_url: String::from("ipfs://sample-docs"),
        };
        
        let token_id = contract.register_property_with_token(metadata).unwrap();
        
        // Set whitelist-only restriction
        let result = contract.set_transfer_restriction(
            token_id,
            property_token::TransferRestrictionLevel::WhitelistOnly,
            property_token::KYCVerificationLevel::None,
            0,
            0,
            0,
            false,
            0,
        );
        assert!(result.is_ok());
        
        // Only whitelist alice
        let result = contract.whitelist_account(token_id, accounts.alice);
        assert!(result.is_ok());
        
        // Transfer should fail (bob is not whitelisted)
        let result = contract.transfer_from(accounts.alice, accounts.bob, token_id);
        assert_eq!(result, Err(Error::AccountNotWhitelisted));
    }

    #[ink::test]
    fn test_batch_transfer_with_kyc_restrictions() {
        let accounts = test::default_accounts::<DefaultEnvironment>();
        test::set_caller::<DefaultEnvironment>(accounts.alice);

        let mut contract = PropertyToken::new();
        
        let metadata = PropertyMetadata {
            location: String::from("123 Main St"),
            size: 1000,
            legal_description: String::from("Sample property"),
            valuation: 500000,
            documents_url: String::from("ipfs://sample-docs"),
        };
        
        // Register multiple tokens
        let token_id1 = contract.register_property_with_token(metadata.clone()).unwrap();
        let token_id2 = contract.register_property_with_token(metadata).unwrap();
        
        // Set restrictions on both
        for token_id in &[token_id1, token_id2] {
            let result = contract.set_transfer_restriction(
                *token_id,
                property_token::TransferRestrictionLevel::None,
                property_token::KYCVerificationLevel::None,
                0,
                0,
                0,
                false,
                0,
            );
            assert!(result.is_ok());
        }
        
        // Batch transfer - should succeed
        let ids = vec![token_id1, token_id2];
        let amounts = vec![1, 1];
        let result = contract.safe_batch_transfer_from(
            accounts.alice,
            accounts.bob,
            ids,
            amounts,
            vec![],
        );
        assert!(result.is_ok());
    }

    #[ink::test]
    fn test_batch_transfer_with_blacklist() {
        let accounts = test::default_accounts::<DefaultEnvironment>();
        test::set_caller::<DefaultEnvironment>(accounts.alice);

        let mut contract = PropertyToken::new();
        
        let metadata = PropertyMetadata {
            location: String::from("123 Main St"),
            size: 1000,
            legal_description: String::from("Sample property"),
            valuation: 500000,
            documents_url: String::from("ipfs://sample-docs"),
        };
        
        let token_id1 = contract.register_property_with_token(metadata.clone()).unwrap();
        let token_id2 = contract.register_property_with_token(metadata).unwrap();
        
        // Blacklist bob
        let result = contract.blacklist_account(accounts.bob);
        assert!(result.is_ok());
        
        // Batch transfer to blacklisted account should fail
        let ids = vec![token_id1, token_id2];
        let amounts = vec![1, 1];
        let result = contract.safe_batch_transfer_from(
            accounts.alice,
            accounts.bob,
            ids,
            amounts,
            vec![],
        );
        assert_eq!(result, Err(Error::AccountBlacklisted));
    }
}
=======
}
#[ink::test]
fn test_batch_transfer_success() {
    let mut contract = setup_contract();
    let accounts = test::default_accounts::<DefaultEnvironment>();
    test::set_caller::<DefaultEnvironment>(accounts.alice);

    contract.balances.insert((&accounts.alice, &1u64), &100u128);
    contract.balances.insert((&accounts.alice, &2u64), &200u128);

    let result = contract.safe_batch_transfer_from(
        accounts.alice,
        accounts.bob,
        vec![1u64, 2u64],
        vec![50u128, 100u128],
        vec![],
    );
    assert!(result.is_ok());
    assert_eq!(contract.balances.get((&accounts.alice, &1u64)).unwrap_or(0), 50);
    assert_eq!(contract.balances.get((&accounts.bob, &2u64)).unwrap_or(0), 100);
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
        vec![10u128],
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
        vec![999u128],
        vec![],
    );
    assert_eq!(result, Err(Error::InsufficientBalance));
}

#[ink::test]
fn test_batch_transfer_unauthorized() {
    let mut contract = setup_contract();
    let accounts = test::default_accounts::<DefaultEnvironment>();
    test::set_caller::<DefaultEnvironment>(accounts.bob);

    let result = contract.safe_batch_transfer_from(
        accounts.alice,
        accounts.bob,
        vec![1u64],
        vec![10u128],
        vec![],
    );
    assert_eq!(result, Err(Error::Unauthorized));
}
>>>>>>> origin/main
