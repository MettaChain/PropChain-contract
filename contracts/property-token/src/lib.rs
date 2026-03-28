#![cfg_attr(not(feature = "std"), no_std)]
#![allow(
    unexpected_cfgs,
    clippy::type_complexity,
    clippy::needless_borrows_for_generic_args
)]

use ink::prelude::string::String;
use ink::storage::Mapping;
use propchain_traits::*;
#[cfg(not(feature = "std"))]
use scale_info::prelude::vec::Vec;

#[ink::contract]
mod property_token {
    use super::*;

    /// Error types for the property token contract
    #[derive(Debug, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub enum Error {
        // Standard ERC errors
        /// Token does not exist
        TokenNotFound,
        /// Caller is not authorized
        Unauthorized,
        // Property-specific errors
        /// Property does not exist
        PropertyNotFound,
        /// Metadata is invalid or malformed
        InvalidMetadata,
        /// Document does not exist
        DocumentNotFound,
        /// Compliance check failed
        ComplianceFailed,
        // Cross-chain bridge errors
        /// Bridge functionality not supported
        BridgeNotSupported,
        /// Invalid chain ID
        InvalidChain,
        /// Token is locked in bridge
        BridgeLocked,
        /// Insufficient signatures for bridge operation
        InsufficientSignatures,
        /// Bridge request has expired
        RequestExpired,
        /// Invalid bridge request
        InvalidRequest,
        /// Bridge operations are paused
        BridgePaused,
        /// Gas limit exceeded
        GasLimitExceeded,
        /// Metadata is corrupted
        MetadataCorruption,
        /// Invalid bridge operator
        InvalidBridgeOperator,
        /// Duplicate bridge request
        DuplicateBridgeRequest,
        /// Bridge operation timed out
        BridgeTimeout,
        /// Already signed this request
        AlreadySigned,
        /// Insufficient balance
        InsufficientBalance,
        /// Invalid amount
        InvalidAmount,
        /// Proposal not found
        ProposalNotFound,
        /// Proposal is closed
        ProposalClosed,
        /// Ask not found
        AskNotFound,
        /// Input batch exceeds maximum allowed size
        BatchSizeExceeded,
    }

    impl core::fmt::Display for Error {
        fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
            match self {
                Error::TokenNotFound => write!(f, "Token does not exist"),
                Error::Unauthorized => write!(f, "Caller is not authorized"),
                Error::PropertyNotFound => write!(f, "Property does not exist"),
                Error::InvalidMetadata => write!(f, "Metadata is invalid or malformed"),
                Error::DocumentNotFound => write!(f, "Document does not exist"),
                Error::ComplianceFailed => write!(f, "Compliance check failed"),
                Error::BridgeNotSupported => write!(f, "Bridge functionality not supported"),
                Error::InvalidChain => write!(f, "Invalid chain ID"),
                Error::BridgeLocked => write!(f, "Token is locked in bridge"),
                Error::InsufficientSignatures => {
                    write!(f, "Insufficient signatures for bridge operation")
                }
                Error::RequestExpired => write!(f, "Bridge request has expired"),
                Error::InvalidRequest => write!(f, "Invalid bridge request"),
                Error::BridgePaused => write!(f, "Bridge operations are paused"),
                Error::GasLimitExceeded => write!(f, "Gas limit exceeded"),
                Error::MetadataCorruption => write!(f, "Metadata is corrupted"),
                Error::InvalidBridgeOperator => write!(f, "Invalid bridge operator"),
                Error::DuplicateBridgeRequest => write!(f, "Duplicate bridge request"),
                Error::BridgeTimeout => write!(f, "Bridge operation timed out"),
                Error::AlreadySigned => write!(f, "Already signed this request"),
                Error::InsufficientBalance => write!(f, "Insufficient balance"),
                Error::InvalidAmount => write!(f, "Invalid amount"),
                Error::ProposalNotFound => write!(f, "Proposal not found"),
                Error::ProposalClosed => write!(f, "Proposal is closed"),
                Error::AskNotFound => write!(f, "Ask not found"),
                Error::BatchSizeExceeded => write!(f, "Input batch exceeds maximum allowed size"),
            }
        }
    }

    impl ContractError for Error {
        fn error_code(&self) -> u32 {
            match self {
                Error::TokenNotFound => property_token_codes::TOKEN_NOT_FOUND,
                Error::Unauthorized => property_token_codes::UNAUTHORIZED_TRANSFER,
                Error::PropertyNotFound => property_token_codes::PROPERTY_NOT_FOUND,
                Error::InvalidMetadata => property_token_codes::INVALID_METADATA,
                Error::DocumentNotFound => property_token_codes::DOCUMENT_NOT_FOUND,
                Error::ComplianceFailed => property_token_codes::COMPLIANCE_FAILED,
                Error::BridgeNotSupported => property_token_codes::BRIDGE_NOT_SUPPORTED,
                Error::InvalidChain => property_token_codes::INVALID_CHAIN,
                Error::BridgeLocked => property_token_codes::BRIDGE_LOCKED,
                Error::InsufficientSignatures => property_token_codes::INSUFFICIENT_SIGNATURES,
                Error::RequestExpired => property_token_codes::REQUEST_EXPIRED,
                Error::InvalidRequest => property_token_codes::INVALID_REQUEST,
                Error::BridgePaused => property_token_codes::BRIDGE_PAUSED,
                Error::GasLimitExceeded => property_token_codes::GAS_LIMIT_EXCEEDED,
                Error::MetadataCorruption => property_token_codes::METADATA_CORRUPTION,
                Error::InvalidBridgeOperator => property_token_codes::INVALID_BRIDGE_OPERATOR,
                Error::DuplicateBridgeRequest => property_token_codes::DUPLICATE_BRIDGE_REQUEST,
                Error::BridgeTimeout => property_token_codes::BRIDGE_TIMEOUT,
                Error::AlreadySigned => property_token_codes::ALREADY_SIGNED,
                Error::InsufficientBalance => property_token_codes::INSUFFICIENT_BALANCE,
                Error::InvalidAmount => property_token_codes::INVALID_AMOUNT,
                Error::ProposalNotFound => property_token_codes::PROPOSAL_NOT_FOUND,
                Error::ProposalClosed => property_token_codes::PROPOSAL_CLOSED,
                Error::AskNotFound => property_token_codes::ASK_NOT_FOUND,
                Error::BatchSizeExceeded => property_token_codes::BATCH_SIZE_EXCEEDED,
            }
        }

        fn error_description(&self) -> &'static str {
            match self {
                Error::TokenNotFound => "The specified token does not exist",
                Error::Unauthorized => "Caller does not have permission to perform this operation",
                Error::PropertyNotFound => "The specified property does not exist",
                Error::InvalidMetadata => "The provided metadata is invalid or malformed",
                Error::DocumentNotFound => "The requested document does not exist",
                Error::ComplianceFailed => "The operation failed compliance verification",
                Error::BridgeNotSupported => "Cross-chain bridging is not supported for this token",
                Error::InvalidChain => "The destination chain ID is invalid",
                Error::BridgeLocked => "The token is currently locked in a bridge operation",
                Error::InsufficientSignatures => {
                    "Not enough signatures collected for bridge operation"
                }
                Error::RequestExpired => {
                    "The bridge request has expired and can no longer be executed"
                }
                Error::InvalidRequest => "The bridge request is invalid or malformed",
                Error::BridgePaused => "Bridge operations are temporarily paused",
                Error::GasLimitExceeded => "The operation exceeded the gas limit",
                Error::MetadataCorruption => "The token metadata has been corrupted",
                Error::InvalidBridgeOperator => "The bridge operator is not authorized",
                Error::DuplicateBridgeRequest => {
                    "A bridge request with these parameters already exists"
                }
                Error::BridgeTimeout => "The bridge operation timed out",
                Error::AlreadySigned => "You have already signed this bridge request",
                Error::InsufficientBalance => "Account has insufficient balance",
                Error::InvalidAmount => "The amount is invalid or out of range",
                Error::ProposalNotFound => "The governance proposal does not exist",
                Error::ProposalClosed => "The governance proposal is closed for voting",
                Error::AskNotFound => "The sell ask does not exist",
                Error::BatchSizeExceeded => {
                    "The input batch exceeds the maximum allowed size"
                }
            }
        }

        fn error_category(&self) -> ErrorCategory {
            ErrorCategory::PropertyToken
        }
    }

    /// Property Token contract that maintains compatibility with ERC-721 and ERC-1155
    /// while adding real estate-specific features and cross-chain support
    #[ink(storage)]
    pub struct PropertyToken {
        // ERC-721 standard mappings
        token_owner: Mapping<TokenId, AccountId>,
        owner_token_count: Mapping<AccountId, u32>,
        token_approvals: Mapping<TokenId, AccountId>,
        operator_approvals: Mapping<(AccountId, AccountId), bool>,

        // ERC-1155 batch operation support
        balances: Mapping<(AccountId, TokenId), u128>,
        operators: Mapping<(AccountId, AccountId), bool>,

        // Property-specific mappings
        token_properties: Mapping<TokenId, PropertyInfo>,
        property_tokens: Mapping<u64, TokenId>, // property_id to token_id mapping
        ownership_history_count: Mapping<TokenId, u32>,
        ownership_history_items: Mapping<(TokenId, u32), OwnershipTransfer>,
        compliance_flags: Mapping<TokenId, ComplianceInfo>,
        legal_documents_count: Mapping<TokenId, u32>,
        legal_documents_items: Mapping<(TokenId, u32), DocumentInfo>,

        // Cross-chain bridge mappings
        bridged_tokens: Mapping<(ChainId, TokenId), BridgedTokenInfo>,
        bridge_operators: Vec<AccountId>,
        bridge_requests: Mapping<u64, MultisigBridgeRequest>,
        bridge_transactions: Mapping<AccountId, Vec<BridgeTransaction>>,
        bridge_config: BridgeConfig,
        verified_bridge_hashes: Mapping<Hash, bool>,
        bridge_request_counter: u64,

        // Standard counters
        total_supply: u64,
        token_counter: u64,
        admin: AccountId,

        // Error logging and monitoring
        error_counts: Mapping<(AccountId, String), u64>,
        error_rates: Mapping<String, (u64, u64)>, // (count, window_start)
        recent_errors: Mapping<u64, ErrorLogEntry>,
        error_log_counter: u64,

        total_shares: Mapping<TokenId, u128>,
        dividends_per_share: Mapping<TokenId, u128>,
        dividend_credit: Mapping<(AccountId, TokenId), u128>,
        dividend_balance: Mapping<(AccountId, TokenId), u128>,
        proposal_counter: Mapping<TokenId, u64>,
        proposals: Mapping<(TokenId, u64), Proposal>,
        votes_cast: Mapping<(TokenId, u64, AccountId), bool>,
        asks: Mapping<(TokenId, AccountId), Ask>,
        escrowed_shares: Mapping<(TokenId, AccountId), u128>,
        last_trade_price: Mapping<TokenId, u128>,
        compliance_registry: Option<AccountId>,
        tax_records: Mapping<(AccountId, TokenId), TaxRecord>,
        max_batch_size: u32,
        /// Optional property-management contract for operational workflows
        property_management_contract: Option<AccountId>,
        /// On-chain management agent per property token (tokenized property)
        management_agent: Mapping<TokenId, AccountId>,
    }

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

    // Events for tracking property token operations
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

    impl PropertyToken {
        /// Creates a new PropertyToken contract
        #[ink(constructor)]
        pub fn new() -> Self {
            let caller = Self::env().caller();

            // Initialize default bridge configuration
            let bridge_config = BridgeConfig {
                supported_chains: vec![1, 2, 3], // Default supported chains
                min_signatures_required: 2,
                max_signatures_required: 5,
                default_timeout_blocks: 100,
                gas_limit_per_bridge: 500000,
                emergency_pause: false,
                metadata_preservation: true,
            };

            Self {
                // ERC-721 standard mappings
                token_owner: Mapping::default(),
                owner_token_count: Mapping::default(),
                token_approvals: Mapping::default(),
                operator_approvals: Mapping::default(),

                // ERC-1155 batch operation support
                balances: Mapping::default(),
                operators: Mapping::default(),

                // Property-specific mappings
                token_properties: Mapping::default(),
                property_tokens: Mapping::default(),
                ownership_history_count: Mapping::default(),
                ownership_history_items: Mapping::default(),
                compliance_flags: Mapping::default(),
                legal_documents_count: Mapping::default(),
                legal_documents_items: Mapping::default(),

                // Cross-chain bridge mappings
                bridged_tokens: Mapping::default(),
                bridge_operators: vec![caller],
                bridge_requests: Mapping::default(),
                bridge_transactions: Mapping::default(),
                bridge_config,
                verified_bridge_hashes: Mapping::default(),
                bridge_request_counter: 0,

                // Standard counters
                total_supply: 0,
                token_counter: 0,
                admin: caller,

                // Error logging and monitoring
                error_counts: Mapping::default(),
                error_rates: Mapping::default(),
                recent_errors: Mapping::default(),
                error_log_counter: 0,

                total_shares: Mapping::default(),
                dividends_per_share: Mapping::default(),
                dividend_credit: Mapping::default(),
                dividend_balance: Mapping::default(),
                proposal_counter: Mapping::default(),
                proposals: Mapping::default(),
                votes_cast: Mapping::default(),
                asks: Mapping::default(),
                escrowed_shares: Mapping::default(),
                last_trade_price: Mapping::default(),
                compliance_registry: None,
                tax_records: Mapping::default(),
                max_batch_size: 50,
                property_management_contract: None,
                management_agent: Mapping::default(),
            }
        }

        /// ERC-721: Returns the balance of tokens owned by an account
        #[ink(message)]
        pub fn balance_of(&self, owner: AccountId) -> u32 {
            self.owner_token_count.get(owner).unwrap_or(0)
        }

        /// ERC-721: Returns the owner of a token
        #[ink(message)]
        pub fn owner_of(&self, token_id: TokenId) -> Option<AccountId> {
            self.token_owner.get(token_id)
        }

        /// ERC-721: Transfers a token from one account to another
        #[ink(message)]
        pub fn transfer_from(
            &mut self,
            from: AccountId,
            to: AccountId,
            token_id: TokenId,
        ) -> Result<(), Error> {
            let caller = self.env().caller();

            // Check if caller is authorized to transfer
            let token_owner = self.token_owner.get(token_id).ok_or_else(|| {
                let caller = self.env().caller();
                self.log_error(
                    caller,
                    "TOKEN_NOT_FOUND".to_string(),
                    format!("Token ID {} does not exist", token_id),
                    vec![
                        ("token_id".to_string(), token_id.to_string()),
                        ("operation".to_string(), "transfer_from".to_string()),
                    ],
                );
                Error::TokenNotFound
            })?;
            if token_owner != from {
                let caller = self.env().caller();
                self.log_error(
                    caller,
                    "UNAUTHORIZED".to_string(),
                    format!("Caller is not authorized to transfer token {}", token_id),
                    vec![
                        ("token_id".to_string(), token_id.to_string()),
                        ("caller".to_string(), format!("{:?}", caller)),
                        ("owner".to_string(), format!("{:?}", token_owner)),
                    ],
                );
                return Err(Error::Unauthorized);
            }

            if caller != from
                && Some(caller) != self.token_approvals.get(token_id)
                && !self.is_approved_for_all(from, caller)
            {
                return Err(Error::Unauthorized);
            }

            // Perform the transfer
            self.remove_token_from_owner(from, token_id)?;
            self.add_token_to_owner(to, token_id)?;

            // Clear approvals
            self.token_approvals.remove(token_id);

            // Update ownership history
            self.update_ownership_history(token_id, from, to)?;

            self.env().emit_event(Transfer {
                from: Some(from),
                to: Some(to),
                id: token_id,
            });

            Ok(())
        }

        /// ERC-721: Approves an account to transfer a specific token
        #[ink(message)]
        pub fn approve(&mut self, to: AccountId, token_id: TokenId) -> Result<(), Error> {
            let caller = self.env().caller();
            let token_owner = self.token_owner.get(token_id).ok_or_else(|| {
                self.log_error(
                    caller,
                    "TOKEN_NOT_FOUND".to_string(),
                    format!("Token ID {} does not exist", token_id),
                    vec![
                        ("token_id".to_string(), token_id.to_string()),
                        ("operation".to_string(), "approve".to_string()),
                    ],
                );
                Error::TokenNotFound
            })?;

            if token_owner != caller && !self.is_approved_for_all(token_owner, caller) {
                self.log_error(
                    caller,
                    "UNAUTHORIZED".to_string(),
                    format!("Caller is not authorized to approve token {}", token_id),
                    vec![
                        ("token_id".to_string(), token_id.to_string()),
                        ("caller".to_string(), format!("{:?}", caller)),
                        ("owner".to_string(), format!("{:?}", token_owner)),
                    ],
                );
                return Err(Error::Unauthorized);
            }

            self.token_approvals.insert(token_id, &to);

            self.env().emit_event(Approval {
                owner: token_owner,
                spender: to,
                id: token_id,
            });

            Ok(())
        }

        /// ERC-721: Sets or unsets an operator for an owner
        #[ink(message)]
        pub fn set_approval_for_all(
            &mut self,
            operator: AccountId,
            approved: bool,
        ) -> Result<(), Error> {
            let caller = self.env().caller();
            self.operator_approvals
                .insert((&caller, &operator), &approved);

            self.env().emit_event(ApprovalForAll {
                owner: caller,
                operator,
                approved,
            });

            Ok(())
        }

        /// ERC-721: Gets the approved account for a token
        #[ink(message)]
        pub fn get_approved(&self, token_id: TokenId) -> Option<AccountId> {
            self.token_approvals.get(token_id)
        }

        /// ERC-721: Checks if an operator is approved for an owner
        #[ink(message)]
        pub fn is_approved_for_all(&self, owner: AccountId, operator: AccountId) -> bool {
            self.operator_approvals
                .get((&owner, &operator))
                .unwrap_or(false)
        }

        /// ERC-1155: Returns the balance of tokens for an account
        #[ink(message)]
        pub fn balance_of_batch(&self, accounts: Vec<AccountId>, ids: Vec<TokenId>) -> Vec<u128> {
            if accounts.len() > self.max_batch_size as usize {
                return Vec::new();
            }
            let mut balances = Vec::new();
            for i in 0..accounts.len() {
                if i < ids.len() {
                    let balance = self.balances.get((&accounts[i], &ids[i])).unwrap_or(0);
                    balances.push(balance);
                } else {
                    balances.push(0);
                }
            }
            balances
        }

        /// ERC-1155: Safely transfers tokens from one account to another
        #[ink(message)]
        pub fn safe_batch_transfer_from(
            &mut self,
            from: AccountId,
            to: AccountId,
            ids: Vec<TokenId>,
            amounts: Vec<u128>,
            _data: Vec<u8>,
        ) -> Result<(), Error> {
            let caller = self.env().caller();

            if from != caller && !self.is_approved_for_all(from, caller) {
                return Err(Error::Unauthorized);
            }

            if ids.len() > self.max_batch_size as usize {
                return Err(Error::BatchSizeExceeded);
            }

            // Verify lengths match
            if ids.len() != amounts.len() {
                return Err(Error::Unauthorized); // Using this as a general error for mismatched arrays
            }

            // Transfer each token
            for i in 0..ids.len() {
                let token_id = ids[i];
                let amount = amounts[i];

                // Check balance
                let from_balance = self.balances.get((&from, &token_id)).unwrap_or(0);
                if from_balance < amount {
                    return Err(Error::Unauthorized);
                }

                // Update balances
                self.balances
                    .insert((&from, &token_id), &(from_balance - amount));
                let to_balance = self.balances.get((&to, &token_id)).unwrap_or(0);
                self.balances
                    .insert((&to, &token_id), &(to_balance + amount));
            }

            // Emit transfer events for each token
            for id in &ids {
                self.env().emit_event(Transfer {
                    from: Some(from),
                    to: Some(to),
                    id: *id,
                });
            }

            Ok(())
        }

        /// ERC-1155: Returns the URI for a token
        #[ink(message)]
        pub fn uri(&self, token_id: TokenId) -> Option<String> {
            // Return a standard URI format for the token metadata
            let _property_info = self.token_properties.get(token_id)?;
            Some(format!(
                "ipfs://property/{:?}/{}/metadata.json",
                self.env().account_id(),
                token_id
            ))
        }

        /// Sets the compliance registry contract address (admin only).
        ///
        /// When set, compliance checks are delegated to this external contract
        /// for share transfers and purchases.
        ///
        /// # Arguments
        ///
        /// * `registry` - The account ID of the compliance registry contract
        ///
        /// # Returns
        ///
        /// Returns `Result<(), Error>` indicating success or failure
        #[ink(message)]
        pub fn set_compliance_registry(&mut self, registry: AccountId) -> Result<(), Error> {
            let caller = self.env().caller();
            if caller != self.admin {
                return Err(Error::Unauthorized);
            }
            self.compliance_registry = Some(registry);
            Ok(())
        }

        /// Links the canonical property-management contract (admin).
        #[ink(message)]
        pub fn set_property_management_contract(
            &mut self,
            management: Option<AccountId>,
        ) -> Result<(), Error> {
            if self.env().caller() != self.admin {
                return Err(Error::Unauthorized);
            }
            self.property_management_contract = management;
            self.env().emit_event(PropertyManagementContractSet {
                contract: management,
            });
            Ok(())
        }

        /// Returns the linked property-management contract address, if set.
        #[ink(message)]
        pub fn get_property_management_contract(&self) -> Option<AccountId> {
            self.property_management_contract
        }

        /// Assigns a management agent for rent, maintenance, and tenant workflows for this token.
        #[ink(message)]
        pub fn assign_management_agent(
            &mut self,
            token_id: TokenId,
            agent: AccountId,
        ) -> Result<(), Error> {
            let caller = self.env().caller();
            let owner = self.token_owner.get(token_id).ok_or(Error::TokenNotFound)?;
            if caller != self.admin && caller != owner {
                return Err(Error::Unauthorized);
            }
            self.management_agent.insert(token_id, &agent);
            self.env()
                .emit_event(ManagementAgentAssigned { token_id, agent });
            Ok(())
        }

        /// Removes the management agent assignment for a token (owner or admin only).
        ///
        /// # Arguments
        ///
        /// * `token_id` - The token to clear the management agent for
        ///
        /// # Returns
        ///
        /// Returns `Result<(), Error>` indicating success or failure
        #[ink(message)]
        pub fn clear_management_agent(&mut self, token_id: TokenId) -> Result<(), Error> {
            let caller = self.env().caller();
            let owner = self.token_owner.get(token_id).ok_or(Error::TokenNotFound)?;
            if caller != self.admin && caller != owner {
                return Err(Error::Unauthorized);
            }
            self.management_agent.remove(token_id);
            self.env().emit_event(ManagementAgentCleared { token_id });
            Ok(())
        }

        /// Returns the management agent for a token, if one is assigned.
        #[ink(message)]
        pub fn get_management_agent(&self, token_id: TokenId) -> Option<AccountId> {
            self.management_agent.get(token_id)
        }

        /// Returns the total number of fractional shares issued for a token.
        #[ink(message)]
        pub fn total_shares(&self, token_id: TokenId) -> u128 {
            self.total_shares.get(token_id).unwrap_or(0)
        }

        /// Returns the fractional share balance for a given owner and token.
        #[ink(message)]
        pub fn share_balance_of(&self, owner: AccountId, token_id: TokenId) -> u128 {
            self.balances.get((owner, token_id)).unwrap_or(0)
        }

        /// Issues new fractional shares for a token to a recipient (owner or admin only).
        ///
        /// Increases both the recipient's balance and the total share supply.
        /// Dividend credits are updated to prevent dilution of existing holders.
        ///
        /// # Arguments
        ///
        /// * `token_id` - The token to issue shares for
        /// * `to` - The recipient of the new shares
        /// * `amount` - The number of shares to issue (must be greater than zero)
        ///
        /// # Returns
        ///
        /// Returns `Result<(), Error>` indicating success or failure
        #[ink(message)]
        pub fn issue_shares(
            &mut self,
            token_id: TokenId,
            to: AccountId,
            amount: u128,
        ) -> Result<(), Error> {
            if amount == 0 {
                return Err(Error::InvalidAmount);
            }
            let caller = self.env().caller();
            let owner = self.token_owner.get(token_id).ok_or(Error::TokenNotFound)?;
            if caller != self.admin && caller != owner {
                return Err(Error::Unauthorized);
            }
            let bal = self.balances.get((to, token_id)).unwrap_or(0);
            self.balances
                .insert((to, token_id), &(bal.saturating_add(amount)));
            let ts = self.total_shares.get(token_id).unwrap_or(0);
            self.total_shares
                .insert(token_id, &(ts.saturating_add(amount)));
            self.update_dividend_credit_on_change(to, token_id)?;
            self.env().emit_event(SharesIssued {
                token_id,
                to,
                amount,
            });
            Ok(())
        }

        /// Redeems (burns) fractional shares from an account.
        ///
        /// The caller must be the account holder or an approved operator.
        /// Reduces both the holder's balance and the total share supply.
        ///
        /// # Arguments
        ///
        /// * `token_id` - The token whose shares are being redeemed
        /// * `from` - The account to redeem shares from
        /// * `amount` - The number of shares to redeem (must be greater than zero)
        ///
        /// # Returns
        ///
        /// Returns `Result<(), Error>` indicating success or failure
        #[ink(message)]
        pub fn redeem_shares(
            &mut self,
            token_id: TokenId,
            from: AccountId,
            amount: u128,
        ) -> Result<(), Error> {
            if amount == 0 {
                return Err(Error::InvalidAmount);
            }
            let caller = self.env().caller();
            if caller != from && !self.is_approved_for_all(from, caller) {
                return Err(Error::Unauthorized);
            }
            let bal = self.balances.get((from, token_id)).unwrap_or(0);
            if bal < amount {
                return Err(Error::InsufficientBalance);
            }
            self.balances
                .insert((from, token_id), &(bal.saturating_sub(amount)));
            let ts = self.total_shares.get(token_id).unwrap_or(0);
            self.total_shares
                .insert(token_id, &(ts.saturating_sub(amount)));
            self.update_dividend_credit_on_change(from, token_id)?;
            self.env().emit_event(SharesRedeemed {
                token_id,
                from,
                amount,
            });
            Ok(())
        }

        /// Transfers fractional shares between accounts with compliance checks.
        ///
        /// Both sender and recipient must pass compliance verification when a
        /// compliance registry is configured. Dividend credits are updated for
        /// both parties before the transfer.
        ///
        /// # Arguments
        ///
        /// * `from` - The account to transfer shares from
        /// * `to` - The account to transfer shares to
        /// * `token_id` - The token whose shares are being transferred
        /// * `amount` - The number of shares to transfer (must be greater than zero)
        ///
        /// # Returns
        ///
        /// Returns `Result<(), Error>` indicating success or failure
        #[ink(message)]
        pub fn transfer_shares(
            &mut self,
            from: AccountId,
            to: AccountId,
            token_id: TokenId,
            amount: u128,
        ) -> Result<(), Error> {
            if amount == 0 {
                return Err(Error::InvalidAmount);
            }
            let caller = self.env().caller();
            if caller != from && !self.is_approved_for_all(from, caller) {
                return Err(Error::Unauthorized);
            }
            if !self.pass_compliance(from)? || !self.pass_compliance(to)? {
                return Err(Error::ComplianceFailed);
            }
            let from_balance = self.balances.get((from, token_id)).unwrap_or(0);
            if from_balance < amount {
                return Err(Error::InsufficientBalance);
            }
            self.update_dividend_credit_on_change(from, token_id)?;
            self.update_dividend_credit_on_change(to, token_id)?;
            self.balances
                .insert((from, token_id), &(from_balance.saturating_sub(amount)));
            let to_balance = self.balances.get((to, token_id)).unwrap_or(0);
            self.balances
                .insert((to, token_id), &(to_balance.saturating_add(amount)));
            Ok(())
        }

        /// Deposits dividends for distribution to all share holders of a token.
        ///
        /// The deposited value is distributed proportionally based on each holder's
        /// share balance. Uses a scaled-integer approach (1e12 scaling factor) to
        /// maintain precision across small balances.
        ///
        /// # Arguments
        ///
        /// * `token_id` - The token to deposit dividends for
        ///
        /// # Returns
        ///
        /// Returns `Result<(), Error>` indicating success or failure
        #[ink(message, payable)]
        pub fn deposit_dividends(&mut self, token_id: TokenId) -> Result<(), Error> {
            let value = self.env().transferred_value();
            if value == 0 {
                return Err(Error::InvalidAmount);
            }
            let ts = self.total_shares.get(token_id).unwrap_or(0);
            if ts == 0 {
                return Err(Error::InvalidRequest);
            }
            let scaling: u128 = 1_000_000_000_000;
            let add = value.saturating_mul(scaling) / ts;
            let cur = self.dividends_per_share.get(token_id).unwrap_or(0);
            let new = cur.saturating_add(add);
            self.dividends_per_share.insert(token_id, &new);
            self.env().emit_event(DividendsDeposited {
                token_id,
                amount: value,
                per_share: add,
            });
            Ok(())
        }

        /// Withdraws accumulated dividends for the caller on a given token.
        ///
        /// Calculates any uncredited dividends, transfers the total owed amount
        /// to the caller, and updates the tax record.
        ///
        /// # Arguments
        ///
        /// * `token_id` - The token to withdraw dividends from
        ///
        /// # Returns
        ///
        /// Returns `Result<u128, Error>` with the amount withdrawn
        #[ink(message)]
        pub fn withdraw_dividends(&mut self, token_id: TokenId) -> Result<u128, Error> {
            let caller = self.env().caller();
            self.update_dividend_credit_on_change(caller, token_id)?;
            let owed = self.dividend_balance.get((caller, token_id)).unwrap_or(0);
            if owed == 0 {
                return Ok(0);
            }
            self.dividend_balance.insert((caller, token_id), &0u128);
            match self.env().transfer(caller, owed) {
                Ok(_) => {
                    let mut rec = self
                        .tax_records
                        .get((caller, token_id))
                        .unwrap_or(TaxRecord {
                            dividends_received: 0,
                            shares_sold: 0,
                            proceeds: 0,
                        });
                    rec.dividends_received = rec.dividends_received.saturating_add(owed);
                    self.tax_records.insert((caller, token_id), &rec);
                    self.env().emit_event(DividendsWithdrawn {
                        token_id,
                        account: caller,
                        amount: owed,
                    });
                    Ok(owed)
                }
                Err(_) => Err(Error::InvalidRequest),
            }
        }

        /// Creates a governance proposal for a tokenized property.
        ///
        /// Only the token owner or admin may create proposals. Voting weight
        /// is determined by each voter's share balance.
        ///
        /// # Arguments
        ///
        /// * `token_id` - The token the proposal applies to
        /// * `quorum` - Minimum for-votes required for the proposal to pass
        /// * `description_hash` - Hash of the off-chain proposal description
        ///
        /// # Returns
        ///
        /// Returns `Result<u64, Error>` with the new proposal ID
        #[ink(message)]
        pub fn create_proposal(
            &mut self,
            token_id: TokenId,
            quorum: u128,
            description_hash: Hash,
        ) -> Result<u64, Error> {
            let owner = self.token_owner.get(token_id).ok_or(Error::TokenNotFound)?;
            let caller = self.env().caller();
            if caller != self.admin && caller != owner {
                return Err(Error::Unauthorized);
            }
            let counter = self.proposal_counter.get(token_id).unwrap_or(0) + 1;
            self.proposal_counter.insert(token_id, &counter);
            let proposal = Proposal {
                id: counter,
                token_id,
                description_hash,
                quorum,
                for_votes: 0,
                against_votes: 0,
                status: ProposalStatus::Open,
                created_at: self.env().block_timestamp(),
            };
            self.proposals.insert((token_id, counter), &proposal);
            self.env().emit_event(ProposalCreated {
                token_id,
                proposal_id: counter,
                quorum,
            });
            Ok(counter)
        }

        /// Casts a vote on an open governance proposal.
        ///
        /// Voting weight equals the caller's share balance for the token.
        /// Each account may only vote once per proposal.
        ///
        /// # Arguments
        ///
        /// * `token_id` - The token the proposal belongs to
        /// * `proposal_id` - The proposal to vote on
        /// * `support` - `true` to vote in favor, `false` to vote against
        ///
        /// # Returns
        ///
        /// Returns `Result<(), Error>` indicating success or failure
        #[ink(message)]
        pub fn vote(
            &mut self,
            token_id: TokenId,
            proposal_id: u64,
            support: bool,
        ) -> Result<(), Error> {
            let mut proposal = self
                .proposals
                .get((token_id, proposal_id))
                .ok_or(Error::ProposalNotFound)?;
            if proposal.status != ProposalStatus::Open {
                return Err(Error::ProposalClosed);
            }
            let voter = self.env().caller();
            if self
                .votes_cast
                .get((token_id, proposal_id, voter))
                .unwrap_or(false)
            {
                return Err(Error::Unauthorized);
            }
            let weight = self.balances.get((voter, token_id)).unwrap_or(0);
            if support {
                proposal.for_votes = proposal.for_votes.saturating_add(weight);
            } else {
                proposal.against_votes = proposal.against_votes.saturating_add(weight);
            }
            self.proposals.insert((token_id, proposal_id), &proposal);
            self.votes_cast
                .insert((token_id, proposal_id, voter), &true);
            self.env().emit_event(Voted {
                token_id,
                proposal_id,
                voter,
                support,
                weight,
            });
            Ok(())
        }

        /// Executes a governance proposal, closing voting and recording the outcome.
        ///
        /// A proposal passes if for-votes meet the quorum and exceed against-votes.
        ///
        /// # Arguments
        ///
        /// * `token_id` - The token the proposal belongs to
        /// * `proposal_id` - The proposal to execute
        ///
        /// # Returns
        ///
        /// Returns `Result<bool, Error>` where `true` means the proposal passed
        #[ink(message)]
        pub fn execute_proposal(
            &mut self,
            token_id: TokenId,
            proposal_id: u64,
        ) -> Result<bool, Error> {
            let mut proposal = self
                .proposals
                .get((token_id, proposal_id))
                .ok_or(Error::ProposalNotFound)?;
            if proposal.status != ProposalStatus::Open {
                return Err(Error::ProposalClosed);
            }
            let passed = proposal.for_votes >= proposal.quorum
                && proposal.for_votes > proposal.against_votes;
            proposal.status = if passed {
                ProposalStatus::Executed
            } else {
                ProposalStatus::Rejected
            };
            self.proposals.insert((token_id, proposal_id), &proposal);
            self.env().emit_event(ProposalExecuted {
                token_id,
                proposal_id,
                passed,
            });
            Ok(passed)
        }

        /// Places a sell order (ask) for fractional shares on the marketplace.
        ///
        /// The specified shares are moved into escrow and a persistent ask is
        /// created. Other accounts can fill the ask via `buy_shares`.
        ///
        /// # Arguments
        ///
        /// * `token_id` - The token whose shares are being offered
        /// * `price_per_share` - Price per share in the native currency
        /// * `amount` - Number of shares to sell
        ///
        /// # Returns
        ///
        /// Returns `Result<(), Error>` indicating success or failure
        #[ink(message)]
        pub fn place_ask(
            &mut self,
            token_id: TokenId,
            price_per_share: u128,
            amount: u128,
        ) -> Result<(), Error> {
            if price_per_share == 0 || amount == 0 {
                return Err(Error::InvalidAmount);
            }
            let seller = self.env().caller();
            let bal = self.balances.get((seller, token_id)).unwrap_or(0);
            if bal < amount {
                return Err(Error::InsufficientBalance);
            }
            let esc = self.escrowed_shares.get((token_id, seller)).unwrap_or(0);
            self.escrowed_shares
                .insert((token_id, seller), &(esc.saturating_add(amount)));
            self.balances
                .insert((seller, token_id), &(bal.saturating_sub(amount)));
            let ask = Ask {
                token_id,
                seller,
                price_per_share,
                amount,
                created_at: self.env().block_timestamp(),
            };
            self.asks.insert((token_id, seller), &ask);
            self.env().emit_event(AskPlaced {
                token_id,
                seller,
                price_per_share,
                amount,
            });
            Ok(())
        }

        /// Cancels an active sell order and returns escrowed shares to the seller.
        ///
        /// # Arguments
        ///
        /// * `token_id` - The token whose ask is being cancelled
        ///
        /// # Returns
        ///
        /// Returns `Result<(), Error>` indicating success or failure
        #[ink(message)]
        pub fn cancel_ask(&mut self, token_id: TokenId) -> Result<(), Error> {
            let seller = self.env().caller();
            let _ask = self
                .asks
                .get((token_id, seller))
                .ok_or(Error::AskNotFound)?;
            let esc = self.escrowed_shares.get((token_id, seller)).unwrap_or(0);
            let bal = self.balances.get((seller, token_id)).unwrap_or(0);
            self.balances
                .insert((seller, token_id), &(bal.saturating_add(esc)));
            self.escrowed_shares.insert((token_id, seller), &0u128);
            self.asks.remove((token_id, seller));
            self.env().emit_event(AskCancelled { token_id, seller });
            Ok(())
        }

        /// Purchases fractional shares from an existing sell order.
        ///
        /// The caller must send exactly `price_per_share * amount` as the
        /// transferred value. Both buyer and seller must pass compliance checks.
        /// Proceeds are forwarded to the seller and a tax record is updated.
        ///
        /// # Arguments
        ///
        /// * `token_id` - The token whose shares are being purchased
        /// * `seller` - The account that placed the sell order
        /// * `amount` - Number of shares to buy
        ///
        /// # Returns
        ///
        /// Returns `Result<(), Error>` indicating success or failure
        #[ink(message, payable)]
        pub fn buy_shares(
            &mut self,
            token_id: TokenId,
            seller: AccountId,
            amount: u128,
        ) -> Result<(), Error> {
            if amount == 0 {
                return Err(Error::InvalidAmount);
            }
            let ask = self
                .asks
                .get((token_id, seller))
                .ok_or(Error::AskNotFound)?;
            if ask.amount < amount {
                return Err(Error::InvalidAmount);
            }
            let cost = ask.price_per_share.saturating_mul(amount);
            let paid = self.env().transferred_value();
            if paid != cost {
                return Err(Error::InvalidAmount);
            }
            let buyer = self.env().caller();
            if !self.pass_compliance(buyer)? || !self.pass_compliance(seller)? {
                return Err(Error::ComplianceFailed);
            }
            let esc = self.escrowed_shares.get((token_id, seller)).unwrap_or(0);
            if esc < amount {
                return Err(Error::AskNotFound);
            }
            let to_balance = self.balances.get((buyer, token_id)).unwrap_or(0);
            self.balances
                .insert((buyer, token_id), &(to_balance.saturating_add(amount)));
            self.escrowed_shares
                .insert((token_id, seller), &(esc.saturating_sub(amount)));
            match self.env().transfer(seller, cost) {
                Ok(_) => {
                    let mut rec = self
                        .tax_records
                        .get((seller, token_id))
                        .unwrap_or(TaxRecord {
                            dividends_received: 0,
                            shares_sold: 0,
                            proceeds: 0,
                        });
                    rec.shares_sold = rec.shares_sold.saturating_add(amount);
                    rec.proceeds = rec.proceeds.saturating_add(cost);
                    self.tax_records.insert((seller, token_id), &rec);
                }
                Err(_) => return Err(Error::InvalidRequest),
            }
            self.last_trade_price.insert(token_id, &ask.price_per_share);
            if ask.amount == amount {
                self.asks.remove((token_id, seller));
            } else {
                let mut new_ask = ask.clone();
                new_ask.amount = ask.amount.saturating_sub(amount);
                self.asks.insert((token_id, seller), &new_ask);
            }
            self.env().emit_event(SharesPurchased {
                token_id,
                seller,
                buyer,
                amount,
                price_per_share: ask.price_per_share,
            });
            Ok(())
        }

        /// Returns the last trade price per share for a token, if any trades have occurred.
        #[ink(message)]
        pub fn get_last_trade_price(&self, token_id: TokenId) -> Option<u128> {
            self.last_trade_price.get(token_id)
        }

        /// Returns a portfolio summary for a set of tokens owned by an account.
        ///
        /// Each entry contains (token_id, share_balance, last_trade_price).
        ///
        /// # Arguments
        ///
        /// * `owner` - The account to query
        /// * `token_ids` - The tokens to include in the portfolio summary
        ///
        /// # Returns
        ///
        /// Returns a vector of `(TokenId, balance, last_price)` tuples
        #[ink(message)]
        pub fn get_portfolio(
            &self,
            owner: AccountId,
            token_ids: Vec<TokenId>,
        ) -> Vec<(TokenId, u128, u128)> {
            let mut out = Vec::new();
            for t in token_ids.iter() {
                let bal = self.balances.get((owner, *t)).unwrap_or(0);
                let price = self.last_trade_price.get(*t).unwrap_or(0);
                out.push((*t, bal, price));
            }
            out
        }

        /// Returns the tax record for an account and token, summarizing dividends and sales.
        #[ink(message)]
        pub fn get_tax_record(&self, owner: AccountId, token_id: TokenId) -> TaxRecord {
            self.tax_records
                .get((owner, token_id))
                .unwrap_or(TaxRecord {
                    dividends_received: 0,
                    shares_sold: 0,
                    proceeds: 0,
                })
        }

        fn pass_compliance(&self, account: AccountId) -> Result<bool, Error> {
            if let Some(registry) = self.compliance_registry {
                use ink::env::call::FromAccountId;
                let checker: ink::contract_ref!(propchain_traits::ComplianceChecker) =
                    FromAccountId::from_account_id(registry);
                Ok(checker.is_compliant(account))
            } else {
                Ok(true)
            }
        }

        fn update_dividend_credit_on_change(
            &mut self,
            account: AccountId,
            token_id: TokenId,
        ) -> Result<(), Error> {
            let scaling: u128 = 1_000_000_000_000;
            let dps = self.dividends_per_share.get(token_id).unwrap_or(0);
            let credited = self.dividend_credit.get((account, token_id)).unwrap_or(0);
            if dps > credited {
                let bal = self.balances.get((account, token_id)).unwrap_or(0);
                let mut owed = self.dividend_balance.get((account, token_id)).unwrap_or(0);
                let delta = dps.saturating_sub(credited);
                let add = bal.saturating_mul(delta) / scaling;
                owed = owed.saturating_add(add);
                self.dividend_balance.insert((account, token_id), &owed);
                self.dividend_credit.insert((account, token_id), &dps);
            } else if credited == 0 && dps > 0 {
                self.dividend_credit.insert((account, token_id), &dps);
            }
            Ok(())
        }

        /// Property-specific: Registers a property and mints a token
        #[ink(message)]
        pub fn register_property_with_token(
            &mut self,
            metadata: PropertyMetadata,
        ) -> Result<TokenId, Error> {
            let caller = self.env().caller();

            // Register property in the property registry (simulated here)
            // In a real implementation, this might call an external contract

            // Mint a new token
            self.token_counter += 1;
            let token_id = self.token_counter;

            // Store property information
            let property_info = PropertyInfo {
                id: token_id, // Using token_id as property id for this implementation
                owner: caller,
                metadata: metadata.clone(),
                registered_at: self.env().block_timestamp(),
            };

            self.token_owner.insert(token_id, &caller);
            self.add_token_to_owner(caller, token_id)?;

            // Initialize balances
            self.balances.insert((&caller, &token_id), &1u128);

            // Store property-specific information
            self.token_properties.insert(token_id, &property_info);
            self.property_tokens.insert(token_id, &token_id); // property_id maps to token_id

            // Initialize ownership history
            let initial_transfer = OwnershipTransfer {
                from: AccountId::from([0u8; 32]), // Zero address for minting
                to: caller,
                timestamp: self.env().block_timestamp(),
                transaction_hash: {
                    use scale::Encode;
                    let data = (&caller, token_id);
                    let encoded = data.encode();
                    let mut hash_bytes = [0u8; 32];
                    let len = encoded.len().min(32);
                    hash_bytes[..len].copy_from_slice(&encoded[..len]);
                    Hash::from(hash_bytes)
                },
            };

            self.ownership_history_count.insert(token_id, &1u32);
            self.ownership_history_items
                .insert((token_id, 0), &initial_transfer);

            // Initialize compliance as unverified
            let compliance_info = ComplianceInfo {
                verified: false,
                verification_date: 0,
                verifier: AccountId::from([0u8; 32]),
                compliance_type: String::from("KYC"),
            };
            self.compliance_flags.insert(token_id, &compliance_info);

            // Initialize legal documents count
            self.legal_documents_count.insert(token_id, &0u32);

            self.total_supply += 1;

            self.env().emit_event(PropertyTokenMinted {
                token_id,
                property_id: token_id,
                owner: caller,
            });

            Ok(token_id)
        }

        /// Property-specific: Batch registers properties in a single gas-efficient transaction
        #[ink(message)]
        pub fn batch_register_properties(
            &mut self,
            metadata_list: Vec<PropertyMetadata>,
        ) -> Result<Vec<TokenId>, Error> {
            if metadata_list.len() > self.max_batch_size as usize {
                return Err(Error::BatchSizeExceeded);
            }
            let caller = self.env().caller();
            let mut issued_tokens = Vec::new();
            let current_time = self.env().block_timestamp();

            for metadata in metadata_list {
                self.token_counter += 1;
                let token_id = self.token_counter;

                let property_info = PropertyInfo {
                    id: token_id,
                    owner: caller,
                    metadata: metadata.clone(),
                    registered_at: current_time,
                };

                self.token_owner.insert(token_id, &caller);
                let balance = self.owner_token_count.get(caller).unwrap_or(0);
                self.owner_token_count.insert(caller, &(balance + 1));

                self.balances.insert((&caller, &token_id), &1u128);
                self.token_properties.insert(token_id, &property_info);
                self.property_tokens.insert(token_id, &token_id);

                let initial_transfer = OwnershipTransfer {
                    from: AccountId::from([0u8; 32]),
                    to: caller,
                    timestamp: current_time,
                    transaction_hash: Hash::default(),
                };

                self.ownership_history_count.insert(token_id, &1u32);
                self.ownership_history_items
                    .insert((token_id, 0), &initial_transfer);

                let compliance_info = ComplianceInfo {
                    verified: false,
                    verification_date: 0,
                    verifier: AccountId::from([0u8; 32]),
                    compliance_type: String::from("KYC"),
                };
                self.compliance_flags.insert(token_id, &compliance_info);
                self.legal_documents_count.insert(token_id, &0u32);

                self.env().emit_event(PropertyTokenMinted {
                    token_id,
                    property_id: token_id,
                    owner: caller,
                });

                issued_tokens.push(token_id);
            }

            self.total_supply += issued_tokens.len() as u64;

            Ok(issued_tokens)
        }

        /// Property-specific: Attaches a legal document to a token
        #[ink(message)]
        pub fn attach_legal_document(
            &mut self,
            token_id: TokenId,
            document_hash: Hash,
            document_type: String,
        ) -> Result<(), Error> {
            let caller = self.env().caller();
            let token_owner = self.token_owner.get(token_id).ok_or(Error::TokenNotFound)?;

            if token_owner != caller {
                return Err(Error::Unauthorized);
            }

            // Get existing documents count
            let document_count = self.legal_documents_count.get(token_id).unwrap_or(0);

            // Add new document
            let document_info = DocumentInfo {
                document_hash,
                document_type: document_type.clone(),
                upload_date: self.env().block_timestamp(),
                uploader: caller,
            };

            // Save updated documents
            self.legal_documents_items
                .insert((token_id, document_count), &document_info);
            self.legal_documents_count
                .insert(token_id, &(document_count + 1));

            self.env().emit_event(LegalDocumentAttached {
                token_id,
                document_hash,
                document_type,
            });

            Ok(())
        }

        /// Property-specific: Verifies compliance for a token
        #[ink(message)]
        pub fn verify_compliance(
            &mut self,
            token_id: TokenId,
            verification_status: bool,
        ) -> Result<(), Error> {
            let caller = self.env().caller();

            // Only admin or bridge operators can verify compliance
            if caller != self.admin && !self.bridge_operators.contains(&caller) {
                return Err(Error::Unauthorized);
            }

            let mut compliance_info = self
                .compliance_flags
                .get(token_id)
                .ok_or(Error::TokenNotFound)?;
            compliance_info.verified = verification_status;
            compliance_info.verification_date = self.env().block_timestamp();
            compliance_info.verifier = caller;

            self.compliance_flags.insert(token_id, &compliance_info);

            self.env().emit_event(ComplianceVerified {
                token_id,
                verified: verification_status,
                verifier: caller,
            });

            Ok(())
        }

        /// Property-specific: Gets ownership history for a token
        #[ink(message)]
        pub fn get_ownership_history(&self, token_id: TokenId) -> Option<Vec<OwnershipTransfer>> {
            let count = self.ownership_history_count.get(token_id).unwrap_or(0);
            if count == 0 {
                return None;
            }
            let mut result = Vec::new();
            for i in 0..count {
                if let Some(item) = self.ownership_history_items.get((token_id, i)) {
                    result.push(item);
                }
            }
            Some(result)
        }

        /// Cross-chain: Initiates token bridging to another chain with multi-signature
        #[ink(message)]
        pub fn initiate_bridge_multisig(
            &mut self,
            token_id: TokenId,
            destination_chain: ChainId,
            recipient: AccountId,
            required_signatures: u8,
            timeout_blocks: Option<u64>,
        ) -> Result<u64, Error> {
            let caller = self.env().caller();
            let token_owner = self.token_owner.get(token_id).ok_or(Error::TokenNotFound)?;

            // Check authorization
            if token_owner != caller {
                return Err(Error::Unauthorized);
            }

            // Check if bridge is paused
            if self.bridge_config.emergency_pause {
                return Err(Error::BridgePaused);
            }

            // Validate destination chain
            if !self
                .bridge_config
                .supported_chains
                .contains(&destination_chain)
            {
                return Err(Error::InvalidChain);
            }

            // Check compliance before bridging
            let compliance_info = self
                .compliance_flags
                .get(token_id)
                .ok_or(Error::ComplianceFailed)?;
            if !compliance_info.verified {
                return Err(Error::ComplianceFailed);
            }

            // Validate signature requirements
            if required_signatures < self.bridge_config.min_signatures_required
                || required_signatures > self.bridge_config.max_signatures_required
            {
                return Err(Error::InsufficientSignatures);
            }

            // Check for duplicate requests
            if self.has_pending_bridge_request(token_id) {
                return Err(Error::DuplicateBridgeRequest);
            }

            // Create bridge request
            self.bridge_request_counter += 1;
            let request_id = self.bridge_request_counter;
            let current_block = self.env().block_number();
            let _expires_at = timeout_blocks.map(|blocks| u64::from(current_block) + blocks);

            let property_info = self
                .token_properties
                .get(token_id)
                .ok_or(Error::PropertyNotFound)?;

            let request = MultisigBridgeRequest {
                request_id,
                token_id,
                source_chain: 1, // Current chain ID
                destination_chain,
                sender: caller,
                recipient,
                required_signatures,
                signatures: Vec::new(),
                created_at: u64::from(current_block),
                expires_at: timeout_blocks.map(|blocks| u64::from(current_block) + blocks),
                status: BridgeOperationStatus::Pending,
                metadata: property_info.metadata.clone(),
            };

            self.bridge_requests.insert(request_id, &request);

            self.env().emit_event(BridgeRequestCreated {
                request_id,
                token_id,
                source_chain: request.source_chain,
                destination_chain,
                requester: caller,
            });

            Ok(request_id)
        }

        /// Cross-chain: Signs a bridge request
        #[ink(message)]
        pub fn sign_bridge_request(&mut self, request_id: u64, approve: bool) -> Result<(), Error> {
            let caller = self.env().caller();

            // Check if caller is a bridge operator
            if !self.bridge_operators.contains(&caller) {
                return Err(Error::Unauthorized);
            }

            let mut request = self
                .bridge_requests
                .get(request_id)
                .ok_or(Error::InvalidRequest)?;

            // Check if request has expired
            if let Some(expires_at) = request.expires_at {
                if u64::from(self.env().block_number()) > expires_at {
                    request.status = BridgeOperationStatus::Expired;
                    self.bridge_requests.insert(request_id, &request);
                    return Err(Error::RequestExpired);
                }
            }

            // Check if already signed
            if request.signatures.contains(&caller) {
                return Err(Error::AlreadySigned);
            }

            // Add signature
            request.signatures.push(caller);

            // Update status based on approval and signatures collected
            if !approve {
                request.status = BridgeOperationStatus::Failed;
                self.env().emit_event(BridgeFailed {
                    request_id,
                    token_id: request.token_id,
                    error: String::from("Request rejected by operator"),
                });
            } else if request.signatures.len() >= request.required_signatures as usize {
                request.status = BridgeOperationStatus::Locked;

                // Lock the token for bridging
                let token_owner = self
                    .token_owner
                    .get(request.token_id)
                    .ok_or(Error::TokenNotFound)?;
                self.balances
                    .insert((&token_owner, &request.token_id), &0u128);
                self.token_owner
                    .insert(request.token_id, &AccountId::from([0u8; 32])); // Lock to zero address
            }

            self.bridge_requests.insert(request_id, &request);

            self.env().emit_event(BridgeRequestSigned {
                request_id,
                signer: caller,
                signatures_collected: request.signatures.len() as u8,
                signatures_required: request.required_signatures,
            });

            Ok(())
        }

        /// Cross-chain: Executes a bridge request after collecting required signatures
        #[ink(message)]
        pub fn execute_bridge(&mut self, request_id: u64) -> Result<(), Error> {
            let caller = self.env().caller();

            // Check if caller is a bridge operator
            if !self.bridge_operators.contains(&caller) {
                return Err(Error::Unauthorized);
            }

            let mut request = self
                .bridge_requests
                .get(request_id)
                .ok_or(Error::InvalidRequest)?;

            // Check if request is ready for execution
            if request.status != BridgeOperationStatus::Locked {
                return Err(Error::InvalidRequest);
            }

            // Check if enough signatures are collected
            if request.signatures.len() < request.required_signatures as usize {
                return Err(Error::InsufficientSignatures);
            }

            // Generate transaction hash
            let transaction_hash = self.generate_bridge_transaction_hash(&request);

            // Create bridge transaction record
            let transaction = BridgeTransaction {
                transaction_id: self.bridge_request_counter,
                token_id: request.token_id,
                source_chain: request.source_chain,
                destination_chain: request.destination_chain,
                sender: request.sender,
                recipient: request.recipient,
                transaction_hash,
                timestamp: self.env().block_timestamp(),
                gas_used: self.estimate_bridge_gas_usage(&request),
                status: BridgeOperationStatus::InTransit,
                metadata: request.metadata.clone(),
            };

            // Update request status
            request.status = BridgeOperationStatus::Completed;
            self.bridge_requests.insert(request_id, &request);

            // Store transaction verification
            self.verified_bridge_hashes.insert(transaction_hash, &true);

            // Add to bridge history
            let mut history = self
                .bridge_transactions
                .get(request.sender)
                .unwrap_or_default();
            history.push(transaction.clone());
            self.bridge_transactions.insert(request.sender, &history);

            // Update bridged token info
            let bridged_info = BridgedTokenInfo {
                original_chain: request.source_chain,
                original_token_id: request.token_id,
                destination_chain: request.destination_chain,
                destination_token_id: request.token_id, // Will be updated on destination
                bridged_at: self.env().block_timestamp(),
                status: BridgingStatus::InTransit,
            };

            self.bridged_tokens.insert(
                (&request.destination_chain, &request.token_id),
                &bridged_info,
            );

            self.env().emit_event(BridgeExecuted {
                request_id,
                token_id: request.token_id,
                transaction_hash,
            });

            Ok(())
        }

        /// Cross-chain: Receives a bridged token from another chain
        #[ink(message)]
        pub fn receive_bridged_token(
            &mut self,
            source_chain: ChainId,
            original_token_id: TokenId,
            recipient: AccountId,
            metadata: PropertyMetadata,
            transaction_hash: Hash,
        ) -> Result<TokenId, Error> {
            // Only bridge operators can receive bridged tokens
            let caller = self.env().caller();
            if !self.bridge_operators.contains(&caller) {
                return Err(Error::Unauthorized);
            }

            // Verify transaction hash
            if !self
                .verified_bridge_hashes
                .get(transaction_hash)
                .unwrap_or(false)
            {
                return Err(Error::InvalidRequest);
            }

            // Create a new token for the recipient
            self.token_counter += 1;
            let new_token_id = self.token_counter;

            // Store property information
            let property_info = PropertyInfo {
                id: new_token_id,
                owner: recipient,
                metadata,
                registered_at: self.env().block_timestamp(),
            };

            self.token_properties.insert(new_token_id, &property_info);
            self.token_owner.insert(new_token_id, &recipient);
            self.add_token_to_owner(recipient, new_token_id)?;
            self.balances.insert((&recipient, &new_token_id), &1u128);

            // Initialize ownership history for the new token
            let initial_transfer = OwnershipTransfer {
                from: AccountId::from([0u8; 32]), // Zero address for minting
                to: recipient,
                timestamp: self.env().block_timestamp(),
                transaction_hash: {
                    use scale::Encode;
                    let data = (&recipient, new_token_id);
                    let encoded = data.encode();
                    let mut hash_bytes = [0u8; 32];
                    let len = encoded.len().min(32);
                    hash_bytes[..len].copy_from_slice(&encoded[..len]);
                    Hash::from(hash_bytes)
                },
            };

            self.ownership_history_count.insert(new_token_id, &1u32);
            self.ownership_history_items
                .insert((new_token_id, 0), &initial_transfer);

            // Initialize compliance as verified for bridged tokens
            let compliance_info = ComplianceInfo {
                verified: true,
                verification_date: self.env().block_timestamp(),
                verifier: caller,
                compliance_type: String::from("Bridge"),
            };
            self.compliance_flags.insert(new_token_id, &compliance_info);

            // Initialize legal documents count
            self.legal_documents_count.insert(new_token_id, &0u32);

            self.total_supply += 1;

            // Update the bridged token status
            if let Some(mut bridged_info) =
                self.bridged_tokens.get((&source_chain, &original_token_id))
            {
                bridged_info.status = BridgingStatus::Completed;
                bridged_info.destination_token_id = new_token_id;
                self.bridged_tokens
                    .insert((&source_chain, &original_token_id), &bridged_info);
            }

            self.env().emit_event(Transfer {
                from: None, // None indicates minting
                to: Some(recipient),
                id: new_token_id,
            });

            Ok(new_token_id)
        }

        /// Cross-chain: Burns a bridged token when returning to original chain
        #[ink(message)]
        pub fn burn_bridged_token(
            &mut self,
            token_id: TokenId,
            destination_chain: ChainId,
            _recipient: AccountId,
        ) -> Result<(), Error> {
            let caller = self.env().caller();
            let token_owner = self.token_owner.get(token_id).ok_or(Error::TokenNotFound)?;

            // Check authorization
            if token_owner != caller {
                return Err(Error::Unauthorized);
            }

            // Check if token is bridged
            let bridged_info = self
                .bridged_tokens
                .get((&destination_chain, &token_id))
                .ok_or(Error::BridgeNotSupported)?;

            if bridged_info.status != BridgingStatus::Completed {
                return Err(Error::InvalidRequest);
            }

            // Burn the token
            self.remove_token_from_owner(caller, token_id)?;
            self.token_owner.remove(token_id);
            self.balances.insert((&caller, &token_id), &0u128);
            self.total_supply -= 1;

            // Update bridged token status
            let mut updated_info = bridged_info;
            updated_info.status = BridgingStatus::Locked;
            self.bridged_tokens
                .insert((&destination_chain, &token_id), &updated_info);

            self.env().emit_event(Transfer {
                from: Some(caller),
                to: None, // None indicates burning
                id: token_id,
            });

            Ok(())
        }

        /// Cross-chain: Recovers from a failed bridge operation
        #[ink(message)]
        pub fn recover_failed_bridge(
            &mut self,
            request_id: u64,
            recovery_action: RecoveryAction,
        ) -> Result<(), Error> {
            let caller = self.env().caller();

            // Only admin can recover failed bridges
            if caller != self.admin {
                return Err(Error::Unauthorized);
            }

            let mut request = self
                .bridge_requests
                .get(request_id)
                .ok_or(Error::InvalidRequest)?;

            // Check if request is in a failed state
            if !matches!(
                request.status,
                BridgeOperationStatus::Failed | BridgeOperationStatus::Expired
            ) {
                return Err(Error::InvalidRequest);
            }

            // Execute recovery action
            match recovery_action {
                RecoveryAction::UnlockToken => {
                    // Unlock the token
                    if let Some(token_owner) = self.token_owner.get(request.token_id) {
                        if token_owner == AccountId::from([0u8; 32]) {
                            // Token is locked, restore ownership to original sender
                            self.token_owner.insert(request.token_id, &request.sender);
                            self.balances
                                .insert((&request.sender, &request.token_id), &1u128);
                            self.add_token_to_owner(request.sender, request.token_id)?;
                        }
                    }
                }
                RecoveryAction::RefundGas => {
                    // Gas refund logic would be implemented here
                    // This would typically involve transferring native tokens
                }
                RecoveryAction::RetryBridge => {
                    // Reset request to pending for retry
                    request.status = BridgeOperationStatus::Pending;
                    request.signatures.clear();
                }
                RecoveryAction::CancelBridge => {
                    // Mark as cancelled and unlock token
                    request.status = BridgeOperationStatus::Failed;
                    if let Some(token_owner) = self.token_owner.get(request.token_id) {
                        if token_owner == AccountId::from([0u8; 32]) {
                            self.token_owner.insert(request.token_id, &request.sender);
                            self.balances
                                .insert((&request.sender, &request.token_id), &1u128);
                            self.add_token_to_owner(request.sender, request.token_id)?;
                        }
                    }
                }
            }

            self.bridge_requests.insert(request_id, &request);

            self.env().emit_event(BridgeRecovered {
                request_id,
                recovery_action,
            });

            Ok(())
        }

        /// Gets gas estimation for bridge operation
        #[ink(message)]
        pub fn estimate_bridge_gas(
            &self,
            token_id: TokenId,
            destination_chain: ChainId,
        ) -> Result<u64, Error> {
            if !self
                .bridge_config
                .supported_chains
                .contains(&destination_chain)
            {
                return Err(Error::InvalidChain);
            }

            let base_gas = self.bridge_config.gas_limit_per_bridge;
            let property_info = self
                .token_properties
                .get(token_id)
                .ok_or(Error::TokenNotFound)?;
            let metadata_gas = property_info.metadata.legal_description.len() as u64 * 100;

            Ok(base_gas + metadata_gas)
        }

        /// Monitors bridge status
        #[ink(message)]
        pub fn monitor_bridge_status(&self, request_id: u64) -> Option<BridgeMonitoringInfo> {
            let request = self.bridge_requests.get(request_id)?;

            Some(BridgeMonitoringInfo {
                bridge_request_id: request.request_id,
                token_id: request.token_id,
                source_chain: request.source_chain,
                destination_chain: request.destination_chain,
                status: request.status,
                created_at: request.created_at,
                expires_at: request.expires_at,
                signatures_collected: request.signatures.len() as u8,
                signatures_required: request.required_signatures,
                error_message: None,
            })
        }

        /// Gets bridge history for an account
        #[ink(message)]
        pub fn get_bridge_history(&self, account: AccountId) -> Vec<BridgeTransaction> {
            self.bridge_transactions.get(account).unwrap_or_default()
        }

        /// Verifies bridge transaction hash
        #[ink(message)]
        pub fn verify_bridge_transaction(
            &self,
            _token_id: TokenId,
            transaction_hash: Hash,
            _source_chain: ChainId,
        ) -> bool {
            self.verified_bridge_hashes
                .get(transaction_hash)
                .unwrap_or(false)
        }

        /// Gets bridge status for a token
        #[ink(message)]
        pub fn get_bridge_status(&self, token_id: TokenId) -> Option<BridgeStatus> {
            // Check through all bridged tokens
            for chain_id in &self.bridge_config.supported_chains {
                if let Some(bridged_info) = self.bridged_tokens.get((*chain_id, token_id)) {
                    return Some(BridgeStatus {
                        is_locked: matches!(
                            bridged_info.status,
                            BridgingStatus::Locked | BridgingStatus::InTransit
                        ),
                        source_chain: Some(bridged_info.original_chain),
                        destination_chain: Some(bridged_info.destination_chain),
                        locked_at: Some(bridged_info.bridged_at),
                        bridge_request_id: None,
                        status: match bridged_info.status {
                            BridgingStatus::Locked => BridgeOperationStatus::Locked,
                            BridgingStatus::Pending => BridgeOperationStatus::Pending,
                            BridgingStatus::InTransit => BridgeOperationStatus::InTransit,
                            BridgingStatus::Completed => BridgeOperationStatus::Completed,
                            BridgingStatus::Failed => BridgeOperationStatus::Failed,
                            BridgingStatus::Recovering => BridgeOperationStatus::Recovering,
                            BridgingStatus::Expired => BridgeOperationStatus::Expired,
                        },
                    });
                }
            }
            None
        }

        /// Adds a bridge operator
        #[ink(message)]
        pub fn add_bridge_operator(&mut self, operator: AccountId) -> Result<(), Error> {
            let caller = self.env().caller();
            if caller != self.admin {
                return Err(Error::Unauthorized);
            }

            if !self.bridge_operators.contains(&operator) {
                self.bridge_operators.push(operator);
            }

            Ok(())
        }

        /// Removes a bridge operator
        #[ink(message)]
        pub fn remove_bridge_operator(&mut self, operator: AccountId) -> Result<(), Error> {
            let caller = self.env().caller();
            if caller != self.admin {
                return Err(Error::Unauthorized);
            }

            self.bridge_operators.retain(|op| op != &operator);
            Ok(())
        }

        /// Checks if an account is a bridge operator
        #[ink(message)]
        pub fn is_bridge_operator(&self, account: AccountId) -> bool {
            self.bridge_operators.contains(&account)
        }

        /// Gets all bridge operators
        #[ink(message)]
        pub fn get_bridge_operators(&self) -> Vec<AccountId> {
            self.bridge_operators.clone()
        }

        /// Updates bridge configuration (admin only)
        #[ink(message)]
        pub fn update_bridge_config(&mut self, config: BridgeConfig) -> Result<(), Error> {
            let caller = self.env().caller();
            if caller != self.admin {
                return Err(Error::Unauthorized);
            }

            self.bridge_config = config;
            Ok(())
        }

        /// Gets current bridge configuration
        #[ink(message)]
        pub fn get_bridge_config(&self) -> BridgeConfig {
            self.bridge_config.clone()
        }

        /// Pauses or unpauses the bridge (admin only)
        #[ink(message)]
        pub fn set_emergency_pause(&mut self, paused: bool) -> Result<(), Error> {
            let caller = self.env().caller();
            if caller != self.admin {
                return Err(Error::Unauthorized);
            }

            self.bridge_config.emergency_pause = paused;
            Ok(())
        }

        /// Returns the total supply of tokens
        #[ink(message)]
        pub fn total_supply(&self) -> u64 {
            self.total_supply
        }

        /// Returns the current token counter
        #[ink(message)]
        pub fn current_token_id(&self) -> TokenId {
            self.token_counter
        }

        /// Returns the admin account
        #[ink(message)]
        pub fn admin(&self) -> AccountId {
            self.admin
        }

        /// Internal helper to add a token to an owner
        fn add_token_to_owner(&mut self, to: AccountId, _token_id: TokenId) -> Result<(), Error> {
            let count = self.owner_token_count.get(to).unwrap_or(0);
            self.owner_token_count.insert(to, &(count + 1));
            Ok(())
        }

        /// Internal helper to remove a token from an owner
        fn remove_token_from_owner(
            &mut self,
            from: AccountId,
            _token_id: TokenId,
        ) -> Result<(), Error> {
            let count = self.owner_token_count.get(from).unwrap_or(0);
            if count == 0 {
                return Err(Error::TokenNotFound);
            }
            self.owner_token_count.insert(from, &(count - 1));
            Ok(())
        }

        /// Internal helper to update ownership history
        fn update_ownership_history(
            &mut self,
            token_id: TokenId,
            from: AccountId,
            to: AccountId,
        ) -> Result<(), Error> {
            let count = self.ownership_history_count.get(token_id).unwrap_or(0);

            let transfer_record = OwnershipTransfer {
                from,
                to,
                timestamp: self.env().block_timestamp(),
                transaction_hash: {
                    use scale::Encode;
                    let data = (&from, &to, token_id);
                    let encoded = data.encode();
                    let mut hash_bytes = [0u8; 32];
                    let len = encoded.len().min(32);
                    hash_bytes[..len].copy_from_slice(&encoded[..len]);
                    Hash::from(hash_bytes)
                },
            };

            self.ownership_history_items
                .insert((token_id, count), &transfer_record);
            self.ownership_history_count.insert(token_id, &(count + 1));

            Ok(())
        }

        /// Helper to check if token has pending bridge request
        fn has_pending_bridge_request(&self, token_id: TokenId) -> bool {
            // This is a simplified check - in a real implementation,
            // you might want to maintain a separate mapping for efficiency
            for i in 1..=self.bridge_request_counter {
                if let Some(request) = self.bridge_requests.get(i) {
                    if request.token_id == token_id
                        && matches!(
                            request.status,
                            BridgeOperationStatus::Pending | BridgeOperationStatus::Locked
                        )
                    {
                        return true;
                    }
                }
            }
            false
        }

        /// Helper to generate bridge transaction hash
        fn generate_bridge_transaction_hash(&self, request: &MultisigBridgeRequest) -> Hash {
            use scale::Encode;
            let data = (
                request.request_id,
                request.token_id,
                request.source_chain,
                request.destination_chain,
                request.sender,
                request.recipient,
                self.env().block_timestamp(),
            );
            let encoded = data.encode();
            // Simple hash: use first 32 bytes of encoded data
            let mut hash_bytes = [0u8; 32];
            let len = encoded.len().min(32);
            hash_bytes[..len].copy_from_slice(&encoded[..len]);
            Hash::from(hash_bytes)
        }

        /// Helper to estimate bridge gas usage
        fn estimate_bridge_gas_usage(&self, request: &MultisigBridgeRequest) -> u64 {
            let base_gas = 100000; // Base gas for bridge operation
            let metadata_gas = request.metadata.legal_description.len() as u64 * 100;
            let signature_gas = request.required_signatures as u64 * 5000; // Gas per signature
            base_gas + metadata_gas + signature_gas
        }

        /// Log an error for monitoring and debugging
        fn log_error(
            &mut self,
            account: AccountId,
            error_code: String,
            message: String,
            context: Vec<(String, String)>,
        ) {
            let timestamp = self.env().block_timestamp();

            // Update error count for this account and error code
            let key = (account, error_code.clone());
            let current_count = self.error_counts.get(&key).unwrap_or(0);
            self.error_counts.insert(&key, &(current_count + 1));

            // Update error rate (1 hour window)
            let window_duration = 3_600_000_u64; // 1 hour in milliseconds
            let rate_key = error_code.clone();
            let (mut count, window_start) =
                self.error_rates.get(&rate_key).unwrap_or((0, timestamp));

            if timestamp >= window_start + window_duration {
                // Reset window
                count = 1;
                self.error_rates.insert(&rate_key, &(count, timestamp));
            } else {
                count += 1;
                self.error_rates.insert(&rate_key, &(count, window_start));
            }

            // Add to recent errors (keep last 100)
            let log_id = self.error_log_counter;
            self.error_log_counter = self.error_log_counter.wrapping_add(1);

            // Only keep last 100 errors (simple circular buffer)
            if log_id >= 100 {
                let old_id = log_id.wrapping_sub(100);
                self.recent_errors.remove(&old_id);
            }

            let error_entry = ErrorLogEntry {
                error_code: error_code.clone(),
                message,
                account,
                timestamp,
                context,
            };
            self.recent_errors.insert(&log_id, &error_entry);
        }

        /// Get error count for an account and error code
        #[ink(message)]
        pub fn get_error_count(&self, account: AccountId, error_code: String) -> u64 {
            self.error_counts.get(&(account, error_code)).unwrap_or(0)
        }

        /// Get error rate for an error code (errors per hour)
        #[ink(message)]
        pub fn get_error_rate(&self, error_code: String) -> u64 {
            let timestamp = self.env().block_timestamp();
            let window_duration = 3_600_000_u64; // 1 hour

            if let Some((count, window_start)) = self.error_rates.get(&error_code) {
                if timestamp >= window_start + window_duration {
                    0 // Window expired
                } else {
                    count
                }
            } else {
                0
            }
        }

        /// Get recent error log entries (admin only)
        #[ink(message)]
        pub fn get_recent_errors(&self, limit: u32) -> Vec<ErrorLogEntry> {
            // Only admin can access error logs
            if self.env().caller() != self.admin {
                return Vec::new();
            }

            let mut errors = Vec::new();
            let start_id = self.error_log_counter.saturating_sub(limit as u64);

            for i in start_id..self.error_log_counter {
                if let Some(entry) = self.recent_errors.get(&i) {
                    errors.push(entry);
                }
            }

            errors
        }
    }

    // Unit tests for the PropertyToken contract
    #[cfg(test)]
    mod tests {
        use super::*;
        use ink::env::{test, DefaultEnvironment};

        fn setup_contract() -> PropertyToken {
            PropertyToken::new()
        }

        #[ink::test]
        fn test_constructor_works() {
            let contract = setup_contract();
            assert_eq!(contract.total_supply(), 0);
            assert_eq!(contract.current_token_id(), 0);
        }

        #[ink::test]
        fn test_register_property_with_token() {
            let mut contract = setup_contract();

            let metadata = PropertyMetadata {
                location: String::from("123 Main St"),
                size: 1000,
                legal_description: String::from("Sample property"),
                valuation: 500000,
                documents_url: String::from("ipfs://sample-docs"),
            };

            let result = contract.register_property_with_token(metadata.clone());
            assert!(result.is_ok());

            let token_id = result.expect("Token registration should succeed in test");
            assert_eq!(token_id, 1);
            assert_eq!(contract.total_supply(), 1);
        }

        #[ink::test]
        fn test_balance_of() {
            let mut contract = setup_contract();

            let metadata = PropertyMetadata {
                location: String::from("123 Main St"),
                size: 1000,
                legal_description: String::from("Sample property"),
                valuation: 500000,
                documents_url: String::from("ipfs://sample-docs"),
            };

            let _token_id = contract
                .register_property_with_token(metadata)
                .expect("Token registration should succeed in test");
            let _caller = AccountId::from([1u8; 32]);

            // Set up mock caller for the test
            let accounts = test::default_accounts::<DefaultEnvironment>();
            test::set_caller::<DefaultEnvironment>(accounts.alice);

            assert_eq!(contract.balance_of(accounts.alice), 1);
        }

        #[ink::test]
        fn test_attach_legal_document() {
            let mut contract = setup_contract();

            let metadata = PropertyMetadata {
                location: String::from("123 Main St"),
                size: 1000,
                legal_description: String::from("Sample property"),
                valuation: 500000,
                documents_url: String::from("ipfs://sample-docs"),
            };

            let token_id = contract
                .register_property_with_token(metadata)
                .expect("Token registration should succeed in test");

            let accounts = test::default_accounts::<DefaultEnvironment>();
            test::set_caller::<DefaultEnvironment>(accounts.alice);

            let doc_hash = Hash::from([1u8; 32]);
            let doc_type = String::from("Deed");

            let result = contract.attach_legal_document(token_id, doc_hash, doc_type);
            assert!(result.is_ok());
        }

        #[ink::test]
        fn test_verify_compliance() {
            let mut contract = setup_contract();

            let metadata = PropertyMetadata {
                location: String::from("123 Main St"),
                size: 1000,
                legal_description: String::from("Sample property"),
                valuation: 500000,
                documents_url: String::from("ipfs://sample-docs"),
            };

            let token_id = contract
                .register_property_with_token(metadata)
                .expect("Token registration should succeed in test");

            let _accounts = test::default_accounts::<DefaultEnvironment>();
            test::set_caller::<DefaultEnvironment>(contract.admin());

            let result = contract.verify_compliance(token_id, true);
            assert!(result.is_ok());

            let compliance_info = contract
                .compliance_flags
                .get(&token_id)
                .expect("Compliance info should exist after verification");
            assert!(compliance_info.verified);
        }

        // ============================================================================
        // EDGE CASE TESTS
        // ============================================================================

        #[ink::test]
        fn test_transfer_from_nonexistent_token() {
            let mut contract = setup_contract();
            let accounts = test::default_accounts::<DefaultEnvironment>();

            let result = contract.transfer_from(accounts.alice, accounts.bob, 999);
            assert_eq!(result, Err(Error::TokenNotFound));
        }

        #[ink::test]
        fn test_transfer_from_unauthorized_caller() {
            let mut contract = setup_contract();
            let accounts = test::default_accounts::<DefaultEnvironment>();
            test::set_caller::<DefaultEnvironment>(accounts.alice);

            let metadata = PropertyMetadata {
                location: String::from("123 Main St"),
                size: 1000,
                legal_description: String::from("Sample property"),
                valuation: 500000,
                documents_url: String::from("ipfs://sample-docs"),
            };

            let token_id = contract
                .register_property_with_token(metadata)
                .expect("Token registration should succeed in test");

            // Bob tries to transfer Alice's token without approval
            test::set_caller::<DefaultEnvironment>(accounts.bob);
            let result = contract.transfer_from(accounts.alice, accounts.bob, token_id);
            assert_eq!(result, Err(Error::Unauthorized));
        }

        #[ink::test]
        fn test_approve_nonexistent_token() {
            let mut contract = setup_contract();
            let accounts = test::default_accounts::<DefaultEnvironment>();

            let result = contract.approve(accounts.bob, 999);
            assert_eq!(result, Err(Error::TokenNotFound));
        }

        #[ink::test]
        fn test_approve_unauthorized_caller() {
            let mut contract = setup_contract();
            let accounts = test::default_accounts::<DefaultEnvironment>();
            test::set_caller::<DefaultEnvironment>(accounts.alice);

            let metadata = PropertyMetadata {
                location: String::from("123 Main St"),
                size: 1000,
                legal_description: String::from("Sample property"),
                valuation: 500000,
                documents_url: String::from("ipfs://sample-docs"),
            };

            let token_id = contract
                .register_property_with_token(metadata)
                .expect("Token registration should succeed in test");

            // Bob tries to approve without being owner or operator
            test::set_caller::<DefaultEnvironment>(accounts.bob);
            let result = contract.approve(accounts.charlie, token_id);
            assert_eq!(result, Err(Error::Unauthorized));
        }

        #[ink::test]
        fn test_owner_of_nonexistent_token() {
            let contract = setup_contract();

            assert_eq!(contract.owner_of(0), None);
            assert_eq!(contract.owner_of(1), None);
            assert_eq!(contract.owner_of(u64::MAX), None);
        }

        #[ink::test]
        fn test_balance_of_nonexistent_account() {
            let contract = setup_contract();
            let nonexistent = AccountId::from([0xFF; 32]);

            assert_eq!(contract.balance_of(nonexistent), 0);
        }

        #[ink::test]
        fn test_attach_document_to_nonexistent_token() {
            let mut contract = setup_contract();
            let doc_hash = Hash::from([1u8; 32]);

            let result = contract.attach_legal_document(999, doc_hash, "Deed".to_string());
            assert_eq!(result, Err(Error::TokenNotFound));
        }

        #[ink::test]
        fn test_attach_document_unauthorized() {
            let mut contract = setup_contract();
            let accounts = test::default_accounts::<DefaultEnvironment>();
            test::set_caller::<DefaultEnvironment>(accounts.alice);

            let metadata = PropertyMetadata {
                location: String::from("123 Main St"),
                size: 1000,
                legal_description: String::from("Sample property"),
                valuation: 500000,
                documents_url: String::from("ipfs://sample-docs"),
            };

            let token_id = contract
                .register_property_with_token(metadata)
                .expect("Token registration should succeed in test");

            // Bob tries to attach document
            test::set_caller::<DefaultEnvironment>(accounts.bob);
            let doc_hash = Hash::from([1u8; 32]);
            let result = contract.attach_legal_document(token_id, doc_hash, "Deed".to_string());
            assert_eq!(result, Err(Error::Unauthorized));
        }

        #[ink::test]
        fn test_verify_compliance_nonexistent_token() {
            let mut contract = setup_contract();
            let accounts = test::default_accounts::<DefaultEnvironment>();
            test::set_caller::<DefaultEnvironment>(accounts.alice);

            let result = contract.verify_compliance(999, true);
            assert_eq!(result, Err(Error::TokenNotFound));
        }

        #[ink::test]
        fn test_initiate_bridge_invalid_chain() {
            let mut contract = setup_contract();
            let accounts = test::default_accounts::<DefaultEnvironment>();
            test::set_caller::<DefaultEnvironment>(accounts.alice);

            let metadata = PropertyMetadata {
                location: String::from("123 Main St"),
                size: 1000,
                legal_description: String::from("Sample property"),
                valuation: 500000,
                documents_url: String::from("ipfs://sample-docs"),
            };

            let token_id = contract
                .register_property_with_token(metadata)
                .expect("Token registration should succeed in test");

            // Try to bridge to unsupported chain
            let result = contract.initiate_bridge_multisig(
                token_id,
                999, // Invalid chain ID
                accounts.bob,
                2,    // required_signatures
                None, // timeout_blocks
            );

            assert_eq!(result, Err(Error::InvalidChain));
        }

        #[ink::test]
        fn test_initiate_bridge_nonexistent_token() {
            let mut contract = setup_contract();
            let accounts = test::default_accounts::<DefaultEnvironment>();

            let result = contract.initiate_bridge_multisig(
                999,          // nonexistent token_id
                2,            // destination_chain
                accounts.bob, // recipient
                2,            // required_signatures
                None,         // timeout_blocks
            );

            assert_eq!(result, Err(Error::TokenNotFound));
        }

        #[ink::test]
        fn test_sign_bridge_request_nonexistent() {
            let mut contract = setup_contract();
            let _accounts = test::default_accounts::<DefaultEnvironment>();

            let result = contract.sign_bridge_request(999, true);
            assert_eq!(result, Err(Error::InvalidRequest));
        }

        #[ink::test]
        fn test_register_multiple_properties_increments_ids() {
            let mut contract = setup_contract();
            let accounts = test::default_accounts::<DefaultEnvironment>();
            test::set_caller::<DefaultEnvironment>(accounts.alice);

            for i in 1..=10 {
                let metadata = PropertyMetadata {
                    location: format!("Property {}", i),
                    size: 1000 + i,
                    legal_description: format!("Description {}", i),
                    valuation: 100_000 + (i as u128 * 1000),
                    documents_url: format!("ipfs://prop{}", i),
                };

                let token_id = contract
                    .register_property_with_token(metadata)
                    .expect("Token registration should succeed in test");
                assert_eq!(token_id, i);
                assert_eq!(contract.total_supply(), i);
            }
        }

        #[ink::test]
        fn test_transfer_preserves_total_supply() {
            let mut contract = setup_contract();
            let accounts = test::default_accounts::<DefaultEnvironment>();
            test::set_caller::<DefaultEnvironment>(accounts.alice);

            let metadata = PropertyMetadata {
                location: String::from("123 Main St"),
                size: 1000,
                legal_description: String::from("Sample property"),
                valuation: 500000,
                documents_url: String::from("ipfs://sample-docs"),
            };

            let token_id = contract
                .register_property_with_token(metadata)
                .expect("Token registration should succeed in test");

            let initial_supply = contract.total_supply();

            contract
                .transfer_from(accounts.alice, accounts.bob, token_id)
                .expect("Transfer should succeed");

            // Total supply should remain constant
            assert_eq!(contract.total_supply(), initial_supply);
        }

        #[ink::test]
        fn test_balance_of_batch_empty_vectors() {
            let contract = setup_contract();

            let result = contract.balance_of_batch(Vec::new(), Vec::new());
            assert_eq!(result, Vec::<u128>::new());
        }

        #[ink::test]
        fn test_get_error_count_nonexistent() {
            let contract = setup_contract();
            let accounts = test::default_accounts::<DefaultEnvironment>();

            let count = contract.get_error_count(accounts.alice, "NONEXISTENT".to_string());
            assert_eq!(count, 0);
        }

        #[ink::test]
        fn test_get_error_rate_nonexistent() {
            let contract = setup_contract();

            let rate = contract.get_error_rate("NONEXISTENT".to_string());
            assert_eq!(rate, 0);
        }

        #[ink::test]
        fn test_get_recent_errors_unauthorized() {
            let contract = setup_contract();
            let accounts = test::default_accounts::<DefaultEnvironment>();

            // Non-admin tries to get errors
            test::set_caller::<DefaultEnvironment>(accounts.bob);
            let errors = contract.get_recent_errors(10);
            assert_eq!(errors, Vec::new());
        }

        #[ink::test]
        fn test_property_management_linkage() {
            let mut contract = setup_contract();
            let accounts = test::default_accounts::<DefaultEnvironment>();
            test::set_caller::<DefaultEnvironment>(accounts.alice);

            let metadata = PropertyMetadata {
                location: String::from("123 Main St"),
                size: 1000,
                legal_description: String::from("Sample property"),
                valuation: 500000,
                documents_url: String::from("ipfs://sample-docs"),
            };
            let token_id = contract
                .register_property_with_token(metadata)
                .expect("register");

            test::set_caller::<DefaultEnvironment>(contract.admin());
            contract
                .set_property_management_contract(Some(accounts.charlie))
                .expect("set pm contract");
            assert_eq!(
                contract.get_property_management_contract(),
                Some(accounts.charlie)
            );

            test::set_caller::<DefaultEnvironment>(accounts.alice);
            contract
                .assign_management_agent(token_id, accounts.bob)
                .expect("agent");
            assert_eq!(contract.get_management_agent(token_id), Some(accounts.bob));

            contract.clear_management_agent(token_id).expect("clear");
            assert_eq!(contract.get_management_agent(token_id), None);
        }
    }
}
