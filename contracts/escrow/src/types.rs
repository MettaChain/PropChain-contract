// Data types for the escrow contract (Issue #101 - extracted from lib.rs)
use propchain_traits::Jurisdiction;

#[derive(Debug, Clone, PartialEq, Eq, scale::Encode, scale::Decode)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
#[derive(ink::storage::traits::StorageLayout)]
pub enum EscrowStatus {
    Created,
    Funded,
    Active,
    Released,
    Refunded,
    Disputed,
    Cancelled,
}

#[derive(Debug, Clone, PartialEq, Eq, scale::Encode, scale::Decode)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
#[derive(ink::storage::traits::StorageLayout)]
pub enum ApprovalType {
    Release,
    Refund,
    EmergencyOverride,
}

#[derive(Debug, Clone, PartialEq, Eq, scale::Encode, scale::Decode)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
#[derive(ink::storage::traits::StorageLayout)]
pub struct EscrowData {
    pub id: u64,
    pub property_id: u64,
    pub buyer: AccountId,
    pub seller: AccountId,
    pub amount: u128,
    pub deposited_amount: u128,
    pub status: EscrowStatus,
    pub created_at: u64,
    pub release_time_lock: Option<u64>,
    pub participants: Vec<AccountId>,
    pub jurisdiction: Jurisdiction,
}

#[derive(Debug, Clone, PartialEq, Eq, scale::Encode, scale::Decode)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
#[derive(ink::storage::traits::StorageLayout)]
pub struct MultiSigConfig {
    pub required_signatures: u8,
    pub signers: Vec<AccountId>,
}

#[derive(Debug, Clone, PartialEq, Eq, scale::Encode, scale::Decode)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
#[derive(ink::storage::traits::StorageLayout)]
pub struct DocumentHash {
    pub hash: Hash,
    pub document_type: String,
    pub uploaded_by: AccountId,
    pub uploaded_at: u64,
    pub verified: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, scale::Encode, scale::Decode)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
#[derive(ink::storage::traits::StorageLayout)]
pub struct Condition {
    pub id: u64,
    pub description: String,
    pub met: bool,
    pub verified_by: Option<AccountId>,
    pub verified_at: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, scale::Encode, scale::Decode)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
#[derive(ink::storage::traits::StorageLayout)]
pub struct DisputeInfo {
    pub escrow_id: u64,
    pub raised_by: AccountId,
    pub reason: String,
    pub raised_at: u64,
    pub resolved: bool,
    pub resolution: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, scale::Encode, scale::Decode)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
#[derive(ink::storage::traits::StorageLayout)]
pub struct AuditEntry {
    pub timestamp: u64,
    pub actor: AccountId,
    pub action: String,
    pub details: String,
}

pub type SignatureKey = (u64, ApprovalType, AccountId);

// ── Multi-Step Approval Types ────────────────────────────────────────────────

/// Tier of approval required based on transfer amount.
#[derive(Debug, Clone, PartialEq, Eq, scale::Encode, scale::Decode)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
#[derive(ink::storage::traits::StorageLayout)]
pub enum TransferApprovalTier {
    /// Amount < LARGE_TRANSFER_THRESHOLD: no extra approval needed.
    Standard,
    /// Amount >= LARGE_TRANSFER_THRESHOLD: requires 2 approvals.
    Large,
    /// Amount >= VERY_LARGE_TRANSFER_THRESHOLD: requires 3 approvals.
    VeryLarge,
}

/// Status of a pending large-transfer approval request.
#[derive(Debug, Clone, PartialEq, Eq, scale::Encode, scale::Decode)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
#[derive(ink::storage::traits::StorageLayout)]
pub enum LargeTransferStatus {
    /// Awaiting the required number of approvals.
    Pending,
    /// All required approvals collected; ready to execute.
    Approved,
    /// Transfer has been executed.
    Executed,
    /// Request was cancelled by the initiator or admin.
    Cancelled,
    /// Request expired before enough approvals were collected.
    Expired,
}

/// A pending large-transfer approval request.
///
/// Created automatically when `release_funds` or `refund_funds` is called
/// on an escrow whose `deposited_amount` exceeds the large-transfer threshold.
/// Authorised signers call `approve_large_transfer` to collect approvals.
/// Once the threshold is met, `execute_large_transfer` finalises the transfer.
#[derive(Debug, Clone, PartialEq, Eq, scale::Encode, scale::Decode)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
#[derive(ink::storage::traits::StorageLayout)]
pub struct LargeTransferRequest {
    /// Unique identifier for this approval request.
    pub request_id: u64,
    /// The escrow this transfer belongs to.
    pub escrow_id: u64,
    /// Whether this is a release (to seller) or refund (to buyer).
    pub approval_type: ApprovalType,
    /// Amount to be transferred.
    pub amount: u128,
    /// Recipient of the funds.
    pub recipient: AccountId,
    /// Approval tier (Large or VeryLarge).
    pub tier: TransferApprovalTier,
    /// Number of approvals required.
    pub required_approvals: u8,
    /// Accounts that have approved so far.
    pub approvals: Vec<AccountId>,
    /// Account that initiated this request.
    pub initiated_by: AccountId,
    /// Block number when this request was created.
    pub created_at_block: u64,
    /// Block number after which this request expires.
    pub expires_at_block: u64,
    /// Current status.
    pub status: LargeTransferStatus,
}
