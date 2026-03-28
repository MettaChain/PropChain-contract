#[cfg(test)]
#[allow(clippy::module_inception)]
mod tests {
    use crate::propchain_contracts::Error;
    use crate::propchain_contracts::PropertyRegistry;
    use ink::primitives::AccountId;
    use propchain_traits::*;

    /// Helper function to get default test accounts
    fn default_accounts() -> ink::env::test::DefaultAccounts<ink::env::DefaultEnvironment> {
        ink::env::test::default_accounts::<ink::env::DefaultEnvironment>()
    }

    /// Helper function to set the caller for the next contract call
    fn set_caller(sender: AccountId) {
        ink::env::test::set_caller::<ink::env::DefaultEnvironment>(sender);
    }

    /// Helper function to create a sample property metadata
    fn create_sample_metadata() -> PropertyMetadata {
        PropertyMetadata {
            location: "123 Main St, City, State 12345".to_string(),
            size: 1000,
            legal_description: "Test property legal description".to_string(),
            valuation: 1000000,
            documents_url: "https://example.com/docs".to_string(),
        }
    }

    /// Helper function to create metadata with custom values
    fn create_custom_metadata(
        location: &str,
        size: u64,
        legal_description: &str,
        valuation: u128,
        documents_url: &str,
    ) -> PropertyMetadata {
        PropertyMetadata {
            location: location.to_string(),
            size,
            legal_description: legal_description.to_string(),
            valuation,
            documents_url: documents_url.to_string(),
        }
    }

    // ============================================================================
    // CORE FUNCTIONALITY TESTS
    // ============================================================================

    #[ink::test]
    fn test_constructor_initializes_correctly() {
        let contract = PropertyRegistry::new();
        assert_eq!(contract.property_count(), 0);
    }

    #[ink::test]
    fn test_register_property_success() {
        let accounts = default_accounts();
        set_caller(accounts.alice);

        // Set a block timestamp
        ink::env::test::set_block_timestamp::<ink::env::DefaultEnvironment>(1000);

        let mut contract = PropertyRegistry::new();
        let metadata = create_sample_metadata();

        let property_id = contract
            .register_property(metadata.clone())
            .expect("Failed to register property");

        assert_eq!(property_id, 1);
        assert_eq!(contract.property_count(), 1);

        let property = contract.get_property(property_id).unwrap();
        assert_eq!(property.id, property_id);
        assert_eq!(property.owner, accounts.alice);
        assert_eq!(property.metadata, metadata);
        assert_eq!(property.registered_at, 1000);
    }

    #[ink::test]
    fn test_register_property_increments_counter() {
        let accounts = default_accounts();
        set_caller(accounts.alice);

        let mut contract = PropertyRegistry::new();

        let property_id_1 = contract
            .register_property(create_sample_metadata())
            .expect("Failed to register property 1");
        assert_eq!(property_id_1, 1);
        assert_eq!(contract.property_count(), 1);

        let property_id_2 = contract
            .register_property(create_sample_metadata())
            .expect("Failed to register property 2");
        assert_eq!(property_id_2, 2);
        assert_eq!(contract.property_count(), 2);
    }

    #[ink::test]
    fn test_register_property_emits_event() {
        let accounts = default_accounts();
        set_caller(accounts.alice);

        let mut contract = PropertyRegistry::new();
        let metadata = create_sample_metadata();

        let _property_id = contract
            .register_property(metadata)
            .expect("Failed to register property");

        // Verify that events were emitted
        // ContractInitialized + PropertyRegistered + SecurityAuditEvent
        let emitted_events = ink::env::test::recorded_events().collect::<Vec<_>>();
        assert_eq!(
            emitted_events.len(),
            3,
            "ContractInitialized, PropertyRegistered, and SecurityAuditEvent should be emitted"
        );
    }

    #[ink::test]
    fn test_transfer_property_success() {
        let accounts = default_accounts();
        set_caller(accounts.alice);

        let mut contract = PropertyRegistry::new();
        let property_id = contract
            .register_property(create_sample_metadata())
            .expect("Failed to register property");

        // Transfer to bob
        set_caller(accounts.alice);
        assert!(contract
            .transfer_property(property_id, accounts.bob)
            .is_ok());

        let property = contract.get_property(property_id).unwrap();
        assert_eq!(property.owner, accounts.bob);
        assert_eq!(property.id, property_id);
    }

    #[ink::test]
    fn test_transfer_property_updates_owner_lists() {
        let accounts = default_accounts();
        set_caller(accounts.alice);

        let mut contract = PropertyRegistry::new();
        let property_id_1 = contract
            .register_property(create_sample_metadata())
            .expect("Failed to register property 1");
        let property_id_2 = contract
            .register_property(create_sample_metadata())
            .expect("Failed to register property 2");

        // Verify alice owns both properties
        let alice_properties = contract.get_owner_properties(accounts.alice);
        assert_eq!(alice_properties.len(), 2);
        assert!(alice_properties.contains(&property_id_1));
        assert!(alice_properties.contains(&property_id_2));

        // Transfer property 1 to bob
        set_caller(accounts.alice);
        assert!(contract
            .transfer_property(property_id_1, accounts.bob)
            .is_ok());

        // Verify alice now only owns property 2
        let alice_properties = contract.get_owner_properties(accounts.alice);
        assert_eq!(alice_properties.len(), 1);
        assert_eq!(alice_properties[0], property_id_2);

        // Verify bob now owns property 1
        let bob_properties = contract.get_owner_properties(accounts.bob);
        assert_eq!(bob_properties.len(), 1);
        assert_eq!(bob_properties[0], property_id_1);
    }

    #[ink::test]
    fn test_transfer_property_emits_event() {
        let accounts = default_accounts();
        set_caller(accounts.alice);

        let mut contract = PropertyRegistry::new();
        let property_id = contract
            .register_property(create_sample_metadata())
            .expect("Failed to register property");

        set_caller(accounts.alice);
        assert!(contract
            .transfer_property(property_id, accounts.bob)
            .is_ok());

        // Verify that a transfer event was emitted
        let emitted_events = ink::env::test::recorded_events().collect::<Vec<_>>();
        assert!(
            !emitted_events.is_empty(),
            "PropertyTransferred event should be emitted"
        );
    }

    #[ink::test]
    fn test_get_property_returns_correct_info() {
        let accounts = default_accounts();
        set_caller(accounts.alice);

        let mut contract = PropertyRegistry::new();
        let metadata = create_custom_metadata(
            "456 Oak Ave",
            2000,
            "Custom legal description",
            2000000,
            "https://ipfs.io/custom",
        );

        let property_id = contract
            .register_property(metadata.clone())
            .expect("Failed to register property");

        let property = contract.get_property(property_id).unwrap();
        assert_eq!(property.id, property_id);
        assert_eq!(property.owner, accounts.alice);
        assert_eq!(property.metadata.location, "456 Oak Ave");
        assert_eq!(property.metadata.size, 2000);
        assert_eq!(
            property.metadata.legal_description,
            "Custom legal description"
        );
        assert_eq!(property.metadata.valuation, 2000000);
        assert_eq!(property.metadata.documents_url, "https://ipfs.io/custom");
    }

    #[ink::test]
    fn test_get_owner_properties_returns_correct_list() {
        let accounts = default_accounts();
        set_caller(accounts.alice);

        let mut contract = PropertyRegistry::new();

        // Register multiple properties
        let property_id_1 = contract
            .register_property(create_sample_metadata())
            .expect("Failed to register property 1");
        let property_id_2 = contract
            .register_property(create_sample_metadata())
            .expect("Failed to register property 2");
        let property_id_3 = contract
            .register_property(create_sample_metadata())
            .expect("Failed to register property 3");

        let alice_properties = contract.get_owner_properties(accounts.alice);
        assert_eq!(alice_properties.len(), 3);
        assert!(alice_properties.contains(&property_id_1));
        assert!(alice_properties.contains(&property_id_2));
        assert!(alice_properties.contains(&property_id_3));
    }

    #[ink::test]
    fn test_get_owner_properties_empty_for_new_owner() {
        let accounts = default_accounts();
        let contract = PropertyRegistry::new();

        let bob_properties = contract.get_owner_properties(accounts.bob);
        assert_eq!(bob_properties.len(), 0);
    }

    #[ink::test]
    fn test_property_count_returns_correct_value() {
        let accounts = default_accounts();
        set_caller(accounts.alice);

        let mut contract = PropertyRegistry::new();
        assert_eq!(contract.property_count(), 0);

        contract
            .register_property(create_sample_metadata())
            .expect("Failed to register property");
        assert_eq!(contract.property_count(), 1);

        contract
            .register_property(create_sample_metadata())
            .expect("Failed to register property");
        assert_eq!(contract.property_count(), 2);
    }

    #[ink::test]
    fn test_ownership_verification_after_multiple_transfers() {
        let accounts = default_accounts();
        set_caller(accounts.alice);

        let mut contract = PropertyRegistry::new();
        let property_id = contract
            .register_property(create_sample_metadata())
            .expect("Failed to register property");

        // Transfer alice -> bob
        set_caller(accounts.alice);
        assert!(contract
            .transfer_property(property_id, accounts.bob)
            .is_ok());
        assert_eq!(
            contract.get_property(property_id).unwrap().owner,
            accounts.bob
        );

        // Transfer bob -> charlie
        set_caller(accounts.bob);
        assert!(contract
            .transfer_property(property_id, accounts.charlie)
            .is_ok());
        assert_eq!(
            contract.get_property(property_id).unwrap().owner,
            accounts.charlie
        );
    }

    #[ink::test]
    fn test_metadata_preserved_after_transfer() {
        let accounts = default_accounts();
        set_caller(accounts.alice);

        let mut contract = PropertyRegistry::new();
        let original_metadata = create_custom_metadata(
            "789 Pine St",
            3000,
            "Original legal description",
            3000000,
            "https://ipfs.io/original",
        );

        let property_id = contract
            .register_property(original_metadata.clone())
            .expect("Failed to register property");

        // Transfer to bob
        set_caller(accounts.alice);
        assert!(contract
            .transfer_property(property_id, accounts.bob)
            .is_ok());

        // Verify metadata is unchanged
        let property = contract.get_property(property_id).unwrap();
        assert_eq!(property.metadata, original_metadata);
    }

    // ============================================================================
    // EDGE CASES
    // ============================================================================

    #[ink::test]
    fn test_register_property_with_max_size_rejected() {
        let accounts = default_accounts();
        set_caller(accounts.alice);

        let mut contract = PropertyRegistry::new();
        let metadata = create_custom_metadata(
            "Max size property",
            u64::MAX,
            "Maximum size property",
            u128::MAX,
            "https://ipfs.io/max",
        );

        // Size exceeds MAX_PROPERTY_SIZE, should be rejected
        assert_eq!(
            contract.register_property(metadata),
            Err(Error::ValueOutOfBounds)
        );
    }

    #[ink::test]
    fn test_register_property_with_zero_values_rejected() {
        let accounts = default_accounts();
        set_caller(accounts.alice);

        let mut contract = PropertyRegistry::new();
        let metadata = create_custom_metadata(
            "Zero value property",
            0,
            "Zero size property",
            0,
            "https://ipfs.io/zero",
        );

        // Size and valuation below minimums, should be rejected
        assert_eq!(
            contract.register_property(metadata),
            Err(Error::ValueOutOfBounds)
        );
    }

    #[ink::test]
    fn test_register_property_with_empty_strings_rejected() {
        let accounts = default_accounts();
        set_caller(accounts.alice);

        let mut contract = PropertyRegistry::new();
        let metadata = create_custom_metadata("", 1000, "", 1000000, "");

        // Empty location and legal_description should be rejected
        assert_eq!(
            contract.register_property(metadata),
            Err(Error::InvalidMetadata)
        );
    }

    #[ink::test]
    fn test_get_nonexistent_property_returns_none() {
        let contract = PropertyRegistry::new();
        assert_eq!(contract.get_property(0), None);
        assert_eq!(contract.get_property(1), None);
        assert_eq!(contract.get_property(999), None);
        assert_eq!(contract.get_property(u64::MAX), None);
    }

    #[ink::test]
    fn test_transfer_nonexistent_property_fails() {
        let accounts = default_accounts();
        set_caller(accounts.alice);

        let mut contract = PropertyRegistry::new();

        assert_eq!(
            contract.transfer_property(999, accounts.bob),
            Err(Error::PropertyNotFound)
        );
    }

    #[ink::test]
    fn test_transfer_property_to_self_rejected() {
        let accounts = default_accounts();
        set_caller(accounts.alice);

        let mut contract = PropertyRegistry::new();
        let property_id = contract
            .register_property(create_sample_metadata())
            .expect("Failed to register property");

        // Transfer to self should be rejected
        set_caller(accounts.alice);
        assert_eq!(
            contract.transfer_property(property_id, accounts.alice),
            Err(Error::SelfTransferNotAllowed)
        );

        // Property should still be owned by alice
        let property = contract.get_property(property_id).unwrap();
        assert_eq!(property.owner, accounts.alice);
    }

    #[ink::test]
    fn test_property_id_sequence() {
        let accounts = default_accounts();
        set_caller(accounts.alice);

        let mut contract = PropertyRegistry::new();

        // Register properties and verify sequential IDs
        for i in 1..=10 {
            let property_id = contract
                .register_property(create_sample_metadata())
                .expect("Failed to register property");
            assert_eq!(property_id, i);
            assert_eq!(contract.property_count(), i);
        }
    }

    // ============================================================================
    // ERROR HANDLING
    // ============================================================================

    #[ink::test]
    fn test_transfer_property_unauthorized_fails() {
        let accounts = default_accounts();
        set_caller(accounts.alice);

        let mut contract = PropertyRegistry::new();
        let property_id = contract
            .register_property(create_sample_metadata())
            .expect("Failed to register property");

        // Try to transfer as charlie (not owner)
        set_caller(accounts.charlie);
        assert_eq!(
            contract.transfer_property(property_id, accounts.bob),
            Err(Error::Unauthorized)
        );

        // Verify ownership unchanged
        let property = contract.get_property(property_id).unwrap();
        assert_eq!(property.owner, accounts.alice);
    }

    #[ink::test]
    fn test_transfer_property_after_already_transferred() {
        let accounts = default_accounts();
        set_caller(accounts.alice);

        let mut contract = PropertyRegistry::new();
        let property_id = contract
            .register_property(create_sample_metadata())
            .expect("Failed to register property");

        // Transfer to bob
        set_caller(accounts.alice);
        assert!(contract
            .transfer_property(property_id, accounts.bob)
            .is_ok());

        // Try to transfer again as alice (no longer owner)
        set_caller(accounts.alice);
        assert_eq!(
            contract.transfer_property(property_id, accounts.charlie),
            Err(Error::Unauthorized)
        );

        // Verify bob still owns it
        let property = contract.get_property(property_id).unwrap();
        assert_eq!(property.owner, accounts.bob);
    }

    #[ink::test]
    fn test_transfer_property_invalid_id() {
        let accounts = default_accounts();
        set_caller(accounts.alice);

        let mut contract = PropertyRegistry::new();

        // Try to transfer non-existent property
        assert_eq!(
            contract.transfer_property(0, accounts.bob),
            Err(Error::PropertyNotFound)
        );
        assert_eq!(
            contract.transfer_property(1, accounts.bob),
            Err(Error::PropertyNotFound)
        );
        assert_eq!(
            contract.transfer_property(u64::MAX, accounts.bob),
            Err(Error::PropertyNotFound)
        );
    }

    #[ink::test]
    fn test_register_property_with_special_characters() {
        let accounts = default_accounts();
        set_caller(accounts.alice);

        let mut contract = PropertyRegistry::new();
        let metadata = create_custom_metadata(
            "123 Main St, Apt #4-B, City, ST 12345-6789",
            1000,
            "Legal description with \"quotes\" and 'apostrophes'",
            1000000,
            "https://example.com/docs?param=value&other=test",
        );

        let property_id = contract
            .register_property(metadata.clone())
            .expect("Failed to register property with special characters");

        let property = contract.get_property(property_id).unwrap();
        assert_eq!(
            property.metadata.location,
            "123 Main St, Apt #4-B, City, ST 12345-6789"
        );
        assert_eq!(
            property.metadata.legal_description,
            "Legal description with \"quotes\" and 'apostrophes'"
        );
        assert_eq!(
            property.metadata.documents_url,
            "https://example.com/docs?param=value&other=test"
        );
    }

    #[ink::test]
    fn test_register_property_with_unicode_characters() {
        let accounts = default_accounts();
        set_caller(accounts.alice);

        let mut contract = PropertyRegistry::new();
        let metadata = create_custom_metadata(
            "123 Main St, 城市, 州 12345",
            1000,
            "Legal description with émojis 🏠 and unicode 中文",
            1000000,
            "https://example.com/docs",
        );

        let property_id = contract
            .register_property(metadata.clone())
            .expect("Failed to register property with unicode");

        let property = contract.get_property(property_id).unwrap();
        assert_eq!(property.metadata.location, "123 Main St, 城市, 州 12345");
        assert_eq!(
            property.metadata.legal_description,
            "Legal description with émojis 🏠 and unicode 中文"
        );
    }

    // ============================================================================
    // PERFORMANCE TESTS
    // ============================================================================

    #[ink::test]
    fn test_bulk_property_registration() {
        let accounts = default_accounts();
        set_caller(accounts.alice);

        let mut contract = PropertyRegistry::new();
        let count = 50;

        // Register multiple properties in bulk
        for i in 1..=count {
            let property_id = contract
                .register_property(create_sample_metadata())
                .expect("Failed to register property");
            assert_eq!(property_id, i);
        }

        assert_eq!(contract.property_count(), count);

        // Verify all properties are accessible
        for i in 1..=count {
            let property = contract.get_property(i);
            assert!(property.is_some());
            let prop = property.unwrap();
            assert_eq!(prop.id, i);
            assert_eq!(prop.owner, accounts.alice);
        }
    }

    #[ink::test]
    fn test_bulk_property_transfer() {
        let accounts = default_accounts();
        set_caller(accounts.alice);

        let mut contract = PropertyRegistry::new();
        let count = 20;

        // Register properties
        let mut property_ids = Vec::new();
        for _ in 0..count {
            let property_id = contract
                .register_property(create_sample_metadata())
                .expect("Failed to register property");
            property_ids.push(property_id);
        }

        // Transfer all to bob
        set_caller(accounts.alice);
        for property_id in &property_ids {
            assert!(contract
                .transfer_property(*property_id, accounts.bob)
                .is_ok());
        }

        // Verify all transferred
        let bob_properties = contract.get_owner_properties(accounts.bob);
        assert_eq!(bob_properties.len(), count);

        for property_id in &property_ids {
            let property = contract.get_property(*property_id).unwrap();
            assert_eq!(property.owner, accounts.bob);
        }
    }

    #[ink::test]
    fn test_get_owner_properties_large_list() {
        let accounts = default_accounts();
        set_caller(accounts.alice);

        let mut contract = PropertyRegistry::new();
        let count = 50;

        // Register many properties for alice
        for _ in 0..count {
            contract
                .register_property(create_sample_metadata())
                .expect("Failed to register property");
        }

        // Get all properties
        let alice_properties = contract.get_owner_properties(accounts.alice);
        assert_eq!(alice_properties.len(), count);

        // Verify all property IDs are unique
        let mut seen = std::collections::HashSet::new();
        for property_id in &alice_properties {
            assert!(!seen.contains(property_id));
            seen.insert(*property_id);
        }
    }

    #[ink::test]
    fn test_property_count_accuracy_under_load() {
        let accounts = default_accounts();
        set_caller(accounts.alice);

        let mut contract = PropertyRegistry::new();
        let count = 100;

        // Register many properties
        for i in 1..=count {
            contract
                .register_property(create_sample_metadata())
                .expect("Failed to register property");
            assert_eq!(contract.property_count(), i);
        }

        assert_eq!(contract.property_count(), count);
    }

    // ============================================================================
    // ADDITIONAL EDGE CASES
    // ============================================================================

    #[ink::test]
    fn test_property_registered_at_timestamp() {
        let accounts = default_accounts();
        set_caller(accounts.alice);

        let mut contract = PropertyRegistry::new();

        // Set a known block timestamp
        ink::env::test::set_block_timestamp::<ink::env::DefaultEnvironment>(1000);

        let property_id = contract
            .register_property(create_sample_metadata())
            .expect("Failed to register property");

        let property = contract.get_property(property_id).unwrap();
        assert_eq!(property.registered_at, 1000);
    }

    #[ink::test]
    fn test_multiple_transfers_same_property() {
        let accounts = default_accounts();
        set_caller(accounts.alice);

        let mut contract = PropertyRegistry::new();
        let property_id = contract
            .register_property(create_sample_metadata())
            .expect("Failed to register property");

        // Transfer multiple times
        set_caller(accounts.alice);
        assert!(contract
            .transfer_property(property_id, accounts.bob)
            .is_ok());

        set_caller(accounts.bob);
        assert!(contract
            .transfer_property(property_id, accounts.charlie)
            .is_ok());

        set_caller(accounts.charlie);
        assert!(contract
            .transfer_property(property_id, accounts.alice)
            .is_ok());

        // Should be back with alice
        let property = contract.get_property(property_id).unwrap();
        assert_eq!(property.owner, accounts.alice);
    }

    #[ink::test]
    fn test_owner_properties_after_transfer_out() {
        let accounts = default_accounts();
        set_caller(accounts.alice);

        let mut contract = PropertyRegistry::new();

        // Register multiple properties
        let property_id_1 = contract
            .register_property(create_sample_metadata())
            .expect("Failed to register property");
        let property_id_2 = contract
            .register_property(create_sample_metadata())
            .expect("Failed to register property");
        let property_id_3 = contract
            .register_property(create_sample_metadata())
            .expect("Failed to register property");

        // Transfer one property out
        set_caller(accounts.alice);
        assert!(contract
            .transfer_property(property_id_2, accounts.bob)
            .is_ok());

        // Alice should only have properties 1 and 3
        let alice_properties = contract.get_owner_properties(accounts.alice);
        assert_eq!(alice_properties.len(), 2);
        assert!(alice_properties.contains(&property_id_1));
        assert!(!alice_properties.contains(&property_id_2));
        assert!(alice_properties.contains(&property_id_3));

        // Bob should have property 2
        let bob_properties = contract.get_owner_properties(accounts.bob);
        assert_eq!(bob_properties.len(), 1);
        assert_eq!(bob_properties[0], property_id_2);
    }

    #[ink::test]
    fn test_property_metadata_immutability() {
        let accounts = default_accounts();
        set_caller(accounts.alice);

        let mut contract = PropertyRegistry::new();
        let original_metadata = create_custom_metadata(
            "Original Location",
            1000,
            "Original Description",
            1000000,
            "https://original.com",
        );

        let property_id = contract
            .register_property(original_metadata.clone())
            .expect("Failed to register property");

        // Transfer property
        set_caller(accounts.alice);
        assert!(contract
            .transfer_property(property_id, accounts.bob)
            .is_ok());

        // Metadata should remain unchanged
        let property = contract.get_property(property_id).unwrap();
        assert_eq!(property.metadata.location, "Original Location");
        assert_eq!(property.metadata.size, 1000);
        assert_eq!(property.metadata.legal_description, "Original Description");
        assert_eq!(property.metadata.valuation, 1000000);
        assert_eq!(property.metadata.documents_url, "https://original.com");
    }

    #[ink::test]
    fn test_default_implementation() {
        let contract = PropertyRegistry::default();
        assert_eq!(contract.property_count(), 0);
    }

    #[ink::test]
    fn test_property_count_consistency_after_transfers() {
        let accounts = default_accounts();
        set_caller(accounts.alice);

        let mut contract = PropertyRegistry::new();

        // Register multiple properties
        let property_id_1 = contract
            .register_property(create_sample_metadata())
            .expect("Failed to register property");
        let property_id_2 = contract
            .register_property(create_sample_metadata())
            .expect("Failed to register property");
        let property_id_3 = contract
            .register_property(create_sample_metadata())
            .expect("Failed to register property");

        assert_eq!(contract.property_count(), 3);

        // Transfer all properties
        set_caller(accounts.alice);
        assert!(contract
            .transfer_property(property_id_1, accounts.bob)
            .is_ok());
        assert!(contract
            .transfer_property(property_id_2, accounts.bob)
            .is_ok());
        assert!(contract
            .transfer_property(property_id_3, accounts.charlie)
            .is_ok());

        // Property count should remain the same
        assert_eq!(contract.property_count(), 3);
    }

    #[ink::test]
    fn test_property_id_uniqueness() {
        let accounts = default_accounts();
        set_caller(accounts.alice);

        let mut contract = PropertyRegistry::new();

        // Register many properties
        let mut property_ids = std::collections::HashSet::new();
        for _ in 0..50 {
            let property_id = contract
                .register_property(create_sample_metadata())
                .expect("Failed to register property");
            assert!(
                property_ids.insert(property_id),
                "Property ID should be unique: {}",
                property_id
            );
        }

        assert_eq!(property_ids.len(), 50);
        assert_eq!(contract.property_count(), 50);
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

        let property_id = contract
            .register_property(metadata.clone())
            .expect("Failed to register");

        let new_metadata = PropertyMetadata {
            location: "123 Main St Updated".to_string(),
            size: 1100,
            legal_description: "Test property updated".to_string(),
            valuation: 1100000,
            documents_url: "https://example.com/docs/new".to_string(),
        };

        assert!(contract
            .update_metadata(property_id, new_metadata.clone())
            .is_ok());

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
        let property_id = contract
            .register_property(metadata)
            .expect("Failed to register");

        set_caller(accounts.bob);
        let new_metadata = PropertyMetadata {
            location: "123 Main St Updated".to_string(),
            size: 1100,
            legal_description: "Test property updated".to_string(),
            valuation: 1100000,
            documents_url: "https://example.com/docs/new".to_string(),
        };
        assert_eq!(
            contract.update_metadata(property_id, new_metadata),
            Err(Error::Unauthorized)
        );
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
        let property_id = contract
            .register_property(metadata)
            .expect("Failed to register");

        // Approve Bob
        assert!(contract.approve(property_id, Some(accounts.bob)).is_ok());
        assert_eq!(contract.get_approved(property_id), Some(accounts.bob));

        // Bob transfers property
        set_caller(accounts.bob);
        assert!(contract
            .transfer_property(property_id, accounts.charlie)
            .is_ok());

        let property = contract.get_property(property_id).unwrap();
        assert_eq!(property.owner, accounts.charlie);

        // Approval should be cleared
        assert_eq!(contract.get_approved(property_id), None);
    }

    // Batch Operations Tests

    #[ink::test]
    fn batch_register_properties_works() {
        let accounts = default_accounts();
        set_caller(accounts.alice);
        let mut contract = PropertyRegistry::new();

        let properties = vec![
            PropertyMetadata {
                location: "Property 1".to_string(),
                size: 1000,
                legal_description: "Test property 1".to_string(),
                valuation: 100000,
                documents_url: "https://example.com/docs1".to_string(),
            },
            PropertyMetadata {
                location: "Property 2".to_string(),
                size: 1500,
                legal_description: "Test property 2".to_string(),
                valuation: 150000,
                documents_url: "https://example.com/docs2".to_string(),
            },
            PropertyMetadata {
                location: "Property 3".to_string(),
                size: 2000,
                legal_description: "Test property 3".to_string(),
                valuation: 200000,
                documents_url: "https://example.com/docs3".to_string(),
            },
        ];

        let property_ids = contract
            .batch_register_properties(properties)
            .expect("Failed to batch register")
            .successes;
        assert_eq!(property_ids.len(), 3);
        assert_eq!(property_ids, vec![1, 2, 3]);
        assert_eq!(contract.property_count(), 3);

        // Verify all properties were registered correctly
        for (i, &property_id) in property_ids.iter().enumerate() {
            let property = contract.get_property(property_id).unwrap();
            assert_eq!(property.owner, accounts.alice);
            assert_eq!(property.id, property_id);
            assert_eq!(property.metadata.location, format!("Property {}", i + 1));
        }

        // Verify owner has all properties
        let owner_properties = contract.get_owner_properties(accounts.alice);
        assert_eq!(owner_properties.len(), 3);
        assert!(owner_properties.contains(&1));
        assert!(owner_properties.contains(&2));
        assert!(owner_properties.contains(&3));
    }

    #[ink::test]
    fn batch_transfer_properties_works() {
        let accounts = default_accounts();
        set_caller(accounts.alice);
        let mut contract = PropertyRegistry::new();

        // Register multiple properties
        let properties = vec![
            PropertyMetadata {
                location: "Property 1".to_string(),
                size: 1000,
                legal_description: "Test property 1".to_string(),
                valuation: 100000,
                documents_url: "https://example.com/docs1".to_string(),
            },
            PropertyMetadata {
                location: "Property 2".to_string(),
                size: 1500,
                legal_description: "Test property 2".to_string(),
                valuation: 150000,
                documents_url: "https://example.com/docs2".to_string(),
            },
        ];

        let property_ids = contract
            .batch_register_properties(properties)
            .expect("Failed to batch register")
            .successes;

        // Transfer all properties to Bob
        assert!(contract
            .batch_transfer_properties(property_ids.clone(), accounts.bob)
            .is_ok());

        // Verify all properties were transferred
        for &property_id in &property_ids {
            let property = contract.get_property(property_id).unwrap();
            assert_eq!(property.owner, accounts.bob);
        }

        // Verify Alice has no properties
        let alice_properties = contract.get_owner_properties(accounts.alice);
        assert!(alice_properties.is_empty());

        // Verify Bob has all properties
        let bob_properties = contract.get_owner_properties(accounts.bob);
        assert_eq!(bob_properties.len(), 2);
        assert!(bob_properties.contains(&1));
        assert!(bob_properties.contains(&2));
    }

    #[ink::test]
    fn batch_transfer_properties_size_guard_works() {
        let accounts = default_accounts();
        set_caller(accounts.alice);
        ink::env::test::set_block_timestamp::<ink::env::DefaultEnvironment>(1000);
        let mut contract = PropertyRegistry::new();

        let props = vec![
            create_custom_metadata("Prop 1", 100, "Desc", 100000, "url"),
            create_custom_metadata("Prop 2", 200, "Desc", 200000, "url"),
        ];
        let ids = contract.batch_register_properties(props).unwrap().successes;

        // Set max to 1 after registering
        contract.update_batch_config(1, 1).unwrap();

        assert_eq!(
            contract.batch_transfer_properties(ids, accounts.bob),
            Err(Error::BatchSizeExceeded)
        );
    }

    #[ink::test]
    fn batch_update_metadata_works() {
        let accounts = default_accounts();
        set_caller(accounts.alice);
        let mut contract = PropertyRegistry::new();

        // Register multiple properties
        let properties = vec![
            PropertyMetadata {
                location: "Property 1".to_string(),
                size: 1000,
                legal_description: "Test property 1".to_string(),
                valuation: 100000,
                documents_url: "https://example.com/docs1".to_string(),
            },
            PropertyMetadata {
                location: "Property 2".to_string(),
                size: 1500,
                legal_description: "Test property 2".to_string(),
                valuation: 150000,
                documents_url: "https://example.com/docs2".to_string(),
            },
        ];

        let property_ids = contract
            .batch_register_properties(properties)
            .expect("Failed to batch register")
            .successes;

        // Update metadata for all properties
        let updates = vec![
            (
                property_ids[0],
                PropertyMetadata {
                    location: "Updated Property 1".to_string(),
                    size: 1200,
                    legal_description: "Updated test property 1".to_string(),
                    valuation: 120000,
                    documents_url: "https://example.com/docs1_updated".to_string(),
                },
            ),
            (
                property_ids[1],
                PropertyMetadata {
                    location: "Updated Property 2".to_string(),
                    size: 1700,
                    legal_description: "Updated test property 2".to_string(),
                    valuation: 170000,
                    documents_url: "https://example.com/docs2_updated".to_string(),
                },
            ),
        ];

        let result = contract.batch_update_metadata(updates).unwrap();
        assert!(result.failures.is_empty());

        // Verify updates
        let property1 = contract.get_property(property_ids[0]).unwrap();
        assert_eq!(property1.metadata.location, "Updated Property 1");
        assert_eq!(property1.metadata.size, 1200);
        assert_eq!(property1.metadata.valuation, 120000);

        let property2 = contract.get_property(property_ids[1]).unwrap();
        assert_eq!(property2.metadata.location, "Updated Property 2");
        assert_eq!(property2.metadata.size, 1700);
        assert_eq!(property2.metadata.valuation, 170000);
    }

    #[ink::test]
    fn batch_transfer_properties_to_multiple_works() {
        let accounts = default_accounts();
        set_caller(accounts.alice);
        let mut contract = PropertyRegistry::new();

        // Register multiple properties
        let properties = vec![
            PropertyMetadata {
                location: "Property 1".to_string(),
                size: 1000,
                legal_description: "Test property 1".to_string(),
                valuation: 100000,
                documents_url: "https://example.com/docs1".to_string(),
            },
            PropertyMetadata {
                location: "Property 2".to_string(),
                size: 1500,
                legal_description: "Test property 2".to_string(),
                valuation: 150000,
                documents_url: "https://example.com/docs2".to_string(),
            },
            PropertyMetadata {
                location: "Property 3".to_string(),
                size: 2000,
                legal_description: "Test property 3".to_string(),
                valuation: 200000,
                documents_url: "https://example.com/docs3".to_string(),
            },
        ];

        let property_ids = contract
            .batch_register_properties(properties)
            .expect("Failed to batch register")
            .successes;

        // Transfer properties to different recipients
        let transfers = vec![
            (property_ids[0], accounts.bob),
            (property_ids[1], accounts.charlie),
            (property_ids[2], accounts.django),
        ];

        assert!(contract
            .batch_transfer_properties_to_multiple(transfers)
            .is_ok());

        // Verify transfers
        let property1 = contract.get_property(property_ids[0]).unwrap();
        assert_eq!(property1.owner, accounts.bob);

        let property2 = contract.get_property(property_ids[1]).unwrap();
        assert_eq!(property2.owner, accounts.charlie);

        let property3 = contract.get_property(property_ids[2]).unwrap();
        assert_eq!(property3.owner, accounts.django);

        // Verify Alice has no properties
        let alice_properties = contract.get_owner_properties(accounts.alice);
        assert!(alice_properties.is_empty());
    }

    // Portfolio Management Tests

    #[ink::test]
    fn get_portfolio_summary_works() {
        let accounts = default_accounts();
        set_caller(accounts.alice);
        let mut contract = PropertyRegistry::new();

        // Register multiple properties
        let properties = vec![
            PropertyMetadata {
                location: "Property 1".to_string(),
                size: 1000,
                legal_description: "Test property 1".to_string(),
                valuation: 100000,
                documents_url: "https://example.com/docs1".to_string(),
            },
            PropertyMetadata {
                location: "Property 2".to_string(),
                size: 1500,
                legal_description: "Test property 2".to_string(),
                valuation: 150000,
                documents_url: "https://example.com/docs2".to_string(),
            },
        ];

        contract
            .batch_register_properties(properties)
            .expect("Failed to batch register");

        // Get portfolio summary
        let summary = contract.get_portfolio_summary(accounts.alice);
        assert_eq!(summary.property_count, 2);
        assert_eq!(summary.total_valuation, 250000);
        assert_eq!(summary.average_valuation, 125000);
        assert_eq!(summary.total_size, 2500);
        assert_eq!(summary.average_size, 1250);
    }

    #[ink::test]
    fn get_portfolio_details_works() {
        let accounts = default_accounts();
        set_caller(accounts.alice);
        let mut contract = PropertyRegistry::new();

        // Register multiple properties
        let properties = vec![
            PropertyMetadata {
                location: "Property 1".to_string(),
                size: 1000,
                legal_description: "Test property 1".to_string(),
                valuation: 100000,
                documents_url: "https://example.com/docs1".to_string(),
            },
            PropertyMetadata {
                location: "Property 2".to_string(),
                size: 1500,
                legal_description: "Test property 2".to_string(),
                valuation: 150000,
                documents_url: "https://example.com/docs2".to_string(),
            },
        ];

        let property_ids = contract
            .batch_register_properties(properties)
            .expect("Failed to batch register")
            .successes;

        // Get portfolio details
        let details = contract.get_portfolio_details(accounts.alice);
        assert_eq!(details.owner, accounts.alice);
        assert_eq!(details.total_count, 2);
        assert_eq!(details.properties.len(), 2);

        // Verify property details
        let prop1 = &details.properties[0];
        assert_eq!(prop1.id, property_ids[0]);
        assert_eq!(prop1.location, "Property 1");
        assert_eq!(prop1.size, 1000);
        assert_eq!(prop1.valuation, 100000);

        let prop2 = &details.properties[1];
        assert_eq!(prop2.id, property_ids[1]);
        assert_eq!(prop2.location, "Property 2");
        assert_eq!(prop2.size, 1500);
        assert_eq!(prop2.valuation, 150000);
    }

    // Analytics Tests

    #[ink::test]
    fn get_global_analytics_works() {
        let accounts = default_accounts();
        set_caller(accounts.alice);
        let mut contract = PropertyRegistry::new();

        // Register properties for Alice
        let alice_properties = vec![PropertyMetadata {
            location: "Alice Property 1".to_string(),
            size: 1000,
            legal_description: "Test property".to_string(),
            valuation: 100000,
            documents_url: "https://example.com/docs".to_string(),
        }];
        contract
            .batch_register_properties(alice_properties)
            .expect("Failed to register Alice properties");

        // Register properties for Bob
        set_caller(accounts.bob);
        let bob_properties = vec![
            PropertyMetadata {
                location: "Bob Property 1".to_string(),
                size: 1500,
                legal_description: "Test property".to_string(),
                valuation: 150000,
                documents_url: "https://example.com/docs".to_string(),
            },
            PropertyMetadata {
                location: "Bob Property 2".to_string(),
                size: 2000,
                legal_description: "Test property".to_string(),
                valuation: 200000,
                documents_url: "https://example.com/docs".to_string(),
            },
        ];
        contract
            .batch_register_properties(bob_properties)
            .expect("Failed to register Bob properties");

        // Get global analytics
        let analytics = contract.get_global_analytics();
        assert_eq!(analytics.total_properties, 3);
        assert_eq!(analytics.total_valuation, 450000);
        assert_eq!(analytics.average_valuation, 150000);
        assert_eq!(analytics.total_size, 4500);
        assert_eq!(analytics.average_size, 1500);
        assert_eq!(analytics.unique_owners, 2);
    }

    #[ink::test]
    fn get_properties_by_price_range_works() {
        let accounts = default_accounts();
        set_caller(accounts.alice);
        let mut contract = PropertyRegistry::new();

        // Register properties with different valuations
        let properties = vec![
            PropertyMetadata {
                location: "Cheap Property".to_string(),
                size: 1000,
                legal_description: "Test property".to_string(),
                valuation: 50000,
                documents_url: "https://example.com/docs".to_string(),
            },
            PropertyMetadata {
                location: "Medium Property".to_string(),
                size: 1500,
                legal_description: "Test property".to_string(),
                valuation: 150000,
                documents_url: "https://example.com/docs".to_string(),
            },
            PropertyMetadata {
                location: "Expensive Property".to_string(),
                size: 2000,
                legal_description: "Test property".to_string(),
                valuation: 250000,
                documents_url: "https://example.com/docs".to_string(),
            },
        ];

        contract
            .batch_register_properties(properties)
            .expect("Failed to batch register");

        // Get properties in medium price range
        let medium_properties = contract.get_properties_by_price_range(100000, 200000).unwrap();
        assert_eq!(medium_properties.len(), 1);
        assert_eq!(medium_properties[0], 2); // Medium Property

        // Get properties in high price range
        let high_properties = contract.get_properties_by_price_range(200000, 300000).unwrap();
        assert_eq!(high_properties.len(), 1);
        assert_eq!(high_properties[0], 3); // Expensive Property

        // Get all properties
        let all_properties = contract.get_properties_by_price_range(0, 300000).unwrap();
        assert_eq!(all_properties.len(), 3);
        assert!(all_properties.contains(&1));
        assert!(all_properties.contains(&2));
        assert!(all_properties.contains(&3));
    }

    #[ink::test]
    fn get_properties_by_size_range_works() {
        let accounts = default_accounts();
        set_caller(accounts.alice);
        let mut contract = PropertyRegistry::new();

        // Register properties with different sizes
        let properties = vec![
            PropertyMetadata {
                location: "Small Property".to_string(),
                size: 500,
                legal_description: "Test property".to_string(),
                valuation: 100000,
                documents_url: "https://example.com/docs".to_string(),
            },
            PropertyMetadata {
                location: "Medium Property".to_string(),
                size: 1500,
                legal_description: "Test property".to_string(),
                valuation: 150000,
                documents_url: "https://example.com/docs".to_string(),
            },
            PropertyMetadata {
                location: "Large Property".to_string(),
                size: 2500,
                legal_description: "Test property".to_string(),
                valuation: 200000,
                documents_url: "https://example.com/docs".to_string(),
            },
        ];

        contract
            .batch_register_properties(properties)
            .expect("Failed to batch register");

        // Get properties in medium size range
        let medium_properties = contract.get_properties_by_size_range(1000, 2000).unwrap();
        assert_eq!(medium_properties.len(), 1);
        assert_eq!(medium_properties[0], 2); // Medium Property

        // Get properties in large size range
        let large_properties = contract.get_properties_by_size_range(2000, 3000).unwrap();
        assert_eq!(large_properties.len(), 1);
        assert_eq!(large_properties[0], 3); // Large Property

        // Get all properties
        let all_properties = contract.get_properties_by_size_range(0, 3000).unwrap();
        assert_eq!(all_properties.len(), 3);
        assert!(all_properties.contains(&1));
        assert!(all_properties.contains(&2));
        assert!(all_properties.contains(&3));
    }

    // Gas Monitoring Tests

    #[ink::test]
    fn gas_metrics_tracking_works() {
        let accounts = default_accounts();
        set_caller(accounts.alice);
        let mut contract = PropertyRegistry::new();

        // Perform some operations
        let metadata = PropertyMetadata {
            location: "Test Property".to_string(),
            size: 1000,
            legal_description: "Test property".to_string(),
            valuation: 100000,
            documents_url: "https://example.com/docs".to_string(),
        };

        contract
            .register_property(metadata)
            .expect("Failed to register");

        // Get gas metrics
        let metrics = contract.get_gas_metrics();
        assert_eq!(metrics.total_operations, 1);
        assert_eq!(metrics.last_operation_gas, 10000);
        assert_eq!(metrics.average_operation_gas, 10000);
        assert_eq!(metrics.min_gas_used, 10000);
        assert_eq!(metrics.max_gas_used, 10000);
    }

    #[ink::test]
    fn performance_recommendations_works() {
        let accounts = default_accounts();
        set_caller(accounts.alice);
        let mut contract = PropertyRegistry::new();

        // Perform multiple operations to generate recommendations
        let metadata = PropertyMetadata {
            location: "Test Property".to_string(),
            size: 1000,
            legal_description: "Test property".to_string(),
            valuation: 100000,
            documents_url: "https://example.com/docs".to_string(),
        };

        // Register multiple properties
        for _ in 0..5 {
            contract
                .register_property(metadata.clone())
                .expect("Failed to register");
        }

        // Get performance recommendations
        let recommendations = contract.get_performance_recommendations();
        assert!(!recommendations.is_empty());

        // Should contain general recommendations
        let recommendation_strings: Vec<&str> =
            recommendations.iter().map(|s| s.as_str()).collect();
        assert!(recommendation_strings
            .contains(&"Use batch operations for multiple property transfers"));
        assert!(recommendation_strings
            .contains(&"Prefer portfolio analytics over individual property queries"));
        assert!(
            recommendation_strings.contains(&"Consider off-chain indexing for complex analytics")
        );
    }

    // Error Cases Tests

    #[ink::test]
    fn batch_transfer_unauthorized_fails() {
        let accounts = default_accounts();
        set_caller(accounts.alice);
        let mut contract = PropertyRegistry::new();

        // Register properties
        let properties = vec![PropertyMetadata {
            location: "Property 1".to_string(),
            size: 1000,
            legal_description: "Test property".to_string(),
            valuation: 100000,
            documents_url: "https://example.com/docs".to_string(),
        }];

        let property_ids = contract
            .batch_register_properties(properties)
            .expect("Failed to batch register")
            .successes;

        // Try to transfer as unauthorized user
        set_caller(accounts.bob);
        assert_eq!(
            contract.batch_transfer_properties(property_ids, accounts.charlie),
            Err(Error::Unauthorized)
        );
    }

    #[ink::test]
    fn batch_update_metadata_unauthorized_fails() {
        let accounts = default_accounts();
        set_caller(accounts.alice);
        let mut contract = PropertyRegistry::new();

        // Register properties
        let properties = vec![PropertyMetadata {
            location: "Property 1".to_string(),
            size: 1000,
            legal_description: "Test property".to_string(),
            valuation: 100000,
            documents_url: "https://example.com/docs".to_string(),
        }];

        let property_ids = contract
            .batch_register_properties(properties)
            .expect("Failed to batch register")
            .successes;

        // Try to update as unauthorized user
        set_caller(accounts.bob);
        let updates = vec![(
            property_ids[0],
            PropertyMetadata {
                location: "Updated Property".to_string(),
                size: 1200,
                legal_description: "Updated test property".to_string(),
                valuation: 120000,
                documents_url: "https://example.com/docs_updated".to_string(),
            },
        )];

        let result = contract.batch_update_metadata(updates).unwrap();
        assert_eq!(result.failures.len(), 1);
        assert_eq!(result.failures[0].error, Error::Unauthorized);
        assert!(result.successes.is_empty());
    }

    #[ink::test]
    fn batch_operations_with_empty_input_rejected() {
        let accounts = default_accounts();
        set_caller(accounts.alice);
        let mut contract = PropertyRegistry::new();

        // Empty batch register should be rejected
        let empty_properties: Vec<PropertyMetadata> = vec![];
        assert_eq!(
            contract.batch_register_properties(empty_properties),
            Err(Error::ValueOutOfBounds)
        );

        // Empty batch transfer should be rejected
        let empty_transfers: Vec<u64> = vec![];
        assert_eq!(
            contract.batch_transfer_properties(empty_transfers, accounts.bob),
            Err(Error::ValueOutOfBounds)
        );

        // Empty batch update should be rejected
        let empty_updates: Vec<(u64, PropertyMetadata)> = vec![];
        assert_eq!(
            contract.batch_update_metadata(empty_updates),
            Err(Error::ValueOutOfBounds)
        );

        // Empty batch transfer to multiple should be rejected
        let empty_multiple_transfers: Vec<(u64, AccountId)> = vec![];
        assert_eq!(
            contract.batch_transfer_properties_to_multiple(empty_multiple_transfers),
            Err(Error::ValueOutOfBounds)
        );
    }

    // ============================================================================
    // BADGE SYSTEM TESTS
    // ============================================================================

    #[ink::test]
    fn test_badge_verifier_management() {
        let accounts = default_accounts();
        set_caller(accounts.alice);
        let mut contract = PropertyRegistry::new();
        assert!(contract.set_verifier(accounts.bob, true).is_ok());
        assert!(contract.is_verifier(accounts.bob));
        assert!(contract.set_verifier(accounts.bob, false).is_ok());
        assert!(!contract.is_verifier(accounts.bob));
        set_caller(accounts.charlie);
        assert_eq!(
            contract.set_verifier(accounts.bob, true),
            Err(Error::Unauthorized)
        );
    }

    #[ink::test]
    fn test_badge_issuance_and_query() {
        use crate::propchain_contracts::BadgeType;
        let accounts = default_accounts();
        set_caller(accounts.alice);
        let mut contract = PropertyRegistry::new();
        let property_id = contract
            .register_property(create_sample_metadata())
            .expect("Failed to register property");
        assert!(contract.set_verifier(accounts.bob, true).is_ok());
        set_caller(accounts.bob);
        assert!(contract
            .issue_badge(
                property_id,
                BadgeType::DocumentVerification,
                None,
                "https://metadata.example.com/badge.json".to_string()
            )
            .is_ok());
        assert!(contract.has_badge(property_id, BadgeType::DocumentVerification));
        let badge = contract.get_badge(property_id, BadgeType::DocumentVerification);
        assert!(badge.is_some());
        assert_eq!(badge.unwrap().issued_by, accounts.bob);
    }

    #[ink::test]
    fn test_verification_request_workflow() {
        use crate::propchain_contracts::BadgeType;
        let accounts = default_accounts();
        set_caller(accounts.alice);
        let mut contract = PropertyRegistry::new();
        let property_id = contract
            .register_property(create_sample_metadata())
            .expect("Failed to register property");
        let request_id = contract
            .request_verification(
                property_id,
                BadgeType::LegalCompliance,
                "https://evidence.example.com/docs.pdf".to_string(),
            )
            .expect("Failed to request verification");
        assert_eq!(request_id, 1);
        assert!(contract.set_verifier(accounts.bob, true).is_ok());
        set_caller(accounts.bob);
        assert!(contract
            .review_verification(
                request_id,
                true,
                Some(1000000),
                "https://metadata.example.com/badge.json".to_string()
            )
            .is_ok());
        assert!(contract.has_badge(property_id, BadgeType::LegalCompliance));
    }

    #[ink::test]
    fn test_badge_revocation() {
        use crate::propchain_contracts::BadgeType;
        let accounts = default_accounts();
        set_caller(accounts.alice);
        let mut contract = PropertyRegistry::new();
        let property_id = contract
            .register_property(create_sample_metadata())
            .expect("Failed to register property");
        assert!(contract.set_verifier(accounts.bob, true).is_ok());
        set_caller(accounts.bob);
        assert!(contract
            .issue_badge(
                property_id,
                BadgeType::OwnerVerification,
                None,
                "https://metadata.example.com/badge.json".to_string()
            )
            .is_ok());
        assert!(contract
            .revoke_badge(
                property_id,
                BadgeType::OwnerVerification,
                "Failed KYC".to_string()
            )
            .is_ok());
        assert!(!contract.has_badge(property_id, BadgeType::OwnerVerification));
        let badge = contract.get_badge(property_id, BadgeType::OwnerVerification);
        assert!(badge.is_some());
        assert!(badge.unwrap().revoked);
    }

    #[ink::test]
    fn test_badge_appeal_process() {
        use crate::propchain_contracts::BadgeType;
        let accounts = default_accounts();
        set_caller(accounts.alice);
        let mut contract = PropertyRegistry::new();
        let property_id = contract
            .register_property(create_sample_metadata())
            .expect("Failed to register property");
        assert!(contract.set_verifier(accounts.bob, true).is_ok());
        set_caller(accounts.bob);
        assert!(contract
            .issue_badge(
                property_id,
                BadgeType::DocumentVerification,
                None,
                "https://metadata.example.com/badge.json".to_string()
            )
            .is_ok());
        assert!(contract
            .revoke_badge(
                property_id,
                BadgeType::DocumentVerification,
                "Documents expired".to_string()
            )
            .is_ok());
        set_caller(accounts.alice);
        let appeal_id = contract
            .submit_appeal(
                property_id,
                BadgeType::DocumentVerification,
                "Documents renewed".to_string(),
            )
            .expect("Failed to submit appeal");
        assert_eq!(appeal_id, 1);
        assert!(contract
            .resolve_appeal(appeal_id, true, "Reinstating badge".to_string())
            .is_ok());
        assert!(contract.has_badge(property_id, BadgeType::DocumentVerification));
    }

    // ============================================================================
    // DYNAMIC FEE INTEGRATION (Issue #38)
    // ============================================================================

    #[ink::test]
    fn test_fee_manager_initially_none() {
        let contract = PropertyRegistry::new();
        assert_eq!(contract.get_fee_manager(), None);
    }

    #[ink::test]
    fn test_get_dynamic_fee_without_manager_returns_zero() {
        let contract = PropertyRegistry::new();
        assert_eq!(contract.get_dynamic_fee(FeeOperation::RegisterProperty), 0);
        assert_eq!(contract.get_dynamic_fee(FeeOperation::TransferProperty), 0);
    }

    #[ink::test]
    fn test_set_fee_manager_admin_only() {
        let accounts = default_accounts();
        set_caller(accounts.alice);
        let mut contract = PropertyRegistry::new();
        let fee_manager_addr = AccountId::from([0x42; 32]);
        assert!(contract.set_fee_manager(Some(fee_manager_addr)).is_ok());
        assert_eq!(contract.get_fee_manager(), Some(fee_manager_addr));

        set_caller(accounts.bob);
        assert!(contract.set_fee_manager(None).is_err());
        assert_eq!(contract.get_fee_manager(), Some(fee_manager_addr));
    }

    #[ink::test]
    fn test_set_fee_manager_clear() {
        let accounts = default_accounts();
        set_caller(accounts.alice);
        let mut contract = PropertyRegistry::new();
        contract
            .set_fee_manager(Some(AccountId::from([0x42; 32])))
            .unwrap();
        assert!(contract.set_fee_manager(None).is_ok());
        assert_eq!(contract.get_fee_manager(), None);
    }

    // ============================================================================
    // COMPLIANCE INTEGRATION (Issue #45)
    // ============================================================================

    #[ink::test]
    fn test_check_account_compliance_without_registry_returns_true() {
        let contract = PropertyRegistry::new();
        let accounts = default_accounts();
        assert_eq!(contract.check_account_compliance(accounts.alice), Ok(true));
        assert_eq!(contract.check_account_compliance(accounts.bob), Ok(true));
    }

    // ============================================================================
    // BATCH CONFIG AND MONITORING TESTS
    // ============================================================================

    #[ink::test]
    fn update_batch_config_works() {
        let accounts = default_accounts();
        set_caller(accounts.alice);
        let mut contract = PropertyRegistry::new();

        // Default config
        let config = contract.get_batch_config();
        assert_eq!(config.max_batch_size, 50);
        assert_eq!(config.max_failure_threshold, 5);

        // Update as admin
        assert!(contract.update_batch_config(100, 10).is_ok());

        let config = contract.get_batch_config();
        assert_eq!(config.max_batch_size, 100);
        assert_eq!(config.max_failure_threshold, 10);
    }

    #[ink::test]
    fn update_batch_config_unauthorized_fails() {
        let accounts = default_accounts();
        set_caller(accounts.alice);
        let mut contract = PropertyRegistry::new();

        // Try as non-admin
        set_caller(accounts.bob);
        assert_eq!(
            contract.update_batch_config(100, 10),
            Err(Error::Unauthorized)
        );
    }

    // ============================================================================
    // INPUT VALIDATION TESTS (Issue #79)
    // ============================================================================

    // -- Zero Address Tests --

    #[ink::test]
    fn test_transfer_to_zero_address_rejected() {
        let accounts = default_accounts();
        set_caller(accounts.alice);
        let mut contract = PropertyRegistry::new();
        let property_id = contract
            .register_property(create_sample_metadata())
            .expect("register");
        let zero = AccountId::from([0u8; 32]);
        assert_eq!(
            contract.transfer_property(property_id, zero),
            Err(Error::ZeroAddress)
        );
    }

    #[ink::test]
    fn update_batch_config_invalid_params_fails() {
        let accounts = default_accounts();
        set_caller(accounts.alice);
        let mut contract = PropertyRegistry::new();

        // max_batch_size = 0
        assert_eq!(
            contract.update_batch_config(0, 5),
            Err(Error::InvalidMetadata)
        );

        // max_batch_size > 200
        assert_eq!(
            contract.update_batch_config(201, 5),
            Err(Error::InvalidMetadata)
        );

        // max_failure_threshold > max_batch_size
        assert_eq!(
            contract.update_batch_config(50, 51),
            Err(Error::InvalidMetadata)
        );

        // max_failure_threshold = 0
        assert_eq!(
            contract.update_batch_config(50, 0),
            Err(Error::InvalidMetadata)
        );
    }

    #[ink::test]
    fn test_change_admin_zero_address_rejected() {
        let accounts = default_accounts();
        set_caller(accounts.alice);
        let mut contract = PropertyRegistry::new();
        let zero = AccountId::from([0u8; 32]);
        assert_eq!(contract.change_admin(zero), Err(Error::ZeroAddress));
    }

    #[ink::test]
    fn test_create_escrow_zero_buyer_rejected() {
        let accounts = default_accounts();
        set_caller(accounts.alice);
        let mut contract = PropertyRegistry::new();
        let property_id = contract
            .register_property(create_sample_metadata())
            .expect("register");
        let zero = AccountId::from([0u8; 32]);
        assert_eq!(
            contract.create_escrow(property_id, zero, 1000),
            Err(Error::ZeroAddress)
        );
    }

    #[ink::test]
    fn test_set_oracle_zero_address_rejected() {
        let accounts = default_accounts();
        set_caller(accounts.alice);
        let mut contract = PropertyRegistry::new();
        let zero = AccountId::from([0u8; 32]);
        assert_eq!(contract.set_oracle(zero), Err(Error::ZeroAddress));
    }

    #[ink::test]
    fn test_grant_role_zero_address_rejected() {
        let accounts = default_accounts();
        set_caller(accounts.alice);
        let mut contract = PropertyRegistry::new();
        let zero = AccountId::from([0u8; 32]);
        assert_eq!(
            contract.grant_role(zero, Role::Verifier),
            Err(Error::ZeroAddress)
        );
    }

    #[ink::test]
    fn test_set_verifier_zero_address_rejected() {
        let accounts = default_accounts();
        set_caller(accounts.alice);
        let mut contract = PropertyRegistry::new();
        let zero = AccountId::from([0u8; 32]);
        assert_eq!(
            contract.set_verifier(zero, true),
            Err(Error::ZeroAddress)
        );
    }

    #[ink::test]
    fn test_set_pause_guardian_zero_address_rejected() {
        let accounts = default_accounts();
        set_caller(accounts.alice);
        let mut contract = PropertyRegistry::new();
        let zero = AccountId::from([0u8; 32]);
        assert_eq!(
            contract.set_pause_guardian(zero, true),
            Err(Error::ZeroAddress)
        );
    }

    #[ink::test]
    fn test_approve_zero_address_rejected() {
        let accounts = default_accounts();
        set_caller(accounts.alice);
        let mut contract = PropertyRegistry::new();
        let property_id = contract
            .register_property(create_sample_metadata())
            .expect("register");
        let zero = AccountId::from([0u8; 32]);
        assert_eq!(
            contract.approve(property_id, Some(zero)),
            Err(Error::ZeroAddress)
        );
    }

    // -- Metadata Validation Tests --

    #[ink::test]
    fn test_register_location_too_long_rejected() {
        let accounts = default_accounts();
        set_caller(accounts.alice);
        let mut contract = PropertyRegistry::new();
        let long_location = "A".repeat(501);
        let metadata = create_custom_metadata(&long_location, 100, "Valid desc", 1000, "https://example.com");
        assert_eq!(
            contract.register_property(metadata),
            Err(Error::StringTooLong)
        );
    }

    #[ink::test]
    fn test_register_legal_desc_too_long_rejected() {
        let accounts = default_accounts();
        set_caller(accounts.alice);
        let mut contract = PropertyRegistry::new();
        let long_desc = "A".repeat(5001);
        let metadata = create_custom_metadata("Valid location", 100, &long_desc, 1000, "https://example.com");
        assert_eq!(
            contract.register_property(metadata),
            Err(Error::StringTooLong)
        );
    }

    #[ink::test]
    fn test_register_size_out_of_bounds_rejected() {
        let accounts = default_accounts();
        set_caller(accounts.alice);
        let mut contract = PropertyRegistry::new();

        // Size = 0 (below minimum)
        let metadata = create_custom_metadata("Valid", 0, "Valid desc", 1000, "https://example.com");
        assert_eq!(
            contract.register_property(metadata),
            Err(Error::ValueOutOfBounds)
        );

        // Size above maximum
        let metadata = create_custom_metadata("Valid", 1_000_000_001, "Valid desc", 1000, "https://example.com");
        assert_eq!(
            contract.register_property(metadata),
            Err(Error::ValueOutOfBounds)
        );
    }

    #[ink::test]
    fn test_register_valuation_below_min_rejected() {
        let accounts = default_accounts();
        set_caller(accounts.alice);
        let mut contract = PropertyRegistry::new();
        let metadata = create_custom_metadata("Valid", 100, "Valid desc", 0, "https://example.com");
        assert_eq!(
            contract.register_property(metadata),
            Err(Error::ValueOutOfBounds)
        );
    }

    // -- Batch Size Tests --

    #[ink::test]
    fn test_batch_register_exceeds_limit_rejected() {
        let accounts = default_accounts();
        set_caller(accounts.alice);
        let mut contract = PropertyRegistry::new();
        let properties: Vec<PropertyMetadata> = (0..51)
            .map(|i| create_custom_metadata(
                &format!("Property {}", i), 100, "Valid desc", 1000, "https://example.com"
            ))
            .collect();
        assert_eq!(
            contract.batch_register_properties(properties),
            Err(Error::BatchSizeExceeded)
        );
    }

    // -- Self-Transfer Tests --

    #[ink::test]
    fn test_approve_self_rejected() {
        let accounts = default_accounts();
        set_caller(accounts.alice);
        let mut contract = PropertyRegistry::new();
        let property_id = contract
            .register_property(create_sample_metadata())
            .expect("register");
        assert_eq!(
            contract.approve(property_id, Some(accounts.alice)),
            Err(Error::SelfTransferNotAllowed)
        );
    }

    // -- String Length Tests --

    #[ink::test]
    fn test_pause_reason_too_long_rejected() {
        let accounts = default_accounts();
        set_caller(accounts.alice);
        let mut contract = PropertyRegistry::new();
        let long_reason = "A".repeat(2001);
        assert_eq!(
            contract.pause_contract(long_reason, None),
            Err(Error::StringTooLong)
        );
    }

    #[ink::test]
    fn test_badge_url_too_long_rejected() {
        use crate::propchain_contracts::BadgeType;
        let accounts = default_accounts();
        set_caller(accounts.alice);
        let mut contract = PropertyRegistry::new();
        let property_id = contract
            .register_property(create_sample_metadata())
            .expect("register");
        let long_url = "https://".to_string() + &"a".repeat(2050);
        assert_eq!(
            contract.issue_badge(property_id, BadgeType::DocumentVerification, None, long_url),
            Err(Error::StringTooLong)
        );
    }

    // -- Numeric Bounds Tests --

    #[ink::test]
    fn test_create_escrow_zero_amount_rejected() {
        let accounts = default_accounts();
        set_caller(accounts.alice);
        let mut contract = PropertyRegistry::new();
        let property_id = contract
            .register_property(create_sample_metadata())
            .expect("register");
        assert_eq!(
            contract.create_escrow(property_id, accounts.bob, 0),
            Err(Error::ValueOutOfBounds)
        );
    }

    // -- Batch Config Size Guard Tests (from main) --

    #[ink::test]
    fn batch_register_properties_size_guard_works() {
        let accounts = default_accounts();
        set_caller(accounts.alice);
        let mut contract = PropertyRegistry::new();

        // Set max batch size to 2
        contract.update_batch_config(2, 1).unwrap();

        let properties = vec![
            create_custom_metadata("Prop 1", 100, "Desc 1", 100000, "url1"),
            create_custom_metadata("Prop 2", 200, "Desc 2", 200000, "url2"),
            create_custom_metadata("Prop 3", 300, "Desc 3", 300000, "url3"),
        ];

        assert_eq!(
            contract.batch_register_properties(properties),
            Err(Error::BatchSizeExceeded)
        );
        assert_eq!(contract.property_count(), 0);
    }

    #[ink::test]
    fn batch_register_properties_partial_success_works() {
        let accounts = default_accounts();
        set_caller(accounts.alice);
        ink::env::test::set_block_timestamp::<ink::env::DefaultEnvironment>(1000);
        let mut contract = PropertyRegistry::new();

        let properties = vec![
            create_custom_metadata("Valid Prop 1", 100, "Desc 1", 100000, "url1"),
            create_custom_metadata("", 200, "Desc 2", 200000, "url2"), // Invalid: empty location
            create_custom_metadata("Valid Prop 3", 300, "Desc 3", 300000, "url3"),
        ];

        let result = contract.batch_register_properties(properties).unwrap();

        // 2 succeed, 1 fails
        assert_eq!(result.successes.len(), 2);
        assert_eq!(result.failures.len(), 1);
        assert_eq!(result.failures[0].index, 1);
        assert_eq!(result.failures[0].error, Error::InvalidMetadata);
        assert_eq!(result.metrics.total_items, 3);
        assert_eq!(result.metrics.successful_items, 2);
        assert_eq!(result.metrics.failed_items, 1);
        assert!(!result.metrics.early_terminated);

        // Verify IDs are contiguous
        assert_eq!(result.successes, vec![1, 2]);
        assert_eq!(contract.property_count(), 2);

        // Verify properties exist and are correct
        let prop1 = contract.get_property(1).unwrap();
        assert_eq!(prop1.metadata.location, "Valid Prop 1");
        let prop2 = contract.get_property(2).unwrap();
        assert_eq!(prop2.metadata.location, "Valid Prop 3");
    }

    #[ink::test]
    fn batch_register_properties_early_termination_works() {
        let accounts = default_accounts();
        set_caller(accounts.alice);
        ink::env::test::set_block_timestamp::<ink::env::DefaultEnvironment>(1000);
        let mut contract = PropertyRegistry::new();

        // Set failure threshold to 2
        contract.update_batch_config(50, 2).unwrap();

        let properties = vec![
            create_custom_metadata("Valid", 100, "Desc", 100000, "url"),
            create_custom_metadata("", 200, "Desc", 200000, "url"),     // fail 1
            create_custom_metadata("", 300, "Desc", 300000, "url"),     // fail 2 -> early terminate
            create_custom_metadata("Never reached", 400, "Desc", 400000, "url"),
        ];

        let result = contract.batch_register_properties(properties).unwrap();

        assert_eq!(result.successes.len(), 1);
        assert_eq!(result.failures.len(), 2);
        assert!(result.metrics.early_terminated);
        assert_eq!(result.metrics.total_items, 4);

        // Stats should record the early termination
        let stats = contract.get_batch_stats();
        assert_eq!(stats.total_early_terminations, 1);
    }

    #[ink::test]
    fn batch_update_metadata_size_guard_works() {
        let accounts = default_accounts();
        set_caller(accounts.alice);
        ink::env::test::set_block_timestamp::<ink::env::DefaultEnvironment>(1000);
        let mut contract = PropertyRegistry::new();

        // Set max to 1
        contract.update_batch_config(1, 1).unwrap();

        let props = vec![
            create_custom_metadata("Prop 1", 100, "Desc", 100000, "url"),
        ];
        let ids = contract.batch_register_properties(props).unwrap().successes;

        let updates = vec![
            (ids[0], create_custom_metadata("Updated 1", 200, "Desc", 200000, "url")),
            (999, create_custom_metadata("Updated 2", 300, "Desc", 300000, "url")),
        ];

        assert_eq!(
            contract.batch_update_metadata(updates),
            Err(Error::BatchSizeExceeded)
        );
    }

    #[ink::test]
    fn batch_update_metadata_partial_success_works() {
        let accounts = default_accounts();
        set_caller(accounts.alice);
        ink::env::test::set_block_timestamp::<ink::env::DefaultEnvironment>(1000);
        let mut contract = PropertyRegistry::new();

        let props = vec![
            create_custom_metadata("Prop 1", 100, "Desc 1", 100000, "url1"),
            create_custom_metadata("Prop 2", 200, "Desc 2", 200000, "url2"),
        ];
        let ids = contract.batch_register_properties(props).unwrap().successes;

        let updates = vec![
            (ids[0], create_custom_metadata("Updated 1", 150, "Updated Desc", 150000, "url_updated")),
            (999, create_custom_metadata("Nonexistent", 300, "Desc", 300000, "url")), // PropertyNotFound
            (ids[1], create_custom_metadata("", 250, "Desc", 250000, "url")),          // InvalidMetadata
        ];

        let result = contract.batch_update_metadata(updates).unwrap();

        assert_eq!(result.successes.len(), 1);
        assert_eq!(result.successes[0], ids[0]);
        assert_eq!(result.failures.len(), 2);
        assert_eq!(result.failures[0].index, 1);
        assert_eq!(result.failures[0].error, Error::PropertyNotFound);
        assert_eq!(result.failures[1].index, 2);
        assert_eq!(result.failures[1].error, Error::InvalidMetadata);

        // Verify the successful update took effect
        let prop = contract.get_property(ids[0]).unwrap();
        assert_eq!(prop.metadata.location, "Updated 1");

        // Verify the untouched property is unchanged
        let prop2 = contract.get_property(ids[1]).unwrap();
        assert_eq!(prop2.metadata.location, "Prop 2");
    }

    #[ink::test]
    fn batch_transfer_to_multiple_size_guard_works() {
        let accounts = default_accounts();
        set_caller(accounts.alice);
        ink::env::test::set_block_timestamp::<ink::env::DefaultEnvironment>(1000);
        let mut contract = PropertyRegistry::new();

        let props = vec![
            create_custom_metadata("Prop 1", 100, "Desc", 100000, "url"),
            create_custom_metadata("Prop 2", 200, "Desc", 200000, "url"),
        ];
        let ids = contract.batch_register_properties(props).unwrap().successes;

        // Set max to 1 AFTER registration
        contract.update_batch_config(1, 1).unwrap();

        let transfers = vec![
            (ids[0], accounts.bob),
            (ids[1], accounts.charlie),
        ];

        assert_eq!(
            contract.batch_transfer_properties_to_multiple(transfers),
            Err(Error::BatchSizeExceeded)
        );
    }

    #[ink::test]
    fn batch_stats_accumulation_works() {
        let accounts = default_accounts();
        set_caller(accounts.alice);
        ink::env::test::set_block_timestamp::<ink::env::DefaultEnvironment>(1000);
        let mut contract = PropertyRegistry::new();

        // Batch 1: Register 3 properties (all succeed)
        let props = vec![
            create_custom_metadata("Prop 1", 100, "Desc", 100000, "url"),
            create_custom_metadata("Prop 2", 200, "Desc", 200000, "url"),
            create_custom_metadata("Prop 3", 300, "Desc", 300000, "url"),
        ];
        let result = contract.batch_register_properties(props).unwrap();
        assert_eq!(result.successes.len(), 3);

        // Batch 2: Register 2 with 1 failure
        let props2 = vec![
            create_custom_metadata("Prop 4", 400, "Desc", 400000, "url"),
            create_custom_metadata("", 500, "Desc", 500000, "url"), // invalid
        ];
        let result2 = contract.batch_register_properties(props2).unwrap();
        assert_eq!(result2.successes.len(), 1);
        assert_eq!(result2.failures.len(), 1);

        // Batch 3: Transfer (atomic, all succeed)
        let ids = result.successes;
        contract
            .batch_transfer_properties(ids, accounts.bob)
            .unwrap();

        // Verify accumulated stats
        let stats = contract.get_batch_stats();
        assert_eq!(stats.total_batches_processed, 3);
        assert_eq!(stats.total_items_processed, 7); // 3 + 1 + 3
        assert_eq!(stats.total_items_failed, 1);
        assert_eq!(stats.total_early_terminations, 0);
        assert_eq!(stats.largest_batch_processed, 3);
    }

    // ========================================================================
    // Security Audit Trail Tests (Issue #82)
    // ========================================================================

    #[ink::test]
    fn test_audit_trail_initialized_on_construction() {
        let contract = PropertyRegistry::new();
        // Constructor logs one initial event (AdminChanged)
        assert_eq!(contract.audit_record_count(), 1);

        let record = contract.get_audit_record(1).unwrap();
        assert_eq!(record.event_type, SecurityEventType::AdminChanged);
        assert_eq!(record.severity, SecuritySeverity::Critical);
        assert_eq!(record.id, 1);
    }

    #[ink::test]
    fn test_audit_chain_head_nonzero_after_init() {
        let contract = PropertyRegistry::new();
        let head = contract.audit_chain_head();
        assert_ne!(head, [0u8; 32], "Chain head should not be genesis hash after init");
    }

    #[ink::test]
    fn test_register_property_creates_audit_record() {
        let mut contract = PropertyRegistry::new();
        let metadata = create_sample_metadata();

        let property_id = contract.register_property(metadata).unwrap();
        // 1 from constructor + 1 from register_property
        assert_eq!(contract.audit_record_count(), 2);

        let record = contract.get_audit_record(2).unwrap();
        assert_eq!(record.event_type, SecurityEventType::PropertyRegistered);
        assert_eq!(record.severity, SecuritySeverity::Low);
        assert_eq!(record.resource_id, property_id);
    }

    #[ink::test]
    fn test_transfer_property_creates_audit_record() {
        let accounts = default_accounts();
        let mut contract = PropertyRegistry::new();
        let metadata = create_sample_metadata();

        let property_id = contract.register_property(metadata).unwrap();
        contract.transfer_property(property_id, accounts.bob).unwrap();

        // 1 init + 1 register + 1 transfer = 3
        assert_eq!(contract.audit_record_count(), 3);

        let record = contract.get_audit_record(3).unwrap();
        assert_eq!(record.event_type, SecurityEventType::PropertyTransferred);
        assert_eq!(record.severity, SecuritySeverity::Medium);
        assert_eq!(record.resource_id, property_id);
    }

    #[ink::test]
    fn test_unauthorized_access_logged() {
        let accounts = default_accounts();
        let mut contract = PropertyRegistry::new();
        let metadata = create_sample_metadata();

        let property_id = contract.register_property(metadata).unwrap();
        let count_before = contract.audit_record_count();

        // Try unauthorized transfer
        set_caller(accounts.bob);
        let result = contract.transfer_property(property_id, accounts.charlie);
        assert_eq!(result, Err(Error::Unauthorized));

        // Should have logged an UnauthorizedAccess event
        let count_after = contract.audit_record_count();
        assert_eq!(count_after, count_before + 1);

        let record = contract.get_audit_record(count_after).unwrap();
        assert_eq!(record.event_type, SecurityEventType::UnauthorizedAccess);
        assert_eq!(record.severity, SecuritySeverity::Critical);
        assert_eq!(record.actor, accounts.bob);
    }

    #[ink::test]
    fn test_change_admin_creates_critical_audit() {
        let accounts = default_accounts();
        let mut contract = PropertyRegistry::new();

        contract.change_admin(accounts.bob).unwrap();

        let count = contract.audit_record_count();
        let record = contract.get_audit_record(count).unwrap();
        assert_eq!(record.event_type, SecurityEventType::AdminChanged);
        assert_eq!(record.severity, SecuritySeverity::Critical);
    }

    #[ink::test]
    fn test_pause_resume_audit_trail() {
        let accounts = default_accounts();
        let mut contract = PropertyRegistry::new();
        let count_before = contract.audit_record_count();

        // Pause
        contract.pause_contract("test".into(), None).unwrap();
        let pause_record = contract.get_audit_record(count_before + 1).unwrap();
        assert_eq!(pause_record.event_type, SecurityEventType::ContractPaused);
        assert_eq!(pause_record.severity, SecuritySeverity::Critical);

        // Set up resume (need a second guardian for multi-sig)
        set_caller(accounts.alice);
        contract.request_resume().unwrap();

        // Add bob as guardian and have him approve
        let account2 = accounts.bob;
        set_caller(contract.admin());
        contract.set_pause_guardian(account2, true).unwrap();

        set_caller(account2);
        contract.approve_resume().unwrap();

        // Find the ContractResumed audit record
        let count_after = contract.audit_record_count();
        let mut found_resume = false;
        for i in (count_before + 1)..=count_after {
            if let Some(r) = contract.get_audit_record(i) {
                if r.event_type == SecurityEventType::ContractResumed {
                    assert_eq!(r.severity, SecuritySeverity::Critical);
                    found_resume = true;
                    break;
                }
            }
        }
        assert!(found_resume, "ContractResumed audit record not found");
    }

    #[ink::test]
    fn test_audit_integrity_verification() {
        let mut contract = PropertyRegistry::new();
        let metadata = create_sample_metadata();

        // Create several audit records
        contract.register_property(metadata.clone()).unwrap();
        contract.register_property(metadata.clone()).unwrap();
        contract.register_property(metadata).unwrap();

        // Verify integrity of the full chain
        let count = contract.audit_record_count();
        assert!(count >= 4); // 1 init + 3 registers
        let is_valid = contract.verify_audit_integrity(1, count);
        assert!(is_valid, "Audit chain should be valid");
    }

    #[ink::test]
    fn test_audit_integrity_invalid_range() {
        let mut contract = PropertyRegistry::new();

        // Invalid ranges should return false
        assert!(!contract.verify_audit_integrity(0, 1));
        assert!(!contract.verify_audit_integrity(3, 2));
        assert!(!contract.verify_audit_integrity(1, 999));
    }

    #[ink::test]
    fn test_audit_records_by_actor() {
        let accounts = default_accounts();
        let mut contract = PropertyRegistry::new();
        let metadata = create_sample_metadata();

        // Admin (alice) creates properties
        contract.register_property(metadata.clone()).unwrap();
        contract.register_property(metadata).unwrap();

        // Query by actor
        let records = contract.get_audit_records_by_actor(accounts.alice, 0, 50);
        // Should have: 1 init + 2 registers = 3 records for alice
        assert_eq!(records.len(), 3);
    }

    #[ink::test]
    fn test_audit_records_by_type() {
        let mut contract = PropertyRegistry::new();
        let metadata = create_sample_metadata();

        contract.register_property(metadata.clone()).unwrap();
        contract.register_property(metadata).unwrap();

        let records = contract.get_audit_records_by_type(SecurityEventType::PropertyRegistered, 0, 50);
        assert_eq!(records.len(), 2);
    }

    #[ink::test]
    fn test_audit_sequential_hash_chain() {
        let mut contract = PropertyRegistry::new();
        let metadata = create_sample_metadata();

        contract.register_property(metadata.clone()).unwrap();
        contract.register_property(metadata).unwrap();

        let record1 = contract.get_audit_record(1).unwrap();
        let record2 = contract.get_audit_record(2).unwrap();
        let record3 = contract.get_audit_record(3).unwrap();

        // Each record should have a unique hash
        assert_ne!(record1.record_hash, record2.record_hash);
        assert_ne!(record2.record_hash, record3.record_hash);
        assert_ne!(record1.record_hash, record3.record_hash);

        // Chain head should match the last record's hash
        assert_eq!(contract.audit_chain_head(), record3.record_hash);
    }

    #[ink::test]
    fn test_escrow_creates_audit_records() {
        let accounts = default_accounts();
        let mut contract = PropertyRegistry::new();
        let metadata = create_sample_metadata();

        let property_id = contract.register_property(metadata).unwrap();
        let escrow_id = contract.create_escrow(property_id, accounts.bob, 1000).unwrap();

        let count = contract.audit_record_count();
        let record = contract.get_audit_record(count).unwrap();
        assert_eq!(record.event_type, SecurityEventType::EscrowCreated);
        assert_eq!(record.severity, SecuritySeverity::Medium);
        assert_eq!(record.resource_id, escrow_id);
    }

    #[ink::test]
    fn test_role_grant_creates_audit_record() {
        let accounts = default_accounts();
        let mut contract = PropertyRegistry::new();

        contract.grant_role(accounts.bob, Role::Verifier).unwrap();

        let count = contract.audit_record_count();
        let record = contract.get_audit_record(count).unwrap();
        assert_eq!(record.event_type, SecurityEventType::RoleGranted);
        assert_eq!(record.severity, SecuritySeverity::Critical);
        assert_eq!(record.extra_data, Role::Verifier as u32);
    }

    #[ink::test]
    fn test_audit_pagination() {
        let mut contract = PropertyRegistry::new();
        let metadata = create_sample_metadata();

        // Create 5 properties (+ 1 init = 6 total records)
        for _ in 0..5 {
            contract.register_property(metadata.clone()).unwrap();
        }

        let accounts = default_accounts();

        // Page 1: offset=0, limit=3
        let page1 = contract.get_audit_records_by_actor(accounts.alice, 0, 3);
        assert_eq!(page1.len(), 3);

        // Page 2: offset=3, limit=3
        let page2 = contract.get_audit_records_by_actor(accounts.alice, 3, 3);
        assert_eq!(page2.len(), 3);

        // No overlap between pages
        for id in &page1 {
            assert!(!page2.contains(id));
        }
    }

    #[ink::test]
    fn test_config_change_creates_high_audit() {
        let mut contract = PropertyRegistry::new();

        contract.update_batch_config(50, 10).unwrap();

        let count = contract.audit_record_count();
        let record = contract.get_audit_record(count).unwrap();
        assert_eq!(record.event_type, SecurityEventType::ConfigurationChanged);
        assert_eq!(record.severity, SecuritySeverity::High);
        assert_eq!(record.extra_data, 50); // max_batch_size
    // -- Issue #79 Numeric/Range/Transfer validation tests --

    #[ink::test]
    fn test_issue_badge_past_expiry_rejected() {
        use crate::propchain_contracts::BadgeType;
        let accounts = default_accounts();
        set_caller(accounts.alice);
        ink::env::test::set_block_timestamp::<ink::env::DefaultEnvironment>(1000);
        let mut contract = PropertyRegistry::new();
        let property_id = contract
            .register_property(create_sample_metadata())
            .expect("register");
        // expires_at in the past
        assert_eq!(
            contract.issue_badge(
                property_id,
                BadgeType::DocumentVerification,
                Some(500),
                "https://metadata.example.com/badge.json".to_string()
            ),
            Err(Error::ValueOutOfBounds)
        );
    }

    #[ink::test]
    fn test_pause_duration_too_long_rejected() {
        let accounts = default_accounts();
        set_caller(accounts.alice);
        let mut contract = PropertyRegistry::new();
        // Duration exceeds MAX_PAUSE_DURATION (30 days = 2_592_000 seconds)
        assert_eq!(
            contract.pause_contract("Maintenance".to_string(), Some(3_000_000)),
            Err(Error::ValueOutOfBounds)
        );
    }

    #[ink::test]
    fn test_pause_duration_too_short_rejected() {
        let accounts = default_accounts();
        set_caller(accounts.alice);
        let mut contract = PropertyRegistry::new();
        // Duration below MIN_PAUSE_DURATION (60 seconds)
        assert_eq!(
            contract.pause_contract("Maintenance".to_string(), Some(10)),
            Err(Error::ValueOutOfBounds)
        );
    }

    // -- Range Query Tests --

    #[ink::test]
    fn test_price_range_invalid_rejected() {
        let accounts = default_accounts();
        set_caller(accounts.alice);
        let contract = PropertyRegistry::new();
        assert_eq!(
            contract.get_properties_by_price_range(200000, 100000),
            Err(Error::InvalidRange)
        );
    }

    #[ink::test]
    fn test_size_range_invalid_rejected() {
        let accounts = default_accounts();
        set_caller(accounts.alice);
        let contract = PropertyRegistry::new();
        assert_eq!(
            contract.get_properties_by_size_range(2000, 1000),
            Err(Error::InvalidRange)
        );
    }

    // -- Batch Transfer Zero Address Tests --

    #[ink::test]
    fn test_batch_transfer_to_zero_address_rejected() {
        let accounts = default_accounts();
        set_caller(accounts.alice);
        let mut contract = PropertyRegistry::new();
        let property_id = contract
            .register_property(create_sample_metadata())
            .expect("register");
        let zero = AccountId::from([0u8; 32]);
        assert_eq!(
            contract.batch_transfer_properties(vec![property_id], zero),
            Err(Error::ZeroAddress)
        );
    }

    #[ink::test]
    fn test_batch_transfer_to_multiple_zero_address_rejected() {
        let accounts = default_accounts();
        set_caller(accounts.alice);
        let mut contract = PropertyRegistry::new();
        let property_id = contract
            .register_property(create_sample_metadata())
            .expect("register");
        let zero = AccountId::from([0u8; 32]);
        assert_eq!(
            contract.batch_transfer_properties_to_multiple(vec![(property_id, zero)]),
            Err(Error::ZeroAddress)
        );
    }
}
