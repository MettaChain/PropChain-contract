# RentalYield upgrade safety, overflow and static analysis

> Status: this document records the overflow-guard review of `RentalYield` and the
> checklist that keeps it safe. It accompanies the tests in
> `contracts/rental-yield/tests/overflow_rental_yield.rs` and resolves the
> checklist on issue #1372.

## Issue #1372: Guard RentalYield against integer overflow

### Review

Every arithmetic operation on amounts, shares and the per-share accumulator is
explicitly guarded and maps a `None` result to `Error::ArithmeticOverflow`, so a
hostile or corrupted input fails the message instead of trapping or wrapping:

| Arithmetic site | Guard |
| :--- | :--- |
| `set_shares` — released shares, new allocation, holder debt | `checked_sub`, `checked_add`, `checked_mul`, `checked_div` |
| `deposit_rent` — `amount * PRECISION`, per-share delta, distributed amount, remainder, accumulator, counters | `checked_mul`, `checked_div`, `checked_add`, `checked_sub` |
| `settle` — gross per-holder product, accrued delta, pending balance | `checked_mul`, `checked_div`, `checked_sub`, `checked_add` |
| `claim` — lifetime claimed counters | `checked_add` |
| `claimable` view — read-only estimate only | `saturating_mul`, `saturating_div`, `saturating_sub`, `saturating_add` (clamping is the documented view behaviour; it never writes state) |

Division sites are safe by construction: `deposit_rent` rejects a registration
with zero allocated shares (`Error::NoSharesAllocated`) before dividing,
`PRECISION` is a constant, and every other divisor is at least one.

### Checklist for future changes to `RentalYield`

- Use `checked_add`/`checked_sub`/`checked_mul`/`checked_div` for every
  arithmetic operation on amounts, counters and ids and map `None` to
  `Error::ArithmeticOverflow`.
- Use `saturating_*` only in read-only views, where clamping is documented
  behaviour.
- Keep `overflow-checks = true` in `[profile.release]` so an unchecked
  operation traps instead of wrapping.
- Add a test for each new arithmetic site at `u128::MAX` boundaries
  (`contracts/rental-yield/tests/overflow_rental_yield.rs`).
- A failed operation must leave property state, the global counters and the
  emitters untouched: assert the snapshot equality as the existing overflow
  tests do.

### Known boundary condition (documented, safe-by-default)

The accumulator is a single `u128` scaled by `PRECISION`. Once
`shares * reward_per_share` no longer fits in a single `u128` product, the
checked guards reject settlement (`Error::ArithmeticOverflow`) rather than
computing a wrong value. The account stays reachable through
`claimable` (saturating) but cannot claim further — a fail-locked state, never
a silently-wrong payout. Reducing this requires migrating `reward_per_share`
to a wider accumulator (512-bit or splitting high/low), which is a
storage-breaking upgrade and must follow `docs/proxy_upgrade_governance.md`.

### Static analysis

- `cargo clippy --all-targets --all-features -- -D warnings` (config in
  `clippy.toml`)
- `cargo audit` (config in `audit.toml`) and `cargo deny check`
  (config in `deny.toml`)
- Review of `unsafe`, `unwrap`/`expect` and unchecked arithmetic via
  `clippy::unwrap_used` and `clippy::arithmetic_side_effects`.

These run in the existing CI workflow for the crate that contains
`RentalYield` (`contracts/rental-yield`, workspace member of the root
`Cargo.toml`); the build fails on any warning.
