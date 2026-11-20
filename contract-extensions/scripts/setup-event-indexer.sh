#!/usr/bin/env bash
# Configure and run the PropChain contract event indexer.
#
# The indexer executable must accept --config <json-file>, read the environment
# variables below, serve GET /health and GET /status, and gracefully handle
# SIGTERM. The root docker-compose configuration sets the same RPC/database/
# bind-address environment variables for its indexer service. Set INDEXER_BIN
# to an executable that implements this config and status interface.
#
# Manifest:
# {
#   "contracts": [
#     {"name":"staking","address":"5...","abi":"./staking.contract"}
#   ]
# }
# ABI must be an ink! metadata JSON file with spec.events.
#
# Usage:
#   setup-event-indexer.sh --manifest FILE [--rpc WS_URL] [--database-url URL]
#                          [--network local|production] [--start-block N]
#                          [--state-dir DIR]
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
NETWORK="local"
MANIFEST=""
RPC_URL="${SUBSTRATE_WS:-ws://127.0.0.1:9944}"
DATABASE_URL="${DATABASE_URL:-}"
START_BLOCK=0
STATE_DIR="${XDG_STATE_HOME:-$HOME/.local/state}/propchain-event-indexer"
INDEXER_BIN="${INDEXER_BIN:-propchain-indexer}"
BIND_ADDR="${BIND_ADDR:-127.0.0.1:8088}"
HEALTH_URL="${INDEXER_HEALTH_URL:-http://127.0.0.1:8088/health}"
STATUS_URL="${INDEXER_STATUS_URL:-http://127.0.0.1:8088/status}"
INDEXER_PID=""

log() { printf '%s [event-indexer] %s\n' "$(date -u +%Y-%m-%dT%H:%M:%SZ)" "$*" >&2; }
die() { log "ERROR: $*"; exit 1; }
need() { command -v "$1" >/dev/null 2>&1 || die "'$1' is required"; }
usage() {
  cat <<'USAGE'
Configure and run a PropChain contract event indexer.

Usage:
  setup-event-indexer.sh --manifest FILE [--rpc WS_URL] [--database-url URL]
                         [--network local|production] [--start-block N]
                         [--state-dir DIR]

Manifest contracts contain name, address, and an ink! metadata ABI path.
Local defaults to SQLite; production requires a PostgreSQL URL.
Set INDEXER_BIN to an executable implementing the documented config/status interface.
USAGE
  exit 0
}

while [ "$#" -gt 0 ]; do
  case "$1" in
    --manifest) [ "$#" -ge 2 ] || die "--manifest needs a value"; MANIFEST="$2"; shift 2 ;;
    --rpc) [ "$#" -ge 2 ] || die "--rpc needs a value"; RPC_URL="$2"; shift 2 ;;
    --database-url) [ "$#" -ge 2 ] || die "--database-url needs a value"; DATABASE_URL="$2"; shift 2 ;;
    --network) [ "$#" -ge 2 ] || die "--network needs a value"; NETWORK="$2"; shift 2 ;;
    --start-block) [ "$#" -ge 2 ] || die "--start-block needs a value"; START_BLOCK="$2"; shift 2 ;;
    --state-dir) [ "$#" -ge 2 ] || die "--state-dir needs a value"; STATE_DIR="$2"; shift 2 ;;
    -h|--help) usage ;;
    *) die "unknown option: $1 (try --help)" ;;
  esac
done

[ -n "$MANIFEST" ] || die "--manifest is required"
[ -f "$MANIFEST" ] || die "manifest not found: $MANIFEST"
[[ "$START_BLOCK" =~ ^[0-9]+$ ]] || die "--start-block must be a non-negative integer"
mkdir -p "$STATE_DIR"
STATE_DIR="$(realpath "$STATE_DIR")"
case "$NETWORK" in
  local)
    if [ -z "$DATABASE_URL" ]; then
      DATABASE_URL="sqlite://$STATE_DIR/events.sqlite"
    fi
    [[ "$DATABASE_URL" == sqlite://* ]] || die "local network requires a sqlite:// database URL"
    ;;
  production)
    [[ "$DATABASE_URL" == postgres://* || "$DATABASE_URL" == postgresql://* ]] \
      || die "production requires a postgres:// or postgresql:// database URL"
    ;;
  *) die "unknown network '$NETWORK' (use local or production)" ;;
esac
[[ "$RPC_URL" == ws://* || "$RPC_URL" == wss://* ]] || die "--rpc must be a ws:// or wss:// URL"

need jq
need realpath
need curl
need "$INDEXER_BIN"
jq -e '.contracts | type == "array" and length > 0' "$MANIFEST" >/dev/null \
  || die 'manifest must contain a non-empty "contracts" array'

duplicates="$(jq -r '[.contracts[].name] | group_by(.) | map(select(length > 1) | .[0]) | .[]' "$MANIFEST")"
[ -z "$duplicates" ] || die "duplicate contract names: $(printf '%s' "$duplicates" | tr '\n' ' ')"

CONFIG_FILE="$STATE_DIR/indexer-config.json"
CONTRACTS='[]'
MANIFEST_DIR="$(cd "$(dirname "$MANIFEST")" && pwd)"

while IFS= read -r contract; do
  name="$(jq -r '.name // empty' <<< "$contract")"
  address="$(jq -r '.address // empty' <<< "$contract")"
  abi_path="$(jq -r '.abi // empty' <<< "$contract")"
  [ -n "$name" ] || die "every contract needs a name"
  [ -n "$address" ] || die "contract '$name' needs an address"
  [ -n "$abi_path" ] || die "contract '$name' needs an ABI path"
  if [[ "$abi_path" != /* ]]; then
    abi_path="$MANIFEST_DIR/$abi_path"
  fi
  [ -f "$abi_path" ] || die "ABI not found for '$name': $abi_path"
  abi_path="$(realpath "$abi_path")"
  jq -e '.spec.events | type == "array" and length > 0' "$abi_path" >/dev/null \
    || die "ABI for '$name' has no spec.events array"
  events="$(jq -c '[.spec.events[].label] | unique' "$abi_path")"
  jq -e 'all(.[]; type == "string" and length > 0)' <<< "$events" >/dev/null \
    || die "ABI for '$name' contains an event without a label"
  CONTRACTS="$(jq -c --arg name "$name" --arg address "$address" --arg abi "$abi_path" \
    --argjson events "$events" '. + [{name: $name, address: $address, abi: $abi, events: $events}]' \
    <<< "$CONTRACTS")"
done < <(jq -c '.contracts[]' "$MANIFEST")

umask 077
jq -n --arg rpc "$RPC_URL" --argjson start_block "$START_BLOCK" \
  --argjson contracts "$CONTRACTS" \
  '{substrate_ws: $rpc, start_block: $start_block, contracts: $contracts}' \
  > "$CONFIG_FILE"

log "starting indexer for $(jq 'length' <<< "$CONTRACTS") contract(s); config: $CONFIG_FILE"
log "backfill starts at block $START_BLOCK; database credentials are passed via environment"
SUBSTRATE_WS="$RPC_URL" DATABASE_URL="$DATABASE_URL" BIND_ADDR="$BIND_ADDR" \
  INDEXER_START_BLOCK="$START_BLOCK" "$INDEXER_BIN" --config "$CONFIG_FILE" &
INDEXER_PID=$!

cleanup() {
  if [ -n "$INDEXER_PID" ] && kill -0 "$INDEXER_PID" 2>/dev/null; then
    log "sending SIGTERM to indexer (pid $INDEXER_PID)"
    if ! kill -TERM "$INDEXER_PID"; then
      log "ERROR: could not send SIGTERM to indexer pid $INDEXER_PID"
    fi
    if wait "$INDEXER_PID"; then
      :
    else
      status=$?
      log "indexer process exited with status $status during cleanup"
    fi
  fi
  if [ -n "$INDEXER_PID" ]; then
    log "indexer stopped"
  fi
  INDEXER_PID=""
  return 0
}
on_signal() {
  trap - INT TERM
  if cleanup; then
    exit 0
  else
    exit $?
  fi
}
trap cleanup EXIT
trap on_signal INT TERM

ready=0
for ((attempt = 0; attempt < 60; attempt++)); do
  if ! kill -0 "$INDEXER_PID" 2>/dev/null; then
    wait "$INDEXER_PID" || die "indexer exited before becoming healthy"
    die "indexer exited before becoming healthy"
  fi
  if curl --silent --show-error --fail "$HEALTH_URL" >/dev/null 2>&1; then
    ready=1
    break
  fi
  sleep 2
done
[ "$ready" -eq 1 ] || die "indexer health check timed out at $HEALTH_URL"

status_json="$(curl --silent --show-error --fail "$STATUS_URL")" \
  || die "could not read indexer status from $STATUS_URL"
jq -e '.contracts | type == "array"' <<< "$status_json" >/dev/null \
  || die "indexer status response must contain a contracts array"
printf 'Indexer healthy at %s\n' "$HEALTH_URL"
jq -r '.contracts[] | "  \(.name): \(.event_count // 0) events"' <<< "$status_json"
log "monitoring indexer process; Ctrl-C or SIGTERM requests graceful shutdown"

if wait "$INDEXER_PID"; then
  INDEXER_PID=""
  log "indexer exited normally"
else
  exit_code=$?
  INDEXER_PID=""
  die "indexer exited with status $exit_code"
fi
