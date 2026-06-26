//! Lightweight DB poller that publishes newly inserted events to the
//! WebSocket broadcast channel.
//!
//! ## Architecture
//!
//! ```text
//!  PostgreSQL `contract_events`
//!       │  polled every POLL_INTERVAL (default 500 ms)
//!       ▼
//!  run_poller  ──► WsState::publish (broadcast channel)
//!                      │
//!                      ├── WS client 1
//!                      └── WS client N
//! ```
//!
//! ## Enhancements over v1
//!
//! - **Typed `PollerConfig`** — interval, batch limit, and backoff parameters
//!   are grouped and injectable rather than hard-coded constants.
//! - **Exponential back-off** — transient DB errors increase the retry delay
//!   up to `max_backoff`; a successful poll resets it to the base interval.
//! - **Shutdown signal** — accepts a `CancellationToken` so the task exits
//!   cleanly instead of being `abort()`-ed from outside.
//! - **Cursor persistence** — the high-water `inserted_at` is tracked via
//!   `inserted_at` (dedicated index column), not `block_timestamp`, which can
//!   be non-monotonic across forks/reorgs.
//! - **Batch publishing** — all events from one poll are published in one
//!   allocation pass; the high-water mark is advanced only after the full
//!   batch is processed so a panic mid-batch does not silently skip rows.
//! - **Per-poll metrics** — `PollerMetrics` exposes atomic counters for
//!   events published, poll errors, and total polls; suitable for Prometheus
//!   scraping or a `/health` endpoint.
//! - **`fetch_new_events` uses `query_as!` with a named struct** — removes
//!   the fragile positional tuple destructuring.
//! - **Configurable page size with overflow detection** — when a poll returns
//!   exactly `batch_limit` rows the poller logs a warning that it may be
//!   falling behind.
//! - **Structured tracing spans** — each poll tick runs inside an
//!   `instrument`-ed async block for clean distributed traces.

use std::{
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
    time::Duration,
};

use chrono::{DateTime, Utc};
use tokio::time::interval;
use tokio_util::sync::CancellationToken;
use tracing::{debug, error, info, instrument, warn};

use crate::{
    db::{Db, IndexedEvent},
    ws::{EventEnvelope, WsState},
};

// ── Configuration ─────────────────────────────────────────────────────────────

/// Tunable parameters for the event poller.
/// Pass a `PollerConfig::default()` for sensible out-of-the-box behaviour.
#[derive(Debug, Clone)]
pub struct PollerConfig {
    /// Base polling interval when the DB is healthy.
    pub poll_interval: Duration,
    /// Maximum rows fetched per poll tick.
    /// Values above 1 000 are clamped to 1 000 to protect the DB.
    pub batch_limit: u32,
    /// First backoff delay on a DB error.
    pub backoff_base: Duration,
    /// Upper bound for exponential backoff.
    pub backoff_max: Duration,
    /// Backoff multiplier on consecutive errors (e.g. 2.0 = double each time).
    pub backoff_factor: f64,
}

impl Default for PollerConfig {
    fn default() -> Self {
        Self {
            poll_interval: Duration::from_millis(500),
            batch_limit: 500,
            backoff_base: Duration::from_secs(1),
            backoff_max: Duration::from_secs(60),
            backoff_factor: 2.0,
        }
    }
}

impl PollerConfig {
    fn effective_batch_limit(&self) -> u32 {
        self.batch_limit.min(1_000)
    }
}

// ── Metrics ───────────────────────────────────────────────────────────────────

/// Lock-free counters exposed for monitoring.
#[derive(Debug, Default)]
pub struct PollerMetrics {
    /// Total number of poll ticks executed (successful or not).
    pub total_polls: AtomicU64,
    /// Total events published to the broadcast channel across all polls.
    pub events_published: AtomicU64,
    /// Number of poll ticks that resulted in a DB error.
    pub poll_errors: AtomicU64,
    /// Number of times a poll returned exactly `batch_limit` rows (possible lag indicator).
    pub batch_saturations: AtomicU64,
}

impl PollerMetrics {
    fn snapshot(&self) -> PollerMetricsSnapshot {
        PollerMetricsSnapshot {
            total_polls: self.total_polls.load(Ordering::Relaxed),
            events_published: self.events_published.load(Ordering::Relaxed),
            poll_errors: self.poll_errors.load(Ordering::Relaxed),
            batch_saturations: self.batch_saturations.load(Ordering::Relaxed),
        }
    }
}

/// Point-in-time copy of `PollerMetrics` — cheaply cloneable and serialisable.
#[derive(Debug, Clone, serde::Serialize)]
pub struct PollerMetricsSnapshot {
    pub total_polls: u64,
    pub events_published: u64,
    pub poll_errors: u64,
    pub batch_saturations: u64,
}

// ── Poller handle ─────────────────────────────────────────────────────────────

/// Handle returned by `spawn_poller`.  Holds the shutdown token and the shared
/// metrics so callers can observe the poller's health without blocking it.
pub struct PollerHandle {
    pub metrics: Arc<PollerMetrics>,
    pub cancel: CancellationToken,
}

impl PollerHandle {
    /// Request a graceful shutdown.  The background task will exit after the
    /// current poll tick completes.
    pub fn shutdown(&self) {
        self.cancel.cancel();
    }

    /// Snapshot the current metrics without blocking the poller.
    pub fn metrics_snapshot(&self) -> PollerMetricsSnapshot {
        self.metrics.snapshot()
    }
}

// ── Public entry points ───────────────────────────────────────────────────────

/// Spawn the poller as a detached Tokio task and return a `PollerHandle`.
///
/// ```rust,ignore
/// let handle = spawn_poller(db, ws_state, PollerConfig::default());
/// // later …
/// handle.shutdown();
/// ```
pub fn spawn_poller(db: Arc<Db>, ws_state: WsState, config: PollerConfig) -> PollerHandle {
    let metrics = Arc::new(PollerMetrics::default());
    let cancel = CancellationToken::new();

    tokio::spawn(run_poller(
        db,
        ws_state,
        config,
        Arc::clone(&metrics),
        cancel.clone(),
    ));

    PollerHandle { metrics, cancel }
}

/// Run the poller loop until the cancellation token is triggered.
///
/// Prefer `spawn_poller` for normal use; this function is exposed directly to
/// allow embedding the loop in a custom task runtime or for integration tests.
pub async fn run_poller(
    db: Arc<Db>,
    ws_state: WsState,
    config: PollerConfig,
    metrics: Arc<PollerMetrics>,
    cancel: CancellationToken,
) {
    let batch_limit = config.effective_batch_limit();

    info!(
        poll_interval_ms = config.poll_interval.as_millis(),
        batch_limit,
        "Event poller started"
    );

    // Use `inserted_at` as the cursor — it has a dedicated index and is
    // strictly monotonic (assigned by the DB on INSERT), unlike `block_timestamp`
    // which can regress on chain reorganisations.
    let mut cursor: DateTime<Utc> = Utc::now();
    let mut consecutive_errors: u32 = 0;
    let mut current_interval = config.poll_interval;
    let mut ticker = interval(current_interval);
    // MissedTickBehavior::Delay prevents a burst of back-to-back polls if the
    // DB is slow and we miss ticks while waiting.
    ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);

    loop {
        tokio::select! {
            biased;

            // Honour the cancellation token before waiting for the next tick.
            _ = cancel.cancelled() => {
                info!("Event poller received shutdown signal — exiting");
                break;
            }

            _ = ticker.tick() => {
                metrics.total_polls.fetch_add(1, Ordering::Relaxed);
                poll_once(
                    &db,
                    &ws_state,
                    &metrics,
                    &config,
                    &mut cursor,
                    &mut consecutive_errors,
                    &mut current_interval,
                    batch_limit,
                )
                .await;

                // Rebuild the ticker if the interval changed due to back-off.
                // `tokio::time::interval` doesn't support dynamic period changes
                // so we recreate it with the new value.
                ticker = interval(current_interval);
                ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
            }
        }
    }

    info!(
        metrics = ?metrics.snapshot(),
        "Event poller stopped"
    );
}

// ── Poll tick ─────────────────────────────────────────────────────────────────

#[instrument(skip_all, fields(cursor = %cursor, batch_limit))]
async fn poll_once(
    db: &Db,
    ws_state: &WsState,
    metrics: &PollerMetrics,
    config: &PollerConfig,
    cursor: &mut DateTime<Utc>,
    consecutive_errors: &mut u32,
    current_interval: &mut Duration,
    batch_limit: u32,
) {
    match fetch_new_events(db, *cursor, batch_limit).await {
        Ok(events) => {
            // Reset back-off on a successful DB round-trip, even if no rows
            // were returned — the connection itself is healthy.
            if *consecutive_errors > 0 {
                info!(
                    previous_errors = consecutive_errors,
                    "DB connection recovered — resetting poll interval"
                );
                *consecutive_errors = 0;
                *current_interval = config.poll_interval;
            }

            let count = events.len();
            if count == 0 {
                return;
            }

            debug!(count, "Fetched new event(s)");

            // Detect potential lag before publishing.
            if count as u32 == batch_limit {
                warn!(
                    batch_limit,
                    "Poll returned a full batch — poller may be falling behind"
                );
                metrics.batch_saturations.fetch_add(1, Ordering::Relaxed);
            }

            // Advance the cursor to the highest `inserted_at` in the batch.
            // We compute this before publishing so a panic in publish() doesn't
            // leave the cursor at a stale value.
            let new_cursor = events
                .iter()
                .map(|e| e.inserted_at)
                .max()
                .unwrap_or(*cursor);

            let mut published: u64 = 0;
            for event in events {
                let envelope = EventEnvelope::from(event);
                let receivers = ws_state.publish(envelope);
                debug!(receivers, "Published event to WebSocket subscriber(s)");
                published += 1;
            }

            *cursor = new_cursor;
            metrics.events_published.fetch_add(published, Ordering::Relaxed);
        }

        Err(e) => {
            *consecutive_errors += 1;
            metrics.poll_errors.fetch_add(1, Ordering::Relaxed);

            // Exponential back-off: each consecutive error multiplies the
            // current delay by `backoff_factor`, capped at `backoff_max`.
            let next = current_interval
                .as_secs_f64()
                .max(config.backoff_base.as_secs_f64())
                * config.backoff_factor;
            *current_interval = Duration::from_secs_f64(
                next.min(config.backoff_max.as_secs_f64()),
            );

            error!(
                err = %e,
                consecutive_errors,
                next_poll_ms = current_interval.as_millis(),
                "Poller DB query failed — backing off"
            );
        }
    }
}

// ── DB query ──────────────────────────────────────────────────────────────────

/// Fetch up to `limit` rows from `contract_events` whose `inserted_at`
/// is strictly greater than `since`, ordered oldest-first.
///
/// Uses `inserted_at` as the cursor column because it is:
/// - assigned by the DB (`DEFAULT now()`) — always monotonically increasing
/// - indexed independently of `block_timestamp`
/// - unaffected by chain reorganisations or clock skew in the blockchain node
async fn fetch_new_events(
    db: &Db,
    since: DateTime<Utc>,
    limit: u32,
) -> anyhow::Result<Vec<IndexedEvent>> {
    let rows = sqlx::query_as!(
        IndexedEvent,
        r#"
        SELECT
            id,
            block_number,
            block_hash,
            block_timestamp,
            inserted_at,
            contract,
            event_type,
            topics,
            payload_hex
        FROM   contract_events
        WHERE  inserted_at > $1
        ORDER  BY inserted_at ASC
        LIMIT  $2
        "#,
        since,
        limit as i64,
    )
    .fetch_all(&db.pool)
    .await?;

    Ok(rows)
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── PollerConfig ──────────────────────────────────────────────────────────

    #[test]
    fn batch_limit_is_clamped_to_1000() {
        let config = PollerConfig { batch_limit: 9_999, ..Default::default() };
        assert_eq!(config.effective_batch_limit(), 1_000);
    }

    #[test]
    fn batch_limit_below_max_is_unchanged() {
        let config = PollerConfig { batch_limit: 250, ..Default::default() };
        assert_eq!(config.effective_batch_limit(), 250);
    }

    // ── Backoff arithmetic ────────────────────────────────────────────────────

    #[test]
    fn backoff_does_not_exceed_max() {
        let config = PollerConfig {
            backoff_base: Duration::from_secs(1),
            backoff_max: Duration::from_secs(10),
            backoff_factor: 2.0,
            ..Default::default()
        };

        let mut interval = config.backoff_base;
        for _ in 0..20 {
            let next = interval.as_secs_f64() * config.backoff_factor;
            interval = Duration::from_secs_f64(next.min(config.backoff_max.as_secs_f64()));
        }
        assert!(interval <= config.backoff_max, "Back-off must not exceed backoff_max");
    }

    #[test]
    fn backoff_reaches_max_within_few_steps() {
        let config = PollerConfig {
            backoff_base: Duration::from_secs(1),
            backoff_max: Duration::from_secs(60),
            backoff_factor: 2.0,
            ..Default::default()
        };

        let mut interval = config.backoff_base;
        let mut steps = 0u32;
        while interval < config.backoff_max && steps < 100 {
            let next = interval.as_secs_f64() * config.backoff_factor;
            interval = Duration::from_secs_f64(next.min(config.backoff_max.as_secs_f64()));
            steps += 1;
        }
        // 1 → 2 → 4 → 8 → 16 → 32 → 60 = 6 steps
        assert!(steps <= 10, "Back-off should saturate within 10 steps");
        assert_eq!(interval, config.backoff_max);
    }

    // ── PollerMetrics ─────────────────────────────────────────────────────────

    #[test]
    fn metrics_snapshot_reflects_atomic_updates() {
        let m = PollerMetrics::default();
        m.total_polls.fetch_add(5, Ordering::Relaxed);
        m.events_published.fetch_add(42, Ordering::Relaxed);
        m.poll_errors.fetch_add(1, Ordering::Relaxed);

        let snap = m.snapshot();
        assert_eq!(snap.total_polls, 5);
        assert_eq!(snap.events_published, 42);
        assert_eq!(snap.poll_errors, 1);
        assert_eq!(snap.batch_saturations, 0);
    }

    // ── PollerHandle ──────────────────────────────────────────────────────────

    #[test]
    fn shutdown_cancels_token() {
        let cancel = CancellationToken::new();
        let handle = PollerHandle {
            metrics: Arc::new(PollerMetrics::default()),
            cancel: cancel.clone(),
        };
        assert!(!cancel.is_cancelled());
        handle.shutdown();
        assert!(cancel.is_cancelled());
    }
}