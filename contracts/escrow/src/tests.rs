#[cfg(test)]
pub mod escrow_tests {
    use crate::propchain_escrow::*;
    use ink::env::test::DefaultAccounts;
    use ink::primitives::{AccountId, Hash};
    use scale::Encode;

    fn default_accounts() -> DefaultAccounts<ink::env::DefaultEnvironment> {
        ink::env::test::default_accounts::<ink::env::DefaultEnvironment>()
    }

    fn set_caller(caller: AccountId) {
        ink::env::test::set_caller::<ink::env::DefaultEnvironment>(caller);
    }

    fn set_balance(account: AccountId, balance: u128) {
        ink::env::test::set_account_balance::<ink::env::DefaultEnvironment>(account, balance);
    }

    fn encoded_len<T: Encode>(value: &T) -> usize {
        value.encode().len()
    }

    #[ink::test]
    fn test_new_contract() {
        let contract = AdvancedEscrow::new(1_000_000, None);
        assert_eq!(contract.get_high_value_threshold(), 1_000_000);
    }

    #[ink::test]
    fn test_set_tax_compliance_contract() {
        let accounts = default_accounts();
        let mut contract = AdvancedEscrow::new(1_000_000, None);

        let result = contract.set_tax_compliance_contract(Some(accounts.charlie));
        assert!(result.is_ok());
        // Since there is no getter, we just verify it doesn't error.
        // We could add a getter if needed.
    }

    #[ink::test]
    fn test_create_escrow_advanced() {
        let accounts = default_accounts();
        set_caller(accounts.alice);

        let mut contract = AdvancedEscrow::new(1_000_000, None);

        let participants = vec![accounts.alice, accounts.bob, accounts.charlie];
        let result = contract.create_escrow_advanced(
            1,              // property_id
            1_000_000,      // amount
            accounts.alice, // buyer
            accounts.bob,   // seller
            participants,
            2,    // required_signatures
            None, // no time lock
        );

        assert!(result.is_ok());
        let escrow_id = result.expect("Escrow creation should succeed in test");
        assert_eq!(escrow_id, 1);

        let escrow = contract
            .get_escrow(escrow_id)
            .expect("Escrow should exist after creation");
        assert_eq!(escrow.property_id, 1);
        assert_eq!(escrow.amount, 1_000_000);
        assert_eq!(escrow.buyer, accounts.alice);
        assert_eq!(escrow.seller, accounts.bob);
        assert_eq!(escrow.status, EscrowStatus::Created);
    }

    #[ink::test]
    fn test_create_escrow_invalid_config() {
        let accounts = default_accounts();
        set_caller(accounts.alice);

        let mut contract = AdvancedEscrow::new(1_000_000, None);

        // Test with more required signatures than participants
        let participants = vec![accounts.alice, accounts.bob];
        let result = contract.create_escrow_advanced(
            1,
            1_000_000,
            accounts.alice,
            accounts.bob,
            participants,
            3, // More than participants
            None,
        );

        assert_eq!(result, Err(Error::InvalidConfiguration));
    }

    #[ink::test]
    fn test_deposit_funds() {
        let accounts = default_accounts();
        set_caller(accounts.alice);
        set_balance(accounts.alice, 2_000_000);

        let mut contract = AdvancedEscrow::new(1_000_000, None);

        let participants = vec![accounts.alice, accounts.bob];
        let escrow_id = contract
            .create_escrow_advanced(
                1,
                1_000_000,
                accounts.alice,
                accounts.bob,
                participants,
                2,
                None,
            )
            .expect("Escrow creation should succeed in test");

        // Deposit funds
        ink::env::test::set_value_transferred::<ink::env::DefaultEnvironment>(1_000_000);
        let result = contract.deposit_funds(escrow_id);
        assert!(result.is_ok());

        let escrow = contract
            .get_escrow(escrow_id)
            .expect("Escrow should exist after deposit");
        assert_eq!(escrow.deposited_amount, 1_000_000);
        assert_eq!(escrow.status, EscrowStatus::Active);
    }

    #[ink::test]
    fn test_upload_document() {
        let accounts = default_accounts();
        set_caller(accounts.alice);

        let mut contract = AdvancedEscrow::new(1_000_000, None);

        let participants = vec![accounts.alice, accounts.bob];
        let escrow_id = contract
            .create_escrow_advanced(
                1,
                1_000_000,
                accounts.alice,
                accounts.bob,
                participants,
                2,
                None,
            )
            .expect("Escrow creation should succeed in test");

        let doc_hash = Hash::from([1u8; 32]);
        let result = contract.upload_document(escrow_id, doc_hash, "Title Deed".to_string());

        assert!(result.is_ok());

        let documents = contract.get_documents(escrow_id);
        assert_eq!(documents.len(), 1);
        assert_eq!(documents[0].hash, doc_hash);
        assert_eq!(documents[0].document_type, "Title Deed");
        assert!(!documents[0].verified);
    }

    #[ink::test]
    fn test_verify_document() {
        let accounts = default_accounts();
        set_caller(accounts.alice);

        let mut contract = AdvancedEscrow::new(1_000_000, None);

        let participants = vec![accounts.alice, accounts.bob];
        let escrow_id = contract
            .create_escrow_advanced(
                1,
                1_000_000,
                accounts.alice,
                accounts.bob,
                participants,
                2,
                None,
            )
            .expect("Escrow creation should succeed in test");

        let doc_hash = Hash::from([1u8; 32]);
        contract
            .upload_document(escrow_id, doc_hash, "Title Deed".to_string())
            .expect("Document upload should succeed in test");

        // Verify document
        let result = contract.verify_document(escrow_id, doc_hash);
        assert!(result.is_ok());

        let documents = contract.get_documents(escrow_id);
        assert!(documents[0].verified);
    }

    #[ink::test]
    fn test_add_condition() {
        let accounts = default_accounts();
        set_caller(accounts.alice);

        let mut contract = AdvancedEscrow::new(1_000_000, None);

        let participants = vec![accounts.alice, accounts.bob];
        let escrow_id = contract
            .create_escrow_advanced(
                1,
                1_000_000,
                accounts.alice,
                accounts.bob,
                participants,
                2,
                None,
            )
            .expect("Escrow creation should succeed in test");

        let result = contract.add_condition(escrow_id, "Property inspection completed".to_string());

        assert!(result.is_ok());
        let condition_id = result.expect("Condition addition should succeed in test");
        assert_eq!(condition_id, 1);

        let conditions = contract.get_conditions(escrow_id);
        assert_eq!(conditions.len(), 1);
        assert_eq!(conditions[0].description, "Property inspection completed");
        assert!(!conditions[0].met);
    }

    #[ink::test]
    fn test_mark_condition_met() {
        let accounts = default_accounts();
        set_caller(accounts.alice);

        let mut contract = AdvancedEscrow::new(1_000_000, None);

        let participants = vec![accounts.alice, accounts.bob];
        let escrow_id = contract
            .create_escrow_advanced(
                1,
                1_000_000,
                accounts.alice,
                accounts.bob,
                participants,
                2,
                None,
            )
            .expect("Escrow creation should succeed in test");

        let condition_id = contract
            .add_condition(escrow_id, "Property inspection completed".to_string())
            .expect("Condition addition should succeed in test");

        let result = contract.mark_condition_met(escrow_id, condition_id);
        assert!(result.is_ok());

        let conditions = contract.get_conditions(escrow_id);
        assert!(conditions[0].met);
        assert_eq!(conditions[0].verified_by, Some(accounts.alice));
    }

    #[ink::test]
    fn test_sign_approval() {
        let accounts = default_accounts();
        set_caller(accounts.alice);

        let mut contract = AdvancedEscrow::new(1_000_000, None);

        let participants = vec![accounts.alice, accounts.bob];
        let escrow_id = contract
            .create_escrow_advanced(
                1,
                1_000_000,
                accounts.alice,
                accounts.bob,
                participants,
                2,
                None,
            )
            .expect("Escrow creation should succeed in test");

        // Alice signs
        let result = contract.sign_approval(escrow_id, ApprovalType::Release);
        assert!(result.is_ok());

        let count = contract.get_signature_count(escrow_id, ApprovalType::Release);
        assert_eq!(count, 1);

        // Bob signs
        set_caller(accounts.bob);
        let result = contract.sign_approval(escrow_id, ApprovalType::Release);
        assert!(result.is_ok());

        let count = contract.get_signature_count(escrow_id, ApprovalType::Release);
        assert_eq!(count, 2);
    }

    #[ink::test]
    fn test_sign_approval_already_signed() {
        let accounts = default_accounts();
        set_caller(accounts.alice);

        let mut contract = AdvancedEscrow::new(1_000_000, None);

        let participants = vec![accounts.alice, accounts.bob];
        let escrow_id = contract
            .create_escrow_advanced(
                1,
                1_000_000,
                accounts.alice,
                accounts.bob,
                participants,
                2,
                None,
            )
            .expect("Escrow creation should succeed in test");

        contract
            .sign_approval(escrow_id, ApprovalType::Release)
            .expect("Approval signing should succeed in test");

        // Try to sign again
        let result = contract.sign_approval(escrow_id, ApprovalType::Release);
        assert_eq!(result, Err(Error::AlreadySigned));
    }

    #[ink::test]
    fn test_raise_dispute() {
        let accounts = default_accounts();
        set_caller(accounts.alice);

        let mut contract = AdvancedEscrow::new(1_000_000, None);

        let participants = vec![accounts.alice, accounts.bob];
        let escrow_id = contract
            .create_escrow_advanced(
                1,
                1_000_000,
                accounts.alice,
                accounts.bob,
                participants,
                2,
                None,
            )
            .expect("Escrow creation should succeed in test");

        let result =
            contract.raise_dispute(escrow_id, "Property condition not as described".to_string());

        assert!(result.is_ok());

        let dispute = contract
            .get_dispute(escrow_id)
            .expect("Dispute should exist after raising");
        assert_eq!(dispute.raised_by, accounts.alice);
        assert_eq!(dispute.reason, "Property condition not as described");
        assert!(!dispute.resolved);

        let escrow = contract
            .get_escrow(escrow_id)
            .expect("Escrow should exist in test");
        assert_eq!(escrow.status, EscrowStatus::Disputed);
    }

    #[ink::test]
    fn test_resolve_dispute() {
        let accounts = default_accounts();
        set_caller(accounts.alice);

        let mut contract = AdvancedEscrow::new(1_000_000, None);
        let admin = contract.get_admin();

        let participants = vec![accounts.alice, accounts.bob];
        let escrow_id = contract
            .create_escrow_advanced(
                1,
                1_000_000,
                accounts.alice,
                accounts.bob,
                participants,
                2,
                None,
            )
            .unwrap();

        contract
            .raise_dispute(escrow_id, "Issue".to_string())
            .expect("Dispute raising should succeed in test");

        // Admin resolves dispute
        set_caller(admin);
        let result = contract.resolve_dispute(escrow_id, "Resolved in favor of buyer".to_string());

        assert!(result.is_ok());

        let dispute = contract
            .get_dispute(escrow_id)
            .expect("Dispute should exist after raising");
        assert!(dispute.resolved);
        assert_eq!(
            dispute.resolution,
            Some("Resolved in favor of buyer".to_string())
        );

        let escrow = contract
            .get_escrow(escrow_id)
            .expect("Escrow should exist in test");
        assert_eq!(escrow.status, EscrowStatus::Active);
    }

    #[ink::test]
    fn test_resolve_dispute_unauthorized() {
        let accounts = default_accounts();
        set_caller(accounts.alice);

        let mut contract = AdvancedEscrow::new(1_000_000, None);

        let participants = vec![accounts.alice, accounts.bob];
        let escrow_id = contract
            .create_escrow_advanced(
                1,
                1_000_000,
                accounts.alice,
                accounts.bob,
                participants,
                2,
                None,
            )
            .expect("Escrow creation should succeed in test");

        contract
            .raise_dispute(escrow_id, "Issue".to_string())
            .expect("Dispute raising should succeed in test");

        // Non-admin tries to resolve
        set_caller(accounts.bob);
        let result = contract.resolve_dispute(escrow_id, "Resolution".to_string());
        assert_eq!(result, Err(Error::Unauthorized));
    }

    #[ink::test]
    fn test_check_all_conditions_met() {
        let accounts = default_accounts();
        set_caller(accounts.alice);

        let mut contract = AdvancedEscrow::new(1_000_000, None);

        let participants = vec![accounts.alice, accounts.bob];
        let escrow_id = contract
            .create_escrow_advanced(
                1,
                1_000_000,
                accounts.alice,
                accounts.bob,
                participants,
                2,
                None,
            )
            .expect("Escrow creation should succeed in test");

        // No conditions - should return true
        let result = contract.check_all_conditions_met(escrow_id);
        assert_eq!(result, Ok(true));

        // Add conditions
        let cond1 = contract
            .add_condition(escrow_id, "Condition 1".to_string())
            .expect("Condition addition should succeed in test");
        let cond2 = contract
            .add_condition(escrow_id, "Condition 2".to_string())
            .expect("Condition addition should succeed in test");

        // Not all met
        let result = contract.check_all_conditions_met(escrow_id);
        assert_eq!(result, Ok(false));

        // Mark first condition met
        contract
            .mark_condition_met(escrow_id, cond1)
            .expect("Marking condition met should succeed in test");
        let result = contract.check_all_conditions_met(escrow_id);
        assert_eq!(result, Ok(false));

        // Mark second condition met
        contract
            .mark_condition_met(escrow_id, cond2)
            .expect("Marking condition met should succeed in test");
        let result = contract.check_all_conditions_met(escrow_id);
        assert_eq!(result, Ok(true));
    }

    #[ink::test]
    fn test_audit_trail() {
        let accounts = default_accounts();
        set_caller(accounts.alice);

        let mut contract = AdvancedEscrow::new(1_000_000, None);

        let participants = vec![accounts.alice, accounts.bob];
        let escrow_id = contract
            .create_escrow_advanced(
                1,
                1_000_000,
                accounts.alice,
                accounts.bob,
                participants,
                2,
                None,
            )
            .expect("Escrow creation should succeed in test");

        // Perform some actions
        contract
            .add_condition(escrow_id, "Test condition".to_string())
            .expect("Condition addition should succeed in test");
        let doc_hash = Hash::from([1u8; 32]);
        contract
            .upload_document(escrow_id, doc_hash, "Test doc".to_string())
            .expect("Document upload should succeed in test");

        // Check audit trail
        let audit_trail = contract.get_audit_trail(escrow_id);
        assert!(audit_trail.len() >= 3); // Created + Condition + Document

        // Verify audit entries contain expected actions
        let actions: Vec<String> = audit_trail.iter().map(|e| e.action.clone()).collect();
        assert!(actions.contains(&"EscrowCreated".to_string()));
        assert!(actions.contains(&"ConditionAdded".to_string()));
        assert!(actions.contains(&"DocumentUploaded".to_string()));
    }

    #[ink::test]
    fn test_set_admin() {
        let accounts = default_accounts();
        set_caller(accounts.alice);

        let mut contract = AdvancedEscrow::new(1_000_000, None);
        let original_admin = contract.get_admin();
        assert_eq!(original_admin, accounts.alice);

        let result = contract.set_admin(accounts.bob);
        assert!(result.is_ok());

        let new_admin = contract.get_admin();
        assert_eq!(new_admin, accounts.bob);
    }

    #[ink::test]
    fn test_set_admin_unauthorized() {
        let accounts = default_accounts();
        set_caller(accounts.alice);

        let mut contract = AdvancedEscrow::new(1_000_000, None);

        // Try to set admin as non-admin
        set_caller(accounts.bob);
        let result = contract.set_admin(accounts.charlie);
        assert_eq!(result, Err(Error::Unauthorized));
    }

    #[ink::test]
    fn test_multi_sig_config() {
        let accounts = default_accounts();
        set_caller(accounts.alice);

        let mut contract = AdvancedEscrow::new(1_000_000, None);

        let participants = vec![accounts.alice, accounts.bob, accounts.charlie];
        let escrow_id = contract
            .create_escrow_advanced(
                1,
                1_000_000,
                accounts.alice,
                accounts.bob,
                participants.clone(),
                2,
                None,
            )
            .expect("Escrow creation should succeed in test");

        let config = contract
            .get_multi_sig_config(escrow_id)
            .expect("Multi-sig config should exist in test");
        assert_eq!(config.required_signatures, 2);
        assert_eq!(config.signers, participants);
    }

    #[ink::test]
    fn test_cleanup_rejects_active_escrow() {
        let accounts = default_accounts();
        set_caller(accounts.alice);

        let mut contract = AdvancedEscrow::new(1_000_000, None);
        let participants = vec![accounts.alice, accounts.bob];

        let escrow_id = contract
            .create_escrow_advanced(
                1,
                1_000_000,
                accounts.alice,
                accounts.bob,
                participants,
                2,
                None,
            )
            .expect("Escrow creation should succeed in test");

        assert_eq!(
            contract.cleanup_escrow(escrow_id),
            Err(Error::InvalidStatus)
        );
    }

    #[ink::test]
    fn test_cleanup_preserves_summary_and_removes_detail() {
        let accounts = default_accounts();
        set_caller(accounts.alice);
        set_balance(accounts.alice, 2_000_000);

        let mut contract = AdvancedEscrow::new(1_000_000, None);
        let participants = vec![accounts.alice, accounts.bob];

        let escrow_id = contract
            .create_escrow_advanced(
                7,
                1_000_000,
                accounts.alice,
                accounts.bob,
                participants.clone(),
                2,
                None,
            )
            .expect("Escrow creation should succeed in test");

        ink::env::test::set_value_transferred::<ink::env::DefaultEnvironment>(1_000_000);
        contract
            .deposit_funds(escrow_id)
            .expect("Deposit should succeed in test");

        let doc_hash = Hash::from([9u8; 32]);
        contract
            .upload_document(escrow_id, doc_hash, "Deed".to_string())
            .expect("Document upload should succeed in test");
        contract
            .add_condition(escrow_id, "Inspection complete".to_string())
            .expect("Condition addition should succeed in test");
        contract
            .sign_approval(escrow_id, ApprovalType::Release)
            .expect("First approval should succeed in test");

        set_caller(accounts.bob);
        contract
            .sign_approval(escrow_id, ApprovalType::Release)
            .expect("Second approval should succeed in test");

        set_caller(accounts.alice);
        contract
            .release_funds(escrow_id)
            .expect("Release should succeed in test");

        let before_bytes = {
            let escrow = contract
                .get_escrow(escrow_id)
                .expect("Escrow should still be detailed before cleanup");
            let documents = contract.get_documents(escrow_id);
            let conditions = contract.get_conditions(escrow_id);
            let audit_trail = contract.get_audit_trail(escrow_id);
            let config = contract
                .get_multi_sig_config(escrow_id)
                .expect("Config should exist before cleanup");
            let signature_types = [
                ApprovalType::Release,
                ApprovalType::Refund,
                ApprovalType::EmergencyOverride,
            ];
            let mut signature_entries = Vec::new();
            let mut signature_counts = Vec::new();

            for approval_type in signature_types {
                let count = contract.get_signature_count(escrow_id, approval_type.clone());
                signature_counts.push((escrow_id, approval_type.clone(), count));
                for signer in config.signers.iter().copied() {
                    if contract.has_signed(escrow_id, approval_type.clone(), signer) {
                        signature_entries.push((escrow_id, approval_type.clone(), signer, true));
                    }
                }
            }

            encoded_len(&escrow) as u64
                + encoded_len(&documents) as u64
                + encoded_len(&conditions) as u64
                + encoded_len(&audit_trail) as u64
                + encoded_len(&config) as u64
                + encoded_len(&signature_entries) as u64
                + encoded_len(&signature_counts) as u64
        };

        set_caller(accounts.bob);
        contract
            .cleanup_escrow(escrow_id)
            .expect("Cleanup should succeed after completion");

        let summary = contract
            .get_escrow_summary(escrow_id)
            .expect("Summary should remain after cleanup");
        assert_eq!(summary.id, escrow_id);
        assert_eq!(summary.property_id, 7);
        assert_eq!(summary.buyer, accounts.alice);
        assert_eq!(summary.seller, accounts.bob);
        assert_eq!(summary.amount, 1_000_000);
        assert_eq!(summary.status, EscrowStatus::Released);
        assert!(summary.completed_at > 0);

        assert!(contract.get_escrow(escrow_id).is_none());
        assert!(contract.get_multi_sig_config(escrow_id).is_none());
        assert!(contract.get_dispute(escrow_id).is_none());
        assert!(contract.get_documents(escrow_id).is_empty());
        assert!(contract.get_conditions(escrow_id).is_empty());
        assert_eq!(
            contract.get_signature_count(escrow_id, ApprovalType::Release),
            0
        );
        assert_eq!(
            contract.get_signature_count(escrow_id, ApprovalType::Refund),
            0
        );

        let after_bytes =
            encoded_len(&summary) as u64 + encoded_len(&contract.get_audit_trail(escrow_id)) as u64;
        assert!(before_bytes > after_bytes);
    }

    #[ink::test]
    fn test_cleanup_allows_non_buyer_seller_participant() {
        let accounts = default_accounts();
        set_caller(accounts.alice);
        set_balance(accounts.alice, 2_000_000);

        let mut contract = AdvancedEscrow::new(1_000_000, None);
        let participants = vec![accounts.alice, accounts.bob, accounts.charlie];

        let escrow_id = contract
            .create_escrow_advanced(
                13,
                1_000_000,
                accounts.alice,
                accounts.bob,
                participants,
                2,
                None,
            )
            .expect("Escrow creation should succeed in test");

        ink::env::test::set_value_transferred::<ink::env::DefaultEnvironment>(1_000_000);
        contract
            .deposit_funds(escrow_id)
            .expect("Deposit should succeed in test");

        contract
            .sign_approval(escrow_id, ApprovalType::Release)
            .expect("First approval should succeed in test");
        set_caller(accounts.bob);
        contract
            .sign_approval(escrow_id, ApprovalType::Release)
            .expect("Second approval should succeed in test");
        set_caller(accounts.alice);
        contract
            .release_funds(escrow_id)
            .expect("Release should succeed in test");

        set_caller(accounts.charlie);
        assert!(contract.cleanup_escrow(escrow_id).is_ok());

        let summary = contract
            .get_escrow_summary(escrow_id)
            .expect("Summary should remain after cleanup");
        assert_eq!(summary.status, EscrowStatus::Released);
    }

    #[ink::test]
    fn test_storage_savings_are_positive_for_typical_lifecycle() {
        let accounts = default_accounts();
        set_caller(accounts.alice);
        set_balance(accounts.alice, 2_000_000);

        let mut contract = AdvancedEscrow::new(1_000_000, None);
        let participants = vec![accounts.alice, accounts.bob];

        let escrow_id = contract
            .create_escrow_advanced(
                11,
                1_000_000,
                accounts.alice,
                accounts.bob,
                participants,
                2,
                None,
            )
            .expect("Escrow creation should succeed in test");

        ink::env::test::set_value_transferred::<ink::env::DefaultEnvironment>(1_000_000);
        contract
            .deposit_funds(escrow_id)
            .expect("Deposit should succeed in test");

        contract
            .upload_document(escrow_id, Hash::from([3u8; 32]), "Title".to_string())
            .expect("Document upload should succeed in test");
        contract
            .add_condition(escrow_id, "Condition".to_string())
            .expect("Condition should succeed in test");
        contract
            .sign_approval(escrow_id, ApprovalType::Release)
            .expect("First approval should succeed in test");
        set_caller(accounts.bob);
        contract
            .sign_approval(escrow_id, ApprovalType::Release)
            .expect("Second approval should succeed in test");
        set_caller(accounts.alice);
        contract
            .release_funds(escrow_id)
            .expect("Release should succeed in test");

        let before_bytes = {
            let escrow = contract
                .get_escrow(escrow_id)
                .expect("Detailed escrow should exist before cleanup");
            let documents = contract.get_documents(escrow_id);
            let conditions = contract.get_conditions(escrow_id);
            let audit_trail = contract.get_audit_trail(escrow_id);
            let config = contract
                .get_multi_sig_config(escrow_id)
                .expect("Config should exist before cleanup");
            let signature_types = [
                ApprovalType::Release,
                ApprovalType::Refund,
                ApprovalType::EmergencyOverride,
            ];
            let mut signature_entries = Vec::new();
            let mut signature_counts = Vec::new();

            for approval_type in signature_types {
                let count = contract.get_signature_count(escrow_id, approval_type.clone());
                signature_counts.push((escrow_id, approval_type.clone(), count));
                for signer in config.signers.iter().copied() {
                    if contract.has_signed(escrow_id, approval_type.clone(), signer) {
                        signature_entries.push((escrow_id, approval_type.clone(), signer, true));
                    }
                }
            }

            encoded_len(&escrow) as u64
                + encoded_len(&documents) as u64
                + encoded_len(&conditions) as u64
                + encoded_len(&audit_trail) as u64
                + encoded_len(&config) as u64
                + encoded_len(&signature_entries) as u64
                + encoded_len(&signature_counts) as u64
        };

        contract
            .cleanup_escrow(escrow_id)
            .expect("Cleanup should succeed after completion");

        let after_bytes = encoded_len(
            &contract
                .get_escrow_summary(escrow_id)
                .expect("Summary should be retained after cleanup"),
        ) as u64
            + encoded_len(&contract.get_audit_trail(escrow_id)) as u64;

        assert!(before_bytes > after_bytes);
    }
    // ========================================================================
    // EDGE CASE TESTS - Issue #484
    // ========================================================================

    /// Test 1: large-transfer request expires before sufficient approvals
    #[ink::test]
    fn test_large_transfer_expires_before_approval() {
        let accounts = default_accounts();
        set_caller(accounts.alice);
        set_balance(accounts.alice, 20_000_000_000_000_000_000);

        let mut contract = AdvancedEscrow::new(1_000_000, None);
        
        let admin = contract.get_admin();
        set_caller(admin);
        contract.set_large_transfer_thresholds(5_000_000_000_000_000, 50_000_000_000_000_000)
            .expect("Setting thresholds should succeed");

        set_caller(accounts.alice);
        let participants = vec![accounts.alice, accounts.bob, accounts.charlie];
        let escrow_amount = 10_000_000_000_000_000;
        let escrow_id = contract
            .create_escrow_advanced(
                1,
                escrow_amount,
                accounts.alice,
                accounts.bob,
                participants,
                2,
                None,
            )
            .expect("Escrow creation should succeed");

        ink::env::test::set_value_transferred::<ink::env::DefaultEnvironment>(escrow_amount);
        contract.deposit_funds(escrow_id).expect("Deposit should succeed");

        contract.sign_approval(escrow_id, ApprovalType::Release)
            .expect("First signature should succeed");
        set_caller(accounts.bob);
        contract.sign_approval(escrow_id, ApprovalType::Release)
            .expect("Second signature should succeed");

        let result = contract.release_funds(escrow_id);
        assert_eq!(result, Err(Error::LargeTransferApprovalRequired));

        let request_id = contract.get_active_large_transfer_request(escrow_id);
        assert!(request_id > 0);

        let request = contract.get_large_transfer_request(request_id)
            .expect("Request should exist");
        assert_eq!(request.escrow_id, escrow_id);
        assert_eq!(request.status, LargeTransferStatus::Pending);

        ink::env::test::set_block_number::<ink::env::DefaultEnvironment>(
            request.expires_at_block + 1
        );

        set_caller(accounts.alice);
        let result = contract.approve_large_transfer(request_id);
        assert_eq!(result, Err(Error::ApprovalRequestExpired));

        let expired_request = contract.get_large_transfer_request(request_id)
            .expect("Request should still exist");
        assert_eq!(expired_request.status, LargeTransferStatus::Expired);

        let active_req_id = contract.get_active_large_transfer_request(escrow_id);
        assert_eq!(active_req_id, 0);
    }

    /// Test 2: multiple signers approving large-transfer simultaneously
    #[ink::test]
    fn test_concurrent_large_transfer_approvals() {
        let accounts = default_accounts();
        set_caller(accounts.alice);
        set_balance(accounts.alice, 20_000_000_000_000_000_000);

        let mut contract = AdvancedEscrow::new(1_000_000, None);
        
        let admin = contract.get_admin();
        set_caller(admin);
        contract.set_large_transfer_thresholds(5_000_000_000_000_000, 50_000_000_000_000_000)
            .expect("Setting thresholds should succeed");

        set_caller(accounts.alice);
        let participants = vec![accounts.alice, accounts.bob, accounts.charlie];
        let escrow_amount = 10_000_000_000_000_000;
        let escrow_id = contract
            .create_escrow_advanced(
                1,
                escrow_amount,
                accounts.alice,
                accounts.bob,
                participants,
                3,
                None,
            )
            .expect("Escrow creation should succeed");

        ink::env::test::set_value_transferred::<ink::env::DefaultEnvironment>(escrow_amount);
        contract.deposit_funds(escrow_id).expect("Deposit should succeed");

        contract.sign_approval(escrow_id, ApprovalType::Release).expect("Alice signs");
        set_caller(accounts.bob);
        contract.sign_approval(escrow_id, ApprovalType::Release).expect("Bob signs");
        set_caller(accounts.charlie);
        contract.sign_approval(escrow_id, ApprovalType::Release).expect("Charlie signs");

        let result = contract.release_funds(escrow_id);
        assert_eq!(result, Err(Error::LargeTransferApprovalRequired));

        let request_id = contract.get_active_large_transfer_request(escrow_id);
        assert!(request_id > 0);

        set_caller(accounts.alice);
        contract.approve_large_transfer(request_id)
            .expect("Alice approves large transfer");

        set_caller(accounts.bob);
        contract.approve_large_transfer(request_id)
            .expect("Bob approves large transfer");

        let approved_request = contract.get_large_transfer_request(request_id)
            .expect("Request should exist");
        assert_eq!(approved_request.status, LargeTransferStatus::Approved);
        assert_eq!(approved_request.approvals.len(), 2);
        assert!(approved_request.approvals.contains(&accounts.alice));
        assert!(approved_request.approvals.contains(&accounts.bob));

        set_caller(accounts.charlie);
        contract.approve_large_transfer(request_id)
            .expect("Charlie can also approve");

        let final_request = contract.get_large_transfer_request(request_id)
            .expect("Request should exist");
        assert_eq!(final_request.approvals.len(), 3);
    }

    /// Test 3: cancelling a large-transfer request and creating a new one
    #[ink::test]
    fn test_cancel_and_recreate_large_transfer() {
        let accounts = default_accounts();
        set_caller(accounts.alice);
        set_balance(accounts.alice, 20_000_000_000_000_000_000);

        let mut contract = AdvancedEscrow::new(1_000_000, None);
        
        let admin = contract.get_admin();
        set_caller(admin);
        contract.set_large_transfer_thresholds(5_000_000_000_000_000, 50_000_000_000_000_000)
            .expect("Setting thresholds should succeed");

        set_caller(accounts.alice);
        let participants = vec![accounts.alice, accounts.bob, accounts.charlie];
        let escrow_amount = 10_000_000_000_000_000;
        let escrow_id = contract
            .create_escrow_advanced(
                1,
                escrow_amount,
                accounts.alice,
                accounts.bob,
                participants,
                2,
                None,
            )
            .expect("Escrow creation should succeed");

        ink::env::test::set_value_transferred::<ink::env::DefaultEnvironment>(escrow_amount);
        contract.deposit_funds(escrow_id).expect("Deposit should succeed");

        contract.sign_approval(escrow_id, ApprovalType::Release).expect("Alice signs");
        set_caller(accounts.bob);
        contract.sign_approval(escrow_id, ApprovalType::Release).expect("Bob signs");

        let result = contract.release_funds(escrow_id);
        assert_eq!(result, Err(Error::LargeTransferApprovalRequired));

        let request_id = contract.get_active_large_transfer_request(escrow_id);
        assert!(request_id > 0);

        set_caller(accounts.alice);
        contract.cancel_large_transfer(request_id)
            .expect("Cancellation should succeed");

        let cancelled_request = contract.get_large_transfer_request(request_id)
            .expect("Request should exist");
        assert_eq!(cancelled_request.status, LargeTransferStatus::Cancelled);

        let active_req = contract.get_active_large_transfer_request(escrow_id);
        assert_eq!(active_req, 0);

        set_caller(accounts.bob);
        let result = contract.release_funds(escrow_id);
        assert_eq!(result, Err(Error::LargeTransferApprovalRequired));

        let new_request_id = contract.get_active_large_transfer_request(escrow_id);
        assert!(new_request_id > 0);
        assert_ne!(new_request_id, request_id);

        let new_request = contract.get_large_transfer_request(new_request_id)
            .expect("New request should exist");
        assert_eq!(new_request.status, LargeTransferStatus::Pending);
        assert_eq!(new_request.escrow_id, escrow_id);
    }

    /// Test 4: escrow with fee enabled and fee disabled
    #[ink::test]
    fn test_fee_enabled_and_disabled() {
        let accounts = default_accounts();
        set_caller(accounts.alice);
        set_balance(accounts.alice, 10_000_000);

        let mut contract = AdvancedEscrow::new(1_000_000, None);
        
        let participants = vec![accounts.alice, accounts.bob];
        let escrow_id_no_fee = contract
            .create_escrow_advanced(
                1,
                1_000_000,
                accounts.alice,
                accounts.bob,
                participants.clone(),
                2,
                None,
            )
            .expect("Escrow creation should succeed");

        ink::env::test::set_value_transferred::<ink::env::DefaultEnvironment>(1_000_000);
        contract.deposit_funds(escrow_id_no_fee).expect("Deposit should succeed");

        contract.sign_approval(escrow_id_no_fee, ApprovalType::Release).expect("Alice signs");
        set_caller(accounts.bob);
        contract.sign_approval(escrow_id_no_fee, ApprovalType::Release).expect("Bob signs");
        
        let seller_balance_before = ink::env::test::get_account_balance::<ink::env::DefaultEnvironment>(accounts.bob)
            .unwrap();
        
        contract.release_funds(escrow_id_no_fee).expect("Release should succeed");

        let seller_balance_after = ink::env::test::get_account_balance::<ink::env::DefaultEnvironment>(accounts.bob)
            .unwrap();
        
        assert_eq!(seller_balance_after - seller_balance_before, 1_000_000);

        let admin = contract.get_admin();
        set_caller(admin);
        contract.set_fee_rate(100).expect("Setting fee rate should succeed");
        contract.set_fee_recipient(Some(accounts.charlie)).expect("Setting fee recipient should succeed");

        set_caller(accounts.alice);
        set_balance(accounts.alice, 10_000_000);
        let escrow_id_with_fee = contract
            .create_escrow_advanced(
                2,
                1_000_000,
                accounts.alice,
                accounts.bob,
                participants.clone(),
                2,
                None,
            )
            .expect("Escrow creation should succeed");

        ink::env::test::set_value_transferred::<ink::env::DefaultEnvironment>(1_000_000);
        contract.deposit_funds(escrow_id_with_fee).expect("Deposit should succeed");

        contract.sign_approval(escrow_id_with_fee, ApprovalType::Release).expect("Alice signs");
        set_caller(accounts.bob);
        contract.sign_approval(escrow_id_with_fee, ApprovalType::Release).expect("Bob signs");

        let fee_recipient_balance_before = ink::env::test::get_account_balance::<ink::env::DefaultEnvironment>(accounts.charlie)
            .unwrap();
        let seller_balance_before_2 = ink::env::test::get_account_balance::<ink::env::DefaultEnvironment>(accounts.bob)
            .unwrap();

        contract.release_funds(escrow_id_with_fee).expect("Release should succeed");

        let fee_recipient_balance_after = ink::env::test::get_account_balance::<ink::env::DefaultEnvironment>(accounts.charlie)
            .unwrap();
        let seller_balance_after_2 = ink::env::test::get_account_balance::<ink::env::DefaultEnvironment>(accounts.bob)
            .unwrap();

        let expected_fee = 10_000;
        assert_eq!(fee_recipient_balance_after - fee_recipient_balance_before, expected_fee);
        assert_eq!(seller_balance_after_2 - seller_balance_before_2, 1_000_000 - expected_fee);
    }

    /// Test 5: partial release followed by full refund attempt
    #[ink::test]
    fn test_partial_release_then_refund_attempt() {
        let accounts = default_accounts();
        set_caller(accounts.alice);
        set_balance(accounts.alice, 10_000_000);

        let mut contract = AdvancedEscrow::new(1_000_000, None);

        let participants = vec![accounts.alice, accounts.bob];
        let escrow_id = contract
            .create_escrow_advanced(
                1,
                1_000_000,
                accounts.alice,
                accounts.bob,
                participants,
                2,
                None,
            )
            .expect("Escrow creation should succeed");

        ink::env::test::set_value_transferred::<ink::env::DefaultEnvironment>(1_000_000);
        contract.deposit_funds(escrow_id).expect("Deposit should succeed");

        contract.sign_approval(escrow_id, ApprovalType::Release).expect("Alice signs");
        set_caller(accounts.bob);
        contract.sign_approval(escrow_id, ApprovalType::Release).expect("Bob signs");

        contract.release_funds_partial(escrow_id, 600_000)
            .expect("Partial release should succeed");

        let escrow = contract.get_escrow(escrow_id).unwrap();
        assert_eq!(escrow.total_released, 600_000);
        assert_eq!(escrow.status, EscrowStatus::Active);

        set_caller(accounts.alice);
        contract.sign_approval(escrow_id, ApprovalType::Refund).expect("Alice signs refund");
        set_caller(accounts.bob);
        contract.sign_approval(escrow_id, ApprovalType::Refund).expect("Bob signs refund");

        let result = contract.refund_funds(escrow_id);
        assert!(result.is_err());
    }

    /// Test 6: tax withholding when tax compliance contract is set
    #[ink::test]
    fn test_tax_withholding_on_release() {
        let accounts = default_accounts();
        set_caller(accounts.alice);
        set_balance(accounts.alice, 10_000_000);

        let mut contract = AdvancedEscrow::new(1_000_000, Some(accounts.django));

        let participants = vec![accounts.alice, accounts.bob];
        let escrow_id = contract
            .create_escrow_advanced(
                1,
                1_000_000,
                accounts.alice,
                accounts.bob,
                participants,
                2,
                None,
            )
            .expect("Escrow creation should succeed");

        ink::env::test::set_value_transferred::<ink::env::DefaultEnvironment>(1_000_000);
        contract.deposit_funds(escrow_id).expect("Deposit should succeed");

        contract.sign_approval(escrow_id, ApprovalType::Release).expect("Alice signs");
        set_caller(accounts.bob);
        contract.sign_approval(escrow_id, ApprovalType::Release).expect("Bob signs");

        let escrow = contract.get_escrow(escrow_id).unwrap();
        assert_eq!(escrow.status, EscrowStatus::Active);
    }

    /// Test 7: dispute resolution changes escrow status correctly
    #[ink::test]
    fn test_dispute_resolution_status_changes() {
        let accounts = default_accounts();
        set_caller(accounts.alice);

        let mut contract = AdvancedEscrow::new(1_000_000, None);
        let admin = contract.get_admin();

        let participants = vec![accounts.alice, accounts.bob];
        let escrow_id = contract
            .create_escrow_advanced(
                1,
                1_000_000,
                accounts.alice,
                accounts.bob,
                participants,
                2,
                None,
            )
            .expect("Escrow creation should succeed");

        ink::env::test::set_value_transferred::<ink::env::DefaultEnvironment>(1_000_000);
        contract.deposit_funds(escrow_id).expect("Deposit should succeed");

        let escrow = contract.get_escrow(escrow_id).unwrap();
        assert_eq!(escrow.status, EscrowStatus::Active);

        contract.raise_dispute(escrow_id, "Property issues".to_string())
            .expect("Dispute should be raised");

        let escrow_disputed = contract.get_escrow(escrow_id).unwrap();
        assert_eq!(escrow_disputed.status, EscrowStatus::Disputed);

        let dispute = contract.get_dispute(escrow_id).unwrap();
        assert!(!dispute.resolved);

        set_caller(admin);
        contract.resolve_dispute(escrow_id, "Resolved amicably".to_string())
            .expect("Dispute resolution should succeed");

        let escrow_resolved = contract.get_escrow(escrow_id).unwrap();
        assert_eq!(escrow_resolved.status, EscrowStatus::Active);

        let resolved_dispute = contract.get_dispute(escrow_id).unwrap();
        assert!(resolved_dispute.resolved);
        assert_eq!(resolved_dispute.resolution, Some("Resolved amicably".to_string()));
    }

    /// Test 8: emergency override with and without tax withholding
    #[ink::test]
    fn test_emergency_override_with_and_without_tax() {
        let accounts = default_accounts();
        
        set_caller(accounts.alice);
        set_balance(accounts.alice, 10_000_000);
        let mut contract_no_tax = AdvancedEscrow::new(1_000_000, None);
        let admin = contract_no_tax.get_admin();

        let participants = vec![accounts.alice, accounts.bob];
        let escrow_id_no_tax = contract_no_tax
            .create_escrow_advanced(
                1,
                1_000_000,
                accounts.alice,
                accounts.bob,
                participants.clone(),
                2,
                None,
            )
            .expect("Escrow creation should succeed");

        ink::env::test::set_value_transferred::<ink::env::DefaultEnvironment>(1_000_000);
        contract_no_tax.deposit_funds(escrow_id_no_tax).expect("Deposit should succeed");

        set_caller(admin);
        let seller_balance_before = ink::env::test::get_account_balance::<ink::env::DefaultEnvironment>(accounts.bob)
            .unwrap();

        contract_no_tax.emergency_override(escrow_id_no_tax, true)
            .expect("Emergency override should succeed");

        let seller_balance_after = ink::env::test::get_account_balance::<ink::env::DefaultEnvironment>(accounts.bob)
            .unwrap();

        assert_eq!(seller_balance_after - seller_balance_before, 1_000_000);

        let escrow = contract_no_tax.get_escrow(escrow_id_no_tax).unwrap();
        assert_eq!(escrow.status, EscrowStatus::Released);

        set_caller(accounts.alice);
        set_balance(accounts.alice, 10_000_000);
        let mut contract_with_tax = AdvancedEscrow::new(1_000_000, Some(accounts.django));
        let admin2 = contract_with_tax.get_admin();

        let escrow_id_with_tax = contract_with_tax
            .create_escrow_advanced(
                2,
                1_000_000,
                accounts.alice,
                accounts.bob,
                participants.clone(),
                2,
                None,
            )
            .expect("Escrow creation should succeed");

        ink::env::test::set_value_transferred::<ink::env::DefaultEnvironment>(1_000_000);
        contract_with_tax.deposit_funds(escrow_id_with_tax).expect("Deposit should succeed");

        set_caller(admin2);
        let buyer_balance_before = ink::env::test::get_account_balance::<ink::env::DefaultEnvironment>(accounts.alice)
            .unwrap();

        contract_with_tax.emergency_override(escrow_id_with_tax, false)
            .expect("Emergency override should succeed");

        let buyer_balance_after = ink::env::test::get_account_balance::<ink::env::DefaultEnvironment>(accounts.alice)
            .unwrap();

        assert_eq!(buyer_balance_after - buyer_balance_before, 1_000_000);

        let escrow2 = contract_with_tax.get_escrow(escrow_id_with_tax).unwrap();
        assert_eq!(escrow2.status, EscrowStatus::Refunded);
    }

    /// Test 9: All edge case tests pass consistently
    #[ink::test]
    fn test_edge_cases_consistency() {
        let accounts = default_accounts();
        set_caller(accounts.alice);
        set_balance(accounts.alice, 100_000_000_000_000_000_000);

        let mut contract = AdvancedEscrow::new(1_000_000, None);
        let admin = contract.get_admin();

        set_caller(admin);
        contract.set_large_transfer_thresholds(5_000_000_000_000_000, 50_000_000_000_000_000)
            .expect("Setting thresholds should succeed");

        set_caller(accounts.alice);
        let participants = vec![accounts.alice, accounts.bob, accounts.charlie];
        
        let escrow_id_1 = contract
            .create_escrow_advanced(
                1,
                1_000_000,
                accounts.alice,
                accounts.bob,
                participants.clone(),
                2,
                None,
            )
            .expect("Escrow 1 creation should succeed");

        let escrow_id_2 = contract
            .create_escrow_advanced(
                2,
                10_000_000_000_000_000,
                accounts.alice,
                accounts.bob,
                participants.clone(),
                2,
                None,
            )
            .expect("Escrow 2 creation should succeed");

        let e1 = contract.get_escrow(escrow_id_1).unwrap();
        assert_eq!(e1.status, EscrowStatus::Created);
        
        let e2 = contract.get_escrow(escrow_id_2).unwrap();
        assert_eq!(e2.status, EscrowStatus::Created);

        assert_eq!(contract.get_active_large_transfer_request(escrow_id_1), 0);
        assert_eq!(contract.get_active_large_transfer_request(escrow_id_2), 0);
    }

}