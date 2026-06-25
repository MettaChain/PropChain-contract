// Data types for the governance contract (Issue #101 - extracted from lib.rs)

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
pub enum GovernanceAction {
    ModifyProperty,
    SaleApproval,
    ChangeThreshold,
    AddSigner,
    RemoveSigner,
    EmergencyOverride,
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
    Active,
    Approved,
    Executed,
    Rejected,
    Cancelled,
    Expired,
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
pub struct GovernanceProposal {
    pub id: u64,
    pub proposer: AccountId,
    pub description_hash: Hash,
    pub action_type: GovernanceAction,
    pub target: Option<AccountId>,
    pub threshold: u32,
    pub votes_for: u32,
    pub votes_against: u32,
    pub status: ProposalStatus,
    pub created_at: u64,
    pub executed_at: u64,
    pub timelock_until: u64,
    pub is_emergency: bool,
}

#[derive(
    Debug,
    Clone,
    PartialEq,
    Eq,
    scale::Encode,
    scale::Decode,
)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub struct GovernanceAnalytics {
    pub total_proposals: u64,
    pub executed_proposals: u64,
    pub rejected_proposals: u64,
    pub cancelled_proposals: u64,
    pub active_proposals: u64,
    pub avg_participation_bps: u32,
}

// ── Discussion Forum Types (Issue #233) ─────────────────────────────────────

/// A single comment or discussion entry on a governance proposal.
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
pub struct DiscussionComment {
    pub discussion_id: u64,
    pub author: AccountId,
    pub content_hash: Hash,
    pub parent_id: Option<u64>,
    pub created_at: u64,
}

// ── Delegation Types (Issue #231) ───────────────────────────────────────────

/// Delegation info for a governance signer.
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
pub struct DelegationInfo {
    pub delegator: AccountId,
    pub delegate: AccountId,
    pub delegated_at: u64,
    pub expires_at: Option<u64>,
}

// ── Quadratic Voting Types (Issue #229) ─────────────────────────────────────

/// Quadratic voting configuration for a proposal.
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
pub struct QuadraticVote {
    pub voter: AccountId,
    pub proposal_id: u64,
    pub support: bool,
    pub voting_power: u32,
    pub credits_spent: u32,
}
