# ink! Storage Patterns in PropChain

This guide describes storage choices visible in the PropChain contracts. The
examples below are from the repository; they are excerpts, not a substitute for
checking the current source and generated storage layout before an upgrade.

## `Mapping` or `Vec`?

`ink::storage::Mapping<K, V>` is the default for records keyed by an account,
identifier, or tuple of identifiers. An individual `get`, `insert`, or `remove`
uses the key to address the corresponding storage entry. It does not expose
iteration over all keys, which avoids accidentally making a common message
proportional to the number of users or records.

```rust
// contracts/staking/src/lib.rs
stakes: Mapping<AccountId, StakeInfo>,
param_proposals: Mapping<u64, ParamProposal>,
param_votes: Mapping<(u64, AccountId), bool>,
```

The staking contract uses keyed records alongside ordered identifier lists:

```rust
// contracts/staking/src/lib.rs
stakes: Mapping<AccountId, StakeInfo>,
staker_list: Vec<AccountId>,
```

The mapping supports direct account lookup; the vector supports enumeration
where an ordered collection is actually needed. `contracts/governance/src/lib.rs`
similarly stores proposals and votes in `Mapping`s, using `(proposal_id,
AccountId)` as the vote key. `contracts/identity/lib.rs` uses mappings for
identity records, reputation, trust assessments, and audit entries.

| Operation / shape | `Mapping` | `Vec` |
|---|---|---|
| Lookup by a known key | Keyed storage access; does not scan other records | Usually scans until a match unless a separate index is maintained |
| Enumerate all records | Not provided by `Mapping` | Natural, but the cost grows with the collection |
| Append | Not applicable; insert by key | Appending changes the collection value and may require encoding/writing it |
| Best fit | User/id keyed records and sparse state | Small, bounded lists or an explicit index of keys |

### Scaling and gas measurement

The following is a **storage-operation scaling comparison**, not a measured
gas/weight benchmark. It counts record accesses/comparisons for a successful
lookup by value in a vector. It does not claim specific gas units: runtime,
chain weights, storage proof size, key/value encoding, and the benchmark
environment all affect actual numbers.

| Entries | Mapping lookup | Linear `Vec` search (up to) | Expected scaling |
|---:|---:|---:|---|
| 10 | 1 keyed lookup | 10 element checks | Mapping stays constant; vector grows with N |
| 100 | 1 keyed lookup | 100 element checks | Mapping stays constant; vector grows with N |
| 1,000 | 1 keyed lookup | 1,000 element checks | Mapping stays constant; vector grows with N |

To produce comparable gas numbers, benchmark the same key and payload sizes on
the target `pallet-contracts` runtime at each collection size, recording both
ref-time and proof-size. No such measurements are published here; the table
must not be treated as numeric gas results.

Avoid fetching and cloning a large vector just to find one item. Prefer a
mapping, or keep a bounded vector of IDs plus a mapping of values if both
enumeration and direct lookup are required. If a vector-backed collection is
necessary, cap its size and consider pagination.

## Lazy loading and `Lazy<T>`

The referenced staking, governance, and identity contracts do not use
`ink::storage::Lazy`; their large keyed collections use `Mapping`. In general,
a lazy cell is useful when a single, potentially large value is expensive to
load on messages that do not need it. Keep in mind that accessing a lazy
aggregate still loads and decodes that aggregate as a value.

`Lazy<HashMap<K, V>>` is therefore not a replacement for
`Mapping<K, V>`: accessing an entry in the in-memory map requires loading the
whole persisted map first, and updating it can encode/write the aggregate.
Prefer a `Mapping` for independently accessed on-chain entries. Use an aggregate
only when its size is bounded and messages genuinely consume it as a whole.
Check the `ink` version's exact `Lazy` API and storage requirements before
introducing it; no `Lazy` pattern is currently present in the cited contracts.

## `#[ink(storage)]` layout and field order

An `#[ink(storage)]` struct is the contract's persistent state schema. The
staking contract keeps small counters/configuration beside mappings and explicit
lists:

```rust
// contracts/staking/src/lib.rs
#[ink(storage)]
pub struct Staking {
    admin: AccountId,
    stakes: Mapping<AccountId, StakeInfo>,
    total_staked: u128,
    reward_pool: u128,
    // More mappings and fields follow.
}
```

Do not reorder, remove, or change the type of deployed storage fields casually.
The generated layout and storage keys are part of the compatibility contract;
changes can make old values unreadable or decode them incorrectly. Group fields
for maintainability, initialize each field in every constructor, and review
generated layout diffs before deployment. Field order is not a substitute for
measuring gas: measure read/write behavior for the target runtime.

## Proxy and upgradeable storage

Proxy and implementation upgrades must preserve the layout expected by existing
instances. `contracts/proxy/src/lib.rs` keeps the implementation address,
upgrade state, and admin state in a stable proxy storage struct. Separately,
`contracts/traits/src/access_control.rs` reserves `__storage_gap` and documents
the rule: consume space from the gap for future fields instead of shifting
existing fields. Use a storage-layout comparison and an explicit migration plan
for incompatible schema changes; a gap does not make arbitrary type changes
safe.

## Patterns and anti-patterns

- **Per-account state:** use a mapping, as `Staking.stakes` and
  `IdentityRegistry.identities` do.
- **Composite keys:** use a tuple key when the record belongs to a pair, as
  governance votes use `(proposal_id, voter)`.
- **Enumeration:** keep a separate ID list only when a message or client needs
  enumeration. Keep it bounded or provide pagination; maintaining both list and
  mapping also means both must be updated atomically.
- **Unbounded `Vec` growth:** avoid a message that appends forever or loops over
  every entry. Its gas and proof-size requirements grow with historical state
  and can eventually exceed block limits.
- **Nested mappings:** they can make independent data addressable, but each
  layer adds key/encoding complexity, and `Mapping` cannot enumerate its
  children. Use a flat composite key where possible, or maintain a separately
  bounded index.
- **Large `Lazy<HashMap>` as a database:** avoid it for independently accessed
  records; use `Mapping`.

### Source references

- [`contracts/staking/src/lib.rs`](../../contracts/staking/src/lib.rs)
- [`contracts/governance/src/lib.rs`](../../contracts/governance/src/lib.rs)
- [`contracts/identity/lib.rs`](../../contracts/identity/lib.rs)
- [`contracts/proxy/src/lib.rs`](../../contracts/proxy/src/lib.rs)
- [`contracts/traits/src/access_control.rs`](../../contracts/traits/src/access_control.rs)
- [`docs/PROPERTY_REGISTRY_GAS_AND_EVENTS.md`](../../docs/PROPERTY_REGISTRY_GAS_AND_EVENTS.md)
