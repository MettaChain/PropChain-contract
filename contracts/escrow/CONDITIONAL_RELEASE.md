# Escrow Conditional Release

## Overview
Funds are released automatically when an oracle confirms that a predefined
condition has been met (e.g. property title transferred, inspection passed).

## Condition struct

```rust
#[derive(Debug, Clone, scale::Encode, scale::Decode, ink::storage::traits::StorageLayout)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub struct ReleaseCondition {
    pub escrow_id: u64,
    pub oracle: AccountId,
    pub condition_key: String,   // e.g. "title_transferred"
    pub expected_value: String,  // e.g. "true"
    pub fulfilled: bool,
    pub fulfilled_at: Option<u64>,
}
```

## API

| Function | Caller | Description |
|---|---|---|
| `set_release_condition(escrow_id, oracle, key, value)` | admin | Attaches a condition to an escrow |
| `fulfill_condition(escrow_id, actual_value)` | registered oracle | Marks condition met if `actual_value == expected_value` |
| `conditional_release(escrow_id)` | anyone | Releases funds if all conditions are fulfilled |

## Flow

```
create_escrow → set_release_condition → oracle submits data
    → fulfill_condition → conditional_release → funds sent to seller
```

## Constraints
- Only `authorized_oracles` may call `fulfill_condition`.
- `conditional_release` panics if any condition is unfulfilled.
- Conditions are immutable once set; add new ones via `set_release_condition`.