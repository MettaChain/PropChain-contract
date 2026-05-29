# PropChain Contract Modularization Guide

This document describes the modular architecture pattern used in PropChain smart
contracts and provides guidelines for maintaining and extending the codebase.

## Architecture Overview

PropChain uses **ink! 5.0** for smart contract development on Substrate-based
chains. Due to constraints imposed by the ink! procedural macro system, modules
are structured as follows:

```
contracts/<contract>/
├── Cargo.toml
└── src/
    ├── lib.rs          # Contract module, storage struct, impl + events
    ├── types.rs        # Data structures and enums (include!-d)
    ├── errors.rs       # Error enum and ContractError impl (include!-d)
    └── tests.rs        # Unit tests (include!-d)
```

### Why `include!` Instead of `mod`?

The `#[ink::contract]` proc-macro expects to process a **single `mod`** block
containing the entire contract definition. Standard Rust modules (`mod foo;`)
create separate compilation units that the ink! macro cannot see into. Using
`include!("file.rs")` performs a **textual paste** at compile time, keeping
everything visible to the ink! macro while splitting code across files.

### What Can Be Extracted

| Content Type          | Can Extract? | Notes                                             |
| --------------------- | ------------ | ------------------------------------------------- |
| Data structs/enums    | ✅ Yes       | No ink! attributes required                       |
| Error types           | ✅ Yes       | Standard Rust types with `ContractError` impls    |
| Unit tests            | ✅ Yes       | `#[ink::test]` is a standalone attribute macro    |
| `#[ink(event)]`       | ❌ No        | Must be inside `#[ink::contract]` module directly |
| `#[ink(storage)]`     | ❌ No        | Must be in `lib.rs` for ink! processing           |
| `#[ink(message)]`     | ❌ No        | Must be in `lib.rs` `impl` block                  |
| `#[ink(constructor)]` | ❌ No        | Must be in `lib.rs` `impl` block                  |

### Shared Traits Library

The `contracts/traits/` crate contains domain-specific modules that define
shared types and trait interfaces used across contracts:

```
contracts/traits/src/
├── lib.rs            # Module declarations + re-exports
├── oracle.rs         # Oracle types, errors, traits
├── bridge.rs         # Cross-chain bridge types and traits
├── property.rs       # Property metadata, registry, escrow traits
├── dex.rs            # DEX/trading types
├── fee.rs            # Dynamic fee types and traits
├── compliance.rs     # Compliance types and traits
├── access_control.rs # Role-based access control
├── constants.rs      # Shared constants
├── errors.rs         # Shared error infrastructure
├── i18n.rs           # Internationalization support
└── monitoring.rs     # Monitoring types
```

All types are re-exported from `lib.rs` for backward compatibility:

```rust
pub use oracle::*;
pub use bridge::*;
pub use property::*;
// etc.
```

## Guidelines for New Contracts

### 1. Start with Clear Separation

When creating a new contract, immediately separate concerns:

```rust
// src/lib.rs
#![cfg_attr(not(feature = "std"), no_std)]
use ink::storage::Mapping;
use propchain_traits::*;

#[ink::contract]
mod my_contract {
    use super::*;

    // Types extracted to types.rs
    include!("types.rs");

    // Errors extracted to errors.rs
    include!("errors.rs");

    // Events MUST stay inline (ink! proc-macro requirement)
    #[ink(event)]
    pub struct MyEvent {
        #[ink(topic)]
        pub id: u64,
    }

    #[ink(storage)]
    pub struct MyContract { /* ... */ }

    impl MyContract {
        #[ink(constructor)]
        pub fn new() -> Self { /* ... */ }

        #[ink(message)]
        pub fn do_thing(&mut self) -> Result<(), MyError> { /* ... */ }
    }

    // Tests extracted to tests.rs
    include!("tests.rs");
}
```

### 2. When a File Gets Too Large

A contract file should be split when it exceeds **~500 lines** of types/errors
or **~200 lines** of tests. Signs that extraction is needed:

- More than 10 struct/enum definitions
- More than 15 error variants
- More than 10 test functions
- Multiple unrelated domain sections (e.g., bridge + governance + marketplace)

### 3. Adding Types to the Traits Library

When a new shared type is needed across multiple contracts:

1. Identify the domain (oracle, bridge, property, dex, fee, compliance)
2. Add the type to the appropriate module in `contracts/traits/src/`
3. It will be automatically re-exported via `pub use module::*` in `lib.rs`
4. **Do not** add contract-specific types to the traits library

### 4. Event Organization

Events must stay in `lib.rs` but should be organized with clear section headers:

```rust
// --- Domain A Events ---
#[ink(event)]
pub struct DomainACreated { /* ... */ }

// --- Domain B Events ---
#[ink(event)]
pub struct DomainBUpdated { /* ... */ }
```

### 5. Include File Rules

Files included via `include!()`:

- **Must not** use `//!` (module-level) doc comments — use `//` instead
- **Must not** contain `#[ink(event)]`, `#[ink(storage)]`, `#[ink(message)]`, or
  `#[ink(constructor)]` attributes
- **Should** use `///` doc comments on individual items
- **Should** have a single-line `//` comment at the top describing the file
- **Must** be in the same `src/` directory as `lib.rs`

## Line Count Targets

| Component               | Target Max Lines | Action If Exceeded                    |
| ----------------------- | ---------------- | ------------------------------------- |
| `lib.rs` (total)        | ~2000            | Extract more types/helpers            |
| Types section           | ~300             | Split into domain-specific type files |
| Events section          | ~250             | Keep inline but organize with headers |
| Tests                   | ~500             | Extract to `tests.rs`                 |
| Error enum + impls      | ~200             | Extract to `errors.rs`                |
| Single `#[ink(message)]` | ~50             | Refactor into helper functions        |

## Verification Checklist

After any modularization change:

```bash
# 1. Compile check
cargo check --workspace

# 2. Full test suite
cargo test --workspace

# 3. Lint check
cargo clippy --workspace -- -D warnings

# 4. Format check
cargo fmt --all -- --check
```
