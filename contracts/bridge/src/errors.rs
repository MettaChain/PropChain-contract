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
        }
    }

    fn error_category(&self) -> ErrorCategory {
        ErrorCategory::Bridge
    }
}
