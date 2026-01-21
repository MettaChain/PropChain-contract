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

### EscrowContract

Handles secure property transfers with escrow protection.

#### Methods

##### `create_escrow(property_id: PropertyId, buyer: AccountId, amount: Balance) -> Result<EscrowId, Error>`
Creates a new escrow for property transfer.

##### `release_escrow(escrow_id: EscrowId) -> Result<(), Error>`
Releases escrow funds to the seller.

##### `refund_escrow(escrow_id: EscrowId) -> Result<(), Error>`
Refunds escrow funds to the buyer.

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

## Error Types

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

## Events

```rust
#[ink(event)]
pub struct PropertyRegistered {
    #[ink(topic)]
    property_id: PropertyId,
    owner: AccountId,
}

#[ink(event)]
pub struct PropertyTransferred {
    #[ink(topic)]
    property_id: PropertyId,
    from: AccountId,
    to: AccountId,
}

#[ink(event)]
pub struct EscrowCreated {
    #[ink(topic)]
    escrow_id: EscrowId,
    property_id: PropertyId,
    amount: Balance,
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
