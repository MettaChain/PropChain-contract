#![cfg_attr(not(feature = "std"), no_std)]


#[cfg(not(feature = "std"))]
use scale_info::prelude::string::String;
use ink::primitives::AccountId;

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
    fn update_metadata(&mut self, property_id: u64, metadata: PropertyMetadata) -> Result<(), Self::Error>;

    /// Approve an account to transfer a specific property
    fn approve(&mut self, property_id: u64, to: Option<AccountId>) -> Result<(), Self::Error>;

    /// Get the approved account for a property
    fn get_approved(&self, property_id: u64) -> Option<AccountId>;
}

/// Property metadata structure
#[derive(Debug, Clone, PartialEq, scale::Encode, scale::Decode, ink::storage::traits::StorageLayout)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub struct PropertyMetadata {
    pub location: String,
    pub size: u64,
    pub legal_description: String,
    pub valuation: u128,
    pub documents_url: String,
}

/// Property information structure
#[derive(Debug, Clone, PartialEq, scale::Encode, scale::Decode, ink::storage::traits::StorageLayout)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub struct PropertyInfo {
    pub id: u64,
    pub owner: AccountId,
    pub metadata: PropertyMetadata,
    pub registered_at: u64,
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
