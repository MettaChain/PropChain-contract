# Insurance Features - Usage Guide & Examples

## Quick Start Guide

### Risk Assessment Model (Task #254)

#### Example 1: Assessing a Low-Risk Property

```rust
// Create a comprehensive risk assessment for a safe, modern property
let (risk_id, premium_multiplier) = contract.assess_property_risk_comprehensive(
    property_id: 1,
    property_age_years: 5,              // Very new property
    property_value: 5_000_000_000_000u128, // High value property
    location_code: "premium_safe_zone".into(),
    construction_type: "steel_frame".into(),
    has_security_system: true,
    has_fire_extinguisher: true,
    has_alarm_system: true,
    owner_age_years: 45,                // Middle-aged owner
    years_as_owner: 15,                 // Stable owner
)?;

// Result: risk_id = 1, premium_multiplier = 5000 (0.5x - 50% discount)
// Risk Score: ~200 (VeryLow)
```

#### Example 2: Assessing a High-Risk Property

```rust
// Assess an older property in a risky area with minimal safety
let (risk_id, premium_multiplier) = contract.assess_property_risk_comprehensive(
    property_id: 2,
    property_age_years: 80,             // Very old property
    property_value: 500_000_000_000u128, // Lower value
    location_code: "high_risk_zone".into(),
    construction_type: "wood_frame".into(),
    has_security_system: false,
    has_fire_extinguisher: false,
    has_alarm_system: false,
    owner_age_years: 25,                // Young owner
    years_as_owner: 1,                  // New owner
)?;

// Result: risk_id = 2, premium_multiplier = 25000 (2.5x - 150% premium)
// Risk Score: ~800 (VeryHigh)
```

#### Example 3: Updating Risk Assessment

```rust
// Owner upgrades property security features
let (new_score, new_multiplier) = contract.update_property_risk_assessment(
    risk_id: 2,
    property_age_years: 81,
    has_security_system: true,        // Just added!
    has_fire_extinguisher: true,      // Just added!
    has_alarm_system: true,           // Just added!
)?;

// Result: new_score = 550 (Medium), new_multiplier = 10000 (1.0x - normal)
// Risk decreased by 250 points due to safety improvements
```

#### Example 4: Retrieving Risk Model

```rust
let risk_model = contract.get_property_risk_model(risk_id)?;

// Access detailed components:
println!("Location Risk: {}", risk_model.location_risk_score);
println!("Construction Risk: {}", risk_model.construction_risk_score);
println!("Age Risk: {}", risk_model.age_risk_score);
println!("Ownership Risk: {}", risk_model.ownership_risk_score);
println!("Claims History Risk: {}", risk_model.claims_history_score);
println!("Safety Features Score: {}", risk_model.safety_features_score);
println!("Overall Score: {}", risk_model.overall_risk_score);
println!("Risk Level: {:?}", risk_model.final_risk_level);
println!("Premium Multiplier: {}x", risk_model.premium_multiplier as f64 / 10000.0);
```

---

### Fraud Detection System (Task #258)

#### Example 1: Assessing a Normal Claim

```rust
// Customer submits a reasonable claim
let claim_id = contract.submit_claim(
    policy_id: 1,
    claim_amount: 100_000_000_000u128,  // Within normal range
    description: "Water damage from burst pipe in kitchen. Occurred on Tuesday morning.".into(),
    evidence_url: "ipfs://Qm...evidence_photos".into(),
)?;

// Perform fraud assessment
let (assessment_id, fraud_score, requires_review) = 
    contract.assess_claim_fraud_risk(claim_id, policy_id)?;

// Result: fraud_score = 150, requires_review = false
// Risk Level: VeryLow - Normal claim, no fraud indicators
```

#### Example 2: Detecting Suspicious Claim

```rust
// Suspicious claim: Too high, minimal documentation, weekend submission
let claim_id = contract.submit_claim(
    policy_id: 2,
    claim_amount: 950_000_000_000u128,  // 95% of coverage (claim stuffing)
    description: "x".into(),             // Suspiciously short
    evidence_url: "".into(),             // No evidence
)?;

let (assessment_id, fraud_score, requires_review) = 
    contract.assess_claim_fraud_risk(claim_id, policy_id)?;

// Result: fraud_score = 750, requires_review = true
// Risk Level: VeryHigh
// Detected Indicators:
//   - ExcessiveCoverageRatio (95% of coverage)
//   - Misrepresentation (short description, no evidence)
//   - SuspiciousTimingPattern (weekend submission)
```

#### Example 3: Detecting Multiple Claims Pattern

```rust
// Customer submits 3rd claim in 30 days
let claim_id = contract.submit_claim(
    policy_id: 3,
    claim_amount: 300_000_000_000u128,  // Reasonable amount
    description: "Another claim".into(),
    evidence_url: "ipfs://Qm...evidence".into(),
)?;

let (assessment_id, fraud_score, requires_review) = 
    contract.assess_claim_fraud_risk(claim_id, policy_id)?;

// Result: fraud_score = 300, requires_review = true
// Detected Indicators:
//   - MultipleClaimsShortPeriod (3 claims in 30 days)
// Even though amount is reasonable, frequency is suspicious
```

#### Example 4: Retrieving Fraud Assessment

```rust
let assessment = contract.get_fraud_assessment(assessment_id)?;

// Access detailed information:
println!("Assessment ID: {}", assessment.assessment_id);
println!("Claim ID: {}", assessment.claim_id);
println!("Fraud Score: {}", assessment.fraud_score);
println!("Fraud Level: {:?}", assessment.fraud_level);
println!("Requires Review: {}", assessment.requires_manual_review);
println!("Detected Indicators: {}", assessment.detected_indicators.len());

for indicator in assessment.detected_indicators {
    println!("  - {:?}", indicator);
}

println!("Claim Amount: {}", assessment.claim_amount);
println!("Expected Range: {} - {}", 
    assessment.expected_amount_range.0,
    assessment.expected_amount_range.1);
println!("Time Since Last Claim: {:?}", assessment.time_since_last_claim);
println!("Similar Claims Count: {}", assessment.similar_claims_count);
```

#### Example 5: Checking Fraud Statistics

```rust
let stats = contract.get_fraud_detection_stats();

if let Some(stats) = stats {
    println!("Total Assessments: {}", stats.total_assessments);
    println!("High Risk Claims: {}", stats.high_risk_claims);
    println!("Rejected Fraud Claims: {}", stats.rejected_fraud_claims);
    println!("Patterns Detected: {}", stats.patterns_detected);
    println!("Average Fraud Score: {}", stats.average_fraud_score);
    println!("False Positives: {}", stats.false_positive_count);
}
```

---

## Integration Workflow

### Step 1: Risk Assessment Setup
```
1. Admin calls assess_property_risk_comprehensive()
   ↓
2. System calculates risk scores
   ↓
3. PropertyRiskModel created and stored
   ↓
4. PropertyRiskModelCreated event emitted
```

### Step 2: Policy Creation
```
1. get_property_risk_model() retrieves risk model
   ↓
2. calculate_premium() uses premium_multiplier
   ↓
3. create_policy() with calculated premium
   ↓
4. Policy premium reflects accurate risk assessment
```

### Step 3: Claim Submission
```
1. submit_claim() creates claim
   ↓
2. ClaimSubmitted event emitted
```

### Step 4: Fraud Assessment
```
1. Admin/Assessor calls assess_claim_fraud_risk()
   ↓
2. 8 fraud indicators analyzed
   ↓
3. FraudRiskAssessment created
   ↓
4. FraudRiskAssessmentCreated event emitted
   ↓
5. If high risk: HighFraudRiskDetected event
   ↓
6. Fraud stats updated
```

### Step 5: Claim Processing
```
1. process_claim() reviews fraud assessment
   ↓
2. If fraud_score > threshold: require manual review
   ↓
3. approve_claim() or reject with reason
   ↓
4. Payout executed for approved claims
```

---

## Thresholds & Constants

### Risk Assessment
| Threshold | Score Range | Multiplier | Interpretation |
|-----------|-------------|-----------|-----------------|
| VeryLow | 0-200 | 0.5x | 50% discount |
| Low | 201-400 | 0.75x | 25% discount |
| Medium | 401-600 | 1.0x | Normal price |
| High | 601-800 | 1.5x | 50% premium |
| VeryHigh | 801+ | 2.5x | 150% premium |

### Fraud Detection
| Threshold | Score Range | Action |
|-----------|-------------|--------|
| VeryLow Risk | 0-250 | Auto-approve (minimal checks) |
| Low Risk | 251-450 | Standard review |
| Medium Risk | 451-600 | Enhanced review |
| High Risk | 601-800 | Manual review required |
| VeryHigh Risk | 801+ | Manual review + flagging |

### Fraud Indicators Scoring
| Indicator | Max Points | Condition |
|-----------|-----------|-----------|
| Multiple Claims | 300 | 3+ claims in 30 days |
| Anomalous Amount | 300 | Claim 150%+ of average |
| Suspicious Timing | 200 | Weekend/holiday submission |
| Excessive Coverage | 250 | Claim >85% of coverage |
| Historical Pattern | 400 | High claim history |
| Misrepresentation | 300 | Poor documentation |
| Fraud Network | 400 | Associated fraud accounts |
| Duplicate Patterns | 400 | Similar to known fraud |

---

## Authorization & Access Control

### Risk Assessment
- **execute**: Admin only
- **retrieve**: Anyone
- **authorize_oracle()**: Admin grants oracle access
- **Assessor Role**: Can perform assessments

### Fraud Detection
- **execute**: Admin or authorized assessor
- **retrieve**: Anyone
- **statistics**: Public access

---

## Event Monitoring

### Risk Assessment Events
```rust
PropertyRiskModelCreated {
    risk_id: u64,
    property_id: u64,
    overall_risk_score: u32,
    final_risk_level: RiskLevel,
    premium_multiplier: u32,
    timestamp: u64,
}

PropertyRiskModelUpdated {
    risk_id: u64,
    property_id: u64,
    new_risk_score: u32,
    new_risk_level: RiskLevel,
    timestamp: u64,
}
```

### Fraud Detection Events
```rust
FraudRiskAssessmentCreated {
    assessment_id: u64,
    claim_id: u64,
    policyholder: AccountId,
    fraud_score: u32,
    fraud_level: RiskLevel,
    requires_manual_review: bool,
    timestamp: u64,
}

HighFraudRiskDetected {
    claim_id: u64,
    policyholder: AccountId,
    fraud_score: u32,
    indicator_count: u32,
    timestamp: u64,
}

FraudPatternDetected {
    claim_id: u64,
    indicator_type: String,
    risk_increase: u32,
    timestamp: u64,
}
```

---

## Best Practices

### For Risk Assessments
1. Reassess properties every 1-2 years
2. Update immediately after major renovations
3. Monitor trends in risk scores
4. Use historical data for validation

### For Fraud Detection
1. Review high-risk claims manually
2. Track false positive rates
3. Update fraud patterns based on outcomes
4. Monitor multiple claims from same policyholder
5. Cross-reference with claims database

### General
1. Maintain accurate property data
2. Keep evidence and documentation
3. Monitor all events for audit trails
4. Regular statistics review
5. Update thresholds based on experience

---

## Error Handling

```rust
// Risk Assessment Errors
RiskAssessmentNotFound     // Model doesn't exist
RiskAssessmentExpired      // Model validity period passed
InvalidRiskFactors         // Invalid input parameters
RiskModelGenerationFailed  // Calculation error

// Fraud Detection Errors
FraudAssessmentNotFound    // Assessment doesn't exist
HighFraudRisk              // Score exceeds threshold
FraudPatternNotFound       // Pattern doesn't exist
InvalidFraudIndicator      // Unknown indicator type

// General Errors
Unauthorized               // Insufficient permissions
PolicyNotFound             // Policy doesn't exist
ClaimNotFound              // Claim doesn't exist
```

---

## Testing Examples

See `contracts/insurance/src/tests.rs` for complete test suite including:
- Risk model creation and validation
- Risk score calculations
- Premium multiplier accuracy
- Fraud detection accuracy
- Authorization enforcement
- Event emission verification
- Statistics tracking

---

## Performance Considerations

- **Risk Assessment**: O(n) where n = number of claims (for history)
- **Fraud Detection**: O(m) where m = number of fraud indicators (constant ~8)
- **Storage**: Minimal - only active models stored
- **Cleanup**: Models automatically expire after 365 days

---

## Future Enhancements

1. Batch risk assessments
2. Predictive fraud scoring
3. Regional risk adjustments
4. Dynamic threshold adjustments
5. Integration with external data sources
6. Machine learning models
7. Real-time anomaly detection
8. Cross-policy fraud rings detection
