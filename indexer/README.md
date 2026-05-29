### PropChain Indexer and Event API

This service ingests on-chain ink! contract events (via `Contracts::ContractEmitted`) and stores them in PostgreSQL with efficient indexes for querying. It also exposes a REST API for filtering and retrieving events, plus Prometheus metrics for performance monitoring.

Setup

- Environment:
  - `DATABASE_URL` (e.g., postgres://propchain:propchain123@localhost:5432/propchain)
  - `SUBSTRATE_WS` (e.g., ws://127.0.0.1:9944)
  - `BIND_ADDR` (default: 0.0.0.0:8088)

Run

```bash
cargo run -p propchain-indexer
```

API

- GET /health
- GET /events
  - Query params: `contract`, `event_type`, `topic`, `from_ts`, `to_ts`, `from_block`, `to_block`, `limit`, `offset`
  - `from_ts`/`to_ts` use RFC3339 timestamps
- GET /contracts
- GET /metrics (Prometheus)

API Documentation

- Swagger UI: http://localhost:8088/swagger-ui/
- OpenAPI JSON: http://localhost:8088/api-docs/openapi.json

Storage layout

- Narrow append-only `contract_events` table:
  - Core columns: `block_number`, `block_hash`, `block_timestamp`, `contract`, `payload_hex`
  - Optional columns for decoded data: `event_type`, `topics[]`
  - Composite/time-based indexes for efficient filtering

Archiving strategy

- Primary table sized for near-term queries (e.g., 90 days).
- Archive older rows to cold storage (separate `events_archive` table or object store) via `scripts/archive-events.sh`.
- Suggested enhancements:
  - Postgres monthly partitioning by `block_timestamp` with retention policy
  - Parquet export to S3 for long-term analytics

Monitoring

- Request metrics exposed at `/metrics`
- Recommended Grafana dashboard: track p95/p99 query latency, insert throughput, errors

