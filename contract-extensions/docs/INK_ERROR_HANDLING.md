# Error Handling in ink! Contracts

PropChain contract messages commonly return typed `Result` values so callers
can distinguish authorization, validation, state, and arithmetic failures
without relying on panics or parsing strings.

## Contract error enums and `include!`

Staking keeps its enum in `contracts/staking/src/errors.rs` and includes it
inside the contract module:

```rust
// contracts/staking/src/lib.rs
include!("errors.rs");
```

The enum derives SCALE encoding/decoding and equality/debug traits required by
the contract and its tests:

```rust
// contracts/staking/src/errors.rs
#[derive(Debug, PartialEq, Eq, scale::Encode, scale::Decode)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub enum Error {
    Unauthorized,
    InsufficientAmount,
    StakeNotFound,
    LockActive,
}
```

Use focused variants rather than a catch-all string. The complete staking enum
also implements `Display` and the shared `ContractError` trait for error codes,
categories, and descriptions.

## Returning `Result` and propagating errors

Use `Result<T, Error>` on messages that can fail. The `?` operator returns an
error immediately while preserving the original variant:

```rust
// contracts/staking/src/lib.rs (stake)
if amount == 0 {
    return Err(Error::ZeroAmount);
}
if amount < self.min_stake {
    return Err(Error::InsufficientAmount);
}
```

The surrounding `#[ink(message)]` returns `Result<(), Error>`.

For fallible mapping reads, convert a missing value to a domain error before
continuing:

```rust
// contracts/staking/src/lib.rs
let stake = self.stakes.get(caller).ok_or(Error::StakeNotFound)?;
```

If a shared primitive has a different error type, define an explicit
`From<SharedError> for ContractError` mapping. Staking maps the shared
reentrancy error to its own stable contract variant:

```rust
// contracts/staking/src/lib.rs
impl From<propchain_traits::ReentrancyError> for Error {
    fn from(_: propchain_traits::ReentrancyError) -> Self {
        Error::ReentrantCall
    }
}
```

Use `?` after this conversion is available. If multiple lower-level variants
have different meaning to callers, preserve that distinction in the contract
error enum rather than mapping all of them to an ambiguous error.

## Useful error categories

| Category | Examples |
|---|---|
| Access control | `Unauthorized`, missing role, unauthorized signer |
| Validation | `ZeroAmount`, invalid threshold/configuration, malformed input |
| State | `StakeNotFound`, `ProposalClosed`, already voted, active lock |
| Arithmetic | Overflow/underflow; prefer checked arithmetic and a specific error |

Keep messages descriptive and actionable. Stable enum variants and error codes
are the contract interface; descriptions should tell users what condition
failed and, where useful, how to correct it. Avoid building dynamic strings in
on-chain message paths.

## Cross-contract calls and `LangError`

Low-level `try_invoke` calls distinguish a language-level contract failure
from an environment/dispatch failure. In `contracts/proxy/src/lib.rs`,
`call_implementation` matches `Ok(Ok(bytes))` for success, `Ok(Err(_lang_err))`
for a callee language error, and `Err(_env_err)` for an environment error. The
example intentionally maps both failures to `ProxyError::ImplementationCallFailed`;
contracts that need more diagnostics can preserve separate domain variants.

```rust
// contracts/proxy/src/lib.rs
match call_result {
    Ok(Ok(bytes)) => Ok(bytes),
    Ok(Err(_lang_err)) => Err(ProxyError::ImplementationCallFailed),
    Err(_env_err) => Err(ProxyError::ImplementationCallFailed),
}
```

Do not assume `?` on a cross-contract invocation has the same error type as a
local message. Inspect the result shape for the `ink` version in use and map
both the inner `LangError` and outer environment error deliberately. Never
silently convert a failed call into a success-shaped default.

## Testing errors

For each meaningful error path, test that the message returns the expected
variant and leaves state unchanged when validation fails. Include unauthorized
caller tests, missing-record tests, invalid-input tests, and arithmetic
boundaries. Also test shared-error conversion and cross-contract call failure
mapping where those paths are part of the contract.

### Source references

- [`contracts/staking/src/lib.rs`](../../contracts/staking/src/lib.rs)
- [`contracts/staking/src/errors.rs`](../../contracts/staking/src/errors.rs)
- [`contracts/governance/src/errors.rs`](../../contracts/governance/src/errors.rs)
- [`contracts/traits/src/errors.rs`](../../contracts/traits/src/errors.rs)
- [`contracts/proxy/src/lib.rs`](../../contracts/proxy/src/lib.rs)
- [`docs/PROPERTY_REGISTRY_ERRORS_AND_VALIDATION.md`](../../docs/PROPERTY_REGISTRY_ERRORS_AND_VALIDATION.md)
