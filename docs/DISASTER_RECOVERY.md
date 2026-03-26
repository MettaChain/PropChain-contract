# PropChain Disaster Recovery Plan

## Backup Strategy

### Automated Backups

Run `scripts/backup.sh` on a schedule (recommended: every 6 hours):

```bash
# Cron entry
0 */6 * * * /path/to/propchain/scripts/backup.sh /path/to/backups
```

**What is backed up:**
- Contract source code and configuration
- Deployment artifacts (addresses, ABIs)
- Environment structure (values redacted)
- Workspace manifest (Cargo.toml, Cargo.lock)
- SHA-256 checksums for integrity verification

**Retention:** Last 30 backups (configurable via `MAX_BACKUPS`).

### Manual Backup

```bash
./scripts/backup.sh ./my-backups
```

## Recovery Procedures

### 1. Contract State Recovery

On-chain state is inherently backed up by the blockchain. Recovery means
redeploying contracts and restoring configuration:

```bash
# Restore from backup
tar -xzf propchain_backup_TIMESTAMP.tar.gz
cd propchain_backup_TIMESTAMP

# Verify checksums
sha256sum -c checksums.sha256

# Redeploy contracts
cd contracts && cargo contract build --release
```

### 2. Configuration Recovery

```bash
# Restore Cargo.toml and Cargo.lock
cp Cargo.toml Cargo.lock /path/to/project/

# Recreate .env from structure
cp env_structure.txt /path/to/project/.env
# Fill in actual values manually
```

### 3. Bridge Recovery

If a bridge operation fails:

1. Call `recover_failed_bridge(request_id, RecoveryAction::RetryBridge)`
2. If retry fails, use `RecoveryAction::CancelBridge` to release funds
3. Use `RecoveryAction::RefundGas` to compensate for failed gas

### 4. Oracle Recovery

If oracle sources are compromised:

1. Pause the oracle via admin
2. Remove compromised sources (reputation drops below threshold)
3. Re-register trusted sources
4. Unpause after verification

## Data Integrity Checks

Run after any recovery:

```bash
# Verify backup integrity
sha256sum -c checksums.sha256

# Verify contract compilation
cargo contract build --release

# Run test suite
cargo test --workspace
```

## Escalation

| Severity | Response Time | Action |
|----------|--------------|--------|
| Low | 24 hours | Standard recovery procedure |
| Medium | 4 hours | Pause affected contracts, notify team |
| High | 1 hour | Emergency pause all contracts, investigate |
| Critical | Immediate | Emergency pause + bridge lockdown |
