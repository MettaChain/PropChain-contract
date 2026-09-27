# Formal Verification (Kani)

`cargo kani` harnesses are the repository's written-down, machine-checked claims about
arithmetic that is hard to test exhaustively. They live here so that a green CI run can
be read against a list, and so that nothing is cited as proved that no harness actually
covers.

Everything is driven by `.github/workflows/formal-verification.yml`.

## Running them locally

```bash
cargo install --locked kani-verifier
cargo kani setup
```

then, per target:

```bash
cd contracts/lib       && cargo kani --harness <name>   # property/interface proofs
cd economic-models     && cargo kani --harness <name>   # value-movement maths
```

## What a green run does and does not mean

Read this before citing a proof.

- Harnesses under `contracts/lib/src/verification/` prove properties of **local stand-in
  types** declared in `invariants.rs` (`TokenLedger`, `AccessControl`, `OraclePrice`).
  They establish that the *reasoning* behind the modelled invariants is sound, not that
  the deployed contract upholds them. The module doc says so, and its stand-ins are
  marked `replace with your actual contract types`.
- Harnesses in `economic-models` prove properties of the formulas transcribed in
  `economic-models/src/lib.rs`. They cannot be pointed at the production functions
  directly: `borrow_rate::compute_borrow_rate` is private to `propchain-lending`, the
  fee rounding functions are `include!`d into an `#[ink::contract]` module, and
  `staking`'s `reward_snapshots` is a private module. `economic-models`' own `pins`
  tests tie each transcription back to the values the production unit tests assert, so a
  drift shows up in an ordinary `cargo test`.
- Kani is a bounded model checker. A proof covers the whole input space only because of
  the `kani::assume` bounds stated in the harness — where a harness excludes the
  saturating regime, the reason is written next to the assumption.

## `contracts/lib` — properties and interface

| Harness                                                  | Property                                                                                  |
| -------------------------------------------------------- | ----------------------------------------------------------------------------------------- |
| `balance_proofs::prove_balance_conservation`             | A transfer never changes total supply, whether it succeeds or fails.                      |
| `balance_proofs::prove_no_balance_underflow`             | A transfer larger than the sender's balance is rejected and leaves the balance untouched. |
| `access_control_proofs::prove_non_admin_always_rejected` | No non-admin role reaches an admin-only action.                                           |
| `access_control_proofs::prove_admin_always_accepted`     | An admin is never wrongly rejected.                                                       |
| `oracle_proofs::prove_stale_oracle_always_rejected`      | Price data older than the staleness bound is always refused.                              |
| `oracle_proofs::prove_fresh_oracle_always_accepted`      | Price data inside the bound is never refused.                                             |

## `economic-models` — value movement (issue #1209)

### Lending: borrow-rate curve

`rate_bps = 200 + 1_000 × min(utilisation_bps, 10_000) / 10_000`, transcribed from
`contracts/lending/src/borrow_rate.rs`.

| Harness                                                         | Property                                                        |
| --------------------------------------------------------------- | --------------------------------------------------------------- |
| `lending_rate_proofs::prove_rate_stays_within_documented_band`  | For every `u32` utilisation, `200 ≤ rate ≤ 1_200` basis points. |
| `lending_rate_proofs::prove_rate_is_monotonic_in_utilisation`   | A higher utilisation never yields a lower rate.                 |
| `lending_rate_proofs::prove_rate_saturates_at_full_utilisation` | Utilisation above 100% is clamped to the 100% rate.             |

### Fees: rounding and dust

Transcribed from `contracts/fees/src/rounding.rs`.

| Harness                                                     | Property                                                                                     |
| ----------------------------------------------------------- | -------------------------------------------------------------------------------------------- |
| `fee_rounding_proofs::prove_floor_never_exceeds_ceiling`    | `round_fee_down ≤ round_fee_up` for every input, zero denominator included.                  |
| `fee_rounding_proofs::prove_dust_is_at_most_one_unit`       | With a non-zero denominator and no saturation, rounding up costs the payer at most one unit. |
| `fee_rounding_proofs::prove_ceiling_covers_the_exact_fee`   | `round_fee_up × denominator ≥ amount × fee_bps`, so the rounded-up fee never under-collects. |
| `fee_rounding_proofs::prove_zero_denominator_yields_no_fee` | A zero denominator produces a zero fee instead of dividing by zero.                          |

### Staking: reward points and pending accrual

Transcribed from `contracts/staking/src/reward_snapshots.rs`.

| Harness                                                                     | Property                                                                         |
| --------------------------------------------------------------------------- | -------------------------------------------------------------------------------- |
| `staking_points_proofs::prove_zero_stake_scores_no_points`                  | A zero stake scores zero points for any reward amount.                           |
| `staking_points_proofs::prove_points_are_monotonic_in_rewards`              | More rewards never score fewer points at the same stake.                         |
| `staking_points_proofs::prove_pending_is_zero_once_the_snapshot_catches_up` | A snapshot that has caught up to the global accumulator reports nothing pending. |
| `staking_points_proofs::prove_pending_is_monotonic_in_the_accumulator`      | A larger global accrual never projects a smaller pending reward.                 |

## Known gap

The `economic-models` harnesses prove the transcribed formulas, not the shipped ones.
Closing that gap means giving the three production items a visibility their crates can
share — most likely by moving the arithmetic into a common crate that both the contracts
and `economic-models` depend on — and re-pointing the harnesses at it. Until then, treat
a green run as evidence about the formulas as written down here, plus the `pins` tests
as the link to the shipped code.
