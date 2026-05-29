//! Dynamic fee and market mechanism types and traits.
//!
//! This module contains operation types for dynamic fee calculation
//! and the trait definition for fee providers.

// =========================================================================
// Data Types
// =========================================================================

/// Operation types for dynamic fee calculation
#[derive(Debug, Clone, Copy, PartialEq, Eq, scale::Encode, scale::Decode)]
#[cfg_attr(
    feature = "std",
    derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
)]
pub enum FeeOperation {
    RegisterProperty,
    TransferProperty,
    UpdateMetadata,
    CreateEscrow,
    ReleaseEscrow,
    PremiumListingBid,
    IssueBadge,
    OracleUpdate,
}

// =========================================================================
// Trait Definitions
// =========================================================================

/// Trait for dynamic fee provider (implemented by fee manager contract)
#[ink::trait_definition]
pub trait DynamicFeeProvider {
    /// Get recommended fee for an operation (market-based price discovery)
    #[ink(message)]
    fn get_recommended_fee(&self, operation: FeeOperation) -> u128;
}
