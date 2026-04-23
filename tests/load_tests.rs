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
    use std::thread;
    use std::time::{Duration, Instant};

    // ── 1. Burst stays within limit ──────────────────────────────────────────

    /// All 20 burst requests at t=0 should be accepted.
    #[test]
    fn test_burst_within_limit() {
        let mut limiter = RateLimiterSim::new(100, 20);
        let results: Vec<bool> = (0..20).map(|_| limiter.try_acquire(0)).collect();
        let accepted = results.iter().filter(|&&r| r).count();
        assert_eq!(accepted, 20, "all 20 burst requests should be accepted");
    }

    /// The 21st request at t=0 (burst exhausted, no refill yet) must be rejected.
    #[test]
    fn test_burst_exceeded_rejected() {
        let mut limiter = RateLimiterSim::new(100, 20);
        for _ in 0..20 {
            limiter.try_acquire(0);
        }
        assert!(
            !limiter.try_acquire(0),
            "request beyond burst must be rejected"
        );
    }

    // ── 2. Refill behaviour ──────────────────────────────────────────────────

    /// After 1 second (rate = 100 rps) the bucket refills to burst_size.
    #[test]
    fn test_refill_after_one_second() {
        let mut limiter = RateLimiterSim::new(100, 20);
        for _ in 0..20 {
            limiter.try_acquire(0);
        }
        // tokens = 0; after 1s: min(0 + 100, 20) = 20
        let accepted = (0..20).filter(|_| limiter.try_acquire(1000)).count();
        assert_eq!(accepted, 20, "bucket should refill to burst cap after 1s");
    }

    /// Partial refill: after 100ms (10 tokens at 100rps) exactly 10 accepted.
    #[test]
    fn test_partial_refill_100ms() {
        let mut limiter = RateLimiterSim::new(100, 20);
        for _ in 0..20 {
            limiter.try_acquire(0);
        }
        // 100ms → 10 tokens refilled (100 tokens/s × 0.1s)
        let accepted = (0..20).filter(|_| limiter.try_acquire(100)).count();
        assert_eq!(accepted, 10, "only 10 tokens should refill in 100ms");
    }

    // ── 3. Concurrent callers ────────────────────────────────────────────────

    /// 50 concurrent threads each fire 10 requests at t=0.
    /// Only `burst_size` (20) of the 500 total should succeed.
    #[test]
    fn test_concurrent_burst_only() {
        let accepted = Arc::new(AtomicU32::new(0));
        let rejected = Arc::new(AtomicU32::new(0));
        let burst: u32 = 20;
        let total_requests: u32 = 500;

        let handles: Vec<_> = (0..50)
            .map(|_| {
                let acc = Arc::clone(&accepted);
                let rej = Arc::clone(&rejected);
                thread::spawn(move || {
                    for _ in 0..10 {
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
            })
            .collect();

        for h in handles {
            h.join().unwrap();
        }

        let total = accepted.load(Ordering::SeqCst) + rejected.load(Ordering::SeqCst);
        assert_eq!(total, total_requests, "all 500 requests accounted for");
        assert!(
            accepted.load(Ordering::SeqCst) <= burst,
            "accepted ({}) must not exceed burst ({})",
            accepted.load(Ordering::SeqCst),
            burst
        );
        println!(
            "Concurrent burst test — accepted: {}, rejected: {}",
            accepted.load(Ordering::SeqCst),
            rejected.load(Ordering::SeqCst)
        );
    }

    // ── 4. Sustained load stays under rate ───────────────────────────────────

    /// Fire 200 requests spread over 2 seconds (100/s) — all should succeed.
    #[test]
    fn test_sustained_load_at_exact_rate_all_succeed() {
        let mut limiter = RateLimiterSim::new(100, 20);
        let mut rejected = 0usize;
        for i in 0..200u64 {
            // Each request is 10ms apart → 100 rps
            let now_ms = i * 10;
            if !limiter.try_acquire(now_ms) {
                rejected += 1;
            }
        }
        assert_eq!(
            rejected, 0,
            "no requests should be rejected at exactly the rate limit"
        );
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
        println!("Overload test — rejected {}/200 requests", rejected);
    }

    // ── 5. Bypass / admin override ────────────────────────────────────────────

    /// When bypass is enabled all requests pass regardless of bucket state.
    #[test]
    fn test_bypass_allows_all_requests() {
        let mut limiter = RateLimiterSim::new(100, 20);
        limiter.set_bypass(true);
        let accepted = (0..500).filter(|_| limiter.try_acquire(0)).count();
        assert_eq!(accepted, 500, "bypass must allow all 500 requests");
    }

    // ── 6. Response time under rate limiting ─────────────────────────────────

    /// Processing 1000 rate-limit checks should complete in <50ms total.
    #[test]
    fn test_rate_limit_check_is_fast() {
        let mut limiter = RateLimiterSim::new(1_000_000, 1_000_000); // effectively unlimited
        let start = Instant::now();
        for i in 0..1000u64 {
            limiter.try_acquire(i);
        }
        let elapsed = start.elapsed();
        assert!(
            elapsed < Duration::from_millis(50),
            "1000 rate-limit checks took {:?}, expected <50ms",
            elapsed
        );
    }

    // ── Simulation helper ────────────────────────────────────────────────────

    struct RateLimiterSim {
        rate_per_second: u64,
        burst_size: u64,
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

        fn set_bypass(&mut self, bypass: bool) {
            self.bypass = bypass;
        }

        fn try_acquire(&mut self, now_ms: u64) -> bool {
            if self.bypass {
                return true;
            }
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

// ── Concurrent User Simulation Tests (Issue #157) ─────────────────────────────

#[cfg(test)]
mod concurrent_user_simulation_tests {
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::sync::{Arc, Mutex};
    use std::thread;
    use std::time::Instant;

    /// Shared ledger simulating on-chain state under concurrent access.
    struct MockLedger {
        balances: Mutex<Vec<u64>>,
        tx_count: AtomicU64,
        failed_count: AtomicU64,
    }

    impl MockLedger {
        fn new(accounts: usize, initial_balance: u64) -> Self {
            Self {
                balances: Mutex::new(vec![initial_balance; accounts]),
                tx_count: AtomicU64::new(0),
                failed_count: AtomicU64::new(0),
            }
        }

        /// Attempt a transfer from `from` to `to`. Returns true if successful.
        fn transfer(&self, from: usize, to: usize, amount: u64) -> bool {
            let mut balances = self.balances.lock().unwrap();
            if balances[from] >= amount {
                balances[from] -= amount;
                balances[to] += amount;
                self.tx_count.fetch_add(1, Ordering::SeqCst);
                true
            } else {
                self.failed_count.fetch_add(1, Ordering::SeqCst);
                false
            }
        }

        fn total_supply(&self) -> u64 {
            self.balances.lock().unwrap().iter().sum()
        }
    }

    /// Spawn 100 concurrent users each performing 10 transfers and verify
    /// that total supply is conserved (no double-spend or lost tokens).
    #[test]
    fn test_concurrent_transfers_conserve_supply() {
        const ACCOUNTS: usize = 10;
        const INITIAL: u64 = 1_000;
        const USERS: usize = 100;
        const OPS_PER_USER: usize = 10;

        let ledger = Arc::new(MockLedger::new(ACCOUNTS, INITIAL));
        let expected_supply = INITIAL * ACCOUNTS as u64;

        let handles: Vec<_> = (0..USERS)
            .map(|uid| {
                let l = Arc::clone(&ledger);
                thread::spawn(move || {
                    for op in 0..OPS_PER_USER {
                        let from = (uid + op) % ACCOUNTS;
                        let to = (uid + op + 1) % ACCOUNTS;
                        l.transfer(from, to, 10);
                    }
                })
            })
            .collect();

        for h in handles {
            h.join().expect("user thread should not panic");
        }

        assert_eq!(
            ledger.total_supply(),
            expected_supply,
            "total supply must be conserved across {} concurrent users",
            USERS
        );
        println!(
            "Concurrent transfers: {} succeeded, {} failed (insufficient balance)",
            ledger.tx_count.load(Ordering::SeqCst),
            ledger.failed_count.load(Ordering::SeqCst),
        );
    }

    /// Verify throughput: 500 concurrent operations complete in under 2 seconds.
    #[test]
    fn test_concurrent_throughput_500_users() {
        const ACCOUNTS: usize = 20;
        const USERS: usize = 500;

        let ledger = Arc::new(MockLedger::new(ACCOUNTS, 10_000));
        let start = Instant::now();

        let handles: Vec<_> = (0..USERS)
            .map(|uid| {
                let l = Arc::clone(&ledger);
                thread::spawn(move || {
                    let from = uid % ACCOUNTS;
                    let to = (uid + 1) % ACCOUNTS;
                    l.transfer(from, to, 1);
                })
            })
            .collect();

        for h in handles {
            h.join().expect("thread should not panic");
        }

        let elapsed = start.elapsed();
        let total_ops =
            ledger.tx_count.load(Ordering::SeqCst) + ledger.failed_count.load(Ordering::SeqCst);
        assert_eq!(
            total_ops, USERS as u64,
            "all user operations must be recorded"
        );
        assert!(
            elapsed.as_secs() < 2,
            "500 concurrent ops took {:?}, expected < 2s",
            elapsed
        );
        println!("Throughput test: {} ops in {:?}", total_ops, elapsed);
    }

    /// Simulate graduated ramp-up: users join in waves and contention increases.
    #[test]
    fn test_graduated_ramp_up() {
        const ACCOUNTS: usize = 5;
        const WAVES: usize = 5;
        const USERS_PER_WAVE: usize = 20;

        let ledger = Arc::new(MockLedger::new(ACCOUNTS, 100_000));
        let initial_supply = ledger.total_supply();

        for wave in 0..WAVES {
            let handles: Vec<_> = (0..USERS_PER_WAVE)
                .map(|uid| {
                    let l = Arc::clone(&ledger);
                    thread::spawn(move || {
                        let from = (wave + uid) % ACCOUNTS;
                        let to = (wave + uid + 1) % ACCOUNTS;
                        l.transfer(from, to, 50);
                    })
                })
                .collect();
            for h in handles {
                h.join().expect("wave thread should not panic");
            }
        }

        assert_eq!(
            ledger.total_supply(),
            initial_supply,
            "supply must be conserved across graduated load waves"
        );
    }

    /// Verify that read operations (balance queries) don't deadlock under
    /// heavy concurrent write pressure.
    #[test]
    fn test_reads_do_not_deadlock_under_write_pressure() {
        const ACCOUNTS: usize = 8;
        const WRITERS: usize = 50;
        const READERS: usize = 50;

        let ledger = Arc::new(MockLedger::new(ACCOUNTS, 5_000));

        let mut handles = vec![];

        for uid in 0..WRITERS {
            let l = Arc::clone(&ledger);
            handles.push(thread::spawn(move || {
                let from = uid % ACCOUNTS;
                let to = (uid + 1) % ACCOUNTS;
                l.transfer(from, to, 10);
            }));
        }

        for _ in 0..READERS {
            let l = Arc::clone(&ledger);
            handles.push(thread::spawn(move || {
                let _supply = l.total_supply();
            }));
        }

        for h in handles {
            h.join().expect("thread should not deadlock");
        }
    }
}

// ── Network Partition Simulation Tests (Issue #160) ────────────────────────────

#[cfg(test)]
mod network_partition_simulation_tests {
    use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
    use std::sync::{Arc, Mutex};
    use std::thread;
    use std::time::Duration;

    /// Simulates a node that can be partitioned from the network.
    struct PartitionableNode {
        id: usize,
        partitioned: AtomicBool,
        committed_ops: AtomicU64,
        rejected_ops: AtomicU64,
        state: Mutex<u64>,
    }

    impl PartitionableNode {
        fn new(id: usize, initial_state: u64) -> Arc<Self> {
            Arc::new(Self {
                id,
                partitioned: AtomicBool::new(false),
                committed_ops: AtomicU64::new(0),
                rejected_ops: AtomicU64::new(0),
                state: Mutex::new(initial_state),
            })
        }

        fn partition(&self) {
            self.partitioned.store(true, Ordering::SeqCst);
        }

        fn reconnect(&self) {
            self.partitioned.store(false, Ordering::SeqCst);
        }

        fn is_partitioned(&self) -> bool {
            self.partitioned.load(Ordering::SeqCst)
        }

        /// Attempt to apply an operation. Fails if node is partitioned.
        fn apply_op(&self, delta: i64) -> bool {
            if self.is_partitioned() {
                self.rejected_ops.fetch_add(1, Ordering::SeqCst);
                return false;
            }
            let mut s = self.state.lock().unwrap();
            if delta < 0 && (*s as i64) < (-delta) {
                self.rejected_ops.fetch_add(1, Ordering::SeqCst);
                return false;
            }
            *s = ((*s as i64) + delta) as u64;
            self.committed_ops.fetch_add(1, Ordering::SeqCst);
            true
        }

        fn state(&self) -> u64 {
            *self.state.lock().unwrap()
        }
    }

    /// Test that operations submitted to a partitioned node are rejected
    /// and that no state mutation occurs during the partition.
    #[test]
    fn test_ops_rejected_during_partition() {
        let node = PartitionableNode::new(0, 1_000);

        // Apply some ops before partition
        for _ in 0..10 {
            node.apply_op(10);
        }
        let state_before_partition = node.state();

        node.partition();

        // All ops during partition must fail
        for _ in 0..20 {
            let ok = node.apply_op(5);
            assert!(!ok, "partitioned node must reject operations");
        }

        assert_eq!(
            node.state(),
            state_before_partition,
            "state must not change while node is partitioned"
        );
        assert_eq!(node.rejected_ops.load(Ordering::SeqCst), 20);
    }

    /// After a partition heals, the node should accept operations again
    /// and accumulate correct state.
    #[test]
    fn test_ops_resume_after_partition_heals() {
        let node = PartitionableNode::new(1, 500);

        node.partition();
        for _ in 0..5 {
            node.apply_op(100);
        }
        let state_during = node.state();

        node.reconnect();

        for _ in 0..5 {
            node.apply_op(100);
        }

        assert_eq!(
            state_during, 500,
            "state must be unchanged during partition"
        );
        assert_eq!(
            node.state(),
            1000,
            "state must advance by 5×100 after reconnect"
        );
    }

    /// Concurrent writes to multiple nodes where a subset are partitioned;
    /// verifies that only connected nodes accumulate state.
    #[test]
    fn test_partial_network_partition_with_concurrent_writers() {
        const NODES: usize = 6;
        const WRITERS_PER_NODE: usize = 20;

        let nodes: Vec<Arc<PartitionableNode>> =
            (0..NODES).map(|i| PartitionableNode::new(i, 0)).collect();

        // Partition the first half of nodes
        for node in &nodes[..NODES / 2] {
            node.partition();
        }

        let mut handles = vec![];
        for node in &nodes {
            for _ in 0..WRITERS_PER_NODE {
                let n = Arc::clone(node);
                handles.push(thread::spawn(move || {
                    n.apply_op(1);
                }));
            }
        }
        for h in handles {
            h.join().expect("writer thread should not panic");
        }

        // Partitioned nodes should have 0 committed ops, connected nodes should have WRITERS_PER_NODE
        for (i, node) in nodes.iter().enumerate() {
            if i < NODES / 2 {
                assert_eq!(
                    node.committed_ops.load(Ordering::SeqCst),
                    0,
                    "node {} is partitioned and must have 0 committed ops",
                    i
                );
                assert_eq!(
                    node.state(),
                    0,
                    "partitioned node {} state must remain 0",
                    i
                );
            } else {
                assert_eq!(
                    node.committed_ops.load(Ordering::SeqCst),
                    WRITERS_PER_NODE as u64,
                    "connected node {} must have all {} ops committed",
                    i,
                    WRITERS_PER_NODE
                );
            }
        }
    }

    /// Simulate a rolling network partition: nodes are partitioned and
    /// reconnected in sequence while concurrent writes continue.
    #[test]
    fn test_rolling_partition_and_reconnect() {
        let node = PartitionableNode::new(0, 10_000);

        let node_ref = Arc::clone(&node);
        let writer = thread::spawn(move || {
            for i in 0..100u64 {
                // Interleave with small delays to allow partition toggling
                if i % 10 == 0 {
                    thread::sleep(Duration::from_millis(1));
                }
                node_ref.apply_op(1);
            }
        });

        // Toggle partition while writer is running
        for _ in 0..5 {
            node.partition();
            thread::sleep(Duration::from_millis(2));
            node.reconnect();
            thread::sleep(Duration::from_millis(2));
        }

        writer.join().expect("writer should complete");

        let committed = node.committed_ops.load(Ordering::SeqCst);
        let rejected = node.rejected_ops.load(Ordering::SeqCst);
        assert_eq!(
            committed + rejected,
            100,
            "all 100 operations must be accounted for (committed + rejected = 100)"
        );
        assert_eq!(
            node.state(),
            10_000 + committed,
            "state must equal initial + committed ops"
        );
        println!(
            "Rolling partition: {} committed, {} rejected during partition windows",
            committed, rejected
        );
    }

    /// Test that a split-brain scenario (two partitioned halves of the network)
    /// does not violate consistency on either side.
    #[test]
    fn test_split_brain_no_double_commit() {
        // Two nodes starting with the same state, both receive the same operations
        // but one is partitioned — only one should apply the changes.
        let primary = PartitionableNode::new(0, 1_000);
        let secondary = PartitionableNode::new(1, 1_000);

        secondary.partition();

        // Apply 10 ops to both; only primary should accept
        let mut primary_commits = 0u64;
        for _ in 0..10 {
            if primary.apply_op(100) {
                primary_commits += 1;
            }
            secondary.apply_op(100);
        }

        assert_eq!(primary_commits, 10);
        assert_eq!(secondary.committed_ops.load(Ordering::SeqCst), 0);
        assert_eq!(
            secondary.state(),
            1_000,
            "partitioned secondary must not change state"
        );
        assert_eq!(primary.state(), 2_000, "primary must reflect all 10 ops");
    }
}
