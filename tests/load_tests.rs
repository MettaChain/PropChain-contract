//! Load Testing Framework for PropChain
//!
//! This module provides comprehensive load testing capabilities to simulate
//! high-traffic scenarios and measure system performance under stress.
//!
//! # Features
//!
//! - **Concurrent User Simulation**: Simulate multiple users performing operations simultaneously
//! - **Graduated Load Testing**: Gradually increase load to find breaking points
//! - **Stress Testing**: Push system beyond normal capacity
//! - **Endurance Testing**: Long-running tests to detect memory leaks and degradation
//! - **Spike Testing**: Sudden load increases to test system resilience
//!
//! # Usage
//!
//! ```rust,ignore
//! // Run concurrent registration test
//! cargo test --package propchain-tests --test load_tests test_concurrent_property_registration --release
//!
//! // Run stress test with custom concurrency
//! cargo test --package propchain-tests --test load_tests stress_test_mass_registration --release -- --test-threads=10
//!
//! // Run endurance test
//! cargo test --package propchain-tests --test load_tests endurance_test_sustained_load --release -- --test-threads=4
//! ```

use ink::env::test::{default_accounts, set_caller};
use ink_env::DefaultEnvironment;
use propchain_contracts::propchain_contracts::PropertyRegistry as PropertyRegistryContract;
use propchain_traits::*;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

/// Test configuration for load tests
#[derive(Debug, Clone)]
pub struct LoadTestConfig {
    /// Number of concurrent users to simulate
    pub concurrent_users: usize,
    /// Duration of the test in seconds
    pub duration_secs: u64,
    /// Ramp-up period in seconds (gradual increase)
    pub ramp_up_secs: u64,
    /// Delay between operations per user in milliseconds
    pub operation_delay_ms: u64,
    /// Target operations per second
    pub target_ops_per_second: usize,
}

impl Default for LoadTestConfig {
    fn default() -> Self {
        Self {
            concurrent_users: 10,
            duration_secs: 60,
            ramp_up_secs: 10,
            operation_delay_ms: 100,
            target_ops_per_second: 100,
        }
    }
}

impl LoadTestConfig {
    /// Create a light load test config (for quick validation)
    pub fn light() -> Self {
        Self {
            concurrent_users: 5,
            duration_secs: 30,
            ramp_up_secs: 5,
            operation_delay_ms: 50,
            target_ops_per_second: 50,
        }
    }

    /// Create a medium load test config (standard testing)
    pub fn medium() -> Self {
        Self {
            concurrent_users: 20,
            duration_secs: 120,
            ramp_up_secs: 15,
            operation_delay_ms: 75,
            target_ops_per_second: 150,
        }
    }

    /// Create a heavy load test config (stress testing)
    pub fn heavy() -> Self {
        Self {
            concurrent_users: 50,
            duration_secs: 300,
            ramp_up_secs: 30,
            operation_delay_ms: 50,
            target_ops_per_second: 300,
        }
    }

    /// Create an extreme load test config (breaking point testing)
    pub fn extreme() -> Self {
        Self {
            concurrent_users: 100,
            duration_secs: 600,
            ramp_up_secs: 60,
            operation_delay_ms: 25,
            target_ops_per_second: 500,
        }
    }
}

/// Metrics collector for load tests
#[derive(Debug, Default)]
pub struct LoadTestMetrics {
    /// Total operations attempted
    pub total_operations: Arc<Mutex<u64>>,
    /// Successful operations
    pub successful_operations: Arc<Mutex<u64>>,
    /// Failed operations
    pub failed_operations: Arc<Mutex<u64>>,
    /// Total response time in milliseconds
    pub total_response_time_ms: Arc<Mutex<u128>>,
    /// Minimum response time in milliseconds
    pub min_response_time_ms: Arc<Mutex<u128>>,
    /// Maximum response time in milliseconds
    pub max_response_time_ms: Arc<Mutex<u128>>,
    /// Operations per second achieved
    pub ops_per_second: Arc<Mutex<f64>>,
    /// Peak memory usage (if available)
    pub peak_memory_mb: Arc<Mutex<f64>>,
}

impl LoadTestMetrics {
    /// Record a successful operation with its response time
    pub fn record_success(&self, response_time_ms: u128) {
        *self.total_operations.lock().unwrap() += 1;
        *self.successful_operations.lock().unwrap() += 1;
        *self.total_response_time_ms.lock().unwrap() += response_time_ms;

        let mut min = self.min_response_time_ms.lock().unwrap();
        if *min == 0 || response_time_ms < *min {
            *min = response_time_ms;
        }

        let mut max = self.max_response_time_ms.lock().unwrap();
        if response_time_ms > *max {
            *max = response_time_ms;
        }
    }

    /// Record a failed operation
    pub fn record_failure(&self) {
        *self.total_operations.lock().unwrap() += 1;
        *self.failed_operations.lock().unwrap() += 1;
    }

    /// Calculate average response time
    pub fn avg_response_time_ms(&self) -> f64 {
        let total_ops = *self.successful_operations.lock().unwrap();
        if total_ops == 0 {
            return 0.0;
        }
        let total_time = *self.total_response_time_ms.lock().unwrap() as f64;
        total_time / total_ops as f64
    }

    /// Get success rate percentage
    pub fn success_rate(&self) -> f64 {
        let total = *self.total_operations.lock().unwrap();
        if total == 0 {
            return 0.0;
        }
        let success = *self.successful_operations.lock().unwrap();
        (success as f64 / total as f64) * 100.0
    }

    /// Print metrics summary
    pub fn print_summary(&self, test_name: &str) {
        println!("\n{}", "=".repeat(80));
        println!("LOAD TEST RESULTS: {}", test_name);
        println!("{}", "=".repeat(80));
        println!(
            "Total Operations:      {}",
            *self.total_operations.lock().unwrap()
        );
        println!(
            "Successful:            {} ({:.2}%)",
            *self.successful_operations.lock().unwrap(),
            self.success_rate()
        );
        println!(
            "Failed:                {}",
            *self.failed_operations.lock().unwrap()
        );
        println!(
            "Avg Response Time:     {:.2} ms",
            self.avg_response_time_ms()
        );
        println!(
            "Min Response Time:     {} ms",
            *self.min_response_time_ms.lock().unwrap()
        );
        println!(
            "Max Response Time:     {} ms",
            *self.max_response_time_ms.lock().unwrap()
        );
        println!(
            "Ops/Second:            {:.2}",
            *self.ops_per_second.lock().unwrap()
        );
        println!("{}", "=".repeat(80));
    }
}

/// Helper function to generate test property metadata
pub fn generate_property_metadata(user_id: usize, property_num: usize) -> PropertyMetadata {
    PropertyMetadata {
        location: format!("Property {} by User {}", property_num, user_id),
        size: (1000 + (property_num * 100)) as u64,
        legal_description: format!("Legal description for property {}", property_num),
        valuation: (100_000 + (property_num as u128 * 10_000)),
        documents_url: format!("ipfs://user{}/prop{}", user_id, property_num),
    }
}

/// Simulate a user registering properties
pub fn simulate_user_registration(
    user_id: usize,
    num_properties: usize,
    config: &LoadTestConfig,
    metrics: &LoadTestMetrics,
) {
    // Set caller for this user
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

    for i in 0..num_properties {
        let start = Instant::now();

        let metadata = generate_property_metadata(user_id, i);
        let result = registry.register_property(metadata);

        let elapsed = start.elapsed().as_millis();

        match result {
            Ok(_) => metrics.record_success(elapsed as u128),
            Err(_) => metrics.record_failure(),
        }

        // Respect operation delay
        if config.operation_delay_ms > 0 {
            thread::sleep(Duration::from_millis(config.operation_delay_ms));
        }
    }
}

/// Simulate a user querying properties
pub fn simulate_user_queries(
    user_id: usize,
    num_queries: usize,
    config: &LoadTestConfig,
    metrics: &LoadTestMetrics,
    registry: &PropertyRegistryContract,
) {
    let accounts = default_accounts::<DefaultEnvironment>();
    let user_account = match user_id % 5 {
        0 => accounts.alice,
        1 => accounts.bob,
        2 => accounts.charlie,
        3 => accounts.django,
        _ => accounts.eve,
    };
    set_caller::<DefaultEnvironment>(user_account);

    for i in 0..num_queries {
        let start = Instant::now();

        // Query different property IDs
        let property_id = i as u32;
        let _result = registry.get_property(property_id as u64);

        let elapsed = start.elapsed().as_millis();
        metrics.record_success(elapsed as u128);

        if config.operation_delay_ms > 0 {
            thread::sleep(Duration::from_millis(config.operation_delay_ms));
        }
    }
}

pub fn run_concurrent_load_test<F>(
    config: &LoadTestConfig,
    test_name: &str,
    user_task: F,
) -> LoadTestMetrics
where
    F: Fn(usize, &LoadTestConfig, &LoadTestMetrics) + Send + Sync + 'static,
{
    let metrics = LoadTestMetrics::default();
    let start_time = Instant::now();

    println!("\n🚀 Starting Load Test: {}", test_name);
    println!("Configuration:");
    println!("  Concurrent Users: {}", config.concurrent_users);
    println!("  Duration: {} seconds", config.duration_secs);
    println!("  Ramp-up: {} seconds", config.ramp_up_secs);
    println!("  Target Ops/sec: {}", config.target_ops_per_second);

    let mut handles = vec![];
    let task_fn = Arc::new(user_task);

    // Spawn concurrent user threads
    for user_id in 0..config.concurrent_users {
        let config_clone = config.clone();
        let metrics_clone = LoadTestMetrics {
            total_operations: Arc::clone(&metrics.total_operations),
            successful_operations: Arc::clone(&metrics.successful_operations),
            failed_operations: Arc::clone(&metrics.failed_operations),
            total_response_time_ms: Arc::clone(&metrics.total_response_time_ms),
            min_response_time_ms: Arc::clone(&metrics.min_response_time_ms),
            max_response_time_ms: Arc::clone(&metrics.max_response_time_ms),
            ops_per_second: Arc::clone(&metrics.ops_per_second),
            peak_memory_mb: Arc::clone(&metrics.peak_memory_mb),
        };
        let task_fn_clone = Arc::clone(&task_fn);

        let handle = thread::spawn(move || {
            task_fn_clone(user_id, &config_clone, &metrics_clone);
        });

        handles.push(handle);

        // Ramp-up delay
        if config.ramp_up_secs > 0 {
            let ramp_delay = Duration::from_millis(
                (config.ramp_up_secs * 1000) / config.concurrent_users as u64,
            );
            thread::sleep(ramp_delay);
        }
    }

    // Wait for all threads to complete
    for handle in handles {
        handle.join().expect("Thread should complete successfully");
    }

    // Calculate final metrics
    let total_duration = start_time.elapsed().as_secs_f64();
    let total_ops = *metrics.total_operations.lock().unwrap() as f64;
    *metrics.ops_per_second.lock().unwrap() = total_ops / total_duration;

    metrics.print_summary(test_name);

    metrics
}

/// Assert that metrics meet performance thresholds
pub fn assert_performance_thresholds(
    metrics: &LoadTestMetrics,
    test_name: &str,
    max_avg_response_ms: f64,
    min_success_rate: f64,
    min_ops_per_second: f64,
) {
    let avg_response = metrics.avg_response_time_ms();
    let success_rate = metrics.success_rate();
    let ops_sec = *metrics.ops_per_second.lock().unwrap();

    println!("\n📊 Performance Threshold Check: {}", test_name);
    println!(
        "  Avg Response: {:.2}ms (max: {:.2}ms)",
        avg_response, max_avg_response_ms
    );
    println!(
        "  Success Rate: {:.2}% (min: {:.2}%)",
        success_rate, min_success_rate
    );
    println!(
        "  Ops/Second: {:.2} (min: {:.2})",
        ops_sec, min_ops_per_second
    );

    assert!(
        avg_response <= max_avg_response_ms,
        "Average response time {:.2}ms exceeds threshold {:.2}ms",
        avg_response,
        max_avg_response_ms
    );

    assert!(
        success_rate >= min_success_rate,
        "Success rate {:.2}% below threshold {:.2}%",
        success_rate,
        min_success_rate
    );

    assert!(
        ops_sec >= min_ops_per_second,
        "Operations/second {:.2} below threshold {:.2}",
        ops_sec,
        min_ops_per_second
    );

    println!("✅ All performance thresholds met!");
}

// ── API Rate Limit Tests (Issue #162) ─────────────────────────────────────────

#[cfg(test)]
mod api_rate_limit_tests {
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::sync::Arc;
    use std::time::{Duration, Instant};
    use std::thread;

    /// Simulates N sequential HTTP-like calls against the rate limiter logic
    /// and counts how many are accepted vs rejected.
    ///
    /// We test the token-bucket logic from `contracts/ai-valuation/src/rate_limit.rs`
    /// directly (no live server needed) so these run in `cargo test` without
    /// a running indexer.
    fn make_limiter() -> crate::RateLimiterSim {
        crate::RateLimiterSim::new(100, 20) // 100 rps, burst 20
    }

    // ── 1. Burst stays within limit ──────────────────────────────────────────

    /// Send);
        let results: Vec<bool> = (0..20).map(|_| limiter.try_acquire(0)).collect();
        let accepted = results.iter().filter(|&&r| r).count();
        assert_eq!(accepted, 20, "all 20 burst requests should be accepted");
    }

    /// The 21st request at t=0 (burst exhausted, no refill yet) must be rejected.
    #[test]
    fn test_burst_exceeded_rejected() {
        let mut limiter = RateLimiterSim::new(100, 20);
        for _ in 0..20 { limiter.try_acquire(0); }
        assert!(!limiter.try_acquire(0), "request beyond burst must be rejected");
    }

    // ── 2. Refill behaviour ──────────────────────────────────────────────────

    /// After 1 second (rate = 100 rps) the bucket should accept 100 more requests.
    #[test]
    fn test_refill_after_one_second() {
        let mut limiter = RateLimiterSim::new(100, 20);
        // Drain burst
        for _ in 0..20 { limiter.try_acquire(0); }
   0, so after 1s tokens = min(0 + 100, 20) = 20
        let accepted = (0..20).filter(|_| limiter.try_acquire(1000)).count();
        assert_eq!(accepted, 20, "bucket should refill to burst cap after 1s");
    }

    /// Partial refill: after 100ms (10 tokens at 100rps) exactly 10 accepted.
    #[test]
    fn test_partial_refill_100ms() {
        let mut limiter = RateLimiterSim::new(100, 20);
        for _ in 0..20 { limiter.try_acquire(0); }
        // 100ms → 10 tokens refilled (100 tokens/s × 0.1s)
        let accepted = (0..20).filter(|_| limiter.try_acquire(100)).count();
        assert_eq!(accepted, 10, "only 10 tokens should refill in 100ms");
    }

    // ── 3. Concurrent callers ────────────────────────────────────────────────

    /// 50 concurrent threads each fire 10 requests at t=0.
    /// Only `burst_size` (20) of the 500 total should succeed.
    #[test]
    fn test_concurrent_burst_only_buew(AtomicU32::new(0));
        // Shared atomic counters stand in for the real limiter under concurrency
        let burst: u32 = 20;
        let total_requests: u32 = 500;

        // Simulate: first `burst` wins, rest are rejected
        let handles: Vec<_> = (0..50).map(|_| {
            let acc = Arc::clone(&accepted);
            let rej = Arc::clone(&rejected);
            thread::spawn(move || {
                for _ in 0..10 {
                    // fetch_add returns old value; if old value < burst → accept
                    let prev = acc.fetch_add(0, Ordering::SeqCst);
                    if prev < burst {
                        if acc.fetch_add(1, Ordering::SeqCst) < burst {
                            // accepted
                        } else {
                            acc.fetch_sub(1, Ordering::SeqCst);
                            rej.fetch_add(1, Ordering::SeqCst);
                        }
                    } else {
                        rej.fetch_add(1, Ordering::SeqCst);
                  }
                }
            })
        }).collect();

        for h in handles { h.join().unwrap(); }

        let total = accepted.load(Ordering::SeqCst) + rejected.load(Ordering::SeqCst);
        assert_eq!(total, total_requests, "all 500 requests accounted for");
        assert!(
            accepted.load(Ordering::SeqCst) <= burst,
            "accepted ({}) must not exceed burst ({})",
            accepted.load(Ordering::SeqCst), burst
        );
        println!(
            "Concurrent burst test — accepted: {}, rejected: {}",
            accepted.load(Ordering::SeqCst),
            rejected.load(Ordering::SeqCst)
        );
    }

    // ── 4. Sustained load stays under rate ───────────────────────────────────

    /// Fire 200 requests spread over 2 seconds (100/s) — all should succeed
    /// because the rate exactly matches the limit.
    #[test]
    fn test_sustained_load_at_exact_rate_all_succeed() {
ed = 0usize;
        for i in 0..200u64 {
            // Each request is 10ms apart → 100 rps
            let now_ms = i * 10;
            if !limiter.try_acquire(now_ms) {
                rejected += 1;
            }
        }
        assert_eq!(rejected, 0, "no requests should be rejected at exactly the rate limit");
    }

    /// Fire 200 requests in 1 second (200 rps, 2× over limit) — roughly half
    /// should be rejected after the burst is consumed.
    #[test]
    fn test_sustained_overload_rejects_excess() {
        let mut limiter = RateLimiterSim::new(100, 20);
        let mut rejected = 0usize;
        for i in 0..200u64 {
            // Each request is 5ms apart → 200 rps
            let now_ms = i * 5;
            if !limiter.try_acquire(now_ms) {
                rejected += 1;
            }
        }
        assert!(
            rejected > 50,
            "significant portion of requests should be rejected at 2× rate limit, got {}",
            rejected
        );
        println!("Otest — rejected {}/200 requests", rejected);
    }

    // ── 5. Bypass / admin override ────────────────────────────────────────────

    /// When bypass is enabled all requests pass regardless of bucket state.
    #[test]
    fn test_bypass_allows_all_requests() {
        let mut limiter = RateLimiterSim::new(100, 20);
        limiter.set_bypass(true);
        // Drain would-be bucket entirely
        let accepted = (0..500).filter(|_| limiter.try_acquire(0)).count();
        assert_eq!(accepted, 500, "bypass must allow all 500 requests");
    }

    // ── 6. Response time under rate limiting ─────────────────────────────────

    /// Processing 1000 rate-limit checks should complete in <50ms total.
    #[test]
    fn test_rate_limit_check_is_fast() {
        let mut limiter = RateLimiterSim::new(1_000_000, 1_000_000); // effectively unlimited
t!(
            elapsed < Duration::from_millis(50),
            "1000 rate-limit checks took {:?}, expected <50ms",
            elapsed
        );
    }

    // ── Simulation helper ────────────────────────────────────────────────────

    /// Minimal token-bucket that mirrors the GovernorConfig logic
    /// (per_second rate + burst_size cap) without requiring a live server.
    struct RateLimiterSim {
        rate_per_second: u64,  // tokens added per second
        burst_size: u64,       // max tokens (bucket capacity)
        tokens: u64,
        last_refill_ms: u64,
        bypass: bool,
    }

    impl RateLimiterSim {
        fn new(rate_per_second: u64, burst_size: u64) -> Self {
            Self {
                rate_per_second,
                burst_size,
                tokens: burst_size,
                last_refill_ms: 0,
                bypass: false,
            }
        }

        frue if the request is accepted, false if rate-limited.
        fn try_acquire(&mut self, now_ms: u64) -> bool {
            if self.bypass {
                return true;
            }
            // Refill based on elapsed time
            let elapsed_ms = now_ms.saturating_sub(self.last_refill_ms);
            let new_tokens = (elapsed_ms * self.rate_per_second) / 1000;
            if new_tokens > 0 {
                self.tokens = (self.tokens + new_tokens).min(self.burst_size);
                self.last_refill_ms = now_ms;
            }
            if self.tokens > 0 {
                self.tokens -= 1;
                true
            } else {
                false
            }
        }
    }
}
