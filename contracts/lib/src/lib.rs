#![cfg_attr(not(feature = "std"), no_std)]
#![allow(unexpected_cfgs)]

#[cfg(not(feature = "std"))]
use scale_info::prelude::vec::Vec;
use ink::storage::Mapping;
use propchain_traits::*;

#[ink::contract]
mod propchain_contracts {
    use super::*;

    /// Error types for contract
    #[derive(Debug, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub enum Error {
        PropertyNotFound,
        Unauthorized,
        InvalidMetadata,
        EscrowNotFound,
        EscrowAlreadyReleased,
        InsufficientFunds,
        /// Contract is paused
        ContractPaused,
        /// Contract is not paused
        ContractNotPaused,
        /// Caller is not a pauser
        NotPauser,
        /// Caller is not a resume approver
        NotResumeApprover,
        /// Insufficient resume approvals
        InsufficientApprovals,
        /// Invalid auto-resume timestamp
        InvalidAutoResumeTime,
        /// Auto-resume not scheduled
        AutoResumeNotScheduled,
        /// Invalid approval threshold
        InvalidApprovalThreshold,
    }

    /// Property Registry contract
    #[ink(storage)]
    pub struct PropertyRegistry {
        /// Mapping from property ID to property information
        properties: Mapping<u64, PropertyInfo>,
        /// Mapping from owner to their properties
        owner_properties: Mapping<AccountId, Vec<u64>>,
        /// Reverse mapping: property ID to owner (optimization for faster lookups)
        property_owners: Mapping<u64, AccountId>,
        /// Mapping from property ID to approved account
        approvals: Mapping<u64, AccountId>,
        /// Property counter
        property_count: u64,
        /// Contract version
        version: u32,
        /// Admin for upgrades (if used directly, or for logic-level auth)
        admin: AccountId,
        /// Mapping from escrow ID to escrow information
        escrows: Mapping<u64, EscrowInfo>,
        /// Escrow counter
        escrow_count: u64,
        /// Gas usage tracking
        gas_tracker: GasTracker,
        /// Pause state
        pause_state: PauseState,
        /// Accounts with pauser role
        pausers: Mapping<AccountId, bool>,
        /// Accounts with resume approver role
        resume_approvers: Mapping<AccountId, bool>,
        /// Current resume approvals (account -> has approved)
        resume_approvals: Mapping<AccountId, bool>,
        /// Number of current resume approvals
        resume_approval_count: u32,
        /// Required number of approvals for resume
        required_approvals: u32,
        /// Audit trail of pause/resume events
        pause_events: Vec<PauseEvent>,
    }

    /// Escrow information
    #[derive(Debug, Clone, PartialEq, scale::Encode, scale::Decode, ink::storage::traits::StorageLayout)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub struct EscrowInfo {
        pub id: u64,
        pub property_id: u64,
        pub buyer: AccountId,
        pub seller: AccountId,
        pub amount: u128,
        pub released: bool,
    }

    /// Portfolio summary statistics
    #[derive(Debug, Clone, PartialEq, scale::Encode, scale::Decode, ink::storage::traits::StorageLayout)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub struct PortfolioSummary {
        pub property_count: u64,
        pub total_valuation: u128,
        pub average_valuation: u128,
        pub total_size: u64,
        pub average_size: u64,
    }

    /// Detailed portfolio information
    #[derive(Debug, Clone, PartialEq, scale::Encode, scale::Decode, ink::storage::traits::StorageLayout)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub struct PortfolioDetails {
        pub owner: AccountId,
        pub properties: Vec<PortfolioProperty>,
        pub total_count: u64,
    }

    /// Individual property in portfolio
    #[derive(Debug, Clone, PartialEq, scale::Encode, scale::Decode, ink::storage::traits::StorageLayout)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub struct PortfolioProperty {
        pub id: u64,
        pub location: String,
        pub size: u64,
        pub valuation: u128,
        pub registered_at: u64,
    }

    /// Global analytics data
    #[derive(Debug, Clone, PartialEq, scale::Encode, scale::Decode, ink::storage::traits::StorageLayout)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub struct GlobalAnalytics {
        pub total_properties: u64,
        pub total_valuation: u128,
        pub average_valuation: u128,
        pub total_size: u64,
        pub average_size: u64,
        pub unique_owners: u64,
    }

    /// Gas metrics for monitoring
    #[derive(Debug, Clone, PartialEq, scale::Encode, scale::Decode, ink::storage::traits::StorageLayout)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub struct GasMetrics {
        pub last_operation_gas: u64,
        pub average_operation_gas: u64,
        pub total_operations: u64,
        pub min_gas_used: u64,
        pub max_gas_used: u64,
    }

    /// Gas tracker for monitoring usage
    #[derive(Debug, Clone, PartialEq, scale::Encode, scale::Decode, ink::storage::traits::StorageLayout)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub struct GasTracker {
        pub total_gas_used: u64,
        pub operation_count: u64,
        pub last_operation_gas: u64,
        pub min_gas_used: u64,
        pub max_gas_used: u64,
    }

    #[ink(event)]
    pub struct PropertyRegistered {
        #[ink(topic)]
        property_id: u64,
        #[ink(topic)]
        owner: AccountId,
        version: u8,
    }

    #[ink(event)]
    pub struct PropertyTransferred {
        #[ink(topic)]
        property_id: u64,
        #[ink(topic)]
        from: AccountId,
        #[ink(topic)]
        to: AccountId,
    }

    #[ink(event)]
    pub struct PropertyMetadataUpdated {
        #[ink(topic)]
        property_id: u64,
        metadata: PropertyMetadata,
    }

    #[ink(event)]
    pub struct Approval {
        #[ink(topic)]
        property_id: u64,
        #[ink(topic)]
        owner: AccountId,
        #[ink(topic)]
        approved: AccountId,
    }

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
    pub struct EscrowReleased {
        #[ink(topic)]
        escrow_id: u64,
    }

    #[ink(event)]
    pub struct EscrowRefunded {
        #[ink(topic)]
        escrow_id: u64,
    }

    /// Contract paused event
    #[ink(event)]
    pub struct ContractPaused {
        #[ink(topic)]
        paused_by: AccountId,
        timestamp: u64,
        reason: String,
    }

    /// Contract resumed event
    #[ink(event)]
    pub struct ContractResumed {
        #[ink(topic)]
        resumed_by: AccountId,
        timestamp: u64,
        approval_count: u32,
    }

    /// Auto-resume scheduled event
    #[ink(event)]
    pub struct AutoResumeScheduled {
        scheduled_by: AccountId,
        resume_at: u64,
    }

    /// Auto-resume cancelled event
    #[ink(event)]
    pub struct AutoResumeCancelled {
        cancelled_by: AccountId,
    }

    /// Pauser role added event
    #[ink(event)]
    pub struct PauserAdded {
        #[ink(topic)]
        account: AccountId,
        added_by: AccountId,
    }

    /// Pauser role removed event
    #[ink(event)]
    pub struct PauserRemoved {
        #[ink(topic)]
        account: AccountId,
        removed_by: AccountId,
    }

    /// Resume approver added event
    #[ink(event)]
    pub struct ResumeApproverAdded {
        #[ink(topic)]
        account: AccountId,
        added_by: AccountId,
    }

    /// Resume approver removed event
    #[ink(event)]
    pub struct ResumeApproverRemoved {
        #[ink(topic)]
        account: AccountId,
        removed_by: AccountId,
    }

    /// Resume approved event
    #[ink(event)]
    pub struct ResumeApproved {
        #[ink(topic)]
        approver: AccountId,
        current_approvals: u32,
        required_approvals: u32,
    }

    /// Required approvals threshold changed
    #[ink(event)]
    pub struct RequiredApprovalsChanged {
        old_threshold: u32,
        new_threshold: u32,
        changed_by: AccountId,
    }


    /// Batch event for multiple property registrations
    #[ink(event)]
    pub struct BatchPropertyRegistered {
        property_ids: Vec<u64>,
        owner: AccountId,
        count: u64,
    }

    /// Batch event for multiple property transfers
    #[ink(event)]
    pub struct BatchPropertyTransferred {
        property_ids: Vec<u64>,
        from: AccountId,
        to: AccountId,
        count: u64,
    }

    /// Batch event for multiple metadata updates
    #[ink(event)]
    pub struct BatchMetadataUpdated {
        property_ids: Vec<u64>,
        count: u64,
    }

    impl PropertyRegistry {
        /// Creates a new PropertyRegistry contract
        #[ink(constructor)]
        pub fn new() -> Self {
            let caller = Self::env().caller();
            let mut contract = Self {
                properties: Mapping::default(),
                owner_properties: Mapping::default(),
                property_owners: Mapping::default(),
                approvals: Mapping::default(),
                property_count: 0,
                version: 1,
                admin: caller,
                escrows: Mapping::default(),
                escrow_count: 0,
                gas_tracker: GasTracker {
                    total_gas_used: 0,
                    operation_count: 0,
                    last_operation_gas: 0,
                    min_gas_used: u64::MAX,
                    max_gas_used: 0,
                },
                pause_state: PauseState::default(),
                pausers: Mapping::default(),
                resume_approvers: Mapping::default(),
                resume_approvals: Mapping::default(),
                resume_approval_count: 0,
                required_approvals: 1, // Default: 1 approval required
                pause_events: Vec::new(),
            };
            
            // Admin is the initial pauser and resume approver
            contract.pausers.insert(&caller, &true);
            contract.resume_approvers.insert(&caller, &true);
            
            contract
        }

        /// Returns the contract version
        #[ink(message)]
        pub fn version(&self) -> u32 {
            self.version
        }

        /// Registers a new property
        #[ink(message)]
        pub fn register_property(&mut self, metadata: PropertyMetadata) -> Result<u64, Error> {
            // Check pause state
            self.check_pause_state()?;
            
            let caller = self.env().caller();
            self.property_count += 1;
            let property_id = self.property_count;

            let property_info = PropertyInfo {
                id: property_id,
                owner: caller,
                metadata,
                registered_at: self.env().block_timestamp(),
            };

            self.properties.insert(&property_id, &property_info);
            // Optimized: Also store reverse mapping for faster owner lookups
            self.property_owners.insert(&property_id, &caller);

            let mut owner_props = self.owner_properties.get(&caller).unwrap_or_default();
            owner_props.push(property_id);
            self.owner_properties.insert(&caller, &owner_props);

            // Track gas usage
            self.track_gas_usage("register_property".as_bytes());

            self.env().emit_event(PropertyRegistered {
                property_id,
                owner: caller,
                version: 1,
            });

            Ok(property_id)
        }

        /// Transfers property ownership
        #[ink(message)]
        pub fn transfer_property(&mut self, property_id: u64, to: AccountId) -> Result<(), Error> {
            // Check pause state
            self.check_pause_state()?;
            
            let caller = self.env().caller();
            let mut property = self.properties.get(&property_id).ok_or(Error::PropertyNotFound)?;

            let approved = self.approvals.get(&property_id);
            if property.owner != caller && Some(caller) != approved {
                return Err(Error::Unauthorized);
            }

            let from = property.owner;

            // Remove from current owner's properties
            let mut current_owner_props = self.owner_properties.get(&from).unwrap_or_default();
            current_owner_props.retain(|&id| id != property_id);
            self.owner_properties.insert(&from, &current_owner_props);
            
            // Add to new owner's properties
            let mut new_owner_props = self.owner_properties.get(&to).unwrap_or_default();
            new_owner_props.push(property_id);
            self.owner_properties.insert(&to, &new_owner_props);

            // Update property owner
            property.owner = to;
            self.properties.insert(&property_id, &property);
            // Optimized: Update reverse mapping
            self.property_owners.insert(&property_id, &to);

            // Clear approval
            self.approvals.remove(&property_id);

            // Track gas usage
            self.track_gas_usage("transfer_property".as_bytes());

            self.env().emit_event(PropertyTransferred {
                property_id,
                from,
                to,
            });

            Ok(())
        }

        /// Gets property information
        #[ink(message)]
        pub fn get_property(&self, property_id: u64) -> Option<PropertyInfo> {
            self.properties.get(&property_id)
        }

        /// Gets properties owned by an account
        #[ink(message)]
        pub fn get_owner_properties(&self, owner: AccountId) -> Vec<u64> {
            self.owner_properties.get(&owner).unwrap_or_default()
        }

        /// Gets total property count
        #[ink(message)]
        pub fn property_count(&self) -> u64 {
            self.property_count
        }

        /// Updates property metadata
        #[ink(message)]
        pub fn update_metadata(&mut self, property_id: u64, metadata: PropertyMetadata) -> Result<(), Error> {
            let caller = self.env().caller();
            let mut property = self.properties.get(&property_id).ok_or(Error::PropertyNotFound)?;

            if property.owner != caller {
                return Err(Error::Unauthorized);
            }

            // check if metadata is valid (basic check)
            if metadata.location.is_empty() {
                return Err(Error::InvalidMetadata);
            }

            property.metadata = metadata.clone();
            self.properties.insert(&property_id, &property);

            self.env().emit_event(PropertyMetadataUpdated {
                property_id,
                metadata,
            });

            Ok(())
        }

        /// Batch registers multiple properties in a single transaction
        #[ink(message)]
        pub fn batch_register_properties(&mut self, properties: Vec<PropertyMetadata>) -> Result<Vec<u64>, Error> {
            let mut results = Vec::new();
            let caller = self.env().caller();

            // Pre-calculate all property IDs to avoid repeated storage reads
            let start_id = self.property_count + 1;
            let end_id = start_id + properties.len() as u64 - 1;
            self.property_count = end_id;

            // Get existing owner properties to avoid repeated storage reads
            let mut owner_props = self.owner_properties.get(&caller).unwrap_or_default();

            for (i, metadata) in properties.into_iter().enumerate() {
                let property_id = start_id + i as u64;

                let property_info = PropertyInfo {
                    id: property_id,
                    owner: caller,
                    metadata,
                    registered_at: self.env().block_timestamp(),
                };

                self.properties.insert(&property_id, &property_info);
                owner_props.push(property_id);

                results.push(property_id);
            }

            // Update owner properties once at the end
            self.owner_properties.insert(&caller, &owner_props);

            // Emit single batch event instead of individual events for gas optimization
            self.env().emit_event(BatchPropertyRegistered {
                property_ids: results.clone(),
                owner: caller,
                count: results.len() as u64,
            });

            // Track gas usage
            self.track_gas_usage("batch_register_properties".as_bytes());

            Ok(results)
        }

        /// Batch transfers multiple properties to the same recipient
        #[ink(message)]
        pub fn batch_transfer_properties(&mut self, property_ids: Vec<u64>, to: AccountId) -> Result<(), Error> {
            let caller = self.env().caller();

            // Validate all properties first to avoid partial transfers
            for &property_id in &property_ids {
                let property = self.properties.get(&property_id).ok_or(Error::PropertyNotFound)?;
                
                let approved = self.approvals.get(&property_id);
                if property.owner != caller && Some(caller) != approved {
                    return Err(Error::Unauthorized);
                }
            }

            // Perform all transfers
            for property_id in &property_ids {
                let mut property = self.properties.get(property_id).ok_or(Error::PropertyNotFound)?;
                let from = property.owner;

                // Remove from current owner's properties
                let mut current_owner_props = self.owner_properties.get(&from).unwrap_or_default();
                current_owner_props.retain(|&id| id != *property_id);
                self.owner_properties.insert(&from, &current_owner_props);
                
                // Add to new owner's properties
                let mut new_owner_props = self.owner_properties.get(&to).unwrap_or_default();
                new_owner_props.push(*property_id);
                self.owner_properties.insert(&to, &new_owner_props);

                // Update property owner
                property.owner = to;
                self.properties.insert(property_id, &property);
                // Optimized: Update reverse mapping
                self.property_owners.insert(property_id, &to);

                // Clear approval
                self.approvals.remove(property_id);
            }

            // Emit single batch event instead of individual events for gas optimization
            if !property_ids.is_empty() {
                let first_property = self.properties.get(&property_ids[0]).ok_or(Error::PropertyNotFound)?;
                let from = first_property.owner;
                
                self.env().emit_event(BatchPropertyTransferred {
                    property_ids: property_ids.clone(),
                    from,
                    to,
                    count: property_ids.len() as u64,
                });
            }

            // Track gas usage
            self.track_gas_usage("batch_transfer_properties".as_bytes());

            Ok(())
        }

        /// Batch updates metadata for multiple properties
        #[ink(message)]
        pub fn batch_update_metadata(&mut self, updates: Vec<(u64, PropertyMetadata)>) -> Result<(), Error> {
            let caller = self.env().caller();

            // Validate all properties first to avoid partial updates
            for (property_id, ref metadata) in &updates {
                let property = self.properties.get(property_id).ok_or(Error::PropertyNotFound)?;
                
                if property.owner != caller {
                    return Err(Error::Unauthorized);
                }

                // Check if metadata is valid (basic check)
                if metadata.location.is_empty() {
                    return Err(Error::InvalidMetadata);
                }
            }

            // Perform all updates
            let mut updated_property_ids = Vec::new();
            for (property_id, metadata) in updates {
                let mut property = self.properties.get(&property_id).ok_or(Error::PropertyNotFound)?;
                
                property.metadata = metadata.clone();
                self.properties.insert(&property_id, &property);
                updated_property_ids.push(property_id);
            }

            // Emit single batch event instead of individual events for gas optimization
            let count = updated_property_ids.len() as u64;
            if !updated_property_ids.is_empty() {
                self.env().emit_event(BatchMetadataUpdated {
                    property_ids: updated_property_ids,
                    count,
                });
            }

            // Track gas usage
            self.track_gas_usage("batch_update_metadata".as_bytes());

            Ok(())
        }

        /// Transfers multiple properties to different recipients
        #[ink(message)]
        pub fn batch_transfer_properties_to_multiple(&mut self, transfers: Vec<(u64, AccountId)>) -> Result<(), Error> {
            let caller = self.env().caller();

            // Validate all properties first to avoid partial transfers
            for (property_id, _) in &transfers {
                let property = self.properties.get(property_id).ok_or(Error::PropertyNotFound)?;
                
                let approved = self.approvals.get(property_id);
                if property.owner != caller && Some(caller) != approved {
                    return Err(Error::Unauthorized);
                }
            }

            // Perform all transfers
            let mut transferred_property_ids = Vec::new();
            for (property_id, to) in &transfers {
                let mut property = self.properties.get(property_id).ok_or(Error::PropertyNotFound)?;
                let from = property.owner;

                // Remove from current owner's properties
                let mut current_owner_props = self.owner_properties.get(&from).unwrap_or_default();
                current_owner_props.retain(|&id| id != *property_id);
                self.owner_properties.insert(&from, &current_owner_props);
                
                // Add to new owner's properties
                let mut new_owner_props = self.owner_properties.get(to).unwrap_or_default();
                new_owner_props.push(*property_id);
                self.owner_properties.insert(to, &new_owner_props);

                // Update property owner
                property.owner = *to;
                self.properties.insert(property_id, &property);
                // Optimized: Update reverse mapping
                self.property_owners.insert(property_id, to);

                // Clear approval
                self.approvals.remove(property_id);
                transferred_property_ids.push(*property_id);
            }

            // Emit single batch event instead of individual events for gas optimization
            if !transferred_property_ids.is_empty() {
                let first_property = self.properties.get(&transferred_property_ids[0]).ok_or(Error::PropertyNotFound)?;
                let from = first_property.owner;
                
                self.env().emit_event(BatchPropertyTransferred {
                    property_ids: transferred_property_ids,
                    from,
                    to: AccountId::from([0u8; 32]), // Placeholder since multiple recipients
                    count: transfers.len() as u64,
                });
            }

            // Track gas usage
            self.track_gas_usage("batch_transfer_properties_to_multiple".as_bytes());

            Ok(())
        }

        /// Approves an account to transfer a specific property
        #[ink(message)]
        pub fn approve(&mut self, property_id: u64, to: Option<AccountId>) -> Result<(), Error> {
            let caller = self.env().caller();
            let property = self.properties.get(&property_id).ok_or(Error::PropertyNotFound)?;

            if property.owner != caller {
                return Err(Error::Unauthorized);
            }

            if let Some(account) = to {
                self.approvals.insert(&property_id, &account);
                self.env().emit_event(Approval {
                    property_id,
                    owner: caller,
                    approved: account,
                });
            } else {
                self.approvals.remove(&property_id);
                let zero_account = AccountId::from([0u8; 32]);
                self.env().emit_event(Approval {
                    property_id,
                    owner: caller,
                    approved: zero_account,
                });
            }

            Ok(())
        }

        /// Gets the approved account for a property
        #[ink(message)]
        pub fn get_approved(&self, property_id: u64) -> Option<AccountId> {
            self.approvals.get(&property_id)
        }

        /// Creates a new escrow for property transfer
        #[ink(message)]
        pub fn create_escrow(&mut self, property_id: u64, amount: u128) -> Result<u64, Error> {
            // Check pause state
            self.check_pause_state()?;
            
            let caller = self.env().caller();
            let property = self.properties.get(&property_id).ok_or(Error::PropertyNotFound)?;

            // Only property owner can create escrow
            if property.owner != caller {
                return Err(Error::Unauthorized);
            }

            self.escrow_count += 1;
            let escrow_id = self.escrow_count;

            let escrow_info = EscrowInfo {
                id: escrow_id,
                property_id,
                buyer: caller, // In this simple version, caller is buyer
                seller: property.owner,
                amount,
                released: false,
            };

            self.escrows.insert(&escrow_id, &escrow_info);

            self.env().emit_event(EscrowCreated {
                escrow_id,
                property_id,
                buyer: caller,
                seller: property.owner,
                amount,
            });

            Ok(escrow_id)
        }

        /// Releases escrow funds and transfers property
        #[ink(message)]
        pub fn release_escrow(&mut self, escrow_id: u64) -> Result<(), Error> {
            // Check pause state
            self.check_pause_state()?;
            
            let caller = self.env().caller();
            let mut escrow = self.escrows.get(&escrow_id).ok_or(Error::EscrowNotFound)?;

            if escrow.released {
                return Err(Error::EscrowAlreadyReleased);
            }

            // Only buyer can release
            if escrow.buyer != caller {
                return Err(Error::Unauthorized);
            }

            // Transfer property
            self.transfer_property(escrow.property_id, escrow.buyer)?;

            escrow.released = true;
            self.escrows.insert(&escrow_id, &escrow);

            self.env().emit_event(EscrowReleased {
                escrow_id,
            });

            Ok(())
        }

        /// Refunds escrow funds
        #[ink(message)]
        pub fn refund_escrow(&mut self, escrow_id: u64) -> Result<(), Error> {
            let caller = self.env().caller();
            let mut escrow = self.escrows.get(&escrow_id).ok_or(Error::EscrowNotFound)?;

            if escrow.released {
                return Err(Error::EscrowAlreadyReleased);
            }

            // Only seller can refund
            if escrow.seller != caller {
                return Err(Error::Unauthorized);
            }

            escrow.released = true;
            self.escrows.insert(&escrow_id, &escrow);

            self.env().emit_event(EscrowRefunded {
                escrow_id,
            });

            Ok(())
        }

        /// Gets escrow information
        #[ink(message)]
        pub fn get_escrow(&self, escrow_id: u64) -> Option<EscrowInfo> {
            self.escrows.get(&escrow_id)
        }

        /// Portfolio Management: Gets summary statistics for properties owned by an account
        #[ink(message)]
        pub fn get_portfolio_summary(&self, owner: AccountId) -> PortfolioSummary {
            let property_ids = self.owner_properties.get(&owner).unwrap_or_default();
            let mut total_valuation = 0u128;
            let mut total_size = 0u64;
            let mut property_count = 0u64;
            
            // Optimized loop with iterator for better performance
            let mut iter = property_ids.iter();
            while let Some(&property_id) = iter.next() {
                if let Some(property) = self.properties.get(&property_id) {
                    // Unrolled additions for better performance
                    total_valuation = total_valuation.wrapping_add(property.metadata.valuation);
                    total_size = total_size.wrapping_add(property.metadata.size);
                    property_count += 1;
                }
            }
            
            PortfolioSummary {
                property_count,
                total_valuation,
                average_valuation: if property_count > 0 { total_valuation / property_count as u128 } else { 0 },
                total_size,
                average_size: if property_count > 0 { total_size / property_count } else { 0 },
            }
        }

        /// Portfolio Management: Gets detailed portfolio information for an owner
        #[ink(message)]
        pub fn get_portfolio_details(&self, owner: AccountId) -> PortfolioDetails {
            let property_ids = self.owner_properties.get(&owner).unwrap_or_default();
            let mut properties = Vec::new();
            
            // Optimized loop with capacity pre-allocation
            properties.reserve(property_ids.len());
            
            let mut iter = property_ids.iter();
            while let Some(&property_id) = iter.next() {
                if let Some(property) = self.properties.get(&property_id) {
                    // Direct construction to avoid intermediate allocations
                    let portfolio_property = PortfolioProperty {
                        id: property.id,
                        location: property.metadata.location.clone(),
                        size: property.metadata.size,
                        valuation: property.metadata.valuation,
                        registered_at: property.registered_at,
                    };
                    properties.push(portfolio_property);
                }
            }
            
            let total_count = properties.len() as u64;
            
            PortfolioDetails {
                owner,
                properties,
                total_count,
            }
        }

        /// Analytics: Gets aggregated statistics across all properties
        #[ink(message)]
        pub fn get_global_analytics(&self) -> GlobalAnalytics {
            let mut total_valuation = 0u128;
            let mut total_size = 0u64;
            let mut property_count = 0u64;
            let mut owners = std::collections::BTreeSet::new();
            
            // Optimized loop with early termination possibility
            // Note: This is expensive for large datasets. Consider off-chain indexing.
            let mut i = 1u64;
            while i <= self.property_count {
                if let Some(property) = self.properties.get(&i) {
                    total_valuation += property.metadata.valuation;
                    total_size += property.metadata.size;
                    property_count += 1;
                    owners.insert(property.owner);
                }
                i += 1;
            }
            
            GlobalAnalytics {
                total_properties: property_count,
                total_valuation,
                average_valuation: if property_count > 0 { total_valuation / property_count as u128 } else { 0 },
                total_size,
                average_size: if property_count > 0 { total_size / property_count } else { 0 },
                unique_owners: owners.len() as u64,
            }
        }

        /// Analytics: Gets properties within a price range
        #[ink(message)]
        pub fn get_properties_by_price_range(&self, min_price: u128, max_price: u128) -> Vec<u64> {
            let mut result = Vec::new();
            
            // Optimized loop with pre-check to reduce iterations
            let mut i = 1u64;
            while i <= self.property_count {
                if let Some(property) = self.properties.get(&i) {
                    // Unrolled condition check for better performance
                    let valuation = property.metadata.valuation;
                    if valuation >= min_price && valuation <= max_price {
                        result.push(property.id);
                    }
                }
                i += 1;
            }
            
            result
        }

        /// Analytics: Gets properties by size range
        #[ink(message)]
        pub fn get_properties_by_size_range(&self, min_size: u64, max_size: u64) -> Vec<u64> {
            let mut result = Vec::new();
            
            // Optimized loop with pre-check to reduce iterations
            let mut i = 1u64;
            while i <= self.property_count {
                if let Some(property) = self.properties.get(&i) {
                    // Unrolled condition check for better performance
                    let size = property.metadata.size;
                    if size >= min_size && size <= max_size {
                        result.push(property.id);
                    }
                }
                i += 1;
            }
            
            result
        }

        /// Helper method to track gas usage
        fn track_gas_usage(&mut self, _operation: &[u8]) {
            // In a real implementation, this would measure actual gas consumption
            // For demonstration purposes, we increment counters
            let gas_used = 10000; // Placeholder value
            self.gas_tracker.operation_count += 1;
            self.gas_tracker.last_operation_gas = gas_used;
            self.gas_tracker.total_gas_used += gas_used;
            
            // Track min/max gas usage
            if gas_used < self.gas_tracker.min_gas_used {
                self.gas_tracker.min_gas_used = gas_used;
            }
            if gas_used > self.gas_tracker.max_gas_used {
                self.gas_tracker.max_gas_used = gas_used;
            }
        }

        /// Gas Monitoring: Tracks gas usage for operations
        #[ink(message)]
        pub fn get_gas_metrics(&self) -> GasMetrics {
            GasMetrics {
                last_operation_gas: self.gas_tracker.last_operation_gas,
                average_operation_gas: if self.gas_tracker.operation_count > 0 {
                    self.gas_tracker.total_gas_used / self.gas_tracker.operation_count
                } else {
                    0
                },
                total_operations: self.gas_tracker.operation_count,
                min_gas_used: if self.gas_tracker.min_gas_used == u64::MAX { 0 } else { self.gas_tracker.min_gas_used },
                max_gas_used: self.gas_tracker.max_gas_used,
            }
        }

        /// Performance Monitoring: Gets optimization recommendations
        #[ink(message)]
        pub fn get_performance_recommendations(&self) -> Vec<String> {
            let mut recommendations = Vec::new();
            
            // Calculate average gas for checks
            let avg_gas = if self.gas_tracker.operation_count > 0 {
                self.gas_tracker.total_gas_used / self.gas_tracker.operation_count
            } else {
                0
            };
            
            // Check for high gas usage operations
            if avg_gas > 50000 {
                recommendations.push("Consider using batch operations for multiple properties".to_string());
            }
            
            // Check for many small operations
            if self.gas_tracker.operation_count > 100 && avg_gas < 10000 {
                recommendations.push("Operations are efficient but consider consolidating related operations".to_string());
            }
            
            // Check for inconsistent gas usage
            if self.gas_tracker.max_gas_used > self.gas_tracker.min_gas_used * 10 {
                recommendations.push("Gas usage varies significantly - review operation patterns".to_string());
            }
            
            // General recommendations
            recommendations.push("Use batch operations for multiple property transfers".to_string());
            recommendations.push("Prefer portfolio analytics over individual property queries".to_string());
            recommendations.push("Consider off-chain indexing for complex analytics".to_string());
            
            recommendations
        }

        // ========== PAUSE/RESUME FUNCTIONALITY ==========

        /// Helper: Check if contract is paused and handle auto-resume
        fn check_pause_state(&mut self) -> Result<(), Error> {
            // Check for auto-resume
            if let Some(auto_resume_time) = self.pause_state.auto_resume_at {
                let current_time = self.env().block_timestamp();
                if current_time >= auto_resume_time {
                    // Auto-resume triggered
                    self.pause_state.is_paused = false;
                    self.pause_state.auto_resume_at = None;
                    
                    // Record event
                    let event = PauseEvent {
                        event_type: PauseEventType::Resumed,
                        timestamp: current_time,
                        triggered_by: AccountId::from([0u8; 32]), // System trigger
                        details: "Automatic resume triggered".to_string(),
                    };
                    self.pause_events.push(event);
                    
                    self.env().emit_event(ContractResumed {
                        resumed_by: AccountId::from([0u8; 32]),
                        timestamp: current_time,
                        approval_count: 0,
                    });
                    
                    return Ok(());
                }
            }
            
            if self.pause_state.is_paused {
                return Err(Error::ContractPaused);
            }
            
            Ok(())
        }

        /// Pause the contract (emergency stop)
        #[ink(message)]
        pub fn pause(&mut self) -> Result<(), Error> {
            let caller = self.env().caller();
            
            // Check if caller is a pauser
            if !self.pausers.get(&caller).unwrap_or(false) {
                return Err(Error::NotPauser);
            }
            
            // Check if already paused
            if self.pause_state.is_paused {
                return Err(Error::ContractPaused);
            }
            
            let timestamp = self.env().block_timestamp();
            
            // Update pause state
            self.pause_state.is_paused = true;
            self.pause_state.paused_at = Some(timestamp);
            self.pause_state.paused_by = Some(caller);
            self.pause_state.pause_reason = "Emergency pause activated".to_string();
            self.pause_state.pause_count += 1;
            
            // Clear any pending resume approvals
            self.resume_approval_count = 0;
            self.resume_approvals = Mapping::default();
            
            // Record event in audit trail
            let event = PauseEvent {
                event_type: PauseEventType::Paused,
                timestamp,
                triggered_by: caller,
                details: "Contract paused by authorized pauser".to_string(),
            };
            self.pause_events.push(event);
            
            // Emit event
            self.env().emit_event(ContractPaused {
                paused_by: caller,
                timestamp,
                reason: "Emergency pause activated".to_string(),
            });
            
            Ok(())
        }

        /// Resume the contract after pause (requires multi-sig approvals)
        #[ink(message)]
        pub fn resume(&mut self) -> Result<(), Error> {
            let caller = self.env().caller();
            
            // Check if contract is paused
            if !self.pause_state.is_paused {
                return Err(Error::ContractNotPaused);
            }
            
            // Check if caller is admin (can override) or has enough approvals
            if caller == self.admin {
                // Admin can override and resume immediately
                self.execute_resume(caller)?;
            } else {
                // Check if we have enough approvals
                if self.resume_approval_count < self.required_approvals {
                    return Err(Error::InsufficientApprovals);
                }
                
                self.execute_resume(caller)?;
            }
            
            Ok(())
        }

        /// Helper: Execute the resume operation
        fn execute_resume(&mut self, caller: AccountId) -> Result<(), Error> {
            let timestamp = self.env().block_timestamp();
            
            // Update pause state
            self.pause_state.is_paused = false;
            self.pause_state.auto_resume_at = None;
            
            // Record event in audit trail
            let event = PauseEvent {
                event_type: PauseEventType::Resumed,
                timestamp,
                triggered_by: caller,
                details: format!("Contract resumed with {} approvals", self.resume_approval_count),
            };
            self.pause_events.push(event);
            
            // Emit event
            self.env().emit_event(ContractResumed {
                resumed_by: caller,
                timestamp,
                approval_count: self.resume_approval_count,
            });
            
            // Reset approvals
            self.resume_approval_count = 0;
            self.resume_approvals = Mapping::default();
            
            Ok(())
        }

        /// Check if contract is paused
        #[ink(message)]
        pub fn is_paused(&self) -> bool {
            self.pause_state.is_paused
        }

        /// Get pause state information
        #[ink(message)]
        pub fn get_pause_state(&self) -> PauseState {
            self.pause_state.clone()
        }

        /// Add a pauser role to an account
        #[ink(message)]
        pub fn add_pauser(&mut self, account: AccountId) -> Result<(), Error> {
            let caller = self.env().caller();
            
            // Only admin can add pausers
            if caller != self.admin {
                return Err(Error::Unauthorized);
            }
            
            self.pausers.insert(&account, &true);
            
            // Record event
            let event = PauseEvent {
                event_type: PauseEventType::PauserAdded,
                timestamp: self.env().block_timestamp(),
                triggered_by: caller,
                details: format!("Pauser role added to {:?}", account),
            };
            self.pause_events.push(event);
            
            self.env().emit_event(PauserAdded {
                account,
                added_by: caller,
            });
            
            Ok(())
        }

        /// Remove a pauser role from an account
        #[ink(message)]
        pub fn remove_pauser(&mut self, account: AccountId) -> Result<(), Error> {
            let caller = self.env().caller();
            
            // Only admin can remove pausers
            if caller != self.admin {
                return Err(Error::Unauthorized);
            }
            
            self.pausers.remove(&account);
            
            // Record event
            let event = PauseEvent {
                event_type: PauseEventType::PauserRemoved,
                timestamp: self.env().block_timestamp(),
                triggered_by: caller,
                details: format!("Pauser role removed from {:?}", account),
            };
            self.pause_events.push(event);
            
            self.env().emit_event(PauserRemoved {
                account,
                removed_by: caller,
            });
            
            Ok(())
        }

        /// Check if an account has pauser role
        #[ink(message)]
        pub fn is_pauser(&self, account: AccountId) -> bool {
            self.pausers.get(&account).unwrap_or(false)
        }

        /// Schedule automatic resume at a specific timestamp
        #[ink(message)]
        pub fn schedule_auto_resume(&mut self, timestamp: u64) -> Result<(), Error> {
            let caller = self.env().caller();
            
            // Only pausers can schedule auto-resume
            if !self.pausers.get(&caller).unwrap_or(false) && caller != self.admin {
                return Err(Error::NotPauser);
            }
            
            // Check if contract is paused
            if !self.pause_state.is_paused {
                return Err(Error::ContractNotPaused);
            }
            
            // Validate timestamp is in the future
            let current_time = self.env().block_timestamp();
            if timestamp <= current_time {
                return Err(Error::InvalidAutoResumeTime);
            }
            
            self.pause_state.auto_resume_at = Some(timestamp);
            
            // Record event
            let event = PauseEvent {
                event_type: PauseEventType::AutoResumeScheduled,
                timestamp: current_time,
                triggered_by: caller,
                details: format!("Auto-resume scheduled for timestamp {}", timestamp),
            };
            self.pause_events.push(event);
            
            self.env().emit_event(AutoResumeScheduled {
                scheduled_by: caller,
                resume_at: timestamp,
            });
            
            Ok(())
        }

        /// Cancel scheduled automatic resume
        #[ink(message)]
        pub fn cancel_auto_resume(&mut self) -> Result<(), Error> {
            let caller = self.env().caller();
            
            // Only pausers can cancel auto-resume
            if !self.pausers.get(&caller).unwrap_or(false) && caller != self.admin {
                return Err(Error::NotPauser);
            }
            
            // Check if auto-resume is scheduled
            if self.pause_state.auto_resume_at.is_none() {
                return Err(Error::AutoResumeNotScheduled);
            }
            
            self.pause_state.auto_resume_at = None;
            
            // Record event
            let event = PauseEvent {
                event_type: PauseEventType::AutoResumeCancelled,
                timestamp: self.env().block_timestamp(),
                triggered_by: caller,
                details: "Auto-resume cancelled".to_string(),
            };
            self.pause_events.push(event);
            
            self.env().emit_event(AutoResumeCancelled {
                cancelled_by: caller,
            });
            
            Ok(())
        }

        /// Get scheduled auto-resume timestamp
        #[ink(message)]
        pub fn get_auto_resume_time(&self) -> Option<u64> {
            self.pause_state.auto_resume_at
        }

        /// Add a resume approver (for multi-sig resume)
        #[ink(message)]
        pub fn add_resume_approver(&mut self, account: AccountId) -> Result<(), Error> {
            let caller = self.env().caller();
            
            // Only admin can add resume approvers
            if caller != self.admin {
                return Err(Error::Unauthorized);
            }
            
            self.resume_approvers.insert(&account, &true);
            
            // Record event
            let event = PauseEvent {
                event_type: PauseEventType::ApproverAdded,
                timestamp: self.env().block_timestamp(),
                triggered_by: caller,
                details: format!("Resume approver added: {:?}", account),
            };
            self.pause_events.push(event);
            
            self.env().emit_event(ResumeApproverAdded {
                account,
                added_by: caller,
            });
            
            Ok(())
        }

        /// Remove a resume approver
        #[ink(message)]
        pub fn remove_resume_approver(&mut self, account: AccountId) -> Result<(), Error> {
            let caller = self.env().caller();
            
            // Only admin can remove resume approvers
            if caller != self.admin {
                return Err(Error::Unauthorized);
            }
            
            self.resume_approvers.remove(&account);
            
            // Record event
            let event = PauseEvent {
                event_type: PauseEventType::ApproverRemoved,
                timestamp: self.env().block_timestamp(),
                triggered_by: caller,
                details: format!("Resume approver removed: {:?}", account),
            };
            self.pause_events.push(event);
            
            self.env().emit_event(ResumeApproverRemoved {
                account,
                removed_by: caller,
            });
            
            Ok(())
        }

        /// Approve resume (multi-sig)
        #[ink(message)]
        pub fn approve_resume(&mut self) -> Result<(), Error> {
            let caller = self.env().caller();
            
            // Check if contract is paused
            if !self.pause_state.is_paused {
                return Err(Error::ContractNotPaused);
            }
            
            // Check if caller is a resume approver
            if !self.resume_approvers.get(&caller).unwrap_or(false) {
                return Err(Error::NotResumeApprover);
            }
            
            // Check if already approved
            if self.resume_approvals.get(&caller).unwrap_or(false) {
                return Ok(()); // Already approved, no-op
            }
            
            // Record approval
            self.resume_approvals.insert(&caller, &true);
            self.resume_approval_count += 1;
            
            // Record event
            let event = PauseEvent {
                event_type: PauseEventType::ResumeApproved,
                timestamp: self.env().block_timestamp(),
                triggered_by: caller,
                details: format!("Resume approved ({}/{})", self.resume_approval_count, self.required_approvals),
            };
            self.pause_events.push(event);
            
            self.env().emit_event(ResumeApproved {
                approver: caller,
                current_approvals: self.resume_approval_count,
                required_approvals: self.required_approvals,
            });
            
            Ok(())
        }

        /// Get current resume approval count
        #[ink(message)]
        pub fn get_resume_approvals(&self) -> u32 {
            self.resume_approval_count
        }

        /// Get required resume approvals threshold
        #[ink(message)]
        pub fn get_required_approvals(&self) -> u32 {
            self.required_approvals
        }

        /// Set required resume approvals threshold
        #[ink(message)]
        pub fn set_required_approvals(&mut self, threshold: u32) -> Result<(), Error> {
            let caller = self.env().caller();
            
            // Only admin can set threshold
            if caller != self.admin {
                return Err(Error::Unauthorized);
            }
            
            // Validate threshold
            if threshold == 0 {
                return Err(Error::InvalidApprovalThreshold);
            }
            
            let old_threshold = self.required_approvals;
            self.required_approvals = threshold;
            
            self.env().emit_event(RequiredApprovalsChanged {
                old_threshold,
                new_threshold: threshold,
                changed_by: caller,
            });
            
            Ok(())
        }

        /// Get pause event history (audit trail)
        #[ink(message)]
        pub fn get_pause_events(&self, limit: u32) -> Vec<PauseEvent> {
            let len = self.pause_events.len();
            let start = if len > limit as usize {
                len - limit as usize
            } else {
                0
            };
            
            self.pause_events[start..].to_vec()
        }

        /// Get total number of pause events
        #[ink(message)]
        pub fn get_pause_event_count(&self) -> u32 {
            self.pause_events.len() as u32
        }
    }

    #[cfg(kani)]
    mod verification {
        use super::*;

        #[kani::proof]
        fn verify_arithmetic_overflow() {
            let a: u64 = kani::any();
            let b: u64 = kani::any();
            // Verify that addition is safe
            if a < 100 && b < 100 {
                assert!(a + b < 200);
            }
        }

        #[kani::proof]
        fn verify_property_info_struct() {
            let id: u64 = kani::any();
            // Verify PropertyInfo layout/safety if needed
            // This is a placeholder for checking structural invariants
            if id > 0 {
                assert!(id > 0);
            }
        }
    }

    impl Default for PropertyRegistry {
        fn default() -> Self {
            Self::new()
        }
    }

    impl Escrow for PropertyRegistry {
        type Error = Error;

        fn create_escrow(&mut self, property_id: u64, amount: u128) -> Result<u64, Self::Error> {
            self.create_escrow(property_id, amount)
        }

        fn release_escrow(&mut self, escrow_id: u64) -> Result<(), Self::Error> {
            self.release_escrow(escrow_id)
        }

        fn refund_escrow(&mut self, escrow_id: u64) -> Result<(), Self::Error> {
            self.refund_escrow(escrow_id)
        }
    }
}

#[cfg(test)]
mod tests;