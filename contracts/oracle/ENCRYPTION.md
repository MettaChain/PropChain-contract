# Oracle Data Encryption

## Overview
Sensitive oracle data (property valuations, identity scores) must be encrypted
in transit and at rest to prevent front-running and data leakage.

## In-transit encryption
All oracle submissions travel over TLS 1.3. No plaintext oracle endpoints are
exposed. The `authorized_oracles` mapping ensures only registered keys may submit.

## At-rest encryption

### Approach: encrypt before storage
```rust
// Derive a per-data-point key from the oracle key + data_id
let enc_key = env.crypto().sha256(&(oracle_pubkey, data_id).into_val(&env));
// Store the ciphertext; decrypt on read with the same derivation
env.storage().persistent().set(&DataKey::EncryptedData(data_id), &ciphertext);
```

### Key management
- Each oracle keypair is rotated on a schedule (see `schedule_oracle_removal`).
- Old keys are retained for decryption of historical data until all dependent
  records are migrated.
- Admin may call `rotate_encryption_key(old_key, new_key)` to re-encrypt
  outstanding records.

## Threat model
| Threat | Mitigation |
|---|---|
| Front-running via mempool observation | Encrypt payload; only oracle can produce valid ciphertext |
| Storage read by malicious contract | Per-record derived keys; no global secret in storage |
| Oracle key compromise | Grace-period rotation; historical data re-encryption |