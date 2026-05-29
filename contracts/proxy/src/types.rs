// Data types for the proxy contract (Issue #101 - extracted from lib.rs)

/// Version information for deployed contract implementations
#[derive(
    Debug, Clone, PartialEq, scale::Encode, scale::Decode, ink::storage::traits::StorageLayout,
)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub struct VersionInfo {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
    pub code_hash: Hash,
    pub deployed_at_block: u32,
    pub deployed_at: u64,
    pub description: String,
    pub deployed_by: AccountId,
}

/// Upgrade proposal requiring governance approval
#[derive(
    Debug, Clone, PartialEq, scale::Encode, scale::Decode, ink::storage::traits::StorageLayout,
)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub struct UpgradeProposal {
    pub id: u64,
    pub new_code_hash: Hash,
    pub version: VersionInfo,
    pub proposer: AccountId,
    pub created_at_block: u32,
    pub created_at: u64,
    pub timelock_until_block: u32,
    pub approvals: Vec<AccountId>,
    pub required_approvals: u32,
    pub cancelled: bool,
    pub executed: bool,
    pub migration_notes: String,
}

/// Migration state tracking
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
pub enum MigrationState {
    None,
    Proposed,
    Approved,
    InProgress,
    Completed,
    RolledBack,
}
