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
        let result = contract.register_property_with_token(metadata);
        assert!(result.is_ok());
        let token_id = result.unwrap();
        assert_eq!(token_id, 1);
        assert_eq!(contract.total_supply(), 1);
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

    #[ink::test]
    fn test_batch_transfer_empty() {
        let mut contract = setup_contract();
        let accounts = test::default_accounts::<DefaultEnvironment>();
        test::set_caller::<DefaultEnvironment>(accounts.alice);

        let result = contract.safe_batch_transfer_from(
            accounts.alice,
            accounts.bob,
            vec![],
            vec![],
            vec![],
        );
        assert_eq!(result, Err(Error::InvalidAmount));
    }
}
