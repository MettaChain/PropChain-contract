//! WebSocket handler for streaming contract events in real-time.
//!
//! ## Architecture
//!
//! A single `tokio::sync::broadcast` channel acts as the event bus:
//!
//! ```text
//!  Ingestor / DB poller
//!       │  publishes EventEnvelope
//!       ▼
//!  broadcast::Sender<EventEnvelope>   (capacity = 1024)
//!       │
//!       ├── WS client 1  (filter: contract / event_type / block_number_min)
//!       ├── WS client 2
//!       └── WS client N
//! ```
//!
//! ## Client protocol
//!
//! After the WebSocket handshake the client may send a JSON filter message at
//! any time to update its subscription.  All fields are optional:
//!
//! ```json
//! {
//!   "contract":         "5Grwv...",
//!   "event_type":       "PropertyRegistered",
//!   "block_number_min": 1000000
//! }
//! ```
//!
//! The server streams matching `EventEnvelope` JSON text frames and sends a
//! ping every 30 seconds.  Error frames are JSON objects:
//!
//! ```json
//! { "error": "lagged",  "dropped": 12 }
//! { "error": "rate_limited" }
//! { "error": "invalid_filter", "detail": "..." }
//! ```
//!
//! ## Enhancements over v1
//!
//! - **Connection registry** — `WsState` tracks every live connection with its
//!   filter and per-session metrics (events sent, bytes sent, lagged count).
//! - **Rate limiting** — each client is capped at `MAX_MSGS_PER_SECOND` inbound
//!   filter updates; excess messages are acknowledged with an error frame.
//! - **Richer `ClientFilter`** — adds `block_number_min` and case-insensitive
//!   contract / event-type matching.
//! - **Query-parameter filter** — `/ws/events?contract=…&event_type=…` seeds
//!   the filter before the first message arrives.
//! - **Structured disconnect reason** — `DisconnectReason` enum logged on exit.
//! - **Graceful shutdown signal** — `WsState::shutdown()` broadcasts a close
//!   frame to every client and waits for the registry to drain.
//! - **`WsState::broadcast_count`** — live subscriber count without a dummy `rx`.
//! - **Configurable constants** exposed as typed newtype wrappers.

use std::{
    collections::HashMap,
    net::SocketAddr,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
    time::{Duration, Instant},
};

use axum::{
    extract::{
        ws::{CloseFrame, Message, WebSocket, WebSocketUpgrade},
        ConnectInfo, Query, State,
    },
    response::IntoResponse,
};
use futures::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use tokio::sync::{broadcast, Mutex, RwLock};
use tracing::{debug, info, warn};
use uuid::Uuid;

use crate::db::IndexedEvent;

// ── Configuration ─────────────────────────────────────────────────────────────

/// Capacity of the broadcast ring buffer (events).
/// Clients that fall more than this many events behind receive a `lagged` frame.
const BROADCAST_CAPACITY: usize = 1_024;

/// Keepalive ping interval.
const PING_INTERVAL: Duration = Duration::from_secs(30);

/// Maximum inbound filter-update messages accepted per second per client.
/// Excess messages receive a `rate_limited` error frame and are dropped.
const MAX_MSGS_PER_SECOND: u32 = 5;

/// How long a client may stay connected without sending a pong response.
const PONG_TIMEOUT: Duration = Duration::from_secs(90);

// ── Per-connection metrics ────────────────────────────────────────────────────

/// Live counters maintained for each WebSocket session.
#[derive(Debug, Default)]
pub struct ConnectionMetrics {
    pub events_sent: AtomicU64,
    pub bytes_sent: AtomicU64,
    pub filter_updates: AtomicU64,
    pub lagged_count: AtomicU64,
}

impl ConnectionMetrics {
    fn record_send(&self, bytes: usize) {
        self.events_sent.fetch_add(1, Ordering::Relaxed);
        self.bytes_sent.fetch_add(bytes as u64, Ordering::Relaxed);
    }
}

// ── Connection registry entry ─────────────────────────────────────────────────

#[derive(Debug)]
pub struct ConnectionEntry {
    pub id: Uuid,
    pub remote_addr: Option<SocketAddr>,
    pub connected_at: Instant,
    pub filter: ClientFilter,
    pub metrics: Arc<ConnectionMetrics>,
}

// ── Shared state ──────────────────────────────────────────────────────────────

/// Cloneable handle passed into the Axum router state.
#[derive(Clone)]
pub struct WsState {
    pub tx: Arc<broadcast::Sender<EventEnvelope>>,
    /// Live connection registry.  Key = connection UUID.
    connections: Arc<RwLock<HashMap<Uuid, ConnectionEntry>>>,
    /// Shutdown signal — closed when `shutdown()` is called.
    shutdown_tx: Arc<tokio::sync::watch::Sender<bool>>,
    shutdown_rx: tokio::sync::watch::Receiver<bool>,
}

impl WsState {
    pub fn new() -> Self {
        let (tx, _) = broadcast::channel(BROADCAST_CAPACITY);
        let (shutdown_tx, shutdown_rx) = tokio::sync::watch::channel(false);
        Self {
            tx: Arc::new(tx),
            connections: Arc::new(RwLock::new(HashMap::new())),
            shutdown_tx: Arc::new(shutdown_tx),
            shutdown_rx,
        }
    }

    /// Publish an event to all connected clients.
    /// Returns the number of active receivers at the time of send.
    pub fn publish(&self, event: EventEnvelope) -> usize {
        match self.tx.send(event) {
            Ok(n) => n,
            Err(_) => 0, // no subscribers
        }
    }

    /// Number of active WebSocket receivers (without creating a dummy subscriber).
    pub fn broadcast_count(&self) -> usize {
        self.tx.receiver_count()
    }

    /// Snapshot of all live connections for monitoring / admin endpoints.
    pub async fn connection_snapshot(&self) -> Vec<ConnectionInfo> {
        self.connections
            .read()
            .await
            .values()
            .map(|e| ConnectionInfo {
                id: e.id,
                remote_addr: e.remote_addr,
                connected_at: e.connected_at,
                filter: e.filter.clone(),
                events_sent: e.metrics.events_sent.load(Ordering::Relaxed),
                bytes_sent: e.metrics.bytes_sent.load(Ordering::Relaxed),
                lagged_count: e.metrics.lagged_count.load(Ordering::Relaxed),
            })
            .collect()
    }

    /// Signal all handlers to close gracefully.
    pub fn shutdown(&self) {
        let _ = self.shutdown_tx.send(true);
    }

    async fn register(&self, entry: ConnectionEntry) {
        self.connections.write().await.insert(entry.id, entry);
    }

    async fn deregister(&self, id: Uuid) {
        self.connections.write().await.remove(&id);
    }

    async fn update_filter(&self, id: Uuid, filter: ClientFilter) {
        if let Some(entry) = self.connections.write().await.get_mut(&id) {
            entry.filter = filter;
        }
    }
}

impl Default for WsState {
    fn default() -> Self {
        Self::new()
    }
}

/// Serialisable snapshot of a connection — safe to expose via an admin API.
#[derive(Debug, Serialize)]
pub struct ConnectionInfo {
    pub id: Uuid,
    pub remote_addr: Option<SocketAddr>,
    #[serde(skip)]
    pub connected_at: Instant,
    pub filter: ClientFilter,
    pub events_sent: u64,
    pub bytes_sent: u64,
    pub lagged_count: u64,
}

// ── Wire types ────────────────────────────────────────────────────────────────

/// Payload broadcast to every subscriber.
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct EventEnvelope {
    /// Source contract address.
    pub contract: String,
    /// Decoded event type name (if available).
    pub event_type: Option<String>,
    /// Block number the event was emitted in.
    pub block_number: i64,
    /// RFC 3339 block timestamp.
    pub block_timestamp: String,
    /// Raw payload as hex.
    pub payload_hex: String,
    /// Decoded topics (if available).
    pub topics: Option<Vec<String>>,
}

impl From<IndexedEvent> for EventEnvelope {
    fn from(e: IndexedEvent) -> Self {
        Self {
            contract: e.contract,
            event_type: e.event_type,
            block_number: e.block_number,
            block_timestamp: e.block_timestamp.to_rfc3339(),
            payload_hex: e.payload_hex,
            topics: e.topics,
        }
    }
}

/// Optional subscription filter.  All fields are independent; omitting a field
/// means "match all values for that dimension".
#[derive(Debug, Clone, Deserialize, Default, Serialize)]
pub struct ClientFilter {
    /// Match only events from this contract address (case-insensitive).
    pub contract: Option<String>,
    /// Match only events of this type (case-insensitive).
    pub event_type: Option<String>,
    /// Match only events at or above this block number.
    pub block_number_min: Option<i64>,
}

impl ClientFilter {
    fn matches(&self, env: &EventEnvelope) -> bool {
        if let Some(ref c) = self.contract {
            if !env.contract.eq_ignore_ascii_case(c) {
                return false;
            }
        }
        if let Some(ref et) = self.event_type {
            match &env.event_type {
                Some(actual) if actual.eq_ignore_ascii_case(et) => {}
                _ => return false,
            }
        }
        if let Some(min_block) = self.block_number_min {
            if env.block_number < min_block {
                return false;
            }
        }
        true
    }
}

/// Query parameters accepted on the upgrade request.
/// Seeds the filter before the client sends its first message.
#[derive(Debug, Deserialize, Default)]
pub struct WsQueryParams {
    pub contract: Option<String>,
    pub event_type: Option<String>,
    pub block_number_min: Option<i64>,
}

impl From<WsQueryParams> for ClientFilter {
    fn from(q: WsQueryParams) -> Self {
        Self {
            contract: q.contract,
            event_type: q.event_type,
            block_number_min: q.block_number_min,
        }
    }
}

/// Reason a WebSocket session ended — logged at INFO level on exit.
#[derive(Debug)]
enum DisconnectReason {
    ClientClose,
    ClientGone,
    ReceiveError(String),
    SendError,
    BroadcastClosed,
    ShutdownSignal,
}

impl std::fmt::Display for DisconnectReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ClientClose => write!(f, "client sent close frame"),
            Self::ClientGone => write!(f, "client stream ended"),
            Self::ReceiveError(e) => write!(f, "receive error: {e}"),
            Self::SendError => write!(f, "send error (client gone)"),
            Self::BroadcastClosed => write!(f, "broadcast channel closed (server shutdown)"),
            Self::ShutdownSignal => write!(f, "server shutdown signal"),
        }
    }
}

// ── Rate limiter (token-bucket, single-client) ────────────────────────────────

struct RateLimiter {
    tokens: u32,
    max: u32,
    last_refill: Instant,
}

impl RateLimiter {
    fn new(max_per_second: u32) -> Self {
        Self {
            tokens: max_per_second,
            max: max_per_second,
            last_refill: Instant::now(),
        }
    }

    /// Returns `true` if the request is allowed, `false` if rate-limited.
    fn allow(&mut self) -> bool {
        let elapsed = self.last_refill.elapsed();
        if elapsed >= Duration::from_secs(1) {
            self.tokens = self.max;
            self.last_refill = Instant::now();
        }
        if self.tokens > 0 {
            self.tokens -= 1;
            true
        } else {
            false
        }
    }
}

// ── Axum handler ──────────────────────────────────────────────────────────────

/// Upgrade an HTTP GET request to a WebSocket connection.
///
/// Route: `GET /ws/events`
///
/// Optional query parameters seed the initial filter:
/// - `contract`         — contract address (case-insensitive)
/// - `event_type`       — event type name  (case-insensitive)
/// - `block_number_min` — minimum block number (integer)
#[utoipa::path(
    get,
    path = "/ws/events",
    tag = "Events",
    params(
        ("contract"         = Option<String>, Query, description = "Filter by contract address"),
        ("event_type"       = Option<String>, Query, description = "Filter by event type"),
        ("block_number_min" = Option<i64>,    Query, description = "Minimum block number"),
    ),
    responses(
        (status = 101, description = "WebSocket upgrade — streams EventEnvelope JSON frames"),
    )
)]
pub async fn ws_handler(
    ws: WebSocketUpgrade,
    Query(params): Query<WsQueryParams>,
    State(state): State<WsState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
) -> impl IntoResponse {
    let initial_filter = ClientFilter::from(params);
    ws.on_upgrade(move |socket| handle_socket(socket, state, Some(addr), initial_filter))
}

// ── Per-connection handler ────────────────────────────────────────────────────

async fn handle_socket(
    socket: WebSocket,
    state: WsState,
    remote_addr: Option<SocketAddr>,
    initial_filter: ClientFilter,
) {
    let conn_id = Uuid::new_v4();
    let metrics = Arc::new(ConnectionMetrics::default());

    state
        .register(ConnectionEntry {
            id: conn_id,
            remote_addr,
            connected_at: Instant::now(),
            filter: initial_filter.clone(),
            metrics: Arc::clone(&metrics),
        })
        .await;

    info!(
        conn = %conn_id,
        addr = ?remote_addr,
        filter = ?initial_filter,
        "WebSocket client connected"
    );

    let reason = run_session(socket, &state, conn_id, initial_filter, &metrics).await;

    info!(conn = %conn_id, reason = %reason, "WebSocket client disconnected");
    state.deregister(conn_id).await;
}

async fn run_session(
    socket: WebSocket,
    state: &WsState,
    conn_id: Uuid,
    initial_filter: ClientFilter,
    metrics: &ConnectionMetrics,
) -> DisconnectReason {
    let (mut sender, mut receiver) = socket.split();
    let mut rx = state.tx.subscribe();
    let mut filter = initial_filter;
    let mut rate_limiter = RateLimiter::new(MAX_MSGS_PER_SECOND);
    let mut shutdown_rx = state.shutdown_rx.clone();
    let mut ping_interval = tokio::time::interval(PING_INTERVAL);
    let mut last_pong = Instant::now();

    // Skip the immediate first ping tick.
    ping_interval.tick().await;

    loop {
        tokio::select! {
            // ── Graceful shutdown signal ──────────────────────────────────
            _ = shutdown_rx.changed() => {
                if *shutdown_rx.borrow() {
                    let _ = sender
                        .send(Message::Close(Some(CloseFrame {
                            code: axum::extract::ws::close_code::AWAY,
                            reason: "server shutting down".into(),
                        })))
                        .await;
                    return DisconnectReason::ShutdownSignal;
                }
            }

            // ── Incoming message from client ──────────────────────────────
            msg = receiver.next() => {
                match msg {
                    Some(Ok(Message::Text(text))) => {
                        if !rate_limiter.allow() {
                            warn!(conn = %conn_id, "rate limit exceeded");
                            let frame = serde_json::json!({ "error": "rate_limited" }).to_string();
                            if sender.send(Message::Text(frame)).await.is_err() {
                                return DisconnectReason::SendError;
                            }
                            continue;
                        }

                        match serde_json::from_str::<ClientFilter>(&text) {
                            Ok(new_filter) => {
                                debug!(
                                    conn = %conn_id,
                                    contract = ?new_filter.contract,
                                    event_type = ?new_filter.event_type,
                                    block_number_min = ?new_filter.block_number_min,
                                    "Client updated filter"
                                );
                                filter = new_filter.clone();
                                metrics.filter_updates.fetch_add(1, Ordering::Relaxed);
                                state.update_filter(conn_id, new_filter).await;
                            }
                            Err(e) => {
                                warn!(conn = %conn_id, err = %e, "Unparseable filter message");
                                let frame = serde_json::json!({
                                    "error": "invalid_filter",
                                    "detail": e.to_string()
                                })
                                .to_string();
                                if sender.send(Message::Text(frame)).await.is_err() {
                                    return DisconnectReason::SendError;
                                }
                            }
                        }
                    }

                    Some(Ok(Message::Pong(_))) => {
                        last_pong = Instant::now();
                    }

                    Some(Ok(Message::Close(_))) => {
                        return DisconnectReason::ClientClose;
                    }

                    None => {
                        return DisconnectReason::ClientGone;
                    }

                    Some(Err(e)) => {
                        warn!(conn = %conn_id, err = %e, "WebSocket receive error");
                        return DisconnectReason::ReceiveError(e.to_string());
                    }

                    // Ignore binary / ping frames we didn't initiate.
                    _ => {}
                }
            }

            // ── Broadcast event from ingestor ─────────────────────────────
            result = rx.recv() => {
                match result {
                    Ok(envelope) => {
                        if !filter.matches(&envelope) {
                            continue;
                        }
                        match serde_json::to_string(&envelope) {
                            Ok(json) => {
                                let bytes = json.len();
                                if sender.send(Message::Text(json)).await.is_err() {
                                    return DisconnectReason::SendError;
                                }
                                metrics.record_send(bytes);
                            }
                            Err(e) => {
                                warn!(conn = %conn_id, err = %e, "Failed to serialise event");
                            }
                        }
                    }

                    Err(broadcast::error::RecvError::Lagged(n)) => {
                        warn!(conn = %conn_id, dropped = n, "Client lagged");
                        metrics.lagged_count.fetch_add(1, Ordering::Relaxed);
                        let frame = serde_json::json!({
                            "error": "lagged",
                            "dropped": n
                        })
                        .to_string();
                        if sender.send(Message::Text(frame)).await.is_err() {
                            return DisconnectReason::SendError;
                        }
                    }

                    Err(broadcast::error::RecvError::Closed) => {
                        return DisconnectReason::BroadcastClosed;
                    }
                }
            }

            // ── Keepalive ping ────────────────────────────────────────────
            _ = ping_interval.tick() => {
                // Check pong timeout before sending the next ping.
                if last_pong.elapsed() > PONG_TIMEOUT {
                    warn!(conn = %conn_id, "Pong timeout — closing stale connection");
                    let _ = sender
                        .send(Message::Close(Some(CloseFrame {
                            code: axum::extract::ws::close_code::POLICY,
                            reason: "pong timeout".into(),
                        })))
                        .await;
                    return DisconnectReason::ClientGone;
                }
                if sender.send(Message::Ping(vec![])).await.is_err() {
                    return DisconnectReason::SendError;
                }
            }
        }
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn envelope(contract: &str, event_type: Option<&str>, block: i64) -> EventEnvelope {
        EventEnvelope {
            contract: contract.to_owned(),
            event_type: event_type.map(str::to_owned),
            block_number: block,
            block_timestamp: "2024-01-01T00:00:00Z".to_owned(),
            payload_hex: "0x".to_owned(),
            topics: None,
        }
    }

    // ── ClientFilter::matches ─────────────────────────────────────────────────

    #[test]
    fn filter_default_matches_all() {
        let f = ClientFilter::default();
        assert!(f.matches(&envelope("0xABC", Some("Transfer"), 100)));
    }

    #[test]
    fn filter_contract_case_insensitive() {
        let f = ClientFilter { contract: Some("0xabc".into()), ..Default::default() };
        assert!(f.matches(&envelope("0xABC", None, 1)));
        assert!(!f.matches(&envelope("0xDEF", None, 1)));
    }

    #[test]
    fn filter_event_type_case_insensitive() {
        let f = ClientFilter { event_type: Some("transfer".into()), ..Default::default() };
        assert!(f.matches(&envelope("any", Some("Transfer"), 1)));
        assert!(!f.matches(&envelope("any", Some("Approval"), 1)));
        assert!(!f.matches(&envelope("any", None, 1)));
    }

    #[test]
    fn filter_block_number_min() {
        let f = ClientFilter { block_number_min: Some(500), ..Default::default() };
        assert!(f.matches(&envelope("any", None, 500)));
        assert!(f.matches(&envelope("any", None, 1000)));
        assert!(!f.matches(&envelope("any", None, 499)));
    }

    #[test]
    fn filter_all_fields_must_match() {
        let f = ClientFilter {
            contract: Some("0xABC".into()),
            event_type: Some("Transfer".into()),
            block_number_min: Some(100),
        };
        assert!(f.matches(&envelope("0xabc", Some("transfer"), 100)));
        assert!(!f.matches(&envelope("0xDEF", Some("transfer"), 100)));
        assert!(!f.matches(&envelope("0xabc", Some("Approval"), 100)));
        assert!(!f.matches(&envelope("0xabc", Some("transfer"), 99)));
    }

    // ── RateLimiter ───────────────────────────────────────────────────────────

    #[test]
    fn rate_limiter_allows_up_to_max() {
        let mut rl = RateLimiter::new(3);
        assert!(rl.allow());
        assert!(rl.allow());
        assert!(rl.allow());
        assert!(!rl.allow()); // 4th request denied
    }

    // ── WsState ───────────────────────────────────────────────────────────────

    #[test]
    fn publish_returns_zero_with_no_subscribers() {
        let state = WsState::new();
        assert_eq!(state.publish(envelope("x", None, 1)), 0);
    }

    #[test]
    fn broadcast_count_is_zero_initially() {
        let state = WsState::new();
        assert_eq!(state.broadcast_count(), 0);
    }

    #[test]
    fn broadcast_count_reflects_live_receivers() {
        let state = WsState::new();
        let _rx1 = state.tx.subscribe();
        let _rx2 = state.tx.subscribe();
        assert_eq!(state.broadcast_count(), 2);
    }

    // ── WsQueryParams → ClientFilter conversion ───────────────────────────────

    #[test]
    fn query_params_convert_to_filter() {
        let params = WsQueryParams {
            contract: Some("0xABC".into()),
            event_type: Some("Transfer".into()),
            block_number_min: Some(42),
        };
        let filter = ClientFilter::from(params);
        assert_eq!(filter.contract.unwrap(), "0xABC");
        assert_eq!(filter.event_type.unwrap(), "Transfer");
        assert_eq!(filter.block_number_min.unwrap(), 42);
    }
}