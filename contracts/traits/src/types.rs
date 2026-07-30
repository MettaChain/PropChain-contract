//! Shared types for PropChain contracts.

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

/// A discrete type for basis points (1/100th of a percent).
///
/// This struct prevents confusion between basis points and other numerical
/// types, ensuring that fee calculations are handled correctly.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, scale::Encode, scale::Decode)]
#[cfg_attr(
    feature = "std",
    derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
)]
pub struct BasisPoints(u32);

impl BasisPoints {
    /// The denominator for basis points, representing 100%.
    pub const DENOM: u32 = 10_000;

    /// Creates a new `BasisPoints` instance from a raw `u32` value.
    pub const fn new(value: u32) -> Self {
        Self(value)
    }

    /// Returns the inner `u32` value.
    pub const fn get(self) -> u32 {
        self.0
    }

    /// Creates `BasisPoints` from a percentage.
    ///
    /// For example, `from_percent(1.5)` is 150 basis points.
    pub fn from_percent(percent: f32) -> Self {
        Self((percent * 100.0) as u32)
    }

    /// Calculates a percentage of a given amount.
    pub fn mul_floor(self, amount: u128) -> u128 {
        amount.saturating_mul(self.0 as u128) / (Self::DENOM as u128)
    }
}
