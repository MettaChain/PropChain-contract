#!/usr/bin/env bash
# =============================================================================
# setup-testnet.sh: testnet faucet and contract initialization (#1478)
#
# Prepares a testnet/local environment from a JSON config: checks that the signing
# accounts hold enough funds (printing faucet instructions if not), deploys the listed
# contracts, runs each contract's init calls (seed data) and writes an environment file
# with the resulting addresses.
#
# Usage:
#   setup-testnet.sh --config FILE [--suri SURI] [--url URL] [--env-out FILE] [--check-only] [--dry-run]
#
#   --config FILE   setup config; see testnet-setup.example.json
#   --suri SURI     deployer secret URI (or SURI env). Defaults to //Alice on local only.
#   --url URL       endpoint (or NODE_URL / TESTNET_URL; default per network)
#   --env-out FILE  where to write NAME_ADDRESS=... lines (default: .env.testnet)
#   --check-only    only check balances and print what would happen
#   --dry-run       print every step without sending anything
#
# Config fields: network (local|testnet), faucet.min_balance (planck), accounts[]
# (secret URIs or addresses whose balance is checked), contracts[] with name,
# manifest_path, constructor, args, init_calls[{message, args}].
# Existing contracts are not redeployed when the env file already has NAME_ADDRESS
# (delete the line to force a fresh deployment).
#
# Balance checks need a funded account; the faucet itself is external:
#   Rococo Contracts: https://use.ink/faucet  (or the network's faucet), local: //Alice is pre-funded.
#
# Requires: bash, jq, cargo-contract. Balance checks use the optional BALANCE_CMD hook (see
# check_balance); without it they are skipped with a warning.
# =============================================================================
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

CONFIG=""; SURI="${SURI:-}"; URL_OVERRIDE=""; ENV_OUT=".env.testnet"; CHECK_ONLY=0; DRY_RUN=0

ts() { date -u +%Y-%m-%dT%H:%M:%SZ; }
log() { local level=$1; shift; printf '%s [%s] %s\n' "$(ts)" "$level" "$*" >&2; }
die() { log ERROR "$*"; exit 1; }
need() { command -v "$1" >/dev/null 2>&1 || die "$1 is required but not installed"; }
usage() { sed -n '3,/^# ====/p' "${BASH_SOURCE[0]}" | sed '$d' | sed 's/^# \{0,1\}//'; }

while [ $# -gt 0 ]; do
  case "$1" in
    --config) CONFIG="${2:?}"; shift 2 ;;
    --suri) SURI="${2:?}"; shift 2 ;;
    --url) URL_OVERRIDE="${2:?}"; shift 2 ;;
    --env-out) ENV_OUT="${2:?}"; shift 2 ;;
    --check-only) CHECK_ONLY=1; shift ;;
    --dry-run) DRY_RUN=1; shift ;;
    -h|--help) usage; exit 0 ;;
    *) die "unknown option: $1 (try --help)" ;;
  esac
done

[ -n "$CONFIG" ] || die "--config is required"
[ -f "$CONFIG" ] || die "config not found: $CONFIG"
need jq
jq -e '.contracts | type == "array"' "$CONFIG" >/dev/null || die "config needs a \"contracts\" array"

NETWORK="$(jq -r '.network // "local"' "$CONFIG")"
case "$NETWORK" in
  local) URL="${URL_OVERRIDE:-${NODE_URL:-ws://127.0.0.1:9944}}" ;;
  testnet) URL="${URL_OVERRIDE:-${TESTNET_URL:-wss://rococo-contracts-rpc.polkadot.io}}" ;;
  *) die "setup-testnet only supports local and testnet (config network: $NETWORK)" ;;
esac
if [ -z "$SURI" ]; then
  [ "$NETWORK" = "local" ] || die "SURI (or --suri) is required on the testnet"
  SURI="//Alice"
fi
[ "$DRY_RUN" = 1 ] || [ "$CHECK_ONLY" = 1 ] || need cargo-contract

# --- step 1: funds -------------------------------------------------------------------
# Balance lookup is chain-specific. Set BALANCE_CMD to a command that prints the free balance
# (in planck) of the account passed as its first argument; without it the check is skipped
# with a warning, because cargo-contract has no balance query.
check_balance() { # account
  local acct=$1 min bal
  min="$(jq -r '.faucet.min_balance // "0"' "$CONFIG")"
  if [ -z "${BALANCE_CMD:-}" ]; then
    log WARN "balance of $acct not checked (set BALANCE_CMD to enable; need at least $min planck)"
    return 0
  fi
  bal="$($BALANCE_CMD "$acct" 2>/dev/null | tr -d '[:space:]')" || bal=""
  [[ "$bal" =~ ^[0-9]+$ ]] || { log WARN "could not read balance of $acct"; return 0; }
  if [ "$bal" -lt "$min" ]; then
    log WARN "$acct holds $bal planck, below the required $min"
    log WARN "request funds from the faucet (testnet: https://use.ink/faucet; local: dev accounts are pre-funded) and re-run"
    return 1
  fi
  log INFO "$acct balance ok ($bal planck)"
}

log INFO "step 1: checking funds ($NETWORK, $URL)"
funds_ok=1
while IFS= read -r acct; do
  [ -n "$acct" ] || continue
  check_balance "$acct" || funds_ok=0
done < <(jq -r '(.accounts // [])[]' "$CONFIG")
[ "$funds_ok" = 1 ] || die "insufficient funds; use the faucet and re-run"
[ "$CHECK_ONLY" = 0 ] || { log INFO "check-only: stopping here"; exit 0; }

# --- step 2: deploy and initialise ----------------------------------------------------
existing_address() { [ -f "$ENV_OUT" ] || return 0; grep "^$1=" "$ENV_OUT" | tail -n1 | cut -d= -f2- || true; }

upper_name() { echo "$1" | tr 'a-z-' 'A-Z_'; }

set_env() { # KEY VALUE
  local tmp
  tmp="$(mktemp)"
  { [ -f "$ENV_OUT" ] && grep -v "^$1=" "$ENV_OUT" || true; echo "$1=$2"; } > "$tmp"
  mv "$tmp" "$ENV_OUT"
}

count="$(jq '.contracts | length' "$CONFIG")"
i=0
while [ "$i" -lt "$count" ]; do
  name="$(jq -r ".contracts[$i].name" "$CONFIG")"
  mpath="$(jq -r ".contracts[$i].manifest_path" "$CONFIG")"
  ctor="$(jq -r ".contracts[$i].constructor // \"new\"" "$CONFIG")"
  key="$(upper_name "$name")_ADDRESS"
  [ -f "$mpath" ] || mpath="$REPO_ROOT/$mpath"
  [ -f "$mpath" ] || die "manifest_path for '$name' not found"

  addr="$(existing_address "$key")"
  fresh=1
  if [ -n "$addr" ]; then
    fresh=0
    log INFO "'$name' already deployed at $addr (from $ENV_OUT); skipping deployment"
  else
    args_text="$(jq -r ".contracts[$i].args // [] | map(tostring) | .[]" "$CONFIG")"
    args_flag=()
    if [ -n "$args_text" ]; then
      args_flag=(--args)
      while IFS= read -r a; do args_flag+=("$a"); done <<< "$args_text"
    fi
    log INFO "step 2: deploying '$name'"
    if [ "$DRY_RUN" = 1 ]; then
      log DRY "cargo contract instantiate --manifest-path $mpath --constructor $ctor ${args_flag[*]-}"
      addr="DRY_RUN_ADDRESS"
    else
      out="$(cargo contract instantiate --manifest-path "$mpath" --constructor "$ctor" \
            ${args_flag[@]+"${args_flag[@]}"} --suri "$SURI" --url "$URL" --skip-confirm --execute --output-json 2>&1)" \
        || { log ERROR "$out"; die "deploying '$name' failed"; }
      addr="$(printf '%s' "$out" | jq -r '.contract // empty' 2>/dev/null || true)"
      [ -n "$addr" ] || { log ERROR "$out"; die "no contract address in the instantiate output for '$name'"; }
      set_env "$key" "$addr"
      log INFO "'$name' deployed at $addr"
    fi
  fi

  # init calls (seed data), run only for freshly deployed contracts
  ncalls="$(jq ".contracts[$i].init_calls // [] | length" "$CONFIG")"
  if [ "$fresh" = 0 ]; then ncalls=0; fi
  c=0
  while [ "$c" -lt "$ncalls" ] && [ -n "$addr" ]; do
    msg="$(jq -r ".contracts[$i].init_calls[$c].message" "$CONFIG")"
    cargs="$(jq -r ".contracts[$i].init_calls[$c].args // [] | map(tostring) | .[]" "$CONFIG")"
    cflag=()
    if [ -n "$cargs" ]; then
      cflag=(--args)
      while IFS= read -r a; do cflag+=("$a"); done <<< "$cargs"
    fi
    log INFO "initialising '$name': $msg ${cflag[*]-}"
    if [ "$DRY_RUN" = 1 ]; then
      log DRY "cargo contract call --contract $addr --message $msg ${cflag[*]-}"
    else
      cargo contract call --manifest-path "$mpath" --contract "$addr" --message "$msg" \
        ${cflag[@]+"${cflag[@]}"} --suri "$SURI" --url "$URL" --skip-confirm --execute >/dev/null \
        || die "init call $msg on '$name' failed"
    fi
    c=$((c + 1))
  done
  i=$((i + 1))
done

if [ "$DRY_RUN" = 1 ]; then
  log INFO "dry run complete; nothing was sent"
else
  log INFO "setup complete; addresses written to $ENV_OUT"
fi
