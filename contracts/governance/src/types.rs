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
}
