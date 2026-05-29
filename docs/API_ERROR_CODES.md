# PropChain Error Codes Documentation

## Overview

This document provides comprehensive documentation for all error types in the PropChain smart contract system. Each error includes trigger conditions, common scenarios, recovery steps, and examples.

---

## Error Taxonomy

PropChain errors are organized into categories:

1. **Authorization Errors** - Permission and access control failures
2. **Validation Errors** - Input validation and data integrity failures  
3. **Compliance Errors** - Regulatory and KYC/AML failures
4. **Operational Errors** - Contract operation failures
5. **System Errors** - Infrastructure and dependency failures
6. **State Errors** - Invalid state or state transition failures

---

## Authorization Errors

### `Error::Unauthorized`

```rust
/// # Unauthorized Access
///
/// ## Description
/// The caller does not have permission to perform the requested operation.
/// This is the most common authorization failure.
///
/// ## Trigger Conditions
/// - Caller is not contract admin
/// - Caller lacks required role (Verifier, Agent, etc.)
/// - Caller is not property owner
/// - Required approval not obtained
///
/// ## Common Scenarios
/// 
/// ### Scenario 1: Non-Admin Configuration Change
/// **Context**: User tries to set oracle address
/// ```rust,ignore
/// // This will fail - caller is not admin
/// let result = contract.set_oracle(new_oracle); // Caller: regular user
/// assert!(matches!(result, Err(Error::Unauthorized)));
/// ```
/// **Solution**: Only admin can call configuration methods
///
/// ### Scenario 2: Unauthorized Property Transfer
/// **Context**: Non-owner attempts to transfer property
/// ```rust,ignore
/// // This will fail - caller is not owner
/// let result = contract.transfer_property(to, token_id); // Caller: non-owner
/// assert!(matches!(result, Err(Error::Unauthorized)));
/// ```
/// **Solution**: Property owner must initiate transfer
///
/// ### Scenario 3: Missing Role Assignment
/// **Context**: User without Verifier role tries to verify badge
/// ```rust,ignore
/// // This will fail - no Verifier role
/// let result = contract.verify_badge(property_id, badge_type);
/// assert!(matches!(result, Err(Error::Unauthorized)));
/// ```
/// **Solution**: Admin must grant Verifier role first
///
/// ## Recovery Steps
/// 1. Identify required role/permission for operation
/// 2. Check caller's current roles via [`get_role`](crate::AccessControl::get_role)
/// 3. Request role assignment from admin if needed
/// 4. Retry operation after permissions granted
///
/// ## HTTP Equivalent
/// `403 Forbidden`
///
/// ## Related Errors
/// - [`NotAuthorizedToPause`](crate::Error::NotAuthorizedToPause) - Specific to pause operations
/// - [`NotVerifier`](crate::Error::NotVerifier) - Badge verification specific
```

---

### `Error::NotAuthorizedToPause`

```rust
/// # Not Authorized to Pause Contract
///
/// ## Description
/// Caller attempted to pause the contract but lacks pause guardian or admin role.
///
/// ## Trigger Conditions
/// - Caller is not admin
/// - Caller is not a designated pause guardian
/// - Caller attempts `pause_contract()` without authorization
///
/// ## Common Scenarios
///
/// ### Scenario: Regular User Tries to Pause
/// **Context**: User notices issue and tries to emergency pause
/// ```rust,ignore
/// // This will fail - user not authorized
/// let result = contract.pause_contract("Emergency!".to_string(), None);
/// assert!(matches!(result, Err(Error::NotAuthorizedToPause)));
/// ```
/// **Solution**: Contact admin or pause guardians immediately
///
/// ## Recovery Steps
/// 1. Do NOT attempt to pause (unauthorized accounts cannot)
/// 2. Contact admin via governance channels
/// 3. Contact pause guardians directly if known
/// 4. Use emergency communication channels (Discord, email)
///
/// ## Prevention
/// - Identify pause guardians before emergencies
/// - Establish clear escalation procedures
/// - Maintain up-to-date contact information
///
/// ## HTTP Equivalent
/// `403 Forbidden` (specific to pause operations)
///
/// ## Related Errors
/// - [`Unauthorized`](crate::Error::Unauthorized) - General authorization failure
/// - [`AlreadyPaused`](crate::Error::AlreadyPaused) - Contract already paused
```

---

## Validation Errors

### `Error::InvalidMetadata`

```rust
/// # Invalid Property Metadata
///
/// ## Description
/// Property metadata provided is malformed, incomplete, or violates constraints.
///
/// ## Trigger Conditions
/// - Missing required fields (location, size, valuation)
/// - Field exceeds maximum length
/// - Valuation below minimum threshold
/// - Invalid format (e.g., malformed IPFS CID)
/// - Inconsistent data (e.g., negative size)
///
/// ## Common Scenarios
///
/// ### Scenario 1: Empty Location String
/// **Context**: Register property without location
/// ```rust,ignore
/// let metadata = PropertyMetadata {
///     location: "".to_string(), // INVALID - empty
///     size: 2000,
///     valuation: 500_000,
///     documents_url: "ipfs://...".to_string(),
/// };
/// let result = contract.register_property(metadata);
/// assert!(matches!(result, Err(Error::InvalidMetadata)));
/// ```
/// **Solution**: Provide valid location string (1-256 chars)
///
/// ### Scenario 2: Unrealistic Property Size
/// **Context**: Size value clearly erroneous
/// ```rust,ignore
/// let metadata = PropertyMetadata {
///     location: "123 Main St".to_string(),
///     size: 0, // INVALID - zero size
///     valuation: 500_000,
///     documents_url: "ipfs://...".to_string(),
/// };
/// let result = contract.register_property(metadata);
/// assert!(matches!(result, Err(Error::InvalidMetadata)));
/// ```
/// **Solution**: Size must be > 0 and <= 10,000,000 sq meters
///
/// ### Scenario 3: Valuation Too Low
/// **Context**: Valuation below minimum threshold
/// ```rust,ignore
/// let metadata = PropertyMetadata {
///     location: "123 Main St".to_string(),
///     size: 2000,
///     valuation: 100, // INVALID - below $10 minimum
///     documents_url: "ipfs://...".to_string(),
/// };
/// let result = contract.register_property(metadata);
/// assert!(matches!(result, Err(Error::InvalidMetadata)));
/// ```
/// **Solution**: Minimum valuation is 1,000 ($10.00 in cents)
///
/// ## Validation Rules
///
/// ### Location
/// - **Required**: Yes
/// - **Length**: 1-256 characters
/// - **Format**: Plain text street address
/// - **Example**: `"123 Main Street, Springfield, IL 62701"`
///
/// ### Size
/// - **Required**: Yes
/// - **Type**: u64
/// - **Range**: 1 - 10,000,000 square meters
/// - **Example**: `2000` (2,000 sq meters)
///
/// ### Valuation
/// - **Required**: Yes
/// - **Type**: u128
/// - **Minimum**: 1,000 (USD $10.00 in cents)
/// - **Maximum**: No limit (practical real estate values)
/// - **Example**: `500_000_000` (USD $5,000,000.00)
///
/// ### Documents URL
/// - **Required**: Recommended
/// - **Format**: IPFS CID or HTTPS URL
/// - **Max Length**: 2048 characters
/// - **Example**: `"ipfs://QmX7Zz9YvPqK8N3mR5wL2bT6cH4dF9gS1aE8uB7vC3nM2k"`
///
/// ## Recovery Steps
/// 1. Review validation rules above
/// 2. Validate metadata locally before submission
/// 3. Check each field against constraints
/// 4. Fix invalid fields
/// 5. Resubmit corrected metadata
///
/// ## Pre-validation Helper
/// ```rust,ignore
/// fn validate_metadata(metadata: &PropertyMetadata) -> Result<(), &'static str> {
///     if metadata.location.is_empty() || metadata.location.len() > 256 {
///         return Err("Location must be 1-256 characters");
///     }
///     if metadata.size == 0 || metadata.size > 10_000_000 {
///         return Err("Size must be 1-10,000,000 sq meters");
///     }
///     if metadata.valuation < 1_000 {
///         return Err("Minimum valuation is $10.00 (1,000 cents)");
///     }
///     if !metadata.documents_url.starts_with("ipfs://") && !metadata.documents_url.starts_with("https://") {
///         return Err("Documents URL must be IPFS or HTTPS");
///     }
///     Ok(())
/// }
/// ```
///
/// ## HTTP Equivalent
/// `400 Bad Request`
///
/// ## Related Errors
/// - [`PropertyNotFound`](crate::Error::PropertyNotFound) - Property doesn't exist
/// - [`OracleError`](crate::Error::OracleError) - Oracle validation failure
```

---

### `Error::PropertyNotFound`

```rust
/// # Property Not Found
///
/// ## Description
/// The specified property ID does not exist in the registry.
///
/// ## Trigger Conditions
/// - Property ID never registered
/// - Property ID out of range
/// - Typo in property ID
/// - Using deleted/archived property ID
///
/// ## Common Scenarios
///
/// ### Scenario: Querying Non-existent Property
/// **Context**: Check ownership of unregistered property
/// ```rust,ignore
/// // This will fail - property doesn't exist yet
/// let result = contract.get_owner(999_999); // Never registered
/// assert!(matches!(result, Err(Error::PropertyNotFound)));
/// ```
/// **Solution**: Verify property ID exists before operations
///
/// ### Scenario: Update Before Registration Complete
/// **Context**: Trying to update metadata immediately after registration
/// ```rust,ignore
/// // Race condition - registration still processing
/// let id = contract.register_property(metadata)?;
/// contract.update_metadata(id, new_metadata)? // May fail if async
/// ```
/// **Solution**: Wait for transaction confirmation
///
/// ## Recovery Steps
/// 1. Verify property ID is correct
/// 2. Check property exists: `contract.property_exists(id)`
/// 3. List registered properties: `contract.get_properties_by_owner(account)`
/// 4. If truly missing, register property first
///
/// ## Prevention
/// ```rust,ignore
/// // Always check existence before operations
/// if !contract.property_exists(property_id) {
///     return Err("Property does not exist");
/// }
/// // Safe to proceed
/// contract.update_metadata(property_id, metadata)?;
/// ```
///
/// ## HTTP Equivalent
/// `404 Not Found`
///
/// ## Related Errors
/// - [`InvalidMetadata`](crate::Error::InvalidMetadata) - Metadata issues
/// - [`EscrowNotFound`](crate::Error::EscrowNotFound) - Escrow-specific not found
```

---

## Compliance Errors

### `Error::NotCompliant`

```rust
/// # Compliance Check Failed
///
/// ## Description
/// The account does not meet regulatory compliance requirements (KYC/AML).
/// This error enforces real estate regulations and anti-money laundering rules.
///
/// ## Trigger Conditions
/// - Account not KYC verified
/// - AML check failed or expired
/// - Sanctions list match
/// - High-risk jurisdiction without enhanced due diligence
/// - GDPR consent not provided
/// - Compliance verification expired
///
/// ## Common Scenarios
///
/// ### Scenario 1: Unverified Account Purchase Attempt
/// **Context**: User tries to buy property without KYC
/// ```rust,ignore
/// // Buyer not KYC verified
/// let result = contract.transfer_property(buyer_account, token_id);
/// assert!(matches!(result, Err(Error::NotCompliant)));
/// ```
/// **Solution**: Complete KYC verification first
///
/// ### Scenario 2: Expired AML Check
/// **Context**: Previous KYC expired, needs renewal
/// ```rust,ignore
/// // KYC was done but expired 6 months ago
/// let result = contract.transfer_property(buyer_account, token_id);
/// assert!(matches!(result, Err(Error::NotCompliant)));
/// ```
/// **Solution**: Re-verify with updated documents
///
/// ### Scenario 3: Sanctions List Match
/// **Context**: Account on OFAC sanctions list
/// ```rust,ignore
/// // Account flagged on sanctions list
/// let result = contract.transfer_property(sanctioned_account, token_id);
/// assert!(matches!(result, Err(Error::NotCompliant)));
/// ```
/// **Solution**: Cannot resolve - sanctioned accounts permanently blocked
///
/// ### Scenario 4: High-Risk Jurisdiction
/// **Context**: User from high-risk country without enhanced DD
/// ```rust,ignore
/// // User from high-risk jurisdiction, standard KYC insufficient
/// let result = contract.transfer_property(high_risk_user, token_id);
/// assert!(matches!(result, Err(Error::NotCompliant)));
/// ```
/// **Solution**: Complete enhanced due diligence process
///
/// ## Compliance Requirements by Jurisdiction
///
/// ### Tier 1: Low Risk (Standard KYC)
/// **Countries**: USA, UK, EU, Canada, Australia, Japan, Singapore
/// **Requirements**:
/// - Government ID verification
/// - Proof of address
/// - Basic AML screening
/// **Validity**: 2 years
///
/// ### Tier 2: Medium Risk (Enhanced KYC)
/// **Countries**: Most G20 nations, developed economies
/// **Requirements**:
/// - All Tier 1 requirements
/// - Source of funds declaration
/// - Enhanced AML screening
/// **Validity**: 1 year
///
/// ### Tier 3: High Risk (Enhanced Due Diligence)
/// **Countries**: Offshore centers, high-risk jurisdictions
/// **Requirements**:
/// - All Tier 2 requirements
/// - In-person verification or video call
/// - Additional documentation
/// - Ongoing monitoring
/// **Validity**: 6 months
///
/// ## Recovery Steps
///
/// ### For Standard KYC Failure
/// 1. Submit KYC application via compliance portal
/// 2. Upload required documents:
///    - Government-issued ID (passport, driver's license)
///    - Proof of address (utility bill, bank statement)
///    - Selfie with ID (for biometric verification)
/// 3. Wait for verification (typically 24-48 hours)
/// 4. Receive compliance certificate
/// 5. Retry property operation
///
/// ### For AML Failure
/// 1. Review AML rejection reason
/// 2. Provide additional documentation if possible
/// 3. Appeal decision if erroneous
/// 4. Consider legal counsel for complex cases
///
/// ### For Sanctions Match
/// **CRITICAL**: This is usually permanent
/// 1. Verify identity match (could be false positive)
/// 2. If true match, cannot proceed legally
/// 3. Consult legal counsel
/// 4. No technical solution available
///
/// ## Example: Complete KYC Flow
/// ```rust,ignore
/// // Step 1: Check compliance status
/// let is_compliant = contract.check_account_compliance(account)?;
///
/// if !is_compliant {
///     // Step 2: Direct user to KYC provider
///     let kyc_provider = get_kyc_provider();
///     kyc_provider.submit_verification(account, documents)?;
///     
///     // Step 3: Wait for approval (off-chain)
///     // Step 4: Poll compliance status
///     while !contract.check_account_compliance(account)? {
///         sleep(Duration::from_secs(3600)); // Check hourly
///     }
///     
///     // Step 5: Proceed with property operation
///     contract.transfer_property(buyer, token_id)?;
/// }
/// ```
///
/// ## HTTP Equivalent
/// `422 Unprocessable Entity`
///
/// ## Related Errors
/// - [`ComplianceCheckFailed`](crate::Error::ComplianceCheckFailed) - Registry call failed
/// - [`Unauthorized`](crate::Error::Unauthorized) - Access control failure
///
/// ## External Resources
/// - [KYC Provider Documentation](https://docs.kyc-provider.com)
/// - [AML Screening Guide](https://aml-compliance.org)
/// - [Sanctions Lists Search](https://sanctionssearch.ofac.treas.gov)
```

---

### `Error::ComplianceCheckFailed`

```rust
/// # Compliance Registry Call Failed
///
/// ## Description
/// The call to the compliance registry contract failed technically.
/// Different from `NotCompliant` - this indicates infrastructure failure, not compliance failure.
///
/// ## Trigger Conditions
/// - Compliance registry contract not deployed
/// - Registry contract reverted during call
/// - Gas limit exceeded during compliance check
/// - Registry interface mismatch (version incompatibility)
///
/// ## Common Scenarios
///
/// ### Scenario: Registry Not Configured Yet
/// **Context**: Contract tries to check compliance but registry address not set
/// ```rust,ignore
/// // Compliance registry not configured
/// contract.set_compliance_registry(None)?;
/// let result = contract.register_property(metadata);
/// // May fail when it tries to verify owner compliance
/// ```
/// **Solution**: Configure compliance registry first
///
/// ## Difference from NotCompliant
/// | Aspect | `ComplianceCheckFailed` | `NotCompliant` |
/// |--------|-------------------------|----------------|
/// | **Meaning** | Technical failure | Compliance failure |
/// | **Cause** | Infrastructure issue | User not verified |
/// | **Resolution** | Fix infrastructure | User completes KYC |
/// | **Frequency** | Rare (system bug) | Common (user action) |
///
/// ## Recovery Steps
/// 1. Verify compliance registry is configured: `contract.get_compliance_registry()`
/// 2. Check registry contract is deployed and operational
/// 3. Ensure interface compatibility
/// 4. Increase gas limit if needed
/// 5. Retry operation
///
/// ## HTTP Equivalent
/// `502 Bad Gateway`
///
/// ## Related Errors
/// - [`NotCompliant`](crate::Error::NotCompliant) - Actual compliance failure
/// - [`OracleError`](crate::Error::OracleError) - Similar cross-contract failure
```

---

## Operational Errors

### `Error::EscrowNotFound`

```rust
/// # Escrow Not Found
///
/// ## Description
/// The specified escrow ID does not exist or has been closed.
///
/// ## Trigger Conditions
/// - Escrow ID never created
/// - Escrow already completed/closed
/// - Typo in escrow ID
/// - Using archived escrow reference
///
/// ## Common Scenarios
///
/// ### Scenario: Query Completed Escrow
/// **Context**: Check status of old completed escrow
/// ```rust,ignore
/// // Escrow was completed and archived
/// let result = contract.get_escrow_info(old_escrow_id);
/// assert!(matches!(result, Err(Error::EscrowNotFound)));
/// ```
/// **Solution**: Escrows may be archived after completion - check historical events
///
/// ## Recovery Steps
/// 1. Verify escrow ID is correct
/// 2. Check escrow exists: `contract.escrow_exists(id)`
/// 3. Query escrow creation events for valid IDs
/// 4. If archived, retrieve from historical events instead
///
/// ## HTTP Equivalent
/// `404 Not Found`
///
/// ## Related Errors
/// - [`PropertyNotFound`](crate::Error::PropertyNotFound) - Property doesn't exist
/// - [`EscrowAlreadyReleased`](crate::Error::EscrowAlreadyReleased) - Escrow completed
```

---

### `Error::EscrowAlreadyReleased`

```rust
/// # Escrow Already Released
///
/// ## Description
/// Attempted to release or modify an escrow that has already been completed.
///
/// ## Trigger Conditions
/// - Double-release attempt
/// - Modifying completed escrow
/// - Refund after successful release
///
/// ## Common Scenarios
///
/// ### Scenario: Duplicate Release Transaction
/// **Context**: Same release submitted twice
/// ```rust,ignore
/// // First release succeeds
/// contract.release_escrow(escrow_id)?;
/// 
/// // Second attempt (maybe re-org or retry) fails
/// let result = contract.release_escrow(escrow_id);
/// assert!(matches!(result, Err(Error::EscrowAlreadyReleased)));
/// ```
/// **Solution**: Check escrow status before release
///
/// ## Prevention
/// ```rust,ignore
/// // Idempotent release pattern
/// let escrow = contract.get_escrow(escrow_id)?;
/// if escrow.released {
///     // Already released - safe to skip
///     return Ok(());
/// }
/// // Safe to release
/// contract.release_escrow(escrow_id)?;
/// ```
///
/// ## HTTP Equivalent
/// `409 Conflict`
///
/// ## Related Errors
/// - [`EscrowNotFound`](crate::Error::EscrowNotFound) - Escrow doesn't exist
```

---

## System Errors

### `Error::OracleError`

```rust
/// # Oracle Operation Failed
///
/// ## Description
/// Interaction with the price oracle contract failed.
/// This is a generic wrapper for oracle-related failures.
///
/// ## Trigger Conditions
/// - Oracle contract not configured
/// - Oracle call reverted
/// - Oracle returned invalid data
/// - Cross-contract call failure
/// - Oracle manipulation detected
///
/// ## Common Scenarios
///
/// ### Scenario 1: Oracle Not Configured
/// **Context**: Try to update valuation without oracle setup
/// ```rust,ignore
/// // Oracle address not set
/// let result = contract.update_valuation_from_oracle(property_id);
/// assert!(matches!(result, Err(Error::OracleError)));
/// ```
/// **Solution**: Configure oracle first: `contract.set_oracle(oracle_address)`
///
/// ### Scenario 2: Oracle Call Reverted
/// **Context**: Oracle contract has internal error
/// ```rust,ignore
/// // Oracle experiencing issues
/// let result = contract.get_valuation(property_id);
/// assert!(matches!(result, Err(Error::OracleError)));
/// ```
/// **Solution**: Check oracle contract health, wait for resolution
///
/// ### Scenario 3: Stale Oracle Data
/// **Context**: Oracle data too old to use safely
/// ```rust,ignore
/// // Last update was 30 days ago
/// let valuation = contract.get_valuation(property_id)?;
/// if valuation.timestamp < now - MAX_AGE {
///     return Err(Error::OracleError); // Treat as error
/// }
/// ```
/// **Solution**: Trigger oracle update before use
///
/// ## Recovery Steps
/// 1. Verify oracle is configured: `contract.oracle()`
/// 2. Check oracle contract is operational
/// 3. Validate oracle data freshness
/// 4. Use fallback valuation if available
/// 5. Contact oracle operator if persistent
///
/// ## Fallback Strategy
/// ```rust,ignore
/// // Graceful degradation when oracle fails
/// match contract.update_valuation_from_oracle(property_id) {
///     Ok(_) => println!("Valuation updated"),
///     Err(Error::OracleError) => {
///         // Use last known good value
///         let last_valuation = get_cached_valuation(property_id);
///         apply_conservative_adjustment(last_valuation);
///     }
///     Err(e) => return Err(e),
/// }
/// ```
///
/// ## HTTP Equivalent
/// `502 Bad Gateway`
///
/// ## Related Errors
/// - [`ComplianceCheckFailed`](crate::Error::ComplianceCheckFailed) - Similar integration failure
/// - [`InvalidMetadata`](crate::Error::InvalidMetadata) - Could result from bad oracle data
```

---

### `Error::ExternalDependencyUnavailable`

```rust
/// # External Dependency Circuit Breaker Open
///
/// ## Description
/// The registry has temporarily blocked calls to an external dependency because
/// its circuit breaker is open.
///
/// ## Trigger Conditions
/// - Admin manually tripped the breaker
/// - Recent failures crossed the configured threshold
/// - Cooldown window has not yet elapsed
///
/// ## Common Scenarios
///
/// ### Scenario 1: Oracle Temporarily Isolated
/// **Context**: Valuation updates are blocked while oracle issues are investigated
/// ```rust,ignore
/// let result = contract.update_valuation_from_oracle(property_id);
/// assert!(matches!(result, Err(Error::ExternalDependencyUnavailable)));
/// ```
///
/// ### Scenario 2: Compliance Registry Held Open
/// **Context**: Registration and transfer flows fail fast instead of repeatedly
/// calling an unhealthy compliance contract
/// ```rust,ignore
/// let result = contract.register_property(metadata);
/// assert!(matches!(result, Err(Error::ExternalDependencyUnavailable)));
/// ```
///
/// ## Recovery Steps
/// 1. Inspect breaker state with `get_external_dependency_breaker(...)`
/// 2. Restore the unhealthy downstream contract or service
/// 3. Wait for cooldown, or manually clear it with `reset_external_dependency_breaker(...)`
/// 4. Re-run the blocked operation
///
/// ## Operational Guidance
/// - Use `trip_external_dependency_breaker(...)` for manual emergency isolation
/// - Keep `get_dynamic_fee(...)` callers tolerant of a `0` fallback when the fee-manager breaker is open
/// - Monitor repeated trips as an indicator of downstream instability
/// ```

---

### `Error::ContractPaused`

```rust
/// # Contract Paused
///
/// ## Description
/// The contract is currently paused and non-critical operations are suspended.
/// This is a safety mechanism for emergencies or upgrades.
///
/// ## Trigger Conditions
/// - Admin activated pause
/// - Pause guardian triggered emergency pause
/// - Automatic pause from circuit breaker
/// - Time-based pause not yet expired
///
/// ## Common Scenarios
///
/// ### Scenario: Operations During Emergency Pause
/// **Context**: User tries to register property during pause
/// ```rust,ignore
/// // Contract paused due to security concern
/// let result = contract.register_property(metadata);
/// assert!(matches!(result, Err(Error::ContractPaused)));
/// ```
/// **Solution**: Wait for contract to resume
///
/// ### Scenario: Auto-Resume Time Not Reached
/// **Context**: Pause had auto-resume time, but time hasn't elapsed
/// ```rust,ignore
/// // Pause set with 24-hour delay
/// let result = contract.some_operation();
/// // Still within pause period
/// assert!(matches!(result, Err(Error::ContractPaused)));
/// ```
/// **Solution**: Wait until auto_resume_at timestamp
///
/// ## Operations Allowed During Pause
/// 
/// ### ✅ Permitted (Read-Only)
/// - View functions (`get_property`, `get_owner`)
/// - Health checks (`health_check`, `ping`)
/// - Compliance queries
/// - Event emission reads
///
/// ### ❌ Blocked (State-Changing)
/// - Property registration
/// - Property transfers
/// - Escrow operations
/// - Metadata updates
/// - Approval grants
///
/// ## Recovery Steps
/// 1. Check pause status: `contract.health_check().is_paused`
/// 2. Review pause reason (if provided)
/// 3. Check auto-resume time: `pause_info.auto_resume_at`
/// 4. Monitor admin announcements
/// 5. Resume operations after unpause
///
/// ## Monitoring Pause Status
/// ```rust,ignore
/// // Check if contract is operational
/// let health = contract.health_check()?;
/// if health.is_paused {
///     println!("Contract paused at: {:?}", health.paused_at);
///     println!("Reason: {:?}", health.pause_reason);
///     
///     if let Some(resume_time) = health.auto_resume_at {
///         let now = env().block_timestamp();
///         if now >= resume_time {
///             println!("Can request auto-resume");
///         } else {
///             println!("Wait {} more seconds", resume_time - now);
///         }
///     }
/// }
/// ```
///
/// ## HTTP Equivalent
/// `503 Service Unavailable`
///
/// ## Related Errors
/// - [`AlreadyPaused`](crate::Error::AlreadyPaused) - Redundant pause attempt
/// - [`NotPaused`](crate::Error::NotPaused) - Expected pause but isn't
/// - [`NotAuthorizedToPause`](crate::Error::NotAuthorizedToPause) - Unauthorized pause attempt
```

---

## Error Handling Best Practices

### 1. Specific Error Handling

```rust,ignore
// ❌ BAD: Generic error handling
match contract.operation() {
    Ok(result) => process(result),
    Err(e) => log_error(e), // Loses specificity
}

// ✅ GOOD: Handle specific errors
match contract.operation() {
    Ok(result) => process(result),
    Err(Error::NotCompliant) => guide_to_kyc(),
    Err(Error::InvalidMetadata) => fix_metadata(),
    Err(Error::ContractPaused) => wait_and_retry(),
    Err(e) => escalate(e),
}
```

### 2. Error Context

```rust,ignore
// Add context to errors
match contract.transfer_property(to, token_id) {
    Ok(_) => success(),
    Err(Error::NotCompliant) => {
        eprintln!("Transfer failed: recipient {} not compliant", to);
        eprintln!("Action required: Complete KYC at https://kyc.propchain.io");
    }
    Err(e) => eprintln!("Unexpected error: {:?}", e),
}
```

### 3. Retry Logic

```rust,ignore
// Retry with backoff for transient errors
async fn operation_with_retry<F>(operation: F) -> Result<(), Error>
where
    F: Fn() -> Result<(), Error>,
{
    let mut attempts = 0;
    loop {
        match operation() {
            Ok(_) => return Ok(()),
            Err(Error::ContractPaused) if attempts < 3 => {
                attempts += 1;
                sleep(backoff(attempts)).await;
            }
            Err(e) => return Err(e),
        }
    }
}
```

### 4. Error Aggregation

```rust,ignore
// Collect multiple errors for batch operations
let mut errors = Vec::new();
for property in properties {
    if let Err(e) = contract.register_property(property) {
        errors.push((property.id, e));
    }
}

if !errors.is_empty() {
    eprintln!("{} registrations failed:", errors.len());
    for (id, err) in errors {
        eprintln!("  Property {}: {:?}", id, err);
    }
}
```

---

## Error Code Reference Table

| Code | Name | HTTP Equivalent | Category | Severity |
|------|------|----------------|----------|----------|
| 1 | `Unauthorized` | 403 Forbidden | Authorization | High |
| 2 | `PropertyNotFound` | 404 Not Found | Validation | Medium |
| 3 | `InvalidMetadata` | 400 Bad Request | Validation | Medium |
| 4 | `NotCompliant` | 422 Unprocessable | Compliance | High |
| 5 | `ComplianceCheckFailed` | 502 Bad Gateway | System | High |
| 6 | `EscrowNotFound` | 404 Not Found | Operational | Low |
| 7 | `EscrowAlreadyReleased` | 409 Conflict | Operational | Low |
| 8 | `OracleError` | 502 Bad Gateway | System | Medium |
| 9 | `ContractPaused` | 503 Service Unavailable | Operational | Medium |
| 10 | `AlreadyPaused` | 409 Conflict | Operational | Low |
| 11 | `NotPaused` | 409 Conflict | Operational | Low |
| 12 | `NotAuthorizedToPause` | 403 Forbidden | Authorization | High |
| 13 | `BadgeNotFound` | 404 Not Found | Operational | Low |
| 14 | `NotVerifier` | 403 Forbidden | Authorization | Medium |
| 15 | `ReentrantCall` | 400 Bad Request | Security | Critical |

---

## Conclusion

Understanding and properly handling these errors is crucial for building robust applications on PropChain. This documentation should be used alongside the main API documentation for complete integration guidance.

**Related Documents**:
- [API Documentation Standards](./API_DOCUMENTATION_STANDARDS.md)
- [Contract API Documentation](./contracts.md)
- [Integration Guide](./integration.md)
- [Troubleshooting FAQ](./troubleshooting-faq.md)
