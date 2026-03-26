#!/bin/bash
# =============================================================================
# PropChain Automated Backup Script
# =============================================================================
# Creates timestamped backups of contract state, configuration, and metadata.
# Designed for cron scheduling: 0 */6 * * * /path/to/backup.sh
#
# Usage: ./backup.sh [backup_dir]
# =============================================================================

set -euo pipefail

BACKUP_DIR="${1:-./backups}"
TIMESTAMP=$(date +%Y%m%d_%H%M%S)
BACKUP_NAME="propchain_backup_${TIMESTAMP}"
BACKUP_PATH="${BACKUP_DIR}/${BACKUP_NAME}"
MAX_BACKUPS=30  # Keep last 30 backups

echo "=== PropChain Backup: ${TIMESTAMP} ==="

# Create backup directory
mkdir -p "${BACKUP_PATH}"

# 1. Backup contract source and configuration
echo "[1/4] Backing up contract source..."
tar -czf "${BACKUP_PATH}/contracts.tar.gz" \
  --exclude='target' \
  --exclude='*.wasm' \
  contracts/ 2>/dev/null || echo "Warning: contracts directory not found"

# 2. Backup deployment artifacts
echo "[2/4] Backing up deployment artifacts..."
if [ -d "deployments" ]; then
  cp -r deployments/ "${BACKUP_PATH}/deployments/"
fi

# Export contract addresses and metadata if available
if command -v cargo-contract &> /dev/null; then
  echo "  Exporting contract metadata..."
  for contract_dir in contracts/*/; do
    contract_name=$(basename "${contract_dir}")
    if [ -f "${contract_dir}/Cargo.toml" ]; then
      echo "  - ${contract_name}"
    fi
  done > "${BACKUP_PATH}/contract_inventory.txt" 2>/dev/null || true
fi

# 3. Backup configuration and environment
echo "[3/4] Backing up configuration..."
if [ -f ".env" ]; then
  # Strip sensitive values, keep structure
  sed 's/=.*/=<REDACTED>/' .env > "${BACKUP_PATH}/env_structure.txt"
fi

# Copy non-sensitive config files
for config_file in Cargo.toml Cargo.lock; do
  [ -f "${config_file}" ] && cp "${config_file}" "${BACKUP_PATH}/"
done

# 4. Create integrity checksum
echo "[4/4] Computing checksums..."
find "${BACKUP_PATH}" -type f -exec sha256sum {} \; > "${BACKUP_PATH}/checksums.sha256"

# Compress the full backup
tar -czf "${BACKUP_DIR}/${BACKUP_NAME}.tar.gz" -C "${BACKUP_DIR}" "${BACKUP_NAME}"
rm -rf "${BACKUP_PATH}"

echo "Backup created: ${BACKUP_DIR}/${BACKUP_NAME}.tar.gz"

# Rotate old backups
BACKUP_COUNT=$(ls -1 "${BACKUP_DIR}"/propchain_backup_*.tar.gz 2>/dev/null | wc -l)
if [ "${BACKUP_COUNT}" -gt "${MAX_BACKUPS}" ]; then
  REMOVE_COUNT=$((BACKUP_COUNT - MAX_BACKUPS))
  ls -1t "${BACKUP_DIR}"/propchain_backup_*.tar.gz | tail -n "${REMOVE_COUNT}" | xargs rm -f
  echo "Rotated ${REMOVE_COUNT} old backup(s)"
fi

echo "=== Backup complete ==="
