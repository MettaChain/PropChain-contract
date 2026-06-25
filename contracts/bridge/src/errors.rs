// Error types for the bridge contract (Issue #101 - extracted from lib.rs)

#[derive(Debug, PartialEq, Eq, scale::Encode, scale::Decode)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub enum Error {
    Unauthorized,
    TokenNotFound,
    InvalidChain,
    BridgeNotSupported,
    InsufficientSignatures,
    RequestExpired,
    AlreadySigned,
    InvalidRequest,
    BridgePaused,
    InvalidMetadata,
    DuplicateRequest,
    GasLimitExceeded,
    RateLimitExceeded,
    ReentrantCall,
    /// No cross-chain transaction status record exists for the given identifier.
    TransactionNotFound,
    /// The requested status transition is not valid for the current status.
    InvalidStatusTransition,
    /// The targeted operation class is currently paused (emergency stop).
    OperationPaused,
    /// Caller is not a registered guardian (and not the admin).
    NotGuardian,
    /// Bridge execution requires travel rule data that has not been submitted.
    TravelRuleDataRequired,
    /// Travel rule data for this request has already been submitted.
    TravelRuleDataAlreadySubmitted,
    /// Caller is not an emergency signer.
    NotEmergencySigner,
    /// Emergency request has already been executed.
    EmergencyRequestAlreadyExecuted,
    /// Emergency request has expired.
    EmergencyRequestExpired,
    /// Asset is already frozen.
    AssetAlreadyFrozen,
    /// Asset is not frozen.
    AssetNotFrozen,
    /// Insufficient emergency signatures.
    InsufficientEmergencySignatures,
}

impl core::fmt::Display for Error {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Error::Unauthorized => write!(f, "Caller is not authorized"),
            Error::TokenNotFound => write!(f, "Token does not exist"),
            Error::InvalidChain => write!(f, "Invalid chain ID"),
            Error::BridgeNotSupported => write!(f, "Bridge not supported for this token"),
            Error::InsufficientSignatures => write!(f, "Insufficient signatures collected"),
            Error::RequestExpired => write!(f, "Bridge request has expired"),
            Error::AlreadySigned => write!(f, "Already signed this request"),
            Error::InvalidRequest => write!(f, "Invalid bridge request"),
            Error::BridgePaused => write!(f, "Bridge operations are paused"),
            Error::InvalidMetadata => write!(f, "Invalid metadata"),
            Error::DuplicateRequest => write!(f, "Duplicate bridge request"),
            Error::GasLimitExceeded => write!(f, "Gas limit exceeded"),
            Error::RateLimitExceeded => write!(f, "Rate limit exceeded"),
            Error::ReentrantCall => write!(f, "Reentrant call"),
            Error::TransactionNotFound => write!(f, "Cross-chain transaction not found"),
            Error::InvalidStatusTransition => write!(f, "Invalid cross-chain status transition"),
            Error::OperationPaused => write!(f, "Operation is currently paused"),
            Error::NotGuardian => write!(f, "Caller is not a guardian"),
            Error::TravelRuleDataRequired => write!(f, "Travel rule data required before bridge execution"),
            Error::TravelRuleDataAlreadySubmitted => write!(f, "Travel rule data already submitted for this request"),
            Error::NotEmergencySigner => write!(f, "Caller is not an emergency signer"),
            Error::EmergencyRequestAlreadyExecuted => write!(f, "Emergency request has already been executed"),
            Error::EmergencyRequestExpired => write!(f, "Emergency request has expired"),
            Error::AssetAlreadyFrozen => write!(f, "Asset is already frozen"),
            Error::AssetNotFrozen => write!(f, "Asset is not frozen"),
            Error::InsufficientEmergencySignatures => write!(f, "Insufficient emergency signatures"),
        }
    }
}

impl ContractError for Error {
    fn error_code(&self) -> u32 {
        match self {
            Error::Unauthorized => bridge_codes::BRIDGE_UNAUTHORIZED,
            Error::TokenNotFound => bridge_codes::BRIDGE_TOKEN_NOT_FOUND,
            Error::InvalidChain => bridge_codes::BRIDGE_INVALID_CHAIN,
            Error::BridgeNotSupported => bridge_codes::BRIDGE_NOT_SUPPORTED,
            Error::InsufficientSignatures => bridge_codes::BRIDGE_INSUFFICIENT_SIGNATURES,
            Error::RequestExpired => bridge_codes::BRIDGE_REQUEST_EXPIRED,
            Error::AlreadySigned => bridge_codes::BRIDGE_ALREADY_SIGNED,
            Error::InvalidRequest => bridge_codes::BRIDGE_INVALID_REQUEST,
            Error::BridgePaused => bridge_codes::BRIDGE_PAUSED,
            Error::InvalidMetadata => bridge_codes::BRIDGE_INVALID_METADATA,
            Error::DuplicateRequest => bridge_codes::BRIDGE_DUPLICATE_REQUEST,
            Error::GasLimitExceeded => bridge_codes::BRIDGE_GAS_LIMIT_EXCEEDED,
            Error::RateLimitExceeded => bridge_codes::BRIDGE_RATE_LIMIT_EXCEEDED,
            Error::ReentrantCall => bridge_codes::REENTRANT_CALL,
            Error::TransactionNotFound => bridge_codes::BRIDGE_TRANSACTION_NOT_FOUND,
            Error::InvalidStatusTransition => bridge_codes::BRIDGE_INVALID_STATUS_TRANSITION,
            Error::OperationPaused => bridge_codes::BRIDGE_OPERATION_PAUSED,
            Error::NotGuardian => bridge_codes::BRIDGE_NOT_GUARDIAN,
            Error::TravelRuleDataRequired => bridge_codes::BRIDGE_TRAVEL_RULE_DATA_REQUIRED,
            Error::TravelRuleDataAlreadySubmitted => bridge_codes::BRIDGE_TRAVEL_RULE_DATA_ALREADY_SUBMITTED,
            Error::NotEmergencySigner => bridge_codes::BRIDGE_UNAUTHORIZED,
            Error::EmergencyRequestAlreadyExecuted => bridge_codes::BRIDGE_INVALID_REQUEST,
            Error::EmergencyRequestExpired => bridge_codes::BRIDGE_REQUEST_EXPIRED,
            Error::AssetAlreadyFrozen => bridge_codes::BRIDGE_INVALID_REQUEST,
            Error::AssetNotFrozen => bridge_codes::BRIDGE_INVALID_REQUEST,
            Error::InsufficientEmergencySignatures => bridge_codes::BRIDGE_INSUFFICIENT_SIGNATURES,
        }
    }

    fn error_description(&self) -> &'static str {
        match self {
            Error::Unauthorized => "Caller does not have permission to perform this operation",
            Error::TokenNotFound => "The specified token does not exist",
            Error::InvalidChain => "The destination chain ID is invalid",
            Error::BridgeNotSupported => "Cross-chain bridging is not supported for this token",
            Error::InsufficientSignatures => {
                "Not enough signatures collected for bridge operation"
            }
            Error::RequestExpired => {
                "The bridge request has expired and can no longer be executed"
            }
            Error::AlreadySigned => "You have already signed this bridge request",
            Error::InvalidRequest => "The bridge request is invalid or malformed",
            Error::BridgePaused => "Bridge operations are temporarily paused",
            Error::InvalidMetadata => "The token metadata is invalid",
            Error::DuplicateRequest => "A bridge request with these parameters already exists",
            Error::GasLimitExceeded => "The operation exceeded the gas limit",
            Error::RateLimitExceeded => "The operation exceeded the daily rate limit",
            Error::ReentrantCall => "Reentrancy guard detected a reentrant call",
            Error::TransactionNotFound => {
                "No cross-chain transaction status record exists for the given identifier"
            }
            Error::InvalidStatusTransition => {
                "The requested per-chain status transition is not allowed from the current status"
            }
            Error::OperationPaused => {
                "The targeted bridge operation class has been emergency-paused"
            }
            Error::NotGuardian => {
                "The caller is not registered as a guardian and is not the admin"
            }
            Error::TravelRuleDataRequired => {
                "FATF travel rule data must be submitted before this bridge request can be executed"
            }
            Error::TravelRuleDataAlreadySubmitted => {
                "Travel rule data has already been submitted for this bridge request"
            }
            Error::NotEmergencySigner => {
                "The caller is not registered as an emergency signer"
            }
            Error::EmergencyRequestAlreadyExecuted => {
                "The emergency multi-sig request has already been executed"
            }
            Error::EmergencyRequestExpired => {
                "The emergency multi-sig request has expired"
            }
            Error::AssetAlreadyFrozen => {
                "The asset is already frozen"
            }
            Error::AssetNotFrozen => {
                "The asset is not frozen"
            }
            Error::InsufficientEmergencySignatures => {
                "Not enough emergency signatures collected for the operation"
            }
        }
    }

    fn error_category(&self) -> ErrorCategory {
        ErrorCategory::Bridge
    }
}
