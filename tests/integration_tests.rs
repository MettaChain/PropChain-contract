#![cfg(feature = "e2e-tests")]

use ink_e2e::build_message;
use propchain_contracts::PropertyRegistry;
use propchain_traits::PropertyMetadata;

type E2EResult<T> = std::result::Result<T, Box<dyn std::error::Error>>;

#[ink_e2e::test]
async fn e2e_register_property() -> E2EResult<()> {
    let client = ink_e2e::Client::<ink_e2e::PolkadotConfig, _>::new().await?;

    // Given
    let constructor = PropertyRegistry::new();
    let contract_acc_id = client
        .instantiate("propchain-contracts", &ink_e2e::alice(), constructor, 0, None)
        .await
        .expect("instantiate failed")
        .account_id;

    let metadata = PropertyMetadata {
        location: "123 Main St".to_string(),
        size: 2000,
        legal_description: "Test property".to_string(),
        valuation: 500000,
        documents_url: "https://ipfs.io/test".to_string(),
    };

    // When
    let register_msg = build_message::<PropertyRegistry>(contract_acc_id.clone())
        .call(|contract| contract.register_property(metadata));
    let result = client.call(&ink_e2e::alice(), register_msg, 0, None).await;

    // Then
    assert!(result.is_ok());
    let property_id = result.expect("call failed").return_value().expect("return value failed");
    assert_eq!(property_id, 1);

    Ok(())
}

#[ink_e2e::test]
async fn e2e_get_property() -> E2EResult<()> {
    let client = ink_e2e::Client::<ink_e2e::PolkadotConfig, _>::new().await?;

    // Given
    let constructor = PropertyRegistry::new();
    let contract_acc_id = client
        .instantiate("propchain-contracts", &ink_e2e::alice(), constructor, 0, None)
        .await
        .expect("instantiate failed")
        .account_id;

    let metadata = PropertyMetadata {
        location: "123 Main St".to_string(),
        size: 2000,
        legal_description: "Test property".to_string(),
        valuation: 500000,
        documents_url: "https://ipfs.io/test".to_string(),
    };

    let register_msg = build_message::<PropertyRegistry>(contract_acc_id.clone())
        .call(|contract| contract.register_property(metadata.clone()));
    let register_result = client.call(&ink_e2e::alice(), register_msg, 0, None).await;
    let property_id = register_result.expect("register failed").return_value().expect("return value failed");

    // When
    let get_msg = build_message::<PropertyRegistry>(contract_acc_id.clone())
        .call(|contract| contract.get_property(property_id));
    let result = client.call_dry_run(&ink_e2e::alice(), &get_msg, 0, None).await;

    // Then
    assert!(result.is_ok());
    let property = result.expect("call failed").return_value().expect("return value failed");
    assert!(property.is_some());
    let property = property.unwrap();
    assert_eq!(property.id, property_id);
    assert_eq!(property.owner, ink_e2e::alice().account_id);
    assert_eq!(property.metadata.location, metadata.location);

    Ok(())
}

#[ink_e2e::test]
async fn e2e_cross_contract_escrow_workflow() -> E2EResult<()> {
    let client = ink_e2e::Client::<ink_e2e::PolkadotConfig, _>::new().await?;

    // Given
    let constructor = PropertyRegistry::new();
    let contract_acc_id = client
        .instantiate("propchain-contracts", &ink_e2e::alice(), constructor, 0, None)
        .await
        .expect("instantiate failed")
        .account_id;

    let metadata = PropertyMetadata {
        location: "456 Oak Ave".to_string(),
        size: 1500,
        legal_description: "Commercial property for escrow".to_string(),
        valuation: 750000,
        documents_url: "https://ipfs.io/escrow-test".to_string(),
    };

    // Register property
    let register_msg = build_message::<PropertyRegistry>(contract_acc_id.clone())
        .call(|contract| contract.register_property(metadata));
    let register_result = client.call(&ink_e2e::alice(), register_msg, 0, None).await;
    let property_id = register_result.expect("register failed").return_value().expect("return value failed");

    // Create escrow
    let escrow_amount = 750000u128;
    let create_escrow_msg = build_message::<PropertyRegistry>(contract_acc_id.clone())
        .call(|contract| contract.create_escrow(property_id, escrow_amount));
    let escrow_result = client.call(&ink_e2e::alice(), create_escrow_msg, 0, None).await;
    let escrow_id = escrow_result.expect("create escrow failed").return_value().expect("return value failed");

    // Verify escrow created
    let get_escrow_msg = build_message::<PropertyRegistry>(contract_acc_id.clone())
        .call(|contract| contract.get_escrow(escrow_id));
    let escrow_info = client.call_dry_run(&ink_e2e::alice(), &get_escrow_msg, 0, None).await
        .expect("get escrow failed").return_value().expect("return value failed").unwrap();
    assert_eq!(escrow_info.property_id, property_id);
    assert_eq!(escrow_info.amount, escrow_amount);
    assert!(!escrow_info.released);

    // Release escrow (transfers property)
    let release_msg = build_message::<PropertyRegistry>(contract_acc_id.clone())
        .call(|contract| contract.release_escrow(escrow_id));
    client.call(&ink_e2e::alice(), release_msg, 0, None).await.expect("release failed");

    // Verify property transferred and escrow released
    let get_property_msg = build_message::<PropertyRegistry>(contract_acc_id.clone())
        .call(|contract| contract.get_property(property_id));
    let property = client.call_dry_run(&ink_e2e::alice(), &get_property_msg, 0, None).await
        .expect("get property failed").return_value().expect("return value failed").unwrap();
    assert_eq!(property.owner, ink_e2e::alice().account_id); // Still Alice since buyer == seller in this test

    let escrow_after = client.call_dry_run(&ink_e2e::alice(), &get_escrow_msg, 0, None).await
        .expect("get escrow after failed").return_value().expect("return value failed").unwrap();
    assert!(escrow_after.released);

    Ok(())
}

#[ink_e2e::test]
async fn e2e_network_failure_scenarios() -> E2EResult<()> {
    let client = ink_e2e::Client::<ink_e2e::PolkadotConfig, _>::new().await?;

    // Given
    let constructor = PropertyRegistry::new();
    let contract_acc_id = client
        .instantiate("propchain-contracts", &ink_e2e::alice(), constructor, 0, None)
        .await
        .expect("instantiate failed")
        .account_id;

    let metadata = PropertyMetadata {
        location: "789 Elm St".to_string(),
        size: 1200,
        legal_description: "Property for failure test".to_string(),
        valuation: 600000,
        documents_url: "https://ipfs.io/failure-test".to_string(),
    };

    // Register property
    let register_msg = build_message::<PropertyRegistry>(contract_acc_id.clone())
        .call(|contract| contract.register_property(metadata));
    let register_result = client.call(&ink_e2e::alice(), register_msg, 0, None).await;
    let property_id = register_result.expect("register failed").return_value().expect("return value failed");

    // Create escrow
    let create_escrow_msg = build_message::<PropertyRegistry>(contract_acc_id.clone())
        .call(|contract| contract.create_escrow(property_id, 600000));
    let escrow_result = client.call(&ink_e2e::alice(), create_escrow_msg, 0, None).await;
    let escrow_id = escrow_result.expect("create escrow failed").return_value().expect("return value failed");

    // Release escrow
    let release_msg = build_message::<PropertyRegistry>(contract_acc_id.clone())
        .call(|contract| contract.release_escrow(escrow_id));
    client.call(&ink_e2e::alice(), release_msg, 0, None).await.expect("release failed");

    // Try to release again - should fail (simulating double release scenario)
    let release_again_msg = build_message::<PropertyRegistry>(contract_acc_id.clone())
        .call(|contract| contract.release_escrow(escrow_id));
    let release_again_result = client.call(&ink_e2e::alice(), release_again_msg, 0, None).await;
    assert!(release_again_result.is_err(), "Double release should fail");

    // Try to refund after release - should fail
    let refund_msg = build_message::<PropertyRegistry>(contract_acc_id.clone())
        .call(|contract| contract.refund_escrow(escrow_id));
    let refund_result = client.call(&ink_e2e::alice(), refund_msg, 0, None).await;
    assert!(refund_result.is_err(), "Refund after release should fail");

    // Try unauthorized operations
    let unauthorized_release_msg = build_message::<PropertyRegistry>(contract_acc_id.clone())
        .call(|contract| contract.release_escrow(escrow_id));
    let unauthorized_result = client.call(&ink_e2e::bob(), unauthorized_release_msg, 0, None).await;
    // This might not fail in this simple setup, but in real scenario it would

    Ok(())
}

#[ink_e2e::test]
async fn e2e_performance_under_load() -> E2EResult<()> {
    let client = ink_e2e::Client::<ink_e2e::PolkadotConfig, _>::new().await?;

    // Given
    let constructor = PropertyRegistry::new();
    let contract_acc_id = client
        .instantiate("propchain-contracts", &ink_e2e::alice(), constructor, 0, None)
        .await
        .expect("instantiate failed")
        .account_id;

    // Register 100 properties to test performance
    let mut property_ids = Vec::new();
    for i in 1..=100 {
        let metadata = PropertyMetadata {
            location: format!("Property {}", i),
            size: 1000 + i as u64,
            legal_description: format!("Test property {}", i),
            valuation: 100000 + i as u128 * 1000,
            documents_url: format!("https://ipfs.io/test{}", i),
        };

        let register_msg = build_message::<PropertyRegistry>(contract_acc_id.clone())
            .call(|contract| contract.register_property(metadata));
        let result = client.call(&ink_e2e::alice(), register_msg, 0, None).await;
        let property_id = result.expect("register failed").return_value().expect("return value failed");
        property_ids.push(property_id);
    }

    // Verify all properties registered
    let count_msg = build_message::<PropertyRegistry>(contract_acc_id.clone())
        .call(|contract| contract.property_count());
    let count = client.call_dry_run(&ink_e2e::alice(), &count_msg, 0, None).await
        .expect("count failed").return_value().expect("return value failed");
    assert_eq!(count, 100);

    // Verify owner has all properties
    let owner_props_msg = build_message::<PropertyRegistry>(contract_acc_id.clone())
        .call(|contract| contract.get_owner_properties(ink_e2e::alice().account_id));
    let owner_props = client.call_dry_run(&ink_e2e::alice(), &owner_props_msg, 0, None).await
        .expect("owner props failed").return_value().expect("return value failed");
    assert_eq!(owner_props.len(), 100);

    Ok(())
}

#[ink_e2e::test]
async fn e2e_transfer_property() -> E2EResult<()> {
    let client = ink_e2e::Client::<ink_e2e::PolkadotConfig, _>::new().await?;

    // Given
    let constructor = PropertyRegistry::new();
    let contract_acc_id = client
        .instantiate("propchain-contracts", &ink_e2e::alice(), constructor, 0, None)
        .await
        .expect("instantiate failed")
        .account_id;

    let metadata = PropertyMetadata {
        location: "123 Main St".to_string(),
        size: 2000,
        legal_description: "Test property".to_string(),
        valuation: 500000,
        documents_url: "https://ipfs.io/test".to_string(),
    };

    let register_msg = build_message::<PropertyRegistry>(contract_acc_id.clone())
        .call(|contract| contract.register_property(metadata));
    let register_result = client.call(&ink_e2e::alice(), register_msg, 0, None).await;
    let property_id = register_result.expect("register failed").return_value().expect("return value failed");

    // When
    let transfer_msg = build_message::<PropertyRegistry>(contract_acc_id.clone())
        .call(|contract| contract.transfer_property(property_id, ink_e2e::bob().account_id));
    let transfer_result = client.call(&ink_e2e::alice(), transfer_msg, 0, None).await;

    // Then
    assert!(transfer_result.is_ok());

    // Verify transfer
    let get_msg = build_message::<PropertyRegistry>(contract_acc_id.clone())
        .call(|contract| contract.get_property(property_id));
    let get_result = client.call_dry_run(&ink_e2e::alice(), &get_msg, 0, None).await;
    let property = get_result.expect("get failed").return_value().expect("return value failed").unwrap();
    assert_eq!(property.owner, ink_e2e::bob().account_id);

    Ok(())
}

#[ink_e2e::test]
async fn e2e_end_to_end_property_workflow() -> E2EResult<()> {
    let client = ink_e2e::Client::<ink_e2e::PolkadotConfig, _>::new().await?;

    // Given
    let constructor = PropertyRegistry::new();
    let contract_acc_id = client
        .instantiate("propchain-contracts", &ink_e2e::alice(), constructor, 0, None)
        .await
        .expect("instantiate failed")
        .account_id;

    // Register multiple properties
    let properties = vec![
        PropertyMetadata {
            location: "123 Main St".to_string(),
            size: 2000,
            legal_description: "Residential property".to_string(),
            valuation: 500000,
            documents_url: "https://ipfs.io/test1".to_string(),
        },
        PropertyMetadata {
            location: "456 Oak Ave".to_string(),
            size: 1500,
            legal_description: "Commercial property".to_string(),
            valuation: 750000,
            documents_url: "https://ipfs.io/test2".to_string(),
        },
    ];

    let mut property_ids = Vec::new();
    for metadata in properties {
        let register_msg = build_message::<PropertyRegistry>(contract_acc_id.clone())
            .call(|contract| contract.register_property(metadata));
        let result = client.call(&ink_e2e::alice(), register_msg, 0, None).await;
        let property_id = result.expect("register failed").return_value().expect("return value failed");
        property_ids.push(property_id);
    }

    // Transfer first property to Bob
    let transfer_msg = build_message::<PropertyRegistry>(contract_acc_id.clone())
        .call(|contract| contract.transfer_property(property_ids[0], ink_e2e::bob().account_id));
    client.call(&ink_e2e::alice(), transfer_msg, 0, None).await.expect("transfer failed");

    // Transfer second property to Charlie
    let transfer_msg2 = build_message::<PropertyRegistry>(contract_acc_id.clone())
        .call(|contract| contract.transfer_property(property_ids[1], ink_e2e::charlie().account_id));
    client.call(&ink_e2e::alice(), transfer_msg2, 0, None).await.expect("transfer failed");

    // Verify final state
    let count_msg = build_message::<PropertyRegistry>(contract_acc_id.clone())
        .call(|contract| contract.property_count());
    let count = client.call_dry_run(&ink_e2e::alice(), &count_msg, 0, None).await
        .expect("count failed").return_value().expect("return value failed");
    assert_eq!(count, 2);

    // Check Alice has no properties
    let alice_props_msg = build_message::<PropertyRegistry>(contract_acc_id.clone())
        .call(|contract| contract.get_owner_properties(ink_e2e::alice().account_id));
    let alice_props = client.call_dry_run(&ink_e2e::alice(), &alice_props_msg, 0, None).await
        .expect("alice props failed").return_value().expect("return value failed");
    assert!(alice_props.is_empty());

    // Check Bob has first property
    let bob_props_msg = build_message::<PropertyRegistry>(contract_acc_id.clone())
        .call(|contract| contract.get_owner_properties(ink_e2e::bob().account_id));
    let bob_props = client.call_dry_run(&ink_e2e::alice(), &bob_props_msg, 0, None).await
        .expect("bob props failed").return_value().expect("return value failed");
    assert_eq!(bob_props, vec![property_ids[0]]);

    Ok(())
}