//! Load Tests for Property Registration
//!
//! This module contains load tests specifically for property registration
//! operations under various concurrent load scenarios.

use crate::load_tests::*;
use ink_env::DefaultEnvironment;
use ink::env::test::{default_accounts, set_caller};
use propchain_contracts::propchain_contracts::PropertyRegistry as PropertyRegistryContract;
use propchain_traits::*;

/// Test concurrent property registration with light load
#[test]
fn load_test_concurrent_registration_light() {
    let config = LoadTestConfig::light();
    
    let metrics = run_concurrent_load_test(
        &config,
        "Concurrent Registration - Light Load",
        |user_id, cfg, m| {
            simulate_user_registration(user_id, 10, cfg, m);
        },
    );
    
    assert_performance_thresholds(
        &metrics,
        "Light Load Registration",
        500.0,  // max avg response 500ms
        95.0,   // min 95% success rate
        20.0,   // min 20 ops/sec
    );
}

/// Test concurrent property registration with medium load
#[test]
fn load_test_concurrent_registration_medium() {
    let config = LoadTestConfig::medium();
    
    let metrics = run_concurrent_load_test(
        &config,
        "Concurrent Registration - Medium Load",
        |user_id, cfg, m| {
            simulate_user_registration(user_id, 20, cfg, m);
        },
    );
    
    assert_performance_thresholds(
        &metrics,
        "Medium Load Registration",
        750.0,  // max avg response 750ms
        92.0,   // min 92% success rate
        50.0,   // min 50 ops/sec
    );
}

/// Test concurrent property registration with heavy load
#[test]
fn load_test_concurrent_registration_heavy() {
    let config = LoadTestConfig::heavy();
    
    let metrics = run_concurrent_load_test(
        &config,
        "Concurrent Registration - Heavy Load",
        |user_id, cfg, m| {
            simulate_user_registration(user_id, 30, cfg, m);
        },
    );
    
    assert_performance_thresholds(
        &metrics,
        "Heavy Load Registration",
        1000.0, // max avg response 1000ms
        90.0,   // min 90% success rate
        100.0,  // min 100 ops/sec
    );
}

/// Stress test: Mass property registration
#[test]
fn stress_test_mass_registration() {
    let config = LoadTestConfig::extreme();
    
    println!("⚠️ STRESS TEST: Pushing system to extreme load");
    
    let metrics = run_concurrent_load_test(
        &config,
        "Stress Test - Mass Registration",
        |user_id, cfg, m| {
            simulate_user_registration(user_id, 50, cfg, m);
        },
    );
    
    // More lenient thresholds for stress test
    assert_performance_thresholds(
        &metrics,
        "Extreme Load Stress Test",
        2000.0, // max avg response 2000ms
        85.0,   // min 85% success rate
        200.0,  // min 200 ops/sec
    );
}

/// Test registration with mixed read/write operations
#[test]
fn load_test_mixed_operations() {
    let config = LoadTestConfig::medium();
    
    // First, register some properties
    let accounts = default_accounts::<DefaultEnvironment>();
    set_caller::<DefaultEnvironment>(accounts.alice);
    let mut registry = PropertyRegistryContract::new();
    
    println!("📦 Pre-registering properties for mixed test...");
    for i in 0..100 {
        let metadata = generate_property_metadata(0, i);
        registry.register_property(metadata).expect("Should register");
    }
    
    let metrics = LoadTestMetrics::default();
    let start_time = Instant::now();
    
    println!("🔄 Starting mixed operations load test...");
    
    // Simulate 70% reads, 30% writes
    let num_writers = (config.concurrent_users * 30) / 100;
    let num_readers = config.concurrent_users - num_writers;
    
    let mut handles = vec![];
    
    // Writer threads
    for user_id in 0..num_writers {
        let cfg = config.clone();
        let m = LoadTestMetrics {
            total_operations: Arc::clone(&metrics.total_operations),
            successful_operations: Arc::clone(&metrics.successful_operations),
            failed_operations: Arc::clone(&metrics.failed_operations),
            total_response_time_ms: Arc::clone(&metrics.total_response_time_ms),
            min_response_time_ms: Arc::clone(&metrics.min_response_time_ms),
            max_response_time_ms: Arc::clone(&metrics.max_response_time_ms),
            ops_per_second: Arc::clone(&metrics.ops_per_second),
            peak_memory_mb: Arc::clone(&metrics.peak_memory_mb),
        };
        
        let handle = thread::spawn(move || {
            simulate_user_registration(user_id, 15, &cfg, &m);
        });
        handles.push(handle);
    }
    
    // Reader threads
    for user_id in num_writers..(num_writers + num_readers) {
        let cfg = config.clone();
        let m = LoadTestMetrics {
            total_operations: Arc::clone(&metrics.total_operations),
            successful_operations: Arc::clone(&metrics.successful_operations),
            failed_operations: Arc::clone(&metrics.failed_operations),
            total_response_time_ms: Arc::clone(&metrics.total_response_time_ms),
            min_response_time_ms: Arc::clone(&metrics.min_response_time_ms),
            max_response_time_ms: Arc::clone(&metrics.max_response_time_ms),
            ops_per_second: Arc::clone(&metrics.ops_per_second),
            peak_memory_mb: Arc::clone(&metrics.peak_memory_mb),
        };
        
        let handle = thread::spawn(move || {
            simulate_user_queries(user_id, 30, &cfg, &m, &registry);
        });
        handles.push(handle);
    }
    
    // Wait for completion
    for handle in handles {
        handle.join().expect("Thread should complete");
    }
    
    let total_duration = start_time.elapsed().as_secs_f64();
    let total_ops = *metrics.total_operations.lock().unwrap() as f64;
    *metrics.ops_per_second.lock().unwrap() = total_ops / total_duration;
    
    metrics.print_summary("Mixed Operations Load Test");
    
    assert_performance_thresholds(
        &metrics,
        "Mixed Read/Write Operations",
        600.0,  // max avg response 600ms
        93.0,   // min 93% success rate
        80.0,   // min 80 ops/sec
    );
}
