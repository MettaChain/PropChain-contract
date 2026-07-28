# Insurance Deductible Penalty Drift — Resolution Record

> Related issue: #786  
> Files affected: `contracts/insurance/DYNAMIC_PREMIUM_CALCULATION.md`,
> `contracts/insurance/IMPLEMENTATION_SUMMARY.md`,
> `contracts/insurance/src/premium_tests.rs`

## Background

Issue #786 identified a discrepancy between the documented safety-feature
deductible reduction and the value actually applied by the
`calculate_deductible()` function in
`contracts/insurance/src/premium_engine.rs`.

| Location | Stated Value |
|----------|-------------|
| `DYNAMIC_PREMIUM_CALCULATION.md` (before fix) | **-5%** |
| `IMPLEMENTATION_SUMMARY.md` (before fix) | **-5%** |
| `premium_engine.rs` — `let reduction: u32 = 50;` | **50 basis points = 0.5%** |

## Root Cause

The documentation was written assuming a reduction of 500 basis points
(5%), but the implementation used `50` basis points (0.5%).  The integer
constant `50` was likely intended as a "50 bps" value when the system
settled on basis-point arithmetic, but the accompanying prose was never
updated to reflect this.

## Resolution

The authoritative source of truth for on-chain behaviour is the
**code**.  Tests already exercised the deductible path and would break
if the constant were changed without careful re-audit of all policies.
Therefore the decision was to **update the documentation** to match the
implemented value.

Changes made:

1. `DYNAMIC_PREMIUM_CALCULATION.md` — changed "Safety Feature Reduction:
   -5%" → **"-0.5% (50 basis points)"** in the Deductible Calculation
   section.

2. `IMPLEMENTATION_SUMMARY.md` — same correction in the Dynamic
   Deductible Calculation formula block.

3. `contracts/insurance/src/premium_tests.rs` — added
   `test_deductible_safety_feature_reduction_is_50_bps` which asserts
   the reduction is exactly 50 basis points (0.5%), pinning the value
   against future accidental changes.

## Canonical Values (as of this fix)

| Parameter | Value | Unit |
|-----------|-------|------|
| Base deductible | 500 | basis points (5%) |
| Safety feature reduction | **50** | basis points (0.5%) |
| Very-high-risk adjustment (score 0–20) | 200 | basis points (+2%) → total 7.5% with base when safety applies |
| High-risk adjustment (score 21–40) | 150 | basis points (+1.5%) |
| Medium-risk adjustment (score 41–60) | 100 | basis points (+1%) |
| Low-risk adjustment (score 61–80) | 75 | basis points (+0.75%) |
| Very-low-risk adjustment (score 81–100) | 50 | basis points (+0.5%) |

> All values are defined in
> `contracts/insurance/src/premium_engine.rs::calculate_deductible()`.
> Any future change to these constants **must** be accompanied by an
> update to this file, `DYNAMIC_PREMIUM_CALCULATION.md`, and the
> corresponding pinning test.
