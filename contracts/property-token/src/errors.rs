// Error types for the property token contract (Issue #101 - extracted from lib.rs)

/// Error types for the property token contract
#[derive(Debug, PartialEq, Eq, scale::Encode, scale::Decode)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub enum Error {
    // Standard ERC errors
    /// Token does not exist
    TokenNotFound,
    /// Caller is not authorized
    Unauthorized,
    // Property-specific errors
    /// Property does not exist
    PropertyNotFound,
    /// Metadata is invalid or malformed
    InvalidMetadata,
    /// Document does not exist
    DocumentNotFound,
    /// Compliance check failed
    ComplianceFailed,
    // Cross-chain bridge errors
    /// Bridge functionality not supported
    BridgeNotSupported,
    /// Invalid chain ID
    InvalidChain,
    /// Token is locked in bridge
    BridgeLocked,
    /// Insufficient signatures for bridge operation
    InsufficientSignatures,
    /// Bridge request has expired
    RequestExpired,
    /// Invalid bridge request
    InvalidRequest,
    /// Bridge operations are paused
    BridgePaused,
    /// Gas limit exceeded
    GasLimitExceeded,
    /// Metadata is corrupted
    MetadataCorruption,
    /// Invalid bridge operator
    InvalidBridgeOperator,
    /// Duplicate bridge request
    DuplicateBridgeRequest,
    /// Bridge operation timed out
    BridgeTimeout,
    /// Already signed this request
    AlreadySigned,
    /// Insufficient balance
    InsufficientBalance,
    /// Invalid amount
    InvalidAmount,
    /// Proposal not found
    ProposalNotFound,
    /// Proposal is closed
    ProposalClosed,
    /// Ask not found
    AskNotFound,
    /// Input batch exceeds maximum allowed size
    BatchSizeExceeded,
    /// No stake found for this account and token
    StakeNotFound,
    /// Stake lock period has not yet expired
    LockActive,
    /// No staking rewards available to claim
    NoRewards,
    /// Stake reward pool has insufficient funds
    InsufficientRewardPool,
    /// An active stake already exists for this account and token
    AlreadyStaked,
    /// Token IDs and amounts vectors have different lengths
    LengthMismatch,
}

impl core::fmt::Display for Error {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Error::TokenNotFound => write!(f, "Token does not exist"),
            Error::Unauthorized => write!(f, "Caller is not authorized"),
            Error::PropertyNotFound => write!(f, "Property does not exist"),
            Error::InvalidMetadata => write!(f, "Metadata is invalid or malformed"),
            Error::DocumentNotFound => write!(f, "Document does not exist"),
            Error::ComplianceFailed => write!(f, "Compliance check failed"),
            Error::BridgeNotSupported => write!(f, "Bridge functionality not supported"),
            Error::InvalidChain => write!(f, "Invalid chain ID"),
            Error::BridgeLocked => write!(f, "Token is locked in bridge"),
            Error::InsufficientSignatures => {
                write!(f, "Insufficient signatures for bridge operation")
            }
            Error::RequestExpired => write!(f, "Bridge request has expired"),
            Error::InvalidRequest => write!(f, "Invalid bridge request"),
            Error::BridgePaused => write!(f, "Bridge operations are paused"),
            Error::GasLimitExceeded => write!(f, "Gas limit exceeded"),
            Error::MetadataCorruption => write!(f, "Metadata is corrupted"),
            Error::InvalidBridgeOperator => write!(f, "Invalid bridge operator"),
            Error::DuplicateBridgeRequest => write!(f, "Duplicate bridge request"),
            Error::BridgeTimeout => write!(f, "Bridge operation timed out"),
            Error::AlreadySigned => write!(f, "Already signed this request"),
            Error::InsufficientBalance => write!(f, "Insufficient balance"),
            Error::InvalidAmount => write!(f, "Invalid amount"),
            Error::ProposalNotFound => write!(f, "Proposal not found"),
            Error::ProposalClosed => write!(f, "Proposal is closed"),
            Error::AskNotFound => write!(f, "Ask not found"),
            Error::BatchSizeExceeded => write!(f, "Input batch exceeds maximum allowed size"),
            Error::StakeNotFound => write!(f, "Stake not found"),
            Error::LockActive => write!(f, "Stake lock period is still active"),
            Error::NoRewards => write!(f, "No staking rewards available"),
            Error::InsufficientRewardPool => write!(f, "Insufficient reward pool balance"),
            Error::AlreadyStaked => write!(f, "An active stake already exists for this token"),
            Error::LengthMismatch => write!(f, "Token IDs and amounts length mismatch"),
        }
    }
}

impl ContractError for Error {
    fn error_code(&self) -> u32 {
        match self {
            Error::TokenNotFound => property_token_codes::TOKEN_NOT_FOUND,
            Error::Unauthorized => property_token_codes::UNAUTHORIZED_TRANSFER,
            Error::PropertyNotFound => property_token_codes::PROPERTY_NOT_FOUND,
            Error::InvalidMetadata => property_token_codes::INVALID_METADATA,
            Error::DocumentNotFound => property_token_codes::DOCUMENT_NOT_FOUND,
            Error::ComplianceFailed => property_token_codes::COMPLIANCE_FAILED,
            Error::BridgeNotSupported => property_token_codes::BRIDGE_NOT_SUPPORTED,
            Error::InvalidChain => property_token_codes::INVALID_CHAIN,
            Error::BridgeLocked => property_token_codes::BRIDGE_LOCKED,
            Error::InsufficientSignatures => property_token_codes::INSUFFICIENT_SIGNATURES,
            Error::RequestExpired => property_token_codes::REQUEST_EXPIRED,
            Error::InvalidRequest => property_token_codes::INVALID_REQUEST,
            Error::BridgePaused => property_token_codes::BRIDGE_PAUSED,
            Error::GasLimitExceeded => property_token_codes::GAS_LIMIT_EXCEEDED,
            Error::MetadataCorruption => property_token_codes::METADATA_CORRUPTION,
            Error::InvalidBridgeOperator => property_token_codes::INVALID_BRIDGE_OPERATOR,
            Error::DuplicateBridgeRequest => property_token_codes::DUPLICATE_BRIDGE_REQUEST,
            Error::BridgeTimeout => property_token_codes::BRIDGE_TIMEOUT,
            Error::AlreadySigned => property_token_codes::ALREADY_SIGNED,
            Error::InsufficientBalance => property_token_codes::INSUFFICIENT_BALANCE,
            Error::InvalidAmount => property_token_codes::INVALID_AMOUNT,
            Error::ProposalNotFound => property_token_codes::PROPOSAL_NOT_FOUND,
            Error::ProposalClosed => property_token_codes::PROPOSAL_CLOSED,
            Error::AskNotFound => property_token_codes::ASK_NOT_FOUND,
            Error::BatchSizeExceeded => property_token_codes::BATCH_SIZE_EXCEEDED,
            Error::StakeNotFound => property_token_codes::STAKE_NOT_FOUND,
            Error::LockActive => property_token_codes::LOCK_ACTIVE,
            Error::NoRewards => property_token_codes::NO_REWARDS,
            Error::InsufficientRewardPool => property_token_codes::INSUFFICIENT_REWARD_POOL,
            Error::AlreadyStaked => property_token_codes::ALREADY_STAKED,
            Error::LengthMismatch => property_token_codes::BATCH_SIZE_EXCEEDED,
        }
    }

    fn error_description(&self) -> &'static str {
        match self {
            Error::TokenNotFound => "The specified token does not exist",
            Error::Unauthorized => "Caller does not have permission to perform this operation",
            Error::PropertyNotFound => "The specified property does not exist",
            Error::InvalidMetadata => "The provided metadata is invalid or malformed",
            Error::DocumentNotFound => "The requested document does not exist",
            Error::ComplianceFailed => "The operation failed compliance verification",
            Error::BridgeNotSupported => "Cross-chain bridging is not supported for this token",
            Error::InvalidChain => "The destination chain ID is invalid",
            Error::BridgeLocked => "The token is currently locked in a bridge operation",
            Error::InsufficientSignatures => {
                "Not enough signatures collected for bridge operation"
            }
            Error::RequestExpired => {
                "The bridge request has expired and can no longer be executed"
            }
            Error::InvalidRequest => "The bridge request is invalid or malformed",
            Error::BridgePaused => "Bridge operations are temporarily paused",
            Error::GasLimitExceeded => "The operation exceeded the gas limit",
            Error::MetadataCorruption => "The token metadata has been corrupted",
            Error::InvalidBridgeOperator => "The bridge operator is not authorized",
            Error::DuplicateBridgeRequest => {
                "A bridge request with these parameters already exists"
            }
            Error::BridgeTimeout => "The bridge operation timed out",
            Error::AlreadySigned => "You have already signed this bridge request",
            Error::InsufficientBalance => "Account has insufficient balance",
            Error::InvalidAmount => "The amount is invalid or out of range",
            Error::ProposalNotFound => "The governance proposal does not exist",
            Error::ProposalClosed => "The governance proposal is closed for voting",
            Error::AskNotFound => "The sell ask does not exist",
            Error::LengthMismatch => "Token IDs and amounts vectors have different lengths",
            Error::BatchSizeExceeded => {
                "The input batch exceeds the maximum allowed size"
            }
            Error::StakeNotFound => "No active stake found for this account and token",
            Error::LockActive => {
                "The stake lock period has not yet expired; unstaking is not permitted"
            }
            Error::NoRewards => "There are no staking rewards available to claim at this time",
            Error::InsufficientRewardPool => {
                "The stake reward pool does not have enough funds to cover the claimed rewards"
            }
            Error::AlreadyStaked => {
                "An active stake already exists for this account and token; unstake first"
            }
        }
    }

    fn error_category(&self) -> ErrorCategory {
        ErrorCategory::PropertyToken
    }
}
