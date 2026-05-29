//! Property types and trait definitions for the PropChain registry.
//!
//! This module contains the core property-related types, metadata structures,
//! and trait definitions for property registration, escrow, and management.

use ink::prelude::string::String;
use ink::prelude::vec::Vec;
use ink::primitives::AccountId;
use crate::compliance::Jurisdiction;

// =========================================================================
// Data Types
// =========================================================================

/// Property metadata structure
#[derive(Debug, Clone, PartialEq, scale::Encode, scale::Decode)]
#[cfg_attr(
    feature = "std",
    derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
)]
pub struct PropertyMetadata {
    pub location: String,
    pub size: u64,
    pub legal_description: String,
    pub valuation: u128,
    pub documents_url: String,
}

/// Property information structure
#[derive(Debug, Clone, PartialEq, scale::Encode, scale::Decode)]
#[cfg_attr(
    feature = "std",
    derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
)]
pub struct PropertyInfo {
    pub id: u64,
    pub owner: AccountId,
    pub metadata: PropertyMetadata,
    pub registered_at: u64,
}

/// Property type enumeration
#[derive(Debug, Clone, PartialEq, scale::Encode, scale::Decode)]
#[cfg_attr(
    feature = "std",
    derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
)]
pub enum PropertyType {
    Residential,
    Commercial,
    Industrial,
    Land,
    MultiFamily,
    Retail,
    Office,
}

/// Approval type for multi-signature operations
#[derive(Debug, Clone, PartialEq, Eq, scale::Encode, scale::Decode)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub enum ApprovalType {
    Release,
    Refund,
    EmergencyOverride,
}

/// Chain ID type for cross-chain operations
pub type ChainId = u64;

/// Token ID type for property tokens
pub type TokenId = u64;

// =========================================================================
// Trait Definitions
// =========================================================================

/// Trait definitions for PropChain contracts
pub trait PropertyRegistry {
    /// Error type for the contract
    type Error;

    /// Register a new property
    fn register_property(&mut self, metadata: PropertyMetadata) -> Result<u64, Self::Error>;

    /// Transfer property ownership
    fn transfer_property(&mut self, property_id: u64, to: AccountId) -> Result<(), Self::Error>;

    /// Get property information
    fn get_property(&self, property_id: u64) -> Option<PropertyInfo>;

    /// Update property metadata
    fn update_metadata(
        &mut self,
        property_id: u64,
        metadata: PropertyMetadata,
    ) -> Result<(), Self::Error>;

    /// Approve an account to transfer a specific property
    fn approve(&mut self, property_id: u64, to: Option<AccountId>) -> Result<(), Self::Error>;

    /// Get the approved account for a property
    fn get_approved(&self, property_id: u64) -> Option<AccountId>;
}

/// Escrow trait for secure property transfers
pub trait Escrow {
    /// Error type for escrow operations
    type Error;

    /// Create a new escrow
    fn create_escrow(&mut self, property_id: u64, amount: u128) -> Result<u64, Self::Error>;

    /// Release escrow funds
    fn release_escrow(&mut self, escrow_id: u64) -> Result<(), Self::Error>;

    /// Refund escrow funds
    fn refund_escrow(&mut self, escrow_id: u64) -> Result<(), Self::Error>;
}

/// Advanced escrow trait with multi-signature and document custody
pub trait AdvancedEscrow {
    /// Error type for escrow operations
    type Error;

    /// Create an advanced escrow with multi-signature support
    #[allow(clippy::too_many_arguments)]
    fn create_escrow_advanced(
        &mut self,
        property_id: u64,
        amount: u128,
        buyer: AccountId,
        seller: AccountId,
        participants: Vec<AccountId>,
        required_signatures: u8,
        release_time_lock: Option<u64>,
        jurisdiction: Jurisdiction,
    ) -> Result<u64, Self::Error>;

    /// Deposit funds to escrow
    fn deposit_funds(&mut self, escrow_id: u64) -> Result<(), Self::Error>;

    /// Release funds with multi-signature approval
    fn release_funds(&mut self, escrow_id: u64) -> Result<(), Self::Error>;

    /// Refund funds with multi-signature approval
    fn refund_funds(&mut self, escrow_id: u64) -> Result<(), Self::Error>;

    /// Upload document hash to escrow
    fn upload_document(
        &mut self,
        escrow_id: u64,
        document_hash: ink::primitives::Hash,
        document_type: String,
    ) -> Result<(), Self::Error>;

    /// Verify a document
    fn verify_document(
        &mut self,
        escrow_id: u64,
        document_hash: ink::primitives::Hash,
    ) -> Result<(), Self::Error>;

    /// Add a condition to the escrow
    fn add_condition(&mut self, escrow_id: u64, description: String) -> Result<u64, Self::Error>;

    /// Mark a condition as met
    fn mark_condition_met(&mut self, escrow_id: u64, condition_id: u64) -> Result<(), Self::Error>;

    /// Sign approval for release or refund
    fn sign_approval(
        &mut self,
        escrow_id: u64,
        approval_type: ApprovalType,
    ) -> Result<(), Self::Error>;

    /// Raise a dispute
    fn raise_dispute(&mut self, escrow_id: u64, reason: String) -> Result<(), Self::Error>;

    /// Resolve a dispute (admin only)
    fn resolve_dispute(&mut self, escrow_id: u64, resolution: String) -> Result<(), Self::Error>;

    /// Emergency override (admin only)
    fn emergency_override(
        &mut self,
        escrow_id: u64,
        release_to_seller: bool,
    ) -> Result<(), Self::Error>;
}
