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

let salt = generate_deterministic_salt(&template.encode_params());

let config = DeploymentConfig {
    contract_type: ContractType::PropertyToken,
    salt,
    init_params: template.encode_params(),
};

let address = factory.deploy_contract(config, "1.0.0".to_string())?;
```

#### Option B: Using Builder Pattern

```rust
use propchain_factory::builder::DeploymentBuilder;

let salt = generate_deterministic_salt(&encoded_params);

let (config, version) = DeploymentBuilder::new()
    .contract_type(ContractType::Escrow)
    .salt(salt)
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

let salt = generate_deterministic_salt(&template.encode_params());

let config = DeploymentConfig {
    contract_type: ContractType::PropertyToken,
    salt,
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

let salt = generate_deterministic_salt(&template.encode_params());

let config = DeploymentConfig {
    contract_type: ContractType::Escrow,
    salt,
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

let salt = generate_deterministic_salt(&template.encode_params());

let config = DeploymentConfig {
    contract_type: ContractType::Oracle,
    salt,
    init_params: template.encode_params(),
};

let oracle_address = factory.deploy_contract(config, "1.0.0".to_string())?;
```

## Deterministic Deployments with CREATE2

The factory now supports CREATE2-style deterministic deployments. This means that the same contract configuration will always be deployed to the same address, regardless of the network or the deployer. This is achieved by using a salt that is based on the contract's initialization parameters.

### Salt Generation for Deterministic Addresses

To generate a deterministic address, you should use a salt that is derived from the contract's initialization parameters. This ensures that any change in the configuration will result in a different address.

```rust
use ink::env::hash::{Blake2x256, HashOutput};

fn generate_deterministic_salt(params: &[u8]) -> [u8; 32] {
    let mut output = <Blake2x256 as HashOutput>::Type::default();
    ink::env::hash_bytes::<Blake2x256>(
        params,
        &mut output,
    );
    output
}
```

## Pre-computing Contract Addresses

A key advantage of deterministic deployments is the ability to pre-compute a contract's address without actually deploying it. This is useful for counter-factual reasoning, setting up off-chain systems, and more.

The address is determined by the factory's address, the salt, and the code hash of the contract being deployed. You can compute the address off-chain using a similar hashing function to the one used in the factory.

```rust
use ink::env::hash::{Blake2x256, HashOutput};

fn pre_compute_address(
    factory_address: &AccountId,
    code_hash: &Hash,
    salt: &[u8; 32],
) -> AccountId {
    let mut output = <Blake2x256 as HashOutput>::Type::default();
    let mut input = Vec::new();
    input.extend_from_slice(factory_address.as_ref());
    input.extend_from_slice(salt);
    input.extend_from_slice(code_hash.as_ref());
    ink::env::hash_bytes::<Blake2x256>(&input, &mut output);
    AccountId::from(output)
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