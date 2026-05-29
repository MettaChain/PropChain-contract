# Insurance Features - Quick Reference

## Risk Assessment Model (Task #254)

### Method Signatures

```rust
// Create comprehensive risk model
pub fn assess_property_risk_comprehensive(
    &mut self,
    property_id: u64,
    property_age_years: u32,
    property_value: u128,
    location_code: String,
    construction_type: String,
    has_security_system: bool,
    has_fire_extinguisher: bool,
    has_alarm_system: bool,
    owner_age_years: u32,
    years_as_owner: u32,
) -> Result<(u64, u32), InsuranceError>

// Retrieve risk model
pub fn get_property_risk_model(
    &self,
    risk_id: u64,
) -> Result<PropertyRiskModel, InsuranceError>

// Update risk assessment
pub fn update_property_risk_assessment(
    &mut self,
    risk_id: u64,
    property_age_years: u32,
    has_security_system: bool,
    has_fire_extinguisher: bool,
    has_alarm_system: bool,
) -> Result<(u32, u32), InsuranceError>
```

### Risk Score Ranges

| Risk Level | Score Range | Premium Multiplier |
|-----------|-------------|-------------------|
| VeryLow | 0-200 | 0.5x (50% discount) |
| Low | 201-400 | 0.75x (25% discount) |
| Medium | 401-600 | 1.0x (normal) |
| High | 601-800 | 1.5x (50% increase) |
| VeryHigh | 801-1000 | 2.5x (150% increase) |

### Location Codes
- premium_safe_zone (100)
- rural_low_risk (200)
- suburban (350)
- flood_prone (750)
- high_risk_zone (800)

### Construction Types
- steel_frame (250)
- reinforced_concrete (300)
- stone_brick (350)
- composite_materials (400)
- masonry_veneer (600)
- wood_frame (750)

---

## Fraud Detection System (Task #258)

### Method Signatures

```rust
// Assess fraud risk on claim
pub fn assess_claim_fraud_risk(
    &mut self,
    claim_id: u64,
    policy_id: u64,
) -> Result<(u64, u32, bool), InsuranceError>

// Retrieve fraud assessment
pub fn get_fraud_assessment(
    &self,
    assessment_id: u64,
) -> Result<FraudRiskAssessment, InsuranceError>

// Get fraud statistics
pub fn get_fraud_detection_stats(
    &self,
) -> Option<FraudDetectionStats>
```

### Fraud Score Interpretation

| Fraud Level | Score Range | Action |
|-----------|-------------|--------|
| VeryLow | 0-250 | Auto-approve |
| Low | 251-450 | Standard review |
| Medium | 451-600 | Enhanced review |
| High | 601-800 | Manual review |
| VeryHigh | 801-1000 | Reject/Flag |

### Fraud Indicators

1. **MultipleClaimsShortPeriod** - 3+ claims in 30 days (300 pts)
2. **AnomalousClaimAmount** - 150%+ above average (200-300 pts)
3. **SuspiciousTimingPattern** - Weekend submission (200 pts)
4. **ExcessiveCoverageRatio** - 85%+ of coverage (100-250 pts)
5. **HistoricalFraudPattern** - High claim history (150-300 pts)
6. **Misrepresentation** - Poor documentation (50-200 pts)
7. **KnownFraudNetwork** - Associated fraud accounts (150-400 pts)
8. **DuplicateClaimPatterns** - Similar to known fraud (150-400 pts)

---

## Data Types

### PropertyRiskModel
```rust
pub struct PropertyRiskModel {
    pub risk_id: u64,
    pub property_id: u64,
    pub property_factors: PropertyRiskFactors,
    pub historical_claims_count: u32,
    pub historical_claims_amount: u128,
    pub location_risk_score: u32,
    pub construction_risk_score: u32,
    pub age_risk_score: u32,
    pub ownership_risk_score: u32,
    pub claims_history_score: u32,
    pub safety_features_score: u32,
    pub overall_risk_score: u32,
    pub final_risk_level: RiskLevel,
    pub premium_multiplier: u32,
    pub assessed_at: u64,
    pub valid_until: u64,
    pub model_version: u32,
}
```

### FraudRiskAssessment
```rust
pub struct FraudRiskAssessment {
    pub assessment_id: u64,
    pub claim_id: u64,
    pub policy_id: u64,
    pub policyholder: AccountId,
    pub fraud_score: u32,
    pub fraud_level: RiskLevel,
    pub detected_indicators: Vec<FraudIndicator>,
    pub claim_amount: u128,
    pub expected_amount_range: (u128, u128),
    pub time_since_last_claim: Option<u64>,
    pub similar_claims_count: u32,
    pub policyholder_claims_count: u32,
    pub assessor_notes: String,
    pub assessment_timestamp: u64,
    pub requires_manual_review: bool,
}
```

---

## Events

### Risk Assessment Events
```rust
PropertyRiskModelCreated {
    risk_id, property_id, overall_risk_score,
    final_risk_level, premium_multiplier, timestamp
}

PropertyRiskModelUpdated {
    risk_id, property_id, new_risk_score,
    new_risk_level, timestamp
}
```

### Fraud Detection Events
```rust
FraudRiskAssessmentCreated {
    assessment_id, claim_id, policyholder,
    fraud_score, fraud_level, requires_manual_review, timestamp
}

HighFraudRiskDetected {
    claim_id, policyholder, fraud_score, indicator_count, timestamp
}

FraudPatternDetected {
    claim_id, indicator_type, risk_increase, timestamp
}
```

---

## Error Types

### Risk Assessment
- `RiskAssessmentNotFound`
- `RiskAssessmentExpired`
- `InvalidRiskFactors`
- `RiskModelGenerationFailed`

### Fraud Detection
- `FraudAssessmentNotFound`
- `HighFraudRisk`
- `FraudPatternNotFound`
- `InvalidFraudIndicator`

### General
- `Unauthorized` - No permission
- `PolicyNotFound` - Policy doesn't exist
- `ClaimNotFound` - Claim doesn't exist

---

## Authorization

### Risk Assessment
- **Admin**: Can create and update assessments
- **Oracle**: With authorization, can create assessments
- **Public**: Can view assessments

### Fraud Detection
- **Admin**: Can assess fraud risk
- **Assessor**: With authorization, can assess fraud risk
- **Public**: Can view assessments

---

## Constants & Thresholds

```rust
// Risk Assessment
const ASSESSMENT_VALIDITY_DAYS: u64 = 365;
const MODEL_VERSION: u32 = 1;

// Fraud Detection
const HIGH_FRAUD_RISK_THRESHOLD: u32 = 700;
const MEDIUM_FRAUD_RISK_THRESHOLD: u32 = 450;
const CLAIMS_SHORT_PERIOD_DAYS: u64 = 30;
```

---

## Workflow

### Risk Assessment Workflow
```
1. Admin calls assess_property_risk_comprehensive()
2. System calculates 6 risk factor scores
3. Weighted average = overall_risk_score
4. Premium multiplier determined
5. PropertyRiskModel stored
6. PropertyRiskModelCreated event emitted
7. Model valid for 365 days
8. Can be updated with new information
```

### Fraud Detection Workflow
```
1. Claim submitted by policyholder
2. Admin/Assessor calls assess_claim_fraud_risk()
3. 8 fraud indicators analyzed
4. Fraud score calculated (0-1000)
5. FraudRiskAssessment created
6. FraudRiskAssessmentCreated event emitted
7. If score > 450: requires_manual_review = true
8. If score > 700: HighFraudRiskDetected event
9. Statistics updated
```

---

## Integration Example

```rust
// 1. Create risk assessment
let (risk_id, premium_multiplier) = contract.assess_property_risk_comprehensive(
    1, 10, 5_000_000_000_000, "premium_safe_zone".into(),
    "steel_frame".into(), true, true, true, 45, 15
)?;

// 2. Calculate premium (premium_multiplier used here)
let premium_calc = contract.calculate_premium(
    1, 
    1_000_000_000_000, 
    CoverageType::Fire
)?;

// 3. Create policy with calculated premium
let policy_id = contract.create_policy(
    1, CoverageType::Fire, 1_000_000_000_000,
    pool_id, 86400 * 365, "ipfs://metadata".into()
)?;

// 4. Submit claim
let claim_id = contract.submit_claim(
    policy_id, 100_000_000_000,
    "Water damage".into(), "ipfs://evidence".into()
)?;

// 5. Assess fraud risk
let (assessment_id, fraud_score, requires_review) = 
    contract.assess_claim_fraud_risk(claim_id, policy_id)?;

// 6. Process claim (considering fraud assessment)
if requires_review {
    // Manual review needed
} else if fraud_score < 250 {
    // Auto-approve
    contract.process_claim(claim_id, true, "".into(), "".into())?;
}
```

---

## Testing

Run all tests:
```bash
cargo test --all
```

Run insurance tests only:
```bash
cargo test -p propchain-insurance
```

Run specific test:
```bash
cargo test test_assess_property_risk_comprehensive_works
```

---

## Documentation

- Full implementation: `docs/INSURANCE_FEATURES_IMPLEMENTATION.md`
- Usage guide: `docs/INSURANCE_FEATURES_USAGE_GUIDE.md`
- Implementation status: `IMPLEMENTATION_COMPLETE.md`

---

## Common Scenarios

### Scenario 1: New Property Purchase
```rust
// New, safe property with all safety features
(risk_id, 5000) // 0.5x multiplier - 50% discount
// Risk score: ~150 (VeryLow)
```

### Scenario 2: Older Property in Risky Area
```rust
// Old property, no safety features, high-risk area
(risk_id, 25000) // 2.5x multiplier - 150% premium
// Risk score: ~800 (VeryHigh)
```

### Scenario 3: Normal Claim
```rust
// Regular claim, proper documentation
fraud_score: 150 // No manual review needed
```

### Scenario 4: Suspicious Claim
```rust
// High claim amount, no documentation
fraud_score: 750 // requires_manual_review: true
```

---

## Performance

- Risk Assessment: O(n) where n = historical claims
- Fraud Detection: O(1) constant time analysis
- Storage: Minimal - only models and assessments
- No expensive computations

---

## Version Info

- Risk Assessment Model: v1
- Fraud Detection: v1
- Compatible with Ink! 5.0+
- Target: Polkadot Substrate chains

---

For more details, see full documentation files or examine test cases.
