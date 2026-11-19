#!/usr/bin/env bash
# =============================================================================
# deployment-tracker.sh: multi-network contract deployment registry (#1482)
#
# Maintains deployment-registry.json (next to this script, or REGISTRY_FILE) mapping
#   network -> contract -> { address, version, deployed_at, code_hash? }
#
# Usage:
#   deployment-tracker.sh add NETWORK CONTRACT ADDRESS [--version V] [--code-hash H] [--validate]
#   deployment-tracker.sh remove NETWORK CONTRACT
#   deployment-tracker.sh list NETWORK                  contracts deployed on a network
#   deployment-tracker.sh where CONTRACT                networks that have a contract (with version)
#   deployment-tracker.sh diff NETWORK_A NETWORK_B      deployed on A but not B, plus version differences
#   deployment-tracker.sh report                        status across all networks
#   deployment-tracker.sh validate [NETWORK]            check every address is well formed and reachable
#
# Validation: the address must look like an SS58 or 0x-hex address. With --validate (add) or the
# validate command the script also asks the node whether code exists at the address using
# 'cargo contract info --contract ADDR --url URL'; set SKIP_REACHABLE=1 to check the format only.
# Node URLs: local = NODE_URL (ws://127.0.0.1:9944), testnet = TESTNET_URL
# (wss://rococo-contracts-rpc.polkadot.io), mainnet = MAINNET_URL (no default).
# Requires: bash, jq (cargo-contract for reachability checks).
# =============================================================================
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REGISTRY_FILE="${REGISTRY_FILE:-$SCRIPT_DIR/deployment-registry.json}"

ts() { date -u +%Y-%m-%dT%H:%M:%SZ; }
log() { local level=$1; shift; printf '%s [%s] %s\n' "$(ts)" "$level" "$*" >&2; }
die() { log ERROR "$*"; exit 1; }
usage() { sed -n '3,/^# ====/p' "${BASH_SOURCE[0]}" | sed '$d' | sed 's/^# \{0,1\}//'; }
command -v jq >/dev/null 2>&1 || die "jq is required but not installed"

[ -f "$REGISTRY_FILE" ] || echo '{}' > "$REGISTRY_FILE"
jq -e 'type == "object"' "$REGISTRY_FILE" >/dev/null 2>&1 || die "registry is not a JSON object: $REGISTRY_FILE"

save() { # reads new JSON from stdin, replaces the registry atomically
  local tmp
  tmp="$(mktemp)"
  cat > "$tmp"
  jq -e . "$tmp" >/dev/null || { rm -f "$tmp"; die "refusing to write invalid JSON"; }
  mv "$tmp" "$REGISTRY_FILE"
}

valid_network() { case "$1" in local|testnet|mainnet) return 0 ;; *) return 1 ;; esac; }
need_network() { valid_network "$1" || die "unknown network '$1' (use local, testnet or mainnet)"; }

valid_address_format() {
  [[ "$1" =~ ^0x[0-9a-fA-F]{64}$ ]] || [[ "$1" =~ ^[1-9A-HJ-NP-Za-km-z]{46,48}$ ]]
}

node_url() {
  case "$1" in
    local) echo "${NODE_URL:-ws://127.0.0.1:9944}" ;;
    testnet) echo "${TESTNET_URL:-wss://rococo-contracts-rpc.polkadot.io}" ;;
    mainnet) echo "${MAINNET_URL:-}" ;;
  esac
}

# 0 = reachable, 1 = bad format, 2 = not found on chain, 3 = could not check
check_address() { # network address
  local network=$1 addr=$2 url
  valid_address_format "$addr" || return 1
  [ "${SKIP_REACHABLE:-0}" = 1 ] && return 0
  command -v cargo-contract >/dev/null 2>&1 || return 3
  url="$(node_url "$network")"
  [ -n "$url" ] || return 3
  if cargo contract info --contract "$addr" --url "$url" >/dev/null 2>&1; then return 0; fi
  return 2
}

report_check() { # network address
  local rc=0
  check_address "$1" "$2" || rc=$?
  case "$rc" in
    0) echo "ok" ;; 1) echo "invalid address format" ;; 2) echo "not found on chain" ;; *) echo "not checked" ;;
  esac
  return "$rc"
}

cmd_add() {
  [ $# -ge 3 ] || die "usage: add NETWORK CONTRACT ADDRESS [--version V] [--code-hash H] [--validate]"
  local network=$1 contract=$2 address=$3 version="unknown" code_hash="" validate=0
  shift 3
  need_network "$network"
  while [ $# -gt 0 ]; do
    case "$1" in
      --version) version="${2:?}"; shift 2 ;;
      --code-hash) code_hash="${2:?}"; shift 2 ;;
      --validate) validate=1; shift ;;
      *) die "unknown option: $1" ;;
    esac
  done
  valid_address_format "$address" || die "invalid address format: $address"
  if [ "$validate" = 1 ]; then
    local rc=0
    check_address "$network" "$address" || rc=$?
    [ "$rc" = 0 ] || [ "$rc" = 3 ] || die "address check failed ($(report_check "$network" "$address" || true))"
    [ "$rc" = 0 ] || log WARN "address reachability could not be checked"
  fi
  jq --arg n "$network" --arg c "$contract" --arg a "$address" --arg v "$version" --arg h "$code_hash" --arg t "$(ts)" '
      .[$n] = ((.[$n] // {}) + {($c): ({address: $a, version: $v, deployed_at: $t} + (if $h == "" then {} else {code_hash: $h} end))})' \
    "$REGISTRY_FILE" | save
  log INFO "recorded $contract on $network: $address (version $version)"
}

cmd_remove() {
  [ $# -eq 2 ] || die "usage: remove NETWORK CONTRACT"
  need_network "$1"
  jq -e --arg n "$1" --arg c "$2" '.[$n][$c]' "$REGISTRY_FILE" >/dev/null 2>&1 || die "$2 is not recorded on $1"
  jq --arg n "$1" --arg c "$2" 'del(.[$n][$c])' "$REGISTRY_FILE" | save
  log INFO "removed $2 from $1"
}

cmd_list() {
  [ $# -eq 1 ] || die "usage: list NETWORK"
  need_network "$1"
  printf '%-28s %-12s %-50s %s\n' "contract" "version" "address" "deployed_at"
  jq -r --arg n "$1" '(.[$n] // {}) | to_entries | sort_by(.key)[] | [.key, .value.version, .value.address, .value.deployed_at] | @tsv' "$REGISTRY_FILE" \
    | while IFS=$'\t' read -r c v a d; do printf '%-28s %-12s %-50s %s\n' "$c" "$v" "$a" "$d"; done
}

cmd_where() {
  [ $# -eq 1 ] || die "usage: where CONTRACT"
  local found
  found="$(jq -r --arg c "$1" 'to_entries[] | select(.value[$c] != null) | [.key, .value[$c].version, .value[$c].address] | @tsv' "$REGISTRY_FILE")"
  if [ -z "$found" ]; then echo "$1 is not deployed on any network"; return 0; fi
  printf '%-10s %-12s %s\n' "network" "version" "address"
  echo "$found" | while IFS=$'\t' read -r n v a; do printf '%-10s %-12s %s\n' "$n" "$v" "$a"; done
}

cmd_diff() {
  [ $# -eq 2 ] || die "usage: diff NETWORK_A NETWORK_B"
  need_network "$1"; need_network "$2"
  echo "Deployed on $1 but not on $2:"
  jq -r --arg a "$1" --arg b "$2" '((.[$a] // {}) | keys) - ((.[$b] // {}) | keys) | if length == 0 then "  (none)" else .[] | "  " + . end' "$REGISTRY_FILE"
  echo "Deployed on $2 but not on $1:"
  jq -r --arg a "$1" --arg b "$2" '((.[$b] // {}) | keys) - ((.[$a] // {}) | keys) | if length == 0 then "  (none)" else .[] | "  " + . end' "$REGISTRY_FILE"
  echo "Version differences:"
  jq -r --arg a "$1" --arg b "$2" '
      [((.[$a] // {}) | keys[]) as $c | select((.[$b] // {})[$c] != null) | select(.[$a][$c].version != .[$b][$c].version)
       | "  \($c): \($a)=\(.[$a][$c].version) \($b)=\(.[$b][$c].version)"] | if length == 0 then "  (none)" else .[] end' "$REGISTRY_FILE"
}

cmd_report() {
  local n total
  echo "Deployment status ($REGISTRY_FILE)"
  for n in local testnet mainnet; do
    total="$(jq --arg n "$n" '(.[$n] // {}) | length' "$REGISTRY_FILE")"
    echo
    echo "== $n: $total contract(s)"
    [ "$total" = 0 ] || cmd_list "$n"
  done
  echo
  echo "== Coverage (contract: networks)"
  jq -r '[to_entries[] | .key as $n | .value | keys[] | {c: ., n: $n}] | group_by(.c)[] | "  \(.[0].c): \(map(.n) | join(", "))"' "$REGISTRY_FILE"
  echo
  echo "== Testnet vs mainnet"
  cmd_diff testnet mainnet
}

cmd_validate() {
  local only="${1:-}" bad=0 n c a status
  [ -z "$only" ] || need_network "$only"
  while IFS=$'\t' read -r n c a; do
    [ -z "$only" ] || [ "$only" = "$n" ] || continue
    status="$(report_check "$n" "$a")" || bad=1
    printf '%-10s %-28s %-50s %s\n' "$n" "$c" "$a" "$status"
  done < <(jq -r 'to_entries[] | .key as $n | .value | to_entries[] | [$n, .key, .value.address] | @tsv' "$REGISTRY_FILE")
  return "$bad"
}

[ $# -ge 1 ] || { usage; exit 1; }
cmd=$1; shift
case "$cmd" in
  add) cmd_add "$@" ;;
  remove) cmd_remove "$@" ;;
  list) cmd_list "$@" ;;
  where) cmd_where "$@" ;;
  diff) cmd_diff "$@" ;;
  report) cmd_report ;;
  validate) cmd_validate "$@" ;;
  -h|--help|help) usage ;;
  *) die "unknown command: $cmd (try --help)" ;;
esac
