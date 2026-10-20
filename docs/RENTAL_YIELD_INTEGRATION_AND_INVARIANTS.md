# RentalYield: integration and invariant tests

The tests live next to the contract:

- [`contracts/rental-yield/tests/integration_rental_yield.rs`](../contracts/rental-yield/tests/integration_rental_yield.rs)
- [`contracts/rental-yield/tests/invariants_rental_yield.rs`](../contracts/rental-yield/tests/invariants_rental_yield.rs)

Run both with:

```bash
cargo test -p rental-yield
```

## Issue #1369 — Integration tests for RentalYield flows

The integration suite drives the public messages exactly as a caller would,
using `ink::env::test` to switch accounts and attach value.

| Test | Flow covered |
| --- | --- |
| `integration_proportional_yield_flow` | register → assign 60/40 shares → deposit → both holders claim their exact share |
| `integration_rounding_never_over_distributes` | a 100-unit deposit over 3 equal shares yields 33 each and records 1 unit of remainder |
| `integration_access_control_rejects_non_operators` | a non-manager cannot register, assign shares, deposit, or pause |
| `integration_manager_role_can_operate_and_is_revocable` | an admin grants `Manager`; the manager operates; revocation removes the permission |
| `integration_pause_blocks_and_resume_restores_flows` | pause rejects deposits and claims without changing balances; unpause restores both |
| `integration_share_change_does_not_earn_retroactively` | changing shares after a deposit keeps the old accrual and applies new shares only to future rent |
| `integration_claim_is_idempotent_after_balance_is_exhausted` | a second claim with no accrual returns `NoYield` instead of paying twice |
| `integration_properties_are_isolated` | deposits for one property never credit holders of another property |

## Issue #1371 — Invariant tests for RentalYield

The invariant suite asserts these properties after every mixed sequence of
deposits, share changes and claims:

1. `allocated_shares <= total_shares` for every property.
2. `claimed <= deposited` for every property.
3. The sum of every holder's `claimable` is at most
   `deposited - claimed`; outstanding liabilities are always backed by
   unclaimed deposits.
4. Lifetime `total_claimed <= total_deposited`.
5. No single holder can claim more than the remaining pool.
6. `reward_per_share` is monotonically non-decreasing across deposits.
7. A failed call (unauthorized caller, zero-value deposit, or claim with no
   accrual) leaves storage and totals unchanged.
8. While paused, deposits and claims are rejected and no accounting state
   moves.
9. Share assignments cannot exceed the property supply, and reducing one
   holder's shares releases capacity for another holder.

| Test | Invariants asserted |
| --- | --- |
| `invariant_accounting_holds_after_mixed_flows` | 1–5 |
| `invariant_reward_per_share_is_monotonic` | 1–6 |
| `invariant_failed_operations_do_not_mutate_state` | 3, 4, 7 |
| `invariant_paused_blocks_all_value_movements` | 3–5, 8 |
| `invariant_share_assignments_cannot_exceed_supply` | 1, 3–5, 9 |
| `invariant_zero_value_deposit_is_rejected_without_state_change` | 4, 7 |

The accounting is rounded down per share. Deposits that cannot be attributed
to a whole unit are kept in the property's `remainder` bucket instead of being
credited, which is what makes invariant 3 hold for every share count.
