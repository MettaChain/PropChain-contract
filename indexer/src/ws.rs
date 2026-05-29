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
//!       ├── WS client 1  (optional filter: contract / event_type)
//!       ├── WS client 2
//!       └── WS client N
//! ```
//!
//! ## Client protocol
//!
//! After the WebSocket handshake the client may send a JSON filter message:
//!
//! ```json
//! { "contract": "5Grwv...", "event_type": "PropertyRegistered" }
//! ```
//!
//! Both fields are optional. Omitting a field means "match all".
//! The server then streams matching `EventEnvelope` objects as JSON text frames.
//! A ping/pong keepalive is sent every 30 seconds.

use crate::db::IndexedEvent;
use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    response::IntoResponse,
};
use futures::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::broadcast;
use tracing::{debug, info, warn};

/// Capacity of the broadcast channel (number of events buffered).
/// Slow clients that fall behind by more than this will receive a
/// `lagged` error and be disconnected gracefully.
const BROADCAST_CAPACITY: usize = 1024;

/// Keepalive interval in seconds.
const PING_INTERVAL_SECS: u64 = 30;

// ── Shared state ─────────────────────────────────────────────────────────────

/// Cloneable handle passed into Axum router state.
#[derive(Clone)]
pub struct WsState {
    pub tx: Arc<broadcast::Sender<EventEnvelope>>,
}

impl WsState {
    pub fn new() -> Self {
        let (tx, _) = broadcast::channel(BROADCAST_CAPACITY);
        Self { tx: Arc::new(tx) }
    }

    /// Publish an event to all connected WebSocket clients.
    /// Returns the number of active receivers.
    pub fn publish(&self, event: EventEnvelope) -> usize {
        match self.tx.send(event) {
            Ok(n) => n,
            // No subscribers — that's fine.
            Err(_) => 0,
        }
    }
}

// ── Wire types ────────────────────────────────────────────────────────────────

/// The payload broadcast to every subscriber.
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct EventEnvelope {
    /// Source contract address.
    pub contract: String,
    /// Decoded event type name (if available).
    pub event_type: Option<String>,
    /// Block number the event was emitted in.
    pub block_number: i64,
    /// RFC3339 block timestamp.
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

/// Optional filter sent by the client after connecting.
#[derive(Debug, Deserialize, Default)]
pub struct ClientFilter {
    /// Only stream events from this contract address.
    pub contract: Option<String>,
    /// Only stream events of this type.
    pub event_type: Option<String>,
}

impl ClientFilter {
    fn matches(&self, env: &EventEnvelope) -> bool {
        if let Some(ref c) = self.contract {
            if &env.contract != c {
                return false;
            }
        }
        if let Some(ref et) = self.event_type {
            match &env.event_type {
                Some(actual) if actual == et => {}
                _ => return false,
            }
        }
        true
    }
}

// ── Axum handler ─────────────────────────────────────────────────────────────

/// Upgrade an HTTP request to a WebSocket connection.
///
/// Route: `GET /ws/events`
///
/// Query params (optional, can also be sent as a JSON message after connect):
/// - `contract` — filter by contract address
/// - `event_type` — filter by event type name
#[utoipa::path(
    get,
    path = "/ws/events",
    tag = "Events",
    responses(
        (status = 101, description = "WebSocket upgrade — streams EventEnvelope JSON frames"),
    )
)]
pub async fn ws_handler(ws: WebSocketUpgrade, State(state): State<WsState>) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, state))
}

async fn handle_socket(socket: WebSocket, state: WsState) {
    let (mut sender, mut receiver) = socket.split();
    let mut rx = state.tx.subscribe();

    // Default filter — accept everything until the client sends one.
    let mut filter = ClientFilter::default();

    info!("WebSocket client connected");

    let mut ping_interval =
        tokio::time::interval(std::time::Duration::from_secs(PING_INTERVAL_SECS));
    // Skip the immediate first tick so we don't ping before the client is ready.
    ping_interval.tick().await;

    loop {
        tokio::select! {
            // ── Incoming message from client ──────────────────────────────
            msg = receiver.next() => {
                match msg {
                    Some(Ok(Message::Text(text))) => {
                        match serde_json::from_str::<ClientFilter>(&text) {
                            Ok(f) => {
                                debug!("Client updated filter: contract={:?} event_type={:?}",
                                    f.contract, f.event_type);
                                filter = f;
                            }
                            Err(e) => {
                                warn!("Ignoring unparseable filter message: {e}");
                            }
                        }
                    }
                    Some(Ok(Message::Close(_))) | None => {
                        info!("WebSocket client disconnected");
                        break;
                    }
                    Some(Ok(Message::Pong(_))) => {
                        // keepalive acknowledged — nothing to do
                    }
                    Some(Err(e)) => {
                        warn!("WebSocket receive error: {e}");
                        break;
                    }
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
                        let json = match serde_json::to_string(&envelope) {
                            Ok(j) => j,
                            Err(e) => {
                                warn!("Failed to serialize event: {e}");
                                continue;
                            }
                        };
                        if sender.send(Message::Text(json)).await.is_err() {
                            // Client disconnected mid-send.
                            break;
                        }
                    }
                    Err(broadcast::error::RecvError::Lagged(n)) => {
                        warn!("WebSocket client lagged, dropped {n} events");
                        // Notify the client and continue — don't disconnect.
                        let notice = serde_json::json!({
                            "error": "lagged",
                            "dropped": n
                        })
                        .to_string();
                        if sender.send(Message::Text(notice)).await.is_err() {
                            break;
                        }
                    }
                    Err(broadcast::error::RecvError::Closed) => {
                        // Broadcast channel shut down (server stopping).
                        break;
                    }
                }
            }

            // ── Keepalive ping ────────────────────────────────────────────
            _ = ping_interval.tick() => {
                if sender.send(Message::Ping(vec![])).await.is_err() {
                    break;
                }
            }
        }
    }

    info!("WebSocket handler exiting");
}
