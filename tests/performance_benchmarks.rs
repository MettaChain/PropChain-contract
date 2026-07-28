//! Performance Benchmarks and Regression Tests
//!
//! This module contains performance benchmarks to detect regressions
//! and ensure contract operations meet performance requirements.

use ink::env::test::DefaultEnvironment;
use propchain_contracts::PropertyRegistry;
use propchain_traits::*;

#[cfg(test)]
mod benchmarks {
    use super::*;

    fn setup_registry() -> PropertyRegistry {
        let accounts = ink::env::test::default_accounts::<DefaultEnvironment>();
        ink::env::test::set_caller::<DefaultEnvironment>(accounts.alice);
        PropertyRegistry::new()
    }

    // Maximum expected execution time (in block timestamp units)
    const MAX_REGISTER_TIME: u64 = 1000;
    const MAX_TRANSFER_TIME: u64 = 500;
    const MAX_QUERY_TIME: u64 = 100;

    // ============================================================================
    // REGISTRATION PERFORMANCE
    // ============================================================================

    #[ink::test]
    fn benchmark_register_property() {
        let mut registry = setup_registry();
        let metadata = PropertyMetadata {
            location: "123 Main St".to_string(),
            size: 2000,
            legal_description: "Test property".to_string(),
            valuation: 500000,
            documents_url: "https://ipfs.io/test".to_string(),
        };

        let start = ink::env::test::get_block_timestamp::<DefaultEnvironment>();
        let _property_id = registry
            .register_property(metadata)
            .expect("Registration should succeed");
        let end = ink::env::test::get_block_timestamp::<DefaultEnvironment>();

        let duration = end.saturating_sub(start);
        assert!(
            duration <= MAX_REGISTER_TIME,
            "Registration took {} units, expected <= {}",
            duration,
            MAX_REGISTER_TIME
        );
    }

    #[ink::test]
    fn benchmark_register_multiple_properties() {
        let mut registry = setup_registry();
        let iterations = 100;

        let start = ink::env::test::get_block_timestamp::<DefaultEnvironment>();
        for i in 1..=iterations {
            let metadata = PropertyMetadata {
                location: format!("Property {}", i),
                size: 1000 + (i * 100),
                legal_description: format!("Description {}", i),
                valuation: 100_000 + (i as u128 * 10_000),
                documents_url: format!("ipfs://prop{}", i),
            };

            registry
                .register_property(metadata)
                .expect("Registration should succeed");
        }
        let end = ink::env::test::get_block_timestamp::<DefaultEnvironment>();

        let total_duration = end.saturating_sub(start);
        let avg_duration = total_duration / iterations as u64;
        
        assert!(
            avg_duration <= MAX_REGISTER_TIME,
            "Average registration took {} units, expected <= {}",
            avg_duration,
            MAX_REGISTER_TIME
        );
    }

    // ============================================================================
    // TRANSFER PERFORMANCE
    // ============================================================================

    #[ink::test]
    fn benchmark_transfer_property() {
        let mut registry = setup_registry();
        let accounts = ink::env::test::default_accounts::<DefaultEnvironment>();

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

        let start = ink::env::test::get_block_timestamp::<DefaultEnvironment>();
        registry
            .transfer_property(property_id, accounts.bob)
            .expect("Transfer should succeed");
        let end = ink::env::test::get_block_timestamp::<DefaultEnvironment>();

        let duration = end.saturating_sub(start);
        assert!(
            duration <= MAX_TRANSFER_TIME,
            "Transfer took {} units, expected <= {}",
            duration,
            MAX_TRANSFER_TIME
        );
    }

    // ============================================================================
    // QUERY PERFORMANCE
    // ============================================================================

    #[ink::test]
    fn benchmark_get_property() {
        let mut registry = setup_registry();

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

        let start = ink::env::test::get_block_timestamp::<DefaultEnvironment>();
        let _property = registry
            .get_property(property_id)
            .expect("Property should exist");
        let end = ink::env::test::get_block_timestamp::<DefaultEnvironment>();

        let duration = end.saturating_sub(start);
        assert!(
            duration <= MAX_QUERY_TIME,
            "Query took {} units, expected <= {}",
            duration,
            MAX_QUERY_TIME
        );
    }

    #[ink::test]
    fn benchmark_get_owner_properties() {
        let mut registry = setup_registry();
        let accounts = ink::env::test::default_accounts::<DefaultEnvironment>();

        // Register multiple properties
        for i in 1..=50 {
            let metadata = PropertyMetadata {
                location: format!("Property {}", i),
                size: 1000,
                legal_description: format!("Description {}", i),
                valuation: 100_000,
                documents_url: format!("ipfs://prop{}", i),
            };

            registry
                .register_property(metadata)
                .expect("Property registration should succeed");
        }

        let start = ink::env::test::get_block_timestamp::<DefaultEnvironment>();
        let _properties = registry.get_owner_properties(accounts.alice);
        let end = ink::env::test::get_block_timestamp::<DefaultEnvironment>();

        let duration = end.saturating_sub(start);
        assert!(
            duration <= MAX_QUERY_TIME * 10, // Allow more time for larger queries
            "Query took {} units, expected <= {}",
            duration,
            MAX_QUERY_TIME * 10
        );
    }

    // ============================================================================
    // STRESS TESTS
    // ============================================================================

    #[ink::test]
    fn stress_test_many_registrations() {
        let mut registry = setup_registry();
        let count = 1000;

        for i in 1..=count {
            let metadata = PropertyMetadata {
                location: format!("Property {}", i),
                size: 1000,
                legal_description: format!("Description {}", i),
                valuation: 100_000,
                documents_url: format!("ipfs://prop{}", i),
            };

            let property_id = registry
                .register_property(metadata)
                .expect("Property registration should succeed");
            assert_eq!(property_id, i);
        }

        assert_eq!(registry.property_count(), count);
    }

    #[ink::test]
    fn stress_test_many_transfers() {
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

        // Transfer many times
        let transfer_chain = vec![accounts.bob, accounts.charlie, accounts.django, accounts.eve];
        for _ in 0..100 {
            for (i, &to_account) in transfer_chain.iter().enumerate() {
                let from_account = if i == 0 {
                    accounts.alice
                } else {
                    transfer_chain[i - 1]
                };

                ink::env::test::set_caller::<DefaultEnvironment>(from_account);
                registry
                    .transfer_property(property_id, to_account)
                    .expect("Transfer should succeed");
            }
        }

        // Final owner should be eve (last in chain)
        let property = registry
            .get_property(property_id)
            .expect("Property should exist");
        assert_eq!(property.owner, accounts.eve);
    }
}
