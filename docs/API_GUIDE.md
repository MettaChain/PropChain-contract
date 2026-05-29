# PropChain API Documentation Guide

## Overview

This guide provides developers with complete, well-documented APIs for integrating with PropChain smart contracts. It follows the standards defined in [API_DOCUMENTATION_STANDARDS.md](./API_DOCUMENTATION_STANDARDS.md) and includes comprehensive error documentation from [API_ERROR_CODES.md](./API_ERROR_CODES.md).

---

## Quick Start

### 1. Find What You Need

**By Use Case**:
- **Interactive API Playground**: See [API Playground](./API_PLAYGROUND.md) for direct local node contract calls in the docs
- **Register Property**: See [`register_property`](#register_property)
- **Transfer Ownership**: See [`transfer_property`](#transfer_property)
- **Check Compliance**: See [`check_account_compliance`](#check_account_compliance)
- **Create Escrow**: See [Escrow Contract](#escrow-contract)
- **Get Valuation**: See [Oracle Contract](#oracle-contract)

**By Role**:
- **Frontend Developer**: Start with examples and basic operations
- **Backend Developer**: Focus on events and state queries
- **Smart Contract Dev**: Review integration patterns and cross-contract calls
- **Auditor**: Study error handling and security requirements

---

## Core API Reference

### Property Registry Contract

The main contract for property management and ownership tracking.

#### Constructor

##### `new()`

Creates and initializes a new PropertyRegistry contract instance.

**Documentation**: See detailed rustdoc in source code  
**Example**:
```rust
// Deployed automatically - no manual call needed
let contract = PropertyRegistry::new();
assert_eq!(contract.version(), 1);
```

---

#### Read-Only Functions (View Methods)

These functions don't modify state and are free to call.

##### `version() -> u32`

Returns the contract version number.

**Parameters**: None  
**Returns**: `u32` - Version number (currently 1)  
**Gas Cost**: ~500 gas  
**Example**:
```rust
let version = contract.version();
if version >= 2 {
    // Use new features
}
```

---

##### `admin() -> AccountId`

Returns the admin account address.

**Parameters**: None  
**Returns**: `AccountId` - Admin's Substrate account  
**Gas Cost**: ~500 gas  
**Example**:
```rust
let admin = contract.admin();
println!("Contract admin: {:?}", admin);
```

---

##### `health_check() -> HealthStatus`

Comprehensive health status for monitoring.

**Parameters**: None  
**Returns**: [`HealthStatus`](crate::HealthStatus) struct with:
- `is_healthy: bool` - Overall health flag
- `is_paused: bool` - Pause state
- `contract_version: u32` - Version number
- `property_count: u64` - Total properties
- `escrow_count: u64` - Active escrows
- `has_oracle: bool` - Oracle configured
- `has_compliance_registry: bool` - Compliance configured
- `has_fee_manager: bool` - Fee manager configured
- `block_number: u32` - Current block
- `timestamp: u64` - Current timestamp

**Gas Cost**: ~2,000 gas  
**Example**:
```rust
let health = contract.health_check();
if !health.is_healthy {
    alert_admins("Contract issues detected!");
}
println!("Properties: {}", health.property_count);
```

---

##### `ping() -> bool`

Simple liveness check.

**Parameters**: None  
**Returns**: `bool` - Always returns `true` if contract is responsive  
**Gas Cost**: ~500 gas  
**Use Case**: Verify contract is deployed and operational

---

##### `dependencies_healthy() -> bool`

Checks if all critical dependencies are configured.

**Parameters**: None  
**Returns**: `bool` - `true` if oracle, compliance, and fee manager all configured  
**Gas Cost**: ~1,000 gas  
**Example**:
```rust
if contract.dependencies_healthy() {
    println!("All systems operational");
} else {
    println!("Some dependencies not configured");
}
```

---

##### `oracle() -> Option<AccountId>`

Returns the oracle contract address.

**Parameters**: None  
**Returns**: `Option<AccountId>` - Oracle address if configured  
**Gas Cost**: ~500 gas  

---

##### `get_fee_manager() -> Option<AccountId>`

Returns the fee manager contract address.

**Parameters**: None  
**Returns**: `Option<AccountId>` - Fee manager address if configured  
**Gas Cost**: ~500 gas  

---

##### `get_compliance_registry() -> Option<AccountId>`

Returns the compliance registry contract address.

**Parameters**: None  
**Returns**: `Option<AccountId>` - Compliance registry address if configured  
**Gas Cost**: ~500 gas  

---

##### `check_account_compliance(account: AccountId) -> Result<bool, Error>`

Checks if an account meets compliance requirements.

**Parameters**:
- `account` (`AccountId`) - Account to check

**Returns**:
- `Ok(bool)` - `true` if compliant, `false` otherwise
- `Err(Error)` - If compliance check fails technically

**Errors**:
- [`Error::ComplianceCheckFailed`](./API_ERROR_CODES.md#error-compliancecheckfailed) - Registry call failed
- [`Error::OracleError`](./API_ERROR_CODES.md#error-oracleerror) - Cross-contract call failure

**Gas Cost**: ~5,000 gas (includes cross-contract call)  
**Example**:
```rust
match contract.check_account_compliance(buyer_account) {
    Ok(true) => println!("Account is compliant"),
    Ok(false) => println!("Account NOT compliant - needs KYC"),
    Err(e) => eprintln!("Compliance check error: {:?}", e),
}
```

---

##### `get_dynamic_fee(operation: FeeOperation) -> u128`

Returns the dynamic fee for a specific operation.

If no fee manager is configured, or if the fee manager circuit breaker is open,
this call returns `0` as a safe fallback.

**Parameters**:
- `operation` (`FeeOperation`) - Type of operation

**Returns**:
- `u128` - Fee amount in smallest currency unit (cents)

**Gas Cost**: ~3,000 gas  
**Example**:
```rust
let fee = contract.get_dynamic_fee(FeeOperation::PropertyTransfer);
println!("Transfer fee: {} cents", fee);
```

---

#### State-Changing Functions (Transactions)

These functions modify contract state and require gas.

##### `change_admin(new_admin: AccountId) -> Result<(), Error>`

Transfers admin privileges to a new account.

**Parameters**:
- `new_admin` (`AccountId`) - Account to receive admin privileges
  - **Format**: 32-byte Substrate account ID
  - **Requirements**: Must be valid account (checksum verified)

**Returns**:
- `Ok(())` - Admin changed successfully
- `Err(Error::Unauthorized)` - Caller is not current admin

**Events Emitted**:
- [`AdminChanged`](crate::AdminChanged) - Logs old/new admin and caller

**Security Requirements**:
- **Access Control**: Only current admin can call
- **Multi-sig Recommended**: Use governance for production changes
- **Timelock**: Consider delay for security

**Gas Cost**: ~50,000 gas  
**Example**:
```rust
// Transfer admin to new multisig wallet
contract.change_admin(new_multisig_wallet)?;
println!("Admin transferred successfully");
```

---

##### `set_oracle(oracle: AccountId) -> Result<(), Error>`

Configures the price oracle contract address.

**Parameters**:
- `oracle` (`AccountId`) - Oracle contract address
  - **Requirements**: Must be deployed oracle contract

**Returns**:
- `Ok(())` - Oracle configured successfully
- `Err(Error::Unauthorized)` - Caller is not admin

**Gas Cost**: ~30,000 gas  
**Example**:
```rust
// Configure oracle after deployment
contract.set_oracle(oracle_contract_address)?;
```

---

##### `set_fee_manager(fee_manager: Option<AccountId>) -> Result<(), Error>`

Configures or removes the fee manager contract.

**Parameters**:
- `fee_manager` (`Option<AccountId>`) - Fee manager address or `None` to disable

**Returns**:
- `Ok(())` - Configuration updated
- `Err(Error::Unauthorized)` - Caller is not admin

**Gas Cost**: ~30,000 gas  

---

##### `set_compliance_registry(registry: Option<AccountId>) -> Result<(), Error>`

Configures or removes the compliance registry contract.

**Parameters**:
- `registry` (`Option<AccountId>`) - Compliance registry address or `None`

**Returns**:
- `Ok(())` - Configuration updated
- `Err(Error::Unauthorized)` - Caller is not admin

**Gas Cost**: ~30,000 gas  

---

##### `update_valuation_from_oracle(property_id: u64) -> Result<(), Error>`

Updates property valuation using oracle price feed.

This call is protected by the oracle circuit breaker. When the breaker is open,
the function fails fast before attempting the external call.

**Parameters**:
- `property_id` (`u64`) - ID of property to update
  - **Constraints**: Must exist in registry

**Returns**:
- `Ok(())` - Valuation updated successfully
- `Err(Error::PropertyNotFound)` - Property doesn't exist
- `Err(Error::OracleError)` - Oracle call failed
- `Err(Error::OracleError)` - Oracle not configured
- `Err(Error::ExternalDependencyUnavailable)` - Oracle circuit breaker is open

**Events Emitted**:
- Property metadata updated event (indirectly)

**Gas Cost**: ~75,000 gas (cross-contract call)  
**Example**:
```rust
// Update valuation before sale
contract.update_valuation_from_oracle(property_id)?;
let valuation = get_current_valuation(property_id);
```

---

##### `get_external_dependency_breaker(dependency: ExternalDependency) -> CircuitBreakerState`

Returns the current circuit breaker state for an external dependency.

**Parameters**:
- `dependency` (`ExternalDependency`) - Dependency to inspect (`Oracle`, `ComplianceRegistry`, `IdentityRegistry`, `FeeManager`)

**Returns**:
- `CircuitBreakerState` - Current breaker counters and cooldown window

---

##### `get_external_dependency_breaker_config() -> CircuitBreakerConfig`

Returns the global circuit breaker configuration for external calls.

**Returns**:
- `CircuitBreakerConfig` - Failure threshold and cooldown period

---

##### `configure_external_dependency_breaker(failure_threshold: u8, cooldown_period_secs: u64) -> Result<(), Error>`

Updates the shared circuit breaker configuration for external dependencies.

**Parameters**:
- `failure_threshold` (`u8`) - Number of consecutive failures before opening the breaker
- `cooldown_period_secs` (`u64`) - Cooldown period before calls are allowed again

**Returns**:
- `Ok(())` - Breaker configuration updated
- `Err(Error::Unauthorized)` - Caller is not admin
- `Err(Error::ValueOutOfBounds)` - Threshold or cooldown is zero

---

##### `trip_external_dependency_breaker(dependency: ExternalDependency) -> Result<(), Error>`

Opens a dependency breaker immediately. Intended for admin-operated emergency isolation.

**Parameters**:
- `dependency` (`ExternalDependency`) - Dependency to isolate

**Returns**:
- `Ok(())` - Breaker opened
- `Err(Error::Unauthorized)` - Caller is not admin

---

##### `reset_external_dependency_breaker(dependency: ExternalDependency) -> Result<(), Error>`

Clears a dependency breaker after the external service has recovered.

**Parameters**:
- `dependency` (`ExternalDependency`) - Dependency to restore

**Returns**:
- `Ok(())` - Breaker reset
- `Err(Error::Unauthorized)` - Caller is not admin

---

##### `pause_contract(reason: String, duration_seconds: Option<u64>) -> Result<(), Error>`

Pauses all non-critical contract operations.

**Parameters**:
- `reason` (`String`) - Human-readable pause reason
  - **Max Length**: 1024 characters
  - **Example**: `"Emergency maintenance - security audit"`
- `duration_seconds` (`Option<u64>`) - Optional auto-resume delay
  - **Example**: `Some(86400)` for 24 hours
  - **None**: Manual resume required

**Returns**:
- `Ok(())` - Contract paused successfully
- `Err(Error::NotAuthorizedToPause)` - Caller lacks permission
- `Err(Error::AlreadyPaused)` - Contract already paused

**Events Emitted**:
- [`ContractPaused`](crate::ContractPaused) - Includes reason and auto-resume time

**Security Requirements**:
- **Access Control**: Admin or pause guardians only
- **Use Sparingly**: Emergency situations only
- **Communication**: Announce pause publicly

**Gas Cost**: ~50,000 gas  
**Example**:
```rust
// Emergency pause
contract.pause_contract(
    "Critical vulnerability discovered".to_string(),
    None // Manual resume required
)?;
```

---

##### `emergency_pause(reason: String) -> Result<(), Error>`

Immediate pause without auto-resume (critical emergencies).

**Parameters**:
- `reason` (`String`) - Emergency reason

**Returns**: Same as `pause_contract`  
**Gas Cost**: ~50,000 gas  
**Note**: Equivalent to `pause_contract(reason, None)`

---

##### `try_auto_resume() -> Result<(), Error>`

Attempts to resume contract if auto-resume time has passed.

**Parameters**: None  
**Returns**:
- `Ok(())` - Contract resumed successfully
- `Err(Error::NotPaused)` - Contract not paused
- `Err(Error::ResumeRequestNotFound)` - No active resume request

**Events Emitted**:
- [`ContractResumed`](crate::ContractResumed)

**Gas Cost**: ~30,000 gas  

---

---

## Error Handling Guide

### Common Error Patterns

#### 1. Authorization Failures

```rust
match contract.operation() {
    Ok(result) => process(result),
    Err(Error::Unauthorized) => {
        eprintln!("Access denied - check permissions");
        // Guide user to request access
    }
    Err(e) => handle_other_error(e),
}
```

#### 2. Compliance Failures

```rust
match contract.transfer_property(buyer, token_id) {
    Ok(_) => println!("Transfer complete"),
    Err(Error::NotCompliant) => {
        eprintln!("Buyer not compliant");
        eprintln!("Required: Complete KYC at https://kyc.propchain.io");
    }
    Err(e) => eprintln!("Error: {:?}", e),
}
```

#### 3. Validation Failures

```rust
// Pre-validate before submission
fn validate_metadata(metadata: &PropertyMetadata) -> Result<(), &'static str> {
    if metadata.location.is_empty() {
        return Err("Location required");
    }
    if metadata.valuation < 1000 {
        return Err("Minimum valuation $10");
    }
    Ok(())
}

// Then submit
match validate_metadata(&metadata) {
    Ok(_) => contract.register_property(metadata)?,
    Err(e) => eprintln!("Invalid metadata: {}", e),
}
```

### Complete Error Reference

See [API_ERROR_CODES.md](./API_ERROR_CODES.md) for comprehensive documentation of all error types including:
- Trigger conditions
- Common scenarios
- Recovery steps
- Examples
- HTTP equivalents

---

## Integration Examples

### Frontend Integration (React/TypeScript)

```typescript
import { useContract } from '@polkadot/react-hooks';

function RegisterPropertyForm() {
  const contract = useContract(CONTRACT_ADDRESS);
  
  const handleSubmit = async (metadata: PropertyMetadata) => {
    try {
      // Check compliance first
      const isCompliant = await contract.query.checkAccountCompliance(
        currentUser.address
      );
      
      if (!isCompliant) {
        throw new Error('Complete KYC first');
      }
      
      // Submit registration
      const tx = await contract.tx.registerProperty(metadata);
      await tx.signAndSend(currentUser.pair, ({ status, events }) => {
        if (status.isInBlock) {
          console.log('Transaction included in block');
          
          // Extract property ID from events
          const propertyRegistered = events.find(
            e => e.event.method === 'PropertyRegistered'
          );
          const propertyId = propertyRegistered?.event.data[0];
          console.log('Property ID:', propertyId.toString());
        }
      });
    } catch (error) {
      if (error.message.includes('NotCompliant')) {
        alert('Please complete KYC verification first');
      } else if (error.message.includes('InvalidMetadata')) {
        alert('Please check property details');
      } else {
        console.error('Registration failed:', error);
      }
    }
  };
  
  return (
    <form onSubmit={handleSubmit}>
      {/* Form fields */}
    </form>
  );
}
```

### Backend Integration (Node.js)

```javascript
const { ApiPromise, WsProvider } = require('@polkadot/api');

async function registerProperty(metadata) {
  const api = await ApiPromise.create({
    provider: new WsProvider('wss://rpc.propchain.io')
  });
  
  // Query current state
  const health = await api.query.propertyRegistry.healthCheck();
  if (!health.isHealthy) {
    throw new Error('Contract not healthy');
  }
  
  // Check compliance
  const isCompliant = await api.query.complianceRegistry.isCompliant(
    userAddress
  );
  if (!isCompliant) {
    throw new Error('User not compliant');
  }
  
  // Submit transaction
  const tx = api.tx.propertyRegistry.registerProperty(metadata);
  const hash = await tx.signAndSend(keypair);
  
  console.log('Transaction submitted:', hash.toHex());
  return hash;
}
```

### Smart Contract Integration

```rust
// Cross-contract call pattern
use ink::env::call::FromAccountId;

fn integrate_with_property_registry(
    registry_addr: AccountId,
    metadata: PropertyMetadata
) -> Result<u64, Error> {
    let registry: ink::contract_ref!(PropertyRegistry) = 
        FromAccountId::from_account_id(registry_addr);
    
    // Call registry method
    let property_id = registry.register_property(metadata)?;
    
    Ok(property_id)
}
```

---

## Events Reference

### Key Events to Monitor

#### `PropertyRegistered`

Emitted when a new property is registered.

**Indexed Fields** (filterable):
- `property_id: u64`
- `owner: AccountId`

**Data Fields**:
- `location: String`
- `size: u64`
- `valuation: u128`
- `timestamp: u64`
- `block_number: u32`
- `transaction_hash: Hash`

**Use Cases**:
- Index property ownership
- Trigger off-chain workflows
- Update analytics dashboards

---

#### `PropertyTransferred`

Emitted when property ownership changes.

**Indexed Fields**:
- `property_id: u64`
- `from: AccountId`
- `to: AccountId`

**Use Cases**:
- Update ownership records
- Calculate transfer taxes
- Track investment portfolios

---

#### `EscrowCreated` / `EscrowReleased`

Track escrow lifecycle for secure transfers.

**Use Cases**:
- Monitor transaction progress
- Detect stuck escrows
- Calculate escrow fees

---

## Gas Optimization Tips

### 1. Batch Operations

```rust
// ❌ Expensive: Multiple transactions
for property in properties {
    contract.register_property(property)?;
}

// ✅ Cheaper: Single batch transaction
contract.batch_register_properties(properties)?;
```

### 2. Pre-validation

```rust
// Validate off-chain first to avoid wasting gas
if !validate_metadata_locally(&metadata) {
    return Err("Invalid metadata"); // Save gas by not submitting
}
```

### 3. Efficient Queries

```rust
// ❌ Expensive: Query in loop
for id in property_ids {
    let prop = contract.get_property(id)?; // Multiple calls
}

// ✅ Better: Batch query if available
let props = contract.get_properties_batch(property_ids)?; // Single call
```

---

## Testing Guide

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_register_property() {
        let mut contract = PropertyRegistry::new();
        let metadata = create_test_metadata();
        
        let result = contract.register_property(metadata);
        assert!(result.is_ok());
        
        let property_id = result.unwrap();
        assert!(property_id > 0);
    }
    
    #[test]
    fn test_unauthorized_admin_change() {
        let mut contract = PropertyRegistry::new();
        let unauthorized_account = AccountId::from([1u8; 32]);
        
        // Set caller to unauthorized account
        set_caller(unauthorized_account);
        
        let result = contract.change_admin(AccountId::from([2u8; 32]));
        assert!(matches!(result, Err(Error::Unauthorized)));
    }
}
```

### Integration Tests

```rust
#[ink_e2e::test]
async fn test_full_property_lifecycle(mut client: ink_e2e::Client<C, E>) {
    // Setup
    let mut builder = build_contract!("propchain_contracts", "PropertyRegistry");
    let contract_id = client.instantiate("propchain_contracts", &bob, 0, &mut builder).await?;
    
    // Register property
    let metadata = create_metadata();
    let register_msg = propchain_contracts::Message::RegisterProperty { metadata };
    let result = client.call(&bob, register_msg, &mut storage()).await?;
    
    // Verify
    assert!(result.return_value().is_ok());
}
```

---

## Related Documentation

- **[API Documentation Standards](./API_DOCUMENTATION_STANDARDS.md)** - How we document APIs
- **[API Error Codes](./API_ERROR_CODES.md)** - Comprehensive error reference
- **[Architecture Overview](./SYSTEM_ARCHITECTURE_OVERVIEW.md)** - System context
- **[Integration Guide](./integration.md)** - General integration patterns
- **[Troubleshooting FAQ](./troubleshooting-faq.md)** - Common issues

---

## Getting Help

### Resources

- **GitHub Issues**: Report bugs or request features
- **Discord**: Real-time developer support
- **Stack Overflow**: Technical Q&A (tag: `propchain`)
- **Documentation**: Complete docs at docs.propchain.io

### Support Channels

| Issue Type | Best Channel | Response Time |
|------------|--------------|---------------|
| Bug Reports | GitHub Issues | 24-48 hours |
| Integration Help | Discord #dev-support | < 1 hour |
| Security Issues | security@propchain.io | Immediate |
| General Questions | Stack Overflow | 2-24 hours |

---

**Last Updated**: March 27, 2026  
**Version**: 1.0.0  
**Maintained By**: PropChain Development Team
