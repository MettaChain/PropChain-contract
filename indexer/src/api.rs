use crate::db::{Db, EventQuery, IndexedEvent};
use axum::{extract::Query, http::StatusCode, Json};
use serde::Deserialize;
use std::sync::Arc;

#[derive(Clone)]
pub struct ApiState {
    pub db: Arc<Db>,
}

#[derive(Deserialize)]
pub struct EventsParams {
    pub contract: Option<String>,
    pub event_type: Option<String>,
    pub topic: Option<String>,
    pub from_ts: Option<String>,
    pub to_ts: Option<String>,
    pub from_block: Option<i64>,
    pub to_block: Option<i64>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

pub async fn health() -> &'static str {
    "ok"
}

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
