// Data types for the staking contract (Issue #101 - extracted from lib.rs)

#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    scale::Encode,
    scale::Decode,
    ink::storage::traits::StorageLayout,
)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub enum LockPeriod {
    Flexible,
    ThirtyDays,
    NinetyDays,
    OneYear,
    /// User-defined lock duration in blocks
    Custom(u64),
}

impl LockPeriod {
    pub fn duration_blocks(&self) -> u64 {
        match self {
            LockPeriod::Flexible => 0,
            LockPeriod::ThirtyDays => constants::LOCK_PERIOD_30_DAYS,
            LockPeriod::NinetyDays => constants::LOCK_PERIOD_90_DAYS,
            LockPeriod::OneYear => constants::LOCK_PERIOD_1_YEAR,
            LockPeriod::Custom(blocks) => *blocks,
        }
    }

    pub fn multiplier(&self) -> u128 {
        match self {
            LockPeriod::Flexible => constants::MULTIPLIER_FLEXIBLE,
            LockPeriod::ThirtyDays => constants::MULTIPLIER_30_DAYS,
            LockPeriod::NinetyDays => constants::MULTIPLIER_90_DAYS,
            LockPeriod::OneYear => constants::MULTIPLIER_1_YEAR,
            // Custom lock period reward multiplier scales linearly
            // between Flexible (1.0x) and OneYear (2.5x) based on duration
            LockPeriod::Custom(blocks) => {
                let max_blocks = constants::LOCK_PERIOD_1_YEAR;
                let ratio = if max_blocks > 0 {
                    (*blocks as u128).min(max_blocks as u128)
                } else {
                    0
                };
                // Scale from MULTIPLIER_FLEXIBLE (100) to MULTIPLIER_1_YEAR (250)
                let range = constants::MULTIPLIER_1_YEAR.saturating_sub(constants::MULTIPLIER_FLEXIBLE);
                constants::MULTIPLIER_FLEXIBLE.saturating_add(
                    range.saturating_mul(ratio) / max_blocks as u128
                )
            }
        }
    }
}

#[derive(
    Debug,
    Clone,
    PartialEq,
    Eq,
    scale::Encode,
    scale::Decode,
    ink::storage::traits::StorageLayout,
)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub struct StakeInfo {
    pub staker: AccountId,
    pub amount: u128,
    pub staked_at: u64,
    pub lock_until: u64,
    pub lock_period: LockPeriod,
    pub reward_debt: u128,
    pub governance_delegate: Option<AccountId>,
    pub auto_compound: bool,
}

#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    scale::Encode,
    scale::Decode,
    ink::storage::traits::StorageLayout,
)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub enum StakingTier {
    Bronze,
    Silver,
    Gold,
    Platinum,
    Diamond,
}

impl StakingTier {
    pub fn name(&self) -> &'static str {
        match self {
            StakingTier::Bronze => "Bronze",
            StakingTier::Silver => "Silver",
            StakingTier::Gold => "Gold",
            StakingTier::Platinum => "Platinum",
            StakingTier::Diamond => "Diamond",
        }
    }

    pub fn reward_multiplier(&self) -> u128 {
        match self {
            StakingTier::Bronze => 100,      // 1.0x
            StakingTier::Silver => 110,      // 1.1x
            StakingTier::Gold => 120,        // 1.2x
            StakingTier::Platinum => 135,    // 1.35x
            StakingTier::Diamond => 150,     // 1.5x
        }
    }
}


/// A staking parameter that stakers can vote to change.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    scale::Encode,
    scale::Decode,
    ink::storage::traits::StorageLayout,
)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub enum ParamKind {
    MinStake(u128),
    RewardRateBps(u128),
    VotingPeriodBlocks(u64),
    QuorumBps(u32),
}

#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    scale::Encode,
    scale::Decode,
    ink::storage::traits::StorageLayout,
)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub enum ProposalStatus {
    Active,
    Executed,
    Rejected,
    Cancelled,
}

/// A proposal to change a staking parameter, voted on by stakers.
#[derive(
    Debug,
    Clone,
    PartialEq,
    Eq,
    scale::Encode,
    scale::Decode,
    ink::storage::traits::StorageLayout,
)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub struct ParamProposal {
    pub id: u64,
    pub proposer: AccountId,
    pub kind: ParamKind,
    pub votes_for: u128,
    pub votes_against: u128,
    pub voting_end: u64,
    pub total_power_snapshot: u128,
    pub status: ProposalStatus,
    pub created_at: u64,
}

// ─── Delegated Staking Types ────────────────────────────────────────────────

/// Maximum commission rate a validator may set (100% in basis points).
pub const MAX_COMMISSION_RATE: u32 = 10_000;

/// Minimum self-stake required for a validator to register or remain active.
pub const MIN_VALIDATOR_STAKE: u128 = 10_000_000;

/// Percentage of stake slashed on a misbehaving validator (and their delegators).
pub const SLASH_PERCENT: u128 = 20;

/// Unbonding period in blocks (~3.5 days at 6-second blocks).
pub const UNBONDING_PERIOD_BLOCKS: u64 = 50_400;

/// Scaling factor used in the per-validator reward accumulator (10^12).
pub const REWARD_PRECISION: u128 = 1_000_000_000_000;

/// On-chain record for a registered validator.
#[derive(
    Debug,
    Clone,
    PartialEq,
    Eq,
    scale::Encode,
    scale::Decode,
    ink::storage::traits::StorageLayout,
)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub struct ValidatorInfo {
    /// The validator's own self-stake (subject to slashing).
    pub self_stake: u128,
    /// Commission rate in basis points (0–10_000).
    pub commission_rate: u32,
    /// Sum of all active delegated amounts to this validator.
    pub total_delegated: u128,
    /// Accumulated commission not yet claimed.
    pub accumulated_commission: u128,
    /// Whether the validator is currently accepting delegations.
    pub is_active: bool,
    /// Cumulative reward-per-share for delegators (scaled by REWARD_PRECISION).
    pub acc_reward_per_share: u128,
    /// Block number of the last reward accumulation update.
    pub last_reward_block: u64,
}

/// On-chain record for a single delegator → validator binding.
#[derive(
    Debug,
    Clone,
    PartialEq,
    Eq,
    scale::Encode,
    scale::Decode,
    ink::storage::traits::StorageLayout,
)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub struct DelegationRecord {
    /// The delegator account that owns this record.
    pub delegator: AccountId,
    /// The validator this delegation is bound to.
    pub validator: AccountId,
    /// The delegated token amount (reduced by slashing).
    pub amount: u128,
    /// Snapshot of validator's acc_reward_per_share at last claim/delegation.
    pub reward_debt: u128,
    /// None = active; Some(block) = unbonding started at that block.
    pub unbonding_start: Option<u64>,
}

/// Reason a validator was deactivated (used in ValidatorDeactivated event).
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    scale::Encode,
    scale::Decode,
    ink::storage::traits::StorageLayout,
)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub enum DeactivationReason {
    Voluntary,
    Slashed,
}
