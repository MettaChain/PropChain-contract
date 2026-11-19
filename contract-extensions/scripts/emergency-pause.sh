#!/usr/bin/env bash
# =============================================================================
# emergency-pause.sh: pause contracts and optionally trigger emergency shutdown (#1480)
#
# Usage:
#   emergency-pause.sh --contracts "ADDR[,ADDR...]" [--manifest-path Cargo.toml] [--network local|testnet|mainnet]
#                      [--suri S] [--url URL] [--reason TEXT] [--only ADDR[,ADDR...]]
#                      [--shutdown] [--yes] [--status-only] [--dry-run]
#   emergency-pause.sh --registry FILE --network NAME ...   (addresses from a deployment registry)
#
#   --contracts      comma-separated contract addresses to pause
#   --manifest-path  Cargo.toml of the contract ABI used for calls (default: contracts/lib/Cargo.toml)
#   --only           pause just these addresses out of --contracts (selective pausing)
#   --reason TEXT    pause reason (default: "emergency pause")
#   --shutdown       also call the emergency shutdown message (see below) on each contract
#   --yes            skip the confirmation prompt (mainnet always asks unless --yes)
#   --status-only    do not pause; just print the paused/unpaused status of every contract
#   --dry-run        print the cargo-contract commands without sending anything
#
# Message names default to the PropChain contract (contracts/lib/src/lib.rs) and can be
# overridden through the environment:
#   PAUSE_MESSAGE      default pause_contract      (args: <reason> <duration: None>)
#   SHUTDOWN_MESSAGE   default emergency_pause     (args: <reason>); used with --shutdown
#   STATE_MESSAGE      default get_pause_state     (read-only; its JSON output must contain a "paused" field)
# There is no generic fund-recovery message in the contracts: in --shutdown mode the script
# calls SHUTDOWN_MESSAGE and, if RECOVERY_MESSAGE is set, that message as well (no args).
# A contract without these messages is reported as "failed" and the run exits non-zero.
#
# Every action is logged with a timestamp and the contract address to stderr and to
# emergency-pause-<timestamp>.log. Requires: bash, jq, cargo-contract.
# =============================================================================
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

CONTRACTS=""; ONLY=""; REGISTRY=""
MANIFEST_PATH="$REPO_ROOT/contracts/lib/Cargo.toml"
NETWORK="local"; SURI="${SURI:-}"; URL_OVERRIDE=""
REASON="emergency pause"; SHUTDOWN=0; YES=0; STATUS_ONLY=0; DRY_RUN=0
PAUSE_MESSAGE="${PAUSE_MESSAGE:-pause_contract}"
SHUTDOWN_MESSAGE="${SHUTDOWN_MESSAGE:-emergency_pause}"
STATE_MESSAGE="${STATE_MESSAGE:-get_pause_state}"
RECOVERY_MESSAGE="${RECOVERY_MESSAGE:-}"

LOG_FILE="emergency-pause-$(date -u +%Y%m%dT%H%M%SZ).log"
ts() { date -u +%Y-%m-%dT%H:%M:%SZ; }
log() { local level=$1; shift; local line; line="$(ts) [$level] $*"; echo "$line" >&2; if [ "$DRY_RUN" = 0 ]; then echo "$line" >> "$LOG_FILE"; fi; }
die() { log ERROR "$*"; exit 1; }
need() { command -v "$1" >/dev/null 2>&1 || die "$1 is required but not installed"; }
usage() { sed -n '3,/^# ====/p' "${BASH_SOURCE[0]}" | sed '$d' | sed 's/^# \{0,1\}//'; }

while [ $# -gt 0 ]; do
  case "$1" in
    --contracts) CONTRACTS="${2:?}"; shift 2 ;;
    --only) ONLY="${2:?}"; shift 2 ;;
    --registry) REGISTRY="${2:?}"; shift 2 ;;
    --manifest-path) MANIFEST_PATH="${2:?}"; shift 2 ;;
    --network) NETWORK="${2:?}"; shift 2 ;;
    --suri) SURI="${2:?}"; shift 2 ;;
    --url) URL_OVERRIDE="${2:?}"; shift 2 ;;
    --reason) REASON="${2:?}"; shift 2 ;;
    --shutdown) SHUTDOWN=1; shift ;;
    --yes) YES=1; shift ;;
    --status-only) STATUS_ONLY=1; shift ;;
    --dry-run) DRY_RUN=1; shift ;;
    -h|--help) usage; exit 0 ;;
    *) die "unknown option: $1 (try --help)" ;;
  esac
done

need jq
[ "$DRY_RUN" = 1 ] || need cargo-contract
[ -f "$MANIFEST_PATH" ] || die "manifest not found: $MANIFEST_PATH"

if [ -n "$REGISTRY" ]; then
  [ -f "$REGISTRY" ] || die "registry not found: $REGISTRY"
  CONTRACTS="$(jq -r --arg n "$NETWORK" '(.[$n] // {}) | [.[] | .address] | join(",")' "$REGISTRY")"
fi
[ -n "$CONTRACTS" ] || die "--contracts (or --registry) is required"

case "$NETWORK" in
  local) URL="${URL_OVERRIDE:-${NODE_URL:-ws://127.0.0.1:9944}}" ;;
  testnet) URL="${URL_OVERRIDE:-${TESTNET_URL:-wss://rococo-contracts-rpc.polkadot.io}}" ;;
  mainnet) URL="${URL_OVERRIDE:-${MAINNET_URL:-}}"; [ -n "$URL" ] || die "set MAINNET_URL or --url for mainnet" ;;
  *) die "unknown network: $NETWORK" ;;
esac
if [ -z "$SURI" ]; then
  [ "$NETWORK" = "local" ] || die "SURI (or --suri) is required on $NETWORK"
  SURI="//Alice"
fi

IFS=',' read -r -a ALL <<< "$CONTRACTS"
if [ -n "$ONLY" ]; then
  IFS=',' read -r -a SELECT <<< "$ONLY"
  TARGETS=()
  for want in "${SELECT[@]}"; do
    found=0
    for have in "${ALL[@]}"; do [ "$have" = "$want" ] && found=1; done
    [ "$found" = 1 ] || die "--only address $want is not in --contracts"
    TARGETS+=("$want")
  done
else
  TARGETS=("${ALL[@]}")
fi

if [ "$STATUS_ONLY" = 0 ] && [ "$NETWORK" = "mainnet" ] && [ "$YES" = 0 ]; then
  echo "About to PAUSE ${#TARGETS[@]} contract(s) on MAINNET ($URL)$([ "$SHUTDOWN" = 1 ] && echo ' and trigger emergency shutdown')." >&2
  printf 'Type "pause mainnet" to continue: ' >&2
  read -r answer || answer=""
  [ "$answer" = "pause mainnet" ] || die "aborted"
fi

contract_call() { # address message [args...]; EXEC=1 sends a transaction
  local addr=$1 message=$2; shift 2
  local -a args=() flags=()
  if [ $# -gt 0 ]; then args=(--args "$@"); fi
  if [ "${EXEC:-0}" = 1 ]; then flags=(--skip-confirm --execute); fi
  cargo contract call --manifest-path "$MANIFEST_PATH" --contract "$addr" --message "$message" \
    ${args[@]+"${args[@]}"} --suri "$SURI" --url "$URL" ${flags[@]+"${flags[@]}"} --output-json
}

# prints "paused", "unpaused" or "unknown"
pause_state() {
  local addr=$1 out value
  if [ "$DRY_RUN" = 1 ]; then log DRY "[$addr] $STATE_MESSAGE"; echo unknown; return; fi
  out="$(contract_call "$addr" "$STATE_MESSAGE" 2>/dev/null)" || { echo unknown; return; }
  value="$(printf '%s' "$out" | jq -r '[.. | objects | select(has("paused")) | .paused][0] // empty' 2>/dev/null || true)"
  case "$value" in true) echo paused ;; false) echo unpaused ;; *) echo unknown ;; esac
}

declare -a REPORT_ADDR=() REPORT_STATE=() REPORT_NOTE=()
FAILED=0

for addr in "${TARGETS[@]}"; do
  note=""
  if [ "$STATUS_ONLY" = 0 ]; then
    log INFO "[$addr] pausing with $PAUSE_MESSAGE"
    if [ "$DRY_RUN" = 1 ]; then
      log DRY "[$addr] $PAUSE_MESSAGE \"$REASON\" None"
    elif ! EXEC=1 contract_call "$addr" "$PAUSE_MESSAGE" "$REASON" None >/dev/null 2>>"$LOG_FILE"; then
      log ERROR "[$addr] $PAUSE_MESSAGE failed"
      note="pause call failed"
      FAILED=1
    fi
    if [ "$SHUTDOWN" = 1 ]; then
      log INFO "[$addr] emergency shutdown with $SHUTDOWN_MESSAGE"
      if [ "$DRY_RUN" = 1 ]; then
        log DRY "[$addr] $SHUTDOWN_MESSAGE \"$REASON\""
      elif ! EXEC=1 contract_call "$addr" "$SHUTDOWN_MESSAGE" "$REASON" >/dev/null 2>>"$LOG_FILE"; then
        log ERROR "[$addr] $SHUTDOWN_MESSAGE failed"
        note="${note:+$note; }shutdown call failed"
        FAILED=1
      fi
      if [ -n "$RECOVERY_MESSAGE" ]; then
        log INFO "[$addr] enabling fund recovery with $RECOVERY_MESSAGE"
        if [ "$DRY_RUN" = 1 ]; then
          log DRY "[$addr] $RECOVERY_MESSAGE"
        elif ! EXEC=1 contract_call "$addr" "$RECOVERY_MESSAGE" >/dev/null 2>>"$LOG_FILE"; then
          log ERROR "[$addr] $RECOVERY_MESSAGE failed"
          note="${note:+$note; }recovery call failed"
          FAILED=1
        fi
      else
        note="${note:+$note; }no RECOVERY_MESSAGE configured"
      fi
    fi
  fi
  state="$(pause_state "$addr")"
  log INFO "[$addr] state: $state"
  if [ "$STATUS_ONLY" = 0 ] && [ "$DRY_RUN" = 0 ] && [ "$state" != "paused" ]; then
    note="${note:+$note; }not confirmed paused"
    FAILED=1
  fi
  REPORT_ADDR+=("$addr"); REPORT_STATE+=("$state"); REPORT_NOTE+=("$note")
done

echo
echo "Pause status report ($NETWORK, $URL)"
printf '%-50s %-10s %s\n' "contract" "state" "notes"
i=0
while [ "$i" -lt "${#REPORT_ADDR[@]}" ]; do
  printf '%-50s %-10s %s\n' "${REPORT_ADDR[$i]}" "${REPORT_STATE[$i]}" "${REPORT_NOTE[$i]}"
  i=$((i + 1))
done
[ "$DRY_RUN" = 1 ] || echo "Log: $LOG_FILE"
exit "$FAILED"
