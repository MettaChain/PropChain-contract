# Monitoring System

The `propchain-monitoring` contract provides comprehensive on-chain observability for the PropChain ecosystem. It collects performance metrics per operation type, exposes a health-check endpoint, stores point-in-time metric snapshots, and emits alert events when configurable thresholds are breached.

## Architecture

```
contracts/monitoring/src/lib.rs   ← ink! contract (MonitoringContract)
contracts/traits/src/monitoring.rs ← shared types + MonitoringSystem trait
contracts/traits/src/constants.rs  ← MONITORING_* constants
contracts/traits/src/errors.rs     ← MonitoringError codes (9000-9999)
```

Other contracts call `record_operation` on the monitoring contract after each significant action. The monitoring contract is autonomous — it does not call back into other contracts.

## Operation types

`OperationType` covers all significant cross-contract operations:

| Variant | Description |
|---|---|
| `RegisterProperty` | Property registration |
| `TransferProperty` | Ownership transfer |
| `UpdateMetadata` | Metadata update |
| `CreateEscrow` / `ReleaseEscrow` / `RefundEscrow` | Escrow lifecycle |
| `MintToken` / `BurnToken` | Token operations |
| `BridgeTransfer` | Cross-chain bridge |
| `Stake` / `Unstake` | Staking operations |
| `GovernanceVote` | Governance vote cast |
| `OracleUpdate` | Oracle price update |
| `ComplianceCheck` | Compliance verification |
| `FeeCollection` | Fee payment |
| `Generic` | Any uncategorized operation |

## Health status

Health status is computed automatically inside `health_check()` and stored automatically when `SystemDegraded` alert fires:

| Status | Error rate |
|---|---|
| `Healthy` | < 10 % (< 1 000 bips) |
| `Degraded` | 10 % – 25 % (1 000 – 2 499 bips) |
| `Critical` | ≥ 25 % (≥ 2 500 bips) |
| `Paused` | Contract manually paused |

## Alert types

| Alert | Trigger condition |
|---|---|
| `HighErrorRate` | Overall error rate exceeds `threshold_bips` |
| `SystemDegraded` | Computed health status is `Degraded` or `Critical` |

Alerts emit an `AlertTriggered` event on-chain. Off-chain infrastructure (indexers, monitoring dashboards) subscribes to this event stream. A cooldown of 5 minutes (300 000 ms) prevents repeated emissions for the same condition.

## Snapshot buffer

`take_metrics_snapshot()` writes a `MetricsSnapshot` into a circular buffer of 100 slots (`MONITORING_MAX_SNAPSHOTS`). The newest snapshot always overwrites slot `snapshot_count % 100`. Retrieve any slot with `get_metrics_snapshot(slot)`.

## Access control

| Role | Capabilities |
|---|---|
| Admin | All messages |
| Authorized reporter | `record_operation`, `take_metrics_snapshot` |
| Anyone | All read-only messages (`health_check`, `get_performance_metrics`, etc.) |

## Building

```bash
cd contracts/monitoring
cargo contract build
```

## Testing

```bash
cd contracts/monitoring
cargo test
```

## Key messages

### Read

```rust
// Live health check
health_check() -> HealthCheckResult

// Stored admin-controlled status
get_system_status() -> HealthStatus

// Per-operation metrics
get_performance_metrics(operation: OperationType) -> PerformanceMetrics
get_all_metrics() -> Vec<PerformanceMetrics>

// Snapshot retrieval
get_metrics_snapshot(slot: u64) -> Option<MetricsSnapshot>

// Alert configuration
get_alert_config(alert_type: AlertType) -> AlertConfig
get_alert_subscribers() -> Vec<AccountId>
is_authorized_reporter(account: AccountId) -> bool
get_admin() -> AccountId
```

### Write

```rust
// Record an operation outcome (admin or authorized reporter)
record_operation(operation: OperationType, success: bool) -> Result<(), MonitoringError>

// Take a metrics snapshot (admin or authorized reporter)
take_metrics_snapshot() -> Result<(), MonitoringError>

// Admin: configure alerts
set_alert_config(alert_type: AlertType, threshold_bips: u32, active: bool) -> Result<(), MonitoringError>
subscribe_alerts(subscriber: AccountId) -> Result<(), MonitoringError>
unsubscribe_alerts(subscriber: AccountId) -> Result<(), MonitoringError>

// Admin: manage reporters
add_reporter(reporter: AccountId) -> Result<(), MonitoringError>
remove_reporter(reporter: AccountId) -> Result<(), MonitoringError>

// Admin: health status & lifecycle
set_health_status(status: HealthStatus) -> Result<(), MonitoringError>
pause() -> Result<(), MonitoringError>
resume() -> Result<(), MonitoringError>
transfer_admin(new_admin: AccountId) -> Result<(), MonitoringError>
```

## Events

| Event | When emitted |
|---|---|
| `OperationRecorded` | Every `record_operation` call |
| `AlertTriggered` | When an active alert threshold is breached (respects cooldown) |
| `HealthStatusChanged` | When stored health status changes |
| `SnapshotTaken` | Every `take_metrics_snapshot` call |
| `ReporterAdded` / `ReporterRemoved` | Reporter management |

## Error codes

All monitoring errors are in the `9000–9999` range and implement `ContractError`.

| Code | Variant | Meaning |
|---|---|---|
| 9001 | `Unauthorized` | Caller is not admin or authorized reporter |
| 9002 | `ContractPaused` | Contract is paused; operation blocked |
| 9003 | `InvalidThreshold` | `threshold_bips > 10 000` |
| 9004 | `SubscriberLimitReached` | Subscriber list is full (max 50) |
| 9005 | `SubscriberNotFound` | Unsubscribe target not in list |
