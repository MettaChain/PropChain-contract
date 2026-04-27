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
use ink::storage::Mapping;
use propchain_contracts::{non_reentrant, ReentrancyError, ReentrancyGuard};
use propchain_traits::*;
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
        /// Vesting schedules for tokens (TokenId, AccountId)
        vesting_schedules: Mapping<(TokenId, AccountId), VestingSchedule>,
        /// Custom URI overrides for tokens
        token_uris: Mapping<TokenId, String>,

        /// Reentrancy protection guard
        reentrancy_guard: ReentrancyGuard,
        /// Snapshot functionality for governance voting (Issue #194)
        snapshot_counter: Mapping<TokenId, u64>,
        snapshots: Mapping<(TokenId, u64), Snapshot>,
        account_snapshots: Mapping<(AccountId, TokenId, u64), u128>, // (account, token_id, snapshot_id) -> balance

        // Staking fields (Issue #197)
        /// Staking information per (staker, token_id)
        share_stakes: Mapping<(AccountId, TokenId), ShareStakeInfo>,
        /// Total staked shares per token
        share_total_staked: Mapping<TokenId, u128>,
        /// Accumulated reward per share (scaled by STAKE_SCALING)
        share_acc_reward_per_share: Mapping<TokenId, u128>,
        /// Last block number when rewards were calculated
        share_last_reward_block: Mapping<TokenId, u64>,
        /// Reward rate in basis points per year
        share_reward_rate_bps: Mapping<TokenId, u128>,
        /// Reward pool balance per token
        share_reward_pool: Mapping<TokenId, u128>,
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
                share_reward_rate_bps: Mapping::default(),
                share_reward_pool: Mapping::default(),
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

            if ids.len() != amounts.len() {
                return Err(Error::LengthMismatch);
            }

            if ids.is_empty() {
                return Err(Error::InvalidAmount);
            }

            // Validate all balances first (fail fast, no partial state)
            for i in 0..ids.len() {
                let from_balance = self.balances.get((&from, &ids[i])).unwrap_or(0);
                if from_balance < amounts[i] {
                    return Err(Error::InsufficientBalance);
                }
                if amounts[i] == 0 {
                    return Err(Error::InvalidAmount);
                }
            }

            // Execute all transfers
            for i in 0..ids.len() {
                let token_id = ids[i];
                let amount = amounts[i];
                let from_balance = self.balances.get((&from, &token_id)).unwrap_or(0);
                self.balances
                    .insert((&from, &token_id), &(from_balance - amount));
                let to_balance = self.balances.get((&to, &token_id)).unwrap_or(0);
                self.balances
                    .insert((&to, &token_id), &(to_balance + amount));
            }

            // Single batch event instead of N individual events
            self.env().emit_event(BatchTransfer {
                from: Some(from),
                to: Some(to),
                ids,
                amounts,
            });

            Ok(())
        }

        /// ERC-1155: Returns the URI for a token
        #[ink(message)]
        pub fn uri(&self, token_id: TokenId) -> Option<String> {
            // First check if there is a custom URI override
            if let Some(custom_uri) = self.token_uris.get(token_id) {
                return Some(custom_uri);
            }
            // Return a standard URI format for the token metadata
            let _property_info = self.token_properties.get(token_id)?;
            Some(format!(
                "ipfs://property/{:?}/{}/metadata.json",
                self.env().account_id(),
                token_id
            ))
        }

        /// Sets the compliance registry contract address (admin only).
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

            non_reentrant!(self, {
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
            })
        }

        /// Deposits dividends for distribution to all share holders of a token.
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
        #[ink(message)]
        pub fn withdraw_dividends(&mut self, token_id: TokenId) -> Result<u128, Error> {
            non_reentrant!(self, {
                let caller = self.env().caller();
                self.update_dividend_credit_on_change(caller, token_id)?;
                let owed = self.dividend_balance.get((caller, token_id)).unwrap_or(0);
                if owed == 0 {
                    return Ok(0);
                }
                self.dividend_balance.insert((caller, token_id), &0u128);
                match self.env().transfer(caller, owed) {
                    Ok(_) => {
                        let mut rec =
                            self.tax_records
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
            })
        }

        /// Creates a governance proposal for a tokenized property.
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

        /// Returns the proposal record for `token_id` and `proposal_id`, if it exists.
        #[ink(message)]
        pub fn get_proposal(&self, token_id: TokenId, proposal_id: u64) -> Option<Proposal> {
            self.proposals.get((token_id, proposal_id))
        }

        /// Creates a snapshot for the property token to capture governance state.
        #[ink(message)]
        pub fn create_snapshot(
            &mut self,
            token_id: TokenId,
            description: String,
        ) -> Result<u64, Error> {
            if self.token_owner.get(token_id).is_none() {
                return Err(Error::TokenNotFound);
            }
            let snapshot_id = self
                .snapshot_counter
                .get(token_id)
                .unwrap_or(0)
                .saturating_add(1);
            self.snapshot_counter.insert(token_id, &snapshot_id);
            let snapshot = Snapshot {
                id: snapshot_id,
                token_id,
                created_at: self.env().block_timestamp(),
                total_supply_at_snapshot: self.total_supply as u128,
                description: description.clone(),
            };
            self.snapshots.insert((token_id, snapshot_id), &snapshot);
            self.env().emit_event(SnapshotCreated {
                token_id,
                snapshot_id,
                total_supply: self.total_supply,
                description,
            });
            Ok(snapshot_id)
        }

        /// Records the balance of an account for a specific snapshot.
        #[ink(message)]
        pub fn record_snapshot_balance(
            &mut self,
            token_id: TokenId,
            snapshot_id: u64,
            account: AccountId,
        ) -> Result<u128, Error> {
            if self.snapshots.get((token_id, snapshot_id)).is_none() {
                return Err(Error::InvalidRequest);
            }
            let balance = self.balances.get((account, token_id)).unwrap_or(0);
            self.account_snapshots
                .insert((account, token_id, snapshot_id), &balance);
            self.env().emit_event(SnapshotBalanceQueried {
                token_id,
                snapshot_id,
                account,
                balance,
            });
            Ok(balance)
        }

        /// Returns the recorded snapshot balance for an account.
        #[ink(message)]
        pub fn get_balance_at_snapshot(
            &self,
            token_id: TokenId,
            snapshot_id: u64,
            account: AccountId,
        ) -> Result<u128, Error> {
            let balance = self
                .account_snapshots
                .get((account, token_id, snapshot_id))
                .unwrap_or(0);
            Ok(balance)
        }

        /// Returns snapshot metadata by token and snapshot ID.
        #[ink(message)]
        pub fn get_snapshot(&self, token_id: TokenId, snapshot_id: u64) -> Option<Snapshot> {
            self.snapshots.get((token_id, snapshot_id))
        }

        /// Returns the latest snapshot ID for a token.
        #[ink(message)]
        pub fn latest_snapshot_id(&self, token_id: TokenId) -> u64 {
            self.snapshot_counter.get(token_id).unwrap_or(0)
        }

        /// Places a sell order (ask) for fractional shares on the marketplace.
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
        #[ink(message, payable)]
        pub fn buy_shares(
            &mut self,
            token_id: TokenId,
            seller: AccountId,
            amount: u128,
        ) -> Result<(), Error> {
            non_reentrant!(self, {
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
                        let mut rec =
                            self.tax_records
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
            })
        }

        /// Returns the last trade price per share for a token, if any trades have occurred.
        #[ink(message)]
        pub fn get_last_trade_price(&self, token_id: TokenId) -> Option<u128> {
            self.last_trade_price.get(token_id)
        }

        /// Returns a portfolio summary for a set of tokens owned by an account.
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

        // =========================================================================
        // Metadata Methods
        // =========================================================================

        /// Updates the on-chain metadata for a property
        #[ink(message)]
        pub fn update_property_metadata(
            &mut self,
            token_id: TokenId,
            metadata: PropertyMetadata,
        ) -> Result<(), Error> {
            let caller = self.env().caller();
            let owner = self.token_owner.get(token_id).ok_or(Error::TokenNotFound)?;
            if caller != self.admin && caller != owner {
                return Err(Error::Unauthorized);
            }

            let mut property_info = self
                .token_properties
                .get(token_id)
                .ok_or(Error::TokenNotFound)?;
            property_info.metadata = metadata;
            self.token_properties.insert(token_id, &property_info);

            self.env().emit_event(MetadataUpdated {
                token_id,
                updated_by: caller,
            });

            Ok(())
        }

        /// Sets a custom URI for a token, overriding the default generated format
        #[ink(message)]
        pub fn set_token_uri(&mut self, token_id: TokenId, new_uri: String) -> Result<(), Error> {
            let caller = self.env().caller();
            let owner = self.token_owner.get(token_id).ok_or(Error::TokenNotFound)?;
            if caller != self.admin && caller != owner {
                return Err(Error::Unauthorized);
            }

            self.token_uris.insert(token_id, &new_uri);

            self.env().emit_event(TokenURIUpdated {
                token_id,
                updated_by: caller,
                new_uri,
            });

            Ok(())
        }

        // =========================================================================
        // Returns the tax record for an account and token, summarizing dividends and sales.
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

            self.token_counter += 1;
            let token_id = self.token_counter;

            let property_info = PropertyInfo {
                id: token_id,
                owner: caller,
                metadata: metadata.clone(),
                registered_at: self.env().block_timestamp(),
            };

            self.token_owner.insert(token_id, &caller);
            self.add_token_to_owner(caller, token_id)?;

            self.balances.insert((&caller, &token_id), &1u128);

            self.token_properties.insert(token_id, &property_info);
            self.property_tokens.insert(token_id, &token_id);

            let initial_transfer = OwnershipTransfer {
                from: AccountId::from([0u8; 32]),
                to: caller,
                timestamp: self.env().block_timestamp(),
                transaction_hash: propchain_traits::crypto::hash_encoded(&(&caller, token_id)),
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

            let document_count = self.legal_documents_count.get(token_id).unwrap_or(0);

            let document_info = DocumentInfo {
                document_hash,
                document_type: document_type.clone(),
                upload_date: self.env().block_timestamp(),
                uploader: caller,
            };

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

            if token_owner != caller {
                return Err(Error::Unauthorized);
            }

            if self.bridge_config.emergency_pause {
                return Err(Error::BridgePaused);
            }

            if !self
                .bridge_config
                .supported_chains
                .contains(&destination_chain)
            {
                return Err(Error::InvalidChain);
            }

            let compliance_info = self
                .compliance_flags
                .get(token_id)
                .ok_or(Error::ComplianceFailed)?;
            if !compliance_info.verified {
                return Err(Error::ComplianceFailed);
            }

            if required_signatures < self.bridge_config.min_signatures_required
                || required_signatures > self.bridge_config.max_signatures_required
            {
                return Err(Error::InsufficientSignatures);
            }

            if self.has_pending_bridge_request(token_id) {
                return Err(Error::DuplicateBridgeRequest);
            }

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
                source_chain: self.current_chain,
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

            if !self.bridge_operators.contains(&caller) {
                return Err(Error::Unauthorized);
            }

            let mut request = self
                .bridge_requests
                .get(request_id)
                .ok_or(Error::InvalidRequest)?;

            if let Some(expires_at) = request.expires_at {
                if u64::from(self.env().block_number()) > expires_at {
                    request.status = BridgeOperationStatus::Expired;
                    self.bridge_requests.insert(request_id, &request);
                    return Err(Error::RequestExpired);
                }
            }

            if request.signatures.contains(&caller) {
                return Err(Error::AlreadySigned);
            }

            request.signatures.push(caller);

            if !approve {
                request.status = BridgeOperationStatus::Failed;
                self.env().emit_event(BridgeFailed {
                    request_id,
                    token_id: request.token_id,
                    error: String::from("Request rejected by operator"),
                });
            } else if request.signatures.len() >= request.required_signatures as usize {
                request.status = BridgeOperationStatus::Locked;

                let token_owner = self
                    .token_owner
                    .get(request.token_id)
                    .ok_or(Error::TokenNotFound)?;
                self.remove_token_from_owner(token_owner, request.token_id)?;
                self.balances
                    .insert((&token_owner, &request.token_id), &0u128);
                self.token_owner
                    .insert(request.token_id, &AccountId::from([0u8; 32]));
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

            if !self.bridge_operators.contains(&caller) {
                return Err(Error::Unauthorized);
            }

            let mut request = self
                .bridge_requests
                .get(request_id)
                .ok_or(Error::InvalidRequest)?;

            if request.status != BridgeOperationStatus::Locked {
                return Err(Error::InvalidRequest);
            }

            if request.signatures.len() < request.required_signatures as usize {
                return Err(Error::InsufficientSignatures);
            }

            let transaction_hash = self.generate_bridge_transaction_hash(&request);

            self.transaction_counter += 1;
            let transaction = BridgeTransaction {
                transaction_id: self.transaction_counter,
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

            request.status = BridgeOperationStatus::Completed;
            self.bridge_requests.insert(request_id, &request);

            self.verified_bridge_hashes.insert(transaction_hash, &true);

            let mut history = self
                .bridge_transactions
                .get(request.sender)
                .unwrap_or_default();
            history.push(transaction.clone());
            self.bridge_transactions.insert(request.sender, &history);

            let bridged_info = BridgedTokenInfo {
                original_chain: request.source_chain,
                original_token_id: request.token_id,
                destination_chain: request.destination_chain,
                destination_token_id: request.token_id,
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
            let caller = self.env().caller();
            if !self.bridge_operators.contains(&caller) {
                return Err(Error::Unauthorized);
            }

            if !self
                .verified_bridge_hashes
                .get(transaction_hash)
                .unwrap_or(false)
            {
                if !self.bridge_operators.contains(&caller) {
                    return Err(Error::Unauthorized);
                }
                self.verified_bridge_hashes.insert(transaction_hash, &true);
            }

            self.token_counter += 1;
            let new_token_id = self.token_counter;

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

            let initial_transfer = OwnershipTransfer {
                from: AccountId::from([0u8; 32]),
                to: recipient,
                timestamp: self.env().block_timestamp(),
                transaction_hash: propchain_traits::crypto::hash_encoded(&(
                    &recipient,
                    new_token_id,
                )),
            };

            self.ownership_history_count.insert(new_token_id, &1u32);
            self.ownership_history_items
                .insert((new_token_id, 0), &initial_transfer);

            let compliance_info = ComplianceInfo {
                verified: true,
                verification_date: self.env().block_timestamp(),
                verifier: caller,
                compliance_type: String::from("Bridge"),
            };
            self.compliance_flags.insert(new_token_id, &compliance_info);

            self.legal_documents_count.insert(new_token_id, &0u32);

            self.total_supply += 1;
            self.bridged_token_origins
                .insert(new_token_id, &(source_chain, original_token_id));

            let mut bridged_info = self
                .bridged_tokens
                .get((&source_chain, &original_token_id))
                .unwrap_or(BridgedTokenInfo {
                    original_chain: source_chain,
                    original_token_id,
                    destination_chain: self.current_chain,
                    destination_token_id: new_token_id,
                    bridged_at: self.env().block_timestamp(),
                    status: BridgingStatus::Completed,
                });
            bridged_info.status = BridgingStatus::Completed;
            bridged_info.destination_token_id = new_token_id;
            bridged_info.destination_chain = self.current_chain;
            self.bridged_tokens
                .insert((&source_chain, &original_token_id), &bridged_info);

            self.env().emit_event(Transfer {
                from: None,
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

            if token_owner != caller {
                return Err(Error::Unauthorized);
            }

            let (source_chain, original_token_id) = self
                .bridged_token_origins
                .get(token_id)
                .ok_or(Error::BridgeNotSupported)?;

            let bridged_info = self
                .bridged_tokens
                .get((source_chain, &original_token_id))
                .ok_or(Error::BridgeNotSupported)?;

            if bridged_info.status != BridgingStatus::Completed {
                return Err(Error::InvalidRequest);
            }

            self.remove_token_from_owner(caller, token_id)?;
            self.token_owner.remove(token_id);
            self.balances.insert((&caller, &token_id), &0u128);
            self.total_supply -= 1;

            if destination_chain != source_chain {
                return Err(Error::InvalidChain);
            }

            let mut updated_info = bridged_info;
            updated_info.status = BridgingStatus::InTransit;
            self.bridged_tokens
                .insert((source_chain, &original_token_id), &updated_info);
            self.bridged_token_origins.remove(token_id);

            self.env().emit_event(Transfer {
                from: Some(caller),
                to: None,
                id: token_id,
            });

            Ok(())
        }

        /// Burn a token for supply management purposes.
        ///
        /// Only the contract admin can burn tokens. This is used for supply management,
        /// such as removing tokens from circulation, handling regulatory requirements,
        /// or managing tokenomics.
        ///
        /// # Arguments
        /// * `token_id` - The ID of the token to burn
        /// * `reason` - A description of why the token is being burned (for audit trail)
        ///
        /// # Requirements
        /// * Caller must be the contract admin
        /// * Token must exist
        /// * Token must not be locked in a bridge operation
        ///
        /// # Effects
        /// * Removes token from owner's balance
        /// * Decrements total supply
        /// * Clears all token approvals
        /// * Emits `Transfer` event (from owner to zero address)
        /// * Emits `TokenBurned` event with reason
        #[ink(message)]
        pub fn burn(&mut self, token_id: TokenId, reason: String) -> Result<(), Error> {
            let caller = self.env().caller();

            // Only admin can burn tokens
            if caller != self.admin {
                return Err(Error::Unauthorized);
            }

            // Check token exists
            let token_owner = self.token_owner.get(token_id).ok_or(Error::TokenNotFound)?;

            // Check token is not locked in bridge
            if self.has_pending_bridge_request(token_id) {
                return Err(Error::BridgeLocked);
            }

            // Remove token from owner
            self.remove_token_from_owner(token_owner, token_id)?;

            // Clear token ownership
            self.token_owner.remove(token_id);

            // Clear approvals
            self.token_approvals.remove(token_id);

            // Clear balances
            self.balances.insert((&token_owner, &token_id), &0u128);

            // Decrement total supply
            self.total_supply = self.total_supply.saturating_sub(1);

            // Emit Transfer event (to zero address indicates burn)
            self.env().emit_event(Transfer {
                from: Some(token_owner),
                to: None,
                id: token_id,
            });

            // Emit TokenBurned event with reason for audit trail
            self.env().emit_event(TokenBurned {
                token_id,
                burned_by: caller,
                reason,
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

            if caller != self.admin {
                return Err(Error::Unauthorized);
            }

            let mut request = self
                .bridge_requests
                .get(request_id)
                .ok_or(Error::InvalidRequest)?;

            if !matches!(
                request.status,
                BridgeOperationStatus::Failed | BridgeOperationStatus::Expired
            ) {
                return Err(Error::InvalidRequest);
            }

            match recovery_action {
                RecoveryAction::UnlockToken => {
                    if let Some(token_owner) = self.token_owner.get(request.token_id) {
                        if token_owner == AccountId::from([0u8; 32]) {
                            self.token_owner.insert(request.token_id, &request.sender);
                            self.balances
                                .insert((&request.sender, &request.token_id), &1u128);
                            self.add_token_to_owner(request.sender, request.token_id)?;
                        }
                    }
                }
                RecoveryAction::RefundGas => {
                    // Gas refund logic would be implemented here
                }
                RecoveryAction::RetryBridge => {
                    request.status = BridgeOperationStatus::Pending;
                    request.signatures.clear();
                }
                RecoveryAction::CancelBridge => {
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
            if let Some((source_chain, original_token_id)) =
                self.bridged_token_origins.get(token_id)
            {
                if let Some(bridged_info) =
                    self.bridged_tokens.get((source_chain, &original_token_id))
                {
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

        /// Gets the current chain ID for this contract instance
        #[ink(message)]
        pub fn get_current_chain_id(&self) -> ChainId {
            self.current_chain
        }

        /// Sets the current chain ID for this contract instance (admin only)
        #[ink(message)]
        pub fn set_current_chain_id(&mut self, chain_id: ChainId) -> Result<(), Error> {
            let caller = self.env().caller();
            if caller != self.admin {
                return Err(Error::Unauthorized);
            }
            self.current_chain = chain_id;
            Ok(())
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
                transaction_hash: propchain_traits::crypto::hash_encoded(&(&from, &to, token_id)),
            };

            self.ownership_history_items
                .insert((token_id, count), &transfer_record);
            self.ownership_history_count.insert(token_id, &(count + 1));

            Ok(())
        }

        /// Helper to check if token has pending bridge request
        fn has_pending_bridge_request(&self, token_id: TokenId) -> bool {
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
            let data = (
                request.request_id,
                request.token_id,
                request.source_chain,
                request.destination_chain,
                request.sender,
                request.recipient,
                self.env().block_timestamp(),
            );
            propchain_traits::crypto::hash_encoded(&data)
        }

        /// Helper to estimate bridge gas usage
        fn estimate_bridge_gas_usage(&self, request: &MultisigBridgeRequest) -> u64 {
            let base_gas = 100000;
            let metadata_gas = request.metadata.legal_description.len() as u64 * 100;
            let signature_gas = request.required_signatures as u64 * 5000;
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

            let key = (account, error_code.clone());
            let current_count = self.error_counts.get(&key).unwrap_or(0);
            self.error_counts.insert(&key, &(current_count + 1));

            let window_duration = 3_600_000_u64;
            let rate_key = error_code.clone();
            let (mut count, window_start) =
                self.error_rates.get(&rate_key).unwrap_or((0, timestamp));

            if timestamp >= window_start + window_duration {
                count = 1;
                self.error_rates.insert(&rate_key, &(count, timestamp));
            } else {
                count += 1;
                self.error_rates.insert(&rate_key, &(count, window_start));
            }

            let log_id = self.error_log_counter;
            self.error_log_counter = self.error_log_counter.wrapping_add(1);

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
            let window_duration = 3_600_000_u64;

            if let Some((count, window_start)) = self.error_rates.get(&error_code) {
                if timestamp >= window_start + window_duration {
                    0
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

        // ── Staking public interface (Issue #197) ──────────────────────────

        /// Locks `amount` fractional shares of `token_id` for the lock period.
        #[ink(message)]
        pub fn stake_shares(
            &mut self,
            token_id: TokenId,
            amount: u128,
            lock_period: LockPeriod,
        ) -> Result<(), Error> {
            if amount == 0 {
                return Err(Error::InvalidAmount);
            }
            let caller = self.env().caller();
            if self.share_stakes.get((caller, token_id)).is_some() {
                return Err(Error::AlreadyStaked);
            }
            let bal = self.balances.get((caller, token_id)).unwrap_or(0);
            if bal < amount {
                return Err(Error::InsufficientBalance);
            }
            self.update_dividend_credit_on_change(caller, token_id)?;
            self.balances
                .insert((caller, token_id), &(bal.saturating_sub(amount)));
            self.update_stake_acc_reward(token_id);
            let acc = self.share_acc_reward_per_share.get(token_id).unwrap_or(0);
            let now = self.env().block_number() as u64;
            let lock_until = now.saturating_add(lock_period.duration_blocks());
            let stake = ShareStakeInfo {
                staker: caller,
                token_id,
                amount,
                staked_at: now,
                lock_until,
                lock_period,
                reward_debt: acc,
            };
            self.share_stakes.insert((caller, token_id), &stake);
            let total = self.share_total_staked.get(token_id).unwrap_or(0);
            self.share_total_staked
                .insert(token_id, &total.saturating_add(amount));
            self.env().emit_event(SharesStaked {
                token_id,
                staker: caller,
                amount,
                lock_period,
                lock_until,
            });
            Ok(())
        }

        /// Unlocks and returns staked shares; pending rewards are auto-claimed.
        #[ink(message)]
        pub fn unstake_shares(&mut self, token_id: TokenId) -> Result<(), Error> {
            non_reentrant!(self, {
                let caller = self.env().caller();
                let stake = self
                    .share_stakes
                    .get((caller, token_id))
                    .ok_or(Error::StakeNotFound)?;
                let now = self.env().block_number() as u64;
                if now < stake.lock_until {
                    return Err(Error::LockActive);
                }
                self.update_stake_acc_reward(token_id);
                let stake = self
                    .share_stakes
                    .get((caller, token_id))
                    .ok_or(Error::StakeNotFound)?;
                let rewards = self.pending_stake_rewards(&stake);
                if rewards > 0 {
                    let pool = self.share_reward_pool.get(token_id).unwrap_or(0);
                    if pool >= rewards {
                        self.share_reward_pool
                            .insert(token_id, &pool.saturating_sub(rewards));
                        let _ = self.env().transfer(caller, rewards);
                        self.env().emit_event(StakeRewardsClaimed {
                            token_id,
                            staker: caller,
                            amount: rewards,
                        });
                    }
                }
                let amount = stake.amount;
                self.update_dividend_credit_on_change(caller, token_id)?;
                let bal = self.balances.get((caller, token_id)).unwrap_or(0);
                self.balances
                    .insert((caller, token_id), &bal.saturating_add(amount));
                self.share_stakes.remove((caller, token_id));
                let total = self.share_total_staked.get(token_id).unwrap_or(0);
                self.share_total_staked
                    .insert(token_id, &total.saturating_sub(amount));
                self.env().emit_event(SharesUnstaked {
                    token_id,
                    staker: caller,
                    amount,
                });
                Ok(())
            })
        }

        /// Claims accrued staking rewards for `token_id` without unstaking.
        #[ink(message)]
        pub fn claim_stake_rewards(&mut self, token_id: TokenId) -> Result<u128, Error> {
            non_reentrant!(self, {
                let caller = self.env().caller();
                if self.share_stakes.get((caller, token_id)).is_none() {
                    return Err(Error::StakeNotFound);
                }
                self.update_stake_acc_reward(token_id);
                let stake = self
                    .share_stakes
                    .get((caller, token_id))
                    .ok_or(Error::StakeNotFound)?;
                let rewards = self.pending_stake_rewards(&stake);
                if rewards == 0 {
                    return Err(Error::NoRewards);
                }
                let pool = self.share_reward_pool.get(token_id).unwrap_or(0);
                if pool < rewards {
                    return Err(Error::InsufficientRewardPool);
                }
                self.share_reward_pool
                    .insert(token_id, &pool.saturating_sub(rewards));
                let new_acc = self.share_acc_reward_per_share.get(token_id).unwrap_or(0);
                let mut updated = stake.clone();
                updated.reward_debt = new_acc;
                self.share_stakes.insert((caller, token_id), &updated);
                let _ = self.env().transfer(caller, rewards);
                self.env().emit_event(StakeRewardsClaimed {
                    token_id,
                    staker: caller,
                    amount: rewards,
                });
                Ok(rewards)
            })
        }

        /// Adds funds to the staking reward pool for `token_id`.
        #[ink(message, payable)]
        pub fn fund_stake_reward_pool(&mut self, token_id: TokenId) -> Result<(), Error> {
            if self.token_owner.get(token_id).is_none() {
                return Err(Error::TokenNotFound);
            }
            let amount = self.env().transferred_value();
            if amount == 0 {
                return Err(Error::InvalidAmount);
            }
            let pool = self.share_reward_pool.get(token_id).unwrap_or(0);
            self.share_reward_pool
                .insert(token_id, &pool.saturating_add(amount));
            let funder = self.env().caller();
            self.env().emit_event(StakeRewardPoolFunded {
                token_id,
                funder,
                amount,
            });
            Ok(())
        }

        /// Sets the annual reward rate in basis points for `token_id` (admin only).
        #[ink(message)]
        pub fn set_stake_reward_rate(
            &mut self,
            token_id: TokenId,
            rate_bps: u128,
        ) -> Result<(), Error> {
            if self.env().caller() != self.admin {
                return Err(Error::Unauthorized);
            }
            self.update_stake_acc_reward(token_id);
            self.share_reward_rate_bps.insert(token_id, &rate_bps);
            Ok(())
        }

        /// Returns the staking record for `staker` on `token_id`, if any.
        #[ink(message)]
        pub fn get_share_stake(
            &self,
            staker: AccountId,
            token_id: TokenId,
        ) -> Option<ShareStakeInfo> {
            self.share_stakes.get((staker, token_id))
        }

        /// Returns the pending (unclaimed) staking rewards for `staker` on `token_id`.
        #[ink(message)]
        pub fn get_pending_stake_rewards(&self, staker: AccountId, token_id: TokenId) -> u128 {
            match self.share_stakes.get((staker, token_id)) {
                Some(stake) => self.pending_stake_rewards(&stake),
                None => 0,
            }
        }

        /// Returns the effective governance voting weight for `voter` on `token_id`.
        #[ink(message)]
        pub fn get_governance_weight(&self, voter: AccountId, token_id: TokenId) -> u128 {
            self.governance_weight(voter, token_id)
        }

        // ── Staking private helpers (Issue #197) ──────────────────────────

        const STAKE_SCALING: u128 = 1_000_000_000_000;
        const REWARD_RATE_PRECISION: u128 = 10_000; // Basis points precision

        fn update_stake_acc_reward(&mut self, token_id: TokenId) {
            let total = self.share_total_staked.get(token_id).unwrap_or(0);
            if total == 0 {
                return;
            }
            let now = self.env().block_number() as u64;
            let last = self.share_last_reward_block.get(token_id).unwrap_or(now);
            let blocks = (now as u128).saturating_sub(last as u128);
            if blocks == 0 {
                return;
            }
            let rate = self.share_reward_rate_bps.get(token_id).unwrap_or(0);
            let reward = total.saturating_mul(rate).saturating_mul(blocks)
                / Self::REWARD_RATE_PRECISION
                / 5_256_000;
            let acc = self.share_acc_reward_per_share.get(token_id).unwrap_or(0);
            self.share_acc_reward_per_share.insert(
                token_id,
                &acc.saturating_add(reward.saturating_mul(Self::STAKE_SCALING) / total),
            );
            self.share_last_reward_block.insert(token_id, &now);
        }

        fn pending_stake_rewards(&self, stake: &ShareStakeInfo) -> u128 {
            let acc = self
                .share_acc_reward_per_share
                .get(stake.token_id)
                .unwrap_or(0);
            let base = stake
                .amount
                .saturating_mul(acc.saturating_sub(stake.reward_debt))
                / Self::STAKE_SCALING;
            base.saturating_mul(stake.lock_period.multiplier()) / 100
        }

        fn governance_weight(&self, voter: AccountId, token_id: TokenId) -> u128 {
            if let Some(stake) = self.share_stakes.get((voter, token_id)) {
                stake.amount.saturating_mul(stake.lock_period.multiplier()) / 100
            } else {
                self.balances.get((voter, token_id)).unwrap_or(0)
            }
        }

        // =========================================================================
        // Vesting Methods
        // =========================================================================

        /// Creates a vesting schedule for an account
        #[ink(message)]
        #[allow(clippy::too_many_arguments)]
        pub fn create_vesting_schedule(
            &mut self,
            token_id: TokenId,
            account: AccountId,
            role: VestingRole,
            total_amount: u128,
            start_time: u64,
            cliff_duration: u64,
            vesting_duration: u64,
        ) -> Result<(), Error> {
            let caller = self.env().caller();
            let owner = self.token_owner.get(token_id).ok_or(Error::TokenNotFound)?;
            if caller != self.admin && caller != owner {
                return Err(Error::Unauthorized);
            }
            if total_amount == 0 {
                return Err(Error::InvalidAmount);
            }
            if self.vesting_schedules.get((token_id, account)).is_some() {
                return Err(Error::Unauthorized);
            }

            let creator_balance = self.balances.get((caller, token_id)).unwrap_or(0);
            if creator_balance < total_amount {
                return Err(Error::Unauthorized);
            }
            self.balances
                .insert((caller, token_id), &(creator_balance - total_amount));

            let schedule = VestingSchedule {
                role: role.clone(),
                total_amount,
                claimed_amount: 0,
                start_time,
                cliff_duration,
                vesting_duration,
            };

            self.vesting_schedules
                .insert((token_id, account), &schedule);

            self.env().emit_event(VestingScheduleCreated {
                token_id,
                account,
                role,
                total_amount,
                start_time,
                cliff_duration,
                vesting_duration,
            });

            Ok(())
        }

        /// Claims available vested tokens
        #[ink(message)]
        pub fn claim_vested_tokens(&mut self, token_id: TokenId) -> Result<(), Error> {
            let caller = self.env().caller();
            let mut schedule = self
                .vesting_schedules
                .get((token_id, caller))
                .ok_or(Error::Unauthorized)?;

            let current_time = self.env().block_timestamp();

            let vested_amount = if current_time < schedule.start_time + schedule.cliff_duration {
                0
            } else if current_time >= schedule.start_time + schedule.vesting_duration {
                schedule.total_amount
            } else {
                let time_vested = current_time - schedule.start_time;
                (schedule.total_amount * time_vested as u128) / (schedule.vesting_duration as u128)
            };

            let claimable = vested_amount.saturating_sub(schedule.claimed_amount);
            if claimable == 0 {
                return Err(Error::InvalidAmount);
            }

            schedule.claimed_amount += claimable;
            self.vesting_schedules.insert((token_id, caller), &schedule);

            let current_balance = self.balances.get((caller, token_id)).unwrap_or(0);
            self.balances
                .insert((caller, token_id), &(current_balance + claimable));

            self.env().emit_event(VestedTokensClaimed {
                token_id,
                account: caller,
                amount: claimable,
            });

            Ok(())
        }

        /// Gets the vesting schedule for an account
        #[ink(message)]
        pub fn get_vesting_schedule(
            &self,
            token_id: TokenId,
            account: AccountId,
        ) -> Option<VestingSchedule> {
            self.vesting_schedules.get((token_id, account))
        }

        /// Calculates the amount of tokens currently vested
        #[ink(message)]
        pub fn get_vested_amount(&self, token_id: TokenId, account: AccountId) -> u128 {
            if let Some(schedule) = self.vesting_schedules.get((token_id, account)) {
                let current_time = self.env().block_timestamp();
                if current_time < schedule.start_time + schedule.cliff_duration {
                    0
                } else if current_time >= schedule.start_time + schedule.vesting_duration {
                    schedule.total_amount
                } else {
                    let time_vested = current_time - schedule.start_time;
                    (schedule.total_amount * time_vested as u128)
                        / (schedule.vesting_duration as u128)
                }
            } else {
                0
            }
        }
    }

    // Unit tests extracted to tests.rs (Issue #101)
    include!("../tests.rs");
}
