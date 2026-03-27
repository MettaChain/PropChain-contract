# PropChain Storage Optimization Guidelines

> Standardized storage patterns for all ink! contracts in the PropChain workspace.
> Ref: [Issue #78 — Inconsistent Storage Patterns](https://github.com/MettaChain/PropChain-contract/issues/78)

## 1. Storage Gap Reservations

Every contract MUST include a storage gap at the **end** of its `#[ink(storage)]` struct
to allow future fields to be added without breaking existing storage layout.

```rust
use propchain_traits::constants::STORAGE_GAP_SIZE;

#[ink(storage)]
pub struct MyContract {
    // ... business fields ...

    /// Reserved for future storage upgrades — DO NOT USE.
    __storage_gap: [u128; STORAGE_GAP_SIZE],
}
```

When adding a new field later, **remove one slot** from `__storage_gap` for each
field added and place the new field directly before the gap. This preserves the
overall layout.

## 2. Struct Packing

- **Group small fields together.** Place `bool`, `u8`, and `u32` fields adjacent to
  each other so the SCALE codec can pack them efficiently.
- **Use `u128` for all balances and monetary values.** This avoids overflow issues and
  aligns with the `TOKEN_SCALING_FACTOR` constant (`1e12`).
- **Prefer `Mapping<K, V>` over `Vec<V>`** for any unbounded collection. Vectors are
  stored inline and load entirely on every access; mappings use lazy storage.

## 3. Mapping Key Conventions

| Pattern | Key type | Example |
|---------|---------|---------|
| Single key | `Mapping<K, V>` | `token_owner: Mapping<TokenId, AccountId>` |
| Composite key | `Mapping<(K1, K2), V>` | `balances: Mapping<(AccountId, TokenId), u128>` |
| Counter + index | Counter `u64` + `Mapping<(Parent, u32), Child>` | `ownership_history_count` + `ownership_history_items` |

- Always use **tuples** for composite keys — never concatenate bytes manually.
- Use the `Counter + Mapping` pattern instead of `Mapping<K, Vec<V>>` to avoid
  loading and re-encoding the entire vector on every mutation.

## 4. Naming Conventions

| Field type | Convention | Example |
|-----------|-----------|---------|
| Mapping | Descriptive noun | `token_owner`, `stake_info` |
| Counter | `*_counter` or `*_count` | `proposal_counter`, `error_log_counter` |
| Config struct | `*_config` | `bridge_config`, `staking_config` |
| Admin/owner | `admin` | `admin: AccountId` |
| Storage gap | `__storage_gap` | `__storage_gap: [u128; 20]` |

## 5. Derive Macro Checklist

All public structs stored on-chain MUST derive:

```rust
#[derive(
    Debug, Clone, PartialEq, Eq,
    scale::Encode, scale::Decode,
    ink::storage::traits::StorageLayout,
)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
```

Omit `Eq` only when the struct contains `f64` or other non-`Eq` types
(none exist in PropChain today).

## 6. Constants

Import all magic numbers from `propchain_traits::constants` instead of
using inline literals. When adding a new constant, add it to `constants.rs`
with a doc comment explaining the valid range and purpose.
