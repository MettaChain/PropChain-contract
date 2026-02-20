#![cfg_attr(not(feature = "std"), no_std)]
#![allow(unexpected_cfgs)]

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
        TokenNotFound,
        Unauthorized,
        // Property-specific errors
        PropertyNotFound,
        InvalidMetadata,
        DocumentNotFound,
        ComplianceFailed,
        // Cross-chain bridge errors
        BridgeNotSupported,
        InvalidChain,
        BridgeLocked,
        InsufficientSignatures,
        RequestExpired,
        InvalidRequest,
        BridgePaused,
        GasLimitExceeded,
        MetadataCorruption,
        InvalidBridgeOperator,
        DuplicateBridgeRequest,
        BridgeTimeout,
        AlreadySigned,
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
        ownership_history: Mapping<TokenId, Vec<OwnershipTransfer>>,
        compliance_flags: Mapping<TokenId, ComplianceInfo>,
        legal_documents: Mapping<TokenId, Vec<DocumentInfo>>,

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
                ownership_history: Mapping::default(),
                compliance_flags: Mapping::default(),
                legal_documents: Mapping::default(),

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
            }
        }

        /// ERC-721: Returns the balance of tokens owned by an account
        #[ink(message)]
        pub fn balance_of(&self, owner: AccountId) -> u32 {
            self.owner_token_count.get(&owner).unwrap_or(0)
        }

        /// ERC-721: Returns the owner of a token
        #[ink(message)]
        pub fn owner_of(&self, token_id: TokenId) -> Option<AccountId> {
            self.token_owner.get(&token_id)
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
            let token_owner = self
                .token_owner
                .get(&token_id)
                .ok_or(Error::TokenNotFound)?;
            if token_owner != from {
                return Err(Error::Unauthorized);
            }

            if caller != from
                && Some(caller) != self.token_approvals.get(&token_id)
                && !self.is_approved_for_all(from, caller)
            {
                return Err(Error::Unauthorized);
            }

            // Perform the transfer
            self.remove_token_from_owner(from, token_id)?;
            self.add_token_to_owner(to, token_id)?;

            // Clear approvals
            self.token_approvals.remove(&token_id);

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
            let token_owner = self
                .token_owner
                .get(&token_id)
                .ok_or(Error::TokenNotFound)?;

            if token_owner != caller && !self.is_approved_for_all(token_owner, caller) {
                return Err(Error::Unauthorized);
            }

            self.token_approvals.insert(&token_id, &to);

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
            self.token_approvals.get(&token_id)
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
            data: Vec<u8>,
        ) -> Result<(), Error> {
            let caller = self.env().caller();

            if from != caller && !self.is_approved_for_all(from, caller) {
                return Err(Error::Unauthorized);
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
            for i in 0..ids.len() {
                self.env().emit_event(Transfer {
                    from: Some(from),
                    to: Some(to),
                    id: ids[i],
                });
            }

            Ok(())
        }

        /// ERC-1155: Returns the URI for a token
        #[ink(message)]
        pub fn uri(&self, token_id: TokenId) -> Option<String> {
            // Return a standard URI format for the token metadata
            let property_info = self.token_properties.get(&token_id)?;
            Some(format!(
                "ipfs://property/{:?}/{}/metadata.json",
                self.env().account_id(),
                token_id
            ))
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

            self.token_owner.insert(&token_id, &caller);
            self.add_token_to_owner(caller, token_id)?;

            // Initialize balances
            self.balances.insert((&caller, &token_id), &1u128);

            // Store property-specific information
            self.token_properties.insert(&token_id, &property_info);
            self.property_tokens.insert(&token_id, &token_id); // property_id maps to token_id

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

            self.ownership_history
                .insert(&token_id, &vec![initial_transfer]);

            // Initialize compliance as unverified
            let compliance_info = ComplianceInfo {
                verified: false,
                verification_date: 0,
                verifier: AccountId::from([0u8; 32]),
                compliance_type: String::from("KYC"),
            };
            self.compliance_flags.insert(&token_id, &compliance_info);

            // Initialize legal documents vector
            self.legal_documents
                .insert(&token_id, &Vec::<DocumentInfo>::new());

            self.total_supply += 1;

            self.env().emit_event(PropertyTokenMinted {
                token_id,
                property_id: token_id,
                owner: caller,
            });

            Ok(token_id)
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
            let token_owner = self
                .token_owner
                .get(&token_id)
                .ok_or(Error::TokenNotFound)?;

            if token_owner != caller {
                return Err(Error::Unauthorized);
            }

            // Get existing documents
            let mut documents = self.legal_documents.get(&token_id).unwrap_or(Vec::new());

            // Add new document
            let document_info = DocumentInfo {
                document_hash,
                document_type: document_type.clone(),
                upload_date: self.env().block_timestamp(),
                uploader: caller,
            };

            documents.push(document_info);

            // Save updated documents
            self.legal_documents.insert(&token_id, &documents);

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
                .get(&token_id)
                .ok_or(Error::TokenNotFound)?;
            compliance_info.verified = verification_status;
            compliance_info.verification_date = self.env().block_timestamp();
            compliance_info.verifier = caller;

            self.compliance_flags.insert(&token_id, &compliance_info);

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
            self.ownership_history.get(&token_id)
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
            let token_owner = self
                .token_owner
                .get(&token_id)
                .ok_or(Error::TokenNotFound)?;

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
                .get(&token_id)
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
            let expires_at =
                timeout_blocks.map(|blocks| u64::from(current_block) + u64::from(blocks));

            let property_info = self
                .token_properties
                .get(&token_id)
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
                expires_at: timeout_blocks
                    .map(|blocks| u64::from(current_block) + u64::from(blocks)),
                status: BridgeOperationStatus::Pending,
                metadata: property_info.metadata.clone(),
            };

            self.bridge_requests.insert(&request_id, &request);

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
                .get(&request_id)
                .ok_or(Error::InvalidRequest)?;

            // Check if request has expired
            if let Some(expires_at) = request.expires_at {
                if u64::from(self.env().block_number()) > expires_at {
                    request.status = BridgeOperationStatus::Expired;
                    self.bridge_requests.insert(&request_id, &request);
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
                    .get(&request.token_id)
                    .ok_or(Error::TokenNotFound)?;
                self.balances
                    .insert((&token_owner, &request.token_id), &0u128);
                self.token_owner
                    .insert(&request.token_id, &AccountId::from([0u8; 32])); // Lock to zero address
            }

            self.bridge_requests.insert(&request_id, &request);

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
                .get(&request_id)
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
            self.bridge_requests.insert(&request_id, &request);

            // Store transaction verification
            self.verified_bridge_hashes.insert(&transaction_hash, &true);

            // Add to bridge history
            let mut history = self
                .bridge_transactions
                .get(&request.sender)
                .unwrap_or(Vec::new());
            history.push(transaction.clone());
            self.bridge_transactions.insert(&request.sender, &history);

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
                .get(&transaction_hash)
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

            self.token_properties.insert(&new_token_id, &property_info);
            self.token_owner.insert(&new_token_id, &recipient);
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

            self.ownership_history
                .insert(&new_token_id, &vec![initial_transfer]);

            // Initialize compliance as verified for bridged tokens
            let compliance_info = ComplianceInfo {
                verified: true,
                verification_date: self.env().block_timestamp(),
                verifier: caller,
                compliance_type: String::from("Bridge"),
            };
            self.compliance_flags
                .insert(&new_token_id, &compliance_info);

            // Initialize legal documents vector
            self.legal_documents
                .insert(&new_token_id, &Vec::<DocumentInfo>::new());

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
            recipient: AccountId,
        ) -> Result<(), Error> {
            let caller = self.env().caller();
            let token_owner = self
                .token_owner
                .get(&token_id)
                .ok_or(Error::TokenNotFound)?;

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
            self.token_owner.remove(&token_id);
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
                .get(&request_id)
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
                    if let Some(token_owner) = self.token_owner.get(&request.token_id) {
                        if token_owner == AccountId::from([0u8; 32]) {
                            // Token is locked, restore ownership to original sender
                            self.token_owner.insert(&request.token_id, &request.sender);
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
                    if let Some(token_owner) = self.token_owner.get(&request.token_id) {
                        if token_owner == AccountId::from([0u8; 32]) {
                            self.token_owner.insert(&request.token_id, &request.sender);
                            self.balances
                                .insert((&request.sender, &request.token_id), &1u128);
                            self.add_token_to_owner(request.sender, request.token_id)?;
                        }
                    }
                }
            }

            self.bridge_requests.insert(&request_id, &request);

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
                .get(&token_id)
                .ok_or(Error::TokenNotFound)?;
            let metadata_gas = property_info.metadata.legal_description.len() as u64 * 100;

            Ok(base_gas + metadata_gas)
        }

        /// Monitors bridge status
        #[ink(message)]
        pub fn monitor_bridge_status(&self, request_id: u64) -> Option<BridgeMonitoringInfo> {
            let request = self.bridge_requests.get(&request_id)?;

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
            self.bridge_transactions.get(&account).unwrap_or(Vec::new())
        }

        /// Verifies bridge transaction hash
        #[ink(message)]
        pub fn verify_bridge_transaction(
            &self,
            token_id: TokenId,
            transaction_hash: Hash,
            source_chain: ChainId,
        ) -> bool {
            self.verified_bridge_hashes
                .get(&transaction_hash)
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
        fn add_token_to_owner(&mut self, to: AccountId, token_id: TokenId) -> Result<(), Error> {
            let count = self.owner_token_count.get(&to).unwrap_or(0);
            self.owner_token_count.insert(&to, &(count + 1));
            Ok(())
        }

        /// Internal helper to remove a token from an owner
        fn remove_token_from_owner(
            &mut self,
            from: AccountId,
            token_id: TokenId,
        ) -> Result<(), Error> {
            let count = self.owner_token_count.get(&from).unwrap_or(0);
            if count == 0 {
                return Err(Error::TokenNotFound);
            }
            self.owner_token_count.insert(&from, &(count - 1));
            Ok(())
        }

        /// Internal helper to update ownership history
        fn update_ownership_history(
            &mut self,
            token_id: TokenId,
            from: AccountId,
            to: AccountId,
        ) -> Result<(), Error> {
            let mut history = self.ownership_history.get(&token_id).unwrap_or(Vec::new());

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

            history.push(transfer_record);

            self.ownership_history.insert(&token_id, &history);

            Ok(())
        }

        /// Helper to check if token has pending bridge request
        fn has_pending_bridge_request(&self, token_id: TokenId) -> bool {
            // This is a simplified check - in a real implementation,
            // you might want to maintain a separate mapping for efficiency
            for i in 1..=self.bridge_request_counter {
                if let Some(request) = self.bridge_requests.get(&i) {
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

            let token_id = result.unwrap();
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

            let token_id = contract.register_property_with_token(metadata).unwrap();
            let caller = AccountId::from([1u8; 32]);

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

            let token_id = contract.register_property_with_token(metadata).unwrap();

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

            let token_id = contract.register_property_with_token(metadata).unwrap();

            let accounts = test::default_accounts::<DefaultEnvironment>();
            test::set_caller::<DefaultEnvironment>(contract.admin());

            let result = contract.verify_compliance(token_id, true);
            assert!(result.is_ok());

            let compliance_info = contract.compliance_flags.get(&token_id).unwrap();
            assert!(compliance_info.verified);
        }
    }
}
