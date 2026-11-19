#!/usr/bin/env bash
# =============================================================================
# upgrade-migration.sh: contract upgrade with state migration support (#1477)
#
# Upgrades a proxy-fronted contract in four phases, stopping at the first failure:
#   1. pre-upgrade checks: new build exists, storage layout is compatible
#      (via contract-extensions/tools/storage-layout-diff if built), snapshot of state
#   2. upload the new code and obtain its code hash
#   3. propose the new implementation on the proxy (and confirm it, if the timelock allows)
#   4. run an optional migration message, then verify
# A rollback command is printed whenever a step fails after the proxy was changed.
#
# Usage:
#   upgrade-migration.sh --proxy ADDR --old-src FILE --new-src FILE --new-contract PATH.contract
#                        [--proxy-manifest Cargo.toml] [--migrate-message NAME [--migrate-arg V]...]
#                        [--verify-message NAME] [--network local|testnet|mainnet] [--suri S]
#                        [--url URL] [--state-message NAME]... [--skip-layout-check]
#                        [--confirm] [--dry-run]
#
#   --proxy ADDR          address of the deployed proxy
#   --old-src / --new-src ink! source files of the current and new contract (for the layout diff)
#   --new-contract PATH   built .contract bundle of the new version
#   --proxy-manifest      proxy Cargo.toml (default: contracts/proxy/Cargo.toml)
#   --migrate-message     message on the proxy to call after the switch (state migration)
#   --migrate-arg V       argument for the migration message (repeatable)
#   --verify-message      read-only message that must succeed after the upgrade
#   --state-message NAME  read-only message whose output is saved before and compared after
#                         (repeatable; e.g. a getter for balances)
#   --confirm             also call the confirm message after proposing (only works once the
#                         proxy timelock has elapsed; otherwise confirm later by hand)
#   --skip-layout-check   skip the storage layout comparison (not recommended)
#   --dry-run             show every step without sending transactions
#
# Proxy message names are not hard-coded by the scripts of other contracts, so they can be
# overridden: PROPOSE_MESSAGE (default set_implementation), CONFIRM_MESSAGE
# (default confirm_implementation), CODE_HASH_ARG is passed as the message argument.
# Check them against contracts/proxy/src/lib.rs before a real upgrade.
#
# State snapshots and a log are kept in ./upgrade-backups/<timestamp>/.
# Requires: bash, jq, cargo-contract, python3 (for the code hash); the layout check needs
# the storage-layout-diff tool built (cargo build --release in contract-extensions/tools/storage-layout-diff).
# =============================================================================
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
LAYOUT_TOOL="$REPO_ROOT/contract-extensions/tools/storage-layout-diff/target/release/storage-layout-diff"

PROPOSE_MESSAGE="${PROPOSE_MESSAGE:-set_implementation}"
CONFIRM_MESSAGE="${CONFIRM_MESSAGE:-confirm_implementation}"

PROXY=""; OLD_SRC=""; NEW_SRC=""; NEW_CONTRACT=""
PROXY_MANIFEST="$REPO_ROOT/contracts/proxy/Cargo.toml"
MIGRATE_MESSAGE=""; VERIFY_MESSAGE=""
MIGRATE_ARGS=(); STATE_MESSAGES=()
NETWORK="local"; SURI="${SURI:-}"; URL_OVERRIDE=""
SKIP_LAYOUT=0; CONFIRM=0; DRY_RUN=0

ts() { date -u +%Y-%m-%dT%H:%M:%SZ; }
LOG_FILE=""
log() { local level=$1; shift; local line; line="$(ts) [$level] $*"; echo "$line" >&2; [ -z "$LOG_FILE" ] || echo "$line" >> "$LOG_FILE"; }
die() { log ERROR "$*"; exit 1; }
need() { command -v "$1" >/dev/null 2>&1 || die "$1 is required but not installed"; }
usage() { sed -n '3,/^# ====/p' "${BASH_SOURCE[0]}" | sed '$d' | sed 's/^# \{0,1\}//'; }

while [ $# -gt 0 ]; do
  case "$1" in
    --proxy) PROXY="${2:?}"; shift 2 ;;
    --old-src) OLD_SRC="${2:?}"; shift 2 ;;
    --new-src) NEW_SRC="${2:?}"; shift 2 ;;
    --new-contract) NEW_CONTRACT="${2:?}"; shift 2 ;;
    --proxy-manifest) PROXY_MANIFEST="${2:?}"; shift 2 ;;
    --migrate-message) MIGRATE_MESSAGE="${2:?}"; shift 2 ;;
    --migrate-arg) MIGRATE_ARGS+=("${2:?}"); shift 2 ;;
    --verify-message) VERIFY_MESSAGE="${2:?}"; shift 2 ;;
    --state-message) STATE_MESSAGES+=("${2:?}"); shift 2 ;;
    --network) NETWORK="${2:?}"; shift 2 ;;
    --suri) SURI="${2:?}"; shift 2 ;;
    --url) URL_OVERRIDE="${2:?}"; shift 2 ;;
    --skip-layout-check) SKIP_LAYOUT=1; shift ;;
    --confirm) CONFIRM=1; shift ;;
    --dry-run) DRY_RUN=1; shift ;;
    -h|--help) usage; exit 0 ;;
    *) die "unknown option: $1 (try --help)" ;;
  esac
done

[ -n "$PROXY" ] || die "--proxy is required"
[ -n "$NEW_CONTRACT" ] || die "--new-contract is required"
[ -f "$NEW_CONTRACT" ] || die "new contract bundle not found: $NEW_CONTRACT"
[ -f "$PROXY_MANIFEST" ] || die "proxy manifest not found: $PROXY_MANIFEST"
if [ "$SKIP_LAYOUT" = 0 ]; then
  [ -n "$OLD_SRC" ] && [ -n "$NEW_SRC" ] || die "--old-src and --new-src are required (or pass --skip-layout-check)"
  [ -f "$OLD_SRC" ] && [ -f "$NEW_SRC" ] || die "old/new source file not found"
fi

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

BACKUP_DIR="upgrade-backups/$(date -u +%Y%m%dT%H%M%SZ)"
if [ "$DRY_RUN" = 0 ]; then
  mkdir -p "$BACKUP_DIR"
  LOG_FILE="$BACKUP_DIR/upgrade.log"
fi

run() { # print and, unless dry-run, execute a cargo-contract command
  if [ "$DRY_RUN" = 1 ]; then log DRY "$*"; return 0; fi
  "$@"
}

call_proxy() { # message [args...]; prints the JSON output
  local message=$1; shift
  local -a extra=()
  if [ $# -gt 0 ]; then extra=(--args "$@"); fi
  local -a exec_flag=()
  if [ "${EXEC:-0}" = 1 ]; then exec_flag=(--skip-confirm --execute); fi
  cargo contract call --manifest-path "$PROXY_MANIFEST" --contract "$PROXY" --message "$message" \
    ${extra[@]+"${extra[@]}"} --suri "$SURI" --url "$URL" ${exec_flag[@]+"${exec_flag[@]}"} --output-json
}

rollback_hint() {
  log WARN "the proxy may already point at the new code. To roll back, propose the previous code hash again:"
  log WARN "  cargo contract call --manifest-path $PROXY_MANIFEST --contract $PROXY --message $PROPOSE_MESSAGE --args <OLD_CODE_HASH> --suri <SURI> --url $URL --execute"
  [ -z "${OLD_CODE_HASH:-}" ] || log WARN "  previous code hash: $OLD_CODE_HASH"
  log WARN "  state snapshots: $BACKUP_DIR"
}

# --- phase 1: pre-upgrade checks ----------------------------------------------
log INFO "phase 1: pre-upgrade checks ($NETWORK, $URL)"
need jq
[ "$DRY_RUN" = 1 ] || need cargo-contract

if [ "$SKIP_LAYOUT" = 1 ]; then
  log WARN "storage layout check skipped"
elif [ -x "$LAYOUT_TOOL" ]; then
  log INFO "comparing storage layouts"
  if ! "$LAYOUT_TOOL" --old "$OLD_SRC" --new "$NEW_SRC" --format text; then
    die "storage layout is not backward compatible; fix it or migrate explicitly (see the report above)"
  fi
else
  die "storage-layout-diff is not built ($LAYOUT_TOOL). Build it with 'cargo build --release' in contract-extensions/tools/storage-layout-diff, or pass --skip-layout-check"
fi

snapshot_state() { # label
  local label=$1 m
  for m in ${STATE_MESSAGES[@]+"${STATE_MESSAGES[@]}"}; do
    if [ "$DRY_RUN" = 1 ]; then log DRY "snapshot $m ($label)"; continue; fi
    call_proxy "$m" > "$BACKUP_DIR/state-$label-$m.json" 2>>"$LOG_FILE" \
      || die "could not read state message '$m' ($label)"
    log INFO "saved $label state of $m"
  done
}
snapshot_state before

OLD_CODE_HASH=""
if [ "$DRY_RUN" = 0 ]; then
  OLD_CODE_HASH="$(cargo contract info --contract "$PROXY" --url "$URL" --output-json 2>/dev/null | jq -r '.code_hash // empty' || true)"
  [ -z "$OLD_CODE_HASH" ] || log INFO "current code hash (of the proxy contract itself): $OLD_CODE_HASH"
fi

# --- phase 2: upload new code ----------------------------------------------------
log INFO "phase 2: uploading new code from $NEW_CONTRACT"
NEW_CODE_HASH="$(python3 - "$NEW_CONTRACT" <<'PY'
import json, sys
bundle = json.load(open(sys.argv[1]))
print(bundle.get("source", {}).get("hash", ""))
PY
)"
[ -n "$NEW_CODE_HASH" ] || die "could not read source.hash from $NEW_CONTRACT"
log INFO "new code hash: $NEW_CODE_HASH"
if [ "$DRY_RUN" = 1 ]; then
  log DRY "cargo contract upload $NEW_CONTRACT --suri <SURI> --url $URL --execute"
else
  cargo contract upload "$NEW_CONTRACT" --suri "$SURI" --url "$URL" --skip-confirm --execute >/dev/null 2>>"$LOG_FILE" \
    || die "uploading the new code failed"
fi

# --- phase 3: switch implementation ---------------------------------------------
log INFO "phase 3: proposing the new implementation with $PROPOSE_MESSAGE"
if [ "$DRY_RUN" = 1 ]; then
  log DRY "call $PROPOSE_MESSAGE $NEW_CODE_HASH"
else
  EXEC=1 call_proxy "$PROPOSE_MESSAGE" "$NEW_CODE_HASH" >/dev/null 2>>"$LOG_FILE" || { rollback_hint; die "$PROPOSE_MESSAGE failed"; }
fi
if [ "$CONFIRM" = 1 ]; then
  log INFO "confirming with $CONFIRM_MESSAGE (fails while the timelock is still running)"
  if [ "$DRY_RUN" = 1 ]; then
    log DRY "call $CONFIRM_MESSAGE"
  else
    EXEC=1 call_proxy "$CONFIRM_MESSAGE" >/dev/null 2>>"$LOG_FILE" \
      || { rollback_hint; die "$CONFIRM_MESSAGE failed; if the timelock has not elapsed, re-run the confirm step later"; }
  fi
else
  log INFO "not confirming: once the proxy timelock has elapsed run: cargo contract call --manifest-path $PROXY_MANIFEST --contract $PROXY --message $CONFIRM_MESSAGE --suri <SURI> --url $URL --execute"
fi

# --- phase 4: migration and verification -----------------------------------------
if [ -n "$MIGRATE_MESSAGE" ]; then
  log INFO "phase 4: running migration message $MIGRATE_MESSAGE"
  if [ "$DRY_RUN" = 1 ]; then
    log DRY "call $MIGRATE_MESSAGE ${MIGRATE_ARGS[*]-}"
  else
    EXEC=1 call_proxy "$MIGRATE_MESSAGE" ${MIGRATE_ARGS[@]+"${MIGRATE_ARGS[@]}"} >/dev/null 2>>"$LOG_FILE" \
      || { rollback_hint; die "migration message $MIGRATE_MESSAGE failed"; }
  fi
fi

if [ "$CONFIRM" = 1 ] || [ -n "$MIGRATE_MESSAGE" ]; then
  snapshot_state after
  if [ "$DRY_RUN" = 0 ]; then
    for m in ${STATE_MESSAGES[@]+"${STATE_MESSAGES[@]}"}; do
      if cmp -s "$BACKUP_DIR/state-before-$m.json" "$BACKUP_DIR/state-after-$m.json"; then
        log INFO "state check '$m': unchanged"
      else
        log WARN "state check '$m': output differs before/after (expected for a migration; review $BACKUP_DIR)"
      fi
    done
  fi
  if [ -n "$VERIFY_MESSAGE" ]; then
    log INFO "verifying with $VERIFY_MESSAGE"
    if [ "$DRY_RUN" = 1 ]; then log DRY "call $VERIFY_MESSAGE"
    else call_proxy "$VERIFY_MESSAGE" >/dev/null 2>>"$LOG_FILE" || { rollback_hint; die "verification message $VERIFY_MESSAGE failed"; }
    fi
  fi
else
  log INFO "skipping post-upgrade verification: the new implementation is not active until it is confirmed"
fi

if [ "$DRY_RUN" = 1 ]; then log INFO "upgrade script finished (dry run)"; else log INFO "upgrade script finished"; fi
