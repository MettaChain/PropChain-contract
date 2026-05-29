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
}

impl LockPeriod {
    pub fn duration_blocks(&self) -> u64 {
        match self {
            LockPeriod::Flexible => 0,
            LockPeriod::ThirtyDays => constants::LOCK_PERIOD_30_DAYS,
            LockPeriod::NinetyDays => constants::LOCK_PERIOD_90_DAYS,
            LockPeriod::OneYear => constants::LOCK_PERIOD_1_YEAR,
        }
    }

    pub fn multiplier(&self) -> u128 {
        match self {
            LockPeriod::Flexible => constants::MULTIPLIER_FLEXIBLE,
            LockPeriod::ThirtyDays => constants::MULTIPLIER_30_DAYS,
            LockPeriod::NinetyDays => constants::MULTIPLIER_90_DAYS,
            LockPeriod::OneYear => constants::MULTIPLIER_1_YEAR,
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
