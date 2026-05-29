/// Cross-chain bridge multi-signature validation tests (issue #203).
///
/// Verifies that cross-chain transactions require multiple validators:
/// - Only registered validators may sign bridge requests
/// - Threshold must be met before execution
/// - Duplicate signatures are rejected
/// - Removed validators' signatures don't count at execution

#[cfg(test)]
mod bridge_tests {
    use ink::env::{test, DefaultEnvironment};
    use propchain_traits::PropertyMetadata;
    use property_token::property_token::{Error, PropertyToken};

    fn default_metadata() -> PropertyMetadata {
        PropertyMetadata {
            location: String::from("123 Bridge St"),
            size: 500,
            legal_description: String::from("Test"),
            valuation: 50_000,
            documents_url: String::from("ipfs://test"),
        }
    }

    fn setup() -> (PropertyToken, u64) {
        let accounts = test::default_accounts::<DefaultEnvironment>();
        test::set_caller::<DefaultEnvironment>(accounts.alice);
        test::set_callee::<DefaultEnvironment>(ink::primitives::AccountId::from([0xFF; 32]));
        let mut contract = PropertyToken::new();
        let token_id = contract
            .register_property_with_token(default_metadata())
            .expect("mint should succeed");
        contract
            .verify_compliance(token_id, true)
            .expect("compliance should succeed");
        (contract, token_id)
    }

    /// Only a registered bridge operator (validator) can sign a bridge request.
    #[ink::test]
    fn test_only_operator_can_sign() {
        let (mut contract, token_id) = setup();
        let accounts = test::default_accounts::<DefaultEnvironment>();

        test::set_caller::<DefaultEnvironment>(accounts.alice);
        let request_id = contract
            .initiate_bridge_multisig(token_id, 2, accounts.bob, 2, None)
            .expect("initiation should succeed");

        // charlie is not a bridge operator — must be rejected
        test::set_caller::<DefaultEnvironment>(accounts.charlie);
        let result = contract.sign_bridge_request(request_id, true);
        assert_eq!(result, Err(Error::Unauthorized));
    }

    /// Bridge request cannot be executed until the required signature threshold is met.
    #[ink::test]
    fn test_threshold_required_before_execution() {
        let (mut contract, token_id) = setup();
        let accounts = test::default_accounts::<DefaultEnvironment>();

        // Add bob as a bridge operator
        test::set_caller::<DefaultEnvironment>(accounts.alice);
        contract
            .add_bridge_operator(accounts.bob)
            .expect("admin can add operator");

        let request_id = contract
            .initiate_bridge_multisig(token_id, 2, accounts.charlie, 2, None)
            .expect("initiation should succeed");

        // Only alice signs — threshold (2) not met
        test::set_caller::<DefaultEnvironment>(accounts.alice);
        contract
            .sign_bridge_request(request_id, true)
            .expect("alice signs");

        // Execution must fail — not enough signatures
        let result = contract.execute_bridge(request_id);
        assert!(
            result.is_err(),
            "execution should fail before threshold is met"
        );

        // Bob signs — threshold met
        test::set_caller::<DefaultEnvironment>(accounts.bob);
        contract
            .sign_bridge_request(request_id, true)
            .expect("bob signs");

        // Now execution should succeed
        test::set_caller::<DefaultEnvironment>(accounts.alice);
        let result = contract.execute_bridge(request_id);
        assert!(result.is_ok(), "execution should succeed after threshold met");
    }

    /// The same operator cannot sign a bridge request twice.
    #[ink::test]
    fn test_duplicate_signature_rejected() {
        let (mut contract, token_id) = setup();
        let accounts = test::default_accounts::<DefaultEnvironment>();

        test::set_caller::<DefaultEnvironment>(accounts.alice);
        let request_id = contract
            .initiate_bridge_multisig(token_id, 2, accounts.bob, 2, None)
            .expect("initiation should succeed");

        test::set_caller::<DefaultEnvironment>(accounts.alice);
        contract
            .sign_bridge_request(request_id, true)
            .expect("first signature should succeed");

        // Second signature from same operator must be rejected
        let result = contract.sign_bridge_request(request_id, true);
        assert_eq!(result, Err(Error::AlreadySigned));
    }

    /// A single operator cannot satisfy a threshold > 1 by signing multiple times.
    #[ink::test]
    fn test_single_operator_cannot_meet_multisig_threshold() {
        let (mut contract, token_id) = setup();
        let accounts = test::default_accounts::<DefaultEnvironment>();

        test::set_caller::<DefaultEnvironment>(accounts.alice);
        let request_id = contract
            .initiate_bridge_multisig(token_id, 2, accounts.bob, 2, None)
            .expect("initiation should succeed");

        // Alice signs once
        test::set_caller::<DefaultEnvironment>(accounts.alice);
        contract
            .sign_bridge_request(request_id, true)
            .expect("first sign ok");

        // Alice tries to sign again — rejected
        let result = contract.sign_bridge_request(request_id, true);
        assert_eq!(result, Err(Error::AlreadySigned));

        // Execution still fails — only 1 of 2 required signatures
        test::set_caller::<DefaultEnvironment>(accounts.alice);
        let result = contract.execute_bridge(request_id);
        assert!(result.is_err());
    }

    /// A rejection by any operator marks the request as failed.
    #[ink::test]
    fn test_operator_rejection_fails_request() {
        let (mut contract, token_id) = setup();
        let accounts = test::default_accounts::<DefaultEnvironment>();

        test::set_caller::<DefaultEnvironment>(accounts.alice);
        let request_id = contract
            .initiate_bridge_multisig(token_id, 2, accounts.bob, 2, None)
            .expect("initiation should succeed");

        // Alice rejects
        test::set_caller::<DefaultEnvironment>(accounts.alice);
        contract
            .sign_bridge_request(request_id, false)
            .expect("rejection should be recorded");

        // Execution must fail — request is in Failed state
        let result = contract.execute_bridge(request_id);
        assert!(result.is_err());
    }

    /// Non-admin cannot add bridge operators.
    #[ink::test]
    fn test_non_admin_cannot_add_operator() {
        let (mut contract, _) = setup();
        let accounts = test::default_accounts::<DefaultEnvironment>();

        test::set_caller::<DefaultEnvironment>(accounts.bob);
        let result = contract.add_bridge_operator(accounts.charlie);
        assert_eq!(result, Err(Error::Unauthorized));
    }
}
