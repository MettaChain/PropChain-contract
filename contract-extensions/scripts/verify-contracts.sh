#!/usr/bin/env bash
# =============================================================================
# verify-contracts.sh: contract verification and source upload (#1479)
#
# For every contract in a deployment manifest: build it, read the local code hash from the
# built bundle, read the on-chain code hash, compare them, and upload source + metadata to a
# verification service. Prints a report with the status of each contract:
#   verified        local and on-chain code hashes match (and the upload succeeded or was skipped)
#   mismatch        the hashes differ
#   upload failed   hashes match but uploading to the verification service failed after retries
#   build failed / chain error   the build or the on-chain lookup failed
#
# Usage:
#   verify-contracts.sh --manifest FILE [--network local|testnet|mainnet] [--url URL]
#                       [--report FILE] [--no-upload] [--no-build] [--retries N]
#
# Manifest (JSON):
#   { "network": "testnet",
#     "contracts": [ { "name": "staking", "address": "5F...", "manifest_path": "contracts/staking/Cargo.toml" } ] }
#
# Verification endpoints (network-specific, no defaults are assumed; uploads are skipped
# with a note when the endpoint for the network is not set):
#   VERIFY_URL_LOCAL / VERIFY_URL_TESTNET / VERIFY_URL_MAINNET   (or VERIFY_URL for all)
#   VERIFY_API_KEY   optional bearer token
# The upload is a multipart POST with fields: address, network, name, bundle (the .contract
# file) and source (a tar.gz of the contract directory). Adapt upload_one() to the exact API
# of the service you use (for example Contracts Explorer); the request shape here is generic.
#
# Node endpoints: local NODE_URL (ws://127.0.0.1:9944), testnet TESTNET_URL
# (wss://rococo-contracts-rpc.polkadot.io), mainnet MAINNET_URL (no default).
# Exit code is 1 if any contract is not "verified".
# Requires: bash, jq, curl, tar, cargo-contract.
# =============================================================================
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

MANIFEST=""; NETWORK=""; URL_OVERRIDE=""; REPORT=""; UPLOAD=1; BUILD=1; RETRIES=3

ts() { date -u +%Y-%m-%dT%H:%M:%SZ; }
log() { local level=$1; shift; printf '%s [%s] %s\n' "$(ts)" "$level" "$*" >&2; }
die() { log ERROR "$*"; exit 1; }
need() { command -v "$1" >/dev/null 2>&1 || die "$1 is required but not installed"; }
usage() { sed -n '3,/^# ====/p' "${BASH_SOURCE[0]}" | sed '$d' | sed 's/^# \{0,1\}//'; }

while [ $# -gt 0 ]; do
  case "$1" in
    --manifest) MANIFEST="${2:?}"; shift 2 ;;
    --network) NETWORK="${2:?}"; shift 2 ;;
    --url) URL_OVERRIDE="${2:?}"; shift 2 ;;
    --report) REPORT="${2:?}"; shift 2 ;;
    --no-upload) UPLOAD=0; shift ;;
    --no-build) BUILD=0; shift ;;
    --retries) RETRIES="${2:?}"; shift 2 ;;
    -h|--help) usage; exit 0 ;;
    *) die "unknown option: $1 (try --help)" ;;
  esac
done

[ -n "$MANIFEST" ] || die "--manifest is required"
[ -f "$MANIFEST" ] || die "manifest not found: $MANIFEST"
need jq; need cargo-contract
[ "$UPLOAD" = 0 ] || need curl
[ "$UPLOAD" = 0 ] || need tar
jq -e '.contracts | type == "array" and length > 0' "$MANIFEST" >/dev/null || die "manifest needs a non-empty \"contracts\" array"
[ -n "$NETWORK" ] || NETWORK="$(jq -r '.network // "local"' "$MANIFEST")"

case "$NETWORK" in
  local) URL="${URL_OVERRIDE:-${NODE_URL:-ws://127.0.0.1:9944}}"; VERIFY_ENDPOINT="${VERIFY_URL_LOCAL:-${VERIFY_URL:-}}" ;;
  testnet) URL="${URL_OVERRIDE:-${TESTNET_URL:-wss://rococo-contracts-rpc.polkadot.io}}"; VERIFY_ENDPOINT="${VERIFY_URL_TESTNET:-${VERIFY_URL:-}}" ;;
  mainnet) URL="${URL_OVERRIDE:-${MAINNET_URL:-}}"; [ -n "$URL" ] || die "set MAINNET_URL or --url"; VERIFY_ENDPOINT="${VERIFY_URL_MAINNET:-${VERIFY_URL:-}}" ;;
  *) die "unknown network: $NETWORK" ;;
esac

retry() { # command...; retries with exponential backoff
  local attempt=1 delay=2
  while true; do
    if "$@"; then return 0; fi
    [ "$attempt" -lt "$RETRIES" ] || return 1
    log WARN "attempt $attempt/$RETRIES failed; retrying in ${delay}s"
    sleep "$delay"
    attempt=$((attempt + 1)); delay=$((delay * 2))
  done
}

onchain_hash() { # address
  cargo contract info --contract "$1" --url "$URL" --output-json 2>/dev/null | jq -er '.code_hash'
}

upload_one() { # name address bundle source_dir
  local name=$1 addr=$2 bundle=$3 src=$4 tarball auth=()
  [ -z "${VERIFY_API_KEY:-}" ] || auth=(-H "Authorization: Bearer $VERIFY_API_KEY")
  tarball="$(mktemp -t verify-src.XXXXXX)"
  tar -czf "$tarball" --exclude target --exclude .git -C "$src" . || { rm -f "$tarball"; return 1; }
  if curl -fsS --max-time 120 ${auth[@]+"${auth[@]}"} -F "address=$addr" -F "network=$NETWORK" -F "name=$name" \
       -F "bundle=@$bundle" -F "source=@$tarball" "$VERIFY_ENDPOINT" >/dev/null; then
    rm -f "$tarball"; return 0
  fi
  rm -f "$tarball"; return 1
}

declare -a R_NAME=() R_ADDR=() R_LOCAL=() R_CHAIN=() R_STATUS=()
BAD=0

count="$(jq '.contracts | length' "$MANIFEST")"
i=0
while [ "$i" -lt "$count" ]; do
  name="$(jq -r ".contracts[$i].name" "$MANIFEST")"
  addr="$(jq -r ".contracts[$i].address" "$MANIFEST")"
  mpath="$(jq -r ".contracts[$i].manifest_path" "$MANIFEST")"
  i=$((i + 1))
  [ -f "$mpath" ] || mpath="$REPO_ROOT/$mpath"
  local_hash="-"; chain_hash="-"; status=""

  log INFO "[$name] verifying $addr"
  if [ ! -f "$mpath" ]; then
    status="build failed (manifest_path not found)"
  else
    bundle=""
    if [ "$BUILD" = 1 ]; then
      if out="$(cargo contract build --release --manifest-path "$mpath" --output-json 2>/dev/null)"; then
        bundle="$(printf '%s' "$out" | jq -r '.dest_bundle // empty' 2>/dev/null || true)"
      else
        status="build failed"
      fi
    else
      bundle="$(dirname "$mpath")/target/ink/$(basename "$(dirname "$mpath")" | tr '-' '_').contract"
    fi
    if [ -z "$status" ]; then
      if [ -f "$bundle" ]; then
        local_hash="$(jq -r '.source.hash // empty' "$bundle")"
      fi
      [ -n "$local_hash" ] && [ "$local_hash" != "-" ] || status="build failed (no code hash in bundle)"
    fi
  fi

  if [ -z "$status" ]; then
    if chain_hash="$(retry onchain_hash "$addr")"; then
      if [ "$chain_hash" != "$local_hash" ]; then
        status="mismatch"
      else
        status="verified"
        if [ "$UPLOAD" = 1 ]; then
          if [ -z "$VERIFY_ENDPOINT" ]; then
            log WARN "[$name] no verification endpoint for $NETWORK (set VERIFY_URL_$(echo "$NETWORK" | tr a-z A-Z)); upload skipped"
            status="verified (upload skipped)"
          elif retry upload_one "$name" "$addr" "$bundle" "$(dirname "$mpath")"; then
            log INFO "[$name] source uploaded"
          else
            status="upload failed"
          fi
        fi
      fi
    else
      chain_hash="-"
      status="chain error"
    fi
  fi

  case "$status" in verified*) ;; *) BAD=1 ;; esac
  log INFO "[$name] $status"
  R_NAME+=("$name"); R_ADDR+=("$addr"); R_LOCAL+=("$local_hash"); R_CHAIN+=("$chain_hash"); R_STATUS+=("$status")
done

print_report() {
  printf 'Verification report (%s, %s) %s\n' "$NETWORK" "$URL" "$(ts)"
  local k=0
  while [ "$k" -lt "${#R_NAME[@]}" ]; do
    printf '%s\n  address:   %s\n  local:     %s\n  on-chain:  %s\n  status:    %s\n' \
      "${R_NAME[$k]}" "${R_ADDR[$k]}" "${R_LOCAL[$k]}" "${R_CHAIN[$k]}" "${R_STATUS[$k]}"
    k=$((k + 1))
  done
}
echo
print_report
if [ -n "$REPORT" ]; then
  print_report > "$REPORT"
  log INFO "report written to $REPORT"
fi
exit "$BAD"
