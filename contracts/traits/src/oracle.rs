//! Oracle types and trait definitions for real-time property valuation.
//!
//! This module contains all oracle-related types, error handling, and trait
//! definitions used across the PropChain ecosystem for property valuations,
//! price feeds, and market analysis.

use crate::errors::{ContractError, ErrorCategory};
use crate::property::PropertyType;
use ink::prelude::string::String;
use ink::prelude::vec::Vec;
use ink::primitives::AccountId;

// =========================================================================
// Error Types
// =========================================================================

/// Error types for the Property Valuation Oracle
#[derive(Debug, Clone, PartialEq, Eq, scale::Encode, scale::Decode)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub enum OracleError {
    /// Property not found in the oracle system
    PropertyNotFound,
    /// Insufficient oracle sources available
    InsufficientSources,
    /// Valuation data is invalid or out of range
    InvalidValuation,
    /// Caller is not authorized to perform this operation
    Unauthorized,
    /// Oracle source does not exist
    OracleSourceNotFound,
    /// Invalid parameters provided
    InvalidParameters,
    /// Error from external price feed
    PriceFeedError,
    /// Price alert not found
    AlertNotFound,
    /// Oracle source has insufficient reputation
    InsufficientReputation,
    /// Oracle source already registered
    SourceAlreadyExists,
    /// Valuation request is still pending
    RequestPending,
    /// Input batch exceeds the configured maximum size
    BatchSizeExceeded,
    /// Circuit breaker is active; transfers are paused due to extreme volatility
    CircuitBreakerActive,
    /// The operation has already been performed (e.g. double-approval)
    AlreadyExists,
}

impl core::fmt::Display for OracleError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            OracleError::PropertyNotFound => write!(f, "Property not found in the oracle system"),
            OracleError::InsufficientSources => write!(f, "Insufficient oracle sources available"),
            OracleError::InvalidValuation => write!(f, "Valuation data is invalid or out of range"),
            OracleError::Unauthorized => {
                write!(f, "Caller is not authorized to perform this operation")
            }
            OracleError::OracleSourceNotFound => write!(f, "Oracle source does not exist"),
            OracleError::InvalidParameters => write!(f, "Invalid parameters provided"),
            OracleError::PriceFeedError => write!(f, "Error from external price feed"),
            OracleError::AlertNotFound => write!(f, "Price alert not found"),
            OracleError::InsufficientReputation => {
                write!(f, "Oracle source has insufficient reputation")
            }
            OracleError::SourceAlreadyExists => write!(f, "Oracle source already registered"),
            OracleError::RequestPending => write!(f, "Valuation request is still pending"),
            OracleError::BatchSizeExceeded => write!(f, "Batch size exceeds maximum allowed"),
            OracleError::CircuitBreakerActive => {
                write!(
                    f,
                    "Circuit breaker is active; transfers paused due to extreme price volatility"
                )
            }
            OracleError::AlreadyExists => write!(f, "Operation already performed"),
        }
    }
}

impl ContractError for OracleError {
    fn error_code(&self) -> u32 {
        use crate::errors::oracle_codes;
        match self {
            OracleError::PropertyNotFound => oracle_codes::ORACLE_PROPERTY_NOT_FOUND,
            OracleError::InsufficientSources => oracle_codes::ORACLE_INSUFFICIENT_SOURCES,
            OracleError::InvalidValuation => oracle_codes::ORACLE_INVALID_VALUATION,
            OracleError::Unauthorized => oracle_codes::ORACLE_UNAUTHORIZED,
            OracleError::OracleSourceNotFound => oracle_codes::ORACLE_SOURCE_NOT_FOUND,
            OracleError::InvalidParameters => oracle_codes::ORACLE_INVALID_PARAMETERS,
            OracleError::PriceFeedError => oracle_codes::ORACLE_PRICE_FEED_ERROR,
            OracleError::AlertNotFound => oracle_codes::ORACLE_ALERT_NOT_FOUND,
            OracleError::InsufficientReputation => oracle_codes::ORACLE_INSUFFICIENT_REPUTATION,
            OracleError::SourceAlreadyExists => oracle_codes::ORACLE_SOURCE_ALREADY_EXISTS,
            OracleError::RequestPending => oracle_codes::ORACLE_REQUEST_PENDING,
            OracleError::BatchSizeExceeded => oracle_codes::ORACLE_BATCH_SIZE_EXCEEDED,
            OracleError::CircuitBreakerActive => 1013,
            OracleError::AlreadyExists => 1014,
        }
    }

    fn error_description(&self) -> &'static str {
        match self {
            OracleError::PropertyNotFound => {
                "The requested property does not exist in the oracle system"
            }
            OracleError::InsufficientSources => {
                "Not enough oracle sources are available to provide a reliable valuation"
            }
            OracleError::InvalidValuation => {
                "The valuation data is invalid, zero, or out of acceptable range"
            }
            OracleError::Unauthorized => {
                "Caller does not have permission to perform this operation"
            }
            OracleError::OracleSourceNotFound => "The specified oracle source does not exist",
            OracleError::InvalidParameters => "One or more function parameters are invalid",
            OracleError::PriceFeedError => "Failed to retrieve data from external price feed",
            OracleError::AlertNotFound => "The requested price alert does not exist",
            OracleError::InsufficientReputation => {
                "Oracle source reputation is below required threshold"
            }
            OracleError::SourceAlreadyExists => {
                "An oracle source with this identifier already exists"
            }
            OracleError::RequestPending => {
                "A valuation request for this property is already pending"
            }
            OracleError::BatchSizeExceeded => {
                "The number of requested items exceeds the configured batch limit"
            }
            OracleError::CircuitBreakerActive => {
                "Circuit breaker is active; all transfers are paused due to extreme price volatility"
            }
            OracleError::AlreadyExists => "This operation has already been performed",
        }
    }

    fn error_category(&self) -> ErrorCategory {
        ErrorCategory::Oracle
    }

    fn error_i18n_key(&self) -> &'static str {
        match self {
            OracleError::PropertyNotFound => "oracle.property_not_found",
            OracleError::InsufficientSources => "oracle.insufficient_sources",
            OracleError::InvalidValuation => "oracle.invalid_valuation",
            OracleError::Unauthorized => "oracle.unauthorized",
            OracleError::OracleSourceNotFound => "oracle.source_not_found",
            OracleError::InvalidParameters => "oracle.invalid_parameters",
            OracleError::PriceFeedError => "oracle.price_feed_error",
            OracleError::AlertNotFound => "oracle.alert_not_found",
            OracleError::InsufficientReputation => "oracle.insufficient_reputation",
            OracleError::SourceAlreadyExists => "oracle.source_already_exists",
            OracleError::RequestPending => "oracle.request_pending",
            OracleError::BatchSizeExceeded => "oracle.batch_size_exceeded",
            OracleError::CircuitBreakerActive => "oracle.circuit_breaker_active",
            OracleError::AlreadyExists => "oracle.already_exists",
        }
    }
}

// =========================================================================
// Data Types
// =========================================================================

/// Price data from external feeds
#[derive(Debug, Clone, PartialEq, scale::Encode, scale::Decode)]
#[cfg_attr(
    feature = "std",
    derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
)]
pub struct PriceData {
    pub price: u128,    // Price in USD with 8 decimals
    pub timestamp: u64, // Timestamp when price was recorded
    pub source: String, // Price feed source identifier
}

/// Property valuation structure
#[derive(Debug, Clone, PartialEq, scale::Encode, scale::Decode)]
#[cfg_attr(
    feature = "std",
    derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
)]
pub struct PropertyValuation {
    pub property_id: u64,
    pub valuation: u128,       // Current valuation in USD with 8 decimals
    pub confidence_score: u32, // Confidence score 0-100
    pub sources_used: u32,     // Number of price sources used
    pub last_updated: u64,     // Last update timestamp
    pub valuation_method: ValuationMethod,
}

/// Valuation method enumeration
#[derive(Debug, Clone, PartialEq, Eq, scale::Encode, scale::Decode)]
#[cfg_attr(
    feature = "std",
    derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
)]
pub enum ValuationMethod {
    Automated,   // AVM (Automated Valuation Model)
    Manual,      // Manual appraisal
    MarketData,  // Based on market comparables
    Hybrid,      // Combination of methods
    AIValuation, // AI-powered machine learning valuation
}

/// Valuation with confidence metrics
#[derive(Debug, Clone, PartialEq, scale::Encode, scale::Decode)]
#[cfg_attr(
    feature = "std",
    derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
)]
pub struct ValuationWithConfidence {
    pub valuation: PropertyValuation,
    pub volatility_index: u32,             // Market volatility 0-100
    pub confidence_interval: (u128, u128), // Min and max valuation range
    pub outlier_sources: u32,              // Number of outlier sources detected
}

/// Volatility metrics for market analysis
#[derive(Debug, Clone, PartialEq, scale::Encode, scale::Decode)]
#[cfg_attr(
    feature = "std",
    derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
)]
pub struct VolatilityMetrics {
    pub property_type: PropertyType,
    pub location: String,
    pub volatility_index: u32,     // 0-100 scale
    pub average_price_change: i32, // Average % change over period (can be negative)
    pub period_days: u32,          // Analysis period in days
    pub last_updated: u64,
}

/// Comparable property for AVM analysis
#[derive(Debug, Clone, PartialEq, scale::Encode, scale::Decode)]
#[cfg_attr(
    feature = "std",
    derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
)]
pub struct ComparableProperty {
    pub property_id: u64,
    pub distance_km: u32,       // Distance from subject property
    pub price_per_sqm: u128,    // Price per square meter
    pub size_sqm: u64,          // Property size in square meters
    pub sale_date: u64,         // When it was sold
    pub adjustment_factor: i32, // Adjustment factor (+/- percentage)
}

/// Price alert configuration
#[derive(Debug, Clone, PartialEq, scale::Encode, scale::Decode)]
#[cfg_attr(
    feature = "std",
    derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
)]
pub struct PriceAlert {
    pub property_id: u64,
    pub threshold_percentage: u32, // Alert threshold (e.g., 5 for 5%)
    pub alert_address: AccountId,  // Address to notify
    pub last_triggered: u64,       // Last time alert was triggered
    pub is_active: bool,
}

/// Oracle source configuration
#[derive(Debug, Clone, PartialEq, scale::Encode, scale::Decode)]
#[cfg_attr(
    feature = "std",
    derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
)]
pub struct OracleSource {
    pub id: String, // Unique source identifier
    pub source_type: OracleSourceType,
    pub address: AccountId, // Contract address for the price feed
    pub is_active: bool,
    pub weight: u32, // Weight in aggregation (0-100)
    pub last_updated: u64,
}

/// Oracle source type enumeration
#[derive(Debug, Clone, PartialEq, Eq, scale::Encode, scale::Decode)]
#[cfg_attr(
    feature = "std",
    derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
)]
pub enum OracleSourceType {
    Chainlink,
    Pyth,
    Substrate,
    Custom,
    Manual,
    AIModel, // AI-powered valuation model
}

/// Location-based adjustment factors
#[derive(Debug, Clone, PartialEq, scale::Encode, scale::Decode)]
#[cfg_attr(
    feature = "std",
    derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
)]
pub struct LocationAdjustment {
    pub location_code: String,      // Geographic location identifier
    pub adjustment_percentage: i32, // Adjustment factor (+/- percentage)
    pub last_updated: u64,
    pub confidence_score: u32,
}

/// Market trend data
#[derive(Debug, Clone, PartialEq, scale::Encode, scale::Decode)]
#[cfg_attr(
    feature = "std",
    derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
)]
pub struct MarketTrend {
    pub property_type: PropertyType,
    pub location: String,
    pub trend_percentage: i32, // Trend direction and magnitude
    pub period_months: u32,    // Analysis period in months
    pub last_updated: u64,
}

// =========================================================================
// Trait Definitions
// =========================================================================

/// Oracle trait for real-time property valuation
#[ink::trait_definition]
pub trait Oracle {
    /// Get current property valuation
    #[ink(message)]
    fn get_valuation(&self, property_id: u64) -> Result<PropertyValuation, OracleError>;

    /// Get valuation with detailed confidence metrics
    #[ink(message)]
    fn get_valuation_with_confidence(
        &self,
        property_id: u64,
    ) -> Result<ValuationWithConfidence, OracleError>;

    /// Request a new valuation for a property (async pattern)
    #[ink(message)]
    fn request_valuation(&mut self, property_id: u64) -> Result<u64, OracleError>;

    /// Batch request valuations for multiple properties
    #[ink(message)]
    fn batch_request_valuations(&mut self, property_ids: Vec<u64>)
        -> Result<Vec<u64>, OracleError>;

    /// Get historical valuations for a property
    #[ink(message)]
    fn get_historical_valuations(&self, property_id: u64, limit: u32) -> Vec<PropertyValuation>;

    /// Get market volatility for a specific location and property type
    #[ink(message)]
    fn get_market_volatility(
        &self,
        property_type: PropertyType,
        location: String,
    ) -> Result<VolatilityMetrics, OracleError>;
}

/// Oracle Registry trait for managing multiple price feeds and reputation
#[ink::trait_definition]
pub trait OracleRegistry {
    /// Register a new oracle source
    #[ink(message)]
    fn add_source(&mut self, source: OracleSource) -> Result<(), OracleError>;

    /// Remove an oracle source
    #[ink(message)]
    fn remove_source(&mut self, source_id: String) -> Result<(), OracleError>;

    /// Update oracle source reputation based on performance
    #[ink(message)]
    fn update_reputation(&mut self, source_id: String, success: bool) -> Result<(), OracleError>;

    /// Get oracle source reputation score
    #[ink(message)]
    fn get_reputation(&self, source_id: String) -> Option<u32>;

    /// Slash oracle source for providing invalid data
    #[ink(message)]
    fn slash_source(&mut self, source_id: String, penalty_amount: u128) -> Result<(), OracleError>;

    /// Check for anomalies in price data
    #[ink(message)]
    fn detect_anomalies(&self, property_id: u64, new_valuation: u128) -> bool;
}
