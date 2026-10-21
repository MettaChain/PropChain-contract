#!/usr/bin/env bash
set -euo pipefail

# Simple archiving script:
# - Move rows older than N days from contract_events to events_archive
# - This is a starting point; consider partitioning for large-scale deployments
#
# Usage:
#   DATABASE_URL=postgres://user:pass@host:5432/db ./scripts/archive-events.sh 90

RETENTION_DAYS="${1:-90}"
DATABASE_URL="${DATABASE_URL:-}"
if [[ -z "${DATABASE_URL}" ]]; then
  echo "DATABASE_URL is required" >&2
  exit 1
fi

psql "${DATABASE_URL}" <<SQL
CREATE TABLE IF NOT EXISTS events_archive (
  LIKE contract_events INCLUDING ALL
);

-- The DELETE and INSERT run as ONE atomic statement: rows are deleted from
-- contract_events and inserted into events_archive together, so if the insert
-- fails, the delete is rolled back and no events are lost.
WITH moved AS (
  DELETE FROM contract_events
  WHERE block_timestamp < NOW() - INTERVAL '${RETENTION_DAYS} days'
  RETURNING *
)
INSERT INTO events_archive SELECT * FROM moved;
SQL

echo "Archived events older than ${RETENTION_DAYS} days."