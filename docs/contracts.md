# Contract API Documentation

## Overview

PropChain smart contracts provide a comprehensive API for real estate tokenization and management on the blockchain. This document outlines the complete contract interface, methods, and data structures.

## Core Contracts

### PropertyRegistry

Manages property registration, ownership tracking, and metadata storage.

#### Methods

##### `new()`
Creates a new PropertyRegistry instance.

```rust
#[ink(constructor)]
pub fn new() -> Self
```

##### `register_property(metadata: PropertyMetadata) -> Result<PropertyId, Error>`
Registers a new property in the registry.

**Parameters:**
- `metadata`: Property metadata including location, size, and legal details

**Returns:**
- `PropertyId`: Unique identifier for the registered property
- `Error`: Registration error if property already exists or invalid data

##### `transfer_property(property_id: PropertyId, to: AccountId) -> Result<(), Error>`
Transfers ownership of a property to another account.

**Parameters:**
- `property_id`: ID of the property to transfer
- `to`: Recipient account address

##### `get_property(property_id: PropertyId) -> Option<PropertyInfo>`
Retrieves property information by ID.

**Parameters:**
- `property_id`: ID of the property to query

**Returns:**
- `PropertyInfo`: Property details if found, `None` otherwise

##### `update_metadata(property_id: PropertyId, metadata: PropertyMetadata) -> Result<(), Error>`
Updates the metadata for a registered property.

**Parameters:**
- `property_id`: ID of the property to update
- `metadata`: New property metadata

##### `approve(property_id: PropertyId, to: Option<AccountId>) -> Result<(), Error>`
Approves an account to transfer a specific property.

**Parameters:**
- `property_id`: ID of the property
- `to`: Account to approve (Some) or remove approval (None)

##### `get_approved(property_id: PropertyId) -> Option<AccountId>`
Gets the approved account for a property.

**Parameters:**
- `property_id`: ID of the property

**Returns:**
- `Option<AccountId>`: Approved account if any

### EscrowContract

Handles secure property transfers with escrow protection.

#### Methods

##### `create_escrow(property_id: PropertyId, buyer: AccountId, amount: Balance) -> Result<EscrowId, Error>`
Creates a new escrow for property transfer.

##### `release_escrow(escrow_id: EscrowId) -> Result<(), Error>`
Releases escrow funds to the seller.

##### `refund_escrow(escrow_id: EscrowId) -> Result<(), Error>`
Refunds escrow funds to the buyer.

### PropertyValuationOracle

Provides real-time property valuations using multiple oracle sources with aggregation and confidence scoring.

#### Methods

##### `get_property_valuation(property_id: PropertyId) -> Result<PropertyValuation, OracleError>`
Gets the current property valuation from aggregated oracle sources.

##### `get_valuation_with_confidence(property_id: PropertyId) -> Result<ValuationWithConfidence, OracleError>`
Gets property valuation with confidence metrics including volatility and confidence intervals.

##### `update_valuation_from_sources(property_id: PropertyId) -> Result<(), OracleError>`
Updates property valuation by aggregating prices from all active oracle sources.

##### `get_historical_valuations(property_id: PropertyId, limit: u32) -> Vec<PropertyValuation>`
Retrieves historical valuations for a property (most recent first).

##### `get_market_volatility(property_type: PropertyType, location: String) -> Result<VolatilityMetrics, OracleError>`
Gets market volatility metrics for specific property types and locations.

##### `set_price_alert(property_id: PropertyId, threshold_percentage: u32, alert_address: AccountId) -> Result<(), OracleError>`
Sets up price change alerts for property valuation monitoring.

##### `add_oracle_source(source: OracleSource) -> Result<(), OracleError>` (Admin only)
Adds a new oracle source for price feeds (Chainlink, Pyth, Custom).

##### `get_comparable_properties(property_id: PropertyId, radius_km: u32) -> Vec<ComparableProperty>`
Gets comparable properties within a specified radius for AVM analysis.

## Data Structures

### PropertyMetadata
```rust
pub struct PropertyMetadata {
    pub location: String,
    pub size: u64,
    pub legal_description: String,
    pub valuation: Balance,
    pub documents_url: String,
}
```

### PropertyInfo
```rust
pub struct PropertyInfo {
    pub id: PropertyId,
    pub owner: AccountId,
    pub metadata: PropertyMetadata,
    pub registered_at: Timestamp,
}
```

### EscrowInfo
```rust
pub struct EscrowInfo {
    pub id: EscrowId,
    pub property_id: PropertyId,
    pub seller: AccountId,
    pub buyer: AccountId,
    pub amount: Balance,
    pub status: EscrowStatus,
    pub created_at: Timestamp,
}
```

### Property Valuation Structures

#### PropertyValuation
```rust
pub struct PropertyValuation {
    pub property_id: u64,
    pub valuation: u128,              // Current valuation in USD with 8 decimals
    pub confidence_score: u32,        // Confidence score 0-100
    pub sources_used: u32,           // Number of price sources used
    pub last_updated: u64,           // Last update timestamp
    pub valuation_method: ValuationMethod,
}
```

#### ValuationWithConfidence
```rust
pub struct ValuationWithConfidence {
    pub valuation: PropertyValuation,
    pub volatility_index: u32,        // Market volatility 0-100
    pub confidence_interval: (u128, u128), // Min and max valuation range
    pub outlier_sources: u32,         // Number of outlier sources detected
}
```

#### PriceData
```rust
pub struct PriceData {
    pub price: u128,      // Price in USD with 8 decimals
    pub timestamp: u64,   // Timestamp when price was recorded
    pub source: String,   // Price feed source identifier
}
```

#### VolatilityMetrics
```rust
pub struct VolatilityMetrics {
    pub property_type: PropertyType,
    pub location: String,
    pub volatility_index: u32,        // 0-100 scale
    pub average_price_change: i32,    // Average % change over period
    pub period_days: u32,            // Analysis period in days
    pub last_updated: u64,
}
```

#### ComparableProperty
```rust
pub struct ComparableProperty {
    pub property_id: u64,
    pub distance_km: u32,            // Distance from subject property
    pub price_per_sqm: u128,         // Price per square meter
    pub size_sqm: u64,              // Property size in square meters
    pub sale_date: u64,             // When it was sold
    pub adjustment_factor: i32,     // Adjustment factor (+/- percentage)
}
```

## Error Types

### Property Registry Errors
```rust
pub enum Error {
    PropertyNotFound,
    Unauthorized,
    InvalidMetadata,
    EscrowNotFound,
    InsufficientBalance,
    TransferFailed,
}
```

### Oracle Errors
```rust
pub enum OracleError {
    PropertyNotFound,
    InsufficientSources,
    InvalidValuation,
    Unauthorized,
    OracleSourceNotFound,
    InvalidParameters,
    PriceFeedError,
    AlertNotFound,
}
```

## Events

#[ink(event)]
pub struct PropertyRegistered {
    #[ink(topic)]
    property_id: PropertyId,
    #[ink(topic)]
    owner: AccountId,
    version: u8,
}

#[ink(event)]
pub struct PropertyTransferred {
    #[ink(topic)]
    property_id: PropertyId,
    #[ink(topic)]
    from: AccountId,
    #[ink(topic)]
    to: AccountId,
}

#[ink(event)]
pub struct PropertyMetadataUpdated {
    #[ink(topic)]
    property_id: PropertyId,
    metadata: PropertyMetadata,
}

#[ink(event)]
pub struct Approval {
    #[ink(topic)]
    property_id: PropertyId,
    #[ink(topic)]
    owner: AccountId,
    #[ink(topic)]
    approved: AccountId,
}

#[ink(event)]
pub struct EscrowCreated {
    #[ink(topic)]
    escrow_id: EscrowId,
    property_id: PropertyId,
    amount: Balance,
}

#[ink(event)]
pub struct ValuationUpdated {
    #[ink(topic)]
    property_id: u64,
    valuation: u128,
    confidence_score: u32,
    timestamp: u64,
}

#[ink(event)]
pub struct PriceAlertTriggered {
    #[ink(topic)]
    property_id: u64,
    old_valuation: u128,
    new_valuation: u128,
    change_percentage: u32,
    alert_address: AccountId,
}

#[ink(event)]
pub struct OracleSourceAdded {
    #[ink(topic)]
    source_id: String,
    source_type: OracleSourceType,
    weight: u32,
}
```

## Usage Examples

### Registering a Property
```rust
let metadata = PropertyMetadata {
    location: "123 Main St, City, State".to_string(),
    size: 2000,
    legal_description: "Lot 1, Block 2".to_string(),
    valuation: 500000,
    documents_url: "https://ipfs.io/...".to_string(),
};

let property_id = contract.register_property(metadata)?;
```

### Creating an Escrow
```rust
let escrow_id = escrow_contract.create_escrow(
    property_id,
    buyer_account,
    500000
)?;
```

### Using the Property Valuation Oracle

#### Getting Property Valuation
```rust
// Get current property valuation
let valuation = oracle.get_property_valuation(property_id)?;

// Get valuation with confidence metrics
let valuation_with_confidence = oracle.get_valuation_with_confidence(property_id)?;
println!("Valuation: ${}", valuation_with_confidence.valuation.valuation);
println!("Confidence: {}%", valuation_with_confidence.valuation.confidence_score);
println!("Volatility: {}%", valuation_with_confidence.volatility_index);
```

#### Setting Up Price Alerts
```rust
// Set up 5% price change alert
oracle.set_price_alert(property_id, 5, alert_recipient_address)?;
```

#### Adding Oracle Sources
```rust
// Add Chainlink price feed (admin only)
let chainlink_source = OracleSource {
    id: "chainlink_usd_feed".to_string(),
    source_type: OracleSourceType::Chainlink,
    address: chainlink_feed_address,
    is_active: true,
    weight: 60,
    last_updated: timestamp,
};
oracle.add_oracle_source(chainlink_source)?;
```

#### Getting Market Analytics
```rust
// Get market volatility for residential properties in NYC
let volatility = oracle.get_market_volatility(
    PropertyType::Residential,
    "NYC".to_string()
)?;

// Get comparable properties within 5km
let comparables = oracle.get_comparable_properties(property_id, 5);
```

## Gas Optimization Tips

1. Use efficient data structures (e.g., `Mapping` over `Vec`)
2. Batch operations when possible
3. Minimize storage writes
4. Use appropriate visibility modifiers

## Security Considerations

1. Always validate input parameters
2. Implement proper access control
3. Use reentrancy guards where applicable
4. Consider gas limits for loops
