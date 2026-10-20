# RentalYield: access control audit and shared primitives

`RentalYield` lives in [`contracts/rental-yield`](../contracts/rental-yield/src/lib.rs).
It distributes rent across the holders of a tokenized property using a
per-share reward accumulator, so a deposit costs the same regardless of how
many owners a property has.

## Issue #1368 — Refactor with OpenZeppelin primitives

OpenZeppelin is a Solidity library and cannot be imported into an ink!
contract. `RentalYield` uses the workspace's audited ink! equivalents instead
of hand-rolled copies:

| Concern | OpenZeppelin equivalent | Used here |
| --- | --- | --- |
| Role-based access control | `AccessControl` | `propchain_traits::AccessControl`, bootstrapped in the constructor |
| Reentrancy protection | `ReentrancyGuard` / `nonReentrant` | `propchain_traits::ReentrancyGuard` plus the `non_reentrant!` macro around `claim` |
| Pause switch | `Pausable` / `whenNotPaused` | The contract `paused` flag checked by `ensure_not_paused` |
| Safe math | `SafeMath` | Explicit `checked_add` / `checked_mul` / `checked_sub` on every accounting path |

The roles used by the contract are:

- `SuperAdmin` / `Admin` — granted to the deployer and able to manage roles.
- `Manager` — may register properties, assign shares and deposit rent.
- `PauseGuardian` — may pause and resume the contract.

Role inheritance means an admin also passes the `Manager` and `PauseGuardian`
checks, matching the rest of the workspace.

## Issue #1370 — Access control audit

| Message | Intended caller | Enforced by | Negative test |
| --- | --- | --- | --- |
| `new` | deployer | constructor caller becomes admin | — |
| `grant_role` / `revoke_role` | role admin (`Admin` by default) | `AccessControl::grant_role` / `revoke_role` | `integration_access_control_rejects_non_operators` |
| `register_property` | `Manager` | `ensure_operator` | `integration_access_control_rejects_non_operators` |
| `set_shares` | `Manager` | `ensure_operator` | `integration_access_control_rejects_non_operators` |
| `deposit_rent` | `Manager` | `ensure_operator` + `ensure_not_paused` | `integration_access_control_rejects_non_operators` |
| `pause` / `unpause` | `PauseGuardian` | `ensure_guardian` | `integration_access_control_rejects_non_operators` |
| `claim` | the holder whose balance is claimed | the storage key is always `(property_id, caller)` | `integration_claim_is_idempotent_after_balance_is_exhausted` |
| all `#[ink(message)]` views | anyone | read-only | — |

Audit findings and decisions:

- There is no admin-transfer message, so the admin cannot be changed outside
  the audited `AccessControl` role flow.
- `claim` never accepts a holder argument: yield can only be paid to
  `self.env().caller()`, which removes the confused-deputy class of bug.
- Every state-changing message that can move value checks the pause flag.
- `set_shares` settles the holder's accrual before applying the new share
  count, so a share change cannot retroactively alter already-accrued yield.
- Role grants and revocations are recorded by `AccessControl`'s audit log and
  emit the shared access-control events.

## Running the checks

```bash
cargo test -p rental-yield
cargo clippy -p rental-yield --all-targets -- -D warnings
```
