#!/usr/bin/env bash
# =============================================================================
# Demo: Mock Oracle Contract
# =============================================================================
# This script demonstrates deploying and interacting with the Mock Oracle
# contract on a local testnet node.
#
# Prerequisites:
#   - `cargo contract` installed (https://github.com/paritytech/cargo-contract)
#   - A local substrate node running (see scripts/local-node.sh)
#
# Usage:
#   ./scripts/demo-mock-oracle.sh
# =============================================================================

set -euo pipefail

# ── Configuration ──────────────────────────────────────────────────────────
NODE="${NODE:-ws://127.0.0.1:9944}"
SURI="${SURI:-//Alice}"
CONTRACT_DIR="contracts/mock-oracle"

# Colours for pretty printing
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

info()  { echo -e "${CYAN}[INFO]${NC}  $*"; }
ok()    { echo -e "${GREEN}[OK]${NC}    $*"; }
warn()  { echo -e "${YELLOW}[WARN]${NC}  $*"; }
err()   { echo -e "${RED}[ERR]${NC}   $*"; }

# ── Step 0: Check prerequisites ────────────────────────────────────────────
info "Checking prerequisites…"

if ! command -v cargo-contract &>/dev/null && ! command -v cargo &>/dev/null; then
    err "cargo / cargo-contract not found. Please install:"
    err "  curl https://getsubstrate.io -sSf | bash"
    err "  cargo install cargo-contract"
    exit 1
fi

# ── Step 1: Build the contract ─────────────────────────────────────────────
info "Building Mock Oracle contract (with mock feature)…"
cd "$(git rev-parse --show-toplevel 2>/dev/null || echo "${CONTRACT_DIR%/*}")"

cargo contract build --release \
    --manifest-path "${CONTRACT_DIR}/Cargo.toml" \
    --features mock

ok "Build complete. Artifacts in ${CONTRACT_DIR}/target/ink/"

# ── Step 2: Deploy ─────────────────────────────────────────────────────────
info "Deploying Mock Oracle to ${NODE} (Alice)…"

DEPLOY_OUTPUT=$(cargo contract instantiate \
    --manifest-path "${CONTRACT_DIR}/Cargo.toml" \
    --constructor new \
    --args '' \
    --suri "${SURI}" \
    --url "${NODE}" \
    --salt 0x00000001 \
    --skip-confirm \
    --output-json 2>&1)

CONTRACT_ADDR=$(echo "${DEPLOY_OUTPUT}" | grep -o '"contract":[^,]*' | head -1 | cut -d'"' -f4)

if [ -z "${CONTRACT_ADDR}" ]; then
    # Fallback: parse from the json output
    CONTRACT_ADDR=$(echo "${DEPLOY_OUTPUT}" | python3 -c "import sys,json; print(json.load(sys.stdin).get('contract',''))" 2>/dev/null || echo "")
fi

if [ -z "${CONTRACT_ADDR}" ]; then
    warn "Could not extract contract address from output. Trying alternative parse…"
    CONTRACT_ADDR=$(echo "${DEPLOY_OUTPUT}" | grep -oE '5[a-zA-Z0-9]{47}')
fi

if [ -z "${CONTRACT_ADDR}" ]; then
    warn "Contract address not found. Deployment may have failed."
    warn "Output was:"
    echo "${DEPLOY_OUTPUT}"
    CONTRACT_ADDR="<DEPLOYED_CONTRACT_ADDRESS>"
else
    ok "Contract deployed at: ${CONTRACT_ADDR}"
fi

# ── Step 3: Interact ───────────────────────────────────────────────────────
info "Querying: is_mock_enabled()…"
cargo contract call \
    --manifest-path "${CONTRACT_DIR}/Cargo.toml" \
    --contract "${CONTRACT_ADDR}" \
    --message is_mock_enabled \
    --suri "${SURI}" \
    --url "${NODE}" \
    --dry-run 2>&1 | grep -E '(result|debug)'

info "Querying: get_valuation(property_id=1)…"
cargo contract call \
    --manifest-path "${CONTRACT_DIR}/Cargo.toml" \
    --contract "${CONTRACT_ADDR}" \
    --message get_valuation \
    --args 1 \
    --suri "${SURI}" \
    --url "${NODE}" \
    --dry-run 2>&1 | grep -E '(result|debug)'

info "Pushing price: set_price(property_id=1, price=750000)…"
cargo contract call \
    --manifest-path "${CONTRACT_DIR}/Cargo.toml" \
    --contract "${CONTRACT_ADDR}" \
    --message set_price \
    --args 1 750000 \
    --suri "${SURI}" \
    --url "${NODE}" \
    --skip-confirm 2>&1

info "Querying: get_valuation(property_id=1) after push…"
cargo contract call \
    --manifest-path "${CONTRACT_DIR}/Cargo.toml" \
    --contract "${CONTRACT_ADDR}" \
    --message get_valuation \
    --args 1 \
    --suri "${SURI}" \
    --url "${NODE}" \
    --dry-run 2>&1 | grep -E '(result|debug)'

info "Batch push: set_prices([ (2, 1200000), (3, 900000) ])…"
cargo contract call \
    --manifest-path "${CONTRACT_DIR}/Cargo.toml" \
    --contract "${CONTRACT_ADDR}" \
    --message set_prices \
    --args '[(2, 1200000), (3, 900000)]' \
    --suri "${SURI}" \
    --url "${NODE}" \
    --skip-confirm 2>&1

info "Querying: get_valuation(property_id=3)…"
cargo contract call \
    --manifest-path "${CONTRACT_DIR}/Cargo.toml" \
    --contract "${CONTRACT_ADDR}" \
    --message get_valuation \
    --args 3 \
    --suri "${SURI}" \
    --url "${NODE}" \
    --dry-run 2>&1 | grep -E '(result|debug)'

# ── Summary ────────────────────────────────────────────────────────────────
echo ""
echo "╔══════════════════════════════════════════════════════════════════╗"
echo "║           Mock Oracle Demo Complete                              ║"
echo "╠══════════════════════════════════════════════════════════════════╣"
echo "║  Contract:  ${CONTRACT_ADDR}"
echo "║  Network:   ${NODE}"
echo "║  Deployer:  ${SURI}"
echo "╚══════════════════════════════════════════════════════════════════╝"
echo ""
echo "Next steps:"
echo "  1. Use the contract address above in your E2E test fixtures."
echo "  2. Call set_price() before each test case for reproducible staging."
echo "  3. Reference the oracle from your PropertyRegistry via set_oracle()."
