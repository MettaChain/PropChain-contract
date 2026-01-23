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
        
        let property_ids = contract.batch_register_properties(properties).expect("Failed to batch register");
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
        
        let property_ids = contract.batch_register_properties(properties).expect("Failed to batch register");
        
        // Transfer all properties to Bob
        assert!(contract.batch_transfer_properties(property_ids.clone(), accounts.bob).is_ok());
        
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
        
        let property_ids = contract.batch_register_properties(properties).expect("Failed to batch register");
        
        // Update metadata for all properties
        let updates = vec![
            (property_ids[0], PropertyMetadata {
                location: "Updated Property 1".to_string(),
                size: 1200,
                legal_description: "Updated test property 1".to_string(),
                valuation: 120000,
                documents_url: "https://example.com/docs1_updated".to_string(),
            }),
            (property_ids[1], PropertyMetadata {
                location: "Updated Property 2".to_string(),
                size: 1700,
                legal_description: "Updated test property 2".to_string(),
                valuation: 170000,
                documents_url: "https://example.com/docs2_updated".to_string(),
            }),
        ];
        
        assert!(contract.batch_update_metadata(updates).is_ok());
        
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
        
        let property_ids = contract.batch_register_properties(properties).expect("Failed to batch register");
        
        // Transfer properties to different recipients
        let transfers = vec![
            (property_ids[0], accounts.bob),
            (property_ids[1], accounts.charlie),
            (property_ids[2], accounts.django),
        ];
        
        assert!(contract.batch_transfer_properties_to_multiple(transfers).is_ok());
        
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
        
        contract.batch_register_properties(properties).expect("Failed to batch register");
        
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
        
        let property_ids = contract.batch_register_properties(properties).expect("Failed to batch register");
        
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
        let alice_properties = vec![
            PropertyMetadata {
                location: "Alice Property 1".to_string(),
                size: 1000,
                legal_description: "Test property".to_string(),
                valuation: 100000,
                documents_url: "https://example.com/docs".to_string(),
            },
        ];
        contract.batch_register_properties(alice_properties).expect("Failed to register Alice properties");
        
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
        contract.batch_register_properties(bob_properties).expect("Failed to register Bob properties");
        
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
        
        contract.batch_register_properties(properties).expect("Failed to batch register");
        
        // Get properties in medium price range
        let medium_properties = contract.get_properties_by_price_range(100000, 200000);
        assert_eq!(medium_properties.len(), 1);
        assert_eq!(medium_properties[0], 2); // Medium Property
        
        // Get properties in high price range
        let high_properties = contract.get_properties_by_price_range(200000, 300000);
        assert_eq!(high_properties.len(), 1);
        assert_eq!(high_properties[0], 3); // Expensive Property
        
        // Get all properties
        let all_properties = contract.get_properties_by_price_range(0, 300000);
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
        
        contract.batch_register_properties(properties).expect("Failed to batch register");
        
        // Get properties in medium size range
        let medium_properties = contract.get_properties_by_size_range(1000, 2000);
        assert_eq!(medium_properties.len(), 1);
        assert_eq!(medium_properties[0], 2); // Medium Property
        
        // Get properties in large size range
        let large_properties = contract.get_properties_by_size_range(2000, 3000);
        assert_eq!(large_properties.len(), 1);
        assert_eq!(large_properties[0], 3); // Large Property
        
        // Get all properties
        let all_properties = contract.get_properties_by_size_range(0, 3000);
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
        
        contract.register_property(metadata).expect("Failed to register");
        
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
            contract.register_property(metadata.clone()).expect("Failed to register");
        }
        
        // Get performance recommendations
        let recommendations = contract.get_performance_recommendations();
        assert!(!recommendations.is_empty());
        
        // Should contain general recommendations
        let recommendation_strings: Vec<&str> = recommendations.iter().map(|s| s.as_str()).collect();
        assert!(recommendation_strings.contains(&"Use batch operations for multiple property transfers"));
        assert!(recommendation_strings.contains(&"Prefer portfolio analytics over individual property queries"));
        assert!(recommendation_strings.contains(&"Consider off-chain indexing for complex analytics"));
    }

    // Error Cases Tests
    
    #[ink::test]
    fn batch_transfer_unauthorized_fails() {
        let accounts = default_accounts();
        set_caller(accounts.alice);
        let mut contract = PropertyRegistry::new();
        
        // Register properties
        let properties = vec![
            PropertyMetadata {
                location: "Property 1".to_string(),
                size: 1000,
                legal_description: "Test property".to_string(),
                valuation: 100000,
                documents_url: "https://example.com/docs".to_string(),
            },
        ];
        
        let property_ids = contract.batch_register_properties(properties).expect("Failed to batch register");
        
        // Try to transfer as unauthorized user
        set_caller(accounts.bob);
        assert_eq!(contract.batch_transfer_properties(property_ids, accounts.charlie), Err(Error::Unauthorized));
    }

    #[ink::test]
    fn batch_update_metadata_unauthorized_fails() {
        let accounts = default_accounts();
        set_caller(accounts.alice);
        let mut contract = PropertyRegistry::new();
        
        // Register properties
        let properties = vec![
            PropertyMetadata {
                location: "Property 1".to_string(),
                size: 1000,
                legal_description: "Test property".to_string(),
                valuation: 100000,
                documents_url: "https://example.com/docs".to_string(),
            },
        ];
        
        let property_ids = contract.batch_register_properties(properties).expect("Failed to batch register");
        
        // Try to update as unauthorized user
        set_caller(accounts.bob);
        let updates = vec![
            (property_ids[0], PropertyMetadata {
                location: "Updated Property".to_string(),
                size: 1200,
                legal_description: "Updated test property".to_string(),
                valuation: 120000,
                documents_url: "https://example.com/docs_updated".to_string(),
            }),
        ];
        
        assert_eq!(contract.batch_update_metadata(updates), Err(Error::Unauthorized));
    }

    #[ink::test]
    fn batch_operations_with_empty_input_works() {
        let accounts = default_accounts();
        set_caller(accounts.alice);
        let mut contract = PropertyRegistry::new();
        
        // Test empty batch register
        let empty_properties: Vec<PropertyMetadata> = vec![];
        let result = contract.batch_register_properties(empty_properties);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 0);
        
        // Test empty batch transfer
        let empty_transfers: Vec<u64> = vec![];
        assert!(contract.batch_transfer_properties(empty_transfers, accounts.bob).is_ok());
        
        // Test empty batch update
        let empty_updates: Vec<(u64, PropertyMetadata)> = vec![];
        assert!(contract.batch_update_metadata(empty_updates).is_ok());
        
        // Test empty batch transfer to multiple
        let empty_multiple_transfers: Vec<(u64, AccountId)> = vec![];
        assert!(contract.batch_transfer_properties_to_multiple(empty_multiple_transfers).is_ok());
    }

    // ========== PAUSE/RESUME FUNCTIONALITY TESTS ==========

    #[ink::test]
    fn pause_works() {
        let accounts = default_accounts();
        set_caller(accounts.alice);
        let mut contract = PropertyRegistry::new();
        
        // Admin should be able to pause
        assert!(contract.pause().is_ok());
        assert!(contract.is_paused());
        
        let pause_state = contract.get_pause_state();
        assert!(pause_state.is_paused);
        assert_eq!(pause_state.paused_by, Some(accounts.alice));
        assert_eq!(pause_state.pause_count, 1);
    }

    #[ink::test]
    fn pause_unauthorized_fails() {
        let accounts = default_accounts();
        set_caller(accounts.alice);
        let mut contract = PropertyRegistry::new();
        
        // Bob is not a pauser
        set_caller(accounts.bob);
        assert_eq!(contract.pause(), Err(Error::NotPauser));
    }

    #[ink::test]
    fn pause_when_already_paused_fails() {
        let accounts = default_accounts();
        set_caller(accounts.alice);
        let mut contract = PropertyRegistry::new();
        
        assert!(contract.pause().is_ok());
        assert_eq!(contract.pause(), Err(Error::ContractPaused));
    }

    #[ink::test]
    fn resume_by_admin_works() {
        let accounts = default_accounts();
        set_caller(accounts.alice);
        let mut contract = PropertyRegistry::new();
        
        // Pause the contract
        assert!(contract.pause().is_ok());
        assert!(contract.is_paused());
        
        // Admin can resume immediately without approvals
        assert!(contract.resume().is_ok());
        assert!(!contract.is_paused());
    }

    #[ink::test]
    fn resume_when_not_paused_fails() {
        let accounts = default_accounts();
        set_caller(accounts.alice);
        let mut contract = PropertyRegistry::new();
        
        assert_eq!(contract.resume(), Err(Error::ContractNotPaused));
    }

    #[ink::test]
    fn resume_with_multisig_works() {
        let accounts = default_accounts();
        set_caller(accounts.alice);
        let mut contract = PropertyRegistry::new();
        
        // Add Bob as resume approver
        assert!(contract.add_resume_approver(accounts.bob).is_ok());
        
        // Set threshold to 2
        assert!(contract.set_required_approvals(2).is_ok());
        
        // Pause the contract
        assert!(contract.pause().is_ok());
        
        // Alice approves
        set_caller(accounts.alice);
        assert!(contract.approve_resume().is_ok());
        assert_eq!(contract.get_resume_approvals(), 1);
        
        // Try to resume with insufficient approvals
        assert_eq!(contract.resume(), Err(Error::InsufficientApprovals));
        
        // Bob approves
        set_caller(accounts.bob);
        assert!(contract.approve_resume().is_ok());
        assert_eq!(contract.get_resume_approvals(), 2);
        
        // Now resume should work
        assert!(contract.resume().is_ok());
        assert!(!contract.is_paused());
        
        // Approvals should be reset
        assert_eq!(contract.get_resume_approvals(), 0);
    }

    #[ink::test]
    fn add_pauser_works() {
        let accounts = default_accounts();
        set_caller(accounts.alice);
        let mut contract = PropertyRegistry::new();
        
        // Add Bob as pauser
        assert!(contract.add_pauser(accounts.bob).is_ok());
        assert!(contract.is_pauser(accounts.bob));
        
        // Bob should now be able to pause
        set_caller(accounts.bob);
        assert!(contract.pause().is_ok());
    }

    #[ink::test]
    fn add_pauser_unauthorized_fails() {
        let accounts = default_accounts();
        set_caller(accounts.alice);
        let mut contract = PropertyRegistry::new();
        
        // Bob is not admin
        set_caller(accounts.bob);
        assert_eq!(contract.add_pauser(accounts.charlie), Err(Error::Unauthorized));
    }

    #[ink::test]
    fn remove_pauser_works() {
        let accounts = default_accounts();
        set_caller(accounts.alice);
        let mut contract = PropertyRegistry::new();
        
        // Add Bob as pauser
        assert!(contract.add_pauser(accounts.bob).is_ok());
        assert!(contract.is_pauser(accounts.bob));
        
        // Remove Bob
        assert!(contract.remove_pauser(accounts.bob).is_ok());
        assert!(!contract.is_pauser(accounts.bob));
        
        // Bob should not be able to pause
        set_caller(accounts.bob);
        assert_eq!(contract.pause(), Err(Error::NotPauser));
    }

    #[ink::test]
    fn add_resume_approver_works() {
        let accounts = default_accounts();
        set_caller(accounts.alice);
        let mut contract = PropertyRegistry::new();
        
        // Add Bob as resume approver
        assert!(contract.add_resume_approver(accounts.bob).is_ok());
        
        // Pause the contract
        assert!(contract.pause().is_ok());
        
        // Bob should be able to approve resume
        set_caller(accounts.bob);
        assert!(contract.approve_resume().is_ok());
    }

    #[ink::test]
    fn approve_resume_unauthorized_fails() {
        let accounts = default_accounts();
        set_caller(accounts.alice);
        let mut contract = PropertyRegistry::new();
        
        // Pause the contract
        assert!(contract.pause().is_ok());
        
        // Bob is not a resume approver
        set_caller(accounts.bob);
        assert_eq!(contract.approve_resume(), Err(Error::NotResumeApprover));
    }

    #[ink::test]
    fn set_required_approvals_works() {
        let accounts = default_accounts();
        set_caller(accounts.alice);
        let mut contract = PropertyRegistry::new();
        
        assert_eq!(contract.get_required_approvals(), 1);
        
        assert!(contract.set_required_approvals(3).is_ok());
        assert_eq!(contract.get_required_approvals(), 3);
    }

    #[ink::test]
    fn set_required_approvals_zero_fails() {
        let accounts = default_accounts();
        set_caller(accounts.alice);
        let mut contract = PropertyRegistry::new();
        
        assert_eq!(contract.set_required_approvals(0), Err(Error::InvalidApprovalThreshold));
    }

    #[ink::test]
    fn set_required_approvals_unauthorized_fails() {
        let accounts = default_accounts();
        set_caller(accounts.alice);
        let mut contract = PropertyRegistry::new();
        
        set_caller(accounts.bob);
        assert_eq!(contract.set_required_approvals(2), Err(Error::Unauthorized));
    }

    #[ink::test]
    fn schedule_auto_resume_works() {
        let accounts = default_accounts();
        set_caller(accounts.alice);
        let mut contract = PropertyRegistry::new();
        
        // Pause the contract
        assert!(contract.pause().is_ok());
        
        // Schedule auto-resume for future timestamp
        let future_time = 1000000;
        assert!(contract.schedule_auto_resume(future_time).is_ok());
        assert_eq!(contract.get_auto_resume_time(), Some(future_time));
    }

    #[ink::test]
    fn schedule_auto_resume_when_not_paused_fails() {
        let accounts = default_accounts();
        set_caller(accounts.alice);
        let mut contract = PropertyRegistry::new();
        
        assert_eq!(contract.schedule_auto_resume(1000000), Err(Error::ContractNotPaused));
    }

    #[ink::test]
    fn schedule_auto_resume_past_time_fails() {
        let accounts = default_accounts();
        set_caller(accounts.alice);
        let mut contract = PropertyRegistry::new();
        
        // Pause the contract
        assert!(contract.pause().is_ok());
        
        // Try to schedule for past/current time
        assert_eq!(contract.schedule_auto_resume(0), Err(Error::InvalidAutoResumeTime));
    }

    #[ink::test]
    fn cancel_auto_resume_works() {
        let accounts = default_accounts();
        set_caller(accounts.alice);
        let mut contract = PropertyRegistry::new();
        
        // Pause and schedule auto-resume
        assert!(contract.pause().is_ok());
        assert!(contract.schedule_auto_resume(1000000).is_ok());
        
        // Cancel auto-resume
        assert!(contract.cancel_auto_resume().is_ok());
        assert_eq!(contract.get_auto_resume_time(), None);
    }

    #[ink::test]
    fn cancel_auto_resume_when_not_scheduled_fails() {
        let accounts = default_accounts();
        set_caller(accounts.alice);
        let mut contract = PropertyRegistry::new();
        
        // Pause without scheduling auto-resume
        assert!(contract.pause().is_ok());
        
        assert_eq!(contract.cancel_auto_resume(), Err(Error::AutoResumeNotScheduled));
    }

    #[ink::test]
    fn paused_contract_blocks_register_property() {
        let accounts = default_accounts();
        set_caller(accounts.alice);
        let mut contract = PropertyRegistry::new();
        
        // Pause the contract
        assert!(contract.pause().is_ok());
        
        // Try to register property
        let metadata = PropertyMetadata {
            location: "Test Property".to_string(),
            size: 1000,
            legal_description: "Test".to_string(),
            valuation: 100000,
            documents_url: "https://example.com".to_string(),
        };
        
        assert_eq!(contract.register_property(metadata), Err(Error::ContractPaused));
    }

    #[ink::test]
    fn paused_contract_blocks_transfer_property() {
        let accounts = default_accounts();
        set_caller(accounts.alice);
        let mut contract = PropertyRegistry::new();
        
        // Register property first
        let metadata = PropertyMetadata {
            location: "Test Property".to_string(),
            size: 1000,
            legal_description: "Test".to_string(),
            valuation: 100000,
            documents_url: "https://example.com".to_string(),
        };
        let property_id = contract.register_property(metadata).unwrap();
        
        // Pause the contract
        assert!(contract.pause().is_ok());
        
        // Try to transfer property
        assert_eq!(contract.transfer_property(property_id, accounts.bob), Err(Error::ContractPaused));
    }

    #[ink::test]
    fn paused_contract_blocks_create_escrow() {
        let accounts = default_accounts();
        set_caller(accounts.alice);
        let mut contract = PropertyRegistry::new();
        
        // Register property first
        let metadata = PropertyMetadata {
            location: "Test Property".to_string(),
            size: 1000,
            legal_description: "Test".to_string(),
            valuation: 100000,
            documents_url: "https://example.com".to_string(),
        };
        let property_id = contract.register_property(metadata).unwrap();
        
        // Pause the contract
        assert!(contract.pause().is_ok());
        
        // Try to create escrow
        assert_eq!(contract.create_escrow(property_id, 100000), Err(Error::ContractPaused));
    }

    #[ink::test]
    fn pause_event_audit_trail_works() {
        let accounts = default_accounts();
        set_caller(accounts.alice);
        let mut contract = PropertyRegistry::new();
        
        // Perform pause/resume operations
        assert!(contract.pause().is_ok());
        assert!(contract.resume().is_ok());
        assert!(contract.pause().is_ok());
        
        // Check audit trail
        let events = contract.get_pause_events(10);
        assert_eq!(events.len(), 3);
        assert_eq!(contract.get_pause_event_count(), 3);
    }

    #[ink::test]
    fn multiple_pause_resume_cycles_work() {
        let accounts = default_accounts();
        set_caller(accounts.alice);
        let mut contract = PropertyRegistry::new();
        
        // First cycle
        assert!(contract.pause().is_ok());
        assert!(contract.is_paused());
        assert!(contract.resume().is_ok());
        assert!(!contract.is_paused());
        
        // Second cycle
        assert!(contract.pause().is_ok());
        assert!(contract.is_paused());
        assert!(contract.resume().is_ok());
        assert!(!contract.is_paused());
        
        // Verify pause count
        let pause_state = contract.get_pause_state();
        assert_eq!(pause_state.pause_count, 2);
    }

    #[ink::test]
    fn double_approval_is_idempotent() {
        let accounts = default_accounts();
        set_caller(accounts.alice);
        let mut contract = PropertyRegistry::new();
        
        // Pause the contract
        assert!(contract.pause().is_ok());
        
        // Approve twice
        assert!(contract.approve_resume().is_ok());
        assert_eq!(contract.get_resume_approvals(), 1);
        
        assert!(contract.approve_resume().is_ok()); // Should be no-op
        assert_eq!(contract.get_resume_approvals(), 1); // Still 1
    }
}

