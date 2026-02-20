#[cfg(test)]
pub mod escrow_tests {
    use crate::propchain_escrow::*;
    use ink::env::test::DefaultAccounts;
    use ink::primitives::{AccountId, Hash};

    fn default_accounts() -> DefaultAccounts<ink::env::DefaultEnvironment> {
        ink::env::test::default_accounts::<ink::env::DefaultEnvironment>()
    }

    fn set_caller(caller: AccountId) {
        ink::env::test::set_caller::<ink::env::DefaultEnvironment>(caller);
    }

    fn set_balance(account: AccountId, balance: u128) {
        ink::env::test::set_account_balance::<ink::env::DefaultEnvironment>(account, balance);
    }

    #[ink::test]
    fn test_new_contract() {
        let contract = AdvancedEscrow::new(1_000_000);
        assert_eq!(contract.get_high_value_threshold(), 1_000_000);
    }

    #[ink::test]
    fn test_create_escrow_advanced() {
        let accounts = default_accounts();
        set_caller(accounts.alice);

        let mut contract = AdvancedEscrow::new(1_000_000);

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
        let escrow_id = result.unwrap();
        assert_eq!(escrow_id, 1);

        let escrow = contract.get_escrow(escrow_id).unwrap();
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

        let mut contract = AdvancedEscrow::new(1_000_000);

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

        let mut contract = AdvancedEscrow::new(1_000_000);

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

        // Deposit funds
        ink::env::test::set_value_transferred::<ink::env::DefaultEnvironment>(1_000_000);
        let result = contract.deposit_funds(escrow_id);
        assert!(result.is_ok());

        let escrow = contract.get_escrow(escrow_id).unwrap();
        assert_eq!(escrow.deposited_amount, 1_000_000);
        assert_eq!(escrow.status, EscrowStatus::Active);
    }

    #[ink::test]
    fn test_upload_document() {
        let accounts = default_accounts();
        set_caller(accounts.alice);

        let mut contract = AdvancedEscrow::new(1_000_000);

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

        let mut contract = AdvancedEscrow::new(1_000_000);

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

        let doc_hash = Hash::from([1u8; 32]);
        contract
            .upload_document(escrow_id, doc_hash, "Title Deed".to_string())
            .unwrap();

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

        let mut contract = AdvancedEscrow::new(1_000_000);

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

        let result = contract.add_condition(escrow_id, "Property inspection completed".to_string());

        assert!(result.is_ok());
        let condition_id = result.unwrap();
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

        let mut contract = AdvancedEscrow::new(1_000_000);

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

        let condition_id = contract
            .add_condition(escrow_id, "Property inspection completed".to_string())
            .unwrap();

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

        let mut contract = AdvancedEscrow::new(1_000_000);

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

        let mut contract = AdvancedEscrow::new(1_000_000);

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
            .sign_approval(escrow_id, ApprovalType::Release)
            .unwrap();

        // Try to sign again
        let result = contract.sign_approval(escrow_id, ApprovalType::Release);
        assert_eq!(result, Err(Error::AlreadySigned));
    }

    #[ink::test]
    fn test_raise_dispute() {
        let accounts = default_accounts();
        set_caller(accounts.alice);

        let mut contract = AdvancedEscrow::new(1_000_000);

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

        let result =
            contract.raise_dispute(escrow_id, "Property condition not as described".to_string());

        assert!(result.is_ok());

        let dispute = contract.get_dispute(escrow_id).unwrap();
        assert_eq!(dispute.raised_by, accounts.alice);
        assert_eq!(dispute.reason, "Property condition not as described");
        assert!(!dispute.resolved);

        let escrow = contract.get_escrow(escrow_id).unwrap();
        assert_eq!(escrow.status, EscrowStatus::Disputed);
    }

    #[ink::test]
    fn test_resolve_dispute() {
        let accounts = default_accounts();
        set_caller(accounts.alice);

        let mut contract = AdvancedEscrow::new(1_000_000);
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
            .unwrap();

        // Admin resolves dispute
        set_caller(admin);
        let result = contract.resolve_dispute(escrow_id, "Resolved in favor of buyer".to_string());

        assert!(result.is_ok());

        let dispute = contract.get_dispute(escrow_id).unwrap();
        assert!(dispute.resolved);
        assert_eq!(
            dispute.resolution,
            Some("Resolved in favor of buyer".to_string())
        );

        let escrow = contract.get_escrow(escrow_id).unwrap();
        assert_eq!(escrow.status, EscrowStatus::Active);
    }

    #[ink::test]
    fn test_resolve_dispute_unauthorized() {
        let accounts = default_accounts();
        set_caller(accounts.alice);

        let mut contract = AdvancedEscrow::new(1_000_000);

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
            .unwrap();

        // Non-admin tries to resolve
        set_caller(accounts.bob);
        let result = contract.resolve_dispute(escrow_id, "Resolution".to_string());
        assert_eq!(result, Err(Error::Unauthorized));
    }

    #[ink::test]
    fn test_check_all_conditions_met() {
        let accounts = default_accounts();
        set_caller(accounts.alice);

        let mut contract = AdvancedEscrow::new(1_000_000);

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

        // No conditions - should return true
        let result = contract.check_all_conditions_met(escrow_id);
        assert_eq!(result, Ok(true));

        // Add conditions
        let cond1 = contract
            .add_condition(escrow_id, "Condition 1".to_string())
            .unwrap();
        let cond2 = contract
            .add_condition(escrow_id, "Condition 2".to_string())
            .unwrap();

        // Not all met
        let result = contract.check_all_conditions_met(escrow_id);
        assert_eq!(result, Ok(false));

        // Mark first condition met
        contract.mark_condition_met(escrow_id, cond1).unwrap();
        let result = contract.check_all_conditions_met(escrow_id);
        assert_eq!(result, Ok(false));

        // Mark second condition met
        contract.mark_condition_met(escrow_id, cond2).unwrap();
        let result = contract.check_all_conditions_met(escrow_id);
        assert_eq!(result, Ok(true));
    }

    #[ink::test]
    fn test_audit_trail() {
        let accounts = default_accounts();
        set_caller(accounts.alice);

        let mut contract = AdvancedEscrow::new(1_000_000);

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

        // Perform some actions
        contract
            .add_condition(escrow_id, "Test condition".to_string())
            .unwrap();
        let doc_hash = Hash::from([1u8; 32]);
        contract
            .upload_document(escrow_id, doc_hash, "Test doc".to_string())
            .unwrap();

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

        let mut contract = AdvancedEscrow::new(1_000_000);
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

        let mut contract = AdvancedEscrow::new(1_000_000);

        // Try to set admin as non-admin
        set_caller(accounts.bob);
        let result = contract.set_admin(accounts.charlie);
        assert_eq!(result, Err(Error::Unauthorized));
    }

    #[ink::test]
    fn test_multi_sig_config() {
        let accounts = default_accounts();
        set_caller(accounts.alice);

        let mut contract = AdvancedEscrow::new(1_000_000);

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
            .unwrap();

        let config = contract.get_multi_sig_config(escrow_id).unwrap();
        assert_eq!(config.required_signatures, 2);
        assert_eq!(config.signers, participants);
    }
}
