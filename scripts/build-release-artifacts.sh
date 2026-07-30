#!/usr/bin/env bash
# Closes #791: build release artifacts for a tagged contract.
# Starter script; wiring this into .github/workflows/release.yml is a
# follow-up (requires a token with the `workflow` scope to push CI files).
set -euo pipefail

CONTRACT_DIR="${1:?usage: build-release-artifacts.sh <contract-dir>}"
OUT_DIR="release-artifacts"

mkdir -p "${OUT_DIR}"
(cd "${CONTRACT_DIR}" && cargo contract build --release)

CONTRACT_NAME="$(basename "${CONTRACT_DIR}")"
cp "${CONTRACT_DIR}/target/ink/${CONTRACT_NAME}.contract" "${OUT_DIR}/"
cp "${CONTRACT_DIR}/target/ink/metadata.json" "${OUT_DIR}/${CONTRACT_NAME}.metadata.json"

(cd "${OUT_DIR}" && sha256sum ./*.contract ./*.metadata.json > SHA256SUMS.txt)

echo "Artifacts ready in ${OUT_DIR}/"
