//! Load Tests for Property Transfer Operations
//!
//! This module contains load tests for property transfer operations
//! under high-traffic scenarios.

use crate::load_tests::*;
use ink_env::DefaultEnvironment;
use ink::env::test::{default_accounts, set_caller};
use propchain_contracts::propchain_contracts::PropertyRegistry as PropertyRegistryContract;
use propchain_traits::*;
use std::thread;
use std::time::{Duration, Instant};

/// Simulate concurrent property transfers
fn simulate_user_transfers(
    user_id: usize,
    num_transfers: usize,
    config: &LoadTestConfig,
    metrics: &LoadTestMetrics,
    registry: &mut PropertyRegistryContract,
    property_ids: &[u32],
) {
    let accounts = default_accounts::<DefaultEnvironment>();
    
    // Alternate between different account pairs
    let sender = match user_id % 2 {
        0 => accounts.alice,
        _ => accounts.bob,
    };
    
    let recipient = match user_id % 2 {
        0 => accounts.bob,
        _ => accounts.charlie,
    };
    
    set_caller::<DefaultEnvironment>(sender);

    for i in 0..num_transfers {
        if i >= property_ids.len() {
            break;
        }
        
        let start = Instant::now();
        let property_id = property_ids[i];
        
        let result = registry.transfer_property(property_id as u64, recipient);
        let elapsed = start.elapsed().as_millis();
        
        match result {
            Ok(_) => metrics.record_success(elapsed),
            Err(_) => metrics.record_failure(),
        }

        if config.operation_delay_ms > 0 {
            thread::sleep(Duration::from_millis(config.operation_delay_ms));
        }
    }
}

/// Test concurrent property transfers with light load
#[test]
fn load_test_concurrent_transfers_light() {
    // Setup: Register properties first
    let accounts = default_accounts::<DefaultEnvironment>();
    set_caller::<DefaultEnvironment>(accounts.alice);
    let mut registry = PropertyRegistryContract::new();
    
    let mut property_ids = Vec::new();
    for i in 0..50 {
        let metadata = generate_property_metadata(0, i);
        if let Ok(id) = registry.register_property(metadata) {
            property_ids.push(id);
        }
    }
    
    let config = LoadTestConfig::light();
    
    let metrics = run_concurrent_load_test(
        &config,
        "Concurrent Transfers - Light Load",
        move |user_id, cfg, m| {
            // Create a fresh registry instance for each thread
            let test_registry = PropertyRegistry::new();
            simulate_user_transfers(user_id, 10, cfg, m, &mut test_registry.clone(), &property_ids);
        },
    );
    
    assert_performance_thresholds(
        &metrics,
        "Light Load Transfers",
        400.0,  // max avg response 400ms
        95.0,   // min 95% success rate
        25.0,   // min 25 ops/sec
    );
}

/// Test concurrent property transfers with medium load
#[test]
fn load_test_concurrent_transfers_medium() {
    // Setup
    let accounts = default_accounts::<DefaultEnvironment>();
    set_caller::<DefaultEnvironment>(accounts.alice);
    let mut registry = PropertyRegistryContract::new();
    
    let mut property_ids = Vec::new();
    for i in 0..100 {
        let metadata = generate_property_metadata(0, i);
        if let Ok(id) = registry.register_property(metadata) {
            property_ids.push(id);
        }
    }
    
    let config = LoadTestConfig::medium();
    
    let metrics = run_concurrent_load_test(
        &config,
        "Concurrent Transfers - Medium Load",
        move |user_id, cfg, m| {
            let test_registry = PropertyRegistry::new();
            simulate_user_transfers(user_id, 20, cfg, m, &mut test_registry.clone(), &property_ids);
        },
    );
    
    assert_performance_thresholds(
        &metrics,
        "Medium Load Transfers",
        600.0,  // max avg response 600ms
        92.0,   // min 92% success rate
        40.0,   // min 40 ops/sec
    );
}

/// Stress test: Mass property transfers
#[test]
fn stress_test_mass_transfers() {
    // Setup
    let accounts = default_accounts::<DefaultEnvironment>();
    set_caller::<DefaultEnvironment>(accounts.alice);
    let mut registry = PropertyRegistryContract::new();
    
    let mut property_ids = Vec::new();
    for i in 0..200 {
        let metadata = generate_property_metadata(0, i);
        if let Ok(id) = registry.register_property(metadata) {
            property_ids.push(id);
        }
    }
    
    let config = LoadTestConfig::heavy();
    
    println!("⚠️ STRESS TEST: Mass transfer operations");
    
    let metrics = run_concurrent_load_test(
        &config,
        "Stress Test - Mass Transfers",
        move |user_id, cfg, m| {
            let test_registry = PropertyRegistry::new();
            simulate_user_transfers(user_id, 30, cfg, m, &mut test_registry.clone(), &property_ids);
        },
    );
    
    assert_performance_thresholds(
        &metrics,
        "Heavy Load Transfers",
        1500.0, // max avg response 1500ms
        88.0,   // min 88% success rate
        80.0,   // min 80 ops/sec
    );
}
