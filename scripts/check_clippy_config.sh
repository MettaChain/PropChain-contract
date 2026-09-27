#!/usr/bin/env bash
# Guard: `clippy.toml` must contain only options Clippy actually parses.
#
# Clippy reports an unrecognised option as a hard error
# (`error: unknown field name`) instead of silently ignoring it, so the cheapest
# way to prove the file parses cleanly is to point Clippy at it and compile
# something trivial. That is exactly what this script does:
#
#   1. build a throwaway, dependency-free probe crate in a private target dir,
#      so Clippy always recompiles and therefore always reads the config; and
#   2. run `cargo clippy` there with `CLIPPY_CONF_DIR` aimed at this repository.
#
# Exits non-zero if Clippy rejects any key, with Clippy's own diagnostic
# (including the accepted-key list) printed above the failure.
#
# Usage: scripts/check_clippy_config.sh [path/to/clippy.toml]
# Env:   CLIPPY_TOOLCHAIN — toolchain to run (default: nightly, the toolchain
#        pinned by rust-toolchain.toml).
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
CLIPPY_CONFIG="${1:-${REPO_ROOT}/clippy.toml}"
CLIPPY_TOOLCHAIN="${CLIPPY_TOOLCHAIN:-nightly}"
CONFIG_DIR="$(cd "$(dirname "${CLIPPY_CONFIG}")" && pwd)"
CONFIG_NAME="$(basename "${CLIPPY_CONFIG}")"

if [[ ! -f "${CLIPPY_CONFIG}" ]]; then
    echo "clippy-config: ${CLIPPY_CONFIG} not found" >&2
    exit 1
fi

# clippy only looks for a file literally named `clippy.toml`; CLIPPY_CONF_DIR
# names the directory it lives in, so an out-of-tree path needs staging.
STAGE_DIR="$(mktemp -d)"
PROBE_DIR="$(mktemp -d)"
cleanup() {
    rm -rf "${STAGE_DIR}" "${PROBE_DIR}"
}
trap cleanup EXIT

cp "${CLIPPY_CONFIG}" "${STAGE_DIR}/clippy.toml"
mkdir -p "${PROBE_DIR}/src"
cat > "${PROBE_DIR}/Cargo.toml" <<'EOF'
[package]
name = "clippy-config-probe"
version = "0.0.0"
edition = "2021"

[workspace]
EOF

cat > "${PROBE_DIR}/src/main.rs" <<'EOF'
fn main() {}
EOF

echo "clippy-config: checking ${CONFIG_NAME} against \`${CLIPPY_TOOLCHAIN}\` Clippy"

# The probe lives outside the workspace and gets its own target dir, so this
# never inherits (or pollutes) the repository's build cache and never consults
# the real clippy.toml by accident.
set +e
PROBE_OUTPUT="$(
    cd "${PROBE_DIR}" &&
        CARGO_TARGET_DIR="${PROBE_DIR}/target" \
        CLIPPY_CONF_DIR="${STAGE_DIR}" \
        cargo "+${CLIPPY_TOOLCHAIN}" clippy --quiet 2>&1
)"
PROBE_STATUS=$?
set -e

if [[ ${PROBE_STATUS} -ne 0 ]]; then
    echo "${PROBE_OUTPUT}" >&2
    if grep -q "unknown field name" <<<"${PROBE_OUTPUT}"; then
        echo "clippy-config: FAIL — ${CONFIG_NAME} sets an option Clippy does not recognise (see the accepted list above)." >&2
    else
        echo "clippy-config: FAIL — Clippy could not be run to validate ${CONFIG_NAME}." >&2
    fi
    exit 1
fi

if grep -q "unknown field name" <<<"${PROBE_OUTPUT}"; then
    echo "${PROBE_OUTPUT}" >&2
    echo "clippy-config: FAIL — ${CONFIG_NAME} sets an option Clippy does not recognise." >&2
    exit 1
fi

echo "clippy-config: OK — every option in ${CONFIG_NAME} is parsed by Clippy."
