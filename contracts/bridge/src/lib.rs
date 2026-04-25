#![cfg_attr(not(feature = "std"), no_std)]
#![allow(unexpected_cfgs)]

use ink::prelude::string::String;
use ink::storage::Mapping;
use propchain_traits::*;
#[cfg(not(feature = "std"))]
use scale_info::prelude::vec::Vec;

#[ink::contract]
mod bridge {
    use super::*;
    use propchain_contracts::{non_reentrant, ReentrancyError, ReentrancyGuard};

    include!("errors.rs");

    impl From<ReentrancyError> for Error {
        fn from(_: ReentrancyError) -> Self {
            Error::ReentrantCall
        }
    }

    /// Bridge contract for cross-chain property token transfers
    #[ink(storage)]
    pub struct PropertyBridge {
        /// Bridge configuration
        config: BridgeConfig,

        /// Multi-signature bridge requests
        bridge_requests: Mapping<u64, MultisigBridgeRequest>,

        /// Bridge transaction history
        bridge_history: Mapping<AccountId, Vec<BridgeTransaction>>,

        /// Chain-specific information
        chain_info: Mapping<ChainId, ChainBridgeInfo>,

        /// Transaction verification records
        verified_transactions: Mapping<Hash, bool>,

        /// Cross-chain DEX settlement intents tracked by the bridge
        cross_chain_trades: Mapping<u64, CrossChainTradeIntent>,

        /// Bridge operators
        bridge_operators: Vec<AccountId>,

        /// Request counter
        request_counter: u64,

        /// Transaction counter
        transaction_counter: u64,

        /// Cross-chain trade settlement counter
        cross_chain_trade_counter: u64,

        /// Admin account
        admin: AccountId,

        /// Registered ECDSA public keys for optional cryptographic signature verification
        operator_public_keys: Mapping<AccountId, [u8; 33]>,

        /// Pending admin key rotation request
        pending_admin_rotation: Option<propchain_traits::KeyRotationRequest>,

        /// Account daily bridge request count for rate limiting
        account_daily_requests: Mapping<AccountId, u64>,

        /// Account last reset day for rate limiting
        account_last_reset_day: Mapping<AccountId, u64>,

        /// Chain daily volume for rate limiting
        chain_daily_volume: Mapping<ChainId, u128>,

        /// Chain last reset day for rate limiting
        chain_last_reset_day: Mapping<ChainId, u64>,

        /// Reentrancy protection
        reentrancy_guard: ReentrancyGuard,
    }

    /// Events for bridge operations
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

    impl PropertyBridge {
        /// Creates a new PropertyBridge contract
        #[ink(constructor)]
        pub fn new(
            supported_chains: Vec<ChainId>,
            min_signatures: u8,
            max_signatures: u8,
            default_timeout: u64,
            gas_limit: u64,
        ) -> Self {
            let caller = Self::env().caller();
            let config = BridgeConfig {
                supported_chains: supported_chains.clone(),
                min_signatures_required: min_signatures,
                max_signatures_required: max_signatures,
                default_timeout_blocks: default_timeout,
                gas_limit_per_bridge: gas_limit,
                emergency_pause: false,
                metadata_preservation: true,
                rate_limit_enabled: true,
                max_requests_per_day: 10,
                max_value_per_day: 1_000_000_000_000_000_000,
            };

            // Initialize chain info for supported chains
            let mut bridge = Self {
                config,
                bridge_requests: Mapping::default(),
                bridge_history: Mapping::default(),
                chain_info: Mapping::default(),
                verified_transactions: Mapping::default(),
                cross_chain_trades: Mapping::default(),
                bridge_operators: vec![caller],
                request_counter: 0,
                transaction_counter: 0,
                cross_chain_trade_counter: 0,
                admin: caller,
                operator_public_keys: Mapping::default(),
                pending_admin_rotation: None,
                account_daily_requests: Mapping::default(),
                account_last_reset_day: Mapping::default(),
                chain_daily_volume: Mapping::default(),
                chain_last_reset_day: Mapping::default(),
                reentrancy_guard: ReentrancyGuard::new(),
            };

            // Set up default chain information
            for chain_id in supported_chains {
                let chain_info = ChainBridgeInfo {
                    chain_id,
                    chain_name: format!("Chain-{}", chain_id),
                    bridge_contract_address: None,
                    is_active: true,
                    gas_multiplier: propchain_traits::constants::DEFAULT_GAS_MULTIPLIER,
                    confirmation_blocks: propchain_traits::constants::DEFAULT_CONFIRMATION_BLOCKS,
                    supported_tokens: Vec::new(),
                    chain_daily_limit: 10_000_000_000_000_000_000, // Example large default
                };
                bridge.chain_info.insert(chain_id, &chain_info);
            }

            bridge
        }

        /// Initiates a bridge request with multi-signature requirement
        #[ink(message)]
        pub fn initiate_bridge_multisig(
            &mut self,
            token_id: TokenId,
            destination_chain: ChainId,
            recipient: AccountId,
            required_signatures: u8,
            timeout_blocks: Option<u64>,
            metadata: PropertyMetadata,
        ) -> Result<u64, Error> {
            let caller = self.env().caller();

            // Check if bridge is paused
            if self.config.emergency_pause {
                return Err(Error::BridgePaused);
            }

            // Validate destination chain
            if !self.config.supported_chains.contains(&destination_chain) {
                return Err(Error::InvalidChain);
            }

            // Validate signature requirements
            if required_signatures < self.config.min_signatures_required
                || required_signatures > self.config.max_signatures_required
            {
                return Err(Error::InsufficientSignatures);
            }

            // Check if caller is authorized (token owner or approved operator)
            if !self.is_authorized_for_token(caller, token_id) {
                return Err(Error::Unauthorized);
            }

            // Enforce rate limiting
            // For NFT bridge, we count requests but value is 0 here since NFT value isn't strictly defined by amount.
            self.check_and_update_rate_limits(caller, destination_chain, 0, true)?;

            // Create bridge request
            self.request_counter += 1;
            let request_id = self.request_counter;
            let current_block = u64::from(self.env().block_number());
            let expires_at = timeout_blocks.map(|blocks| current_block + blocks);

            let request = MultisigBridgeRequest {
                request_id,
                token_id,
                source_chain: self.get_current_chain_id(),
                destination_chain,
                sender: caller,
                recipient,
                required_signatures,
                signatures: Vec::new(),
                created_at: current_block,
                expires_at,
                status: BridgeOperationStatus::Pending,
                metadata,
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

        /// Signs a bridge request
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
            } else if request.signatures.len() >= request.required_signatures as usize {
                request.status = BridgeOperationStatus::Locked;
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

        /// Register an ECDSA public key for cryptographic signature verification.
        #[ink(message)]
        pub fn register_operator_public_key(&mut self, public_key: [u8; 33]) -> Result<(), Error> {
            let caller = self.env().caller();
            if !self.bridge_operators.contains(&caller) {
                return Err(Error::Unauthorized);
            }
            self.operator_public_keys.insert(caller, &public_key);
            Ok(())
        }

        /// Sign a bridge request with optional ECDSA cryptographic signature verification.
        #[ink(message)]
        pub fn sign_bridge_request_with_signature(
            &mut self,
            request_id: u64,
            approve: bool,
            signed_approval: Option<propchain_traits::SignedApproval>,
        ) -> Result<(), Error> {
            let caller = self.env().caller();

            if let Some(ref approval) = signed_approval {
                let expected_key = self
                    .operator_public_keys
                    .get(caller)
                    .ok_or(Error::Unauthorized)?;
                propchain_traits::crypto::verify_signed_approval(approval, &expected_key)
                    .map_err(|_| Error::Unauthorized)?;

                let expected_hash = propchain_traits::crypto::hash_encoded(&(
                    request_id,
                    approve,
                    caller,
                    self.env().block_number(),
                ));
                if approval.message_hash != <[u8; 32]>::from(expected_hash) {
                    return Err(Error::Unauthorized);
                }
            }

            self.sign_bridge_request(request_id, approve)
        }

        /// Executes a bridge request after collecting required signatures
        #[ink(message)]
        pub fn execute_bridge(&mut self, request_id: u64) -> Result<(), Error> {
            non_reentrant!(self, {
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
                let transaction_hash = self.generate_transaction_hash(&request);

                // Create bridge transaction record
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
                    gas_used: self.estimate_gas_usage(&request),
                    status: BridgeOperationStatus::InTransit,
                    metadata: request.metadata.clone(),
                };

                // Update request status
                request.status = BridgeOperationStatus::Completed;
                self.bridge_requests.insert(request_id, &request);

                // Store transaction verification
                self.verified_transactions.insert(transaction_hash, &true);

                // Add to bridge history
                let mut history = self.bridge_history.get(request.sender).unwrap_or_default();
                history.push(transaction.clone());
                self.bridge_history.insert(request.sender, &history);

                self.env().emit_event(BridgeExecuted {
                    request_id,
                    token_id: request.token_id,
                    transaction_hash,
                });

                Ok(())
            })
        }

        /// Recovers from a failed bridge operation
        #[ink(message)]
        pub fn recover_failed_bridge(
            &mut self,
            request_id: u64,
            recovery_action: RecoveryAction,
        ) -> Result<(), Error> {
            non_reentrant!(self, {
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
                        // Logic to unlock the token would be implemented here
                        // This would typically call back to the property token contract
                    }
                    RecoveryAction::RefundGas => {
                        // Logic to refund gas costs would be implemented here
                    }
                    RecoveryAction::RetryBridge => {
                        // Reset request to pending for retry
                        request.status = BridgeOperationStatus::Pending;
                        request.signatures.clear();
                    }
                    RecoveryAction::CancelBridge => {
                        // Mark as cancelled
                        request.status = BridgeOperationStatus::Failed;
                    }
                }

                self.bridge_requests.insert(request_id, &request);

                self.env().emit_event(BridgeRecovered {
                    request_id,
                    recovery_action,
                });

                Ok(())
            })
        }

        /// Gets gas estimation for a bridge operation
        #[ink(message)]
        pub fn estimate_bridge_gas(
            &self,
            _token_id: TokenId,
            destination_chain: ChainId,
        ) -> Result<u64, Error> {
            let chain_info = self
                .chain_info
                .get(destination_chain)
                .ok_or(Error::InvalidChain)?;

            let base_gas = self.config.gas_limit_per_bridge;
            let multiplier = chain_info.gas_multiplier;

            Ok(base_gas * multiplier as u64 / 100)
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

        /// Verifies a bridge transaction
        #[ink(message)]
        pub fn verify_bridge_transaction(
            &self,
            transaction_hash: Hash,
            _source_chain: ChainId,
        ) -> bool {
            self.verified_transactions
                .get(transaction_hash)
                .unwrap_or(false)
        }

        /// Gets bridge history for an account
        #[ink(message)]
        pub fn get_bridge_history(&self, account: AccountId) -> Vec<BridgeTransaction> {
            self.bridge_history.get(account).unwrap_or_default()
        }

        /// Quotes bridge fees for a DEX settlement.
        #[ink(message)]
        pub fn quote_cross_chain_trade(
            &self,
            destination_chain: ChainId,
            amount_in: u128,
        ) -> Result<BridgeFeeQuote, Error> {
            let gas_estimate = self.estimate_bridge_gas(0, destination_chain)?;
            let protocol_fee = amount_in / 200;
            Ok(BridgeFeeQuote {
                destination_chain,
                gas_estimate,
                protocol_fee,
                total_fee: protocol_fee.saturating_add(gas_estimate as u128),
            })
        }

        /// Registers a cross-chain DEX trade intent on the bridge.
        #[ink(message)]
        pub fn register_cross_chain_trade(
            &mut self,
            pair_id: u64,
            order_id: Option<u64>,
            destination_chain: ChainId,
            recipient: AccountId,
            amount_in: u128,
            min_amount_out: u128,
        ) -> Result<u64, Error> {
            if self.config.emergency_pause {
                return Err(Error::BridgePaused);
            }
            if !self.config.supported_chains.contains(&destination_chain) {
                return Err(Error::InvalidChain);
            }

            // Enforce rate limiting
            // For cross-chain trades, we track the volume (amount_in) but don't count it as an NFT request.
            self.check_and_update_rate_limits(
                self.env().caller(),
                destination_chain,
                amount_in,
                false,
            )?;

            self.cross_chain_trade_counter += 1;
            let trade_id = self.cross_chain_trade_counter;
            let quote = self.quote_cross_chain_trade(destination_chain, amount_in)?;
            let intent = CrossChainTradeIntent {
                trade_id,
                pair_id,
                order_id,
                source_chain: self.get_current_chain_id(),
                destination_chain,
                trader: self.env().caller(),
                recipient,
                amount_in,
                min_amount_out,
                bridge_request_id: None,
                bridge_fee_quote: quote,
                status: CrossChainTradeStatus::Pending,
                created_at: self.env().block_timestamp(),
            };
            self.cross_chain_trades.insert(trade_id, &intent);
            Ok(trade_id)
        }

        /// Attaches a bridge request to a pending cross-chain trade.
        #[ink(message)]
        pub fn attach_bridge_request_to_trade(
            &mut self,
            trade_id: u64,
            bridge_request_id: u64,
        ) -> Result<(), Error> {
            let caller = self.env().caller();
            let mut trade = self
                .cross_chain_trades
                .get(trade_id)
                .ok_or(Error::InvalidRequest)?;
            if caller != trade.trader && caller != self.admin {
                return Err(Error::Unauthorized);
            }
            trade.bridge_request_id = Some(bridge_request_id);
            trade.status = CrossChainTradeStatus::BridgeRequested;
            self.cross_chain_trades.insert(trade_id, &trade);
            Ok(())
        }

        /// Marks a cross-chain trade settlement as complete.
        #[ink(message)]
        pub fn settle_cross_chain_trade(&mut self, trade_id: u64) -> Result<(), Error> {
            let caller = self.env().caller();
            if caller != self.admin && !self.bridge_operators.contains(&caller) {
                return Err(Error::Unauthorized);
            }
            let mut trade = self
                .cross_chain_trades
                .get(trade_id)
                .ok_or(Error::InvalidRequest)?;
            trade.status = CrossChainTradeStatus::Settled;
            self.cross_chain_trades.insert(trade_id, &trade);
            Ok(())
        }

        /// Gets a cross-chain trade settlement intent.
        #[ink(message)]
        pub fn get_cross_chain_trade(&self, trade_id: u64) -> Option<CrossChainTradeIntent> {
            self.cross_chain_trades.get(trade_id)
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
        pub fn update_config(&mut self, config: BridgeConfig) -> Result<(), Error> {
            let caller = self.env().caller();
            if caller != self.admin {
                return Err(Error::Unauthorized);
            }

            self.config = config;
            Ok(())
        }

        /// Gets current bridge configuration
        #[ink(message)]
        pub fn get_config(&self) -> BridgeConfig {
            self.config.clone()
        }

        /// Pauses or unpauses the bridge (admin only)
        #[ink(message)]
        pub fn set_emergency_pause(&mut self, paused: bool) -> Result<(), Error> {
            let caller = self.env().caller();
            if caller != self.admin {
                return Err(Error::Unauthorized);
            }

            self.config.emergency_pause = paused;
            Ok(())
        }

        /// Gets chain information
        #[ink(message)]
        pub fn get_chain_info(&self, chain_id: ChainId) -> Option<ChainBridgeInfo> {
            self.chain_info.get(chain_id)
        }

        /// Updates chain information (admin only)
        #[ink(message)]
        pub fn update_chain_info(
            &mut self,
            chain_id: ChainId,
            info: ChainBridgeInfo,
        ) -> Result<(), Error> {
            let caller = self.env().caller();
            if caller != self.admin {
                return Err(Error::Unauthorized);
            }

            self.chain_info.insert(chain_id, &info);
            Ok(())
        }

        /// Request a two-step admin rotation with cooldown.
        #[ink(message)]
        pub fn request_admin_rotation(&mut self, new_admin: AccountId) -> Result<(), Error> {
            let caller = self.env().caller();
            if caller != self.admin {
                return Err(Error::Unauthorized);
            }

            let block = self.env().block_number();
            let effective_at =
                block.saturating_add(propchain_traits::constants::KEY_ROTATION_COOLDOWN_BLOCKS);

            self.pending_admin_rotation = Some(propchain_traits::KeyRotationRequest {
                old_account: caller,
                new_account: new_admin,
                requested_at: block,
                effective_at,
                confirmed: false,
            });

            Ok(())
        }

        /// Confirm a pending admin rotation after cooldown.
        #[ink(message)]
        pub fn confirm_admin_rotation(&mut self) -> Result<(), Error> {
            let caller = self.env().caller();
            let block = self.env().block_number();

            let request = self
                .pending_admin_rotation
                .as_ref()
                .ok_or(Error::InvalidRequest)?;

            if request.new_account != caller {
                return Err(Error::Unauthorized);
            }
            if block < request.effective_at {
                return Err(Error::InvalidRequest);
            }
            let expiry = request
                .effective_at
                .saturating_add(propchain_traits::constants::KEY_ROTATION_EXPIRY_BLOCKS);
            if block > expiry {
                self.pending_admin_rotation = None;
                return Err(Error::RequestExpired);
            }

            self.admin = caller;
            self.pending_admin_rotation = None;
            Ok(())
        }

        /// Cancel a pending admin rotation.
        #[ink(message)]
        pub fn cancel_admin_rotation(&mut self) -> Result<(), Error> {
            let caller = self.env().caller();
            let request = self
                .pending_admin_rotation
                .as_ref()
                .ok_or(Error::InvalidRequest)?;

            if caller != request.old_account && caller != request.new_account {
                return Err(Error::Unauthorized);
            }

            self.pending_admin_rotation = None;
            Ok(())
        }

        // Helper functions

        fn is_authorized_for_token(&self, _account: AccountId, _token_id: TokenId) -> bool {
            // This would typically check with the property token contract
            // For now, we'll assume any account can initiate a bridge
            true
        }

        fn get_current_chain_id(&self) -> ChainId {
            // This should return the current chain ID
            // For now, we'll use a default value
            1
        }

        fn generate_transaction_hash(&self, request: &MultisigBridgeRequest) -> Hash {
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

        fn estimate_gas_usage(&self, request: &MultisigBridgeRequest) -> u64 {
            // Estimate gas usage based on request complexity
            let base_gas = 100000; // Base gas for bridge operation
            let metadata_gas = request.metadata.legal_description.len() as u64 * 100; // Gas for metadata
            base_gas + metadata_gas
        }

        fn check_and_update_rate_limits(
            &mut self,
            account: AccountId,
            destination_chain: ChainId,
            amount: u128,
            is_nft: bool,
        ) -> Result<(), Error> {
            if !self.config.rate_limit_enabled {
                return Ok(());
            }

            let current_day = self.env().block_timestamp() / 86_400_000;

            if is_nft {
                let last_reset = self.account_last_reset_day.get(account).unwrap_or(0);
                let mut daily_requests = self.account_daily_requests.get(account).unwrap_or(0);

                if last_reset < current_day {
                    daily_requests = 0;
                    self.account_last_reset_day.insert(account, &current_day);
                }

                if daily_requests >= self.config.max_requests_per_day {
                    return Err(Error::RateLimitExceeded);
                }

                self.account_daily_requests
                    .insert(account, &(daily_requests + 1));
            }

            if amount > 0 {
                let chain_info = self
                    .chain_info
                    .get(destination_chain)
                    .ok_or(Error::InvalidChain)?;
                let last_chain_reset = self
                    .chain_last_reset_day
                    .get(destination_chain)
                    .unwrap_or(0);
                let mut chain_volume = self.chain_daily_volume.get(destination_chain).unwrap_or(0);

                if last_chain_reset < current_day {
                    chain_volume = 0;
                    self.chain_last_reset_day
                        .insert(destination_chain, &current_day);
                }

                if chain_volume.saturating_add(amount) > chain_info.chain_daily_limit {
                    return Err(Error::RateLimitExceeded);
                }

                self.chain_daily_volume
                    .insert(destination_chain, &(chain_volume + amount));
            }

            Ok(())
        }
    }
}
