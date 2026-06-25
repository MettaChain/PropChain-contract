#![cfg_attr(not(feature = "std"), no_std)]

use ink::env::{
    test::{self, DefaultAccounts},
    DefaultEnvironment,
};
use propchain_contracts::PropertyRegistry;

#[cfg(test)]
mod tests {
    use super::*;

    fn setup_contract() -> PropertyRegistry<DefaultEnvironment> {
        let accounts = DefaultAccounts::default();
        test::set_caller::<DefaultEnvironment>(accounts.alice);
        PropertyRegistry::new()
    }

    #[ink::test]
    fn test_new_works() {
        let contract = setup_contract();
        assert_eq!(contract.property_count(), 0);
    }

    #[ink::test]
    fn test_register_property_works() {
        let mut contract = setup_contract();
        
        let metadata = PropertyMetadata {
            location: "123 Main St".to_string(),
            size: 2000,
            legal_description: "Test property".to_string(),
            valuation: 500000,
            documents_url: "https://ipfs.io/test".to_string(),
        };

        let result = contract.register_property(metadata);
        assert!(result.is_ok());
        
        let property_id = result.unwrap();
        assert_eq!(property_id, 1);
        assert_eq!(contract.property_count(), 1);
    }

    #[ink::test]
    fn test_get_property_works() {
        let mut contract = setup_contract();
        let accounts = DefaultAccounts::default();
        
        let metadata = PropertyMetadata {
            location: "123 Main St".to_string(),
            size: 2000,
            legal_description: "Test property".to_string(),
            valuation: 500000,
            documents_url: "https://ipfs.io/test".to_string(),
        };

        let property_id = contract.register_property(metadata).unwrap();
        let property = contract.get_property(property_id);
        
        assert!(property.is_some());
        let property = property.unwrap();
        assert_eq!(property.owner, accounts.alice);
        assert_eq!(property.metadata.location, "123 Main St");
    }

    #[ink::test]
    fn test_transfer_property_works() {
        let mut contract = setup_contract();
        let accounts = DefaultAccounts::default();
        
        let metadata = PropertyMetadata {
            location: "123 Main St".to_string(),
            size: 2000,
            legal_description: "Test property".to_string(),
            valuation: 500000,
            documents_url: "https://ipfs.io/test".to_string(),
        };

        let property_id = contract.register_property(metadata).unwrap();
        
        // Transfer to Bob
        test::set_caller::<DefaultEnvironment>(accounts.alice);
        let result = contract.transfer_property(property_id, accounts.bob);
        assert!(result.is_ok());
        
        // Verify transfer
        let property = contract.get_property(property_id).unwrap();
        assert_eq!(property.owner, accounts.bob);
    }

    #[ink::test]
    fn test_unauthorized_transfer_fails() {
        let mut contract = setup_contract();
        let accounts = DefaultAccounts::default();
        
        let metadata = PropertyMetadata {
            location: "123 Main St".to_string(),
            size: 2000,
            legal_description: "Test property".to_string(),
            valuation: 500000,
            documents_url: "https://ipfs.io/test".to_string(),
        };

        let property_id = contract.register_property(metadata).unwrap();
        
        // Try to transfer as Charlie (unauthorized)
        test::set_caller::<DefaultEnvironment>(accounts.charlie);
        let result = contract.transfer_property(property_id, accounts.bob);
        assert!(result.is_err());
    }

    fn make_property_metadata(index: u64) -> PropertyMetadata {
        PropertyMetadata {
            location: format!("{} Main St", index),
            size: 2000 + index,
            legal_description: format!("Test property {}", index),
            valuation: 500000 + index as u128,
            documents_url: format!("https://ipfs.io/test/{}", index),
        }
    }

    #[ink::test]
    fn test_batch_register_properties_success() {
        let mut contract = setup_contract();
        let batch_size = contract.get_max_batch_size();
        let mut batch = Vec::new();

        for i in 1..=batch_size {
            batch.push(make_property_metadata(i as u64));
        }

        let result = contract.batch_register_properties(batch);
        assert!(result.is_ok());

        let property_ids = result.unwrap();
        assert_eq!(property_ids.len() as u32, batch_size);
        assert_eq!(contract.property_count(), batch_size as u64);

        let first_property = contract.get_property(property_ids[0]).unwrap();
        assert_eq!(first_property.metadata.location, "1 Main St");
        let last_property = contract.get_property(*property_ids.last().unwrap()).unwrap();
        assert_eq!(last_property.metadata.location, format!("{} Main St", batch_size));
    }

    #[ink::test]
    fn test_batch_register_properties_invalid_property_rolls_back() {
        let mut contract = setup_contract();
        let mut batch = Vec::new();

        for i in 1..=49 {
            batch.push(make_property_metadata(i as u64));
        }
        batch.push(PropertyMetadata {
            location: "".to_string(),
            size: 0,
            legal_description: "Invalid property".to_string(),
            valuation: 0,
            documents_url: "https://ipfs.io/test".to_string(),
        });

        let result = contract.batch_register_properties(batch);
        assert!(result.is_err());
        assert_eq!(contract.property_count(), 0);
    }

    #[ink::test]
    fn test_batch_register_properties_empty_batch_errors() {
        let mut contract = setup_contract();
        let result = contract.batch_register_properties(Vec::new());
        assert_eq!(result, Err(Error::ValueOutOfBounds));
        assert_eq!(contract.property_count(), 0);
    }
}
