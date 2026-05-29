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
}
