#![cfg_attr(not(feature = "std"), no_std)]
#![allow(clippy::needless_borrows_for_generic_args)]
#![allow(clippy::too_many_arguments)]

use ink::storage::Mapping;
use propchain_traits::*;
#[cfg(not(feature = "std"))]
use scale_info::prelude::{string::String, vec::Vec};

pub mod tests;

#[ink::contract]
mod propchain_escrow {
    use super::*;
    use propchain_traits::{non_reentrant, ReentrancyError, ReentrancyGuard};

    include!("errors.rs");
    include!("types.rs");

    impl From<ReentrancyError> for Error {
        fn from(_: ReentrancyError) -> Self {
            Error::ReentrantCall
        }
    }

    /// Main contract storage
    #[ink(storage)]
    pub struct AdvancedEscrow {
        /// Escrow data mapping
        escrows: Mapping<u64, EscrowData>,
        /// Escrow counter
        escrow_count: u64,
        /// Multi-signature configurations
        multi_sig_configs: Mapping<u64, MultiSigConfig>,
        /// Signature tracking: (escrow_id, approval_type, signer) -> bool
        signatures: Mapping<SignatureKey, bool>,
        /// Signature counts: (escrow_id, approval_type) -> count
        signature_counts: Mapping<(u64, ApprovalType), u8>,
        /// Documents per escrow
        documents: Mapping<u64, Vec<DocumentHash>>,
        /// Conditions per escrow
        conditions: Mapping<u64, Vec<Condition>>,
        /// Condition counter per escrow
        condition_counters: Mapping<u64, u64>,
        /// Disputes
        disputes: Mapping<u64, DisputeInfo>,
        /// Audit logs
        audit_logs: Mapping<u64, Vec<AuditEntry>>,
        /// Admin account
        admin: AccountId,
        /// High-value threshold for mandatory multi-sig
        min_high_value_threshold: u128,
        /// Registered ECDSA public keys for optional cryptographic signature verification
        signer_public_keys: Mapping<AccountId, [u8; 33]>,
        /// Pending admin key rotation request
        pending_admin_rotation: Option<propchain_traits::KeyRotationRequest>,
        /// Reentrancy protection guard
        reentrancy_guard: ReentrancyGuard,
        /// Pending large-transfer approval requests: request_id -> LargeTransferRequest
        large_transfer_requests: Mapping<u64, LargeTransferRequest>,
        /// Counter for large-transfer request IDs
        large_transfer_request_count: u64,
        /// Index: escrow_id -> active large-transfer request_id (0 = none)
        escrow_active_large_transfer: Mapping<u64, u64>,
        /// Large-transfer threshold override (0 = use global constant)
        large_transfer_threshold: u128,
        /// Very-large-transfer threshold override (0 = use global constant)
        very_large_transfer_threshold: u128,
    }

    // Events
    #[ink(event)]
    pub struct EscrowCreated {
        #[ink(topic)]
        escrow_id: u64,
        property_id: u64,
        buyer: AccountId,
        seller: AccountId,
        amount: u128,
    }

    #[ink(event)]
    pub struct FundsDeposited {
        #[ink(topic)]
        escrow_id: u64,
        amount: u128,
        depositor: AccountId,
    }

    #[ink(event)]
    pub struct FundsReleased {
        #[ink(topic)]
        escrow_id: u64,
        amount: u128,
        recipient: AccountId,
    }

    #[ink(event)]
    pub struct FundsRefunded {
        #[ink(topic)]
        escrow_id: u64,
        amount: u128,
        recipient: AccountId,
    }

    #[ink(event)]
    pub struct DocumentUploaded {
        #[ink(topic)]
        escrow_id: u64,
        document_hash: Hash,
        document_type: String,
        uploader: AccountId,
    }

    #[ink(event)]
    pub struct DocumentVerified {
        #[ink(topic)]
        escrow_id: u64,
        document_hash: Hash,
        verifier: AccountId,
    }

    #[ink(event)]
    pub struct ConditionAdded {
        #[ink(topic)]
        escrow_id: u64,
        condition_id: u64,
        description: String,
    }

    #[ink(event)]
    pub struct ConditionMet {
        #[ink(topic)]
        escrow_id: u64,
        condition_id: u64,
        verified_by: AccountId,
    }

    #[ink(event)]
    pub struct SignatureAdded {
        #[ink(topic)]
        escrow_id: u64,
        approval_type: ApprovalType,
        signer: AccountId,
    }

    #[ink(event)]
    pub struct DisputeRaised {
        #[ink(topic)]
        escrow_id: u64,
        raised_by: AccountId,
        reason: String,
    }

    #[ink(event)]
    pub struct DisputeResolved {
        #[ink(topic)]
        escrow_id: u64,
        resolution: String,
    }

    #[ink(event)]
    pub struct EmergencyOverride {
        #[ink(topic)]
        escrow_id: u64,
        admin: AccountId,
    }

    // ── Large-Transfer Multi-Step Approval Events ────────────────────────────

    /// Emitted when a large-transfer approval request is created.
    #[ink(event)]
    pub struct LargeTransferRequested {
        #[ink(topic)]
        pub request_id: u64,
        #[ink(topic)]
        pub escrow_id: u64,
        pub approval_type: ApprovalType,
        pub amount: u128,
        pub recipient: AccountId,
        pub required_approvals: u8,
        pub expires_at_block: u64,
    }

    /// Emitted when an approver signs a large-transfer request.
    #[ink(event)]
    pub struct LargeTransferApproved {
        #[ink(topic)]
        pub request_id: u64,
        #[ink(topic)]
        pub approver: AccountId,
        pub approvals_collected: u8,
        pub approvals_required: u8,
    }

    /// Emitted when a large-transfer is executed after all approvals are collected.
    #[ink(event)]
    pub struct LargeTransferExecuted {
        #[ink(topic)]
        pub request_id: u64,
        #[ink(topic)]
        pub escrow_id: u64,
        pub amount: u128,
        pub recipient: AccountId,
        pub executed_by: AccountId,
    }

    /// Emitted when a large-transfer approval request is cancelled.
    #[ink(event)]
    pub struct LargeTransferCancelled {
        #[ink(topic)]
        pub request_id: u64,
        #[ink(topic)]
        pub escrow_id: u64,
        pub cancelled_by: AccountId,
    }

    impl AdvancedEscrow {
        /// Constructor
        #[ink(constructor)]
        pub fn new(min_high_value_threshold: u128) -> Self {
            Self {
                escrows: Mapping::default(),
                escrow_count: 0,
                multi_sig_configs: Mapping::default(),
                signatures: Mapping::default(),
                signature_counts: Mapping::default(),
                documents: Mapping::default(),
                conditions: Mapping::default(),
                condition_counters: Mapping::default(),
                disputes: Mapping::default(),
                audit_logs: Mapping::default(),
                admin: Self::env().caller(),
                min_high_value_threshold,
                signer_public_keys: Mapping::default(),
                pending_admin_rotation: None,
                reentrancy_guard: ReentrancyGuard::new(),
                large_transfer_requests: Mapping::default(),
                large_transfer_request_count: 0,
                escrow_active_large_transfer: Mapping::default(),
                // 0 means "use global constant from propchain_traits::constants"
                large_transfer_threshold: 0,
                very_large_transfer_threshold: 0,
            }
        }

        /// Create a new escrow with advanced features
        #[ink(message)]
        pub fn create_escrow_advanced(
            &mut self,
            property_id: u64,
            amount: u128,
            buyer: AccountId,
            seller: AccountId,
            participants: Vec<AccountId>,
            required_signatures: u8,
            release_time_lock: Option<u64>,
        ) -> Result<u64, Error> {
            let caller = self.env().caller();

            // Validate configuration
            if required_signatures == 0 || participants.is_empty() {
                return Err(Error::InvalidConfiguration);
            }

            if required_signatures as usize > participants.len() {
                return Err(Error::InvalidConfiguration);
            }

            self.escrow_count += 1;
            let escrow_id = self.escrow_count;

            // Create escrow data
            let escrow_data = EscrowData {
                id: escrow_id,
                property_id,
                buyer,
                seller,
                amount,
                deposited_amount: 0,
                status: EscrowStatus::Created,
                created_at: self.env().block_timestamp(),
                release_time_lock,
                participants: participants.clone(),
            };

            self.escrows.insert(&escrow_id, &escrow_data);

            // Set up multi-sig configuration
            let multi_sig_config = MultiSigConfig {
                required_signatures,
                signers: participants.clone(),
            };
            self.multi_sig_configs.insert(&escrow_id, &multi_sig_config);

            // Initialize empty collections
            self.documents
                .insert(&escrow_id, &Vec::<DocumentHash>::new());
            self.conditions.insert(&escrow_id, &Vec::<Condition>::new());
            self.condition_counters.insert(&escrow_id, &0);
            self.audit_logs
                .insert(&escrow_id, &Vec::<AuditEntry>::new());

            // Add audit entry
            self.add_audit_entry(
                escrow_id,
                caller,
                "EscrowCreated".to_string(),
                format!("Property: {}, Amount: {}", property_id, amount),
            );

            self.env().emit_event(EscrowCreated {
                escrow_id,
                property_id,
                buyer,
                seller,
                amount,
            });

            Ok(escrow_id)
        }

        /// Deposit funds to escrow
        #[ink(message, payable)]
        pub fn deposit_funds(&mut self, escrow_id: u64) -> Result<(), Error> {
            let caller = self.env().caller();
            let transferred = self.env().transferred_value();

            let mut escrow = self.escrows.get(&escrow_id).ok_or(Error::EscrowNotFound)?;

            // Verify escrow is in correct state
            if escrow.status != EscrowStatus::Created && escrow.status != EscrowStatus::Funded {
                return Err(Error::InvalidStatus);
            }

            // Update deposited amount
            escrow.deposited_amount += transferred;

            // Check if fully funded
            if escrow.deposited_amount >= escrow.amount {
                escrow.status = EscrowStatus::Active;
            } else {
                escrow.status = EscrowStatus::Funded;
            }

            self.escrows.insert(&escrow_id, &escrow);

            // Add audit entry
            self.add_audit_entry(
                escrow_id,
                caller,
                "FundsDeposited".to_string(),
                format!("Amount: {}", transferred),
            );

            self.env().emit_event(FundsDeposited {
                escrow_id,
                amount: transferred,
                depositor: caller,
            });

            Ok(())
        }

        /// Release funds with multi-signature approval.
        ///
        /// If the escrow's deposited amount exceeds the large-transfer threshold,
        /// this call creates a `LargeTransferRequest` and returns
        /// `Err(Error::LargeTransferApprovalRequired)`.  Authorised signers must
        /// then call `approve_large_transfer`, and once the required number of
        /// approvals is collected, anyone may call `execute_large_transfer` to
        /// finalise the transfer.
        #[ink(message)]
        pub fn release_funds(&mut self, escrow_id: u64) -> Result<(), Error> {
            non_reentrant!(self, {
                let caller = self.env().caller();
                let escrow = self.escrows.get(&escrow_id).ok_or(Error::EscrowNotFound)?;

                // Check status
                if escrow.status != EscrowStatus::Active {
                    return Err(Error::InvalidStatus);
                }

                // Check for active dispute
                if let Some(dispute) = self.disputes.get(&escrow_id) {
                    if !dispute.resolved {
                        return Err(Error::DisputeActive);
                    }
                }

                // Check time lock
                if let Some(time_lock) = escrow.release_time_lock {
                    if self.env().block_timestamp() < time_lock {
                        return Err(Error::TimeLockActive);
                    }
                }

                // Check all conditions are met
                if !self.check_all_conditions_met(escrow_id)? {
                    return Err(Error::ConditionsNotMet);
                }

                // Check multi-sig threshold
                if !self.check_signature_threshold(escrow_id, ApprovalType::Release)? {
                    return Err(Error::SignatureThresholdNotMet);
                }

                // ── Large-transfer gate ──────────────────────────────────────
                // If the amount exceeds the threshold, create a pending approval
                // request instead of transferring immediately.
                let tier = self.classify_transfer_tier(escrow.deposited_amount);
                if !matches!(tier, TransferApprovalTier::Standard) {
                    // Only create a new request if there isn't one already pending.
                    if self
                        .escrow_active_large_transfer
                        .get(&escrow_id)
                        .unwrap_or(0)
                        == 0
                    {
                        self.create_large_transfer_request(
                            escrow_id,
                            ApprovalType::Release,
                            escrow.deposited_amount,
                            escrow.seller,
                            tier,
                            caller,
                        )?;
                    }
                    return Err(Error::LargeTransferApprovalRequired);
                }
                // ── End large-transfer gate ──────────────────────────────────

                // Transfer funds to seller
                if self
                    .env()
                    .transfer(escrow.seller, escrow.deposited_amount)
                    .is_err()
                {
                    return Err(Error::InsufficientFunds);
                }

                // Update status AFTER transfer
                let mut updated_escrow = escrow.clone();
                updated_escrow.status = EscrowStatus::Released;
                self.escrows.insert(&escrow_id, &updated_escrow);

                // Add audit entry
                self.add_audit_entry(
                    escrow_id,
                    caller,
                    "FundsReleased".to_string(),
                    format!("Amount: {} to seller", escrow.deposited_amount),
                );

                self.env().emit_event(FundsReleased {
                    escrow_id,
                    amount: escrow.deposited_amount,
                    recipient: escrow.seller,
                });

                Ok(())
            })
        }

        /// Refund funds with multi-signature approval.
        ///
        /// Same large-transfer gate as `release_funds`: amounts above the
        /// threshold create a `LargeTransferRequest` and return
        /// `Err(Error::LargeTransferApprovalRequired)`.
        #[ink(message)]
        pub fn refund_funds(&mut self, escrow_id: u64) -> Result<(), Error> {
            non_reentrant!(self, {
                let caller = self.env().caller();
                let escrow = self.escrows.get(&escrow_id).ok_or(Error::EscrowNotFound)?;

                // Check status
                if escrow.status != EscrowStatus::Active && escrow.status != EscrowStatus::Funded {
                    return Err(Error::InvalidStatus);
                }

                // Check multi-sig threshold
                if !self.check_signature_threshold(escrow_id, ApprovalType::Refund)? {
                    return Err(Error::SignatureThresholdNotMet);
                }

                // ── Large-transfer gate ──────────────────────────────────────
                let tier = self.classify_transfer_tier(escrow.deposited_amount);
                if !matches!(tier, TransferApprovalTier::Standard) {
                    if self
                        .escrow_active_large_transfer
                        .get(&escrow_id)
                        .unwrap_or(0)
                        == 0
                    {
                        self.create_large_transfer_request(
                            escrow_id,
                            ApprovalType::Refund,
                            escrow.deposited_amount,
                            escrow.buyer,
                            tier,
                            caller,
                        )?;
                    }
                    return Err(Error::LargeTransferApprovalRequired);
                }
                // ── End large-transfer gate ──────────────────────────────────

                // Transfer funds back to buyer
                if self
                    .env()
                    .transfer(escrow.buyer, escrow.deposited_amount)
                    .is_err()
                {
                    return Err(Error::InsufficientFunds);
                }

                // Update status AFTER transfer
                let mut updated_escrow = escrow.clone();
                updated_escrow.status = EscrowStatus::Refunded;
                self.escrows.insert(&escrow_id, &updated_escrow);

                // Add audit entry
                self.add_audit_entry(
                    escrow_id,
                    caller,
                    "FundsRefunded".to_string(),
                    format!("Amount: {} to buyer", escrow.deposited_amount),
                );

                self.env().emit_event(FundsRefunded {
                    escrow_id,
                    amount: escrow.deposited_amount,
                    recipient: escrow.buyer,
                });

                Ok(())
            })
        }

        /// Upload document hash
        #[ink(message)]
        pub fn upload_document(
            &mut self,
            escrow_id: u64,
            document_hash: Hash,
            document_type: String,
        ) -> Result<(), Error> {
            let caller = self.env().caller();
            let escrow = self.escrows.get(&escrow_id).ok_or(Error::EscrowNotFound)?;

            // Check if caller is a participant
            if !escrow.participants.contains(&caller)
                && caller != escrow.buyer
                && caller != escrow.seller
            {
                return Err(Error::Unauthorized);
            }

            let document = DocumentHash {
                hash: document_hash,
                document_type: document_type.clone(),
                uploaded_by: caller,
                uploaded_at: self.env().block_timestamp(),
                verified: false,
            };

            let mut docs = self.documents.get(&escrow_id).unwrap_or_default();
            docs.push(document);
            self.documents.insert(&escrow_id, &docs);

            // Add audit entry
            self.add_audit_entry(
                escrow_id,
                caller,
                "DocumentUploaded".to_string(),
                format!("Type: {}", document_type),
            );

            self.env().emit_event(DocumentUploaded {
                escrow_id,
                document_hash,
                document_type,
                uploader: caller,
            });

            Ok(())
        }

        /// Verify document
        #[ink(message)]
        pub fn verify_document(
            &mut self,
            escrow_id: u64,
            document_hash: Hash,
        ) -> Result<(), Error> {
            let caller = self.env().caller();
            let escrow = self.escrows.get(&escrow_id).ok_or(Error::EscrowNotFound)?;

            // Check if caller is a participant
            if !escrow.participants.contains(&caller) {
                return Err(Error::Unauthorized);
            }

            let mut docs = self
                .documents
                .get(&escrow_id)
                .ok_or(Error::DocumentNotFound)?;
            let mut found = false;

            for doc in docs.iter_mut() {
                if doc.hash == document_hash {
                    doc.verified = true;
                    found = true;
                    break;
                }
            }

            if !found {
                return Err(Error::DocumentNotFound);
            }

            self.documents.insert(&escrow_id, &docs);

            // Add audit entry
            self.add_audit_entry(
                escrow_id,
                caller,
                "DocumentVerified".to_string(),
                "Document verified".to_string(),
            );

            self.env().emit_event(DocumentVerified {
                escrow_id,
                document_hash,
                verifier: caller,
            });

            Ok(())
        }

        /// Add condition to escrow
        #[ink(message)]
        pub fn add_condition(&mut self, escrow_id: u64, description: String) -> Result<u64, Error> {
            let caller = self.env().caller();
            let escrow = self.escrows.get(&escrow_id).ok_or(Error::EscrowNotFound)?;

            // Only buyer or seller can add conditions
            if caller != escrow.buyer && caller != escrow.seller {
                return Err(Error::Unauthorized);
            }

            let mut counter = self.condition_counters.get(&escrow_id).unwrap_or(0);
            counter += 1;

            let condition = Condition {
                id: counter,
                description: description.clone(),
                met: false,
                verified_by: None,
                verified_at: None,
            };

            let mut conditions = self.conditions.get(&escrow_id).unwrap_or_default();
            conditions.push(condition);
            self.conditions.insert(&escrow_id, &conditions);
            self.condition_counters.insert(&escrow_id, &counter);

            // Add audit entry
            self.add_audit_entry(
                escrow_id,
                caller,
                "ConditionAdded".to_string(),
                format!("Condition: {}", description),
            );

            self.env().emit_event(ConditionAdded {
                escrow_id,
                condition_id: counter,
                description,
            });

            Ok(counter)
        }

        /// Mark condition as met
        #[ink(message)]
        pub fn mark_condition_met(
            &mut self,
            escrow_id: u64,
            condition_id: u64,
        ) -> Result<(), Error> {
            let caller = self.env().caller();
            let escrow = self.escrows.get(&escrow_id).ok_or(Error::EscrowNotFound)?;

            // Check if caller is a participant
            if !escrow.participants.contains(&caller) {
                return Err(Error::Unauthorized);
            }

            let mut conditions = self.conditions.get(&escrow_id).unwrap_or_default();
            let mut found = false;

            for condition in conditions.iter_mut() {
                if condition.id == condition_id {
                    condition.met = true;
                    condition.verified_by = Some(caller);
                    condition.verified_at = Some(self.env().block_timestamp());
                    found = true;
                    break;
                }
            }

            if !found {
                return Err(Error::EscrowNotFound);
            }

            self.conditions.insert(&escrow_id, &conditions);

            // Add audit entry
            self.add_audit_entry(
                escrow_id,
                caller,
                "ConditionMet".to_string(),
                format!("Condition ID: {}", condition_id),
            );

            self.env().emit_event(ConditionMet {
                escrow_id,
                condition_id,
                verified_by: caller,
            });

            Ok(())
        }

        /// Sign approval for release or refund
        #[ink(message)]
        pub fn sign_approval(
            &mut self,
            escrow_id: u64,
            approval_type: ApprovalType,
        ) -> Result<(), Error> {
            let caller = self.env().caller();
            let _escrow = self.escrows.get(&escrow_id).ok_or(Error::EscrowNotFound)?;
            let config = self
                .multi_sig_configs
                .get(&escrow_id)
                .ok_or(Error::EscrowNotFound)?;

            // Check if caller is a valid signer
            if !config.signers.contains(&caller) {
                return Err(Error::Unauthorized);
            }

            // Check if already signed
            let sig_key = (escrow_id, approval_type.clone(), caller);
            if self.signatures.get(&sig_key).unwrap_or(false) {
                return Err(Error::AlreadySigned);
            }

            // Add signature
            self.signatures.insert(&sig_key, &true);

            // Update signature count
            let count_key = (escrow_id, approval_type.clone());
            let current_count = self.signature_counts.get(&count_key).unwrap_or(0);
            self.signature_counts
                .insert(&count_key, &(current_count + 1));

            // Add audit entry
            self.add_audit_entry(
                escrow_id,
                caller,
                "SignatureAdded".to_string(),
                format!("Approval type: {:?}", approval_type),
            );

            self.env().emit_event(SignatureAdded {
                escrow_id,
                approval_type,
                signer: caller,
            });

            Ok(())
        }

        /// Register an ECDSA public key for cryptographic signature verification.
        /// Once registered, the caller can use `sign_approval_with_signature` for
        /// defense-in-depth signature verification on top of Substrate's caller auth.
        #[ink(message)]
        pub fn register_public_key(&mut self, public_key: [u8; 33]) -> Result<(), Error> {
            let caller = self.env().caller();
            self.signer_public_keys.insert(caller, &public_key);
            Ok(())
        }

        /// Sign approval with optional ECDSA cryptographic signature verification.
        /// When `signed_approval` is `Some`, the contract verifies the ECDSA signature
        /// and checks the recovered key matches the caller's registered public key.
        /// When `None`, falls back to caller-identity-only (backward compatible).
        #[ink(message)]
        pub fn sign_approval_with_signature(
            &mut self,
            escrow_id: u64,
            approval_type: ApprovalType,
            signed_approval: Option<propchain_traits::SignedApproval>,
        ) -> Result<(), Error> {
            let caller = self.env().caller();

            // Verify cryptographic signature if provided
            if let Some(ref approval) = signed_approval {
                let expected_key = self
                    .signer_public_keys
                    .get(caller)
                    .ok_or(Error::Unauthorized)?;
                propchain_traits::crypto::verify_signed_approval(approval, &expected_key)
                    .map_err(|_| Error::Unauthorized)?;

                // Verify the message hash matches the expected payload
                let expected_hash = propchain_traits::crypto::hash_encoded(&(
                    escrow_id,
                    approval_type.clone(),
                    caller,
                    self.env().block_number(),
                ));
                if approval.message_hash != <[u8; 32]>::from(expected_hash) {
                    return Err(Error::Unauthorized);
                }
            }

            // Delegate to existing sign_approval logic
            self.sign_approval(escrow_id, approval_type)
        }

        /// Raise a dispute
        #[ink(message)]
        pub fn raise_dispute(&mut self, escrow_id: u64, reason: String) -> Result<(), Error> {
            let caller = self.env().caller();
            let escrow = self.escrows.get(&escrow_id).ok_or(Error::EscrowNotFound)?;

            // Only buyer or seller can raise dispute
            if caller != escrow.buyer && caller != escrow.seller {
                return Err(Error::Unauthorized);
            }

            // Check if dispute already exists
            if let Some(existing_dispute) = self.disputes.get(&escrow_id) {
                if !existing_dispute.resolved {
                    return Err(Error::DisputeActive);
                }
            }

            let dispute = DisputeInfo {
                escrow_id,
                raised_by: caller,
                reason: reason.clone(),
                raised_at: self.env().block_timestamp(),
                resolved: false,
                resolution: None,
            };

            self.disputes.insert(&escrow_id, &dispute);

            // Update escrow status
            let mut updated_escrow = escrow;
            updated_escrow.status = EscrowStatus::Disputed;
            self.escrows.insert(&escrow_id, &updated_escrow);

            // Add audit entry
            self.add_audit_entry(
                escrow_id,
                caller,
                "DisputeRaised".to_string(),
                format!("Reason: {}", reason),
            );

            self.env().emit_event(DisputeRaised {
                escrow_id,
                raised_by: caller,
                reason,
            });

            Ok(())
        }

        /// Resolve dispute (admin only)
        #[ink(message)]
        pub fn resolve_dispute(&mut self, escrow_id: u64, resolution: String) -> Result<(), Error> {
            let caller = self.env().caller();

            // Only admin can resolve disputes
            if caller != self.admin {
                return Err(Error::Unauthorized);
            }

            let mut dispute = self.disputes.get(&escrow_id).ok_or(Error::EscrowNotFound)?;
            dispute.resolved = true;
            dispute.resolution = Some(resolution.clone());
            self.disputes.insert(&escrow_id, &dispute);

            // Update escrow status back to Active
            let mut escrow = self.escrows.get(&escrow_id).ok_or(Error::EscrowNotFound)?;
            escrow.status = EscrowStatus::Active;
            self.escrows.insert(&escrow_id, &escrow);

            // Add audit entry
            self.add_audit_entry(
                escrow_id,
                caller,
                "DisputeResolved".to_string(),
                format!("Resolution: {}", resolution),
            );

            self.env().emit_event(DisputeResolved {
                escrow_id,
                resolution,
            });

            Ok(())
        }

        /// Emergency override (admin only)
        #[ink(message)]
        pub fn emergency_override(
            &mut self,
            escrow_id: u64,
            release_to_seller: bool,
        ) -> Result<(), Error> {
            non_reentrant!(self, {
                let caller = self.env().caller();

                // Only admin can perform emergency override
                if caller != self.admin {
                    return Err(Error::Unauthorized);
                }

                let escrow = self.escrows.get(&escrow_id).ok_or(Error::EscrowNotFound)?;

                let recipient = if release_to_seller {
                    escrow.seller
                } else {
                    escrow.buyer
                };

                // Transfer funds
                if self
                    .env()
                    .transfer(recipient, escrow.deposited_amount)
                    .is_err()
                {
                    return Err(Error::InsufficientFunds);
                }

                // Update status AFTER transfer
                let mut updated_escrow = escrow.clone();
                updated_escrow.status = if release_to_seller {
                    EscrowStatus::Released
                } else {
                    EscrowStatus::Refunded
                };
                self.escrows.insert(&escrow_id, &updated_escrow);

                // Add audit entry
                self.add_audit_entry(
                    escrow_id,
                    caller,
                    "EmergencyOverride".to_string(),
                    format!("Funds sent to: {:?}", recipient),
                );

                self.env().emit_event(EmergencyOverride {
                    escrow_id,
                    admin: caller,
                });

                Ok(())
            })
        }

        // ── Multi-Step Approval Public Messages ─────────────────────────────

        /// Approve a pending large-transfer request.
        ///
        /// Only authorised signers (participants listed in the escrow's
        /// `MultiSigConfig`) may call this.  Each signer may approve at most
        /// once.  Once the required number of approvals is reached the request
        /// status transitions to `Approved` and `execute_large_transfer` can
        /// be called.
        #[ink(message)]
        pub fn approve_large_transfer(&mut self, request_id: u64) -> Result<(), Error> {
            let caller = self.env().caller();

            let mut request = self
                .large_transfer_requests
                .get(&request_id)
                .ok_or(Error::ApprovalRequestNotFound)?;

            // Status checks
            if matches!(request.status, LargeTransferStatus::Executed) {
                return Err(Error::ApprovalRequestAlreadyExecuted);
            }
            if matches!(request.status, LargeTransferStatus::Cancelled) {
                return Err(Error::ApprovalRequestCancelled);
            }

            // Expiry check
            let current_block = u64::from(self.env().block_number());
            if current_block > request.expires_at_block {
                request.status = LargeTransferStatus::Expired;
                self.large_transfer_requests.insert(&request_id, &request);
                // Clear the active index so a new request can be created
                self.escrow_active_large_transfer.remove(&request.escrow_id);
                return Err(Error::ApprovalRequestExpired);
            }

            // Authorisation: caller must be a signer in the escrow's MultiSigConfig
            let config = self
                .multi_sig_configs
                .get(&request.escrow_id)
                .ok_or(Error::EscrowNotFound)?;
            if !config.signers.contains(&caller) {
                return Err(Error::Unauthorized);
            }

            // Duplicate approval check
            if request.approvals.contains(&caller) {
                return Err(Error::AlreadySigned);
            }

            // Record approval
            request.approvals.push(caller);
            let approvals_collected = request.approvals.len() as u8;

            // Transition to Approved when threshold is met
            if approvals_collected >= request.required_approvals {
                request.status = LargeTransferStatus::Approved;
            }

            self.large_transfer_requests.insert(&request_id, &request);

            self.add_audit_entry(
                request.escrow_id,
                caller,
                "LargeTransferApproved".to_string(),
                format!(
                    "Request {}: {}/{} approvals",
                    request_id, approvals_collected, request.required_approvals
                ),
            );

            self.env().emit_event(LargeTransferApproved {
                request_id,
                approver: caller,
                approvals_collected,
                approvals_required: request.required_approvals,
            });

            Ok(())
        }

        /// Execute a large-transfer request that has collected all required approvals.
        ///
        /// Can be called by any participant once the request status is `Approved`.
        /// Performs the actual on-chain transfer and updates the escrow status.
        #[ink(message)]
        pub fn execute_large_transfer(&mut self, request_id: u64) -> Result<(), Error> {
            non_reentrant!(self, {
                let caller = self.env().caller();

                let request = self
                    .large_transfer_requests
                    .get(&request_id)
                    .ok_or(Error::ApprovalRequestNotFound)?;

                // Must be in Approved state
                if !matches!(request.status, LargeTransferStatus::Approved) {
                    if matches!(request.status, LargeTransferStatus::Executed) {
                        return Err(Error::ApprovalRequestAlreadyExecuted);
                    }
                    if matches!(request.status, LargeTransferStatus::Cancelled) {
                        return Err(Error::ApprovalRequestCancelled);
                    }
                    // Pending or Expired
                    let current_block = u64::from(self.env().block_number());
                    if current_block > request.expires_at_block {
                        return Err(Error::ApprovalRequestExpired);
                    }
                    return Err(Error::SignatureThresholdNotMet);
                }

                // Expiry check (belt-and-suspenders)
                let current_block = u64::from(self.env().block_number());
                if current_block > request.expires_at_block {
                    let mut expired = request.clone();
                    expired.status = LargeTransferStatus::Expired;
                    self.large_transfer_requests.insert(&request_id, &expired);
                    self.escrow_active_large_transfer.remove(&request.escrow_id);
                    return Err(Error::ApprovalRequestExpired);
                }

                // Caller must be a participant or admin
                let escrow = self
                    .escrows
                    .get(&request.escrow_id)
                    .ok_or(Error::EscrowNotFound)?;
                if caller != self.admin
                    && !escrow.participants.contains(&caller)
                    && caller != escrow.buyer
                    && caller != escrow.seller
                {
                    return Err(Error::Unauthorized);
                }

                // Perform the transfer
                if self
                    .env()
                    .transfer(request.recipient, request.amount)
                    .is_err()
                {
                    return Err(Error::InsufficientFunds);
                }

                // Update escrow status
                let new_escrow_status = match request.approval_type {
                    ApprovalType::Release => EscrowStatus::Released,
                    ApprovalType::Refund => EscrowStatus::Refunded,
                    ApprovalType::EmergencyOverride => EscrowStatus::Released,
                };
                let mut updated_escrow = escrow.clone();
                updated_escrow.status = new_escrow_status;
                self.escrows.insert(&request.escrow_id, &updated_escrow);

                // Mark request as executed
                let mut executed_request = request.clone();
                executed_request.status = LargeTransferStatus::Executed;
                self.large_transfer_requests
                    .insert(&request_id, &executed_request);

                // Clear the active index
                self.escrow_active_large_transfer.remove(&request.escrow_id);

                self.add_audit_entry(
                    request.escrow_id,
                    caller,
                    "LargeTransferExecuted".to_string(),
                    format!(
                        "Request {}: {} transferred to {:?}",
                        request_id, request.amount, request.recipient
                    ),
                );

                self.env().emit_event(LargeTransferExecuted {
                    request_id,
                    escrow_id: request.escrow_id,
                    amount: request.amount,
                    recipient: request.recipient,
                    executed_by: caller,
                });

                Ok(())
            })
        }

        /// Cancel a pending large-transfer approval request.
        ///
        /// Only the initiator of the request or the admin may cancel.
        /// Cancellation is only allowed while the request is still `Pending`
        /// (not yet `Approved` or `Executed`).
        #[ink(message)]
        pub fn cancel_large_transfer(&mut self, request_id: u64) -> Result<(), Error> {
            let caller = self.env().caller();

            let mut request = self
                .large_transfer_requests
                .get(&request_id)
                .ok_or(Error::ApprovalRequestNotFound)?;

            if matches!(request.status, LargeTransferStatus::Executed) {
                return Err(Error::ApprovalRequestAlreadyExecuted);
            }
            if matches!(request.status, LargeTransferStatus::Cancelled) {
                return Err(Error::ApprovalRequestCancelled);
            }

            // Only initiator or admin may cancel
            if caller != request.initiated_by && caller != self.admin {
                return Err(Error::Unauthorized);
            }

            request.status = LargeTransferStatus::Cancelled;
            self.large_transfer_requests.insert(&request_id, &request);

            // Clear the active index so a new request can be created
            self.escrow_active_large_transfer.remove(&request.escrow_id);

            self.add_audit_entry(
                request.escrow_id,
                caller,
                "LargeTransferCancelled".to_string(),
                format!("Request {} cancelled", request_id),
            );

            self.env().emit_event(LargeTransferCancelled {
                request_id,
                escrow_id: request.escrow_id,
                cancelled_by: caller,
            });

            Ok(())
        }

        /// Update the large-transfer thresholds (admin only).
        ///
        /// Pass `0` for either value to revert to the global constant defined
        /// in `propchain_traits::constants`.
        #[ink(message)]
        pub fn set_large_transfer_thresholds(
            &mut self,
            large_threshold: u128,
            very_large_threshold: u128,
        ) -> Result<(), Error> {
            if self.env().caller() != self.admin {
                return Err(Error::Unauthorized);
            }
            // very_large must be strictly greater than large (or both zero)
            if large_threshold > 0
                && very_large_threshold > 0
                && very_large_threshold <= large_threshold
            {
                return Err(Error::InvalidConfiguration);
            }
            self.large_transfer_threshold = large_threshold;
            self.very_large_transfer_threshold = very_large_threshold;
            Ok(())
        }

        // ── Multi-Step Approval Query Messages ──────────────────────────────

        /// Get a large-transfer approval request by ID.
        #[ink(message)]
        pub fn get_large_transfer_request(&self, request_id: u64) -> Option<LargeTransferRequest> {
            self.large_transfer_requests.get(&request_id)
        }

        /// Get the active large-transfer request ID for an escrow (0 = none).
        #[ink(message)]
        pub fn get_active_large_transfer_request(&self, escrow_id: u64) -> u64 {
            self.escrow_active_large_transfer
                .get(&escrow_id)
                .unwrap_or(0)
        }

        /// Get the effective large-transfer thresholds (respects overrides).
        #[ink(message)]
        pub fn get_large_transfer_thresholds(&self) -> (u128, u128) {
            (
                self.effective_large_threshold(),
                self.effective_very_large_threshold(),
            )
        }

        // Query functions

        /// Get escrow details
        #[ink(message)]
        pub fn get_escrow(&self, escrow_id: u64) -> Option<EscrowData> {
            self.escrows.get(&escrow_id)
        }

        /// Get documents for escrow
        #[ink(message)]
        pub fn get_documents(&self, escrow_id: u64) -> Vec<DocumentHash> {
            self.documents.get(&escrow_id).unwrap_or_default()
        }

        /// Get conditions for escrow
        #[ink(message)]
        pub fn get_conditions(&self, escrow_id: u64) -> Vec<Condition> {
            self.conditions.get(&escrow_id).unwrap_or_default()
        }

        /// Get dispute information
        #[ink(message)]
        pub fn get_dispute(&self, escrow_id: u64) -> Option<DisputeInfo> {
            self.disputes.get(&escrow_id)
        }

        /// Get audit trail
        #[ink(message)]
        pub fn get_audit_trail(&self, escrow_id: u64) -> Vec<AuditEntry> {
            self.audit_logs.get(&escrow_id).unwrap_or_default()
        }

        /// Get multi-sig configuration
        #[ink(message)]
        pub fn get_multi_sig_config(&self, escrow_id: u64) -> Option<MultiSigConfig> {
            self.multi_sig_configs.get(&escrow_id)
        }

        /// Get signature count for approval type
        #[ink(message)]
        pub fn get_signature_count(&self, escrow_id: u64, approval_type: ApprovalType) -> u8 {
            self.signature_counts
                .get(&(escrow_id, approval_type))
                .unwrap_or(0)
        }

        /// Check if all conditions are met
        #[ink(message)]
        pub fn check_all_conditions_met(&self, escrow_id: u64) -> Result<bool, Error> {
            let conditions = self.conditions.get(&escrow_id).unwrap_or_default();

            // If no conditions, return true
            if conditions.is_empty() {
                return Ok(true);
            }

            // Check if all conditions are met
            Ok(conditions.iter().all(|c| c.met))
        }

        /// Set admin (deprecated — prefer request_admin_rotation + confirm_admin_rotation)
        #[ink(message)]
        pub fn set_admin(&mut self, new_admin: AccountId) -> Result<(), Error> {
            let caller = self.env().caller();

            if caller != self.admin {
                return Err(Error::Unauthorized);
            }

            self.admin = new_admin;
            Ok(())
        }

        /// Request a two-step admin rotation with cooldown.
        /// The new admin must call `confirm_admin_rotation` after the cooldown period.
        #[ink(message)]
        pub fn request_admin_rotation(&mut self, new_admin: AccountId) -> Result<(), Error> {
            let caller = self.env().caller();
            if caller != self.admin {
                return Err(Error::Unauthorized);
            }

            let block = self.env().block_number();
            let effective_at =
                block.saturating_add(propchain_traits::constants::KEY_ROTATION_COOLDOWN_BLOCKS);

            let request = propchain_traits::KeyRotationRequest {
                old_account: caller,
                new_account: new_admin,
                requested_at: block,
                effective_at,
                confirmed: false,
            };

            self.pending_admin_rotation = Some(request);

            self.add_audit_entry(
                0,
                caller,
                "AdminRotationRequested".to_string(),
                format!("New admin: {:?}", new_admin),
            );

            Ok(())
        }

        /// Confirm a pending admin rotation. Must be called by the new admin
        /// after the cooldown period has elapsed.
        #[ink(message)]
        pub fn confirm_admin_rotation(&mut self) -> Result<(), Error> {
            let caller = self.env().caller();
            let block = self.env().block_number();

            let request = self
                .pending_admin_rotation
                .as_ref()
                .ok_or(Error::InvalidConfiguration)?;

            if request.new_account != caller {
                return Err(Error::Unauthorized);
            }

            if block < request.effective_at {
                return Err(Error::TimeLockActive);
            }

            let expiry = request
                .effective_at
                .saturating_add(propchain_traits::constants::KEY_ROTATION_EXPIRY_BLOCKS);
            if block > expiry {
                self.pending_admin_rotation = None;
                return Err(Error::InvalidConfiguration);
            }

            self.admin = caller;
            self.pending_admin_rotation = None;

            self.add_audit_entry(
                0,
                caller,
                "AdminRotationCompleted".to_string(),
                "Admin rotation confirmed".to_string(),
            );

            Ok(())
        }

        /// Cancel a pending admin rotation.
        #[ink(message)]
        pub fn cancel_admin_rotation(&mut self) -> Result<(), Error> {
            let caller = self.env().caller();

            let request = self
                .pending_admin_rotation
                .as_ref()
                .ok_or(Error::InvalidConfiguration)?;

            if caller != request.old_account && caller != request.new_account {
                return Err(Error::Unauthorized);
            }

            self.pending_admin_rotation = None;
            Ok(())
        }

        /// Get admin
        #[ink(message)]
        pub fn get_admin(&self) -> AccountId {
            self.admin
        }

        /// Get high-value threshold
        #[ink(message)]
        pub fn get_high_value_threshold(&self) -> u128 {
            self.min_high_value_threshold
        }

        // Helper functions

        // ── Large-Transfer Helpers ───────────────────────────────────────────

        /// Returns the effective large-transfer threshold, preferring the
        /// per-contract override when set.
        fn effective_large_threshold(&self) -> u128 {
            if self.large_transfer_threshold > 0 {
                self.large_transfer_threshold
            } else {
                propchain_traits::constants::LARGE_TRANSFER_THRESHOLD
            }
        }

        /// Returns the effective very-large-transfer threshold.
        fn effective_very_large_threshold(&self) -> u128 {
            if self.very_large_transfer_threshold > 0 {
                self.very_large_transfer_threshold
            } else {
                propchain_traits::constants::VERY_LARGE_TRANSFER_THRESHOLD
            }
        }

        /// Classify an amount into a `TransferApprovalTier`.
        fn classify_transfer_tier(&self, amount: u128) -> TransferApprovalTier {
            if amount >= self.effective_very_large_threshold() {
                TransferApprovalTier::VeryLarge
            } else if amount >= self.effective_large_threshold() {
                TransferApprovalTier::Large
            } else {
                TransferApprovalTier::Standard
            }
        }

        /// Create and store a new `LargeTransferRequest`.
        ///
        /// Also records the request ID in `escrow_active_large_transfer` so
        /// callers can look it up without iterating.
        fn create_large_transfer_request(
            &mut self,
            escrow_id: u64,
            approval_type: ApprovalType,
            amount: u128,
            recipient: AccountId,
            tier: TransferApprovalTier,
            initiated_by: AccountId,
        ) -> Result<u64, Error> {
            let required_approvals = match tier {
                TransferApprovalTier::VeryLarge => {
                    propchain_traits::constants::VERY_LARGE_TRANSFER_REQUIRED_APPROVALS
                }
                TransferApprovalTier::Large => {
                    propchain_traits::constants::LARGE_TRANSFER_REQUIRED_APPROVALS
                }
                TransferApprovalTier::Standard => 1,
            };

            self.large_transfer_request_count += 1;
            let request_id = self.large_transfer_request_count;
            let current_block = u64::from(self.env().block_number());
            let expires_at_block = current_block
                .saturating_add(propchain_traits::constants::LARGE_TRANSFER_APPROVAL_EXPIRY_BLOCKS);

            let request = LargeTransferRequest {
                request_id,
                escrow_id,
                approval_type: approval_type.clone(),
                amount,
                recipient,
                tier,
                required_approvals,
                approvals: Vec::new(),
                initiated_by,
                created_at_block: current_block,
                expires_at_block,
                status: LargeTransferStatus::Pending,
            };

            self.large_transfer_requests.insert(&request_id, &request);
            self.escrow_active_large_transfer
                .insert(&escrow_id, &request_id);

            self.add_audit_entry(
                escrow_id,
                initiated_by,
                "LargeTransferRequested".to_string(),
                format!(
                    "Request {}: amount={}, required_approvals={}, expires_at_block={}",
                    request_id, amount, required_approvals, expires_at_block
                ),
            );

            self.env().emit_event(LargeTransferRequested {
                request_id,
                escrow_id,
                approval_type,
                amount,
                recipient,
                required_approvals,
                expires_at_block,
            });

            Ok(request_id)
        }

        /// Check if signature threshold is met
        fn check_signature_threshold(
            &self,
            escrow_id: u64,
            approval_type: ApprovalType,
        ) -> Result<bool, Error> {
            let config = self
                .multi_sig_configs
                .get(&escrow_id)
                .ok_or(Error::EscrowNotFound)?;
            let count = self
                .signature_counts
                .get(&(escrow_id, approval_type))
                .unwrap_or(0);
            Ok(count >= config.required_signatures)
        }

        /// Add audit entry
        fn add_audit_entry(
            &mut self,
            escrow_id: u64,
            actor: AccountId,
            action: String,
            details: String,
        ) {
            let entry = AuditEntry {
                timestamp: self.env().block_timestamp(),
                actor,
                action,
                details,
            };

            let mut logs = self.audit_logs.get(&escrow_id).unwrap_or_default();
            logs.push(entry);
            self.audit_logs.insert(&escrow_id, &logs);
        }
    }

    impl Default for AdvancedEscrow {
        fn default() -> Self {
            Self::new(1_000_000_000_000) // Default threshold: 1 token
        }
    }
}
