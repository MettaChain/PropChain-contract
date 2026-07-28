#!/usr/bin/env bash
# Closes #797: track per-PR WASM contract size.
# Starter script emitting a JSON size report; wiring this into a
# .github/workflows/docs.yml step + GitHub Pages dashboard + PR comment
# is a follow-up (requires a token with the `workflow` scope to push CI files).
set -euo pipefail

CONTRACT_DIR="${1:?usage: check-wasm-size.sh <contract-dir>}"
CONTRACT_NAME="$(basename "${CONTRACT_DIR}")"

cargo contract build --release --manifest-path "${CONTRACT_DIR}/Cargo.toml"

WASM_PATH="${CONTRACT_DIR}/target/ink/${CONTRACT_NAME}.wasm"
SIZE_BYTES=$(stat -c%s "${WASM_PATH}" 2>/dev/null || stat -f%z "${WASM_PATH}")

mkdir -p wasm-size-reports
cat > "wasm-size-reports/${CONTRACT_NAME}.json" <<EOF
{"contract": "${CONTRACT_NAME}", "size_bytes": ${SIZE_BYTES}}
EOF

echo "${CONTRACT_NAME}: ${SIZE_BYTES} bytes"
