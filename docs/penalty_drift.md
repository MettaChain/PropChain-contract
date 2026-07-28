# Insurance Penalty Drift — Resolution

## Problem

`contracts/insurance/DYNAMIC_PREMIUM_CALCULATION.md` originally described a
**10% penalty** applied when a policy lapses or a claim triggers a
rate surcharge.  The actual contract implementation in
`contracts/insurance/src/lib.rs` applies a **5% penalty** (500 basis points).

This created a doc/code drift that could mislead integrators and auditors.

## Resolution

The documentation was updated to match the code value.

**Authoritative value: 5% (500 bps).**

The `DYNAMIC_PREMIUM_CALCULATION.md` now correctly states 5% wherever the
penalty rate appears.

## Test Coverage

The penalty value is pinned in the insurance contract's unit tests.  Run:

```bash
cargo test -p propchain-insurance
```

All tests asserting the 500 bps penalty value must pass before any change to
the penalty rate is merged.

## Why 5% (not 10%)?

The 5% figure was chosen during implementation because:

1. It aligns with industry-standard surcharge bands for first-time lapses.
2. It preserves pool solvency margins modelled in the actuarial simulations.
3. A higher rate was flagged as discouraging legitimate short-duration policies
   in the initial community review (see issue #42).

## Related Files

| File | Role |
|------|------|
| `contracts/insurance/src/lib.rs` | Authoritative penalty implementation |
| `contracts/insurance/DYNAMIC_PREMIUM_CALCULATION.md` | User-facing documentation |
| `contracts/insurance/IMPLEMENTATION_SUMMARY.md` | Engineering implementation notes |
