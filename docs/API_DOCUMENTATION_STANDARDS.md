# PropChain API Documentation Standards

## Overview

This document defines the standards and templates for documenting all PropChain smart contract APIs. Following these standards ensures consistency, completeness, and usability for developers integrating with PropChain.

---

## Documentation Principles

### 1. Completeness
Every public API must have:
- Clear description of purpose
- All parameters documented
- Return value explained
- All error scenarios covered
- At least one usage example
- Gas considerations (if applicable)

### 2. Consistency
Use standardized format across all contracts:
- Same section ordering
- Consistent terminology
- Uniform example style
- Standard error documentation

### 3. Clarity
- Use plain English where possible
- Define technical terms on first use
- Provide context for complex operations
- Include edge cases and limitations

### 4. Practicality
- Examples should be copy-paste ready
- Include real-world values
- Show both success and failure cases
- Link to related functions and guides

---

## rustdoc Template

### Standard Function Documentation Format

```rust
/// # Function Name
///
/// ## Description
/// [Clear, concise description of what the function does]
///
/// ## Parameters
/// - `param_name` - [Description of parameter, including valid ranges/constraints]
/// - `param_name2` - [Description, type, constraints]
///
/// ## Returns
/// [Description of return value]
/// - `Ok(type)` - [When successful, what is returned]
/// - `Err(Error::Variant)` - [Link to specific error types]
///
/// ## Errors
/// | Error | Condition | Recovery |
/// |-------|-----------|----------|
/// | `Error::Unauthorized` | Caller lacks required role | Request access from admin |
/// | `Error::InvalidInput` | Parameter validation failed | Correct input values |
///
/// ## Events Emitted
/// - [`EventName`](crate::EventName) - [When emitted, key fields]
///
/// ## Example
/// ```rust,ignore
/// // Example showing typical usage
/// let result = contract.function_name(param1, param2)?;
/// assert_eq!(result, expected_value);
/// ```
///
/// ## Gas Considerations
/// [Gas cost range, factors affecting cost, optimization tips]
///
/// ## Security Requirements
/// [Access control, permissions, compliance checks]
///
/// ## Related Functions
/// - [`related_function`](crate::Contract::related_function) - [Brief description]
///
/// ## Version History
/// - **v1.0.0** - Initial implementation
/// - **v1.1.0** - Enhanced with [feature]
```

---

## Error Documentation Standards

### Error Type Template

```rust
/// # Error Variant Name
///
/// ## Description
/// [Clear explanation of when this error occurs]
///
/// ## Trigger Conditions
/// - Condition 1 that triggers this error
/// - Condition 2
///
/// ## Common Scenarios
/// 1. **Scenario**: User tries to [action] without [prerequisite]
///    **Solution**: Complete [prerequisite] first
///
/// 2. **Scenario**: Invalid parameter value provided
///    **Solution**: Validate input against [requirements]
///
/// ## Recovery Steps
/// 1. Identify the root cause from transaction logs
/// 2. Check [specific condition or requirement]
/// 3. Retry with corrected parameters
///
/// ## Example
/// ```rust,ignore
/// // This will trigger Error::Unauthorized
/// let result = restricted_function(); // Caller: non-admin
/// assert!(matches!(result, Err(Error::Unauthorized)));
/// ```
///
/// ## Related Errors
/// - [`RelatedError`](crate::Error::RelatedError) - [Distinction]
```

---

## Example Usage Guidelines

### Example Categories

#### 1. Basic Usage
Show the simplest common case:
```rust,ignore
// Register a property with standard metadata
let metadata = PropertyMetadata {
    location: "123 Main St".to_string(),
    size: 2000,
    valuation: 500_000,
};
let property_id = registry.register_property(metadata)?;
```

#### 2. Advanced Usage
Demonstrate complex scenarios:
```rust,ignore
// Batch register multiple properties with error handling
let mut property_ids = Vec::new();
for metadata in properties {
    match registry.register_property(metadata) {
        Ok(id) => property_ids.push(id),
        Err(Error::ComplianceCheckFailed) => {
            // Handle compliance issue
            continue;
        }
        Err(e) => return Err(e),
    }
}
```

#### 3. Error Handling
Show how to handle common errors:
```rust,ignore
match contract.transfer_property(to, token_id) {
    Ok(_) => println!("Transfer successful"),
    Err(Error::NotCompliant) => {
        eprintln!("Recipient not compliant - verify KYC/AML");
        // Suggest compliance verification flow
    }
    Err(Error::InsufficientAllowance) => {
        eprintln!("Approve tokens first");
        // Guide through approval process
    }
    Err(e) => eprintln!("Unexpected error: {:?}", e),
}
```

#### 4. Integration Examples
Real-world integration patterns:
```rust,ignore
// Frontend integration pattern
async function registerProperty(metadata) {
  const tx = await contract.methods
    .register_property(metadata)
    .signAndSend(accountPair);
  
  // Handle events
  tx.events.forEach(event => {
    if (event.method === 'PropertyRegistered') {
      console.log('Property ID:', event.data.property_id);
    }
  });
}
```

---

## Parameter Documentation

### Required Information

For each parameter, document:

1. **Type**: Rust type (e.g., `AccountId`, `u64`, `String`)
2. **Constraints**: Valid ranges, format requirements
3. **Purpose**: Why this parameter exists
4. **Examples**: Representative values

### Example Format

```rust
/// ## Parameters
/// - `property_id` (`u64`) - Unique identifier of the property
///   - **Constraints**: Must be > 0 and <= max_property_count
///   - **Example**: `12345`
///   
/// - `metadata` (`PropertyMetadata`) - Property information structure
///   - **location**: Physical address (max 256 chars)
///   - **size**: Area in square meters (1-10,000,000)
///   - **valuation**: Value in USD cents (min: 1000 = $10.00)
///   - **documents_url**: IPFS CID for legal documents
///
/// - `recipient` (`AccountId`) - Account receiving the property
///   - **Format**: 32-byte Substrate account ID
///   - **Requirements**: Must be KYC/AML verified
///   - **Example**: `5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY`
```

---

## Return Value Documentation

### Success Cases

Document what successful returns indicate:

```rust
/// ## Returns
/// - `Ok(u64)` - Property ID of newly registered property
///   - Value always > 0
///   - Can be used immediately for subsequent operations
///   - Emitted in `PropertyRegistered` event
///
/// - `Ok(TransferResult)` - Detailed transfer information
///   - `property_id`: Transferred property
///   - `from`: Previous owner
///   - `to`: New owner
///   - `timestamp`: Block timestamp of transfer
```

### Error Cases

Link to comprehensive error documentation:

```rust
/// ## Errors
/// Returns [`Error`](crate::Error) with specific variants:
///
/// | Error Variant | When | HTTP Equivalent |
/// |---------------|------|-----------------|
/// | [`Unauthorized`](crate::Error::Unauthorized) | Caller lacks permission | 403 Forbidden |
/// | [`PropertyNotFound`](crate::Error::PropertyNotFound) | Invalid property ID | 404 Not Found |
/// | [`InvalidMetadata`](crate::Error::InvalidMetadata) | Malformed input | 400 Bad Request |
/// | [`NotCompliant`](crate::Error::NotCompliant) | Compliance check failed | 422 Unprocessable |
```

---

## Event Documentation

### Standard Event Format

```rust
/// # Event Name
///
/// ## When Emitted
/// [Trigger condition - which operation causes this event]
///
/// ## Indexed Fields (Topics)
/// Fields marked with `#[ink(topic)]` for efficient filtering:
/// - `property_id` - Filter by specific property
/// - `owner` - Filter by owner account
///
/// ## Data Fields
/// Non-indexed fields with detailed information:
/// - `location` - Property location string
/// - `size` - Property size in square meters
/// - `valuation` - Property valuation in USD cents
///
/// ## Example Query
/// ```rust,ignore
/// // Query all properties owned by account
/// let events = api.query::<PropertyRegistered>()
///     .filter(|event| event.owner == target_account)
///     .collect();
/// ```
///
/// ## Off-chain Indexing
/// Indexers should:
/// 1. Listen for this event
/// 2. Extract property_id and owner
/// 3. Update ownership records in database
/// 4. Cache metadata for quick retrieval
```

---

## Gas Documentation

### What to Include

1. **Base Cost**: Typical gas consumption
2. **Variable Factors**: What increases/decreases cost
3. **Optimization Tips**: How to reduce gas usage
4. **Comparisons**: Relative cost vs other operations

### Example Format

```rust
/// ## Gas Considerations
///
/// ### Base Cost
/// - **Minimum**: ~50,000 gas (simple property registration)
/// - **Average**: ~75,000 gas (with compliance checks)
/// - **Maximum**: ~150,000 gas (batch operations)
///
/// ### Variable Factors
/// - **Storage writes**: +10,000 gas per new property
/// - **Compliance checks**: +15,000 gas if registry configured
/// - **Cross-contract calls**: +5,000 gas per call
/// - **String length**: +100 gas per KB of metadata
///
/// ### Optimization Tips
/// 1. Use batch operations for multiple registrations
/// 2. Pre-validate metadata before submission
/// 3. Ensure compliance status is current
/// 4. Avoid very long location strings
///
/// ### Cost Comparison
/// - Cheaper than: [`transfer_property`](crate::Contract::transfer_property) (~100k gas)
/// - More expensive than: [`ping`](crate::Contract::ping) (~1,000 gas)
```

---

## Security Documentation

### Access Control Matrix

Document who can call what:

```rust
/// ## Security Requirements
///
/// ### Access Control
/// | Role | Permission | Notes |
/// |------|------------|-------|
/// | Admin | ✅ Full access | Can bypass some checks |
/// | Verifier | ✅ Verification only | Cannot modify ownership |
/// | Agent | ⚠️ Limited | Requires owner approval |
/// | Public | ❌ No access | View-only functions only |
///
/// ### Compliance Checks
/// - Recipient must pass KYC/AML verification
/// - Property must have verified badges (if required)
/// - Transaction must meet jurisdiction thresholds
///
/// ### Rate Limiting
/// - Max 100 properties per account
/// - Max 10 transfers per day per account
/// - Cooldown period: 60 seconds between operations
///
/// ### Audit Trail
/// All operations logged with:
/// - Caller account
/// - Timestamp
/// - Transaction hash
/// - Operation parameters
```

---

## Version History

### Changelog Format

```rust
/// ## Version History
///
/// ### v1.2.0 (Current)
/// - Added fractional ownership support
/// - Enhanced compliance checks
/// - Gas optimization (-15% average cost)
///
/// ### v1.1.0
/// - Added badge verification system
/// - Improved error messages
/// - Added event versioning
///
/// ### v1.0.0
/// - Initial implementation
/// - Core property registration
/// - Basic escrow functionality
```

---

## Cross-Reference Standards

### Linking Related Items

Help developers navigate the API:

```rust
/// ## Related Functions
///
/// ### See Also
/// - [`update_metadata`](crate::Contract::update_metadata) - Modify property details
/// - [`transfer_property`](crate::Contract::transfer_property) - Change ownership
/// - [`get_property`](crate::Contract::get_property) - Query property info
///
/// ### Complementary Operations
/// 1. After registering: [`attach_document`](crate::Contract::attach_document)
/// 2. Before transferring: [`verify_compliance`](crate::Contract::verify_compliance)
/// 3. For valuation: [`update_valuation`](crate::Contract::update_valuation)
///
/// ### Trait Implementations
/// - Implements [`PropertyRegistryTrait::register`](crate::traits::PropertyRegistryTrait::register)
/// - Part of [`IPropertyRegistry`](crate::traits::IPropertyRegistry) interface
```

---

## Documentation Quality Checklist

Before marking documentation complete, verify:

### Content Quality
- [ ] Every public function has documentation
- [ ] All parameters described with constraints
- [ ] All return values explained
- [ ] All error variants documented
- [ ] At least one example per function
- [ ] Edge cases mentioned

### Format Quality
- [ ] Consistent section ordering
- [ ] Proper rustdoc syntax
- [ ] Working code examples
- [ ] Correct cross-references
- [ ] No broken links
- [ ] Proper markdown formatting

### Usability Quality
- [ ] Examples are copy-paste ready
- [ ] Real-world values used
- [ ] Common pitfalls highlighted
- [ ] Recovery steps provided
- [ ] Gas costs estimated
- [ ] Security requirements clear

### Maintenance Quality
- [ ] Version history tracked
- [ ] Deprecation notices added
- [ ] Migration guides for breaking changes
- [ ] Last updated date included
- [ ] Maintainer contact info

---

## Tooling & Automation

### rustdoc Generation

Generate HTML documentation:
```bash
# Generate documentation
cargo doc --no-deps --open

# Generate with private items (for internal review)
cargo doc --document-private-items --no-deps --open

# Check documentation links
cargo doc --no-deps
```

### Documentation Tests

Run examples as tests:
```bash
# Test all documentation examples
cargo test --doc

# Test specific module docs
cargo test --doc propchain_contracts
```

### Linting

Check documentation quality:
```bash
# Check for missing docs
cargo rustdoc -- -W missing_docs

# Enforce documentation style
cargo clippy -- -W clippy::missing_errors_doc
```

---

## Migration Guide Template

When API changes, provide migration path:

```markdown
# API Migration Guide: v1.x to v2.0

## Breaking Changes

### Function Signature Changes
**Old**: `fn register_property(metadata: PropertyMetadata)`
**New**: `fn register_property_v2(metadata: PropertyMetadataV2, compliance_proof: Option<ZkProof>)`

**Migration**:
```rust
// Before
let id = registry.register_property(old_metadata)?;

// After
let new_metadata = migrate_metadata(old_metadata);
let id = registry.register_property_v2(new_metadata, None)?;
```

### Error Code Changes
**Removed**: `Error::OldError`
**Added**: `Error::NewError`

Update error handling:
```rust
// Before
if matches!(err, Error::OldError) { ... }

// After
if matches!(err, Error::NewError) { ... }
```

## Deprecation Timeline

- **v1.x**: Current version (supported until 2024-Q2)
- **v2.0**: Released 2024-Q1 (migration period starts)
- **v2.1**: Old APIs emit warnings
- **v3.0**: Old APIs removed (planned 2024-Q4)
```

---

## Conclusion

Following these standards ensures:
1. **Consistency** across all PropChain contracts
2. **Completeness** of API documentation
3. **Usability** for developers
4. **Maintainability** for the core team

All new code must follow these standards. Existing code should be updated during regular maintenance cycles.

**Related Documents**:
- [Architecture Documentation](./ARCHITECTURE_INDEX.md)
- [Contributing Guide](../CONTRIBUTING.md)
- [Rust Documentation Guidelines](https://doc.rust-lang.org/rustdoc/)
