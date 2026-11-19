#!/usr/bin/env bash
# =============================================================================
# deploy-when-gas-ok.sh: gas price monitoring and deployment optimization (#1481)
#
# Polls the current gas/fee price of the target network and deploys the contract only when
# the price is at or below a threshold, or gives up after a maximum wait. Prints the price
# history, an estimated cost before deploying and a cost summary afterwards.
#
# Usage:
#   deploy-when-gas-ok.sh --manifest-path Cargo.toml --max-price N [--constructor new] [--args "a b"]
#                         [--network local|testnet|mainnet] [--suri S] [--url URL]
#                         [--interval SECONDS] [--max-wait SECONDS] [--history FILE]
#                         [--estimate-only] [--dry-run]
#
#   --max-price N   deploy only when the observed price is <= N (same unit as the price source)
#   --interval      seconds between polls (default 30)
#   --max-wait      stop waiting after this many seconds (default 3600); exit code 2 on timeout
#   --history FILE  append "timestamp price" lines (default: gas-price-history.log)
#   --estimate-only report the estimated cost and exit without deploying
#
# Where the price comes from. Substrate chains have no single "gas price" (the Ethereum idea
# of gwei does not exist there), so the source is configurable; one of:
#   GAS_PRICE_CMD    a command that prints the current price as a number (full control; e.g. a
#                    script that queries your chain's fee multiplier)
#   GAS_RPC_URL      HTTP JSON-RPC endpoint (e.g. https://host:9933) together with
#   GAS_RPC_METHOD   JSON-RPC method returning a number or 0x-hex string (default eth_gasPrice,
#                    suitable for EVM-compatible parachains; set another method for your node)
# Without either, the script exits with an error instead of guessing a price.
#
# Cost estimate: a dry-run instantiate (no --execute) is run first; the reported gas
# (ref_time) is multiplied by the observed price. Treat the number as a rough estimate in
# the price source's unit; storage deposit is shown separately when the node reports it.
#
# Notifications when the deployment starts: WEBHOOK_URL (JSON POST {"text": ...}) and/or
# SLACK_WEBHOOK_URL (Slack incoming webhook).
# Requires: bash, jq, curl (for RPC/webhooks), cargo-contract.
# =============================================================================
set -euo pipefail

MANIFEST_PATH=""; CONSTRUCTOR="new"; ARGS_TEXT=""; MAX_PRICE=""
NETWORK="local"; SURI="${SURI:-}"; URL_OVERRIDE=""
INTERVAL=30; MAX_WAIT=3600; HISTORY="gas-price-history.log"; ESTIMATE_ONLY=0; DRY_RUN=0

ts() { date -u +%Y-%m-%dT%H:%M:%SZ; }
log() { local level=$1; shift; printf '%s [%s] %s\n' "$(ts)" "$level" "$*" >&2; }
die() { log ERROR "$*"; exit 1; }
need() { command -v "$1" >/dev/null 2>&1 || die "$1 is required but not installed"; }
usage() { sed -n '3,/^# ====/p' "${BASH_SOURCE[0]}" | sed '$d' | sed 's/^# \{0,1\}//'; }

while [ $# -gt 0 ]; do
  case "$1" in
    --manifest-path) MANIFEST_PATH="${2:?}"; shift 2 ;;
    --constructor) CONSTRUCTOR="${2:?}"; shift 2 ;;
    --args) ARGS_TEXT="${2:?}"; shift 2 ;;
    --max-price) MAX_PRICE="${2:?}"; shift 2 ;;
    --network) NETWORK="${2:?}"; shift 2 ;;
    --suri) SURI="${2:?}"; shift 2 ;;
    --url) URL_OVERRIDE="${2:?}"; shift 2 ;;
    --interval) INTERVAL="${2:?}"; shift 2 ;;
    --max-wait) MAX_WAIT="${2:?}"; shift 2 ;;
    --history) HISTORY="${2:?}"; shift 2 ;;
    --estimate-only) ESTIMATE_ONLY=1; shift ;;
    --dry-run) DRY_RUN=1; shift ;;
    -h|--help) usage; exit 0 ;;
    *) die "unknown option: $1 (try --help)" ;;
  esac
done

[ -n "$MANIFEST_PATH" ] || die "--manifest-path is required"
[ -f "$MANIFEST_PATH" ] || die "manifest not found: $MANIFEST_PATH"
[[ "$MAX_PRICE" =~ ^[0-9]+([.][0-9]+)?$ ]] || die "--max-price must be a number"
[[ "$INTERVAL" =~ ^[0-9]+$ ]] && [ "$INTERVAL" -gt 0 ] || die "--interval must be a positive integer"
[[ "$MAX_WAIT" =~ ^[0-9]+$ ]] || die "--max-wait must be a non-negative integer"
need jq
[ "$DRY_RUN" = 1 ] || need cargo-contract
[ -n "${GAS_PRICE_CMD:-}" ] || [ -n "${GAS_RPC_URL:-}" ] || die "no price source: set GAS_PRICE_CMD or GAS_RPC_URL (see --help)"
[ -z "${GAS_RPC_URL:-}" ] || need curl

case "$NETWORK" in
  local) URL="${URL_OVERRIDE:-${NODE_URL:-ws://127.0.0.1:9944}}" ;;
  testnet) URL="${URL_OVERRIDE:-${TESTNET_URL:-wss://rococo-contracts-rpc.polkadot.io}}" ;;
  mainnet) URL="${URL_OVERRIDE:-${MAINNET_URL:-}}"; [ -n "$URL" ] || die "set MAINNET_URL or --url" ;;
  *) die "unknown network: $NETWORK" ;;
esac
if [ -z "$SURI" ]; then
  [ "$NETWORK" = "local" ] || die "SURI (or --suri) is required on $NETWORK"
  SURI="//Alice"
fi

current_price() { # prints a decimal number, or fails
  local raw
  if [ -n "${GAS_PRICE_CMD:-}" ]; then
    raw="$($GAS_PRICE_CMD 2>/dev/null)" || return 1
  else
    raw="$(curl -fsS --max-time 15 -H 'Content-Type: application/json' \
      -d "{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"${GAS_RPC_METHOD:-eth_gasPrice}\",\"params\":[]}" "$GAS_RPC_URL" \
      | jq -er '.result')" || return 1
  fi
  raw="$(echo "$raw" | tr -d '[:space:]"')"
  if [[ "$raw" =~ ^0x[0-9a-fA-F]+$ ]]; then raw=$((raw)); fi
  [[ "$raw" =~ ^[0-9]+([.][0-9]+)?$ ]] || return 1
  echo "$raw"
}

le() { awk -v a="$1" -v b="$2" 'BEGIN { exit !(a <= b) }'; }

notify() { # message
  local payload
  payload="$(jq -n --arg t "$1" '{text: $t}')"
  if [ -n "${WEBHOOK_URL:-}" ]; then curl -fsS --max-time 15 -H 'Content-Type: application/json' -d "$payload" "$WEBHOOK_URL" >/dev/null || log WARN "webhook notification failed"; fi
  if [ -n "${SLACK_WEBHOOK_URL:-}" ]; then curl -fsS --max-time 15 -H 'Content-Type: application/json' -d "$payload" "$SLACK_WEBHOOK_URL" >/dev/null || log WARN "slack notification failed"; fi
}

args_flag=()
if [ -n "$ARGS_TEXT" ]; then
  read -r -a parts <<< "$ARGS_TEXT"
  args_flag=(--args "${parts[@]}")
fi
instantiate() { # extra flags...
  cargo contract instantiate --manifest-path "$MANIFEST_PATH" --constructor "$CONSTRUCTOR" \
    ${args_flag[@]+"${args_flag[@]}"} --suri "$SURI" --url "$URL" --skip-confirm --output-json "$@"
}

# --- wait for an acceptable price ---------------------------------------------------
log INFO "waiting for price <= $MAX_PRICE (poll ${INTERVAL}s, max wait ${MAX_WAIT}s, network $NETWORK)"
start="$(date +%s)"
price=""
while true; do
  if p="$(current_price)"; then
    echo "$(ts) $p" >> "$HISTORY"
    log INFO "price: $p (threshold $MAX_PRICE)"
    if le "$p" "$MAX_PRICE"; then price="$p"; break; fi
  else
    log WARN "could not read the gas price this round"
  fi
  elapsed=$(( $(date +%s) - start ))
  if [ "$elapsed" -ge "$MAX_WAIT" ]; then
    log ERROR "timed out after ${elapsed}s without an acceptable price; history in $HISTORY"
    exit 2
  fi
  sleep "$INTERVAL"
done
log INFO "price $price is acceptable"

# --- estimate cost ----------------------------------------------------------------------
ref_time=""; deposit=""
if [ "$DRY_RUN" = 1 ]; then
  log DRY "dry-run instantiate for the estimate, then: cargo contract instantiate ... --execute"
else
  if est="$(instantiate 2>/dev/null)"; then
    ref_time="$(printf '%s' "$est" | jq -r '[.. | objects | select(has("ref_time")) | .ref_time][0] // empty' 2>/dev/null || true)"
    deposit="$(printf '%s' "$est" | jq -r '[.. | objects | select(has("storage_deposit")) | .storage_deposit][0] // empty' 2>/dev/null | head -c 200 || true)"
  else
    log WARN "dry-run estimate failed; continuing without a cost estimate"
  fi
fi
est_cost="n/a"
if [[ "${ref_time:-}" =~ ^[0-9]+$ ]]; then
  est_cost="$(awk -v g="$ref_time" -v p="$price" 'BEGIN { printf "%.0f", g * p }')"
  log INFO "estimated cost: $est_cost (ref_time $ref_time x price $price)"
fi
[ "$ESTIMATE_ONLY" = 0 ] || { log INFO "estimate-only: not deploying"; exit 0; }

# --- deploy ----------------------------------------------------------------------------
notify "Deployment of $MANIFEST_PATH starting on $NETWORK at price $price (estimated cost $est_cost)"
if [ "$DRY_RUN" = 1 ]; then
  log DRY "cargo contract instantiate --manifest-path $MANIFEST_PATH --constructor $CONSTRUCTOR ... --execute"
  exit 0
fi
log INFO "deploying"
out="$(instantiate --execute 2>&1)" || { log ERROR "$out"; die "deployment failed"; }
addr="$(printf '%s' "$out" | jq -r '.contract // empty' 2>/dev/null || true)"

echo
echo "Deployment cost summary"
echo "  network:         $NETWORK ($URL)"
echo "  contract:        ${addr:-unknown}"
echo "  gas price:       $price (threshold $MAX_PRICE)"
echo "  estimated gas:   ${ref_time:-n/a} (ref_time)"
echo "  estimated cost:  $est_cost"
echo "  storage deposit: ${deposit:-n/a}"
echo "  price history:   $HISTORY"
