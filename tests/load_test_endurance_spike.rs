//! Endurance and Spike Load Tests
//!
//! This module contains endurance tests (long-running) and spike tests
//! (sudden load increases) to validate system stability.

use crate::load_tests::*;
use ink_env::DefaultEnvironment;
use ink::env::test::{default_accounts, set_caller};
use propchain_contracts::propchain_contracts::PropertyRegistry as PropertyRegistryContract;
use propchain_traits::*;
use std::thread;
use std::time::{Duration, Instant};

/// Simulate sustained load for extended period
fn simulate_sustained_load(
    user_id: usize,
    duration_secs: u64,
    config: &LoadTestConfig,
    metrics: &LoadTestMetrics,
) {
    let start = Instant::now();
    let mut ops_count = 0;
    
    while start.elapsed() < Duration::from_secs(duration_secs) {
        // Register a property
        let accounts = default_accounts::<DefaultEnvironment>();
        let user_account = match user_id % 5 {
            0 => accounts.alice,
            1 => accounts.bob,
            2 => accounts.charlie,
            3 => accounts.django,
            _ => accounts.eve,
        };
        set_caller::<DefaultEnvironment>(user_account);
        
        let mut registry = PropertyRegistryContract::new();
        let metadata = generate_property_metadata(user_id, ops_count);
        
        let op_start = Instant::now();
        match registry.register_property(metadata) {
            Ok(_) => metrics.record_success(op_start.elapsed().as_millis()),
            Err(_) => metrics.record_failure(),
        }
        
        ops_count += 1;
        
        if config.operation_delay_ms > 0 {
            thread::sleep(Duration::from_millis(config.operation_delay_ms));
        }
    }
}

/// Endurance test: Sustained load for extended period
#[test]
fn endurance_test_sustained_load() {
    println!("🏃 Starting Endurance Test - 5 minutes sustained load");
    
    let config = LoadTestConfig {
        concurrent_users: 10,
        duration_secs: 300, // 5 minutes
        ramp_up_secs: 30,
        operation_delay_ms: 200,
        target_ops_per_second: 50,
    };
    
    let metrics = run_concurrent_load_test(
        &config,
        "Endurance Test - 5 Minute Sustained Load",
        |user_id, cfg, m| {
            simulate_sustained_load(user_id, 300, cfg, m);
        },
    );
    
    // Check for performance degradation over time
    assert_performance_thresholds(
        &metrics,
        "Endurance Test (5 min)",
        800.0,  // max avg response 800ms
        95.0,   // min 95% success rate
        30.0,   // min 30 ops/sec
    );
    
    println!("✅ System maintained stable performance under sustained load");
}

/// Short endurance test for CI/CD (1 minute)
#[test]
fn endurance_test_short() {
    println!("⏱️ Starting Short Endurance Test - 1 minute");
    
    let config = LoadTestConfig {
        concurrent_users: 15,
        duration_secs: 60,
        ramp_up_secs: 10,
        operation_delay_ms: 100,
        target_ops_per_second: 100,
    };
    
    let metrics = run_concurrent_load_test(
        &config,
        "Endurance Test - 1 Minute",
        |user_id, cfg, m| {
            simulate_sustained_load(user_id, 60, cfg, m);
        },
    );
    
    assert_performance_thresholds(
        &metrics,
        "Endurance Test (1 min)",
        600.0,  // max avg response 600ms
        96.0,   // min 96% success rate
        80.0,   // min 80 ops/sec
    );
}

/// Spike test: Sudden load increase
#[test]
fn spike_test_sudden_load_increase() {
    println!("📈 Starting Spike Test - Sudden load from 5 to 50 users");
    
    // Phase 1: Low load baseline
    println!("Phase 1: Establishing baseline (5 users)...");
    let baseline_config = LoadTestConfig {
        concurrent_users: 5,
        duration_secs: 30,
        ramp_up_secs: 5,
        operation_delay_ms: 200,
        target_ops_per_second: 25,
    };
    
    let baseline_metrics = run_concurrent_load_test(
        &baseline_config,
        "Spike Test - Baseline",
        |user_id, cfg, m| {
            simulate_sustained_load(user_id, 30, cfg, m);
        },
    );
    
    // Phase 2: Sudden spike to high load
    println!("Phase 2: Spiking to 50 users...");
    let spike_config = LoadTestConfig {
        concurrent_users: 50,
        duration_secs: 60,
        ramp_up_secs: 5, // Very fast ramp-up
        operation_delay_ms: 50,
        target_ops_per_second: 500,
    };
    
    let spike_metrics = run_concurrent_load_test(
        &spike_config,
        "Spike Test - High Load",
        |user_id, cfg, m| {
            simulate_sustained_load(user_id, 60, cfg, m);
        },
    );
    
    // Phase 3: Return to normal load
    println!("Phase 3: Returning to normal load...");
    let recovery_config = LoadTestConfig {
        concurrent_users: 5,
        duration_secs: 30,
        ramp_up_secs: 5,
        operation_delay_ms: 200,
        target_ops_per_second: 25,
    };
    
    let recovery_metrics = run_concurrent_load_test(
        &recovery_config,
        "Spike Test - Recovery",
        |user_id, cfg, m| {
            simulate_sustained_load(user_id, 30, cfg, m);
        },
    );
    
    // Analyze results
    println!("\n📊 SPIKE TEST ANALYSIS:");
    println!("Baseline Performance:");
    println!("  Avg Response: {:.2}ms", baseline_metrics.avg_response_time_ms());
    println!("  Success Rate: {:.2}%", baseline_metrics.success_rate());
    
    println!("\nSpike Performance:");
    println!("  Avg Response: {:.2}ms", spike_metrics.avg_response_time_ms());
    println!("  Success Rate: {:.2}%", spike_metrics.success_rate());
    
    println!("\nRecovery Performance:");
    println!("  Avg Response: {:.2}ms", recovery_metrics.avg_response_time_ms());
    println!("  Success Rate: {:.2}%", recovery_metrics.success_rate());
    
    // Validate system handled the spike
    let spike_degradation = spike_metrics.avg_response_time_ms() / baseline_metrics.avg_response_time_ms();
    assert!(
        spike_degradation < 5.0,
        "Performance degraded too much during spike ({}x slower)",
        spike_degradation
    );
    
    // Validate recovery
    let recovery_ratio = recovery_metrics.avg_response_time_ms() / baseline_metrics.avg_response_time_ms();
    assert!(
        recovery_ratio < 1.5,
        "System did not recover properly (still {}x slower than baseline)",
        recovery_ratio
    );
    
    println!("✅ System successfully handled load spike and recovered");
}

/// Gradual load increase test (ramp test)
#[test]
fn ramp_test_gradual_increase() {
    println!("📈 Starting Ramp Test - Gradual load increase from 5 to 30 users");
    
    let stages = vec![
        (5, 30),   // 5 users for 30 seconds
        (10, 30),  // 10 users for 30 seconds
        (20, 30),  // 20 users for 30 seconds
        (30, 60),  // 30 users for 60 seconds
    ];
    
    let mut all_metrics = Vec::new();
    
    for (users, duration) in stages {
        println!("Stage: {} users for {} seconds", users, duration);
        
        let config = LoadTestConfig {
            concurrent_users: users,
            duration_secs: duration,
            ramp_up_secs: 5,
            operation_delay_ms: 150,
            target_ops_per_second: users * 5,
        };
        
        let metrics = run_concurrent_load_test(
            &config,
            &format!("Ramp Test - {} Users", users),
            |user_id, cfg, m| {
                simulate_sustained_load(user_id, duration, cfg, m);
            },
        );
        
        all_metrics.push((users, metrics));
    }
    
    // Print ramp analysis
    println!("\n📊 RAMP TEST ANALYSIS:");
    println!("{:<10} {:<15} {:<15} {:<15}", "Users", "Avg Response", "Success Rate", "Ops/Sec");
    println!("{}", "-".repeat(55));
    
    for (users, metrics) in &all_metrics {
        println!(
            "{:<10} {:<15.2} {:<15.2} {:<15.2}",
            users,
            metrics.avg_response_time_ms(),
            metrics.success_rate(),
            *metrics.ops_per_second.lock().unwrap()
        );
    }
    
    // Validate graceful degradation
    let first_metric = &all_metrics[0].1;
    let last_metric = &all_metrics.last().unwrap().1;
    
    let load_increase = last_metric.success_rate() / first_metric.success_rate();
    assert!(
        load_increase > 0.8,
        "Success rate dropped too significantly under increased load"
    );
    
    println!("✅ System showed graceful degradation under increasing load");
}
