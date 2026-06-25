# Identity Recovery

This document describes the secure identity recovery mechanism for the
`propchain_identity` contract.

## Recovery flow

1. **Initiate** — the identity owner calls `initiate_recovery(new_key, guardians)`
   with a list of trusted guardian `AccountId`s and the replacement public key.
2. **Approve** — each guardian calls `approve_recovery(identity_id)`.
   The contract records each approval.
3. **Execute** — once approvals reach the configured threshold (e.g. 3-of-5),
   anyone may call `execute_recovery(identity_id)` to swap in the new key.
4. **Cancel** — the original owner (if still accessible) may call
   `cancel_recovery(identity_id)` at any time before execution.

## Error variants used

| Variant | Meaning |
|---|---|
| `RecoveryInProgress` | A recovery is already active for this identity |
| `RecoveryNotActive` | No recovery process is currently active |
| `RecoveryThresholdNotMet` | Not enough guardian approvals yet |
| `InvalidRecoveryParams` | Guardian list empty or threshold out of range |

## Storage keys

```rust
recovery_requests: Mapping<AccountId, RecoveryRequest>,
recovery_approvals: Mapping<(AccountId, AccountId), bool>, // (identity, guardian)
```

## Security notes

- Guardians must be registered identities themselves.
- A time-lock of 48 h is enforced between initiation and execution.
- The old key is invalidated immediately upon execution.