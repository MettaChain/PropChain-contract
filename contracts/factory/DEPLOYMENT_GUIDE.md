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

## Pre-Deployment Checklist (Production)

Complete all items in this checklist before deploying the factory to mainnet:

### 📋 Smart Contract Audits
- [ ] All factory contract dependencies have been audited by a reputable third party
- [ ] Factory code itself has undergone a full security audit with all findings resolved
- [ ] All template contracts (PropertyToken, Escrow, Oracle) have been independently audited
- [ ] Audit reports are publicly disclosed and published in the repository
- [ ] Reentrancy guards, access controls, and input validation have been verified by auditors

### 🔐 Multisig & Access Control Configuration
- [ ] Factory admin is set to a 2/3 or 3/5 multisig wallet
- [ ] Multisig signers are distributed geographically and organizationally
- [ ] Timelock is configured for all admin operations (minimum 48-hour delay)
- [ ] Code hash update permissions are restricted to the multisig only
- [ ] Emergency pause functionality is tested and operational
- [ ] Admin key rotation process is documented and agreed upon by all signers

### 🔮 Oracle Setup
- [ ] Price oracle nodes are deployed and synced with major data providers
- [ ] Oracle update intervals are set according to asset volatility requirements
- [ ] Oracle dispute mechanisms are tested and operational
- [ ] Fallback oracles are configured for redundancy
- [ ] Minimum required oracle signatures are set to prevent single points of failure
- [ ] Historical price data integrity has been verified

### 🏛️ Governance Configuration
- [ ] Proposal threshold is set to the required token supply percentage
- [ ] Voting period duration is appropriate for the community size
- [ ] Execution delay is configured to allow for review periods
- [ ] Quorum requirements are tested and validated
- [ ] All governance parameters are documented and communicated to stakeholders
- [ ] Emergency governance procedures are in place

## Post-Deployment Verification Script

After deploying to production, run this comprehensive verification script to confirm everything is configured correctly:

```bash
#!/bin/bash
set -euo pipefail

# Configuration
FACTORY_ADDRESS="YOUR_FACTORY_ADDRESS"
RPC_URL="YOUR_RPC_ENDPOINT"
REQUIRED_CODE_HASHES=(
  "PropertyToken:0xabc123..."
  "Escrow:0xdef456..."
  "Oracle:0xghi789..."
)

echo "🔍 Starting post-deployment verification for PropChain Factory"
echo "Factory Address: $FACTORY_ADDRESS"
echo "RPC URL: $RPC_URL"
echo "========================================"

# 1. Verify factory deployment exists
echo -e "\n📦 Step 1: Verifying factory deployment exists..."
cargo contract call \
  --contract $FACTORY_ADDRESS \
  --message get_deployment_count \
  --url $RPC_URL
echo "✅ Factory is deployed and responsive"

# 2. Verify all code hashes are correctly set
echo -e "\n🔑 Step 2: Verifying all contract code hashes..."
for entry in "${REQUIRED_CODE_HASHES[@]}"; do
  IFS=':' read -r contract_type expected_hash <<< "$entry"
  stored_hash=$(cargo contract call \
    --contract $FACTORY_ADDRESS \
    --message get_code_hash \
    --args $contract_type \
    --url $RPC_URL | grep -o '0x[0-9a-f]*')
  
  if [ "$stored_hash" = "$expected_hash" ]; then
    echo "  ✅ $contract_type: Code hash matches"
  else
    echo "  ❌ $contract_type: Code hash mismatch! Expected $expected_hash, got $stored_hash"
    exit 1
  fi
done

# 3. Verify admin configuration
echo -e "\n👤 Step 3: Verifying admin configuration..."
admin_address=$(cargo contract call \
  --contract $FACTORY_ADDRESS \
  --message get_admin \
  --url $RPC_URL | grep -o '5[0-9a-zA-Z]*')

# Check if admin is the expected multisig address
EXPECTED_MULTISIG="YOUR_MULTISIG_ADDRESS"
if [ "$admin_address" = "$EXPECTED_MULTISIG" ]; then
  echo "  ✅ Admin is correctly set to multisig: $admin_address"
else
  echo "  ⚠️  Admin address ($admin_address) does not match expected multisig"
fi

# 4. Test contract deployment workflow
echo -e "\n🧪 Step 4: Testing contract deployment workflow..."
echo "  Deploying a test PropertyToken to verify factory functionality..."
# This runs a test deployment to ensure the factory can create contracts successfully
cargo contract call \
  --contract $FACTORY_ADDRESS \
  --message deploy_contract \
  --args '{"contract_type":"PropertyToken","salt":"0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef","init_params":"0x..."}' "1.0.0" \
  --url $RPC_URL \
  --suri //TestDeployer
echo "  ✅ Test deployment completed successfully"

# 5. Verify event emission
echo -e "\n📡 Step 5: Verifying event emission..."
cargo contract events --url $RPC_URL --contract $FACTORY_ADDRESS --limit 10 | grep -E "ContractDeployed|CodeHashUpdated"
echo "✅ Factory events are being emitted correctly"

echo -e "\n🎉 All post-deployment checks passed! Factory is ready for production use."
```

### Script Usage
1. Save the script as `verify-deployment.sh`
2. Update the configuration variables at the top
3. Make it executable: `chmod +x verify-deployment.sh`
4. Run it: `./verify-deployment.sh`

## Security Considerations

1. **Admin Security**: Protect admin private keys
2. **Code Verification**: Verify contract code before uploading
3. **Parameter Validation**: Validate all init parameters
4. **Event Monitoring**: Monitor deployment events for unauthorized activity
5. **Access Logs**: Review deployment history regularly