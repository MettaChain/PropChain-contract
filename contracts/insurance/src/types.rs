// Data types for the insurance contract (Issue #101 - extracted from lib.rs)
// Parametric insurance types added for Issue #249
// Circuit breaker types added for Issue #494
// Admin key rotation types added for Issue #496

// =========================================================================
// CIRCUIT BREAKER TYPES (Issue #494)
// =========================================================================

/// Configuration parameters for the insurance payout circuit breaker.
#[derive(
    Debug, Clone, PartialEq, scale::Encode, scale::Decode, ink::storage::traits::StorageLayout,
)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub struct CircuitBreakerConfig {
    /// Maximum amount that can be paid out in a single claim (0 = no limit)
    pub max_single_payout: u128,
    /// Maximum total payout allowed within one `window_seconds` rolling window per pool
    pub max_daily_payout: u128,
    /// Length of the rolling payout-tracking window in seconds (default: 86400 = 1 day)
    pub window_seconds: u64,
}

/// The comparison operator used to evaluate oracle data against a trigger threshold.
#[derive(
    Debug,
    Clone,
    PartialEq,
    Eq,
    scale::Encode,
    scale::Decode,
    ink::storage::traits::StorageLayout,
)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub enum TriggerComparison {
    /// Payout when oracle value >= threshold (e.g. flood depth >= 2m)
    GreaterThanOrEqual,
    /// Payout when oracle value <= threshold (e.g. temperature <= -10°C)
    LessThanOrEqual,
}

/// Status of a parametric policy.
#[derive(
    Debug,
    Clone,
    PartialEq,
    Eq,
    scale::Encode,
    scale::Decode,
    ink::storage::traits::StorageLayout,
)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub enum ParametricPolicyStatus {
    Active,
    Triggered,
    Expired,
    Cancelled,
}

/// A parametric insurance policy that pays out automatically when an oracle
/// reports a value that crosses the defined trigger threshold.
#[derive(
    Debug, Clone, PartialEq, scale::Encode, scale::Decode, ink::storage::traits::StorageLayout,
)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub struct ParametricPolicy {
    pub policy_id: u64,
    pub property_id: u64,
    pub policyholder: AccountId,
    /// Human-readable label for the metric being tracked (e.g. "flood_depth_cm")
    pub metric: String,
    /// The threshold value (scaled integer, e.g. centimetres or tenths of a degree)
    pub trigger_threshold: i128,
    pub comparison: TriggerComparison,
    /// Full coverage amount paid out automatically when triggered
    pub coverage_amount: u128,
    /// Premium paid upfront
    pub premium_amount: u128,
    pub pool_id: u64,
    pub start_time: u64,
    pub end_time: u64,
    pub status: ParametricPolicyStatus,
}

/// An oracle data submission that may trigger parametric payouts.
#[derive(
    Debug, Clone, PartialEq, scale::Encode, scale::Decode, ink::storage::traits::StorageLayout,
)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub struct OracleDataPoint {
    pub data_id: u64,
    pub property_id: u64,
    pub metric: String,
    pub value: i128,
    pub submitted_by: AccountId,
    pub submitted_at: u64,
}

#[derive(
    Debug,
    Clone,
    PartialEq,
    Eq,
    scale::Encode,
    scale::Decode,
    ink::storage::traits::StorageLayout,
)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub enum PolicyStatus {
    Active,
    Expired,
    Cancelled,
    Claimed,
    Suspended,
}

#[derive(
    Debug,
    Clone,
    PartialEq,
    Eq,
    scale::Encode,
    scale::Decode,
    ink::storage::traits::StorageLayout,
)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub enum CoverageType {
    Fire,
    Flood,
    Earthquake,
    Theft,
    LiabilityDamage,
    NaturalDisaster,
    Comprehensive,
}

#[derive(
    Debug,
    Clone,
    PartialEq,
    Eq,
    scale::Encode,
    scale::Decode,
    ink::storage::traits::StorageLayout,
)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub enum ClaimStatus {
    Pending,
    UnderReview,
    OracleVerifying,
    Approved,
    Rejected,
    Paid,
    Disputed,
}

#[derive(
    Debug,
    Clone,
    PartialEq,
    Eq,
    scale::Encode,
    scale::Decode,
    ink::storage::traits::StorageLayout,
)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub enum RiskLevel {
    VeryLow,
    Low,
    Medium,
    High,
    VeryHigh,
}

#[derive(
    Debug, Clone, PartialEq, scale::Encode, scale::Decode, ink::storage::traits::StorageLayout,
)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub struct InsurancePolicy {
    pub policy_id: u64,
    pub property_id: u64,
    pub policyholder: AccountId,
    pub coverage_type: CoverageType,
    pub coverage_amount: u128,
    pub premium_amount: u128,
    pub deductible: u128,
    pub start_time: u64,
    pub end_time: u64,
    pub status: PolicyStatus,
    pub risk_level: RiskLevel,
    pub pool_id: u64,
    pub claims_count: u32,
    pub total_claimed: u128,
    pub metadata_url: String,
}

#[derive(
    Debug, Clone, PartialEq, scale::Encode, scale::Decode, ink::storage::traits::StorageLayout,
)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub struct InsuranceClaim {
    pub claim_id: u64,
    pub policy_id: u64,
    pub claimant: AccountId,
    pub claim_amount: u128,
    pub description: String,
    pub evidence_url: String,
    pub oracle_report_url: String,
    pub status: ClaimStatus,
    pub submitted_at: u64,
    pub processed_at: Option<u64>,
    pub payout_amount: u128,
    pub assessor: Option<AccountId>,
    pub rejection_reason: String,
}

#[derive(
    Debug, Clone, PartialEq, scale::Encode, scale::Decode, ink::storage::traits::StorageLayout,
)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub struct RiskPool {
    pub pool_id: u64,
    pub name: String,
    pub coverage_type: CoverageType,
    pub total_capital: u128,
    pub available_capital: u128,
    pub total_premiums_collected: u128,
    pub total_claims_paid: u128,
    pub active_policies: u64,
    pub max_coverage_ratio: u32,
    pub reinsurance_threshold: u128,
    pub created_at: u64,
    pub is_active: bool,
}

#[derive(
    Debug, Clone, PartialEq, scale::Encode, scale::Decode, ink::storage::traits::StorageLayout,
)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub struct RiskAssessment {
    pub property_id: u64,
    pub location_risk_score: u32,
    pub construction_risk_score: u32,
    pub age_risk_score: u32,
    pub claims_history_score: u32,
    pub overall_risk_score: u32,
    pub risk_level: RiskLevel,
    pub assessed_at: u64,
    pub valid_until: u64,
}

#[derive(
    Debug, Clone, PartialEq, scale::Encode, scale::Decode, ink::storage::traits::StorageLayout,
)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub struct PremiumCalculation {
    pub base_rate: u32,
    pub risk_multiplier: u32,
    pub coverage_multiplier: u32,
    pub pool_utilization_multiplier: u32,
    pub time_multiplier: u32,
    pub discount_multiplier: u32,
    /// Claim-frequency surcharge multiplier in basis points (10 000 = 1.0×).
    /// Values above 10 000 indicate a surcharge for high claim frequency.
    pub claim_freq_multiplier: u32,
    pub annual_premium: u128,
    pub monthly_premium: u128,
    pub deductible: u128,
    pub breakdown: PremiumBreakdown,
}

#[derive(
    Debug, Clone, PartialEq, scale::Encode, scale::Decode, ink::storage::traits::StorageLayout,
)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub struct PremiumBreakdown {
    pub base_premium: u128,
    pub risk_adjustment: u128,
    pub coverage_adjustment: u128,
    pub pool_adjustment: u128,
    pub time_adjustment: u128,
    pub discount_amount: u128,
    /// Additional premium added by the rolling-window claim-frequency surcharge.
    /// Zero when `recent_claims_count` is 0 (baseline, no surcharge).
    pub claim_freq_adjustment: u128,
}

#[derive(
    Debug, Clone, PartialEq, scale::Encode, scale::Decode, ink::storage::traits::StorageLayout,
)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub struct PremiumModifiers {
    pub has_multiple_policies: bool,
    pub claim_free_years: u32,
    pub has_safety_features: bool,
    pub loyalty_years: u32,
    /// Number of approved claims filed against this property within the
    /// rolling observation window (typically the last 12 months).
    /// Used by the premium engine to apply a claim-frequency surcharge.
    /// Set to `0` for new policyholders or those with no recent claims.
    pub recent_claims_count: u32,
}

#[derive(
    Debug, Clone, PartialEq, scale::Encode, scale::Decode, ink::storage::traits::StorageLayout,
)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub struct ReinsuranceAgreement {
    pub agreement_id: u64,
    pub reinsurer: AccountId,
    pub coverage_limit: u128,
    pub retention_limit: u128,
    /// Basis points of premium to cede (e.g. 2000 = 20%). Used for QuotaShare.
    pub premium_ceded_rate: u32,
    pub coverage_types: Vec<CoverageType>,
    pub start_time: u64,
    pub end_time: u64,
    pub is_active: bool,
    pub total_ceded_premiums: u128,
    pub total_recoveries: u128,
    /// How risk is distributed with this reinsurer
    pub treaty_type: ReinsuranceTreatyType,
    /// Running count of premium cessions under this agreement
    pub cession_count: u64,
    /// Running count of loss recoveries under this agreement
    pub recovery_count: u64,
}

#[derive(
    Debug, Clone, PartialEq, scale::Encode, scale::Decode, ink::storage::traits::StorageLayout,
)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub struct InsuranceToken {
    pub token_id: u64,
    pub policy_id: u64,
    pub owner: AccountId,
    pub face_value: u128,
    pub is_tradeable: bool,
    pub created_at: u64,
    pub listed_price: Option<u128>,
}

#[derive(
    Debug, Clone, PartialEq, scale::Encode, scale::Decode, ink::storage::traits::StorageLayout,
)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub struct ActuarialModel {
    pub model_id: u64,
    pub coverage_type: CoverageType,
    pub loss_frequency: u32,
    pub average_loss_severity: u128,
    pub expected_loss_ratio: u32,
    pub confidence_level: u32,
    pub last_updated: u64,
    pub data_points: u32,
}

#[derive(
    Debug, Clone, PartialEq, scale::Encode, scale::Decode, ink::storage::traits::StorageLayout,
)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub struct UnderwritingCriteria {
    pub max_property_age_years: u32,
    pub min_property_value: u128,
    pub max_property_value: u128,
    pub excluded_locations: Vec<String>,
    pub required_safety_features: bool,
    pub max_previous_claims: u32,
    pub min_risk_score: u32,
}

#[derive(
    Debug, Clone, PartialEq, scale::Encode, scale::Decode, ink::storage::traits::StorageLayout,
)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub struct PoolLiquidityProvider {
    pub provider: AccountId,
    pub pool_id: u64,
    pub deposited_amount: u128,
    pub share_percentage: u32,
    pub deposited_at: u64,
    pub last_reward_claim: u64,
    pub accumulated_rewards: u128,
}

// =========================================================================
// RISK ASSESSMENT MODEL TYPES (Task #254)
// =========================================================================

/// Bit positions for `PropertyRiskFactors::safety_flags`.
///
/// Packing three `bool` fields into a single `u8` reduces the SCALE-encoded
/// size of every stored `PropertyRiskModel` by 2 bytes and cuts the number of
/// storage reads needed to inspect all three flags from 3 to 1.
pub mod safety_flag {
    pub const SECURITY_SYSTEM: u8 = 0b0000_0001;
    pub const FIRE_EXTINGUISHER: u8 = 0b0000_0010;
    pub const ALARM_SYSTEM: u8 = 0b0000_0100;
}

/// Property risk factors for comprehensive risk assessment.
///
/// ### Storage layout change (issue #515)
/// The three separate `bool` fields `has_security_system`, `has_fire_extinguisher`,
/// and `has_alarm_system` have been packed into a single `u8 safety_flags` field:
///
/// | bit | meaning                |
/// |-----|------------------------|
/// |  0  | `has_security_system`  |
/// |  1  | `has_fire_extinguisher`|
/// |  2  | `has_alarm_system`     |
///
/// Use the [`safety_flag`] constants and the accessor methods below.
#[derive(
    Debug, Clone, PartialEq, scale::Encode, scale::Decode, ink::storage::traits::StorageLayout,
)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub struct PropertyRiskFactors {
    pub property_id: u64,
    pub property_age_years: u32,
    pub property_value: u128,
    pub location_code: String,
    pub construction_type: String,
    /// Packed safety-feature flags.  See [`safety_flag`] for bit positions.
    pub safety_flags: u8,
    pub owner_age_years: u32,
    pub years_as_owner: u32,
    pub assessed_at: u64,
}

impl PropertyRiskFactors {
    /// Construct `safety_flags` from three individual boolean values.
    #[inline]
    pub fn encode_safety_flags(
        has_security_system: bool,
        has_fire_extinguisher: bool,
        has_alarm_system: bool,
    ) -> u8 {
        (has_security_system as u8 * safety_flag::SECURITY_SYSTEM)
            | (has_fire_extinguisher as u8 * safety_flag::FIRE_EXTINGUISHER)
            | (has_alarm_system as u8 * safety_flag::ALARM_SYSTEM)
    }

    #[inline]
    pub fn has_security_system(&self) -> bool {
        self.safety_flags & safety_flag::SECURITY_SYSTEM != 0
    }

    #[inline]
    pub fn has_fire_extinguisher(&self) -> bool {
        self.safety_flags & safety_flag::FIRE_EXTINGUISHER != 0
    }

    #[inline]
    pub fn has_alarm_system(&self) -> bool {
        self.safety_flags & safety_flag::ALARM_SYSTEM != 0
    }
}

/// Comprehensive risk assessment model with detailed scoring
#[derive(
    Debug, Clone, PartialEq, scale::Encode, scale::Decode, ink::storage::traits::StorageLayout,
)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub struct PropertyRiskModel {
    pub risk_id: u64,
    pub property_id: u64,
    pub property_factors: PropertyRiskFactors,
    pub historical_claims_count: u32,
    pub historical_claims_amount: u128,
    pub location_risk_score: u32,     // 0-1000
    pub construction_risk_score: u32, // 0-1000
    pub age_risk_score: u32,          // 0-1000
    pub ownership_risk_score: u32,    // 0-1000
    pub claims_history_score: u32,    // 0-1000
    pub safety_features_score: u32,   // 0-1000 (higher is safer)
    pub overall_risk_score: u32,      // 0-1000 (weighted average)
    pub final_risk_level: RiskLevel,
    pub premium_multiplier: u32,      // 10000 = 1.0x
    pub assessed_at: u64,
    pub valid_until: u64,
    pub model_version: u32,
}

// =========================================================================
// FRAUD DETECTION TYPES (Task #258)
// =========================================================================

/// Types of fraud indicators detected in claims
// REINSURANCE DISTRIBUTION TYPES
// =========================================================================

/// Treaty type determines how risk is shared with the reinsurer
#[derive(
    Debug,
    Clone,
    PartialEq,
    Eq,
    scale::Encode,
    scale::Decode,
    ink::storage::traits::StorageLayout,
)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub enum FraudIndicator {
    MultipleClaimsShortPeriod,   // Multiple claims within days
    AnomalousClaimAmount,         // Claim amount far above normal
    SuspiciousTimingPattern,      // Claims on weekends/holidays
    ExcessiveCoverageRatio,       // Claim close to max coverage
    HistoricalFraudPattern,       // Policyholder with history
    Misrepresentation,            // Inconsistent claim details
    KnownFraudNetwork,            // Associated with fraudulent accounts
    DuplicateClaimPatterns,       // Similar to previous fraud claims
}

/// Treaty type determines how risk is shared with the reinsurer
#[derive(
    Debug,
    Clone,
    PartialEq,
    Eq,
    scale::Encode,
    scale::Decode,
    ink::storage::traits::StorageLayout,
)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub enum ReinsuranceTreatyType {
    /// Quota Share: cede a fixed % of every premium and claim
    QuotaShare,
    /// Excess of Loss: reinsurer covers losses above a retention threshold
    ExcessOfLoss,
    /// Surplus: cede the portion of risk exceeding the insurer's line
    Surplus,
}

/// Tracks a single premium cession event for audit purposes
#[derive(
    Debug, Clone, PartialEq, scale::Encode, scale::Decode, ink::storage::traits::StorageLayout,
)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub struct FraudRiskAssessment {
    pub assessment_id: u64,
    pub claim_id: u64,
    pub policy_id: u64,
    pub policyholder: AccountId,
    pub fraud_score: u32,              // 0-1000 (higher = more fraud risk)
    pub fraud_level: RiskLevel,        // Fraud risk level
    pub detected_indicators: Vec<FraudIndicator>,
    pub claim_amount: u128,
    pub expected_amount_range: (u128, u128), // (min, max) expected
    pub time_since_last_claim: Option<u64>,  // seconds
    pub similar_claims_count: u32,     // Similar historical claims
    pub policyholder_claims_count: u32,
    pub assessor_notes: String,
    pub assessment_timestamp: u64,
    pub requires_manual_review: bool,
}

/// Historical fraud pattern for detection
#[derive(
    Debug, Clone, PartialEq, scale::Encode, scale::Decode, ink::storage::traits::StorageLayout,
)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub struct PremiumCession {
    pub cession_id: u64,
    pub agreement_id: u64,
    pub policy_id: u64,
    pub gross_premium: u128,
    pub ceded_premium: u128,
    pub ceded_at: u64,
}

/// Tracks a single loss recovery request from a reinsurer
#[derive(
    Debug, Clone, PartialEq, scale::Encode, scale::Decode, ink::storage::traits::StorageLayout,
)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub struct FraudPattern {
    pub pattern_id: u64,
    pub pattern_type: FraudIndicator,
    pub description: String,
    pub severity_weight: u32, // Weight in fraud scoring (0-1000)
    pub triggered_count: u32, // How many times this pattern triggered
    pub last_triggered: u64,
    pub is_active: bool,
}

/// Statistics for fraud detection and prevention
#[derive(
    Debug, Clone, PartialEq, scale::Encode, scale::Decode, ink::storage::traits::StorageLayout,
)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub struct LossRecovery {
    pub recovery_id: u64,
    pub agreement_id: u64,
    pub claim_id: u64,
    pub gross_loss: u128,
    pub recovered_amount: u128,
    pub recovered_at: u64,
}

/// Summary statistics for a reinsurance agreement
#[derive(
    Debug, Clone, PartialEq, scale::Encode, scale::Decode, ink::storage::traits::StorageLayout,
)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub struct FraudDetectionStats {
    pub total_assessments: u32,
    pub high_risk_claims: u32,
    pub rejected_fraud_claims: u32,
    pub patterns_detected: u32,
    pub false_positive_count: u32,
    pub average_fraud_score: u32,
    pub last_update: u64,
}

/// Summary statistics for a reinsurance agreement.
///
/// Previously missing derives caused compile errors when this type was
/// returned from an ink! message or stored in a Mapping (fixed bug — see #487).
#[derive(
    Debug, Clone, PartialEq, scale::Encode, scale::Decode, ink::storage::traits::StorageLayout,
)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub struct ReinsuranceStats {
    pub agreement_id: u64,
    pub treaty_type: ReinsuranceTreatyType,
    pub total_ceded_premiums: u128,
    pub total_recoveries: u128,
    pub cession_count: u64,
    pub recovery_count: u64,
    /// Net position: recoveries - ceded_premiums (can be negative conceptually, stored as i128)
    pub net_recovery: i128,
}

/// The oracle metric being monitored by a claim trigger.
#[derive(
    Debug, Clone, PartialEq, Eq,
    scale::Encode, scale::Decode, ink::storage::traits::StorageLayout,
)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub enum TriggerMetric {
    FloodDepthCm,
    WindSpeedKph,
    TemperatureCelsius,
    RainfallMm,
    EarthquakeMagnitude,
}

/// Comparison operator for evaluating oracle data against a trigger threshold.
#[derive(
    Debug, Clone, PartialEq, Eq,
    scale::Encode, scale::Decode, ink::storage::traits::StorageLayout,
)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub enum TriggerComparator {
    GreaterOrEqual,
    LessOrEqual,
}

/// How the payout amount is calculated when a trigger fires.
#[derive(
    Debug, Clone, PartialEq, Eq,
    scale::Encode, scale::Decode, ink::storage::traits::StorageLayout,
)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub enum PayoutMode {
    Fixed(u128),
    PercentBps(u32),
    FullCoverage,
}

/// An oracle-driven claim trigger stored in contract state.
#[derive(
    Debug, Clone, PartialEq,
    scale::Encode, scale::Decode, ink::storage::traits::StorageLayout,
)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub struct ClaimTrigger {
    pub trigger_id: u64,
    pub policy_id: u64,
    pub metric: TriggerMetric,
    pub threshold: i128,
    pub comparator: TriggerComparator,
    pub payout_mode: PayoutMode,
    pub is_active: bool,
    pub triggered: bool,
    pub last_observed_value: Option<u128>,
    pub last_report_url: String,
    pub created_at: u64,
    pub triggered_at: Option<u64>,
    pub triggering_claim_id: Option<u64>,
}

