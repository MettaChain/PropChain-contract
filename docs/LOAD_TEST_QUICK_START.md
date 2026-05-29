# Load Testing Quick Start Guide

## 🚀 Quick Start (2 minutes)

Run a quick validation test to verify system performance:

```bash
# Option 1: Using the load test script
./scripts/load_test.sh quick

# Option 2: Direct cargo command
cargo test --package propchain-tests load_test_concurrent_registration_light --release --nocapture
```

**Expected Results:**
- ✅ Success Rate: >95%
- ✅ Average Response: <500ms
- ✅ Throughput: >20 ops/sec

---

## 📋 Common Commands

### Daily Development

```bash
# Quick sanity check after code changes (2-3 min)
./scripts/load_test.sh quick

# Standard performance validation (10-15 min)
./scripts/load_test.sh standard
```

### Before Merging PRs

```bash
# Run medium load tests
cargo test --package propchain-tests load_test_concurrent_registration_medium --release --nocapture
```

### Weekly Performance Review

```bash
# Full test suite (30+ min)
./scripts/load_test.sh full

# Or run specific categories
./scripts/load_test.sh stress      # Stress tests
./scripts/load_test.sh endurance   # Endurance tests  
./scripts/load_test.sh scalability # Scalability tests
```

---

## 📊 Understanding Results

### Sample Output

```
================================================================================
LOAD TEST RESULTS: Concurrent Registration - Light Load
================================================================================
Total Operations:      50
Successful:            49 (98.00%)
Failed:                1
Avg Response Time:     387.42 ms
Min Response Time:     234 ms
Max Response Time:     678 ms
Ops/Second:            23.45
================================================================================

📊 Performance Threshold Check: Light Load Registration
  Avg Response: 387.42ms (max: 500.00ms) ✓
  Success Rate: 98.00% (min: 95.00%) ✓
  Ops/Second: 23.45 (min: 20.00) ✓
✅ All performance thresholds met!
```

### Performance Thresholds

| Load Level | Success Rate | Avg Response | Min Ops/Sec |
|------------|--------------|--------------|-------------|
| **Light**  | >95%         | <500ms       | >20         |
| **Medium** | >92%         | <750ms       | >50         |
| **Heavy**  | >90%         | <1000ms      | >100        |
| **Stress** | >85%         | <2000ms      | >200        |

---

## 🎯 Test Categories

### 1. Quick Tests (2-5 minutes)

```bash
# Light load validation
./scripts/load_test.sh quick

# Single user scenario
cargo test load_test_concurrent_registration_light --release --nocapture
```

**When to use:** After code changes, quick validation

### 2. Standard Tests (10-15 minutes)

```bash
# Medium load scenarios
./scripts/load_test.sh standard

# All registration tests
cargo test load_test_concurrent_registration --release --nocapture
```

**When to use:** Before merging PRs, regular development

### 3. Stress Tests (15-20 minutes)

```bash
# Breaking point testing
./scripts/load_test.sh stress

# Mass operations
cargo test stress_test_mass_registration --release --nocapture
```

**When to use:** Monthly validation, before major releases

### 4. Endurance Tests (5-10 minutes)

```bash
# Sustained load testing
./scripts/load_test.sh endurance

# Short endurance for CI/CD
cargo test endurance_test_short --release --nocapture

# Sustained endurance with leak monitoring
cargo test endurance_test_sustained_load --release --nocapture
```

**When to use:** Weekly in staging, before deployments

### 5. Scalability Tests (10-15 minutes)

```bash
# Growth analysis
./scripts/load_test.sh scalability

# Database scaling
cargo test scalability_test_growing_database --release --nocapture

# Memory usage analysis
cargo test scalability_test_memory_usage --release --nocapture
```

**When to use:** Quarterly, capacity planning

---

## 🔍 Troubleshooting

### Test Fails Immediately

**Problem:** Test crashes or fails to start

**Solution:**
```bash
# Check Rust version
rustc --version  # Should be 1.70+

# Update toolchain
rustup update

# Clean and rebuild
cargo clean
cargo build --package propchain-tests --release
```

### High Failure Rate (>15%)

**Problem:** Many operations failing

**Solutions:**
1. Reduce concurrent users:
```rust
let config = LoadTestConfig {
    concurrent_users: 5,  // Reduce from higher value
    ..LoadTestConfig::medium()
};
```

2. Increase operation delay:
```rust
let config = LoadTestConfig {
    operation_delay_ms: 200,  // Increase from 50
    ..LoadTestConfig::medium()
};
```

### High Latency (>2000ms)

**Problem:** Operations taking too long

**Solutions:**
1. Check system resources:
```bash
# Monitor CPU/memory
htop  # Linux/Mac
tasklist  # Windows
```

2. Reduce load:
```bash
# Run lighter test first
./scripts/load_test.sh quick
```

3. Profile to find bottlenecks:
```bash
cargo install flamegraph
cargo flamegraph -p propchain-tests endurance_test_short
```

### Low Throughput (<50% target)

**Problem:** Not enough operations per second

**Solutions:**
1. Increase test threads:
```bash
cargo test <test_name> --release -- --test-threads=20
```

2. Check for sequential bottlenecks
3. Review contract gas optimization

---

## 📈 Performance Baselines

### Reference Environment

**Hardware:**
- CPU: 8-core modern processor
- Memory: 16GB RAM
- Storage: SSD

**Software:**
- Rust: 1.70+
- ink!: 5.0.0

### Expected Metrics

| Operation | Light | Medium | Heavy |
|-----------|-------|--------|-------|
| Register  | 350ms | 650ms  | 950ms |
| Transfer  | 280ms | 520ms  | 780ms |
| Query     | 45ms  | 78ms   | 120ms |
| Success % | 98%   | 95%    | 92%   |

---

## 🎓 Learning Resources

### Documentation

- **[Load Testing Guide](LOAD_TESTING_GUIDE.md)** - Comprehensive guide with all details
- **[Monitoring Guide](LOAD_TEST_MONITORING.md)** - Metrics and alerting setup
- **[Implementation Summary](LOAD_TEST_IMPLEMENTATION_SUMMARY.md)** - Technical details

### Video Tutorials (Coming Soon)

- Introduction to Load Testing
- Running Your First Test
- Analyzing Results
- Performance Optimization

### Examples

See test files for working examples:
- `tests/load_tests.rs` - Core framework
- `tests/load_test_property_registration.rs` - Registration examples
- `tests/load_test_property_transfer.rs` - Transfer examples

---

## ⚡ Advanced Usage

### Custom Test Configuration

```rust
use crate::load_tests::*;

#[test]
fn custom_load_test() {
    let config = LoadTestConfig {
        concurrent_users: 15,
        duration_secs: 60,
        ramp_up_secs: 10,
        operation_delay_ms: 100,
        target_ops_per_second: 100,
    };
    
    let metrics = run_concurrent_load_test(
        &config,
        "Custom Test",
        |user_id, cfg, m| {
            // Your simulation logic
        },
    );
}
```

### CI/CD Integration

```yaml
# .github/workflows/load-tests.yml
name: Load Tests

on:
  push:
    branches: [main]
  schedule:
    - cron: '0 2 * * *'  # Daily at 2 AM

jobs:
  load-tests:
    runs-on: ubuntu-latest
    steps:
    - uses: actions/checkout@v3
    - name: Run Load Tests
      run: cargo test --package propchain-tests --release
```

### Custom Metrics

```rust
pub struct CustomMetrics {
    pub cache_hits: Arc<Mutex<u64>>,
    pub gas_used: Arc<Mutex<u64>>,
}

impl CustomMetrics {
    pub fn record_cache_hit(&self) {
        *self.cache_hits.lock().unwrap() += 1;
    }
}
```

---

## 🤝 Best Practices

### DO ✅

- Run light tests after every code change
- Include performance thresholds in CI/CD
- Document baseline metrics for your hardware
- Investigate failures immediately
- Track trends over time
- Test on dedicated hardware when possible

### DON'T ❌

- Skip load testing before releases
- Ignore failing tests
- Test on shared development machines
- Change hardware between test runs
- Focus only on average response time
- Dismiss occasional failures

---

## 📞 Support

### Getting Help

1. **Documentation:** See comprehensive guides in `docs/` folder
2. **Examples:** Check test files for working implementations
3. **Issues:** Open GitHub issue for bugs or questions
4. **Discussions:** Join PropChain developer community

### Common Questions

**Q: How often should I run load tests?**  
A: Light tests after code changes, standard tests weekly, full tests monthly.

**Q: What if tests pass but production is slow?**  
A: Check hardware differences, network latency, and database size.

**Q: Can I run tests in parallel?**  
A: Yes, but use separate test databases to avoid conflicts.

**Q: How do I compare results?**  
A: Use the monitoring guide to establish baselines and track trends.

---

## 🎯 Next Steps

1. **Start Simple:** Run `./scripts/load_test.sh quick`
2. **Review Results:** Compare against performance thresholds
3. **Explore:** Try different test categories
4. **Integrate:** Add to your CI/CD pipeline
5. **Monitor:** Set up continuous performance tracking

**Ready?** Let's run your first load test! 🚀

```bash
./scripts/load_test.sh quick
```

For detailed information, see the full [Load Testing Guide](LOAD_TESTING_GUIDE.md).
