// Reward-point bookkeeping for staker history (Issue #1154).
//
// This file used to sit in the crate as an orphan: nothing declared it, so
// `RewardSnapshot` and the `1_000_000_000` divisor inside `pending` were never
// compiled. The live reward path in `lib.rs` instead recomputes a staker's
// entire history from `StakeInfo::staked_at` on every call, which has two
// consequences #1154 called out:
//
//   * `update_config` changes `reward_rate_bps`, and every block already
//     served was re-priced at the new rate on the next read, so a config
//     change retroactively rewrote what a staker had earned; and
//   * a staker could never see a settled history, only a projection that
//     moved every time the admin touched the config.
//
// What lives here now is deliberately not a second reward engine: the amount a
// staker is paid is still decided by `Staking::claim_rewards`. This module owns
// the record of those decisions - the per-staker snapshot and the per-boundary
// checkpoints - so a closed segment cannot be re-priced later.
//
// The orphan `RewardSnapshot` also carried a `staker: [u8; 32]` field that
// nothing ever populated. It is gone: snapshots are stored in a mapping keyed by
// staker, so a second copy of the key inside the value had nowhere to come from.

/// Fixed-point scale for reward points.
///
/// A point is "rewards earned per unit of stake" scaled up by this factor, so
/// that per-token values with many leading zeros survive integer division.
pub const REWARD_POINTS_DENOM: u128 = 1_000_000_000;

/// The most recent settled position of a staker's reward history.
///
/// One snapshot exists per staker from the block they staked at. It is written
/// at every reward boundary and is the staker's own record of where their
/// accrual stood at that block.
#[derive(
    Clone,
    Copy,
    Debug,
    Default,
    PartialEq,
    Eq,
    scale::Encode,
    scale::Decode,
    ink::storage::traits::StorageLayout,
)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub struct RewardSnapshot {
    /// Cumulative reward points per unit of stake, scaled by
    /// [`REWARD_POINTS_DENOM`], as of `last_updated_block`.
    pub accrued_per_token: u128,
    /// Block at which this snapshot was last written.
    pub last_updated_block: u64,
}

impl RewardSnapshot {
    /// An empty snapshot, valid from block 0.
    pub fn new() -> Self {
        Self {
            accrued_per_token: 0,
            last_updated_block: 0,
        }
    }

    /// Rewards still to come from a global accumulator that has moved on to
    /// `global_accrued` since this snapshot was taken.
    ///
    /// Kept for callers that track accrual through a global points figure; the
    /// staking contract settles segments in [`RewardCheckpoint`]s instead.
    pub fn pending(&self, global_accrued: u128, staked: u128) -> u128 {
        global_accrued
            .saturating_sub(self.accrued_per_token)
            .saturating_mul(staked)
            / REWARD_POINTS_DENOM
    }

    /// Settle a segment at `block`, moving the snapshot to `accrued`.
    pub fn flush(&mut self, accrued: u128, block: u64) {
        self.accrued_per_token = accrued;
        self.last_updated_block = block;
    }
}

/// Reward points for `rewards` earned against a stake of `staked`.
///
/// A zero stake has no per-token value to record, so it scores 0 points rather
/// than dividing by zero.
pub fn points_for(rewards: u128, staked: u128) -> u128 {
    if staked == 0 {
        return 0;
    }
    rewards
        .saturating_mul(REWARD_POINTS_DENOM)
        .saturating_div(staked)
}

/// Why a reward segment was settled.
#[derive(
    Clone,
    Copy,
    Debug,
    Default,
    PartialEq,
    Eq,
    scale::Encode,
    scale::Decode,
    ink::storage::traits::StorageLayout,
)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub enum CheckpointReason {
    /// The staker claimed a reward segment.
    #[default]
    Claimed,
    /// The staker's rewards were auto-compounded back into the stake.
    AutoCompounded,
    /// The staker claimed against a vesting schedule.
    VestingClaimed,
}

/// One settled reward segment, stamped with the block it was settled at.
///
/// Checkpoints are append-only: a segment recorded here is never re-priced, so
/// a later `reward_rate_bps` change cannot reach back into it.
#[derive(
    Clone,
    Copy,
    Debug,
    Default,
    PartialEq,
    Eq,
    scale::Encode,
    scale::Decode,
    ink::storage::traits::StorageLayout,
)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub struct RewardCheckpoint {
    /// Block the segment was settled at.
    pub block: u64,
    /// Reward points for the segment, per [`points_for`].
    pub points: u128,
    /// Reward amount settled by the segment.
    pub rewards: u128,
    /// What caused the segment to settle.
    pub reason: CheckpointReason,
}

impl RewardCheckpoint {
    /// A checkpoint for `rewards` settled at `block`.
    pub fn new(staked: u128, rewards: u128, block: u64, reason: CheckpointReason) -> Self {
        Self {
            block,
            points: points_for(rewards, staked),
            rewards,
            reason,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pending_before_flush_is_positive() {
        let snap = RewardSnapshot::new();
        assert!(snap.pending(500_000_000, 2_000_000_000) > 0);
    }

    #[test]
    fn no_pending_after_flush() {
        let mut snap = RewardSnapshot::new();
        snap.flush(500_000_000, 100);
        assert_eq!(snap.pending(500_000_000, 1_000_000_000), 0);
    }

    #[test]
    fn rewards_accumulate_between_flushes() {
        let mut snap = RewardSnapshot::new();
        snap.flush(100_000_000, 1);
        assert!(snap.pending(200_000_000, 1_000_000_000) > 0);
    }

    #[test]
    fn zero_stake_yields_no_pending() {
        let snap = RewardSnapshot::new();
        assert_eq!(snap.pending(1_000_000_000, 0), 0);
    }

    #[test]
    fn pending_uses_the_named_denominator() {
        let snap = RewardSnapshot::new();
        // 1 point per token over a 10-token stake, at the named scale.
        assert_eq!(
            snap.pending(REWARD_POINTS_DENOM, 10),
            10,
            "pending must divide by REWARD_POINTS_DENOM"
        );
    }

    #[test]
    fn flush_moves_the_snapshot_forward() {
        let mut snap = RewardSnapshot::new();
        snap.flush(42, 77);
        assert_eq!(snap.accrued_per_token, 42);
        assert_eq!(snap.last_updated_block, 77);
    }

    #[test]
    fn points_scale_with_stake_size() {
        // Same rewards against twice the stake scores half the points.
        assert_eq!(points_for(1_000, 100), 10 * REWARD_POINTS_DENOM);
        assert_eq!(points_for(1_000, 200), 5 * REWARD_POINTS_DENOM);
    }

    #[test]
    fn points_of_zero_stake_are_zero() {
        assert_eq!(points_for(1_000, 0), 0);
    }

    #[test]
    fn checkpoint_stamps_block_points_and_rewards() {
        let cp = RewardCheckpoint::new(1_000, 50, 42, CheckpointReason::Claimed);
        assert_eq!(cp.block, 42);
        assert_eq!(cp.rewards, 50);
        assert_eq!(cp.points, points_for(50, 1_000));
        assert_eq!(cp.reason, CheckpointReason::Claimed);
    }

    #[test]
    fn checkpoint_distinguishes_claim_from_compound() {
        let claimed = RewardCheckpoint::new(1_000, 10, 1, CheckpointReason::Claimed);
        let compounded = RewardCheckpoint::new(1_000, 10, 1, CheckpointReason::AutoCompounded);
        assert_ne!(claimed, compounded);
    }
}
