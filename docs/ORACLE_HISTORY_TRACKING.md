# Oracle Data History Tracking

## Overview

Oracle Data History Tracking maintains comprehensive historical records of all oracle data updates and valuations. This feature enables analysis, trend identification, anomaly detection, and complete audit trails for property valuations across the PropChain ecosystem.

## Features

### 1. Oracle Data Snapshots
Complete snapshots of oracle state at each update, including:
- Property ID and oracle source
- Valuation amount
- Timestamp of the update
- Confidence score
- Valuation method used
- Anomaly detection flag

**Storage**: `oracle_snapshots` mapping (property_id → Vec<OracleDataSnapshot>)
**Limit**: Last 1000 snapshots per property

### 2. Per-Source History Tracking
Maintains historical entries for each oracle source, allowing source-level analysis:
- Timestamp of the update
- Valuation provided
- Associated property
- Success/failure status
- Confidence score
- Update count

**Storage**: `source_history` mapping (source_id → Vec<SourceHistoryEntry>)
**Limit**: Last 5000 entries per source

### 3. Historical Data Queries
Multiple query methods for flexible data retrieval:

#### Get Oracle Snapshots
```rust
pub fn get_oracle_snapshots(
    property_id: u64,
    limit: u32,
) -> Vec<OracleDataSnapshot>
```
Returns most recent snapshots for a property (newest first).

#### Get History by Date Range
```rust
pub fn get_history_by_date_range(
    property_id: u64,
    start_timestamp: u64,
    end_timestamp: u64,
) -> Vec<OracleDataSnapshot>
```
Returns all snapshots within the specified timestamp range.

#### Get Source History
```rust
pub fn get_source_history(
    source_id: String,
    limit: u32,
) -> Vec<SourceHistoryEntry>
```
Returns recent updates from a specific oracle source.

### 4. Historical Statistics

Generate comprehensive statistics from historical data:
```rust
pub fn get_history_statistics(
    property_id: u64,
    days_lookback: u32,
) -> Result<OracleHistoryStatistics, OracleError>
```

**Returned Statistics**:
- `min_valuation`: Lowest valuation in period
- `max_valuation`: Highest valuation in period
- `average_valuation`: Mean valuation value
- `data_points`: Number of data points analyzed
- `period_start` / `period_end`: Time range of data
- `volatility_percentage`: Market volatility (0-100%)
- `trend_direction`: Trend indicator (positive=up, negative=down)

**Example Use Cases**:
- Analyze 30-day price trends: `get_history_statistics(prop_id, 30)`
- Calculate quarterly volatility: `get_history_statistics(prop_id, 90)`
- Year-over-year trend analysis: `get_history_statistics(prop_id, 365)`

## Configuration

### History Retention Period

Control how long historical data is retained:

```rust
pub fn set_history_retention_ms(&mut self, retention_ms: u64) -> Result<(), OracleError>
```

**Constraints**:
- Minimum: 7 days (604,800,000 ms)
- Maximum: 2 years (63,072,000,000 ms)
- Default: 90 days (7,776,000,000 ms)

### Enable/Disable History Tracking

Toggle history tracking on/off (default: enabled):

```rust
pub fn set_history_tracking_enabled(&mut self, enabled: bool) -> Result<(), OracleError>

pub fn is_history_tracking_enabled(&self) -> bool

pub fn get_history_retention_ms(&self) -> u64
```

**Admin-only operations**.

## Data Retention

Historical data is automatically cleaned based on:

1. **Retention Period**: Old data beyond the configured retention window is removed
2. **Size Limits**:
   - Oracle snapshots: 1000 per property
   - Source history: 5000 per source

When either limit is reached, the oldest entries are removed.

## Events

### HistorySnapshotRecorded
Emitted when an oracle data snapshot is captured:
```rust
pub struct HistorySnapshotRecorded {
    property_id: u64,          // Topic
    source_id: String,         // Topic
    valuation: u128,
    timestamp: u64,
    confidence_score: u32,
}
```

### SourceHistoryUpdated
Emitted when source history is recorded:
```rust
pub struct SourceHistoryUpdated {
    source_id: String,         // Topic
    property_id: u64,          // Topic
    success: bool,
    timestamp: u64,
}
```

## Integration

### Automatic Recording
History is automatically recorded when:
1. A property valuation is updated via `update_property_valuation()`
2. New oracle data is submitted (when integrated with oracle sources)

**Source ID** is recorded with each update to track which source provided the data.

### Manual Recording (Internal Use)
For oracle source integrations:

```rust
fn record_oracle_snapshot(
    &mut self,
    property_id: u64,
    source_id: String,
    valuation: u128,
    confidence_score: u32,
    valuation_method: ValuationMethod,
)

fn record_source_history(
    &mut self,
    source_id: String,
    property_id: u64,
    valuation: u128,
    success: bool,
    confidence_score: u32,
)
```

## Use Cases

### 1. Compliance and Auditing
- Track all valuation updates with source attribution
- Maintain complete audit trail for regulatory compliance
- Verify data quality over time

### 2. Market Analysis
- Analyze market trends by location and property type
- Identify volatility patterns
- Generate market reports and insights

### 3. Source Performance Analysis
- Track oracle source accuracy and consistency
- Identify underperforming sources
- Calculate source reputation scores based on historical accuracy

### 4. Price Discovery
- Analyze historical pricing data
- Support AI/ML model training
- Provide context for new valuations

### 5. Risk Management
- Identify anomalies and outliers
- Monitor for sudden price changes
- Support circuit breaker decisions

## Data Types

### OracleDataSnapshot
```rust
pub struct OracleDataSnapshot {
    pub property_id: u64,
    pub source_id: String,
    pub valuation: u128,
    pub timestamp: u64,
    pub confidence_score: u32,
    pub valuation_method: ValuationMethod,
    pub is_anomaly: bool,
}
```

### SourceHistoryEntry
```rust
pub struct SourceHistoryEntry {
    pub timestamp: u64,
    pub valuation: u128,
    pub property_id: u64,
    pub success: bool,
    pub confidence_score: u32,
    pub update_count: u32,
}
```

### OracleHistoryStatistics
```rust
pub struct OracleHistoryStatistics {
    pub property_id: u64,
    pub min_valuation: u128,
    pub max_valuation: u128,
    pub average_valuation: u128,
    pub data_points: u32,
    pub period_start: u64,
    pub period_end: u64,
    pub volatility_percentage: u32,
    pub trend_direction: i32,
}
```

## Performance Considerations

### Storage
- Oracle snapshots: ~200 bytes per snapshot
- Source history: ~100 bytes per entry
- With limits: ~200KB per property + ~500KB per source

### Query Performance
- Historical queries are O(n) where n = number of historical records
- Date range queries filter the in-memory vector
- Statistics calculation involves full dataset scan

### Optimization Tips
1. Use appropriate `days_lookback` values for statistics
2. Limit snapshot/history queries with `limit` parameter
3. Consider external indexing for large-scale analysis
4. Use date range queries for specific time windows

## Admin Functions

```rust
// Enable/disable history tracking
pub fn set_history_tracking_enabled(&mut self, enabled: bool) -> Result<(), OracleError>

// Configure retention period
pub fn set_history_retention_ms(&mut self, retention_ms: u64) -> Result<(), OracleError>

// Query configuration
pub fn is_history_tracking_enabled(&self) -> bool

pub fn get_history_retention_ms(&self) -> u64
```

## Future Enhancements

1. **Compression**: Implement data compression for older records
2. **Archival**: Off-chain storage for historical data
3. **Aggregation**: Pre-calculated statistics for common queries
4. **Indexing**: Time-based indexes for faster queries
5. **Pagination**: Cursor-based pagination for large datasets
6. **Filtering**: Advanced filtering by source type, confidence range, etc.

## Testing

Comprehensive test coverage includes:
- Snapshot recording and retrieval
- Source history tracking
- Date range queries
- Statistical calculations
- Data retention and cleanup
- Configuration updates
- Admin permissions

See `tests.rs` for detailed test cases.
