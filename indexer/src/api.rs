use crate::db::{Db, EventQuery, IndexedEvent};
use axum::{
    extract::Query,
    http::{Request, StatusCode},
    middleware::Next,
    response::Response,
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Current API version string (#174).
pub const API_VERSION: &str = "v1";

/// Response body for the `GET /api/v1/version` endpoint (#174).
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct VersionResponse {
    /// Semantic API version (e.g. "v1").
    pub version: &'static str,
    /// Service name.
    pub service: &'static str,
}

/// `GET /api/v1/version` — returns the current API version (#174).
#[utoipa::path(
    get,
    path = "/api/v1/version",
    tag = "System",
    responses(
        (status = 200, description = "Current API version", body = VersionResponse)
    )
)]
pub async fn api_version() -> Json<VersionResponse> {
    Json(VersionResponse {
        version: API_VERSION,
        service: "propchain-indexer",
    })
}

/// Axum middleware that injects `X-API-Version` into every response (#174).
pub async fn set_api_version_header<B>(req: Request<B>, next: Next<B>) -> Response {
    let mut response = next.run(req).await;
    response
        .headers_mut()
        .insert("X-API-Version", API_VERSION.parse().unwrap());
    response
}

#[derive(Clone)]
pub struct ApiState {
    pub db: Arc<Db>,
}

#[derive(Deserialize, utoipa::IntoParams, utoipa::ToSchema)]
#[into_params(parameter_in = Query)]
pub struct EventsParams {
    /// Filter by contract address
    pub contract: Option<String>,
    /// Filter by event type name
    pub event_type: Option<String>,
    /// Filter by a topic value (matches any element in the topics array)
    pub topic: Option<String>,
    /// Lower bound timestamp (RFC3339)
    pub from_ts: Option<String>,
    /// Upper bound timestamp (RFC3339)
    pub to_ts: Option<String>,
    /// Lower bound block number (inclusive)
    pub from_block: Option<i64>,
    /// Upper bound block number (inclusive)
    pub to_block: Option<i64>,
    /// Max records to return (1–1000, default 100)
    #[param(minimum = 1, maximum = 1000)]
    pub limit: Option<i64>,
    /// Number of records to skip (>= 0)
    #[param(minimum = 0)]
    pub offset: Option<i64>,
}

#[utoipa::path(
    get,
    path = "/health",
    tag = "System",
    responses(
        (status = 200, description = "Service is healthy", body = String)
    )
)]
pub async fn health() -> &'static str {
    "ok"
}

#[utoipa::path(
    get,
    path = "/events",
    tag = "Events",
    params(EventsParams),
    responses(
        (status = 200, description = "Paginated list of indexed contract events", body = Vec<IndexedEvent>),
        (status = 400, description = "Invalid query parameters"),
        (status = 500, description = "Database error")
    )
)]
pub async fn list_events(
    state: axum::extract::State<ApiState>,
    Query(params): Query<EventsParams>,
) -> Result<Json<Vec<IndexedEvent>>, (StatusCode, String)> {
    let parse_ts = |s: Option<String>| -> Result<_, String> {
        if let Some(v) = s {
            chrono::DateTime::parse_from_rfc3339(&v)
                .map_err(|e| format!("invalid timestamp: {}", e))
                .map(|dt| dt.with_timezone(&chrono::Utc))
                .map(Some)
        } else {
            Ok(None)
        }
    };

    let from_ts = parse_ts(params.from_ts).map_err(|e| (StatusCode::BAD_REQUEST, e))?;
    let to_ts = parse_ts(params.to_ts).map_err(|e| (StatusCode::BAD_REQUEST, e))?;

    if let (Some(f), Some(t)) = (from_ts, to_ts) {
        if f > t {
            return Err((
                StatusCode::BAD_REQUEST,
                "from_ts must be <= to_ts".to_string(),
            ));
        }
    }

    if let (Some(f), Some(t)) = (params.from_block, params.to_block) {
        if f > t {
            return Err((
                StatusCode::BAD_REQUEST,
                "from_block must be <= to_block".to_string(),
            ));
        }
    }

    if let Some(limit) = params.limit {
        if limit <= 0 || limit > 1000 {
            return Err((
                StatusCode::BAD_REQUEST,
                "limit must be between 1 and 1000".to_string(),
            ));
        }
    }

    if let Some(offset) = params.offset {
        if offset < 0 {
            return Err((StatusCode::BAD_REQUEST, "offset must be >= 0".to_string()));
        }
    }

    let q = EventQuery {
        contract: params.contract,
        event_type: params.event_type,
        topic: params.topic,
        from_ts,
        to_ts,
        from_block: params.from_block,
        to_block: params.to_block,
        limit: params.limit,
        offset: params.offset,
    };

    let res = state.db.query_events(&q).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("query failed: {}", e),
        )
    })?;
    Ok(Json(res))
}

#[utoipa::path(
    get,
    path = "/contracts",
    tag = "Events",
    responses(
        (status = 200, description = "Distinct list of contract addresses with indexed events", body = Vec<String>),
        (status = 500, description = "Database error")
    )
)]
pub async fn list_contracts(
    state: axum::extract::State<ApiState>,
) -> Result<Json<Vec<String>>, (StatusCode, String)> {
    let rows = sqlx::query_scalar::<_, String>(
        r#"
		SELECT DISTINCT contract
		FROM contract_events
		ORDER BY contract
		"#,
    )
    .fetch_all(&state.db.pool)
    .await
    .map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("query failed: {}", e),
        )
    })?;
    Ok(Json(rows))
}
