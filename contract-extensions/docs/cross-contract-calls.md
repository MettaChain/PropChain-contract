# Cross-contract call patterns in ink!

This guide describes cross-contract calls in PropChain contracts. Paths below are relative to the repository root.

## Enable `ink-as-dependency`

Add the dependency feature to a caller crate so ink! exposes that crate's contract metadata while it is compiled as a dependency:

```toml
[dependencies]
oracle = { path = "../oracle", package = "propchain-oracle", default-features = false, features = ["ink-as-dependency"] }
```

In `contracts/oracle/Cargo.toml`, the feature is intentionally declared:

```toml
[features]
default = ["std"]
std = ["ink/std"]
ink-as-dependency = []
```

At runtime, do not depend on Rust's `Self` contract type. Import the callee's generated contract reference from the same metadata-aware dependency.

## Trait-based interfaces

Keep reusable interfaces in `contracts/traits/src/`. For example, domain traits live in `contracts/traits/src/property.rs`, `dex.rs`, and `bridge.rs`. A caller can depend on the trait crate and use a contract address plus the ABI for dispatch:

```rust
use propchain_traits::{PropertyTokenBridge, BridgeStatus};

fn bridge_status(
    account: &ink::env::AccountId,
) -> Option<BridgeStatus> {
    let _ = account;
    None
}
```

Traits describe capabilities; implementation remains in the target contract. Prefer shared trait types and account IDs over duplicating domain structs. Cross-contract ABI changes should update both the trait and the implementing contract.

## Typed `CallBuilder` calls

The typed builder is the preferred API. It validates selectors and result types at compile time:

```rust
use ink::env::call::{build_call, ExecutionInput, Selector};

let result: bool = build_call::<ink::env::DefaultEnvironment>()
    .call_v1(target)
    .gas_limit(ink::env::Gas::from(100_000))
    .exec_input(
        ExecutionInput::new(Selector::new(ink::selector_bytes!("view_property")))
            .push_arg(&property_id),
    )
    .returns::<bool>()
    .try_invoke();
```

The repository's `CallBuilder`-style raw pattern is demonstrated in `contracts/multicall/src/lib.rs`:

```rust
let outcome = ink::env::call::build_call::<ink::env::DefaultEnvironment>()
    .call_v1(callee)
    .gas_limit(gas_limit)
    .transferred_value(transferred_value)
    .call_flags(ink::env::CallFlags::empty())
    .exec_input(
        ink::env::call::ExecutionInput::new(
            ink::env::call::Selector::new(selector),
        )
        .push_arg(&encoded_arguments),
    )
    .returns::<Vec<u8>>()
    .try_invoke();
```

Use `.call_v1()` for a contract address and `.call()` only when the ink! version/API requires the compatibility form.

## Calling view functions

View calls should not mutate the callee. Encode the message selector and arguments, and choose a finite gas limit:

```rust
use ink::env::call::{build_call, ExecutionInput, Selector};

fn owner_at(
    target: ink::env::AccountId,
    token_id: u128,
) -> Result<Option<ink::env::AccountId>, ink::env::Error> {
    build_call::<ink::env::DefaultEnvironment>()
        .call_v1(target)
        .gas_limit(50_000)
        .exec_input(
            ExecutionInput::new(Selector::new(ink::selector_bytes!("owner_of")))
                .push_arg(&token_id),
        )
        .returns::<Option<ink::env::AccountId>>()
        .try_invoke()
        .map_err(|error| ink::env::Error::CallRuntimeFailed(error))
}
```

Example PropChain relationships include the staking/factory/deployment patterns and oracle reads. Search `try_invoke()` in `contracts/oracle/src/lib.rs` and `contracts/multicall/src/lib.rs` for concrete implementations.

## Constructors and instantiation

Deploying a child contract uses a generated contract reference and `instantiate_v1`/`instantiate` on the `CallBuilder`:

```rust
let child_address = ink::env::call::build_create::<ink::env::DefaultEnvironment>()
    .code_hash(code_hash)
    .gas_limit(creation_gas)
    .endowment(0)
    .salt_bytes(salt)
    .instantiate_v1()
    .try_instantiate()
    .map_err(|_| FactoryError::DeploymentFailed)?
    .map_err(|_| FactoryError::DeploymentFailed)?;
```

The existing implementation in `contracts/factory/src/builder.rs` and `contracts/factory/src/lib.rs` shows constructor argument encoding and deployment-record updates. Pass constructor values with SCALE encoding in the exact order declared by the constructor.

## Error handling

Cross-contract calls may fail in three ways:

1. **Environment failure**: `try_invoke()` returns `LangError`, including `CouldNotReadInput`, `CouldNotReadOutput`, or another language-level failure.
2. **Contract failure**: the callee may revert with `ReturnErrorCode`.
3. **Application failure**: SCALE-decode the callee's own `Result<T, E>`.

```rust
match call.try_invoke() {
    Ok(Ok(value)) => Ok(value),
    Ok(Err(lang_error)) => Err(MyError::CrossContractLang(lang_error)),
    Err(env_error) => Err(MyError::CrossContractEnv(env_error)),
}
```

Ink! 5 exposes `ink::env::LangError` and `ink::env::ReturnErrorCode`. Propagate the call result rather than replacing all failures with a generic error unless the public API intentionally hides ABI details. Preserve revert behavior for assets and state: read, validate, call, then write local state only after the result is accepted.

For an application-level return:

```rust
let result = build_call::<ink::env::DefaultEnvironment>()
    // configured call
    .returns::<Result<u128, ContractError>>()
    .try_invoke();
```

Decode `Ok(Ok(...))`, `Ok(Err(...))`, and `Err(...)` independently.

## Gas and value

- Set explicit limits for external calls and retain enough gas for local validation and event emission.
- `.gas_limit(...)` reserves execution gas; omitted limits use the remaining environment amount.
- `.transferred_value(...)` forwards native value. Prefer zero unless the target contract deliberately accepts value.
- Propagate a bounded child call error so the parent call reverts atomically.
- Use `ink::env::CallFlags` to control reentrancy and delegate behavior only when documented by the target.

## Reentrancy and composability

Update checks and effects before external calls when practical. PropChain contracts use reentrancy guards from `contracts/traits/src/reentrancy_guard.rs` and `contracts/lib/src/lib.rs`. Use existing guard mixins rather than adding ad-hoc locks.

Examples:

- `contracts/factory/src/lib.rs`: deployment and constructor interaction.
- `contracts/multicall/src/lib.rs`: selector-based batch calls.
- `contracts/oracle/src/lib.rs`: external oracle reads.
- `contracts/crowdfunding/src/sequence.rs`: typed account and fee interactions.
