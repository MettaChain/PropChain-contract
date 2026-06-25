# Escrow Template System

## Overview
Reusable escrow templates let integrators spin up standard escrow configurations
without repeating boilerplate parameters.

## Template struct

```rust
#[derive(Debug, Clone, scale::Encode, scale::Decode, ink::storage::traits::StorageLayout)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub struct EscrowTemplate {
    pub template_id: u64,
    pub name: String,
    pub release_time_lock_secs: Option<u64>,
    pub required_signatures: u8,
    pub jurisdiction: Jurisdiction,
    pub created_by: AccountId,
}
```

## API

| Function | Description |
|---|---|
| `create_template(name, release_lock, sigs, jurisdiction)` | Admin registers a new template |
| `instantiate_from_template(template_id, buyer, seller, amount)` | Creates an `EscrowData` pre-filled from the template |
| `list_templates()` | Returns all registered templates |
| `get_template(id)` | Returns a single template |

## Common templates

| Name | Lock | Sigs | Use case |
|---|---|---|---|
| `residential_sale` | 7 days | 2 | Standard residential property purchase |
| `commercial_lease` | 30 days | 3 | Commercial lease deposit |
| `instant_release` | None | 1 | Low-value, trusted-party transfers |

## Storage key
```rust
EscrowTemplate(template_id: u64)  // persistent storage
TemplateCount                      // instance storage counter
```