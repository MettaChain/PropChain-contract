#[cfg(test)]
mod tests {
    use crate::propchain_contracts::PropertyRegistry;
    use crate::propchain_contracts::Error;
    use ink::primitives::AccountId;
    use propchain_traits::*;

    fn default_accounts() -> ink::env::test::DefaultAccounts<ink::env::DefaultEnvironment> {
        ink::env::test::default_accounts::<ink::env::DefaultEnvironment>()
    }

    fn set_caller(sender: AccountId) {
        ink::env::test::set_caller::<ink::env::DefaultEnvironment>(sender);
    }

    #[ink::test]
    fn new_works() {
        let contract = PropertyRegistry::new();
        assert_eq!(contract.property_count(), 0);
    }

    #[ink::test]
    fn register_property_works() {
        let accounts = default_accounts();
        set_caller(accounts.alice);

        let mut contract = PropertyRegistry::new();
        
        let metadata = PropertyMetadata {
            location: "123 Main St".to_string(),
            size: 1000,
            legal_description: "Test property".to_string(),
            valuation: 1000000,
            documents_url: "https://example.com/docs".to_string(),
        };

        let property_id = contract.register_property(metadata).expect("Failed to register property");
        assert_eq!(property_id, 1);
        assert_eq!(contract.property_count(), 1);

        let property = contract.get_property(property_id).unwrap();
        assert_eq!(property.owner, accounts.alice);
        assert_eq!(property.metadata.location, "123 Main St");
    }

    #[ink::test]
    fn transfer_property_works() {
        let accounts = default_accounts();
        set_caller(accounts.alice);

        let mut contract = PropertyRegistry::new();
        
        let metadata = PropertyMetadata {
            location: "123 Main St".to_string(),
            size: 1000,
            legal_description: "Test property".to_string(),
            valuation: 1000000,
            documents_url: "https://example.com/docs".to_string(),
        };

        let property_id = contract.register_property(metadata).expect("Failed to register property");
        
        // Transfer to bob
        set_caller(accounts.alice);
        assert!(contract.transfer_property(property_id, accounts.bob).is_ok());

        let property = contract.get_property(property_id).unwrap();
        assert_eq!(property.owner, accounts.bob);
    }

    #[ink::test]
    fn transfer_unauthorized_fails() {
        let accounts = default_accounts();
        set_caller(accounts.alice);

        let mut contract = PropertyRegistry::new();
        
        let metadata = PropertyMetadata {
            location: "123 Main St".to_string(),
            size: 1000,
            legal_description: "Test property".to_string(),
            valuation: 1000000,
            documents_url: "https://example.com/docs".to_string(),
        };

        let property_id = contract.register_property(metadata).expect("Failed to register property");
        
        // Try to transfer as charlie (not owner)
        set_caller(accounts.charlie);
        assert_eq!(contract.transfer_property(property_id, accounts.bob), Err(Error::Unauthorized));
    }

    #[ink::test]
    fn get_nonexistent_property_fails() {
        let contract = PropertyRegistry::new();
        assert_eq!(contract.get_property(999), None);
    }

    #[ink::test]
    fn update_metadata_works() {
        let accounts = default_accounts();
        set_caller(accounts.alice);

        let mut contract = PropertyRegistry::new();
        
        let metadata = PropertyMetadata {
            location: "123 Main St".to_string(),
            size: 1000,
            legal_description: "Test property".to_string(),
            valuation: 1000000,
            documents_url: "https://example.com/docs".to_string(),
        };

        let property_id = contract.register_property(metadata.clone()).expect("Failed to register");

        let new_metadata = PropertyMetadata {
            location: "123 Main St Updated".to_string(),
            size: 1100,
            legal_description: "Test property updated".to_string(),
            valuation: 1100000,
            documents_url: "https://example.com/docs/new".to_string(),
        };

        assert!(contract.update_metadata(property_id, new_metadata.clone()).is_ok());

        let property = contract.get_property(property_id).unwrap();
        assert_eq!(property.metadata, new_metadata);

        // Check event emission
        let events = ink::env::test::recorded_events().collect::<Vec<_>>();
        assert!(events.len() > 1); // Registration + Update
    }

    #[ink::test]
    fn update_metadata_unauthorized_fails() {
        let accounts = default_accounts();
        set_caller(accounts.alice);
        let mut contract = PropertyRegistry::new();
        
        let metadata = PropertyMetadata {
            location: "123 Main St".to_string(),
            size: 1000,
            legal_description: "Test property".to_string(),
            valuation: 1000000,
            documents_url: "https://example.com/docs".to_string(),
        };
        let property_id = contract.register_property(metadata).expect("Failed to register");

        set_caller(accounts.bob);
        let new_metadata = PropertyMetadata {
            location: "123 Main St Updated".to_string(),
            size: 1100,
            legal_description: "Test property updated".to_string(),
            valuation: 1100000,
            documents_url: "https://example.com/docs/new".to_string(),
        };
        assert_eq!(contract.update_metadata(property_id, new_metadata), Err(Error::Unauthorized));
    }

    #[ink::test]
    fn approval_work() {
        let accounts = default_accounts();
        set_caller(accounts.alice);
        let mut contract = PropertyRegistry::new();
        
        let metadata = PropertyMetadata {
            location: "123 Main St".to_string(),
            size: 1000,
            legal_description: "Test property".to_string(),
            valuation: 1000000,
            documents_url: "https://example.com/docs".to_string(),
        };
        let property_id = contract.register_property(metadata).expect("Failed to register");

        // Approve Bob
        assert!(contract.approve(property_id, Some(accounts.bob)).is_ok());
        assert_eq!(contract.get_approved(property_id), Some(accounts.bob));

        // Bob transfers property
        set_caller(accounts.bob);
        assert!(contract.transfer_property(property_id, accounts.charlie).is_ok());

        let property = contract.get_property(property_id).unwrap();
        assert_eq!(property.owner, accounts.charlie);

        // Approval should be cleared
        assert_eq!(contract.get_approved(property_id), None);
    }
}
