# Dynamic Premium Calculation System

## Overview

The PropChain Insurance module now features a sophisticated dynamic premium calculation engine that adjusts insurance premiums based on comprehensive risk assessment, market conditions, policyholder behavior, and actuarial models.

## Architecture

```
Premium Calculation Flow:
┌─────────────────────────────────────────────────────────┐
│                  Risk Assessment                        │
│  - Location Risk (30%)                                  │
│  - Construction Risk (25%)                              │
│  - Age Risk (20%)                                       │
│  - Claims History (25%)                                 │
└────────────────────┬────────────────────────────────────┘
                     │
                     ▼
┌─────────────────────────────────────────────────────────┐
│              Actuarial Model (Optional)                 │
│  - Expected Loss Ratio                                  │
│  - Confidence Level                                     │
│  - Loss Frequency                                       │
│  - Average Loss Severity                                │
└────────────────────┬────────────────────────────────────┘
                     │
                     ▼
┌─────────────────────────────────────────────────────────┐
│              Market Conditions                          │
│  - Pool Utilization Rate                                │
│  - Available Capital                                    │
│  - Claims History in Pool                               │
└────────────────────┬────────────────────────────────────┘
                     │
                     ▼
┌─────────────────────────────────────────────────────────┐
│              Policyholder Factors                       │
│  - Multi-policy Discount                                │
│  - Claim-free Years                                     │
│  - Safety Features                                      │
│  - Loyalty Program                                      │
└────────────────────┬────────────────────────────────────┘
                     │
                     ▼
┌─────────────────────────────────────────────────────────┐
│              Time-based Adjustments                     │
│  - Policy Duration                                      │
│  - Seasonal Factors (future)                            │
└────────────────────┬────────────────────────────────────┘
                     │
                     ▼
┌─────────────────────────────────────────────────────────┐
│           Dynamic Premium Calculation                   │
│  Premium = Coverage × Base Rate × Risk × Coverage      │
│           × Pool × Time × Discount                      │
└─────────────────────────────────────────────────────────┘
```

## Premium Calculation Formula

```
Annual Premium = Coverage Amount 
               × Base Rate 
               × Risk Multiplier 
               × Coverage Type Multiplier 
               × Pool Utilization Multiplier 
               × Time Multiplier 
               × Discount Multiplier 
               ÷ Calculation Divisor
```

### Base Rate

**Source**: Actuarial Model or Default Rates

If an actuarial model is available:
```
Base Rate = Expected Loss Ratio × Confidence Adjustment × Expense Loading
```
- Expected Loss Ratio: From actuarial model (e.g., 600 = 6%)
- Confidence Adjustment: 95% = 1.0, 99% = 1.2
- Expense Loading: 20% for operational costs

Default Base Rates by Coverage Type:
- Fire: 1.2% (120 basis points)
- Flood: 2.0% (200 basis points)
- Earthquake: 2.5% (250 basis points)
- Theft: 1.0% (100 basis points)
- Liability Damage: 1.5% (150 basis points)
- Natural Disaster: 2.2% (220 basis points)
- Comprehensive: 3.0% (300 basis points)

### Risk Multiplier

Calculated from weighted risk assessment scores:

**Weighted Score Calculation**:
```
Weighted Score = (Location Risk × 30%) 
               + (Construction Risk × 25%) 
               + (Age Risk × 20%) 
               + (Claims History × 25%)
```

**Risk Score to Multiplier Mapping**:
| Weighted Score | Risk Level | Multiplier | Description |
|---------------|------------|------------|-------------|
| 0-10 | Very High | 4.0x | Extreme risk properties |
| 11-20 | High | 3.5x | Significant risk factors |
| 21-30 | High-Medium | 3.0x | Above average risk |
| 31-40 | Medium-High | 2.5x | Moderately elevated risk |
| 41-50 | Medium | 2.0x | Average risk profile |
| 51-60 | Medium-Low | 1.7x | Below average risk |
| 61-70 | Low-Medium | 1.4x | Good risk profile |
| 71-80 | Low | 1.1x | Very good risk profile |
| 81-90 | Very Low | 0.85x | Excellent risk profile |
| 91-100 | Minimal | 0.6x | Best possible risk |

### Coverage Type Multiplier

Reflects the relative risk of different coverage types:
- Fire: 1.0x
- Theft: 0.8x
- Flood: 1.5x
- Earthquake: 2.0x
- Liability Damage: 1.2x
- Natural Disaster: 1.8x
- Comprehensive: 2.5x

### Pool Utilization Multiplier

Dynamic adjustment based on risk pool capacity:

**Utilization Rate** = (Total Capital - Available Capital) / Total Capital

| Utilization Rate | Multiplier | Description |
|-----------------|------------|-------------|
| 0-30% | 0.9x | Low utilization - discount applied |
| 31-50% | 1.0x | Normal utilization |
| 51-70% | 1.15x | Medium-high - slight increase |
| 71-85% | 1.35x | High utilization - significant increase |
| 86-100% | 1.6x | Critical - major increase to manage risk |

### Time Multiplier

Rewards longer policy commitments:
| Duration | Multiplier | Description |
|----------|------------|-------------|
| < 30 days | 1.05x | Short-term premium |
| 1-3 months | 1.0x | Standard rate |
| 3-6 months | 0.95x | Slight discount |
| 6-12 months | 0.9x | Good discount |
| > 12 months | 0.85x | Best discount |

### Discount Multiplier

Multiple discount factors can stack (capped at 40% total):

**Multi-policy Discount**: 15%
- Applies when policyholder has multiple active policies

**Claim-free Discount**:
- 1 year: 5%
- 2 years: 10%
- 3 years: 15%
- 4+ years: 20%

**Safety Features Discount**: 10%
- Fire suppression systems
- Security systems
- Storm shutters
- Earthquake retrofitting

**Loyalty Discount**:
- 1-2 years: 3%
- 3-5 years: 6%
- 6+ years: 10%

**Maximum Total Discount**: 40%

### Deductible Calculation

Dynamic deductible based on risk profile:

**Base Deductible**: 5% of coverage amount

**Risk Adjustment**:
| Risk Score | Additional Deductible | Total Deductible |
|-----------|----------------------|------------------|
| 0-20 (Very High) | +20% | 25% |
| 21-40 (High) | +15% | 20% |
| 41-60 (Medium) | +10% | 15% |
| 61-80 (Low) | +7.5% | 12.5% |
| 81-100 (Very Low) | +5% | 10% |

**Safety Feature Reduction**: -5%

## Usage Examples

### Basic Premium Calculation

```rust
// Simple calculation with defaults
let premium = insurance.calculate_premium(
    property_id,
    500_000,           // $500,000 coverage
    CoverageType::Fire
)?;

println!("Annual Premium: ${}", premium.annual_premium);
println!("Monthly Premium: ${}", premium.monthly_premium);
println!("Deductible: ${}", premium.deductible);
```

### Advanced Premium Calculation with Modifiers

```rust
let modifiers = PremiumModifiers {
    has_multiple_policies: true,
    claim_free_years: 3,
    has_safety_features: true,
    loyalty_years: 5,
};

let premium = insurance.calculate_premium_with_modifiers(
    property_id,
    500_000,
    CoverageType::Comprehensive,
    31_536_000,  // 1 year in seconds
    modifiers
)?;

// Detailed breakdown
println!("Base Premium: ${}", premium.breakdown.base_premium);
println!("Risk Adjustment: +${}", premium.breakdown.risk_adjustment);
println!("Pool Adjustment: +${}", premium.breakdown.pool_adjustment);
println!("Discount Applied: -${}", premium.breakdown.discount_amount);
println!("Final Premium: ${}", premium.annual_premium);
```

## Premium Breakdown Structure

The `PremiumBreakdown` structure provides full transparency:

```rust
pub struct PremiumBreakdown {
    pub base_premium: u128,          // Coverage × Base Rate
    pub risk_adjustment: u128,       // Additional cost due to risk
    pub coverage_adjustment: u128,   // Coverage type adjustment
    pub pool_adjustment: u128,       // Pool utilization impact
    pub time_adjustment: u128,       // Duration-based adjustment
    pub discount_amount: u128,       // Total discounts applied
}
```

## Risk Assessment Requirements

Before calculating premiums, properties must have a valid risk assessment:

```rust
pub struct RiskAssessment {
    pub property_id: u64,
    pub location_risk_score: u32,      // 0-100 (100 = safest)
    pub construction_risk_score: u32,  // 0-100
    pub age_risk_score: u32,           // 0-100
    pub claims_history_score: u32,     // 0-100
    pub overall_risk_score: u32,       // 0-100
    pub risk_level: RiskLevel,
    pub assessed_at: u64,
    pub valid_until: u64,
}
```

## Actuarial Models

Optional actuarial models provide more accurate base rates:

```rust
pub struct ActuarialModel {
    pub model_id: u64,
    pub coverage_type: CoverageType,
    pub loss_frequency: u32,           // Claims per 10,000 policies
    pub average_loss_severity: u128,   // Average claim amount
    pub expected_loss_ratio: u32,      // Expected losses as % of premiums
    pub confidence_level: u32,         // Statistical confidence (95-99%)
    pub last_updated: u64,
    pub data_points: u32,              // Number of data points used
}
```

## Pool Dynamics

Risk pools automatically adjust premiums based on utilization:

```rust
pub struct RiskPool {
    pub pool_id: u64,
    pub total_capital: u128,
    pub available_capital: u128,
    pub total_premiums_collected: u128,
    pub total_claims_paid: u128,
    pub active_policies: u64,
    pub max_coverage_ratio: u32,       // Maximum exposure as % of capital
    pub is_active: bool,
}
```

**Example Pool Behavior**:
- Pool with $1M capital, $800K available (20% utilization) → 10% discount
- Pool with $1M capital, $200K available (80% utilization) → 35% increase
- This incentivizes liquidity providers and manages risk exposure

## Discount Optimization Strategies

### For Policyholders

1. **Bundle Policies**: Get 15% discount with multiple policies
2. **Maintain Claim-Free Record**: Up to 20% discount after 4+ years
3. **Install Safety Features**: 10% discount for approved systems
4. **Stay Loyal**: Up to 10% discount for long-term customers
5. **Choose Longer Terms**: 10-15% discount for annual+ policies

**Maximum Possible Discount**: 40% (capped)

### For Pool Operators

1. **Maintain Healthy Capitalization**: Keep utilization below 50%
2. **Diversify Risk**: Mix low and high-risk properties
3. **Monitor Claims Ratio**: Adjust pool parameters as needed
4. **Attract Liquidity Providers**: Competitive rewards

## Testing

Run the premium calculation tests:

```bash
cargo test --package propchain-insurance premium_tests
```

### Test Coverage

- ✅ Basic premium calculation
- ✅ High-risk property pricing
- ✅ Discount application and capping
- ✅ Pool utilization impact
- ✅ Duration-based adjustments
- ✅ Actuarial model integration
- ✅ Premium breakdown accuracy
- ✅ Edge cases and boundary conditions

## Future Enhancements

1. **Seasonal Adjustments**: Weather pattern integration
2. **Geographic Risk Zones**: Micro-location risk scoring
3. **Climate Change Factors**: Long-term risk projections
4. **Machine Learning**: AI-driven risk assessment
5. **Oracle Integration**: Real-time external data feeds
6. **Dynamic Deductibles**: Adjustable based on claim history
7. **Usage-Based Insurance**: IoT sensor integration
8. **Peer-to-Peer Pools**: Community-based risk sharing

## API Reference

### `calculate_premium()`

Calculate premium with default parameters.

**Parameters**:
- `property_id: u64` - Property identifier
- `coverage_amount: u128` - Desired coverage amount
- `coverage_type: CoverageType` - Type of coverage

**Returns**: `Result<PremiumCalculation, InsuranceError>`

### `calculate_premium_with_modifiers()`

Calculate premium with full customization.

**Parameters**:
- `property_id: u64` - Property identifier
- `coverage_amount: u128` - Desired coverage amount
- `coverage_type: CoverageType` - Type of coverage
- `duration_seconds: u64` - Policy duration
- `modifiers: PremiumModifiers` - Discount modifiers

**Returns**: `Result<PremiumCalculation, InsuranceError>`

## Implementation Files

- `src/premium_engine.rs` - Core calculation engine
- `src/types.rs` - Data structures
- `src/premium_tests.rs` - Comprehensive test suite
- `src/lib.rs` - Contract integration

## Mathematical Precision

All calculations use basis points (1/100th of 1%) for precision:
- 100 basis points = 1%
- Multipliers are in basis points (e.g., 150 = 1.5x)
- Saturation arithmetic prevents overflow
- Rounding errors < 100 units (negligible for typical premiums)

## Compliance & Auditability

- All calculations are deterministic and reproducible
- Premium breakdown provides full transparency
- Actuarial models can be audited independently
- Risk assessments are timestamped and versioned
- Pool utilization is publicly verifiable on-chain
