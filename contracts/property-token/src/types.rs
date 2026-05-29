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

/// KYC verification levels
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, scale::Encode, scale::Decode, ink::storage::traits::StorageLayout,
)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub enum KYCVerificationLevel {
    /// No KYC verification
    None = 0,
    /// Basic KYC with document verification
    Basic = 1,
    /// Standard KYC with AML and sanctions checks
    Standard = 2,
    /// Enhanced KYC with biometric and risk assessment
    Enhanced = 3,
    /// Institutional verification with full due diligence
    Institutional = 4,
}

/// Transfer restriction levels/types
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, scale::Encode, scale::Decode, ink::storage::traits::StorageLayout,
)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub enum TransferRestrictionLevel {
    /// No restrictions
    None,
    /// Only KYC verified users can transfer
    KYCRequired,
    /// Requires specific verification level
    VerificationLevelRequired,
    /// Whitelist only transfers
    WhitelistOnly,
    /// Blacklist prevents transfers
    BlacklistBased,
}

/// Per-token transfer restrictions configuration
#[derive(
    Debug, Clone, Copy, scale::Encode, scale::Decode, ink::storage::traits::StorageLayout,
)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub struct TransferRestrictionConfig {
    /// Restriction level for this token
    pub restriction_level: TransferRestrictionLevel,
    /// Minimum KYC verification level required
    pub min_verification_level: KYCVerificationLevel,
    /// Maximum transfer amount per period (0 = unlimited)
    pub max_transfer_amount: u128,
    /// Period for transfer quota (in blocks)
    pub quota_period: u32,
    /// Minimum hold period before transfer allowed (in blocks)
    pub hold_period: u32,
    /// Enable risk level checking
    pub check_risk_level: bool,
    /// Maximum allowed risk level (0-100)
    pub max_allowed_risk_level: u8,
}

/// User transfer quota tracking
#[derive(
    Debug, Clone, Copy, scale::Encode, scale::Decode, ink::storage::traits::StorageLayout,
)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub struct UserTransferQuota {
    /// Total amount transferred in current period
    pub amount_transferred: u128,
    /// Block when the current period started
    pub period_start_block: u32,
    /// Block when the user first acquired this token
    pub acquisition_block: u32,
}

/// KYC transfer event for audit logging
#[derive(
    Debug, Clone, PartialEq, scale::Encode, scale::Decode, ink::storage::traits::StorageLayout,
)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub struct KYCTransferEvent {
    pub from: AccountId,
    pub to: AccountId,
    pub token_id: TokenId,
    pub amount: u128,
    pub timestamp: u64,
    pub from_verification_level: KYCVerificationLevel,
    pub to_verification_level: KYCVerificationLevel,
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
pub struct VestingSchedule {
    pub role: VestingRole,
    pub total_amount: u128,
    pub claimed_amount: u128,
    pub start_time: u64,
    pub cliff_duration: u64,
    pub vesting_duration: u64,
}


/// Snapshot for governance voting (Issue #194)
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
pub struct Snapshot {
    pub id: u64,
    pub token_id: TokenId,
    pub created_at: u64,
    pub total_supply_at_snapshot: u128,
    pub description: String, // Optional description of why snapshot was taken
}


/// Lock period for staking shares (Issue #197)
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
    /// Returns the duration in blocks for this lock period
    /// Assuming ~6 second block time: 1 day ≈ 14,400 blocks
    pub fn duration_blocks(&self) -> u64 {
        match self {
            LockPeriod::Flexible => 0,
            LockPeriod::ThirtyDays => 30 * 14_400,
            LockPeriod::NinetyDays => 90 * 14_400,
            LockPeriod::OneYear => 365 * 14_400,
        }
    }

    /// Returns the reward multiplier for this lock period (in percentage)
    pub fn multiplier(&self) -> u128 {
        match self {
            LockPeriod::Flexible => 100,      // 1x
            LockPeriod::ThirtyDays => 110,    // 1.1x
            LockPeriod::NinetyDays => 125,    // 1.25x
            LockPeriod::OneYear => 150,       // 1.5x
        }
    }
}

/// Staking information for fractional shares (Issue #197)
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
    pub lock_period: LockPeriod,
    pub reward_debt: u128,
}
