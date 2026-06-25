#![cfg_attr(not(feature = "std"), no_std)]
#![allow(
    unexpected_cfgs,
    clippy::type_complexity,
    clippy::needless_borrows_for_generic_args,
    clippy::cast_possible_truncation,
    clippy::arithmetic_side_effects,
    clippy::cast_sign_loss
)]

use ink::prelude::string::String;
use ink::storage::{Lazy, Mapping};
use ink::prelude::collections::HashMap as StorageHashMap;
use propchain_traits::*;
use propchain_traits::{non_reentrant, ReentrancyError, ReentrancyGuard};
#[cfg(not(feature = "std"))]
use scale_info::prelude::vec::Vec;

#[ink::contract]
pub mod property_token {
    use super::*;

    // Error types extracted to errors.rs (Issue #101)
    include!("errors.rs");

    impl From<ReentrancyError> for Error {
        fn from(_: ReentrancyError) -> Self {
            Error::ReentrantCall
        }
    }

    /// Property Token contract that maintains compatibility with ERC-721 and ERC-1155
    /// while adding real estate-specific features and cross-chain support
    #[ink(storage)]
    pub struct PropertyToken {
        // ERC-721 standard mappings
        token_owner: Mapping<TokenId, AccountId>,
        owner_token_count: Mapping<AccountId, u32>,
        token_approvals: Lazy<StorageHashMap<TokenId, AccountId>>,
        operator_approvals: Lazy<StorageHashMap<(AccountId, AccountId), ()>>,
        // Ephemeral (instance) storage for frequently-read operator approvals
        ephemeral_operators: Lazy<StorageHashMap<(AccountId, AccountId), ()>>,

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
        bridged_token_origins: Mapping<TokenId, (ChainId, TokenId)>,
        bridge_operators: Vec<AccountId>,
        bridge_requests: Mapping<u64, MultisigBridgeRequest>,
        bridge_transactions: Mapping<AccountId, Vec<BridgeTransaction>>,
        bridge_config: BridgeConfig,
        current_chain: ChainId,
        verified_bridge_hashes: Mapping<Hash, bool>,
        bridge_request_counter: u64,
        transaction_counter: u64,

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

        // KYC-based transfer restriction fields
        /// Transfer restriction configuration per token
        transfer_restrictions: Mapping<TokenId, TransferRestrictionConfig>,
        /// User transfer quota tracking (token_id, account) -> quota
        user_transfer_quotas: Mapping<(TokenId, AccountId), UserTransferQuota>,
        /// Blacklisted accounts that cannot transfer tokens
        blacklist: Mapping<AccountId, bool>,
        /// Explicit KYC approval flag for transfer recipients
        kyc_approved: Mapping<AccountId, bool>,
        /// Whitelisted accounts (if whitelist-only restriction is enabled)
        whitelist: Mapping<(TokenId, AccountId), bool>,
        /// Cached KYC verification levels to reduce cross-contract calls
        kyc_verification_cache: Mapping<AccountId, (KYCVerificationLevel, u64)>, // (level, block_cached)
        /// KYC transfer audit log
        kyc_transfer_log: Mapping<u64, KYCTransferEvent>,
        kyc_transfer_log_counter: u64,

        /// Vesting schedules for tokens (TokenId, AccountId)
        vesting_schedules: Mapping<(TokenId, AccountId), VestingSchedule>,
        /// Custom URI overrides for tokens
        token_uris: Mapping<TokenId, String>,

        // Staking state (Issue #197)
        share_stakes: Mapping<(AccountId, TokenId), ShareStakeInfo>,
        share_total_staked: Mapping<TokenId, u128>,
        share_reward_pool: Mapping<TokenId, u128>,
        share_reward_rate_bps: Mapping<TokenId, u128>,
        share_acc_reward_per_share: Mapping<TokenId, u128>,
        share_last_reward_block: Mapping<TokenId, u64>,

        /// Reentrancy protection guard
        reentrancy_guard: ReentrancyGuard,
        /// Snapshot functionality for governance voting (Issue #194)
        snapshot_counter: Mapping<TokenId, u64>,
        snapshots: Mapping<(TokenId, u64), Snapshot>,
        account_snapshots: Mapping<(AccountId, TokenId, u64), u128>, // (account, token_id, snapshot_id) -> balance

        // Metadata versioning (Issue #557)
        /// Number of historical versions stored for a token (0 = no updates yet)
        metadata_version_count: Mapping<TokenId, u32>,
        /// Versioned metadata snapshots: (token_id, version_number) -> MetadataVersion
        metadata_versions: Mapping<(TokenId, u32), MetadataVersion>,
    }

    // Data types extracted to types.rs (Issue #101)
    include!("types.rs");

    // Events organized by domain (Issue #101 - see events.rs for reference copy)

    // --- ERC-721/1155 Standard Events ---
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
    pub struct BatchTransfer {
        #[ink(topic)]
        pub from: Option<AccountId>,
        #[ink(topic)]
        pub to: Option<AccountId>,
        pub ids: Vec<TokenId>,
        pub amounts: Vec<u128>,
    }

    // --- Property Events ---
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

    // --- Bridge Events ---
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

    // --- Fractional / Dividend Events ---
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

    // --- Governance Events ---
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

    // --- Snapshot Events (Issue #194) ---
    #[ink(event)]
    pub struct SnapshotCreated {
        #[ink(topic)]
        pub token_id: TokenId,
        #[ink(topic)]
        pub snapshot_id: u64,
        pub total_supply: u64,
        pub description: String,
    }

    #[ink(event)]
    pub struct SnapshotBalanceQueried {
        #[ink(topic)]
        pub token_id: TokenId,
        #[ink(topic)]
        pub snapshot_id: u64,
        #[ink(topic)]
        pub account: AccountId,
        pub balance: u128,
    }

    // --- Marketplace Events ---
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

    // --- Management Events ---
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

    // --- KYC Transfer Restriction Events ---
    #[ink(event)]
    pub struct TransferRestrictionConfigured {
        #[ink(topic)]
        pub token_id: TokenId,
        pub restriction_level: String,
        pub min_verification_level: u8,
        pub max_transfer_amount: u128,
    }

    #[ink(event)]
    pub struct TransferRestrictionRemoved {
        #[ink(topic)]
        pub token_id: TokenId,
    }

    #[ink(event)]
    pub struct KYCTransferVerified {
        #[ink(topic)]
        pub from: AccountId,
        #[ink(topic)]
        pub to: AccountId,
        #[ink(topic)]
        pub token_id: TokenId,
        pub amount: u128,
        pub from_verification_level: u8,
        pub to_verification_level: u8,
    }

    #[ink(event)]
    pub struct KYCTransferRejected {
        #[ink(topic)]
        pub from: AccountId,
        #[ink(topic)]
        pub to: AccountId,
        #[ink(topic)]
        pub token_id: TokenId,
        pub reason: String,
    }

    #[ink(event)]
    pub struct AccountBlacklisted {
        #[ink(topic)]
        pub account: AccountId,
        pub status: bool,
    }

    #[ink(event)]
    pub struct AccountWhitelisted {
        #[ink(topic)]
        pub token_id: TokenId,
        #[ink(topic)]
        pub account: AccountId,
        pub status: bool,
    }

    // --- Staking Events ---
    #[ink(event)]
    pub struct SharesStaked {
        #[ink(topic)]
        pub token_id: TokenId,
        #[ink(topic)]
        pub staker: AccountId,
        pub amount: u128,
        pub lock_period: LockPeriod,
        pub lock_until: u64,
    }

    #[ink(event)]
    pub struct SharesUnstaked {
        #[ink(topic)]
        pub token_id: TokenId,
        #[ink(topic)]
        pub staker: AccountId,
        pub amount: u128,
    }

    #[ink(event)]
    pub struct StakeRewardsClaimed {
        #[ink(topic)]
        pub token_id: TokenId,
        #[ink(topic)]
        pub staker: AccountId,
        pub amount: u128,
    }

    #[ink(event)]
    pub struct StakeRewardPoolFunded {
        #[ink(topic)]
        pub token_id: TokenId,
        #[ink(topic)]
        pub funder: AccountId,
        pub amount: u128,
    }

    // --- Vesting Events ---
    #[ink(event)]
    pub struct VestingScheduleCreated {
        #[ink(topic)]
        pub token_id: TokenId,
        #[ink(topic)]
        pub account: AccountId,
        pub role: VestingRole,
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

    // --- Supply Management Events ---
    #[ink(event)]
    pub struct TokenBurned {
        #[ink(topic)]
        pub token_id: TokenId,
        #[ink(topic)]
        pub burned_by: AccountId,
        pub reason: String,
    }

    // --- Batch Mint Events (Issue #556) ---
    #[ink(event)]
    pub struct BatchMinted {
        #[ink(topic)]
        pub owner: AccountId,
        pub token_ids: Vec<TokenId>,
        pub count: u32,
    }

    impl Default for PropertyToken {
        fn default() -> Self {
            Self::new()
        }
    }

    impl PropertyToken {
        /// Creates a new PropertyToken contract
        #[ink(constructor)]
        pub fn new() -> Self {
            let caller = Self::env().caller();

            // Initialize default bridge configuration
            let bridge_config = BridgeConfig {
                supported_chains: vec![1, 2, 3],
                min_signatures_required: 2,
                max_signatures_required: 5,
                default_timeout_blocks: 100,
                gas_limit_per_bridge: 500000,
                emergency_pause: false,
                metadata_preservation: true,
                rate_limit_enabled: false,
                max_requests_per_day: 1000,
                max_value_per_day: 10_000_000,
            };
            let current_chain = bridge_config.supported_chains[0];

            Self {
                // ERC-721 standard mappings
                token_owner: Mapping::default(),
                owner_token_count: Mapping::default(),
                token_approvals: Lazy::new(StorageHashMap::new()),
                operator_approvals: Lazy::new(StorageHashMap::new()),
                ephemeral_operators: Lazy::new(StorageHashMap::new()),

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
                current_chain,
                verified_bridge_hashes: Mapping::default(),
                bridge_request_counter: 0,
                transaction_counter: 0,
                bridged_token_origins: Mapping::default(),

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

                // Initialize KYC transfer restriction fields
                transfer_restrictions: Mapping::default(),
                user_transfer_quotas: Mapping::default(),
                blacklist: Mapping::default(),
                kyc_approved: Mapping::default(),
                whitelist: Mapping::default(),
                kyc_verification_cache: Mapping::default(),
                kyc_transfer_log: Mapping::default(),
                kyc_transfer_log_counter: 0,

                vesting_schedules: Mapping::default(),
                token_uris: Mapping::default(),

                reentrancy_guard: ReentrancyGuard::new(),
                snapshot_counter: Mapping::default(),
                snapshots: Mapping::default(),
                account_snapshots: Mapping::default(),
                // Staking fields (Issue #197)
                share_stakes: Mapping::default(),
                share_total_staked: Mapping::default(),
                share_acc_reward_per_share: Mapping::default(),
                share_last_reward_block: Mapping::default(),
                share_reward_pool: Mapping::default(),
                share_reward_rate_bps: Mapping::default(),

                // Metadata versioning (Issue #557)
                metadata_version_count: Mapping::default(),
                metadata_versions: Mapping::default(),
            }
        }

        // --- ERC-721/1155 Query Functions ---
        #[ink(message)]
        pub fn balance_of(&self, owner: AccountId) -> u32 {
            self.owner_token_count.get(&owner).unwrap_or(0)
        }

        #[ink(message)]
        pub fn owner_of(&self, id: TokenId) -> Option<AccountId> {
            self.token_owner.get(&id)
        }

        #[ink(message)]
        pub fn get_approved(&self, id: TokenId) -> Option<AccountId> {
            self.token_approvals.get(&id).cloned()
        }

        #[ink(message)]
        pub fn is_approved_for_all(&self, owner: AccountId, operator: AccountId) -> bool {
            self.operators.get(&(owner, operator)).unwrap_or(false)
                || self.ephemeral_operators.get(&(owner, operator)).is_some()
        }

        /// Checks if an operator is approved for all (ephemeral only)
        #[ink(message)]
        pub fn is_approved_for_all_ephemeral(&self, owner: AccountId, operator: AccountId) -> bool {
            self.ephemeral_operators.get(&(owner, operator)).is_some()
        }

        // --- ERC-721/1155 Transfer Functions ---
        #[ink(message)]
        pub fn transfer(&mut self, to: AccountId, id: TokenId) -> Result<(), Error> {
            let caller = self.env().caller();
            self.transfer_from(caller, to, id)
        }

        #[ink(message)]
        pub fn transfer_from(
            &mut self,
            from: AccountId,
            to: AccountId,
            id: TokenId,
        ) -> Result<(), Error> {
            if to == AccountId::from([0x0; 32]) {
                return Err(Error::InvalidRecipient);
            }
            let owner = self.owner_of(id).ok_or(Error::TokenNotFound)?;
            if owner != from {
                return Err(Error::NotOwner);
            }
            let caller = self.env().caller();
            let approved = self.get_approved(id);
            if !(owner == caller
                || approved == Some(caller)
                || self.is_approved_for_all(owner, caller))
            {
                return Err(Error::NotApproved);
            }

            self.token_owner.remove(&id);
            self.token_owner.insert(id, to);

            let from_balance = self.balance_of(from);
            self.owner_token_count.insert(from, from_balance - 1);

            let to_balance = self.balance_of(to);
            self.owner_token_count.insert(to, to_balance + 1);

            self.env().emit_event(Transfer {
                from: Some(from),
                to: Some(to),
                id,
            });

            Ok(())
        }

        // --- ERC-721/1155 Approval Functions ---
        #[ink(message)]
        pub fn approve(&mut self, to: AccountId, id: TokenId) -> Result<(), Error> {
            let owner = self.owner_of(id).ok_or(Error::TokenNotFound)?;
            if owner != self.env().caller() {
                return Err(Error::NotOwner);
            }
            self.token_approvals.insert(id, to);
            Ok(())
        }

        #[ink(message)]
        pub fn set_approval_for_all(&mut self, operator: AccountId, approved: bool) -> Result<(), Error> {
            let caller = self.env().caller();
            if approved {
                self.ephemeral_operators.insert((caller, operator), ());
            } else {
                self.ephemeral_operators.take(&(caller, operator));
            }
            self.env().emit_event(ApprovalForAll {
                owner: caller,
                operator,
                approved,
            });
            Ok(())
        }

        // --- Property Token Functions ---
        #[ink(message)]
        pub fn mint_property_token(
            &mut self,
            property_id: u64,
            owner: AccountId,
            property_info: PropertyInfo,
        ) -> Result<TokenId, Error> {
            let caller = self.env().caller();
            if caller != self.admin {
                return Err(Error::AdminRequired);
            }

            let token_id = self.token_counter;
            self.token_owner.insert(token_id, owner);
            let owner_balance = self.balance_of(owner);
            self.owner_token_count.insert(owner, owner_balance + 1);

            self.token_properties.insert(token_id, property_info);
            self.property_tokens.insert(property_id, token_id);

            self.total_supply += 1;
            self.token_counter += 1;

            self.env().emit_event(PropertyTokenMinted {
                token_id,
                property_id,
                owner,
            });

            Ok(token_id)
        }
    }
}