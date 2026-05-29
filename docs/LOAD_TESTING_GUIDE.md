# Load Testing Guide

## Overview

This guide provides comprehensive instructions for running, understanding, and extending the load testing framework for PropChain smart contracts.

## Table of Contents

1. [Quick Start](#quick-start)
2. [Load Testing Framework](#load-testing-framework)
3. [Test Categories](#test-categories)
4. [Running Load Tests](#running-load-tests)
5. [Interpreting Results](#interpreting-results)
6. [Performance Benchmarks](#performance-benchmarks)
7. [Troubleshooting](#troubleshooting)
8. [Best Practices](#best-practices)

---

## Quick Start

### Run All Load Tests

```bash
# Run all load tests (takes ~30 minutes)
cargo test --package propchain-tests --release

# Run with output
cargo test --package propchain-tests --release -- --nocapture
```

### Run Specific Test Categories

```bash
# Registration load tests (5-10 minutes)
cargo test --package propchain-tests load_test_concurrent_registration --release --nocapture

# Stress tests (10-15 minutes)
cargo test --package propchain-tests stress_test_ --release --nocapture

# Endurance tests (5-10 minutes)
cargo test --package propchain-tests endurance_test_short --release --nocapture

# Scalability tests (10-15 minutes)
cargo test --package propchain-tests scalability_test_memory_usage --release --nocapture
```

### Quick Performance Check

```bash
# Fast validation (2-3 minutes)
cargo test --package propchain-tests load_test_concurrent_registration_light --release --nocapture
```

---

## Load Testing Framework

### Architecture

The load testing framework consists of several components:

```
tests/
├── load_tests.rs                          # Core framework and utilities
├── load_test_property_registration.rs     # Registration-specific tests
├── load_test_property_transfer.rs         # Transfer-specific tests
├── load_test_endurance_spike.rs           # Endurance and spike tests
└── load_test_scalability.rs               # Scalability tests
```

### Key Components

#### 1. LoadTestConfig

Configuration for controlling test parameters:

```rust
pub struct LoadTestConfig {
    pub concurrent_users: usize,      // Number of simulated users
    pub duration_secs: u64,           // Test duration
    pub ramp_up_secs: u64,            // Gradual load increase period
    pub operation_delay_ms: u64,      // Delay between operations
    pub target_ops_per_second: usize, // Target throughput
}
```

**Predefined Configurations:**

- `Light()`: 5 users, 30 seconds - Quick validation
- `Medium()`: 20 users, 120 seconds - Standard testing
- `Heavy()`: 50 users, 300 seconds - Stress testing
- `Extreme()`: 100 users, 600 seconds - Breaking point

#### 2. LoadTestMetrics

Collects and analyzes performance metrics:

```rust
pub struct LoadTestMetrics {
    pub total_operations: Arc<Mutex<u64>>,
    pub successful_operations: Arc<Mutex<u64>>,
    pub failed_operations: Arc<Mutex<u64>>,
    pub total_response_time_ms: Arc<Mutex<u128>>,
    pub min_response_time_ms: Arc<Mutex<u128>>,
    pub max_response_time_ms: Arc<Mutex<u128>>,
    pub ops_per_second: Arc<Mutex<f64>>,
}
```

**Key Metrics:**

- **Success Rate**: Percentage of successful operations
- **Average Response Time**: Mean execution time
- **Min/Max Response Time**: Best/worst case latency
- **Operations per Second**: Throughput measurement

#### 3. Test Execution

```rust
run_concurrent_load_test(
    &config,
    "Test Name",
    |user_id, cfg, metrics| {
        // User simulation logic
    }
);
```

---

## Test Categories

### 1. Concurrent Load Tests

**Purpose**: Validate system behavior under simultaneous user load.

**Tests:**

- `load_test_concurrent_registration_light` - 5 users, light load
- `load_test_concurrent_registration_medium` - 20 users, medium load
- `load_test_concurrent_registration_heavy` - 50 users, heavy load
- `load_test_mixed_operations` - 70% reads, 30% writes

**When to Run:**

- After each feature development
- Before production deployments
- During performance optimization

**Expected Results:**

| Load Level | Success Rate | Avg Response | Min Ops/Sec |
|------------|--------------|--------------|-------------|
| Light      | >95%         | <500ms       | >20         |
| Medium     | >92%         | <750ms       | >50         |
| Heavy      | >90%         | <1000ms      | >100        |

### 2. Stress Tests

**Purpose**: Push system beyond normal capacity to find breaking points.

**Tests:**

- `stress_test_mass_registration` - 100 users, extreme load
- `stress_test_mass_transfers` - Mass transfer operations

**When to Run:**

- Monthly or quarterly
- Before major releases
- When scaling infrastructure

**Expected Results:**

| Metric | Threshold |
|--------|-----------|
| Success Rate | >85% |
| Avg Response | <2000ms |
| Min Ops/Sec | >200 |

### 3. Endurance Tests

**Purpose**: Detect memory leaks and performance degradation over time.

**Tests:**

- `endurance_test_sustained_load` - 5 minutes continuous load
- `endurance_test_short` - 1 minute (CI/CD friendly)

**When to Run:**

- Weekly in staging environment
- Before major deployments
- When investigating memory issues

**Expected Results:**

| Duration | Success Rate | Avg Response | Stability |
|----------|--------------|--------------|-----------|
| 1 min    | >96%         | <600ms       | Stable    |
| 5 min    | >95%         | <800ms       | No degradation |

Endurance runs now sample process RSS during execution and fail if memory growth exceeds the configured leak budget. This makes allocator growth and teardown leaks visible during long-running sessions instead of only at the end of the test.

### 4. Spike Tests

**Purpose**: Validate system resilience to sudden load changes.

**Tests:**

- `spike_test_sudden_load_increase` - 5 → 50 users suddenly
- `ramp_test_gradual_increase` - Gradual load increase

**When to Run:**

- Before high-traffic events
- When implementing auto-scaling
- Monthly validation

**Expected Results:**

| Phase | Max Degradation | Recovery |
|-------|-----------------|----------|
| Baseline | Normal | N/A |
| Spike | <5x slower | Maintains >85% success |
| Recovery | <1.5x baseline | Returns to normal |

### 5. Scalability Tests

**Purpose**: Understand how system scales with growth.

**Tests:**

- `scalability_test_growing_database` - 100 → 2000 properties
- `scalability_test_concurrent_users` - 5 → 40 users
- `scalability_test_memory_usage` - Memory growth analysis
- `scalability_test_storage_costs` - Storage cost projection

**When to Run:**

- Quarterly
- Before infrastructure planning
- When designing capacity upgrades

**Expected Results:**

| Scaling Type | Expected Pattern |
|--------------|------------------|
| Database Size | Linear or sub-linear query time |
| User Count | Reasonable throughput per user |
| Memory Usage | Linear growth with data |
| Storage | Linear bytes per property |

---

## Running Load Tests

### Basic Commands

```bash
# Single test
cargo test --package propchain-tests <test_name> --release --nocapture

# Multiple tests matching pattern
cargo test --package propchain-tests load_test_concurrent --release --nocapture

# With specific number of threads
cargo test --package propchain-tests <test_name> --release -- --test-threads=10
```

### Advanced Options

```bash
# Show stdout/stderr
cargo test --package propchain-tests <test_name> --release -- --nocapture

# Show timing information
cargo test --package propchain-tests <test_name> --release -- --show-output

# Run ignored tests (if any)
cargo test --package propchain-tests <test_name> --release --ignored

# Generate test report
cargo test --package propchain-tests <test_name> --release -- --format=json > results.json
```

### CI/CD Integration

```yaml
# .github/workflows/load-tests.yml
name: Load Tests

on:
  push:
    branches: [main, develop]
  pull_request:
    branches: [main]

jobs:
  load-tests:
    runs-on: ubuntu-latest
    
    steps:
    - uses: actions/checkout@v3
    
    - name: Install Rust
      uses: dtolnay/rust-action@stable
    
    - name: Run Load Tests
      run: cargo test --package propchain-tests --release
    
    - name: Upload Results
      uses: actions/upload-artifact@v3
      with:
        name: load-test-results
        path: target/release/.fingerprint/propchain-tests/
```

---

## Interpreting Results

### Sample Output

```
================================================================================
LOAD TEST RESULTS: Concurrent Registration - Medium Load
================================================================================
Total Operations:      240
Successful:            228 (95.00%)
Failed:                12
Avg Response Time:     687.42 ms
Min Response Time:     234 ms
Max Response Time:     1456 ms
Ops/Second:            52.18
================================================================================
```

### Understanding Metrics

#### Success Rate

- **>95%**: Excellent - System handling load well
- **90-95%**: Good - Minor issues under load
- **85-90%**: Fair - Some failures, investigate
- **<85%**: Poor - Significant problems, needs attention

#### Response Time

- **<500ms**: Excellent - Very responsive
- **500-750ms**: Good - Acceptable for most use cases
- **750-1000ms**: Fair - May need optimization
- **>1000ms**: Poor - Performance bottleneck detected

#### Throughput (Ops/Second)

Compare against `target_ops_per_second` in config:

- **>100% of target**: Exceeding expectations
- **80-100% of target**: Meeting expectations
- **<80% of target**: Below expectations, investigate bottlenecks

### Performance Threshold Validation

The framework automatically validates against thresholds:

```rust
assert_performance_thresholds(
    &metrics,
    "Test Name",
    750.0,  // max avg response ms
    92.0,   // min success rate %
    50.0,   // min ops/sec
);
```

**If thresholds fail:**

1. Check error logs for failure reasons
2. Review system resources (CPU, memory)
3. Identify bottlenecks using profiling tools
4. Compare with historical baselines

---

## Performance Benchmarks

### Baseline Metrics (Reference Hardware)

**Environment:**
- CPU: 8-core modern processor
- Memory: 16GB RAM
- Storage: SSD
- Network: Local (no network latency)

**Baseline Results:**

| Operation | Light Load | Medium Load | Heavy Load |
|-----------|------------|-------------|------------|
| Register Property | 350ms | 650ms | 950ms |
| Transfer Property | 280ms | 520ms | 780ms |
| Query Property | 45ms | 78ms | 120ms |
| Success Rate | 98% | 95% | 92% |

### Scaling Expectations

**User Scaling:**

| Users | Expected Throughput | Expected Latency |
|-------|---------------------|------------------|
| 5     | 25 ops/sec          | 300ms            |
| 10    | 50 ops/sec          | 400ms            |
| 20    | 90 ops/sec          | 600ms            |
| 40    | 160 ops/sec         | 850ms            |
| 50    | 180 ops/sec         | 1000ms           |

**Database Scaling:**

| Properties | Query Time | Growth Factor |
|------------|------------|---------------|
| 100       | 50ms       | 1.0x          |
| 500       | 55ms       | 1.1x          |
| 1000      | 62ms       | 1.24x         |
| 2000      | 75ms       | 1.5x          |

---

## Troubleshooting

### Common Issues

#### 1. High Failure Rate (>15%)

**Symptoms:**
- Success rate below 85%
- Many error messages in logs

**Possible Causes:**
- Insufficient system resources
- Contract state corruption
- Thread synchronization issues
- Gas limit exceeded

**Solutions:**
```bash
# Check system resources during test
htop  # Linux/Mac
tasklist  # Windows

# Reduce concurrent users
let config = LoadTestConfig {
    concurrent_users: 10,  // Reduce from 50
    ..LoadTestConfig::medium()
};

# Increase operation delay
let config = LoadTestConfig {
    operation_delay_ms: 200,  // Increase from 50
    ..LoadTestConfig::medium()
};
```

#### 2. High Latency (>2000ms avg)

**Symptoms:**
- Average response time exceeds thresholds
- Max response time very high (>5000ms)

**Possible Causes:**
- CPU contention
- Memory pressure
- Lock contention
- Inefficient contract code

**Solutions:**
```rust
// Profile to identify hotspots
cargo install flamegraph
cargo flamegraph -p propchain-tests endurance_test_short --test-threads=1

// Check for lock contention
// Look for long waits in mutex operations
```

#### 3. Low Throughput (<50% target)

**Symptoms:**
- Ops/sec significantly below target
- System appears underutilized

**Possible Causes:**
- Sequential bottlenecks
- Resource constraints
- Thread pool exhaustion
- I/O wait

**Solutions:**
```rust
// Increase test threads
cargo test --package propchain-tests <test_name> --release -- --test-threads=20

// Check thread utilization
// Monitor CPU usage during test
```

#### 4. Memory Issues

**Symptoms:**
- Tests slow down over time
- Out of memory errors
- Performance degradation in endurance tests

**Solutions:**
```bash
# Monitor memory usage
watch -n 1 'ps aux | grep propchain'

# Endurance tests now print peak RSS and memory growth in the test summary
./scripts/load_test.sh endurance

# Reduce test scale
let config = LoadTestConfig {
    concurrent_users: 5,  // Reduce load
    duration_secs: 30,    // Shorter test
    ..LoadTestConfig::default()
};
```

### Debugging Tips

#### Enable Detailed Logging

```rust
// In load_tests.rs, add logging
println!("User {} starting operation {}", user_id, op_num);
println!("Operation took {}ms", elapsed);
```

#### Isolate Issues

```bash
# Run single-threaded to eliminate concurrency issues
cargo test --package propchain-tests <test_name> --release -- --test-threads=1

# Run with specific user count
let config = LoadTestConfig {
    concurrent_users: 1,  // Single user
    ..LoadTestConfig::light()
};
```

#### Collect Metrics Over Time

```rust
// Add periodic reporting
use std::time::Instant;

let start = Instant::now();
loop {
    if start.elapsed().as_secs() % 10 == 0 {
        println!("10s elapsed: {} ops completed", *total_ops.lock().unwrap());
    }
    // ... test logic
}
```

---

## Best Practices

### 1. Test Environment

✅ **DO:**
- Use dedicated testing hardware
- Close unnecessary applications
- Ensure adequate cooling
- Use consistent hardware for comparisons
- Document environment specifications

❌ **DON'T:**
- Run on shared development machines
- Test while compiling other projects
- Run in resource-constrained VMs
- Change hardware between test runs

### 2. Test Configuration

✅ **DO:**
- Start with light load, gradually increase
- Include warm-up period
- Run multiple iterations
- Document configuration changes
- Use realistic operation delays

❌ **DON'T:**
- Jump straight to maximum load
- Skip ramp-up periods
- Run single iteration only
- Change configs mid-test
- Use zero delays (unrealistic)

### 3. Result Analysis

✅ **DO:**
- Compare against established baselines
- Look at all metrics (not just success rate)
- Analyze trends across runs
- Document anomalies
- Investigate outliers

❌ **DON'T:**
- Compare across different hardware
- Focus only on average response time
- Ignore failed operations
- Dismiss occasional failures
- Skip statistical analysis

### 4. Performance Optimization

✅ **DO:**
- Profile before optimizing
- Focus on bottlenecks
- Measure impact of changes
- Optimize common case first
- Consider trade-offs

❌ **DON'T:**
- Optimize prematurely
- Micro-optimize rare operations
- Ignore correctness for speed
- Optimize without measurements
- Forget about maintainability

### 5. Continuous Testing

✅ **DO:**
- Run light tests on every PR
- Run medium tests daily
- Run heavy tests weekly
- Run endurance tests monthly
- Track metrics over time

❌ **DON'T:**
- Skip load testing before releases
- Ignore failing tests
- Change test frequency arbitrarily
- Lose historical data
- Test only manually

---

## Extending the Framework

### Adding New Test Scenarios

```rust
#[test]
fn load_test_custom_scenario() {
    let config = LoadTestConfig {
        concurrent_users: 15,
        duration_secs: 60,
        ramp_up_secs: 10,
        operation_delay_ms: 100,
        target_ops_per_second: 100,
    };
    
    let metrics = run_concurrent_load_test(
        &config,
        "Custom Scenario",
        |user_id, cfg, m| {
            // Your custom simulation logic
            simulate_custom_operation(user_id, cfg, m);
        },
    );
    
    assert_performance_thresholds(
        &metrics,
        "Custom Scenario",
        500.0,  // max avg response
        95.0,   // min success rate
        50.0,   // min ops/sec
    );
}
```

### Custom Metrics Collection

```rust
pub struct CustomMetrics {
    // Add your custom metrics
    pub cache_hit_rate: Arc<Mutex<f64>>,
    pub gas_used: Arc<Mutex<u64>>,
}

impl CustomMetrics {
    pub fn record_cache_hit(&self) {
        // Implementation
    }
}
```

### Integration with Monitoring Tools

```rust
// Example: Send metrics to Prometheus
use prometheus::{register_counter, Counter};

fn register_prometheus_metrics() {
    lazy_static! {
        static ref OPS_TOTAL: Counter = register_counter!(
            "propchain_ops_total",
            "Total operations performed"
        ).unwrap();
    }
}
```

---

## Performance Tuning Guide

### Contract-Level Optimizations

1. **Minimize Storage Operations**
   - Batch storage writes
   - Use efficient data structures
   - Cache frequently accessed data

2. **Optimize Data Structures**
   - Use HashMap for O(1) lookups
   - Avoid nested mappings where possible
   - Keep values small and compact

3. **Reduce Computation**
   - Pre-compute values when possible
   - Use lazy evaluation
   - Avoid loops in hot paths

### Test-Level Optimizations

1. **Parallel Execution**
   ```rust
   // Increase parallelism
   cargo test --package propchain-tests --release -- --test-threads=20
   ```

2. **Efficient Setup**
   ```rust
   // Reuse setup across tests where possible
   lazy_static! {
       static ref SHARED_REGISTRY: PropertyRegistry = setup_registry();
   }
   ```

3. **Smart Delays**
   ```rust
   // Use adaptive delays based on system response
   let delay = if avg_response > 1000 {
       200  // Slow down under load
   } else {
       50   // Speed up when fast
   };
   ```

---

## Conclusion

Load testing is critical for ensuring PropChain can handle production workloads. This framework provides comprehensive tools for:

- Validating performance under various load conditions
- Identifying bottlenecks before they affect users
- Building confidence in system scalability
- Establishing performance baselines for regression detection

**Regular Testing Schedule:**

- **Every PR**: Light load tests
- **Daily**: Medium load tests
- **Weekly**: Heavy load + stress tests
- **Monthly**: Endurance + scalability tests
- **Quarterly**: Full performance audit

For questions or issues, please refer to the troubleshooting section or open an issue on GitHub.
