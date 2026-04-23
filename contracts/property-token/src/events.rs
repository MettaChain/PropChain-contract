// Event definitions for the property token contract (Issue #101 - extracted from lib.rs)

// =========================================================================
// ERC-721/1155 Standard Events
// =========================================================================

#[ink(event)]
pub struct Transfer {
    #[ink(topic)]
    pub from: Option<AccountId>,
    #[ink(topic)]
    pub to: Option<AccountId>,
    #[ink(topic)]
    pub id: TokenId,
}

#[ink(event)]
pub struct Approval {
    #[ink(topic)]
    pub owner: AccountId,
    #[ink(topic)]
    pub spender: AccountId,
    #[ink(topic)]
    pub id: TokenId,
}

#[ink(event)]
pub struct ApprovalForAll {
    #[ink(topic)]
    pub owner: AccountId,
    #[ink(topic)]
    pub operator: AccountId,
    pub approved: bool,
}

// =========================================================================
// Property Events
// =========================================================================

#[ink(event)]
pub struct PropertyTokenMinted {
    #[ink(topic)]
    pub token_id: TokenId,
    #[ink(topic)]
    pub property_id: u64,
    #[ink(topic)]
    pub owner: AccountId,
}

#[ink(event)]
pub struct LegalDocumentAttached {
    #[ink(topic)]
    pub token_id: TokenId,
    #[ink(topic)]
    pub document_hash: Hash,
    #[ink(topic)]
    pub document_type: String,
}

#[ink(event)]
pub struct ComplianceVerified {
    #[ink(topic)]
    pub token_id: TokenId,
    #[ink(topic)]
    pub verified: bool,
    #[ink(topic)]
    pub verifier: AccountId,
}

// =========================================================================
// Bridge Events
// =========================================================================

#[ink(event)]
pub struct TokenBridged {
    #[ink(topic)]
    pub token_id: TokenId,
    #[ink(topic)]
    pub destination_chain: ChainId,
    #[ink(topic)]
    pub recipient: AccountId,
    pub bridge_request_id: u64,
}

#[ink(event)]
pub struct BridgeRequestCreated {
    #[ink(topic)]
    pub request_id: u64,
    #[ink(topic)]
    pub token_id: TokenId,
    #[ink(topic)]
    pub source_chain: ChainId,
    #[ink(topic)]
    pub destination_chain: ChainId,
    #[ink(topic)]
    pub requester: AccountId,
}

#[ink(event)]
pub struct BridgeRequestSigned {
    #[ink(topic)]
    pub request_id: u64,
    #[ink(topic)]
    pub signer: AccountId,
    pub signatures_collected: u8,
    pub signatures_required: u8,
}

#[ink(event)]
pub struct BridgeExecuted {
    #[ink(topic)]
    pub request_id: u64,
    #[ink(topic)]
    pub token_id: TokenId,
    #[ink(topic)]
    pub transaction_hash: Hash,
}

#[ink(event)]
pub struct BridgeFailed {
    #[ink(topic)]
    pub request_id: u64,
    #[ink(topic)]
    pub token_id: TokenId,
    pub error: String,
}

#[ink(event)]
pub struct BridgeRecovered {
    #[ink(topic)]
    pub request_id: u64,
    #[ink(topic)]
    pub recovery_action: RecoveryAction,
}

// =========================================================================
// Fractional / Dividend Events
// =========================================================================

#[ink(event)]
pub struct SharesIssued {
    #[ink(topic)]
    pub token_id: TokenId,
    #[ink(topic)]
    pub to: AccountId,
    pub amount: u128,
}

#[ink(event)]
pub struct SharesRedeemed {
    #[ink(topic)]
    pub token_id: TokenId,
    #[ink(topic)]
    pub from: AccountId,
    pub amount: u128,
}

#[ink(event)]
pub struct DividendsDeposited {
    #[ink(topic)]
    pub token_id: TokenId,
    pub amount: u128,
    pub per_share: u128,
}

#[ink(event)]
pub struct DividendsWithdrawn {
    #[ink(topic)]
    pub token_id: TokenId,
    #[ink(topic)]
    pub account: AccountId,
    pub amount: u128,
}



// =========================================================================
// Metadata Events
// =========================================================================

#[ink(event)]
pub struct MetadataUpdated {
    #[ink(topic)]
    pub token_id: TokenId,
    #[ink(topic)]
    pub updated_by: AccountId,
}

#[ink(event)]
pub struct TokenURIUpdated {
    #[ink(topic)]
    pub token_id: TokenId,
    #[ink(topic)]
    pub updated_by: AccountId,
    pub new_uri: String,
}

// =========================================================================
// Governance Events
// =========================================================================



#[ink(event)]
pub struct ProposalCreated {
    #[ink(topic)]
    pub token_id: TokenId,
    #[ink(topic)]
    pub proposal_id: u64,
    pub quorum: u128,
}

#[ink(event)]
pub struct Voted {
    #[ink(topic)]
    pub token_id: TokenId,
    #[ink(topic)]
    pub proposal_id: u64,
    #[ink(topic)]
    pub voter: AccountId,
    pub support: bool,
    pub weight: u128,
}

#[ink(event)]
pub struct ProposalExecuted {
    #[ink(topic)]
    pub token_id: TokenId,
    #[ink(topic)]
    pub proposal_id: u64,
    pub passed: bool,
}

// =========================================================================
// Marketplace Events
// =========================================================================

#[ink(event)]
pub struct AskPlaced {
    #[ink(topic)]
    pub token_id: TokenId,
    #[ink(topic)]
    pub seller: AccountId,
    pub price_per_share: u128,
    pub amount: u128,
}

#[ink(event)]
pub struct AskCancelled {
    #[ink(topic)]
    pub token_id: TokenId,
    #[ink(topic)]
    pub seller: AccountId,
}

#[ink(event)]
pub struct SharesPurchased {
    #[ink(topic)]
    pub token_id: TokenId,
    #[ink(topic)]
    pub seller: AccountId,
    #[ink(topic)]
    pub buyer: AccountId,
    pub amount: u128,
    pub price_per_share: u128,
}

// =========================================================================
// Management Events
// =========================================================================

#[ink(event)]
pub struct PropertyManagementContractSet {
    #[ink(topic)]
    pub contract: Option<AccountId>,
}

#[ink(event)]
pub struct ManagementAgentAssigned {
    #[ink(topic)]
    pub token_id: TokenId,
    #[ink(topic)]
    pub agent: AccountId,
}

#[ink(event)]
pub struct ManagementAgentCleared {
    #[ink(topic)]
    pub token_id: TokenId,
}

// =========================================================================
// Vesting Events
// =========================================================================

#[ink(event)]
pub struct VestingScheduleCreated {
    #[ink(topic)]
    pub token_id: TokenId,
    #[ink(topic)]
    pub account: AccountId,
    pub role: crate::property_token::VestingRole,
    pub total_amount: u128,
    pub start_time: u64,
    pub cliff_duration: u64,
    pub vesting_duration: u64,
}

#[ink(event)]
pub struct VestedTokensClaimed {
    #[ink(topic)]
    pub token_id: TokenId,
    #[ink(topic)]
    pub account: AccountId,
    pub amount: u128,
}

