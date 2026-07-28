// Error types for the insurance contract (Issue #101 - extracted from types.rs)

#[derive(Debug, PartialEq, Eq, scale::Encode, scale::Decode)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub enum InsuranceError {
    Unauthorized,
    PolicyNotFound,
    ClaimNotFound,
    PoolNotFound,
    PolicyAlreadyActive,
    PolicyExpired,
    PolicyInactive,
    InsufficientPremium,
    InsufficientPoolFunds,
    ClaimAlreadyProcessed,
    ClaimExceedsCoverage,
    InvalidParameters,
    OracleVerificationFailed,
    ReinsuranceCapacityExceeded,
    TokenNotFound,
    TransferFailed,
    CooldownPeriodActive,
    PropertyNotInsurable,
    DuplicateClaim,
    ReentrantCall,
    // Risk Assessment Errors (Task #254)
    RiskAssessmentNotFound,
    RiskAssessmentExpired,
    InvalidRiskFactors,
    RiskModelGenerationFailed,
    // Fraud Detection Errors (Task #258)
    FraudAssessmentNotFound,
    HighFraudRisk,
    FraudPatternNotFound,
    InvalidFraudIndicator,
    ReinsuranceAgreementNotFound,
    ReinsuranceAgreementExpired,
    ReinsuranceAgreementInactive,
    // Claim automation errors (#433)
    TriggerNotFound,
    TriggerInactive,
    TriggerAlreadyFired,
    TriggerConditionNotMet,
    InvalidPayoutMode,
    // Parametric policy errors (#433)
    ParametricPolicyNotFound,
    ParametricPolicyInactive,
    ParametricPolicyAlreadyTriggered,
    // Circuit breaker errors (#494)
    CircuitBreakerActive,
    SinglePayoutLimitExceeded,
    DailyPayoutLimitExceeded,
    // Admin key rotation errors (#496)
    KeyRotationCooldown,
    KeyRotationExpired,
    NoPendingRotation,
    RotationUnauthorized,
    RequestExpired,
}