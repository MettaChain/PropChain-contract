// Data types for the property token contract (Issue #101 - extracted from lib.rs)

/// Token ID type alias
pub type TokenId = u64;

/// Chain ID type alias
pub type ChainId = u64;

/// Ownership transfer record
#[derive(
    Debug, Clone, PartialEq, scale::Encode, scale::Decode, ink::storage::traits::StorageLayout,
)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub struct OwnershipTransfer {
    pub from: AccountId,
    pub to: AccountId,
    pub timestamp: u64,
    pub transaction_hash: Hash,
}

/// Compliance information
#[derive(
    Debug, Clone, PartialEq, scale::Encode, scale::Decode, ink::storage::traits::StorageLayout,
)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub struct ComplianceInfo {
    pub verified: bool,
    pub verification_date: u64,
    pub verifier: AccountId,
    pub compliance_type: String,
}

/// Legal document information
#[derive(
    Debug, Clone, PartialEq, scale::Encode, scale::Decode, ink::storage::traits::StorageLayout,
)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub struct DocumentInfo {
    pub document_hash: Hash,
    pub document_type: String,
    pub upload_date: u64,
    pub uploader: AccountId,
}

/// Bridged token information
#[derive(
    Debug, Clone, PartialEq, scale::Encode, scale::Decode, ink::storage::traits::StorageLayout,
)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub struct BridgedTokenInfo {
    pub original_chain: ChainId,
    pub original_token_id: TokenId,
    pub destination_chain: ChainId,
    pub destination_token_id: TokenId,
    pub bridged_at: u64,
    pub status: BridgingStatus,
}

/// Bridging status enum
#[derive(
    Debug, Clone, PartialEq, scale::Encode, scale::Decode, ink::storage::traits::StorageLayout,
)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub enum BridgingStatus {
    Locked,
    Pending,
    InTransit,
    Completed,
    Failed,
    Recovering,
    Expired,
}

/// Error log entry for monitoring and debugging
#[derive(
    Debug, Clone, PartialEq, scale::Encode, scale::Decode, ink::storage::traits::StorageLayout,
)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub struct ErrorLogEntry {
    pub error_code: String,
    pub message: String,
    pub account: AccountId,
    pub timestamp: u64,
    pub context: Vec<(String, String)>,
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
pub struct Proposal {
    pub id: u64,
    pub token_id: TokenId,
    pub description_hash: Hash,
    pub quorum: u128,
    pub for_votes: u128,
    pub against_votes: u128,
    pub status: ProposalStatus,
    pub created_at: u64,
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
pub enum ProposalStatus {
    Open,
    Executed,
    Rejected,
    Closed,
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
pub struct Ask {
    pub token_id: TokenId,
    pub seller: AccountId,
    pub price_per_share: u128,
    pub amount: u128,
    pub created_at: u64,
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
pub struct TaxRecord {
    pub dividends_received: u128,
    pub shares_sold: u128,
    pub proceeds: u128,
}

/// Lock period options for staking fractional shares
#[derive(
    Debug,
    Clone,
    Copy,
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
pub enum ShareLockPeriod {
    Flexible,
    ThirtyDays,
    NinetyDays,
    OneYear,
}

impl ShareLockPeriod {
    pub fn duration_blocks(&self) -> u64 {
        match self {
            ShareLockPeriod::Flexible => 0,
            ShareLockPeriod::ThirtyDays => LOCK_PERIOD_30_DAYS,
            ShareLockPeriod::NinetyDays => LOCK_PERIOD_90_DAYS,
            ShareLockPeriod::OneYear => LOCK_PERIOD_1_YEAR,
        }
    }

    /// Returns the reward/governance multiplier in basis points (100 = 1×)
    pub fn multiplier(&self) -> u128 {
        match self {
            ShareLockPeriod::Flexible => MULTIPLIER_FLEXIBLE,
            ShareLockPeriod::ThirtyDays => MULTIPLIER_30_DAYS,
            ShareLockPeriod::NinetyDays => MULTIPLIER_90_DAYS,
            ShareLockPeriod::OneYear => MULTIPLIER_1_YEAR,
        }
    }
}

/// Per-account, per-token staking record
pub enum VestingRole {
    Team,
    Investor,
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
pub struct ShareStakeInfo {
    pub staker: AccountId,
    pub token_id: TokenId,
    pub amount: u128,
    pub staked_at: u64,
    pub lock_until: u64,
    pub lock_period: ShareLockPeriod,
    pub reward_debt: u128,
}
pub struct VestingSchedule {
    pub role: VestingRole,
    pub total_amount: u128,
    pub claimed_amount: u128,
    pub start_time: u64,
    pub cliff_duration: u64,
    pub vesting_duration: u64,
}

