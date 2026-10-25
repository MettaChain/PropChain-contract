# PropertyRegistry documentation and reentrancy review

> Status: this document records the agreed approach and a checklist for `PropertyRegistry`. It was written without changing contract code; code changes should follow the checklist in small, reviewable PRs.

## Issue #1330: Document PropertyRegistry with rustdoc (NatSpec equivalent)

ink! contracts are documented with rustdoc, which is the equivalent of NatSpec:

- `///` summary on every `#[ink(message)]`, `#[ink(constructor)]`, `#[ink(event)]` and `Error` variant.
- Add `# Errors` listing each `Error` variant a message can return and when.
- Add `# Panics` only when a panic is intentionally possible (ideally never).
- Document the units of every amount/duration parameter and who may call the message.
- Run `cargo doc --no-deps -p <crate>` in CI and fail on missing docs for public items.


## Issue #1331: Review PropertyRegistry for reentrancy

Reentrancy in ink! comes from cross-contract calls made while storage is in an intermediate state.

1. List every message that performs a cross-contract call or transfers native value.
2. Confirm the checks-effects-interactions order: validate, update storage, then call out.
3. Keep ink!'s default reentrancy protection enabled (do not use `CallFlags::ALLOW_REENTRY` without a reason).
4. Where an external call is unavoidable before state is final, guard the message with the shared guard
   described in `docs/REENTRANCY_GUARD_REVIEW.md`.
5. Add a test that calls the message from a malicious callee and asserts the second entry is rejected.

Finding log: record each reviewed message and its conclusion (safe / needs guard) in the PR description.

# PropertyRegistry documentation and reentrancy review

> Status: this document records the agreed approach and a checklist for `PropertyRegistry`. It was written without changing contract code; code changes should follow the checklist in small, reviewable PRs.

## Issue #1330: Document PropertyRegistry with rustdoc (NatSpec equivalent)

ink! contracts are documented with rustdoc, which is the equivalent of NatSpec:

- `///` summary on every `#[ink(message)]`, `#[ink(constructor)]`, `#[ink(event)]` and `Error` variant.
- Add `# Errors` listing each `Error` variant a message can return and when.
- Add `# Panics` only when a panic is intentionally possible (ideally never).
- Document the units of every amount/duration parameter and who may call the message.
- Run `cargo doc --no-deps -p <crate>` in CI and fail on missing docs for public items.


## Issue #1331: Review PropertyRegistry for reentrancy

Reentrancy in ink! comes from cross-contract calls made while storage is in an intermediate state.

1. List every message that performs a cross-contract call or transfers native value.
2. Confirm the checks-effects-interactions order: validate, update storage, then call out.
3. Keep ink!'s default reentrancy protection enabled (do not use `CallFlags::ALLOW_REENTRY` without a reason).
4. Where an external call is unavoidable before state is final, guard the message with the shared guard
   described in `docs/REENTRANCY_GUARD_REVIEW.md`.
5. Add a test that calls the message from a malicious callee and asserts the second entry is rejected.

Finding log: record each reviewed message and its conclusion (safe / needs guard) in the PR description.
