#!/usr/bin/env bash
# scripts/dev-watch.sh
#
# Hot-reload development watcher for PropChain contracts.
# Watches contracts/ and tests/ for changes, then rebuilds and reruns tests.
#
# Prerequisites: cargo-watch (cargo install cargo-watch)
#
# Usage:
#   ./scripts/dev-watch.sh              # watch all contracts
#   WATCH_PATHS="contracts/staking contracts/escrow" ./scripts/dev-watch.sh
#   CONTRACT=staking ./scripts/dev-watch.sh  # run only one contract's tests

set -euo pipefail

# ── Configurable defaults ──────────────────────────────────────────────────
WATCH_PATHS="${WATCH_PATHS:-contracts tests}"
CONTRACT="${CONTRACT:-}"          # if set, run only tests matching this name
BUILD_FLAGS="${BUILD_FLAGS:-}"    # extra flags forwarded to cargo build
TEST_FLAGS="${TEST_FLAGS:-}"      # extra flags forwarded to cargo test

# ── Colour helpers ─────────────────────────────────────────────────────────
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
RESET='\033[0m'

log()  { echo -e "${CYAN}[watch]${RESET} $*"; }
ok()   { echo -e "${GREEN}[pass]${RESET}  $*"; }
err()  { echo -e "${RED}[fail]${RESET}  $*"; }
warn() { echo -e "${YELLOW}[warn]${RESET}  $*"; }

# ── Dependency check ───────────────────────────────────────────────────────
if ! command -v cargo-watch &>/dev/null; then
    warn "cargo-watch not found. Installing..."
    cargo install cargo-watch --locked
fi

# ── Build & test command ───────────────────────────────────────────────────
build_cmd() {
    log "Building workspace..."
    if cargo build --workspace ${BUILD_FLAGS}; then
        ok "Build succeeded"
    else
        err "Build failed"
        return 1
    fi
}

test_cmd() {
    log "Running tests..."
    local filter=""
    if [[ -n "${CONTRACT}" ]]; then
        filter="--package ${CONTRACT}"
        log "Filtering to contract: ${CONTRACT}"
    fi
    if cargo test --workspace ${filter} ${TEST_FLAGS} 2>&1; then
        ok "All tests passed"
    else
        err "Tests failed"
        return 1
    fi
}

# ── Watch paths ────────────────────────────────────────────────────────────
# Build watch flags (-w path1 -w path2 ...)
watch_flags=""
for p in ${WATCH_PATHS}; do
    watch_flags="${watch_flags} -w ${p}"
done

log "Watching: ${WATCH_PATHS}"
log "Press Ctrl-C to stop"

# ── Start watcher ──────────────────────────────────────────────────────────
# shellcheck disable=SC2086
exec cargo watch \
    ${watch_flags} \
    --clear \
    --shell "bash -c '$(declare -f build_cmd test_cmd log ok err warn); build_cmd && test_cmd'"
