# PropChain Contract Factory

A standardized factory pattern implementation for deploying PropChain smart contracts.

## Overview

The Contract Factory provides a centralized, secure, and standardized way to deploy PropChain contracts. It manages code hashes, tracks deployments, and ensures consistent deployment patterns across the ecosystem.

## Features

- **Standardized Deployment**: Consistent deployment process for all contract types
- **Code Hash Management**: Centralized registry of approved contract code hashes
- **Deployment Tracking**: Complete audit trail of all deployed contracts
- **Access Control**: Admin-controlled code hash updates
- **Multi-Contract Support**: Supports all PropChain contract types

## Supported Contract Types

- PropertyToken
- Escrow
- Oracle
- Bridge
- Insurance
- Governance
- Dex
- Lending
- Crowdfunding
- Fractional

## Usage

### 1. Deploy the Factory

```rust
let factory = ContractFactory::new();
```

### 2. Set Code Hashes (Admin Only)

```rust
factory.set_code_hash(
    ContractType::PropertyToken,
    property_token_code_hash
)?;
```

### 3. Deploy a Contract

```rust
let config = DeploymentConfig {
    contract_type: ContractType::PropertyToken,
    salt: [0u8; 32],
    init_params: encoded_params,
};

let contract_address = factory.deploy_contract(
    config,
    "1.0.0".to_string()
)?;
```

### 4. Query Deployments

```rust
// Get deployment info
let deployment = factory.get_deployment(deployment_id)?;

// Get all contracts deployed by an account
let contracts = factory.get_deployer_contracts(deployer_account);

// Get total deployment count
let count = factory.get_deployment_count();
```

## Architecture Benefits

1. **Centralized Control**: Single point for managing approved contract versions
2. **Auditability**: Complete deployment history with timestamps and deployers
3. **Upgradeability**: Easy to update contract implementations by changing code hashes
4. **Security**: Admin-controlled deployment process prevents unauthorized contracts
5. **Standardization**: Consistent deployment patterns across all contract types

## Security Considerations

- Only admin can update code hashes
- All deployments are tracked and auditable
- Salt-based deployment prevents address collisions
- Version tracking for contract upgrades

## Events

- `ContractDeployed`: Emitted when a new contract is deployed
- `CodeHashUpdated`: Emitted when a code hash is updated

## Error Handling

- `Unauthorized`: Caller is not authorized for the operation
- `InvalidContractType`: Unsupported contract type
- `DeploymentFailed`: Contract deployment failed
- `CodeHashNotSet`: No code hash configured for contract type
- `ContractNotFound`: Deployment ID not found
- `InvalidParameters`: Invalid deployment parameters
