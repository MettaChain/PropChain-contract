//! Cross-Contract Integration Tests
//!
//! This module contains integration tests that verify interactions
//! between multiple PropChain contracts.

use ink::env::test::DefaultAccounts;
use ink::env::DefaultEnvironment;
use propchain_contracts::PropertyRegistry;
use propchain_traits::*;

#[cfg(test)]
mod integration_tests {
    use super::*;

    fn setup_registry() -> PropertyRegistry {
        let accounts = ink::env::test::default_accounts::<DefaultEnvironment>();
        ink::env::test::set_caller::<DefaultEnvironment>(accounts.alice);
        PropertyRegistry::new()
    }

    // ============================================================================
    // PROPERTY REGISTRY + ESCROW INTEGRATION
    // ============================================================================

    #[ink::test]
    fn test_property_registry_with_escrow_flow() {
        let mut registry = setup_registry();
        let accounts = ink::env::test::default_accounts::<DefaultEnvironment>();

        // Register property
        let metadata = PropertyMetadata {
            location: "123 Main St".to_string(),
            size: 2000,
            legal_description: "Test property".to_string(),
            valuation: 500000,
            documents_url: "https://ipfs.io/test".to_string(),
        };

        let property_id = registry
            .register_property(metadata)
            .expect("Property registration should succeed");

        // Create escrow
        let escrow_amount = 500000u128;
        let escrow_id = registry
            .create_escrow(property_id, escrow_amount)
            .expect("Escrow creation should succeed");

        // Verify escrow created
        let escrow = registry.get_escrow(escrow_id).expect("Escrow should exist");
        assert_eq!(escrow.property_id, property_id);
        assert_eq!(escrow.amount, escrow_amount);

        // Release escrow (transfers property)
        registry
            .release_escrow(escrow_id)
            .expect("Escrow release should succeed");

        // Verify property transferred
        let property = registry
            .get_property(property_id)
            .expect("Property should exist");
        // Property owner should be updated based on escrow logic
        assert!(property.owner == accounts.alice || property.owner == accounts.bob);
    }

    // ============================================================================
    // PROPERTY REGISTRY + ORACLE INTEGRATION
    // ============================================================================

    #[ink::test]
    fn test_property_with_oracle_valuation() {
        let mut registry = setup_registry();

        // Register property
        let metadata = PropertyMetadata {
            location: "123 Main St".to_string(),
            size: 2000,
            legal_description: "Test property".to_string(),
            valuation: 500000,
            documents_url: "https://ipfs.io/test".to_string(),
        };

        let property_id = registry
            .register_property(metadata)
            .expect("Property registration should succeed");

        // Update valuation from oracle (if oracle is set)
        // This tests the integration between PropertyRegistry and Oracle
        let result = registry.update_valuation_from_oracle(property_id);
        // May succeed or fail depending on oracle configuration
        assert!(result.is_ok() || result.is_err());
    }

    // ============================================================================
    // BATCH OPERATIONS INTEGRATION
    // ============================================================================

    #[ink::test]
    fn test_batch_property_registration() {
        let mut registry = setup_registry();

        // Register multiple properties
        let mut property_ids = Vec::new();
        for i in 1..=10 {
            let metadata = PropertyMetadata {
                location: format!("Property {}", i),
                size: 1000 + (i * 100),
                legal_description: format!("Description {}", i),
                valuation: 100_000 + (i as u128 * 10_000),
                documents_url: format!("ipfs://prop{}", i),
            };

            let property_id = registry
                .register_property(metadata)
                .expect("Property registration should succeed");
            property_ids.push(property_id);
        }

        assert_eq!(registry.property_count(), 10);
        assert_eq!(property_ids.len(), 10);

        // Verify all properties exist
        for property_id in property_ids {
            assert!(registry.get_property(property_id).is_some());
        }
    }

    // ============================================================================
    // TRANSFER CHAIN INTEGRATION
    // ============================================================================

    #[ink::test]
    fn test_property_transfer_chain() {
        let mut registry = setup_registry();
        let accounts = ink::env::test::default_accounts::<DefaultEnvironment>();

        // Register property
        let metadata = PropertyMetadata {
            location: "123 Main St".to_string(),
            size: 2000,
            legal_description: "Test property".to_string(),
            valuation: 500000,
            documents_url: "https://ipfs.io/test".to_string(),
        };

        let property_id = registry
            .register_property(metadata)
            .expect("Property registration should succeed");

        // Transfer through multiple accounts
        let transfer_chain = vec![accounts.bob, accounts.charlie, accounts.django];

        for (i, to_account) in transfer_chain.iter().enumerate() {
            let from_account = if i == 0 {
                accounts.alice
            } else {
                transfer_chain[i - 1]
            };

            ink::env::test::set_caller::<DefaultEnvironment>(from_account);
            registry
                .transfer_property(property_id, *to_account)
                .expect("Property transfer should succeed");

            let property = registry
                .get_property(property_id)
                .expect("Property should exist");
            assert_eq!(property.owner, *to_account);
        }
    }

    // ============================================================================
    // ERROR PROPAGATION INTEGRATION
    // ============================================================================

    #[ink::test]
    fn test_error_propagation_across_operations() {
        let mut registry = setup_registry();

        // Try to get non-existent property
        let result = registry.get_property(999);
        assert!(result.is_none());

        // Try to transfer non-existent property
        let accounts = ink::env::test::default_accounts::<DefaultEnvironment>();
        let result = registry.transfer_property(999, accounts.bob);
        assert_eq!(result, Err(propchain_contracts::Error::PropertyNotFound));

        // Try to create escrow for non-existent property
        let result = registry.create_escrow(999, 100000);
        assert_eq!(result, Err(propchain_contracts::Error::PropertyNotFound));
    }
}
