#!/usr/bin/env bash
# Run contract-extensions tests with formatting, linting, coverage, and JUnit reporting.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
EXT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
REPORT_DIR="${REPORT_DIR:-$EXT_ROOT/test-results}"
NODE_URL="${NODE_URL:-ws://127.0.0.1:9944}"
START_NODE="${START_NODE:-1}"
SELECTED_CONTRACT=""
BASE_REF="${BASE_REF:-}"
NODE_PID=""

mkdir -p "$REPORT_DIR"
STARTED_AT="$(date +%s)"
PASSED=0
FAILED=0
SKIPPED=0
declare -a RESULTS=()

usage() {
  cat <<'EOF'
Usage: contract-extensions/scripts/ci-test.sh [options]

  --contract NAME       Run only NAME under contract-extensions/contracts
  --base-ref REF        Select changed contracts by diff from REF
  --report-dir DIR      Report destination (default: contract-extensions/test-results)
  --node-url URL        Local node WebSocket URL
  --no-node             Do not start or probe a local node
  -h, --help            Show this help
EOF
}

while (($#)); do
  case "$1" in
    --contract) SELECTED_CONTRACT="${2:?missing contract name}"; shift 2 ;;
    --base-ref) BASE_REF="${2:?missing base ref}"; shift 2 ;;
    --report-dir) REPORT_DIR="${2:?missing report directory}"; shift 2 ;;
    --node-url) NODE_URL="${2:?missing node URL}"; shift 2 ;;
    --no-node) START_NODE=0; shift ;;
    -h|--help) usage; exit 0 ;;
    *) echo "Unknown option: $1" >&2; usage >&2; exit 2 ;;
  esac
done
mkdir -p "$REPORT_DIR"

cleanup() {
  if [[ -n "$NODE_PID" ]] && kill -0 "$NODE_PID" 2>/dev/null; then
    kill "$NODE_PID" 2>/dev/null || true
    wait "$NODE_PID" 2>/dev/null || true
  fi
}
trap cleanup EXIT

start_local_node() {
  if [[ "$START_NODE" != "1" ]]; then return 0; fi
  if curl --silent --fail --max-time 1 "http://127.0.0.1:9944/health" >/dev/null 2>&1; then
    echo "[ci] local node already running"
    return 0
  fi
  local node=""
  for candidate in substrate-contracts-node contracts-node; do
    if command -v "$candidate" >/dev/null 2>&1; then node="$candidate"; break; fi
  done
  if [[ -z "$node" ]]; then
    echo "[ci] local ink! node not found; integration/E2E node-dependent tests may skip" >&2
    SKIPPED=$((SKIPPED + 1))
    RESULTS+=("local-node|skipped|Local node binary not installed")
    return 0
  fi
  echo "[ci] starting $node"
  "$node" --dev --tmp --ws-port 9944 >/ "$REPORT_DIR/local-node.log" 2>&1 &
  NODE_PID=$!
  for _ in $(seq 1 30); do
    if kill -0 "$NODE_PID" 2>/dev/null &&
       curl --silent --fail --max-time 1 "http://127.0.0.1:9944/health" >/dev/null 2>&1; then
      echo "[ci] local node ready"
      return 0
    fi
    sleep 1
  done
  echo "[ci] local node failed to become ready" >&2
  cleanup
  return 1
}

xml_escape() {
  python3 - "$1" <<'PY'
import sys
print(sys.argv[1].replace("&","&amp;").replace("<","&lt;").replace(">","&gt;").replace('"',"&quot;"))
PY
}

record_result() {
  local name="$1" status="$2" duration="$3" message="${4:-}"
  RESULTS+=("$name|$status|$duration|$message")
  if [[ "$status" == "passed" ]]; then PASSED=$((PASSED + 1))
  elif [[ "$status" == "failed" ]]; then FAILED=$((FAILED + 1))
  else SKIPPED=$((SKIPPED + 1)); fi
}

run_check() {
  local name="$1"
  shift
  local began now
  began="$(date +%s)"
  local log="$REPORT_DIR/$name.log"
  if "$@" >"$log" 2>&1; then
    now="$(date +%s)"
    record_result "$name" "passed" "$((now - began))"
  else
    now="$(date +%s)"
    record_result "$name" "failed" "$((now - began))" "See $log"
  fi
}

find_manifests() {
  local -a manifests=()
  while IFS= read -r manifest; do
    if [[ -n "$SELECTED_CONTRACT" ]]; then
      [[ "$manifest" == *"/contracts/$SELECTED_CONTRACT/Cargo.toml" ||
         "$manifest" == *"/tests/$SELECTED_CONTRACT/Cargo.toml" ]] || continue
    fi
    manifests+=("$manifest")
  done < <(find "$EXT_ROOT/contracts" "$EXT_ROOT/tests" -name Cargo.toml -type f 2>/dev/null | sort)

  if [[ -n "$BASE_REF" && -z "$SELECTED_CONTRACT" ]]; then
    local changed
    changed="$(git -C "$(cd "$EXT_ROOT/.." && pwd)" diff --name-only "$BASE_REF"...HEAD -- contract-extensions/contracts contract-extensions/tests 2>/dev/null || true)"
    local -a filtered=()
    for manifest in "${manifests[@]}"; do
      local rel="${manifest#$EXT_ROOT/}"
      if grep -Fq "contract-extensions/${rel%%/Cargo.toml}" <<<"$changed"; then filtered+=("$manifest"); fi
    done
    manifests=("${filtered[@]}")
  fi
  printf '%s\n' "${manifests[@]}"
}

MANIFESTS=()
while IFS= read -r manifest; do [[ -n "$manifest" ]] && MANIFESTS+=("$manifest"); done < <(find_manifests)

start_local_node

run_manifest_check() {
  local name="$1"
  shift
  local began
  began="$(date +%s)"
  local log="$REPORT_DIR/$name.log"
  : >"$log"
  local failures=0
  for manifest in "${MANIFESTS[@]}"; do
    if ! cargo "$@" --manifest-path "$manifest" >>"$log" 2>&1; then failures=$((failures + 1)); fi
  done
  local now status
  now="$(date +%s)"
  if (( failures == 0)); then status="passed"; else status="failed"; fi
  record_result "$name" "$status" "$((now - began))" "${failures} manifest(s) failed"
}

run_manifest_check "fmt" fmt --check
run_manifest_check "clippy" clippy --all-targets --all-features

# Unit, integration, and E2E groups run concurrently. Each manifest is isolated
# by --manifest-path so every extension crate is exercised independently.
declare -a PIDS=()
run_test_group() {
  local group="$1"
  shift
  local began="$2"
  shift
  local log="$REPORT_DIR/$group.log"
  : >"$log"
  local failures=0
  for manifest in "${MANIFESTS[@]}"; do
    echo "[ci] $group $(basename "$(dirname "$manifest")")" >>"$log"
    if ! cargo "$@" --manifest-path "$manifest" >>"$log" 2>&1; then failures=$((failures + 1)); fi
  done
  local now status message
  now="$(date +%s)"
  if (( failures == 0)); then status="passed"; else status="failed"; fi
  message="${failures} manifest(s) failed"
  printf '%s|%s|%s|%s\n' "$group" "$status" "$((now - began))" "$message" >"$REPORT_DIR/$group.result"
}

unit_begin="$(date +%s)"; integration_begin="$(date +%s)"; e2e_begin="$(date +%s)"
run_test_group "unit" "$unit_begin" test --lib &
PIDS+=($!)
run_test_group "integration" "$integration_begin" test --tests &
PIDS+=($!)
run_test_group "e2e" "$e2e_begin" test --test '*' &
PIDS+=($!)
for pid in "${PIDS[@]}"; do wait "$pid" || true; done
for group in unit integration e2e; do
  IFS='|' read -r _ status duration message <"$REPORT_DIR/$group.result"
  record_result "$group" "$status" "$duration" "$message"
done

# Use cargo-nextest when it is available; otherwise cargo test is the complete fallback.
if command -v cargo-nextest >/dev/null 2>&1; then
  nextest_log="$REPORT_DIR/nextest.log"
  : >"$nextest_log"
  for manifest in "${MANIFESTS[@]}"; do
    if ! cargo nextest run --manifest-path "$manifest" --profile ci >>"$nextest_log" 2>&1; then
      echo "[ci] nextest failed for $manifest" >>"$nextest_log"
    fi
  done
fi

if command -v cargo-tarpaulin >/dev/null 2>&1; then
  coverage_begin="$(date +%s)"
  coverage_log="$REPORT_DIR/coverage.log"
  coverage_failures=0
  : >"$coverage_log"
  for manifest in "${MANIFESTS[@]}"; do
    coverage_dir="$REPORT_DIR/coverage/$(basename "$(dirname "$manifest")")"
    mkdir -p "$coverage_dir"
    if ! cargo tarpaulin --manifest-path "$manifest" --out Xml --out Html --output-dir "$coverage_dir" >>"$coverage_log" 2>&1; then
      coverage_failures=$((coverage_failures + 1))
    fi
  done
  coverage_status="passed"
  (( coverage_failures == 0 )) || coverage_status="failed"
  record_result "coverage" "$coverage_status" "$(( $(date +%s) - coverage_begin ))" "${coverage_failures} manifest(s) failed"
elif command -v cargo-llvm-cov >/dev/null 2>&1; then
  coverage_begin="$(date +%s)"
  coverage_log="$REPORT_DIR/coverage.log"
  coverage_failures=0
  : >"$coverage_log"
  for manifest in "${MANIFESTS[@]}"; do
    output="$REPORT_DIR/coverage-$(basename "$(dirname "$manifest")").lcov"
    if ! cargo llvm-cov --manifest-path "$manifest" --lcov --output-path "$output" >>"$coverage_log" 2>&1; then
      coverage_failures=$((coverage_failures + 1))
    fi
  done
  coverage_status="passed"
  (( coverage_failures == 0 )) || coverage_status="failed"
  record_result "coverage" "$coverage_status" "$(( $(date +%s) - coverage_begin ))" "${coverage_failures} manifest(s) failed"
else
  record_result "coverage" "skipped" "0" "Install cargo-tarpaulin or cargo-llvm-cov"
  echo "[ci] coverage tool not installed" >"$REPORT_DIR/coverage.log"
fi

# A small JUnit-compatible report is generated for every CI consumer.
{
  echo '<?xml version="1.0" encoding="UTF-8"?>'
  echo "<testsuites tests=\"$((PASSED + FAILED + SKIPPED))\" failures=\"$FAILED\" skipped=\"$SKIPPED\">"
  for result in "${RESULTS[@]}"; do
    IFS='|' read -r name status duration message <<<"$result"
    escaped_name="$(xml_escape "$name")"
    escaped_message="$(xml_escape "$message")"
    echo "  <testsuite name=\"$escaped_name\" tests=\"1\" failures=\"$([[ "$status" == "failed" ]] && echo 1 || echo 0)\" skipped=\"$([[ "$status" == "skipped" ]] && echo 1 || echo 0)\" time=\"$duration\">"
    echo "    <testcase classname=\"contract-extensions.ci\" name=\"$escaped_name\" time=\"$duration\">"
    case "$status" in
      failed) echo "      <failure message=\"$escaped_message\">$escaped_message</failure>" ;;
      skipped) echo "      <skipped message=\"$escaped_message\"/>" ;;
    esac
    echo "    </testcase>"
    echo "  </testsuite>"
  done
  echo "</testsuites>"
} >"$REPORT_DIR/junit.xml"

ENDED_AT="$(date +%s)"
TOTAL_DURATION=$((ENDED_AT - STARTED_AT))
{
  echo "Contract CI Summary"
  echo "==================="
  echo "Passed:  $PASSED"
  echo "Failed:  $FAILED"
  echo "Skipped: $SKIPPED"
  echo "Duration: ${TOTAL_DURATION}s"
  echo "Coverage: $(if [[ -f "$REPORT_DIR/cobertura.xml" ]]; then echo "report at $REPORT_DIR/cobertura.xml"; elif [[ -d "$REPORT_DIR/coverage" ]]; then echo "report at $REPORT_DIR/coverage"; else echo "not available"; fi)"
  echo "JUnit: $REPORT_DIR/junit.xml"
} | tee "$REPORT_DIR/summary.txt"

if (( FAILED > 0)); then exit 1; fi
