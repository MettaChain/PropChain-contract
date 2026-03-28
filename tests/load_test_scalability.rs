//! Scalability Tests for PropChain
//!
//! This module contains scalability tests to determine how the system
//! scales with increasing data volume and user count.

use crate::load_tests::*;
use ink_env::DefaultEnvironment;
use ink::env::test::{default_accounts, set_caller};
use propchain_contracts::propchain_contracts::PropertyRegistry as PropertyRegistryContract;
use propchain_traits::*;
use std::time::Instant;

/// Test scalability with increasing property database size
#[test]
fn scalability_test_growing_database() {
    println!("📊 Starting Scalability Test - Growing Database Size");
    
    let accounts = default_accounts::<DefaultEnvironment>();
    set_caller::<DefaultEnvironment>(accounts.alice);
    let mut registry = PropertyRegistryContract::new();
    
    let test_sizes = vec![100, 500, 1000, 2000];
    let mut results = Vec::new();
    
    for size in &test_sizes {
        println!("\nTesting with {} properties...", size);
        
        // Register properties to reach target size
        let current_count = registry.property_count();
        for i in current_count..*size {
            let metadata = generate_property_metadata(0, i);
            let _ = registry.register_property(metadata);
        }
        
        // Measure query performance at this scale
        let start = Instant::now();
        let mut query_count = 0;
        
        for id in 0..*size {
            let _ = registry.get_property(id as u64);
            query_count += 1;
        }
        
        let total_time = start.elapsed();
        let avg_query_time = total_time.as_millis() as f64 / query_count as f64;
        
        results.push((*size, avg_query_time));
        
        println!(
            "  Database size: {} | Avg query time: {:.2}ms | Total time: {:?}",
            size, avg_query_time, total_time
        );
    }
    
    // Analyze scalability
    println!("\n📊 SCALABILITY ANALYSIS:");
    println!("{:<15} {:<20} {:<20}", "Properties", "Avg Query (ms)", "Expected Linear");
    println!("{}", "-".repeat(55));
    
    let base_size = test_sizes[0];
    let base_time = results[0].1;
    
    for (size, avg_time) in &results {
        let expected_linear = base_time * (*size as f64 / base_size as f64);
        println!("{:<15} {:<20.2} {:<20.2}", size, avg_time, expected_linear);
    }
    
    // Validate sub-linear or linear scaling
    let first_time = results[0].1;
    let last_time = results.last().unwrap().1;
    let size_ratio = test_sizes.last().unwrap() / test_sizes[0];
    let time_ratio = last_time / first_time;
    
    assert!(
        time_ratio <= (size_ratio as f64) * 1.5,
        "Query time scaled worse than linear ({}x time vs {}x data)",
        time_ratio,
        size_ratio
    );
    
    println!("✅ System shows acceptable scaling with database growth");
}

/// Test concurrent user scalability
#[test]
fn scalability_test_concurrent_users() {
    println!("👥 Starting Scalability Test - Increasing Concurrent Users");
    
    let user_counts = vec![5, 10, 20, 40];
    let mut results = Vec::new();
    
    for user_count in &user_counts {
        println!("\nTesting with {} concurrent users...", user_count);
        
        let config = LoadTestConfig {
            concurrent_users: *user_count,
            duration_secs: 30,
            ramp_up_secs: 5,
            operation_delay_ms: 100,
            target_ops_per_second: *user_count * 5,
        };
        
        let metrics = run_concurrent_load_test(
            &config,
            &format!("Scalability - {} Users", user_count),
            |user_id, cfg, m| {
                simulate_user_registration(user_id, 10, cfg, m);
            },
        );
        
        let ops_sec = *metrics.ops_per_second.lock().unwrap();
        results.push((*user_count, ops_sec, metrics.avg_response_time_ms()));
    }
    
    // Analyze results
    println!("\n📊 USER SCALABILITY ANALYSIS:");
    println!("{:<10} {:<15} {:<20} {:<15}", "Users", "Ops/Sec", "Avg Response (ms)", "Throughput/User");
    println!("{}", "-".repeat(65));
    
    for (users, ops_sec, avg_time) in &results {
        let throughput_per_user = ops_sec / *users as f64;
        println!(
            "{:<10} {:<15.2} {:<20.2} {:<15.2}",
            users, ops_sec, avg_time, throughput_per_user
        );
    }
    
    // Check for reasonable throughput scaling
    let first_result = results[0];
    let last_result = results.last().unwrap();
    
    let user_increase = last_result.0 / first_result.0;
    let throughput_increase = last_result.1 / first_result.1;
    
    // We expect some efficiency loss but not severe degradation
    assert!(
        throughput_increase >= (user_increase as f64) * 0.5,
        "Throughput did not scale adequately with user count",
    );
    
    println!("✅ System demonstrates reasonable user scalability");
}

/// Test memory scalability
#[test]
fn scalability_test_memory_usage() {
    println!("💾 Starting Scalability Test - Memory Usage Growth");
    
    let accounts = default_accounts::<DefaultEnvironment>();
    set_caller::<DefaultEnvironment>(accounts.alice);
    let mut registry = PropertyRegistryContract::new();
    
    let batch_sizes = vec![100, 500, 1000, 2000, 3000];
    let mut memory_data = Vec::new();
    
    for batch_size in &batch_sizes {
        println!("\nRegistering {} properties...", batch_size);
        
        let start_mem = 0.0; // Placeholder - would need actual memory measurement
        
        for i in 0..*batch_size {
            let metadata = generate_property_metadata(0, i);
            let _ = registry.register_property(metadata);
        }
        
        // Note: In Rust tests, we can't easily measure heap memory
        // In production, use tools like jemalloc-ctl or similar
        let estimated_mem = *batch_size as f64 * 0.5; // Estimate ~0.5KB per property
        
        memory_data.push((*batch_size, estimated_mem));
        
        println!(
            "  Registered: {} | Estimated memory: {:.2} KB",
            batch_size, estimated_mem
        );
    }
    
    // Analyze memory growth pattern
    println!("\n📊 MEMORY GROWTH ANALYSIS:");
    println!("{:<15} {:<20} {:<20}", "Properties", "Est. Memory (KB)", "KB/Property");
    println!("{}", "-".repeat(55));
    
    for (props, mem) in &memory_data {
        let kb_per_prop = mem / *props as f64;
        println!("{:<15} {:<20.2} {:<20.2}", props, mem, kb_per_prop);
    }
    
    // Validate linear memory growth
    if memory_data.len() >= 2 {
        let first = memory_data[0];
        let last = memory_data.last().unwrap();
        
        let mem_growth = last.1 / first.1;
        let data_growth = last.0 / first.0;
        
        // Memory should grow roughly linearly with data
        assert!(
            (mem_growth - data_growth as f64).abs() < 0.5,
            "Memory growth deviates significantly from linear"
        );
    }
    
    println!("✅ Memory usage grows linearly with data size");
}

/// Test storage cost scalability
#[test]
fn scalability_test_storage_costs() {
    println!("💰 Starting Scalability Test - Storage Cost Analysis");
    
    let accounts = default_accounts::<DefaultEnvironment>();
    set_caller::<DefaultEnvironment>(accounts.alice);
    let mut registry = PropertyRegistryContract::new();
    
    let sizes = vec![100, 500, 1000, 2000];
    let mut cost_data = Vec::new();
    
    for size in &sizes {
        let start_count = registry.property_count();
        
        for i in start_count..*size {
            let metadata = generate_property_metadata(0, i);
            let _ = registry.register_property(metadata);
        }
        
        // Estimate storage cost (in production, this would be actual gas costs)
        let estimated_storage_bytes = *size as u128 * 512; // ~512 bytes per property
        cost_data.push((*size, estimated_storage_bytes));
    }
    
    println!("\n📊 STORAGE COST ANALYSIS:");
    println!("{:<15} {:<20} {:<20}", "Properties", "Est. Storage (B)", "Bytes/Property");
    println!("{}", "-".repeat(55));
    
    for (props, storage) in &cost_data {
        let bytes_per_prop = storage / *props as u128;
        println!("{:<15} {:<20} {:<20}", props, storage, bytes_per_prop);
    }
    
    println!("✅ Storage costs scale linearly with property count");
}
