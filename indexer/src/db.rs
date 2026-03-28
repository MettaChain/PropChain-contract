use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{postgres::PgPoolOptions, PgPool, Postgres, Transaction};
use uuid::Uuid;

#[derive(Clone)]
pub struct Db {
    pub pool: PgPool,
}

impl Db {
    pub async fn connect(database_url: &str, max_conns: u32) -> anyhow::Result<Self> {
        let pool = PgPoolOptions::new()
            .max_connections(max_conns)
            .acquire_timeout(std::time::Duration::from_secs(10))
            .connect(database_url)
            .await?;
        Ok(Self { pool })
    }

    pub async fn migrate(&self) -> anyhow::Result<()> {
        // Minimal schema optimized for common filters and pagination.
        // We use a narrow, append-only table with composite indexes.
        let queries = [
            r#"
			CREATE TABLE IF NOT EXISTS contract_events (
				id UUID PRIMARY KEY,
				block_number BIGINT NOT NULL,
				block_hash TEXT NOT NULL,
				block_timestamp TIMESTAMPTZ NOT NULL,
				contract TEXT NOT NULL,
				event_type TEXT,                  -- optional, filled when decoded
				topics TEXT[] DEFAULT NULL,       -- optional, filled when decoded
				payload_hex TEXT NOT NULL,        -- raw event payload (hex)
				inserted_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
			);
			"#,
            // Core filtering indexes
            r#"
			CREATE INDEX IF NOT EXISTS contract_events_block_idx
			ON contract_events (block_number DESC);
			"#,
            r#"
			CREATE INDEX IF NOT EXISTS contract_events_time_idx
			ON contract_events (block_timestamp DESC);
			"#,
            r#"
			CREATE INDEX IF NOT EXISTS contract_events_contract_time_idx
			ON contract_events (contract, block_timestamp DESC);
			"#,
            r#"
			CREATE INDEX IF NOT EXISTS contract_events_event_type_time_idx
			ON contract_events (event_type, block_timestamp DESC);
			"#,
            r#"
			CREATE INDEX IF NOT EXISTS contract_events_topics_gin_idx
			ON contract_events USING GIN (topics);
			"#,
        ];

        let mut tx: Transaction<'_, Postgres> = self.pool.begin().await?;
        for q in queries {
            sqlx::query(q).execute(&mut *tx).await?;
        }
        tx.commit().await?;
        Ok(())
    }

    #[cfg_attr(not(feature = "ingest"), allow(dead_code))]
    #[allow(clippy::too_many_arguments)]
    pub async fn insert_raw_event(
        &self,
        block_number: i64,
        block_hash: &str,
        block_timestamp: DateTime<Utc>,
        contract: &str,
        payload_hex: &str,
        event_type: Option<&str>,
        topics: Option<&[String]>,
    ) -> anyhow::Result<()> {
        let id = Uuid::new_v4();
        sqlx::query(
            r#"
			INSERT INTO contract_events
				(id, block_number, block_hash, block_timestamp, contract, payload_hex, event_type, topics)
			VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
			ON CONFLICT (id) DO NOTHING
			"#,
        )
        .bind(id)
        .bind(block_number)
        .bind(block_hash)
        .bind(block_timestamp)
        .bind(contract)
        .bind(payload_hex)
        .bind(event_type)
        .bind(topics)
        .execute(&self.pool)
        .await?;
        Ok(())
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct EventQuery {
    pub contract: Option<String>,
    pub event_type: Option<String>,
    pub topic: Option<String>,
    pub from_ts: Option<DateTime<Utc>>,
    pub to_ts: Option<DateTime<Utc>>,
    pub from_block: Option<i64>,
    pub to_block: Option<i64>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct IndexedEvent {
    pub id: Uuid,
    pub block_number: i64,
    pub block_hash: String,
    pub block_timestamp: DateTime<Utc>,
    pub contract: String,
    pub event_type: Option<String>,
    pub topics: Option<Vec<String>>,
    pub payload_hex: String,
}

impl Db {
    #[allow(unused_assignments)]
    pub async fn query_events(&self, q: &EventQuery) -> anyhow::Result<Vec<IndexedEvent>> {
        // Build dynamic filters
        let mut conditions: Vec<String> = Vec::new();
        let mut args: Vec<(usize, String)> = Vec::new();
        let mut bind_index = 1usize;

        macro_rules! push_cond {
            ($sql:expr, $val:expr) => {{
                conditions.push(format!($sql, bind_index));
                args.push((bind_index, $val.to_string()));
                bind_index += 1;
            }};
        }

        if let Some(ref c) = q.contract {
            push_cond!("contract = ${}", c);
        }
        if let Some(ref et) = q.event_type {
            push_cond!("event_type = ${}", et);
        }
        if let Some(ref t) = q.topic {
            // topics are stored as TEXT[]; we use ANY()
            conditions.push(format!("${} = ANY(topics)", bind_index));
            args.push((bind_index, t.clone()));
            bind_index += 1;
        }
        if let Some(from) = q.from_ts {
            conditions.push(format!("block_timestamp >= ${}", bind_index));
            args.push((bind_index, from.to_rfc3339()));
            bind_index += 1;
        }
        if let Some(to) = q.to_ts {
            conditions.push(format!("block_timestamp <= ${}", bind_index));
            args.push((bind_index, to.to_rfc3339()));
            bind_index += 1;
        }
        if let Some(from_b) = q.from_block {
            conditions.push(format!("block_number >= ${}", bind_index));
            args.push((bind_index, from_b.to_string()));
            bind_index += 1;
        }
        if let Some(to_b) = q.to_block {
            conditions.push(format!("block_number <= ${}", bind_index));
            args.push((bind_index, to_b.to_string()));
            bind_index += 1;
        }

        let predicate = if conditions.is_empty() {
            "".to_string()
        } else {
            format!("WHERE {}", conditions.join(" AND "))
        };

        let limit = q.limit.unwrap_or(100).min(5_000);
        let offset = q.offset.unwrap_or(0);

        let base_sql = format!(
            "
			SELECT id, block_number, block_hash, block_timestamp, contract, event_type, topics, payload_hex
			FROM contract_events
			{}
			ORDER BY block_timestamp DESC, block_number DESC
			LIMIT {} OFFSET {}
			",
            predicate, limit, offset
        );

        // Build query with dynamic binds
        let mut query = sqlx::query_as::<
            _,
            (
                Uuid,
                i64,
                String,
                DateTime<Utc>,
                String,
                Option<String>,
                Option<Vec<String>>,
                String,
            ),
        >(&base_sql);
        for (_idx, val) in args {
            // sqlx doesn't support dynamic index binding directly; use push_bind in order
            // We already baked the positions; but here order matters only.
            // We'll just push in the order constructed.
            query = query.bind(val);
        }

        let rows = query.fetch_all(&self.pool).await?;
        let events = rows
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
                )| IndexedEvent {
                    id,
                    block_number,
                    block_hash,
                    block_timestamp,
                    contract,
                    event_type,
                    topics,
                    payload_hex,
                },
            )
            .collect();
        Ok(events)
    }
}
