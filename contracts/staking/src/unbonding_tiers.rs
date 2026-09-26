// Unbonding-tier ladder for staker exits (Issue #1155).
//
// This file used to sit in the crate as an orphan: nothing declared it, so
// `ValidatorTier`, `unlock_at` and `is_unbonded` were never compiled and the
// A/B/C ladder they describe governed nothing. The live exit path in `lib.rs`
// knew only about `StakeInfo::lock_until`, a single number derived from the
// `LockPeriod` the staker picked, and `unstake` applied one global
// `early_withdrawal_penalty_bps` to anything that exited before it. Two
// consequences #1155 called out:
//
//   * `LockPeriod::Custom(blocks)` accepted any `blocks` the caller liked,
//     including `Custom(1)`, so the "lock" a staker opted into could be one
//     block long and then be exited from for the same flat 10% penalty a
//     28-day lock would attract; and
//   * the tier parameters (28d / 14d / 7d) were compile-time constants with no
//     on-chain representation, so an operator could not tune them and no
//     message exposed them.
//
// Two corrections to the original file are worth stating explicitly, because
// both are silent-wrongness rather than dead code:
//
//   * **Units.** The orphan tier durations were in *seconds* (`28 * 24 * 3600`)
//     while every other duration in this contract is in *blocks* —
//     `LockPeriod::duration_blocks`, `types::UNBONDING_PERIOD_BLOCKS`,
//     `constants::LOCK_PERIOD_30_DAYS` and the `self.env().block_number()`
//     reads that produce `lock_until` and `staked_at`. Comparing a
//     seconds-denominated tier against a blocks-denominated `lock_until` is a
//     ~14,400x error, so the tier durations below are expressed in blocks at
//     the chain's 6-second block time, matching `LOCK_PERIOD_30_DAYS`
//     (432_000 == 30 * 14_400).
//   * **Naming.** The type was called `ValidatorTier`, which reads as a
//     validator-set concept and collides conceptually with `StakingTier`
//     (Bronze..Diamond) in `types.rs`, a *reward* tier keyed on stake amount.
//     Nothing external ever named the type, so it is renamed to
//     `UnbondingTier` to keep the two ladders distinguishable.
//
// # What is deliberately *not* here
//
// The ladder is keyed on **lock duration only**. The issue also suggested
// deriving the tier from "stake size/period", but size already selects a
// `StakingTier` through `Staking::get_tier_internal`, which feeds
// `StakingTier::reward_multiplier`. Deriving the unbonding tier from amount as
// well would make one field drive two independent ladders and give operators
// two knobs that can disagree; the tier is a statement about *time*, so it is
// keyed on time. `tier_for_duration` is the single place that decides it.

use ink::storage::traits::StorageLayout;
use propchain_traits::constants::LOCK_PERIOD_1_YEAR;

/// Block time assumed by the tier ladder, in seconds. Matches the 6-second
/// blocks implied by `constants::LOCK_PERIOD_30_DAYS` (432_000 blocks).
pub const SECONDS_PER_BLOCK: u64 = 6;

/// Blocks in one day at `SECONDS_PER_BLOCK`.
pub const BLOCKS_PER_DAY: u64 = 86_400 / SECONDS_PER_BLOCK;

/// Longest tier-A unbonding window: 28 days.
pub const TIER_A_UNBONDING_BLOCKS: u64 = 28 * BLOCKS_PER_DAY;

/// Tier-B unbonding window: 14 days.
pub const TIER_B_UNBONDING_BLOCKS: u64 = 14 * BLOCKS_PER_DAY;

/// Tier-C unbonding window: 7 days.
pub const TIER_C_UNBONDING_BLOCKS: u64 = 7 * BLOCKS_PER_DAY;

/// Largest unbonding window an operator may configure for a tier.
///
/// One year, matching `constants::LOCK_PERIOD_1_YEAR`. The cap exists so a
/// mistyped `set_unbonding_tier_config` cannot set a window so long that the
/// tier becomes a permanent lock with no path back: the admin can always lower
/// it again, but a value near `u64::MAX` would saturate `staked_at + window`
/// and pin every affected stake until the chain state is replaced.
pub const MAX_TIER_UNBONDING_BLOCKS: u64 = LOCK_PERIOD_1_YEAR;

/// A staker's rung on the unbonding ladder.
///
/// Tiers are ordered longest-lock-first in prose and in `ALL`: `A` is the most
/// committed rung and carries the longest minimum window.
///
/// Note this type deliberately derives no `Ord`. A derived ordering would rank
/// variants by *declaration* order, which is the opposite of the commitment
/// order above (`A` is declared first but is the most restrictive). Compare
/// `default_unbonding_blocks` when a total order over commitment is wanted.
#[derive(
    Clone,
    Copy,
    Debug,
    Default,
    PartialEq,
    Eq,
    scale::Encode,
    scale::Decode,
    StorageLayout,
)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub enum UnbondingTier {
    /// 28-day minimum window.
    A,
    /// 14-day minimum window.
    B,
    /// 7-day minimum window. Also the tier of any lock shorter than 7 days
    /// and the tier assumed for stakes recorded before #1155.
    #[default]
    C,
}

impl UnbondingTier {
    /// Every tier, longest window first. Used by the config getter so a caller
    /// can read the whole ladder in one call.
    pub const ALL: [UnbondingTier; 3] = [UnbondingTier::A, UnbondingTier::B, UnbondingTier::C];

    /// The shipped default minimum unbonding window for this tier, in blocks.
    pub const fn default_unbonding_blocks(&self) -> u64 {
        match self {
            UnbondingTier::A => TIER_A_UNBONDING_BLOCKS,
            UnbondingTier::B => TIER_B_UNBONDING_BLOCKS,
            UnbondingTier::C => TIER_C_UNBONDING_BLOCKS,
        }
    }

    /// Short stable label, for events and off-chain decoding.
    pub const fn as_str(&self) -> &'static str {
        match self {
            UnbondingTier::A => "A",
            UnbondingTier::B => "B",
            UnbondingTier::C => "C",
        }
    }
}

/// Operator-configurable parameters for one rung of the ladder.
///
/// Stored per tier by `Staking::unbonding_tier_configs`, so the ladder is
/// readable and writable on-chain instead of being frozen into the binary.
#[derive(
    Clone,
    Copy,
    Debug,
    PartialEq,
    Eq,
    scale::Encode,
    scale::Decode,
    StorageLayout,
)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub struct UnbondingTierConfig {
    /// Minimum blocks a stake on this tier must be held before it can exit
    /// without penalty. `0` disables the tier's minimum window, which
    /// reproduces the pre-#1155 behaviour where the staker's own `LockPeriod`
    /// was the only bound.
    pub unbonding_blocks: u64,
    /// Penalty in basis points applied when a stake on this tier exits before
    /// its effective unlock. Clamped by
    /// `constants::MAX_EARLY_WITHDRAWAL_PENALTY_BPS` at the setter, and
    /// combined with the contract-wide `early_withdrawal_penalty_bps` by
    /// `effective_penalty_bps` — a tier may raise the price of an early exit
    /// but never lower it below the operator's global setting.
    pub early_exit_penalty_bps: u128,
    /// When `false`, exiting before `unbonding_blocks` has elapsed is rejected
    /// outright with `Error::LockActive` instead of being allowed through with
    /// a penalty. Ships `true` so existing early-exit flows keep working.
    pub early_exit_allowed: bool,
}

impl UnbondingTierConfig {
    /// The configuration a tier is created with: the documented window, no
    /// tier-specific penalty on top of the global one, and early exit allowed.
    ///
    /// `early_exit_penalty_bps` defaults to `0` deliberately. The effective
    /// penalty is `max(global, tier)`, so a zero tier penalty makes the ladder
    /// enforce the *windows* out of the box while leaving the *price* of an
    /// early exit exactly where `DEFAULT_EARLY_WITHDRAWAL_PENALTY_BPS` already
    /// put it. An operator opts into repricing by calling
    /// `set_unbonding_tier_config`; existing stakers are never silently
    /// repriced by an upgrade.
    pub const fn default_for(tier: UnbondingTier) -> Self {
        Self {
            unbonding_blocks: tier.default_unbonding_blocks(),
            early_exit_penalty_bps: 0,
            early_exit_allowed: true,
        }
    }
}

/// The tier a lock of `duration_blocks` falls into.
///
/// The rungs are half-open upward: a lock is tier `A` once it reaches
/// `TIER_A_UNBONDING_BLOCKS`, tier `B` from `TIER_B_UNBONDING_BLOCKS`, and
/// tier `C` for everything shorter — including `LockPeriod::Flexible`, whose
/// `duration_blocks()` is `0`.
///
/// Note that a `LockPeriod` longer than a tier's window does not lower the
/// tier's minimum. `Staking` takes the maximum of the staker's own `lock_until`
/// and the tier window (see `effective_unlock_block`), so a 90-day lock is
/// tier `A` by duration and still exits no earlier than 90 days.
pub fn tier_for_duration(duration_blocks: u64) -> UnbondingTier {
    if duration_blocks >= TIER_A_UNBONDING_BLOCKS {
        UnbondingTier::A
    } else if duration_blocks >= TIER_B_UNBONDING_BLOCKS {
        UnbondingTier::B
    } else {
        UnbondingTier::C
    }
}

/// Block at which a stake taken at `staked_at` on `tier` becomes free to exit.
///
/// This is the tier's *minimum* window. `Staking::unstake` compares it against
/// the staker's own `lock_until` and honours whichever is later.
pub fn tier_unlock_at(staked_at: u64, config: &UnbondingTierConfig) -> u64 {
    staked_at.saturating_add(config.unbonding_blocks)
}

/// The block a stake actually becomes free to exit, given both bounds.
///
/// `staked_at` and `lock_until` are the staker's, `tier_config` is the rung the
/// staker was placed on. `honours_tier_window` is false for
/// `LockPeriod::Flexible`, which is the contract's explicit "no lock" option:
/// gating it behind a tier window would silently turn the flexible product
/// into a 7-day lock, so the staker's own `lock_until` is used alone.
pub fn effective_unlock_block(
    staked_at: u64,
    lock_until: u64,
    tier_config: &UnbondingTierConfig,
    honours_tier_window: bool,
) -> u64 {
    if !honours_tier_window {
        return lock_until;
    }
    let tier_unlock = tier_unlock_at(staked_at, tier_config);
    if tier_unlock > lock_until {
        tier_unlock
    } else {
        lock_until
    }
}

/// The penalty rate that applies to an early exit from a stake on `tier`.
///
/// `max` of the contract-wide setting and the tier's own, so a tier can only
/// make leaving early more expensive. A tier configured at `0` therefore
/// behaves exactly as it did before #1155.
pub fn effective_penalty_bps(global_bps: u128, tier_bps: u128) -> u128 {
    if tier_bps > global_bps {
        tier_bps
    } else {
        global_bps
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cfg(tier: UnbondingTier) -> UnbondingTierConfig {
        UnbondingTierConfig::default_for(tier)
    }

    // ---- Window arithmetic, in blocks (the unit the rest of the crate uses) --

    #[test]
    fn tier_windows_are_the_documented_day_counts() {
        assert_eq!(TIER_A_UNBONDING_BLOCKS, 28 * 14_400);
        assert_eq!(TIER_B_UNBONDING_BLOCKS, 14 * 14_400);
        assert_eq!(TIER_C_UNBONDING_BLOCKS, 7 * 14_400);
    }

    #[test]
    fn windows_are_strictly_descending() {
        assert!(TIER_A_UNBONDING_BLOCKS > TIER_B_UNBONDING_BLOCKS);
        assert!(TIER_B_UNBONDING_BLOCKS > TIER_C_UNBONDING_BLOCKS);
    }

    #[test]
    fn unlock_at_offsets_from_staked_at() {
        assert_eq!(tier_unlock_at(1_000, &cfg(UnbondingTier::A)), 1_000 + TIER_A_UNBONDING_BLOCKS);
        assert_eq!(tier_unlock_at(1_000, &cfg(UnbondingTier::C)), 1_000 + TIER_C_UNBONDING_BLOCKS);
    }

    #[test]
    fn unlock_at_saturates_instead_of_wrapping() {
        // A stake taken near the end of the u64 block space must not wrap to a
        // block in the past, which would read as "already unlocked".
        assert_eq!(tier_unlock_at(u64::MAX, &cfg(UnbondingTier::A)), u64::MAX);
    }

    // ---- Tier boundaries: one block either side of every rung ----

    #[test]
    fn tier_a_boundary_is_inclusive_at_28_days() {
        assert_eq!(tier_for_duration(TIER_A_UNBONDING_BLOCKS - 1), UnbondingTier::B);
        assert_eq!(tier_for_duration(TIER_A_UNBONDING_BLOCKS), UnbondingTier::A);
        assert_eq!(tier_for_duration(TIER_A_UNBONDING_BLOCKS + 1), UnbondingTier::A);
    }

    #[test]
    fn tier_b_boundary_is_inclusive_at_14_days() {
        assert_eq!(tier_for_duration(TIER_B_UNBONDING_BLOCKS - 1), UnbondingTier::C);
        assert_eq!(tier_for_duration(TIER_B_UNBONDING_BLOCKS), UnbondingTier::B);
        assert_eq!(tier_for_duration(TIER_B_UNBONDING_BLOCKS + 1), UnbondingTier::B);
    }

    #[test]
    fn tier_c_catches_everything_shorter_than_7_days() {
        assert_eq!(tier_for_duration(0), UnbondingTier::C);
        assert_eq!(tier_for_duration(1), UnbondingTier::C);
        assert_eq!(tier_for_duration(TIER_C_UNBONDING_BLOCKS - 1), UnbondingTier::C);
        assert_eq!(tier_for_duration(TIER_C_UNBONDING_BLOCKS), UnbondingTier::C);
    }

    #[test]
    fn a_lock_longer_than_a_year_stays_tier_a() {
        // The 1-year cap bounds *configuration*, not tier assignment: a longer
        // lock must not fall through to a shorter window.
        assert_eq!(tier_for_duration(MAX_TIER_UNBONDING_BLOCKS + 1), UnbondingTier::A);
        assert_eq!(tier_for_duration(u64::MAX), UnbondingTier::A);
    }

    #[test]
    fn commitment_order_matches_window_length() {
        // `ALL` is documented longest-window-first, and every rung's default
        // window is strictly shorter than the one before it.
        let mut previous = u64::MAX;
        for tier in UnbondingTier::ALL {
            let window = tier.default_unbonding_blocks();
            assert!(window < previous, "tier {} window must shrink", tier.as_str());
            previous = window;
        }
        assert_eq!(previous, TIER_C_UNBONDING_BLOCKS);
    }

    // ---- Effective unlock: the staker's lock and the tier window, max of both ----

    #[test]
    fn tier_window_raises_a_custom_lock_shorter_than_the_window() {
        // The regression #1155 is about: Custom(1_000) blocks used to be free to
        // leave after 1_000 blocks. On tier C the floor is 7 days.
        let unlock = effective_unlock_block(0, 1_000, &cfg(UnbondingTier::C), true);
        assert_eq!(unlock, TIER_C_UNBONDING_BLOCKS);
    }

    #[test]
    fn a_longer_staker_lock_wins_over_the_tier_window() {
        // A 90-day lock is tier A by duration, but its own lock_until is later
        // than the 28-day tier window and must not be shortened.
        let lock_until = TIER_A_UNBONDING_BLOCKS + 1_000;
        let unlock = effective_unlock_block(0, lock_until, &cfg(UnbondingTier::A), true);
        assert_eq!(unlock, lock_until);
    }

    #[test]
    fn flexible_stakes_are_never_gated_by_a_tier_window() {
        // LockPeriod::Flexible resolves to lock_until == staked_at. Applying the
        // tier window here would silently convert "flexible" into a 7-day lock.
        let unlock = effective_unlock_block(500, 500, &cfg(UnbondingTier::A), false);
        assert_eq!(unlock, 500);
    }

    #[test]
    fn a_zero_window_tier_reproduces_pre_1155_behaviour() {
        // An operator who sets unbonding_blocks = 0 gets exactly the old
        // single-`lock_until` semantics back.
        let zero = UnbondingTierConfig {
            unbonding_blocks: 0,
            early_exit_penalty_bps: 0,
            early_exit_allowed: true,
        };
        assert_eq!(effective_unlock_block(0, 1_000, &zero, true), 1_000);
    }

    // ---- Effective penalty: the tier may raise the price, never lower it ----

    #[test]
    fn tier_penalty_raises_the_global_rate() {
        assert_eq!(effective_penalty_bps(1_000, 3_000), 3_000);
    }

    #[test]
    fn tier_penalty_below_the_global_rate_is_ignored() {
        // This is what keeps the shipped defaults from repricing anyone.
        assert_eq!(effective_penalty_bps(1_000, 0), 1_000);
        assert_eq!(effective_penalty_bps(5_000, 2_500), 5_000);
    }

    #[test]
    fn equal_rates_are_not_double_charged() {
        assert_eq!(effective_penalty_bps(1_000, 1_000), 1_000);
    }

    // ---- Shipped defaults ----

    #[test]
    fn default_config_matches_the_documented_ladder() {
        assert_eq!(
            UnbondingTierConfig::default_for(UnbondingTier::A).unbonding_blocks,
            28 * 86_400 / SECONDS_PER_BLOCK
        );
        assert_eq!(
            UnbondingTierConfig::default_for(UnbondingTier::B).unbonding_blocks,
            14 * 86_400 / SECONDS_PER_BLOCK
        );
        assert_eq!(
            UnbondingTierConfig::default_for(UnbondingTier::C).unbonding_blocks,
            7 * 86_400 / SECONDS_PER_BLOCK
        );
    }

    #[test]
    fn default_config_does_not_reprice_early_exits() {
        // Global default is 1_000 bps; a default tier must not exceed it.
        for tier in UnbondingTier::ALL {
            let config = UnbondingTierConfig::default_for(tier);
            assert_eq!(config.early_exit_penalty_bps, 0);
            assert!(config.early_exit_allowed);
            assert_eq!(effective_penalty_bps(1_000, config.early_exit_penalty_bps), 1_000);
        }
    }

    #[test]
    fn every_default_window_is_configurable() {
        for tier in UnbondingTier::ALL {
            assert!(UnbondingTierConfig::default_for(tier).unbonding_blocks
                <= MAX_TIER_UNBONDING_BLOCKS);
        }
    }

    #[test]
    fn default_tier_is_c() {
        assert_eq!(UnbondingTier::default(), UnbondingTier::C);
    }
}
