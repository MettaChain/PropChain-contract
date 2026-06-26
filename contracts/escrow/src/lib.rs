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
        /// Compact escrow summaries retained after cleanup
        escrow_summaries: Mapping<u64, EscrowSummary>,
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
        /// Compressed audit logs retained after cleanup
        compressed_audit_logs: Mapping<u64, Vec<CompressedAuditEntry>>,
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
        /// Tax compliance contract address
        tax_compliance_contract: Option<AccountId>,
        /// Escrow fee rate in basis points (e.g. 100 = 1%)
        fee_rate_bps: u16,
        /// Fee recipient account
        fee_recipient: Option<AccountId>,
        /// Aggregated escrow analytics for dashboard display
        analytics: EscrowAnalytics,
        /// Tracks unique participants: AccountId -> bool
        analytics_participants: Mapping<AccountId, bool>,
    }

    // Events
    #[ink(event)]
    pub struct EscrowCreated {
        #[ink(topic)]
        escrow_id: u64,
        #[ink(topic)]
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
    pub struct FundsPartiallyReleased {
        #[ink(topic)]
        escrow_id: u64,
        amount: u128,
        recipient: AccountId,
        remaining: u128,
    }

    #[ink(event)]
    pub struct FundsRefunded {
        #[ink(topic)]
        escrow_id: u64,
        amount: u128,
        recipient: AccountId,
    }

    /// Emitted when escrow storage has been cleaned up after completion.
    #[ink(event)]
    pub struct EscrowCleanedUp {
        #[ink(topic)]
        escrow_id: u64,
        #[ink(topic)]
        property_id: u64,
        #[ink(topic)]
        status: EscrowStatus,
        #[ink(topic)]
        cleaned_by: AccountId,
        event_version: u8,
        completed_at: u64,
        storage_saved_bytes: u64,
        timestamp: u64,
        block_number: u32,
        transaction_hash: Hash,
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

    #[ink(event)]
    pub struct FeeCollected {
        #[ink(topic)]
        escrow_id: u64,
        #[ink(topic)]
        fee_recipient: AccountId,
        fee_amount: u128,
        fee_rate_bps: u16,
    }

    #[ink(event)]
    pub struct FeeRateUpdated {
        #[ink(topic)]
        updated_by: AccountId,
        old_rate: u16,
        new_rate: u16,
    }

    #[ink(event)]
    pub struct FeeRecipientUpdated {
        #[ink(topic)]
        updated_by: AccountId,
        old_recipient: Option<AccountId>,
        new_recipient: Option<AccountId>,
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

    // ── Rating Events (Issue #216) ───────────────────────────────────────────

    /// Emitted when a participant rates another participant after an escrow.
    #[ink(event)]
    pub struct ParticipantRated {
        #[ink(topic)]
        escrow_id: u64,
        #[ink(topic)]
        rater: AccountId,
        #[ink(topic)]
        participant: AccountId,
        score: u8,
        rating_id: u64,
    }

    impl AdvancedEscrow {
        /// Constructor
        #[ink(constructor)]
        pub fn new(
            min_high_value_threshold: u128,
            tax_compliance_contract: Option<AccountId>,
        ) -> Self {
            Self {
                escrows: Mapping::default(),
                escrow_summaries: Mapping::default(),
                escrow_count: 0,
                multi_sig_configs: Mapping::default(),
                signatures: Mapping::default(),
                signature_counts: Mapping::default(),
                documents: Mapping::default(),
                conditions: Mapping::default(),
                condition_counters: Mapping::default(),
                disputes: Mapping::default(),
                audit_logs: Mapping::default(),
                compressed_audit_logs: Mapping::default(),
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
                tax_compliance_contract,
                fee_rate_bps: 0,
                fee_recipient: None,
                analytics: EscrowAnalytics {
                    total_created: 0,
                    total_released: 0,
                    total_refunded: 0,
                    total_disputed: 0,
                    total_active: 0,
                    total_volume: 0,
                    total_released_volume: 0,
                    total_fees_collected: 0,
                    average_escrow_amount: 0,
                    average_dispute_resolution_time: 0,
                    total_disputes_resolved: 0,
                    unique_participants: 0,
                },
                analytics_participants: Mapping::default(),
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
            jurisdiction: Jurisdiction,
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
                completed_at: None,
                release_time_lock,
                participants: participants.clone(),
                jurisdiction,
                total_released: 0,
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

            // Track analytics
            self.analytics.total_created += 1;
            self.analytics.total_volume = self.analytics.total_volume.saturating_add(amount);
            self.analytics.total_active += 1;
            self.analytics.average_escrow_amount =
                self.analytics.total_volume / self.analytics.total_created as u128;
            // Track unique participants
            for participant in &[buyer, seller] {
                if !self
                    .analytics_participants
                    .get(participant)
                    .unwrap_or(false)
                {
                    self.analytics_participants.insert(participant, &true);
                    self.analytics.unique_participants += 1;
                }
            }

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

            Ok(escrow_.id)
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

                // ── Tax Withholding ──────────────────────────────────────────
                let mut final_transfer_amount = escrow.deposited_amount;
                if let Some(tax_contract) = self.tax_compliance_contract {
                    use ink::env::call::FromAccountId;
                    let mut withholder: ink::contract_ref!(TaxWithholder) =
                        FromAccountId::from_account_id(tax_contract);

                    let (withheld_amount, collector) = withholder.withhold_tax(
                        escrow.property_id,
                        escrow.jurisdiction,
                        escrow.deposited_amount,
                    );

                    if withheld_amount > 0 {
                        if self.env().transfer(collector, withheld_amount).is_err() {
                            return Err(Error::InsufficientFunds);
                        }
                        final_transfer_amount =
                            final_transfer_amount.saturating_sub(withheld_amount);
                    }
                }
                // ── End Tax Withholding ──────────────────────────────────────

                // ── Fee Deduction ───────────────────────────────────────────
                let fee = if self.fee_rate_bps > 0 && self.fee_recipient.is_some() {
                    let calculated =
                        final_transfer_amount.saturating_mul(self.fee_rate_bps as u128) / 10_000;
                    if calculated > 0 {
                        let recipient = self.fee_recipient.unwrap();
                        if self.env().transfer(recipient, calculated).is_err() {
                            return Err(Error::InvalidFeeAmount);
                        }
                        self.env().emit_event(FeeCollected {
                            escrow_id,
                            fee_recipient: recipient,
                            fee_amount: calculated,
                            fee_rate_bps: self.fee_rate_bps,
                        });
                    }
                    calculated
                } else {
                    0
                };
                final_transfer_amount = final_transfer_amount.saturating_sub(fee);
                // ── End Fee Deduction ─────────────────────────────────────

                // Perform the transfer
                if self.env().transfer(escrow.seller, final_transfer_amount).is_err() {
                    return Err(Error::InsufficientFunds);
                }

                // Update escrow status
                let mut escrow_mut = escrow;
                escrow_mut.status = EscrowStatus::Released;
                escrow_mut.completed_at = Some(self.env().block_timestamp());
                self.escrows.insert(&escrow_id, &escrow_mut);

                // Add audit entry
                self.add_audit_entry(
                    escrow_id,
                    caller,
                    "FundsReleased".to_string(),
                    format!("Amount: {}", final_transfer_amount),
                );

                self.env().emit_event(FundsReleased {
                    escrow_id,
                    amount: final_transfer_amount,
                    recipient: escrow_mut.seller,
                });

                Ok(())
            })
        }

        /// Refund funds to the buyer with multi-signature approval.
        ///
        /// If the escrow's deposited amount exceeds the large-transfer threshold,
        /// this call creates a `LargeTransferRequest` and returns
        /// `Err(Error::LargeTransferApprovalRequired)`.  Authorised signers must
        /// then call `approve_large_transfer`, and once the required number of
        /// approvals is collected, anyone may call `execute_large_transfer` to
        /// finalise the transfer.
        #[ink(message)]
        pub fn refund_funds(&mut self, escrow_id: u64) -> Result<(), Error> {
            non_reentrant!(self, {
                let caller = self.env().caller();
                let escrow = self.escrows.get(&escrow_id).ok_or(Error::EscrowNotFound)?;

                // Check status
                if escrow.status != EscrowStatus::Active {
                    return Err(Error::InvalidStatus);
                }

                // Check multi-sig threshold
                if !self.check_signature_threshold(escrow_id, ApprovalType::Refund)? {
                    return Err(Error::SignatureThresholdNotMet);
                }

                // ── Large-transfer gate ──────────────────────────────────────
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

                // Perform the refund
                if self.env().transfer(escrow.buyer, escrow.deposited_amount).is_err() {
                    return Err(Error::InsufficientFunds);
                }

                // Update escrow status
                let mut escrow_mut = escrow;
                escrow_mut.status = EscrowStatus::Refunded;
                escrow_mut.completed_at = Some(self.env().block_timestamp());
                self.escrows.insert(&escrow_id, &escrow_mut);

                // Add audit entry
                self.add_audit_entry(
                    escrow_id,
                    caller,
                    "FundsRefunded".to_string(),
                    format!("Amount: {}", escrow.deposited_amount),
                );

                self.env().emit_event(FundsRefunded {
                    escrow_id,
                    amount: escrow.deposited_amount,
                    recipient: escrow_mut.buyer,
                });

                Ok(())
            })
        }

        /// Execute a large-transfer request that has collected all required approvals.
        ///
        /// Can be called by any participant once the request status is `Approved`.
        /// Performs the actual on-chain transfer and updates the escrow status.
        #[ink(message)]
        pub fn execute_large_transfer(&mut self, request_id: u64) -> Result<(), Error> {
            non_reentrant!(self, {
                let caller = self.env().caller();

                let mut request = self
                    .large_transfer_requests
                    .get(&request_id)
                    .ok_or(Error::ApprovalRequestNotFound)?;

                // Must be in Approved state
                if !matches!(request.status, LargeTransferStatus::Approved) {
                    return Err(Error::ApprovalNotComplete);
                }

                // Check for expiry
                if self.env().block_number() > request.expires_at_block {
                    request.status = LargeTransferStatus::Expired;
                    self.large_transfer_requests.insert(&request_id, &request);
                    return Err(Error::ApprovalRequestExpired);
                }

                let mut escrow = self
                    .escrows
                    .get(&request.escrow_id)
                    .ok_or(Error::EscrowNotFound)?;

                // Double-check escrow status
                if !matches!(escrow.status, EscrowStatus::Active) {
                    return Err(Error::InvalidStatus);
                }

                // The logic here depends on whether it's a release or refund
                match request.approval_type {
                    ApprovalType::Release => {
                        // This is a release to the seller
                        if self.env().transfer(escrow.seller, request.amount).is_err() {
                            return Err(Error::InsufficientFunds);
                        }
                        escrow.status = EscrowStatus::Released;
                    }
                    ApprovalType::Refund => {
                        // This is a refund to the buyer
                        if self.env().transfer(escrow.buyer, request.amount).is_err() {
                            return Err(Error::InsufficientFunds);
                        }
                        escrow.status = EscrowStatus::Refunded;
                    }
                    _ => return Err(Error::InvalidApprovalType),
                }

                // Update escrow and request states
                escrow.completed_at = Some(self.env().block_timestamp());
                self.escrows.insert(&request.escrow_id, &escrow);

                request.status = LargeTransferStatus::Executed;
                self.large_transfer_requests.insert(&request_id, &request);

                // Clear the active request index for this escrow
                self.escrow_active_large_transfer
                    .insert(&request.escrow_id, &0);

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
    }
}