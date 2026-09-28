#![allow(unexpected_cfgs)] // `kani` is set by the Kani compiler, not by Cargo

//! Models of PropChain's value-movement arithmetic, and the Kani proofs that
//! pin it (issue #1209).
//!
//! # Why this crate exists
//!
//! Three pieces of arithmetic move real value and had no formal backstop:
//!
//! * the lending borrow-rate curve (`contracts/lending/src/borrow_rate.rs`);
//! * fee rounding and the dust it leaves (`contracts/fees/src/rounding.rs`);
//! * staking reward points and pending accrual
//!   (`contracts/staking/src/reward_snapshots.rs`).
//!
//! None of them can be called from a proof harness as-is. `compute_borrow_rate`
//! is a `pub fn` inside a **private** module (`contracts/lending/src/lib.rs:12`)
//! exposed only through `borrow_rate(pool_id)`, which needs a pool and returns a
//! `Result`; `round_fee_up` / `round_fee_down` / `fee_dust` are `include!`d into
//! the `#[ink::contract] mod propchain_fees`, so they are items of a generated
//! contract module rather than crate exports; `points_for` and
//! `RewardSnapshot::pending` sit behind a private `mod reward_snapshots`
//! (`contracts/staking/src/lib.rs:10`).
//!
//! So each formula is transcribed here, in a crate with **no dependencies at
//! all** — which is also what makes it a usable Kani target: `cargo kani` has to
//! compile the whole dependency graph, and the existing harnesses live inside an
//! ink! contract crate. The models are not a second implementation: nothing
//! calls them at runtime, the [`pins`] tests tie them to the exact values the
//! production unit tests assert, and the [`proofs`] module is the only consumer.
//!
//! # Scope — read this before citing a green run
//!
//! A proof discharged here is evidence about *these models*. It shows the
//! reasoning in the transcribed formulas is sound; it does not show that a
//! deployed contract upholds them, because the deployed contracts run different
//! code. Binding the harnesses to the real functions (widening the three items
//! above, or moving the arithmetic into a shared crate both sides depend on) is
//! tracked as follow-up in `docs/formal_verification.md`.
//!
//! # Where the proofs run
//!
//! The `proofs` module is `#[cfg(kani)]`-gated and never ships. It is executed
//! by `.github/workflows/formal-verification.yml`.

// ─────────────────────────────────────────────────────────────────────────────
// 1. LENDING — linear borrow-rate curve
//    Mirrors `contracts/lending/src/borrow_rate.rs`.
// ─────────────────────────────────────────────────────────────────────────────

/// Base borrow rate in basis points (2%), charged at zero utilisation.
pub const BASE_RATE_BPS: u32 = 200;
/// Additional rate at full utilisation, in basis points (+10%).
pub const SLOPE_BPS: u32 = 1_000;
/// Utilisation is expressed in basis points, so 100% is `10_000`.
pub const FULL_UTILISATION_BPS: u32 = 10_000;

/// Borrow rate in basis points for a utilisation of `utilisation_bps`.
///
/// `rate_bps = BASE_RATE_BPS + SLOPE_BPS * min(utilisation_bps, 10_000) / 10_000`
pub fn compute_borrow_rate(utilisation_bps: u32) -> u32 {
    let utilisation_bps = utilisation_bps.min(FULL_UTILISATION_BPS);
    BASE_RATE_BPS + (SLOPE_BPS as u64 * utilisation_bps as u64 / 10_000) as u32
}

// ─────────────────────────────────────────────────────────────────────────────
// 2. FEES — rounding a proportional fee, and the dust it leaves
//    Mirrors `contracts/fees/src/rounding.rs`.
// ─────────────────────────────────────────────────────────────────────────────

/// Fee rounded up: the smallest whole-unit fee that covers
/// `amount * fee_bps / denominator`.
pub fn round_fee_up(amount: u128, fee_bps: u128, denominator: u128) -> u128 {
    if denominator == 0 {
        return 0;
    }
    let n = amount.saturating_mul(fee_bps);
    n.saturating_add(denominator.saturating_sub(1))
        .saturating_div(denominator)
}

/// Fee rounded down: the largest whole-unit fee that never exceeds
/// `amount * fee_bps / denominator`.
pub fn round_fee_down(amount: u128, fee_bps: u128, denominator: u128) -> u128 {
    if denominator == 0 {
        return 0;
    }
    amount.saturating_mul(fee_bps).saturating_div(denominator)
}

/// The dust between the two roundings: what rounding up costs a payer beyond
/// rounding down. With a non-zero denominator this is at most one unit.
pub fn fee_dust(amount: u128, fee_bps: u128, denominator: u128) -> u128 {
    round_fee_up(amount, fee_bps, denominator).saturating_sub(round_fee_down(
        amount,
        fee_bps,
        denominator,
    ))
}

// ─────────────────────────────────────────────────────────────────────────────
// 3. STAKING — reward points and pending-accrual arithmetic
//    Mirrors `contracts/staking/src/reward_snapshots.rs`.
// ─────────────────────────────────────────────────────────────────────────────

/// Fixed-point scale for reward points (rewards earned per unit of stake).
pub const REWARD_POINTS_DENOM: u128 = 1_000_000_000;

/// Reward points for `rewards` earned against a stake of `staked`.
///
/// A zero stake has no per-token value to record, so it scores 0 rather than
/// dividing by zero.
pub fn reward_points(rewards: u128, staked: u128) -> u128 {
    if staked == 0 {
        return 0;
    }
    rewards
        .saturating_mul(REWARD_POINTS_DENOM)
        .saturating_div(staked)
}

/// Rewards still to come from a global accumulator that has moved on to
/// `global_accrued` since a snapshot was taken at `accrued_per_token`.
pub fn pending_rewards(accrued_per_token: u128, global_accrued: u128, staked: u128) -> u128 {
    global_accrued
        .saturating_sub(accrued_per_token)
        .saturating_mul(staked)
        / REWARD_POINTS_DENOM
}

// ─────────────────────────────────────────────────────────────────────────────
// Pins: the models against the values the production unit tests assert.
// ─────────────────────────────────────────────────────────────────────────────

/// Concrete-value tests that keep each model honest.
///
/// These do not call the production functions (see the crate docs for why they
/// cannot). They restate, at the same inputs, the expectations the production
/// unit tests already encode — `contracts/lending/src/borrow_rate.rs`,
/// `contracts/fees/src/rounding.rs` and
/// `contracts/staking/src/reward_snapshots.rs`. If a model drifts from the
/// formula it transcribes, these fail.
#[cfg(test)]
mod pins {
    use super::*;

    #[test]
    fn lending_model_matches_the_production_curve() {
        assert_eq!(compute_borrow_rate(0), BASE_RATE_BPS);
        assert_eq!(
            compute_borrow_rate(FULL_UTILISATION_BPS),
            BASE_RATE_BPS + SLOPE_BPS
        );
        // Above 100% utilisation clamps to the same rate as exactly 100%.
        assert_eq!(
            compute_borrow_rate(50_000),
            compute_borrow_rate(FULL_UTILISATION_BPS)
        );
        // Half utilisation is half the slope.
        assert_eq!(compute_borrow_rate(5_000), BASE_RATE_BPS + SLOPE_BPS / 2);
    }

    #[test]
    fn fee_model_matches_the_production_rounding() {
        // 0.3% of 1001 = 3.003 → ceil 4, floor 3, dust 1.
        assert_eq!(round_fee_up(1_001, 30, 10_000), 4);
        assert_eq!(round_fee_down(1_001, 30, 10_000), 3);
        assert_eq!(fee_dust(1_001, 30, 10_000), 1);

        // Exact amounts leave no dust.
        assert_eq!(
            round_fee_up(10_000, 30, 10_000),
            round_fee_down(10_000, 30, 10_000)
        );
        assert_eq!(fee_dust(10_000, 30, 10_000), 0);

        // A zero denominator cannot divide, and yields no fee.
        assert_eq!(round_fee_up(100, 30, 0), 0);
        assert_eq!(round_fee_down(100, 30, 0), 0);
        assert_eq!(round_fee_up(0, 30, 10_000), 0);
    }

    #[test]
    fn staking_model_matches_the_production_points_maths() {
        assert_eq!(REWARD_POINTS_DENOM, 1_000_000_000);
        // Half the stake scores twice the points for the same rewards.
        assert_eq!(reward_points(1_000, 100), 10 * REWARD_POINTS_DENOM);
        assert_eq!(reward_points(1_000, 200), 5 * REWARD_POINTS_DENOM);
        assert_eq!(reward_points(1_000, 0), 0);

        // One point per token over a 10-token stake, at the named scale.
        assert_eq!(pending_rewards(0, REWARD_POINTS_DENOM, 10), 10);
        // Nothing pending once the snapshot has caught up.
        assert_eq!(pending_rewards(500_000_000, 500_000_000, 1_000_000_000), 0);
        // A projection never goes backwards when the accumulator moves on.
        assert!(pending_rewards(100_000_000, 200_000_000, 1_000_000_000) > 0);
        assert_eq!(pending_rewards(1_000_000_000, 1_000_000_000, 0), 0);
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Kani proofs. Compiled only by `cargo kani`.
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(kani)]
mod proofs {
    use super::*;

    /// The borrow rate never leaves its band, never falls as utilisation
    /// rises, and saturates above 100%.
    pub mod lending_rate_proofs {
        use super::*;

        #[kani::proof]
        fn prove_rate_stays_within_documented_band() {
            let utilisation_bps: u32 = kani::any();

            let rate = compute_borrow_rate(utilisation_bps);

            assert!(
                rate >= BASE_RATE_BPS,
                "rate fell below the base rate: a borrower would be undercharged"
            );
            assert!(
                rate <= BASE_RATE_BPS + SLOPE_BPS,
                "rate exceeded base + slope: a borrower would be overcharged"
            );
        }

        #[kani::proof]
        fn prove_rate_is_monotonic_in_utilisation() {
            let low: u32 = kani::any();
            let high: u32 = kani::any();
            kani::assume(low <= high);

            assert!(
                compute_borrow_rate(low) <= compute_borrow_rate(high),
                "a higher utilisation produced a lower rate"
            );
        }

        #[kani::proof]
        fn prove_rate_saturates_at_full_utilisation() {
            let utilisation_bps: u32 = kani::any();
            kani::assume(utilisation_bps >= FULL_UTILISATION_BPS);

            assert_eq!(
                compute_borrow_rate(utilisation_bps),
                compute_borrow_rate(FULL_UTILISATION_BPS),
                "utilisation above 100% was not clamped"
            );
        }
    }

    /// The two roundings bracket the exact fee, and the dust between them is at
    /// most one unit.
    pub mod fee_rounding_proofs {
        use super::*;

        #[kani::proof]
        fn prove_floor_never_exceeds_ceiling() {
            let amount: u128 = kani::any();
            let fee_bps: u128 = kani::any();
            let denominator: u128 = kani::any();

            assert!(
                round_fee_down(amount, fee_bps, denominator)
                    <= round_fee_up(amount, fee_bps, denominator)
            );
        }

        #[kani::proof]
        fn prove_dust_is_at_most_one_unit() {
            let amount: u128 = kani::any();
            let fee_bps: u128 = kani::any();
            let denominator: u128 = kani::any();

            kani::assume(denominator > 0);
            // Exclude the saturating regime: there the fee is capped, not
            // rounded, and the dust bound does not apply.
            kani::assume(amount.checked_mul(fee_bps).is_some());
            kani::assume(
                amount
                    .checked_mul(fee_bps)
                    .and_then(|n| n.checked_add(denominator - 1))
                    .is_some(),
            );

            assert!(
                fee_dust(amount, fee_bps, denominator) <= 1,
                "rounding up cost the payer more than one unit of dust"
            );
        }

        #[kani::proof]
        fn prove_ceiling_covers_the_exact_fee() {
            let amount: u128 = kani::any();
            let fee_bps: u128 = kani::any();
            let denominator: u128 = kani::any();

            kani::assume(denominator > 0);
            kani::assume(amount.checked_mul(fee_bps).is_some());
            kani::assume(
                amount
                    .checked_mul(fee_bps)
                    .and_then(|n| n.checked_add(denominator - 1))
                    .is_some(),
            );

            let exact = amount * fee_bps;
            assert!(
                round_fee_up(amount, fee_bps, denominator) * denominator >= exact,
                "rounding up produced a fee that does not cover the exact amount"
            );
        }

        #[kani::proof]
        fn prove_zero_denominator_yields_no_fee() {
            let amount: u128 = kani::any();
            let fee_bps: u128 = kani::any();

            assert_eq!(round_fee_up(amount, fee_bps, 0), 0);
            assert_eq!(round_fee_down(amount, fee_bps, 0), 0);
        }
    }

    /// Reward points and pending accrual behave monotonically and settle to
    /// zero.
    pub mod staking_points_proofs {
        use super::*;

        #[kani::proof]
        fn prove_zero_stake_scores_no_points() {
            let rewards: u128 = kani::any();

            assert_eq!(reward_points(rewards, 0), 0);
        }

        #[kani::proof]
        fn prove_points_are_monotonic_in_rewards() {
            let low: u128 = kani::any();
            let high: u128 = kani::any();
            let staked: u128 = kani::any();
            kani::assume(low <= high);

            assert!(reward_points(low, staked) <= reward_points(high, staked));
        }

        #[kani::proof]
        fn prove_pending_is_zero_once_the_snapshot_catches_up() {
            let accrued_per_token: u128 = kani::any();
            let staked: u128 = kani::any();

            assert_eq!(
                pending_rewards(accrued_per_token, accrued_per_token, staked),
                0
            );
        }

        #[kani::proof]
        fn prove_pending_is_monotonic_in_the_accumulator() {
            let accrued_per_token: u128 = kani::any();
            let lower: u128 = kani::any();
            let higher: u128 = kani::any();
            let staked: u128 = kani::any();
            kani::assume(lower <= higher);

            assert!(
                pending_rewards(accrued_per_token, lower, staked)
                    <= pending_rewards(accrued_per_token, higher, staked),
                "a larger global accrual produced a smaller pending reward"
            );
        }
    }
}
