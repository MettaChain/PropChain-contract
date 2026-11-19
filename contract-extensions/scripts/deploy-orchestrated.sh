#!/usr/bin/env bash
# =============================================================================
# deploy-orchestrated.sh: multi-contract orchestrated deployment (#1476)
#
# Deploys several ink! contracts in dependency order, passes earlier contracts'
# addresses to later constructors, health-checks each deployment and writes the
# resulting addresses to an output manifest.
#
# Usage:
#   deploy-orchestrated.sh --manifest FILE [--network local|testnet|mainnet]
#                          [--output FILE] [--suri SURI] [--url URL] [--dry-run]
#
# Options:
#   --manifest FILE   deployment manifest (JSON, format below). Required.
#   --network NAME    local (default), testnet or mainnet. Overrides "network" in the manifest.
#   --output FILE     where to write the addresses (default: deployed-<network>.json)
#   --suri SURI       signer secret URI; or set SURI. Defaults to //Alice on local only.
#   --url URL         node endpoint; or NODE_URL (local), TESTNET_URL, MAINNET_URL.
#   --dry-run         print the deployment plan (order, constructors, arguments) and exit.
#
# Endpoints: local = ws://127.0.0.1:9944, testnet = wss://rococo-contracts-rpc.polkadot.io,
# mainnet has no default: set MAINNET_URL (or --url).
#
# Deployment manifest format:
#   {
#     "network": "local",
#     "contracts": [
#       {
#         "name": "oracle",                        # unique id used by depends_on / placeholders
#         "manifest_path": "contracts/mock-oracle/Cargo.toml",
#         "constructor": "new",                    # default: "new"
#         "args": [],                              # constructor arguments, strings or numbers
#         "depends_on": [],                        # contracts that must be deployed first
#         "health_check": { "message": "admin" }   # a read-only message; optional
#       },
#       {
#         "name": "staking",
#         "manifest_path": "contracts/staking/Cargo.toml",
#         "constructor": "new",
#         "args": ["500", "1000"],
#         "depends_on": ["oracle"],
#         "health_check": { "message": "get_total_staked" }
#       },
#       {
#         "name": "consumer",
#         "manifest_path": "contracts/some/Cargo.toml",
#         "args": ["@oracle"],                     # "@name" = address of the deployed contract "name"
#         "depends_on": ["oracle"]
#       }
#     ]
#   }
#
# A "@name" argument must name a contract listed in that contract's depends_on.
# Contracts are deployed in topological order; a cycle or an unknown dependency
# is an error. The script stops at the first failure and says which contract
# failed; addresses deployed so far are already in the output file.
#
# Requires: bash, jq, cargo-contract (not needed for --dry-run).
# =============================================================================
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

MANIFEST=""
NETWORK=""
OUTPUT=""
SURI="${SURI:-}"
URL_OVERRIDE=""
DRY_RUN=0

ts() { date -u +%Y-%m-%dT%H:%M:%SZ; }
log() { local level=$1; shift; printf '%s [%s] %s\n' "$(ts)" "$level" "$*" >&2; }
die() { log ERROR "$*"; exit 1; }
need() { command -v "$1" >/dev/null 2>&1 || die "$1 is required but not installed"; }

usage() { sed -n '2,/^# ====.*$/p' "${BASH_SOURCE[0]}" | sed -n '3,$p' | sed 's/^# \{0,1\}//' | sed '$d'; }

while [ $# -gt 0 ]; do
  case "$1" in
    --manifest) MANIFEST="${2:?--manifest needs a value}"; shift 2 ;;
    --network) NETWORK="${2:?--network needs a value}"; shift 2 ;;
    --output) OUTPUT="${2:?--output needs a value}"; shift 2 ;;
    --suri) SURI="${2:?--suri needs a value}"; shift 2 ;;
    --url) URL_OVERRIDE="${2:?--url needs a value}"; shift 2 ;;
    --dry-run) DRY_RUN=1; shift ;;
    -h|--help) usage; exit 0 ;;
    *) die "unknown option: $1 (try --help)" ;;
  esac
done

[ -n "$MANIFEST" ] || die "--manifest is required"
[ -f "$MANIFEST" ] || die "manifest not found: $MANIFEST"
need jq
jq -e '.contracts | type == "array" and length > 0' "$MANIFEST" >/dev/null \
  || die "manifest must contain a non-empty \"contracts\" array"

[ -n "$NETWORK" ] || NETWORK="$(jq -r '.network // "local"' "$MANIFEST")"
[ -n "$OUTPUT" ] || OUTPUT="deployed-${NETWORK}.json"

resolve_url() {
  if [ -n "$URL_OVERRIDE" ]; then echo "$URL_OVERRIDE"; return; fi
  case "$NETWORK" in
    local) echo "${NODE_URL:-ws://127.0.0.1:9944}" ;;
    testnet) echo "${TESTNET_URL:-wss://rococo-contracts-rpc.polkadot.io}" ;;
    mainnet)
      [ -n "${MAINNET_URL:-}" ] || die "MAINNET_URL is not set and no default mainnet endpoint is assumed"
      echo "$MAINNET_URL" ;;
    *) die "unknown network: $NETWORK (use local, testnet or mainnet)" ;;
  esac
}
URL="$(resolve_url)"

# --- validation and ordering --------------------------------------------------

duplicates="$(jq -r '[.contracts[].name] | group_by(.) | map(select(length > 1) | .[0]) | .[]' "$MANIFEST")"
[ -z "$duplicates" ] || die "duplicate contract names in the manifest: $(echo "$duplicates" | tr '\n' ' ')"

unknown_deps="$(jq -r '[.contracts[].name] as $names | .contracts[] | .name as $n | (.depends_on // [])[] | select(. as $d | $names | index($d) | not) | "\($n) -> \(.)"' "$MANIFEST")"
[ -z "$unknown_deps" ] || die "unknown dependencies: $(echo "$unknown_deps" | tr '\n' ';')"

bad_placeholders="$(jq -r '.contracts[] | .name as $n | (.depends_on // []) as $deps | (.args // [])[] | select(type == "string" and startswith("@")) | .[1:] as $d | select($deps | index($d) | not) | "\($n) uses @\($d) without listing it in depends_on"' "$MANIFEST")"
[ -z "$bad_placeholders" ] || die "$bad_placeholders"

# Prints contract names in dependency order (Kahn's algorithm); dies on a cycle.
topological_order() {
  local remaining next name deps ready progress dep resolved=" "
  remaining="$(jq -r '.contracts[].name' "$MANIFEST")"
  while [ -n "$remaining" ]; do
    next=""
    progress=0
    while IFS= read -r name; do
      [ -n "$name" ] || continue
      deps="$(jq -r --arg n "$name" '.contracts[] | select(.name == $n) | (.depends_on // [])[]' "$MANIFEST")"
      ready=1
      for dep in $deps; do
        case "$resolved" in *" $dep "*) ;; *) ready=0 ;; esac
      done
      if [ "$ready" = 1 ]; then
        echo "$name"
        resolved="${resolved}${name} "
        progress=1
      else
        next="${next}${name}"$'\n'
      fi
    done <<< "$remaining"
    [ "$progress" = 1 ] || die "dependency cycle among: $(echo "$next" | tr '\n' ' ')"
    remaining="$next"
  done
}

ORDER="$(topological_order)"

contract_field() { jq -r --arg n "$1" --arg f "$2" --arg d "${3:-}" '.contracts[] | select(.name == $n) | (.[$f] // $d)' "$MANIFEST"; }

print_plan() {
  log INFO "deployment plan for network '$NETWORK' ($URL):"
  local i=1 name
  for name in $ORDER; do
    printf '  %d. %-24s %s  %s(%s)  depends on: %s\n' "$i" "$name" \
      "$(contract_field "$name" manifest_path)" \
      "$(contract_field "$name" constructor new)" \
      "$(jq -r --arg n "$name" '.contracts[] | select(.name == $n) | (.args // []) | map(tostring) | join(", ")' "$MANIFEST")" \
      "$(jq -r --arg n "$name" '.contracts[] | select(.name == $n) | (.depends_on // []) | if length == 0 then "-" else join(", ") end' "$MANIFEST")" >&2
    i=$((i + 1))
  done
}

print_plan
if [ "$DRY_RUN" = 1 ]; then
  log INFO "dry run: nothing was deployed"
  exit 0
fi

# --- deployment --------------------------------------------------------------

need cargo-contract
if [ -z "$SURI" ]; then
  [ "$NETWORK" = "local" ] || die "SURI (or --suri) is required on the $NETWORK network"
  SURI="//Alice"
fi

ADDRESSES='{}'   # name -> address, filled in as contracts are deployed
RESULTS='{}'     # name -> { address, manifest_path, health }

write_output() {
  jq -n --arg network "$NETWORK" --arg url "$URL" --arg at "$(ts)" --argjson contracts "$RESULTS" \
    '{network: $network, url: $url, updated_at: $at, contracts: $contracts}' > "$OUTPUT"
}

fail() { # name, message
  write_output
  die "deployment stopped at '$1': $2 (addresses deployed so far are in $OUTPUT)"
}

deploy_one() {
  local name=$1 manifest_path constructor health_msg args_text line out addr
  manifest_path="$(contract_field "$name" manifest_path)"
  [ -n "$manifest_path" ] || fail "$name" "no manifest_path"
  [ -f "$REPO_ROOT/$manifest_path" ] || [ -f "$manifest_path" ] || fail "$name" "manifest_path not found: $manifest_path"
  [ -f "$manifest_path" ] || manifest_path="$REPO_ROOT/$manifest_path"
  constructor="$(contract_field "$name" constructor new)"

  # Replace "@dep" arguments with the address of the deployed dependency.
  args_text="$(jq -r --arg n "$name" --argjson addrs "$ADDRESSES" '
      .contracts[] | select(.name == $n) | (.args // [])
      | map(if type == "string" and startswith("@") then ($addrs[.[1:]] // error("unresolved " + .)) else tostring end)
      | .[]' "$MANIFEST")" || fail "$name" "could not resolve constructor arguments"
  local -a args_flag=()
  if [ -n "$args_text" ]; then
    args_flag=(--args)
    while IFS= read -r line; do args_flag+=("$line"); done <<< "$args_text"
  fi

  log INFO "deploying '$name' ($manifest_path, constructor $constructor)"
  out="$(cargo contract instantiate --manifest-path "$manifest_path" --constructor "$constructor" \
        ${args_flag[@]+"${args_flag[@]}"} --suri "$SURI" --url "$URL" --skip-confirm --execute --output-json 2>&1)" \
    || { log ERROR "$out"; fail "$name" "cargo contract instantiate failed"; }
  addr="$(printf '%s' "$out" | jq -r '.contract // empty' 2>/dev/null || true)"
  [ -n "$addr" ] || { log ERROR "$out"; fail "$name" "could not read the contract address from the instantiate output"; }
  log INFO "'$name' deployed at $addr"

  ADDRESSES="$(jq -c --arg n "$name" --arg a "$addr" '. + {($n): $a}' <<< "$ADDRESSES")"
  RESULTS="$(jq -c --arg n "$name" --arg a "$addr" --arg m "$manifest_path" '. + {($n): {address: $a, manifest_path: $m, health: "not checked"}}' <<< "$RESULTS")"
  write_output

  health_msg="$(jq -r --arg n "$name" '.contracts[] | select(.name == $n) | .health_check.message // empty' "$MANIFEST")"
  if [ -n "$health_msg" ]; then
    log INFO "health check '$name': calling $health_msg"
    if cargo contract call --manifest-path "$manifest_path" --contract "$addr" --message "$health_msg" \
         --suri "$SURI" --url "$URL" --output-json >/dev/null 2>&1; then
      RESULTS="$(jq -c --arg n "$name" '.[$n].health = "ok"' <<< "$RESULTS")"
      log INFO "health check '$name': ok"
    else
      RESULTS="$(jq -c --arg n "$name" '.[$n].health = "failed"' <<< "$RESULTS")"
      fail "$name" "health check call to $health_msg failed"
    fi
    write_output
  fi
}

for name in $ORDER; do
  deploy_one "$name"
done

write_output
log INFO "all $(echo "$ORDER" | wc -l | tr -d ' ') contracts deployed"

echo
echo "Deployment summary ($NETWORK, $URL)"
printf '%-24s %-50s %s\n' "contract" "address" "health"
jq -r '.contracts | to_entries[] | [.key, .value.address, .value.health] | @tsv' "$OUTPUT" \
  | while IFS=$'\t' read -r n a h; do printf '%-24s %-50s %s\n' "$n" "$a" "$h"; done
echo
echo "Addresses written to $OUTPUT"
