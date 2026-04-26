# Contract Factory Deployment Guide

This guide explains how to use the PropChain Contract Factory for standardized contract deployment.

## Prerequisites

1. Factory contract deployed and initialized
2. Code hashes for contracts you want to deploy
3. Admin access to set code hashes (first time only)

## Step-by-Step Deployment

### 1. Deploy the Factory Contract

```bash
cargo contract build --manifest-path contracts/factory/Cargo.toml
cargo contract instantiate \
  --constructor new \
  --suri //Alice \
  target/ink/factory.contract
```

### 2. Register Contract Code Hashes (Admin Only)

First, upload the contract code you want to deploy:

```bash
# Upload PropertyToken contract
cargo contract upload target/ink/property_token.contract

# Note the code hash from the output
```

Then register it with the factory:

```rust
// Set code hash for PropertyToken
factory.set_code_hash(
    ContractType::PropertyToken,
    "0x1234...abcd" // code hash from upload
)?;
```

### 3. Deploy a Contract Using the Factory

#### Option A: Using Templates

```rust
use propchain_factory::templates::PropertyTokenTemplate;

let template = PropertyTokenTemplate {
    admin: admin_account,
    name: "Property Token".to_string(),
    symbol: "PROP".to_string(),
};

let config = DeploymentConfig {
    contract_type: ContractType::PropertyToken,
    salt: generate_salt(),
    init_params: template.encode_params(),
};

let address = factory.deploy_contract(config, "1.0.0".to_string())?;
```

#### Option B: Using Builder Pattern

```rust
use propchain_factory::builder::DeploymentBuilder;

let (config, version) = DeploymentBuilder::new()
    .contract_type(ContractType::Escrow)
    .salt(generate_salt())
    .init_params(encoded_params)
    .version("1.0.0".to_string())
    .build()?;

let address = factory.deploy_contract(config, version)?;
```

### 4. Query Deployments

```rust
// Get specific deployment
let deployment = factory.get_deployment(0)?;
println!("Contract address: {:?}", deployment.address);

// Get all contracts deployed by an account
let my_contracts = factory.get_deployer_contracts(my_account);

// Get total deployment count
let total = factory.get_deployment_count();
```

## Deployment Examples

### Deploy PropertyToken

```rust
let template = PropertyTokenTemplate {
    admin: admin_account,
    name: "Luxury Apartment Token".to_string(),
    symbol: "LAT".to_string(),
};

let config = DeploymentConfig {
    contract_type: ContractType::PropertyToken,
    salt: [1u8; 32],
    init_params: template.encode_params(),
};

let token_address = factory.deploy_contract(config, "1.0.0".to_string())?;
```

### Deploy Escrow

```rust
let template = EscrowTemplate {
    admin: admin_account,
    fee_percentage: 250, // 2.5%
};

let config = DeploymentConfig {
    contract_type: ContractType::Escrow,
    salt: [2u8; 32],
    init_params: template.encode_params(),
};

let escrow_address = factory.deploy_contract(config, "1.0.0".to_string())?;
```

### Deploy Oracle

```rust
let template = OracleTemplate {
    admin: admin_account,
    update_interval: 3600, // 1 hour
};

let config = DeploymentConfig {
    contract_type: ContractType::Oracle,
    salt: [3u8; 32],
    init_params: template.encode_params(),
};

let oracle_address = factory.deploy_contract(config, "1.0.0".to_string())?;
```

## Salt Generation

Generate unique salts to avoid address collisions:

```rust
use ink::env::hash::{Blake2x256, HashOutput};

fn generate_salt() -> [u8; 32] {
    let mut output = <Blake2x256 as HashOutput>::Type::default();
    ink::env::hash_bytes::<Blake2x256>(
        &[
            &ink::env::block_timestamp().to_le_bytes()[..],
            &ink::env::caller().as_ref()[..],
        ].concat(),
        &mut output,
    );
    output
}
```

## Upgrading Contracts

To deploy a new version:

1. Upload new contract code
2. Update code hash in factory (admin only)
3. Deploy using new version string

```rust
// Update to v2
factory.set_code_hash(ContractType::PropertyToken, new_code_hash)?;

// Deploy v2 instance
let address = factory.deploy_contract(config, "2.0.0".to_string())?;
```

## Best Practices

1. **Use Unique Salts**: Always generate unique salts to avoid deployment conflicts
2. **Version Tracking**: Use semantic versioning for deployed contracts
3. **Test First**: Deploy to testnet before mainnet
4. **Verify Code Hashes**: Double-check code hashes before setting
5. **Monitor Events**: Subscribe to deployment events for tracking
6. **Access Control**: Restrict admin access to trusted accounts
7. **Audit Trail**: Keep records of all deployments

## Troubleshooting

### Deployment Failed

- Check code hash is set correctly
- Ensure sufficient gas and balance
- Verify init parameters are correct
- Check salt is unique

### Unauthorized Error

- Verify you're using admin account
- Check admin hasn't changed

### Code Hash Not Set

- Upload contract code first
- Set code hash using `set_code_hash`

## Security Considerations

1. **Admin Security**: Protect admin private keys
2. **Code Verification**: Verify contract code before uploading
3. **Parameter Validation**: Validate all init parameters
4. **Event Monitoring**: Monitor deployment events for unauthorized activity
5. **Access Logs**: Review deployment history regularly
