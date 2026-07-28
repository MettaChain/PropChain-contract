//! Dynamic fee and market mechanism types and traits.
//!
//! This module contains operation types for dynamic fee calculation
//! and the trait definition for fee providers.

use crate::types::FeeOperation;

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
