# Load Test Monitoring and Reporting Guide

## Overview

This guide provides comprehensive instructions for monitoring load tests, analyzing results, and creating performance reports for PropChain smart contracts.

---

## Table of Contents

1. [Monitoring Dashboard](#monitoring-dashboard)
2. [Key Performance Indicators](#key-performance-indicators)
3. [Real-time Monitoring](#real-time-monitoring)
4. [Performance Report Template](#performance-report-template)
5. [Trend Analysis](#trend-analysis)
6. [Alert Configuration](#alert-configuration)
7. [Capacity Planning](#capacity-planning)

---

## Monitoring Dashboard

### Essential Metrics to Track

#### System-Level Metrics

| Metric | Description | Tool | Threshold |
|--------|-------------|------|-----------|
| CPU Usage | Processor utilization | htop, top | <80% |
| Memory Usage | RAM consumption | free, Task Manager | <85% |
| Disk I/O | Storage operations | iostat, Resource Monitor | <70% capacity |
| Thread Count | Active threads | ps, Task Manager | Stable growth |

#### Application-Level Metrics

| Metric | Description | Importance | Target |
|--------|-------------|------------|--------|
| Success Rate | % successful operations | Critical | >95% |
| Avg Response Time | Mean execution time | High | <750ms |
| P95 Response Time | 95th percentile latency | High | <1500ms |
| P99 Response Time | 99th percentile latency | Medium | <2000ms |
| Throughput | Operations per second | High | >50 ops/sec |
| Error Rate | % failed operations | Critical | <5% |

#### Business-Level Metrics

| Metric | Description | Formula | Target |
|--------|-------------|---------|--------|
| User Capacity | Max concurrent users | Derived from load tests | >50 users |
| Property Scale | Max properties in DB | From scalability tests | >10,000 |
| Cost per Operation | Gas/resource cost | Total cost / ops | Minimize |
| Degradation Rate | Performance over time | (End - Start) / Start | <10% |

### Dashboard Example

```
┌─────────────────────────────────────────────────────────────┐
│              PROPCHAIN LOAD TEST DASHBOARD                  │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  Current Test: Concurrent Registration - Medium Load        │
│  Duration: 00:02:15 / 00:02:00                              │
│  Concurrent Users: 20                                       │
│                                                             │
│  ┌──────────────────┐  ┌──────────────────┐                │
│  │ SUCCESS RATE     │  │ THROUGHPUT       │                │
│  │    94.2% ✓       │  │  52.3 ops/sec ✓  │                │
│  │  Target: >92%    │  │  Target: >50     │                │
│  └──────────────────┘  └──────────────────┘                │
│                                                             │
│  ┌──────────────────┐  ┌──────────────────┐                │
│  │ AVG RESPONSE     │  │ ACTIVE USERS     │                │
│  │   687 ms ✓       │  │      20          │                │
│  │  Target: <750ms  │  │                  │                │
│  └──────────────────┘  └──────────────────┘                │
│                                                             │
│  Response Time Distribution:                                │
│  ├▓▓▓▓▓▓▓▓░░░░░░░░░░┤ P50: 456ms                           │
│  ├▓▓▓▓▓▓▓▓▓▓░░░░░░░┤ P95: 1234ms                           │
│  ├▓▓▓▓▓▓▓▓▓▓▓▓░░░░░┤ P99: 1678ms                           │
│                                                             │
│  Recent Errors: 12 (5.8%)                                   │
│  └─ Contract execution failed: 8                            │
│  └─ Timeout: 4                                              │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

---

## Key Performance Indicators (KPIs)

### KPI Definitions

#### 1. Success Rate

**Formula:** `(Successful Ops / Total Ops) × 100`

**Measurement:**
```rust
let success_rate = metrics.success_rate();
println!("Success Rate: {:.2}%", success_rate);
```

**Targets:**
- 🟢 Excellent: >98%
- 🟡 Good: 95-98%
- 🟠 Fair: 90-95%
- 🔴 Poor: <90%

#### 2. Average Response Time

**Formula:** `Total Response Time / Successful Operations`

**Measurement:**
```rust
let avg_response = metrics.avg_response_time_ms();
println!("Avg Response: {:.2}ms", avg_response);
```

**Targets by Operation:**
- Registration: <750ms
- Transfer: <600ms
- Query: <150ms
- Batch (10 items): <2000ms

#### 3. Throughput

**Formula:** `Total Operations / Test Duration (seconds)`

**Measurement:**
```rust
let throughput = *metrics.ops_per_second.lock().unwrap();
println!("Throughput: {:.2} ops/sec", throughput);
```

**Targets:**
- Light load: >20 ops/sec
- Medium load: >50 ops/sec
- Heavy load: >100 ops/sec
- Stress: >200 ops/sec

#### 4. P95 Latency

**Formula:** 95th percentile of all response times

**Calculation:**
```rust
fn calculate_percentile(mut times: Vec<u128>, percentile: f64) -> u128 {
    times.sort();
    let index = ((percentile / 100.0) * times.len() as f64) as usize;
    times[index.min(times.len() - 1)]
}

let p95 = calculate_percentile(response_times, 95.0);
```

**Target:** <1500ms

#### 5. Scalability Index

**Formula:** `(Throughput at 2x users) / (Throughput at 1x users)`

**Interpretation:**
- >1.8: Excellent linear scaling
- 1.5-1.8: Good scaling
- 1.2-1.5: Fair scaling
- <1.2: Poor scaling, bottlenecks present

---

## Real-time Monitoring

### Live Metrics Collection

```rust
use std::time::Instant;
use std::thread;

pub struct LiveMonitor {
    start_time: Instant,
    last_report: Instant,
    report_interval_secs: u64,
}

impl LiveMonitor {
    pub fn new(report_interval_secs: u64) -> Self {
        Self {
            start_time: Instant::now(),
            last_report: Instant::now(),
            report_interval_secs,
        }
    }
    
    pub fn update(&mut self, metrics: &LoadTestMetrics) {
        if self.last_report.elapsed().as_secs() >= self.report_interval_secs {
            self.print_status(metrics);
            self.last_report = Instant::now();
        }
    }
    
    fn print_status(&self, metrics: &LoadTestMetrics) {
        let elapsed = self.start_time.elapsed().as_secs();
        let total_ops = *metrics.total_operations.lock().unwrap();
        let success_ops = *metrics.successful_operations.lock().unwrap();
        let current_throughput = total_ops as f64 / elapsed as f64;
        
        println!(
            "[{:02}:{:02}] Ops: {} | Success: {} ({:.1}%) | Throughput: {:.1} ops/sec",
            elapsed / 60,
            elapsed % 60,
            total_ops,
            success_ops,
            (success_ops as f64 / total_ops as f64) * 100.0,
            current_throughput
        );
    }
}

// Usage in load test
#[test]
fn load_test_with_monitoring() {
    let config = LoadTestConfig::medium();
    let metrics = LoadTestMetrics::default();
    let mut monitor = LiveMonitor::new(5); // Report every 5 seconds
    
    let start = Instant::now();
    
    // Run test in background thread
    let metrics_clone = /* ... */;
    thread::spawn(move || {
        run_concurrent_load_test(&config, "Test", |user_id, cfg, m| {
            simulate_user_registration(user_id, 20, cfg, m);
        });
    });
    
    // Monitor in main thread
    while start.elapsed().as_secs() < config.duration_secs {
        thread::sleep(Duration::from_secs(1));
        monitor.update(&metrics);
    }
}
```

### Memory Leak Detection

For long-running sessions, sample resident set size during the run so leak growth becomes visible before the test finishes.

```rust
if let Some(rss_mb) = current_process_memory_mb() {
    metrics.record_peak_memory_mb(rss_mb);
}

// Compare peak RSS against the baseline captured at test start.
// Stable endurance runs should stay within the configured leak budget.
```

Suggested leak budgets:
- Short endurance runs: peak RSS growth under 24 MB
- Sustained endurance runs: peak RSS growth under 32 MB
- Final RSS drift after teardown: close to baseline

### Alert Conditions

Configure alerts for immediate notification of issues:

```rust
pub struct AlertConfig {
    pub min_success_rate: f64,
    pub max_avg_response_ms: f64,
    pub min_throughput: f64,
    pub max_error_burst: usize, // consecutive errors
}

impl AlertConfig {
    pub fn check(&self, metrics: &LoadTestMetrics) -> Vec<String> {
        let mut alerts = Vec::new();
        
        let success_rate = metrics.success_rate();
        if success_rate < self.min_success_rate {
            alerts.push(format!(
                "🚨 CRITICAL: Success rate dropped to {:.1}% (min: {:.1}%)",
                success_rate, self.min_success_rate
            ));
        }
        
        let avg_response = metrics.avg_response_time_ms();
        if avg_response > self.max_avg_response_ms {
            alerts.push(format!(
                "⚠️ WARNING: High latency {:.0}ms (max: {:.0}ms)",
                avg_response, self.max_avg_response_ms
            ));
        }
        
        let throughput = *metrics.ops_per_second.lock().unwrap();
        if throughput < self.min_throughput {
            alerts.push(format!(
                "⚠️ WARNING: Low throughput {:.1} ops/sec (min: {:.1})",
                throughput, self.min_throughput
            ));
        }
        
        alerts
    }
}

// Default alert thresholds
impl Default for AlertConfig {
    fn default() -> Self {
        Self {
            min_success_rate: 90.0,
            max_avg_response_ms: 1500.0,
            min_throughput: 30.0,
            max_error_burst: 10,
        }
    }
}
```

---

## Performance Report Template

### Standard Performance Report

```markdown
# Load Test Performance Report

**Test Name:** [Test Name]  
**Date:** YYYY-MM-DD HH:MM  
**Environment:** [Hardware/Software specs]  
**Tester:** [Name]

---

## Executive Summary

[Brief overview of results and key findings]

**Overall Status:** ✅ PASS / ⚠️ WARNING / ❌ FAIL

Key Metrics:
- Success Rate: XX.X% (Target: >XX%)
- Average Response: XXXms (Target: <XXXms)
- Throughput: XX.X ops/sec (Target: >XX)
- Peak Concurrent Users: XX

---

## Test Configuration

### Load Profile
- Concurrent Users: XX
- Duration: XX minutes
- Ramp-up Period: XX seconds
- Operations Delay: XX ms
- Target Throughput: XX ops/sec

### Environment
- **CPU:** [Model, cores]
- **Memory:** [Size, type]
- **Storage:** [Type, capacity]
- **Network:** [Bandwidth, latency]
- **Rust Version:** X.XX.X
- **ink! Version:** X.X.X

---

## Results Summary

### Overall Performance

| Metric | Result | Target | Status |
|--------|--------|--------|--------|
| Total Operations | XXX | - | - |
| Successful | XXX (XX.X%) | >XX% | ✅ |
| Failed | XXX (X.X%) | <X% | ✅ |
| Avg Response Time | XXXms | <XXXms | ✅ |
| Min Response Time | XXms | - | - |
| Max Response Time | XXXXms | <XXXXms | ✅ |
| Throughput | XX.X ops/sec | >XX | ✅ |

### Response Time Distribution

| Percentile | Time (ms) | % of Total |
|------------|-----------|------------|
| P50 (Median) | XXX | - |
| P75 | XXX | XX% |
| P90 | XXX | XX% |
| P95 | XXX | XX% |
| P99 | XXX | XX% |
| P99.9 | XXX | XX% |

### Timeline Analysis

| Time Period | Operations | Success Rate | Avg Response |
|-------------|------------|--------------|--------------|
| 00:00-00:30 | XXX | XX.X% | XXXms |
| 00:30-01:00 | XXX | XX.X% | XXXms |
| 01:00-01:30 | XXX | XX.X% | XXXms |
| ... | ... | ... | ... |

---

## Detailed Analysis

### Success Rate Trend

[Graph or description of success rate over time]

**Observations:**
- Initial success rate: XX.X%
- Final success rate: XX.X%
- Trend: Stable / Improving / Degrading
- Notable incidents: [Describe any drops or anomalies]

### Response Time Analysis

[Graph or description of response time distribution]

**Observations:**
- Fastest operation: XXms
- Slowest operation: XXXXms
- Consistency: [Stable / Variable / Erratic]
- Outliers: X operations > XXXXms

### Throughput Analysis

[Graph or description of throughput over time]

**Observations:**
- Peak throughput: XX.X ops/sec at XX:XX
- Minimum throughput: XX.X ops/sec at XX:XX
- Average throughput: XX.X ops/sec
- Stability: [Consistent / Fluctuating]

---

## Error Analysis

### Error Breakdown

| Error Type | Count | Percentage |
|------------|-------|------------|
| Contract Execution Failed | XX | XX% |
| Timeout | XX | XX% |
| Insufficient Gas | XX | XX% |
| Validation Failed | XX | XX% |
| Other | XX | XX% |
| **Total** | **XX** | **100%** |

### Error Timeline

[Description of when errors occurred]

**Root Cause Analysis:**
[Investigation of primary error causes]

---

## Resource Utilization

### CPU Usage

- Average: XX%
- Peak: XX%
- Correlation with load: [Strong / Moderate / Weak]

### Memory Usage

- Average: XXX MB
- Peak: XXX MB
- Growth trend: [Stable / Increasing / Decreasing]

### Other Resources

[Disk I/O, Network usage, etc.]

---

## Bottleneck Identification

### Observed Bottlenecks

1. **[Bottleneck Name]**
   - **Symptom:** [Description]
   - **Impact:** [Effect on performance]
   - **Evidence:** [Metrics supporting conclusion]
   - **Recommendation:** [Suggested fix]

2. **[Additional bottlenecks...]**

### Constraint Analysis

- **Primary Constraint:** [Main limiting factor]
- **Secondary Constraints:** [Other factors]
- **Headroom Remaining:** [How much capacity left]

---

## Comparison with Baseline

### vs Previous Test

| Metric | Previous | Current | Change |
|--------|----------|---------|--------|
| Success Rate | XX.X% | XX.X% | +X.X% |
| Avg Response | XXXms | XXXms | -X% |
| Throughput | XX ops/sec | XX ops/sec | +X% |

### vs Targets

| Metric | Target | Actual | Variance |
|--------|--------|--------|----------|
| Success Rate | >XX% | XX.X% | +X.X% ✅ |
| Avg Response | <XXXms | XXXms | +XXms ❌ |
| Throughput | >XX | XX | +XX ✅ |

---

## Recommendations

### Immediate Actions

1. **[Priority 1]**
   - **Issue:** [Problem description]
   - **Action:** [What to do]
   - **Expected Impact:** [Improvement estimate]

2. **[Priority 2]**
   - ...

### Long-term Improvements

1. **[Architectural change]**
   - **Benefit:** [Long-term value]
   - **Effort:** [Implementation complexity]
   - **Timeline:** [When to implement]

### Further Investigation

- [Areas needing more analysis]
- [Questions to answer]
- [Additional tests to run]

---

## Appendix

### Test Artifacts

- [Link to raw data]
- [Link to logs]
- [Link to monitoring dashboard]
- [Link to test code]

### Methodology Notes

[Any deviations from standard test procedures]

### Reviewers

- [ ] Lead Developer
- [ ] Performance Engineer
- [ ] DevOps Team

---

**Report Generated:** YYYY-MM-DD HH:MM:SS  
**Next Scheduled Test:** YYYY-MM-DD
```

---

## Trend Analysis

### Historical Performance Tracking

Create a trend database to track performance over time:

```rust
pub struct PerformanceTrend {
    pub date: String,
    pub test_name: String,
    pub success_rate: f64,
    pub avg_response_ms: f64,
    pub throughput: f64,
    pub concurrent_users: usize,
}

pub fn analyze_trend(data: Vec<PerformanceTrend>) -> TrendAnalysis {
    // Calculate trends over time
    let success_trend = calculate_slope(&data.iter().map(|d| d.success_rate).collect());
    let response_trend = calculate_slope(&data.iter().map(|d| d.avg_response_ms).collect());
    let throughput_trend = calculate_slope(&data.iter().map(|d| d.throughput).collect());
    
    TrendAnalysis {
        success_improving: success_trend > 0.0,
        response_improving: response_trend < 0.0,
        throughput_improving: throughput_trend > 0.0,
    }
}
```

### Trend Visualization

```
Performance Trends (Last 10 Tests)
==================================

Success Rate (%)
100 ┤                                    ● ●
 95 ┤              ● ● ● ● ● ● ● ●
 90 ┤    ●
 85 ┤
    └─────────────────────────────────────
      1  2  3  4  5  6  7  8  9 10  (Test #)

Avg Response Time (ms)
1000 ┤  ●
 750 ┤      ● ●
 500 ┤          ● ● ● ● ● ● ●
 250 ┤                      ●
    └─────────────────────────────────────
      1  2  3  4  5  6  7  8  9 10

Throughput (ops/sec)
100 ┤                          ● ● ●
 75 ┤              ● ● ● ● ●
 50 ┤    ● ●
 25 ┤
    └─────────────────────────────────────
      1  2  3  4  5  6  7  8  9 10
```

### Regression Detection

Automatically detect performance regressions:

```rust
pub fn detect_regression(
    current: &LoadTestMetrics,
    baseline: &LoadTestMetrics,
    threshold_pct: f64,
) -> Option<String> {
    let success_change = current.success_rate() - baseline.success_rate();
    let response_change = current.avg_response_time_ms() - baseline.avg_response_time_ms();
    let throughput_change = *current.ops_per_second.lock().unwrap() 
        - *baseline.ops_per_second.lock().unwrap();
    
    let mut regressions = Vec::new();
    
    if success_change < -threshold_pct {
        regressions.push(format!(
            "Success rate degraded by {:.1}% (threshold: {:.1}%)",
            success_change.abs(), threshold_pct
        ));
    }
    
    let response_degradation_pct = (response_change / baseline.avg_response_time_ms()) * 100.0;
    if response_degradation_pct > threshold_pct {
        regressions.push(format!(
            "Response time degraded by {:.1}% (threshold: {:.1}%)",
            response_degradation_pct, threshold_pct
        ));
    }
    
    let throughput_degradation_pct = (throughput_change.abs() / *baseline.ops_per_second.lock().unwrap()) * 100.0;
    if throughput_change < 0.0 && throughput_degradation_pct > threshold_pct {
        regressions.push(format!(
            "Throughput degraded by {:.1}% (threshold: {:.1}%)",
            throughput_degradation_pct, threshold_pct
        ));
    }
    
    if regressions.is_empty() {
        None
    } else {
        Some(regressions.join("\n"))
    }
}
```

---

## Alert Configuration

### Alert Rules

Configure automated alerts for production monitoring:

```yaml
# prometheus_alerts.yml
groups:
- name: propchain_performance
  rules:
  - alert: HighErrorRate
    expr: |
      (propchain_failed_operations / propchain_total_operations) > 0.10
    for: 2m
    labels:
      severity: critical
    annotations:
      summary: "High error rate detected"
      description: "Error rate is {{ $value | humanizePercentage }} over the last 2 minutes"
  
  - alert: HighLatency
    expr: |
      propchain_avg_response_time_ms > 1500
    for: 5m
    labels:
      severity: warning
    annotations:
      summary: "High latency detected"
      description: "Average response time is {{ $value }}ms"
  
  - alert: LowThroughput
    expr: |
      propchain_ops_per_second < 30
    for: 5m
    labels:
      severity: warning
    annotations:
      summary: "Low throughput detected"
      description: "Throughput is {{ $value }} ops/sec"
```

### Notification Channels

Configure notifications for different severity levels:

```yaml
# alertmanager.yml
route:
  receiver: 'default'
  routes:
  - match:
      severity: critical
    receiver: 'pagerduty'
  - match:
      severity: warning
    receiver: 'slack'

receivers:
- name: 'pagerduty'
  pagerduty_configs:
  - service_key: '<your-service-key>'

- name: 'slack'
  slack_configs:
  - api_url: 'https://hooks.slack.com/services/YOUR/WEBHOOK/URL'
    channel: '#alerts'
```

---

## Capacity Planning

### Load Projection

Use load test results to plan for future capacity:

```rust
pub struct CapacityPlan {
    pub current_capacity: usize,
    pub projected_growth_pct: f64,
    pub recommended_capacity: usize,
    pub timeline_months: u32,
}

impl CapacityPlan {
    pub fn calculate(
        current_metrics: &LoadTestMetrics,
        growth_rate_pct: f64,
        safety_margin_pct: f64,
    ) -> Self {
        let current_throughput = *current_metrics.ops_per_second.lock().unwrap() as usize;
        
        // Project future demand
        let projected_demand = (current_throughput as f64 * (1.0 + growth_rate_pct / 100.0)) as usize;
        
        // Add safety margin
        let recommended = (projected_demand as f64 * (1.0 + safety_margin_pct / 100.0)) as usize;
        
        Self {
            current_capacity: current_throughput,
            projected_growth_pct: growth_rate_pct,
            recommended_capacity: recommended,
            timeline_months: 12,
        }
    }
}

// Example usage
let capacity_plan = CapacityPlan::calculate(
    &metrics,
    50.0,  // Expecting 50% growth
    30.0,  // 30% safety margin
);

println!("Current Capacity: {} ops/sec", capacity_plan.current_capacity);
println!("Recommended Capacity: {} ops/sec", capacity_plan.recommended_capacity);
println!("Growth Timeline: {} months", capacity_plan.timeline_months);
```

### Scaling Recommendations

Based on load test results, provide scaling guidance:

```markdown
## Capacity Planning Recommendations

### Current State
- **Peak Load:** 50 concurrent users
- **Max Throughput:** 180 ops/sec
- **Database Size:** 2,000 properties
- **Resource Utilization:** 65% CPU, 70% Memory

### 12-Month Projections
Assuming 50% annual growth:

| Metric | Current | Month 6 | Month 12 |
|--------|---------|---------|----------|
| Users | 50 | 65 | 85 |
| Throughput Needed | 180 ops/sec | 235 ops/sec | 310 ops/sec |
| Properties | 2,000 | 3,500 | 6,000 |

### Recommended Actions

#### Immediate (0-3 months)
- [ ] Optimize database indexes
- [ ] Implement caching layer
- [ ] Set up auto-scaling triggers

#### Short-term (3-6 months)
- [ ] Upgrade to 16-core servers
- [ ] Increase memory to 32GB
- [ ] Deploy read replicas

#### Long-term (6-12 months)
- [ ] Implement sharding strategy
- [ ] Migrate to distributed architecture
- [ ] Evaluate L2 scaling solutions

### Investment Required

| Initiative | Cost | Timeline | Priority |
|------------|------|----------|----------|
| Infrastructure Upgrade | $X,XXX | Q2 | High |
| Caching Implementation | $X,XXX | Q3 | Medium |
| Architecture Redesign | $XX,XXX | Q4 | Low |

### Risk Assessment

**If no action taken:**
- Performance degradation expected at month 8
- System may fail to handle peak loads by month 10
- User experience will decline progressively

**Mitigation:**
- Implement recommendations proactively
- Monitor metrics monthly
- Review capacity plan quarterly
```

---

## Conclusion

Effective load test monitoring and reporting requires:

1. **Comprehensive Metrics**: Track success rate, response time, throughput, and resource utilization
2. **Real-time Visibility**: Implement live dashboards and alerts
3. **Historical Analysis**: Maintain trend data for regression detection
4. **Actionable Reports**: Create clear, concise performance reports with recommendations
5. **Proactive Planning**: Use data to drive capacity planning decisions

**Regular Review Cadence:**
- Daily: Automated alerts and monitoring
- Weekly: Performance report review
- Monthly: Trend analysis and capacity planning
- Quarterly: Comprehensive performance audit

For questions about monitoring setup or report templates, refer to the Load Testing Guide or contact the performance engineering team.
