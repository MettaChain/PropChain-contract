//! Lightweight DB poller that publishes newly inserted events to the
//! WebSocket broadcast channel.
//!
//! This runs as a background task and polls `contract_events` every
//! `POLL_INTERVAL_MS` milliseconds for rows inserted since the last
//! seen `inserted_at` timestamp.  It is intentionally simple and does
//! not require the `subxt` / `ingest` feature to be enabled.

use crate::db::Db;
use crate::ws::{EventEnvelope, WsState};
use chrono::{DateTime, Utc};
use std::sync::Arc;
use tokio::time::{interval, Duration};
use tracing::{debug, error, info};

/// How often to poll for new events (milliseconds).
const POLL_INTERVAL_MS: u64 = 500;

/// Run the poller loop indefinitely.
///
/// Publishes every new `contract_events` row to `ws_state` so connected
/// WebSocket clients receive it in near-real-time.
pub async fn run_poller(db: Arc<Db>, ws_state: WsState) {
    info!("Event poller started (interval={}ms)", POLL_INTERVAL_MS);

    let mut ticker = interval(Duration::from_millis(POLL_INTERVAL_MS));
    // Track the high-water mark so we only fetch rows we haven't seen yet.
    let mut last_seen: DateTime<Utc> = Utc::now();

    loop {
        ticker.tick().await;

        match fetch_new_events(&db, last_seen).await {
            Ok(events) => {
                if events.is_empty() {
                    continue;
                }
                debug!("Poller fetched {} new event(s)", events.len());
                for event in events {
                    // Advance the high-water mark.
                    if event.block_timestamp > last_seen {
                        last_seen = event.block_timestamp;
                    }
                    let envelope = EventEnvelope::from(event);
                    let receivers = ws_state.publish(envelope);
                    debug!("Published event to {receivers} WebSocket subscriber(s)");
                }
            }
            Err(e) => {
                error!("Poller DB query failed: {e}");
            }
        }
    }
}

async fn fetch_new_events(
    db: &Db,
    since: DateTime<Utc>,
) -> anyhow::Result<Vec<crate::db::IndexedEvent>> {
    let rows = sqlx::query_as::<
        _,
        (
            uuid::Uuid,
            i64,
            String,
            DateTime<Utc>,
            String,
            Option<String>,
            Option<Vec<String>>,
            String,
        ),
    >(
        r#"
        SELECT id, block_number, block_hash, block_timestamp,
               contract, event_type, topics, payload_hex
        FROM   contract_events
        WHERE  inserted_at > $1
        ORDER  BY inserted_at ASC
        LIMIT  500
        "#,
    )
    .bind(since)
    .fetch_all(&db.pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(
            |(
                id,
                block_number,
                block_hash,
                block_timestamp,
                contract,
                event_type,
                topics,
                payload_hex,
            )| {
                crate::db::IndexedEvent {
                    id,
                    block_number,
                    block_hash,
                    block_timestamp,
                    contract,
                    event_type,
                    topics,
                    payload_hex,
                }
            },
        )
        .collect())
}
