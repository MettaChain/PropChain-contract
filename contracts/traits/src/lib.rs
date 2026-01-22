#![cfg_attr(not(feature = "std"), no_std)]

use ink::prelude::*;
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

/// Property type enumeration
#[derive(Debug, Clone, PartialEq, scale::Encode, scale::Decode, ink::storage::traits::StorageLayout)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
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
#[derive(Debug, Clone, PartialEq, scale::Encode, scale::Decode, ink::storage::traits::StorageLayout)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub struct PriceData {
    pub price: u128,      // Price in USD with 8 decimals
    pub timestamp: u64,   // Timestamp when price was recorded
    pub source: String,   // Price feed source identifier
}

/// Property valuation structure
#[derive(Debug, Clone, PartialEq, scale::Encode, scale::Decode, ink::storage::traits::StorageLayout)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub struct PropertyValuation {
    pub property_id: u64,
    pub valuation: u128,              // Current valuation in USD with 8 decimals
    pub confidence_score: u32,        // Confidence score 0-100
    pub sources_used: u32,           // Number of price sources used
    pub last_updated: u64,           // Last update timestamp
    pub valuation_method: ValuationMethod,
}

/// Valuation method enumeration
#[derive(Debug, Clone, PartialEq, scale::Encode, scale::Decode, ink::storage::traits::StorageLayout)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub enum ValuationMethod {
    Automated,      // AVM (Automated Valuation Model)
    Manual,         // Manual appraisal
    MarketData,     // Based on market comparables
    Hybrid,         // Combination of methods
}

/// Valuation with confidence metrics
#[derive(Debug, Clone, PartialEq, scale::Encode, scale::Decode, ink::storage::traits::StorageLayout)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub struct ValuationWithConfidence {
    pub valuation: PropertyValuation,
    pub volatility_index: u32,        // Market volatility 0-100
    pub confidence_interval: (u128, u128), // Min and max valuation range
    pub outlier_sources: u32,         // Number of outlier sources detected
}

/// Volatility metrics for market analysis
#[derive(Debug, Clone, PartialEq, scale::Encode, scale::Decode, ink::storage::traits::StorageLayout)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub struct VolatilityMetrics {
    pub property_type: PropertyType,
    pub location: String,
    pub volatility_index: u32,        // 0-100 scale
    pub average_price_change: i32,    // Average % change over period (can be negative)
    pub period_days: u32,            // Analysis period in days
    pub last_updated: u64,
}

/// Comparable property for AVM analysis
#[derive(Debug, Clone, PartialEq, scale::Encode, scale::Decode, ink::storage::traits::StorageLayout)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub struct ComparableProperty {
    pub property_id: u64,
    pub distance_km: u32,            // Distance from subject property
    pub price_per_sqm: u128,         // Price per square meter
    pub size_sqm: u64,              // Property size in square meters
    pub sale_date: u64,             // When it was sold
    pub adjustment_factor: i32,     // Adjustment factor (+/- percentage)
}

/// Price alert configuration
#[derive(Debug, Clone, PartialEq, scale::Encode, scale::Decode, ink::storage::traits::StorageLayout)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub struct PriceAlert {
    pub property_id: u64,
    pub threshold_percentage: u32,   // Alert threshold (e.g., 5 for 5%)
    pub alert_address: AccountId,    // Address to notify
    pub last_triggered: u64,         // Last time alert was triggered
    pub is_active: bool,
}

/// Oracle source configuration
#[derive(Debug, Clone, PartialEq, scale::Encode, scale::Decode, ink::storage::traits::StorageLayout)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub struct OracleSource {
    pub id: String,                 // Unique source identifier
    pub source_type: OracleSourceType,
    pub address: AccountId,         // Contract address for the price feed
    pub is_active: bool,
    pub weight: u32,                // Weight in aggregation (0-100)
    pub last_updated: u64,
}

/// Oracle source type enumeration
#[derive(Debug, Clone, PartialEq, scale::Encode, scale::Decode, ink::storage::traits::StorageLayout)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub enum OracleSourceType {
    Chainlink,
    Pyth,
    Custom,
    Manual,
}

/// Location-based adjustment factors
#[derive(Debug, Clone, PartialEq, scale::Encode, scale::Decode, ink::storage::traits::StorageLayout)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub struct LocationAdjustment {
    pub location_code: String,      // Geographic location identifier
    pub adjustment_percentage: i32, // Adjustment factor (+/- percentage)
    pub last_updated: u64,
    pub confidence_score: u32,
}

/// Market trend data
#[derive(Debug, Clone, PartialEq, scale::Encode, scale::Decode, ink::storage::traits::StorageLayout)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
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

/// Property Valuation Oracle trait
pub trait PropertyValuationOracle {
    /// Error type for oracle operations
    type Error;

    /// Get property valuation from multiple sources
    fn get_property_valuation(&self, property_id: u64) -> Result<PropertyValuation, Self::Error>;

    /// Get property valuation with confidence score
    fn get_valuation_with_confidence(&self, property_id: u64) -> Result<ValuationWithConfidence, Self::Error>;

    /// Update property valuation manually (admin only)
    fn update_property_valuation(&mut self, property_id: u64, valuation: PropertyValuation) -> Result<(), Self::Error>;

    /// Get historical valuations for a property
    fn get_historical_valuations(&self, property_id: u64, limit: u32) -> Vec<PropertyValuation>;

    /// Get market volatility for a property type/location
    fn get_market_volatility(&self, property_type: PropertyType, location: String) -> Result<VolatilityMetrics, Self::Error>;

    /// Set alert thresholds for price changes
    fn set_price_alert(&mut self, property_id: u64, threshold_percentage: u32, alert_address: AccountId) -> Result<(), Self::Error>;

    /// Get comparable properties for AVM
    fn get_comparable_properties(&self, property_id: u64, radius_km: u32) -> Vec<ComparableProperty>;
}

/// Price Feed trait for external price sources
pub trait PriceFeed {
    /// Error type for price feed operations
    type Error;

    /// Get latest price from this feed
    fn get_latest_price(&self, asset_id: String) -> Result<PriceData, Self::Error>;

    /// Get price at specific timestamp
    fn get_price_at(&self, asset_id: String, timestamp: u64) -> Result<PriceData, Self::Error>;

    /// Get price history for time range
    fn get_price_history(&self, asset_id: String, from_timestamp: u64, to_timestamp: u64) -> Vec<PriceData>;
}
