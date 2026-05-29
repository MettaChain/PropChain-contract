use async_graphql::{
    Context, EmptyMutation, EmptySubscription, InputObject, Object, Result as GqlResult, Schema,
};
use axum::{extract::State, response::IntoResponse, Json};
use std::sync::Arc;

use crate::db::{Db, EventQuery, IndexedEvent};

#[derive(async_graphql::SimpleObject)]
pub struct GqlEvent {
    pub id: String,
    pub block_number: i64,
    pub block_hash: String,
    pub block_timestamp: String,
    pub contract: String,
    pub event_type: Option<String>,
    pub topics: Option<Vec<String>>,
    pub payload_hex: String,
}

impl From<IndexedEvent> for GqlEvent {
    fn from(e: IndexedEvent) -> Self {
        Self {
            id: e.id.to_string(),
            block_number: e.block_number,
            block_hash: e.block_hash,
            block_timestamp: e.block_timestamp.to_rfc3339(),
            contract: e.contract,
            event_type: e.event_type,
            topics: e.topics,
            payload_hex: e.payload_hex,
        }
    }
}

#[derive(InputObject, Default)]
pub struct EventFilterInput {
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

pub struct QueryRoot;

#[Object]
impl QueryRoot {
    async fn events(
        &self,
        ctx: &Context<'_>,
        filter: Option<EventFilterInput>,
    ) -> GqlResult<Vec<GqlEvent>> {
        let db = ctx.data::<Arc<Db>>()?;
        let f = filter.unwrap_or_default();
        let from_ts = parse_rfc3339(f.from_ts)?;
        let to_ts = parse_rfc3339(f.to_ts)?;
        let q = EventQuery {
            contract: f.contract,
            event_type: f.event_type,
            topic: f.topic,
            from_ts,
            to_ts,
            from_block: f.from_block,
            to_block: f.to_block,
            limit: f.limit,
            offset: f.offset,
        };
        let rows = db
            .query_events(&q)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;
        Ok(rows.into_iter().map(GqlEvent::from).collect())
    }

    async fn contracts(&self, ctx: &Context<'_>) -> GqlResult<Vec<String>> {
        let db = ctx.data::<Arc<Db>>()?;
        let rows = sqlx::query_scalar::<_, String>(
            "SELECT DISTINCT contract FROM contract_events ORDER BY contract",
        )
        .fetch_all(&db.pool)
        .await
        .map_err(|e| async_graphql::Error::new(e.to_string()))?;
        Ok(rows)
    }
}

fn parse_rfc3339(s: Option<String>) -> GqlResult<Option<chrono::DateTime<chrono::Utc>>> {
    match s {
        None => Ok(None),
        Some(v) => chrono::DateTime::parse_from_rfc3339(&v)
            .map(|dt| Some(dt.with_timezone(&chrono::Utc)))
            .map_err(|e| async_graphql::Error::new(format!("invalid timestamp: {e}"))),
    }
}

pub type PropChainSchema = Schema<QueryRoot, EmptyMutation, EmptySubscription>;

pub fn build_schema(db: Arc<Db>) -> PropChainSchema {
    Schema::build(QueryRoot, EmptyMutation, EmptySubscription)
        .data(db)
        .finish()
}

pub async fn graphql_handler(
    State(schema): State<PropChainSchema>,
    Json(req): Json<async_graphql::Request>,
) -> Json<async_graphql::Response> {
    Json(schema.execute(req).await)
}

pub async fn graphql_playground() -> impl IntoResponse {
    axum::response::Html(async_graphql::http::playground_source(
        async_graphql::http::GraphQLPlaygroundConfig::new("/graphql"),
    ))
}
