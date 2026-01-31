#![cfg_attr(not(feature = "std"), no_std)]

use ink::prelude::string::String;
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
#[derive(Debug, Clone, PartialEq, scale::Encode, scale::Decode)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout))]
pub struct PropertyMetadata {
    pub location: String,
    pub size: u64,
    pub legal_description: String,
    pub valuation: u128,
    pub documents_url: String,
}

/// Property information structure
#[derive(Debug, Clone, PartialEq, scale::Encode, scale::Decode)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout))]
pub struct PropertyInfo {
    pub id: u64,
    pub owner: AccountId,
    pub metadata: PropertyMetadata,
    pub registered_at: u64,
}

/// Property type enumeration
#[derive(Debug, Clone, PartialEq, scale::Encode, scale::Decode)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout))]
pub enum PropertyType {
    Residential,
    Commercial,
    Industrial,
    Land,
    MultiFamily,
    Retail,
    Office,
}

/// Price data from external feeds
#[derive(Debug, Clone, PartialEq, scale::Encode, scale::Decode)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout))]
pub struct PriceData {
    pub price: u128,      // Price in USD with 8 decimals
    pub timestamp: u64,   // Timestamp when price was recorded
    pub source: String,   // Price feed source identifier
}

/// Property valuation structure
#[derive(Debug, Clone, PartialEq, scale::Encode, scale::Decode)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout))]
pub struct PropertyValuation {
    pub property_id: u64,
    pub valuation: u128,              // Current valuation in USD with 8 decimals
    pub confidence_score: u32,        // Confidence score 0-100
    pub sources_used: u32,           // Number of price sources used
    pub last_updated: u64,           // Last update timestamp
    pub valuation_method: ValuationMethod,
}

/// Valuation method enumeration
#[derive(Debug, Clone, PartialEq, scale::Encode, scale::Decode)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout))]
pub enum ValuationMethod {
    Automated,      // AVM (Automated Valuation Model)
    Manual,         // Manual appraisal
    MarketData,     // Based on market comparables
    Hybrid,         // Combination of methods
}

/// Valuation with confidence metrics
#[derive(Debug, Clone, PartialEq, scale::Encode, scale::Decode)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout))]
pub struct ValuationWithConfidence {
    pub valuation: PropertyValuation,
    pub volatility_index: u32,        // Market volatility 0-100
    pub confidence_interval: (u128, u128), // Min and max valuation range
    pub outlier_sources: u32,         // Number of outlier sources detected
}

/// Volatility metrics for market analysis
#[derive(Debug, Clone, PartialEq, scale::Encode, scale::Decode)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout))]
pub struct VolatilityMetrics {
    pub property_type: PropertyType,
    pub location: String,
    pub volatility_index: u32,        // 0-100 scale
    pub average_price_change: i32,    // Average % change over period (can be negative)
    pub period_days: u32,            // Analysis period in days
    pub last_updated: u64,
}

/// Comparable property for AVM analysis
#[derive(Debug, Clone, PartialEq, scale::Encode, scale::Decode)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout))]
pub struct ComparableProperty {
    pub property_id: u64,
    pub distance_km: u32,            // Distance from subject property
    pub price_per_sqm: u128,         // Price per square meter
    pub size_sqm: u64,              // Property size in square meters
    pub sale_date: u64,             // When it was sold
    pub adjustment_factor: i32,     // Adjustment factor (+/- percentage)
}

/// Price alert configuration
#[derive(Debug, Clone, PartialEq, scale::Encode, scale::Decode)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout))]
pub struct PriceAlert {
    pub property_id: u64,
    pub threshold_percentage: u32,   // Alert threshold (e.g., 5 for 5%)
    pub alert_address: AccountId,    // Address to notify
    pub last_triggered: u64,         // Last time alert was triggered
    pub is_active: bool,
}

/// Oracle source configuration
#[derive(Debug, Clone, PartialEq, scale::Encode, scale::Decode)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout))]
pub struct OracleSource {
    pub id: String,                 // Unique source identifier
    pub source_type: OracleSourceType,
    pub address: AccountId,         // Contract address for the price feed
    pub is_active: bool,
    pub weight: u32,                // Weight in aggregation (0-100)
    pub last_updated: u64,
}

/// Oracle source type enumeration
#[derive(Debug, Clone, PartialEq, scale::Encode, scale::Decode)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout))]
pub enum OracleSourceType {
    Chainlink,
    Pyth,
    Custom,
    Manual,
}

/// Location-based adjustment factors
#[derive(Debug, Clone, PartialEq, scale::Encode, scale::Decode)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout))]
pub struct LocationAdjustment {
    pub location_code: String,      // Geographic location identifier
    pub adjustment_percentage: i32, // Adjustment factor (+/- percentage)
    pub last_updated: u64,
    pub confidence_score: u32,
}

/// Market trend data
#[derive(Debug, Clone, PartialEq, scale::Encode, scale::Decode)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout))]
pub struct MarketTrend {
    pub property_type: PropertyType,
    pub location: String,
    pub trend_percentage: i32,      // Trend direction and magnitude
    pub period_months: u32,         // Analysis period in months
    pub last_updated: u64,
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

#[cfg(not(feature = "std"))]
use scale_info::prelude::vec::Vec;

/// Advanced escrow trait with multi-signature and document custody
pub trait AdvancedEscrow {
    /// Error type for escrow operations
    type Error;

    /// Create an advanced escrow with multi-signature support
    fn create_escrow_advanced(
        &mut self,
        property_id: u64,
        amount: u128,
        buyer: AccountId,
        seller: AccountId,
        participants: Vec<AccountId>,
        required_signatures: u8,
        release_time_lock: Option<u64>,
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
    fn sign_approval(&mut self, escrow_id: u64, approval_type: ApprovalType) -> Result<(), Self::Error>;

    /// Raise a dispute
    fn raise_dispute(&mut self, escrow_id: u64, reason: String) -> Result<(), Self::Error>;

    /// Resolve a dispute (admin only)
    fn resolve_dispute(&mut self, escrow_id: u64, resolution: String) -> Result<(), Self::Error>;

    /// Emergency override (admin only)
    fn emergency_override(&mut self, escrow_id: u64, release_to_seller: bool) -> Result<(), Self::Error>;
}

/// Approval type for multi-signature operations
#[derive(Debug, Clone, PartialEq, Eq, scale::Encode, scale::Decode)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub enum ApprovalType {
    Release,
    Refund,
    EmergencyOverride,
}
