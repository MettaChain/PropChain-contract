#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(not(feature = "std"))]
use scale_info::prelude::{string::String, vec::Vec};
use ink::storage::Mapping;

pub mod tests;

#[ink::contract]
mod propchain_escrow {
    use super::*;

    /// Error types for the escrow contract
    #[derive(Debug, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub enum Error {
        EscrowNotFound,
        Unauthorized,
        InvalidStatus,
        InsufficientFunds,
        ConditionsNotMet,
        SignatureThresholdNotMet,
        AlreadySigned,
        DocumentNotFound,
        DisputeActive,
        TimeLockActive,
        InvalidConfiguration,
        EscrowAlreadyFunded,
        ParticipantNotFound,
    }

    /// Escrow status enumeration
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

    /// Approval type for multi-signature operations
    #[derive(Debug, Clone, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    #[derive(ink::storage::traits::StorageLayout)]
    pub enum ApprovalType {
        Release,
        Refund,
        EmergencyOverride,
    }

    /// Main escrow data structure
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
    }

    /// Multi-signature configuration
    #[derive(Debug, Clone, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    #[derive(ink::storage::traits::StorageLayout)]
    pub struct MultiSigConfig {
        pub required_signatures: u8,
        pub signers: Vec<AccountId>,
    }

    /// Document hash with metadata
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

    /// Condition for escrow release
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

    /// Dispute information
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

    /// Audit trail entry
    #[derive(Debug, Clone, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    #[derive(ink::storage::traits::StorageLayout)]
    pub struct AuditEntry {
        pub timestamp: u64,
        pub actor: AccountId,
        pub action: String,
        pub details: String,
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
        signatures: Mapping<(u64, ApprovalType, AccountId), bool>,
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
            self.documents.insert(&escrow_id, &Vec::<DocumentHash>::new());
            self.conditions.insert(&escrow_id, &Vec::<Condition>::new());
            self.condition_counters.insert(&escrow_id, &0);
            self.audit_logs.insert(&escrow_id, &Vec::<AuditEntry>::new());

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

        /// Release funds with multi-signature approval
        #[ink(message)]
        pub fn release_funds(&mut self, escrow_id: u64) -> Result<(), Error> {
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

            // Transfer funds to seller
            if self.env().transfer(escrow.seller, escrow.deposited_amount).is_err() {
                return Err(Error::InsufficientFunds);
            }

            // Update status
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
        }

        /// Refund funds with multi-signature approval
        #[ink(message)]
        pub fn refund_funds(&mut self, escrow_id: u64) -> Result<(), Error> {
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

            // Transfer funds back to buyer
            if self.env().transfer(escrow.buyer, escrow.deposited_amount).is_err() {
                return Err(Error::InsufficientFunds);
            }

            // Update status
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
            if !escrow.participants.contains(&caller) && caller != escrow.buyer && caller != escrow.seller {
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
        pub fn verify_document(&mut self, escrow_id: u64, document_hash: Hash) -> Result<(), Error> {
            let caller = self.env().caller();
            let escrow = self.escrows.get(&escrow_id).ok_or(Error::EscrowNotFound)?;

            // Check if caller is a participant
            if !escrow.participants.contains(&caller) {
                return Err(Error::Unauthorized);
            }

            let mut docs = self.documents.get(&escrow_id).ok_or(Error::DocumentNotFound)?;
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
        pub fn mark_condition_met(&mut self, escrow_id: u64, condition_id: u64) -> Result<(), Error> {
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
        pub fn sign_approval(&mut self, escrow_id: u64, approval_type: ApprovalType) -> Result<(), Error> {
            let caller = self.env().caller();
            let _escrow = self.escrows.get(&escrow_id).ok_or(Error::EscrowNotFound)?;
            let config = self.multi_sig_configs.get(&escrow_id).ok_or(Error::EscrowNotFound)?;

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
            self.signature_counts.insert(&count_key, &(current_count + 1));

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
        pub fn emergency_override(&mut self, escrow_id: u64, release_to_seller: bool) -> Result<(), Error> {
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
            if self.env().transfer(recipient, escrow.deposited_amount).is_err() {
                return Err(Error::InsufficientFunds);
            }

            // Update status
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
            self.signature_counts.get(&(escrow_id, approval_type)).unwrap_or(0)
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

        /// Set admin
        #[ink(message)]
        pub fn set_admin(&mut self, new_admin: AccountId) -> Result<(), Error> {
            let caller = self.env().caller();

            if caller != self.admin {
                return Err(Error::Unauthorized);
            }

            self.admin = new_admin;
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

        /// Check if signature threshold is met
        fn check_signature_threshold(&self, escrow_id: u64, approval_type: ApprovalType) -> Result<bool, Error> {
            let config = self.multi_sig_configs.get(&escrow_id).ok_or(Error::EscrowNotFound)?;
            let count = self.signature_counts.get(&(escrow_id, approval_type)).unwrap_or(0);
            Ok(count >= config.required_signatures)
        }

        /// Add audit entry
        fn add_audit_entry(&mut self, escrow_id: u64, actor: AccountId, action: String, details: String) {
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
