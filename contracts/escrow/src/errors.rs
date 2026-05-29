// Error types for the escrow contract (Issue #101 - extracted from lib.rs)

#[derive(Debug, PartialEq, Eq, scale::Encode, scale::Decode)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub enum Error {
    EscrowNotFound,
    Unauthorized,
    InvalidStatus,
    InsufficientFunds,
    ConditionsNotMet,
    SignatureThresholdNotMet,
    AlreadySigned,
    DocumentNotFound,
    DisputeActive,
    TimeLockActive,
    InvalidConfiguration,
    EscrowAlreadyFunded,
    ParticipantNotFound,
    ReentrantCall,
    /// A large-transfer approval request was not found
    ApprovalRequestNotFound,
    /// The large-transfer approval request has expired
    ApprovalRequestExpired,
    /// The large-transfer approval request was already executed
    ApprovalRequestAlreadyExecuted,
    /// The large-transfer approval request was cancelled
    ApprovalRequestCancelled,
    /// Transfer amount exceeds the large-transfer threshold and requires multi-step approval
    LargeTransferApprovalRequired,
}

impl core::fmt::Display for Error {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Error::EscrowNotFound => write!(f, "Escrow does not exist"),
            Error::Unauthorized => write!(f, "Caller is not authorized"),
            Error::InvalidStatus => write!(f, "Invalid escrow status for operation"),
            Error::InsufficientFunds => write!(f, "Insufficient funds in escrow"),
            Error::ConditionsNotMet => write!(f, "Required conditions not met"),
            Error::SignatureThresholdNotMet => write!(f, "Signature threshold not reached"),
            Error::AlreadySigned => write!(f, "Already signed this request"),
            Error::DocumentNotFound => write!(f, "Document does not exist"),
            Error::DisputeActive => write!(f, "Dispute is currently active"),
            Error::TimeLockActive => write!(f, "Time lock period still active"),
            Error::InvalidConfiguration => write!(f, "Invalid configuration parameters"),
            Error::EscrowAlreadyFunded => write!(f, "Escrow already funded"),
            Error::ParticipantNotFound => write!(f, "Participant not found"),
            Error::ReentrantCall => write!(f, "Reentrant call"),
            Error::ApprovalRequestNotFound => write!(f, "Large-transfer approval request not found"),
            Error::ApprovalRequestExpired => write!(f, "Large-transfer approval request has expired"),
            Error::ApprovalRequestAlreadyExecuted => write!(f, "Large-transfer approval request already executed"),
            Error::ApprovalRequestCancelled => write!(f, "Large-transfer approval request was cancelled"),
            Error::LargeTransferApprovalRequired => write!(f, "Transfer requires multi-step approval due to large amount"),
        }
    }
}

impl ContractError for Error {
    fn error_code(&self) -> u32 {
        match self {
            Error::EscrowNotFound => propchain_traits::errors::escrow_codes::ESCROW_NOT_FOUND,
            Error::Unauthorized => propchain_traits::errors::escrow_codes::UNAUTHORIZED_ACCESS,
            Error::InvalidStatus => propchain_traits::errors::escrow_codes::INVALID_STATUS,
            Error::InsufficientFunds => {
                propchain_traits::errors::escrow_codes::INSUFFICIENT_ESCROW_FUNDS
            }
            Error::ConditionsNotMet => {
                propchain_traits::errors::escrow_codes::CONDITIONS_NOT_MET
            }
            Error::SignatureThresholdNotMet => {
                propchain_traits::errors::escrow_codes::SIGNATURE_THRESHOLD_NOT_MET
            }
            Error::AlreadySigned => {
                propchain_traits::errors::escrow_codes::ALREADY_SIGNED_ESCROW
            }
            Error::DocumentNotFound => {
                propchain_traits::errors::escrow_codes::DOCUMENT_NOT_FOUND
            }
            Error::DisputeActive => propchain_traits::errors::escrow_codes::DISPUTE_ACTIVE,
            Error::TimeLockActive => propchain_traits::errors::escrow_codes::TIME_LOCK_ACTIVE,
            Error::InvalidConfiguration => {
                propchain_traits::errors::escrow_codes::INVALID_CONFIGURATION
            }
            Error::EscrowAlreadyFunded => {
                propchain_traits::errors::escrow_codes::ESCROW_ALREADY_FUNDED
            }
            Error::ParticipantNotFound => {
                propchain_traits::errors::escrow_codes::PARTICIPANT_NOT_FOUND
            }
            Error::ReentrantCall => propchain_traits::errors::escrow_codes::REENTRANT_CALL,
            Error::ApprovalRequestNotFound => {
                propchain_traits::errors::escrow_codes::APPROVAL_REQUEST_NOT_FOUND
            }
            Error::ApprovalRequestExpired => {
                propchain_traits::errors::escrow_codes::APPROVAL_REQUEST_EXPIRED
            }
            Error::ApprovalRequestAlreadyExecuted => {
                propchain_traits::errors::escrow_codes::APPROVAL_REQUEST_ALREADY_EXECUTED
            }
            Error::ApprovalRequestCancelled => {
                propchain_traits::errors::escrow_codes::APPROVAL_REQUEST_CANCELLED
            }
            Error::LargeTransferApprovalRequired => {
                propchain_traits::errors::escrow_codes::LARGE_TRANSFER_APPROVAL_REQUIRED
            }
        }
    }

    fn error_description(&self) -> &'static str {
        match self {
            Error::EscrowNotFound => "The specified escrow does not exist",
            Error::Unauthorized => "Caller does not have permission to perform this operation",
            Error::InvalidStatus => {
                "The escrow is not in the required state for this operation"
            }
            Error::InsufficientFunds => "The escrow does not have sufficient funds",
            Error::ConditionsNotMet => "Not all required conditions have been met",
            Error::SignatureThresholdNotMet => "Insufficient signatures collected",
            Error::AlreadySigned => "You have already signed this request",
            Error::DocumentNotFound => "The requested document does not exist",
            Error::DisputeActive => "A dispute is currently active on this escrow",
            Error::TimeLockActive => "The time lock period has not yet expired",
            Error::InvalidConfiguration => "The escrow configuration is invalid",
            Error::EscrowAlreadyFunded => "This escrow has already been funded",
            Error::ParticipantNotFound => "The specified participant is not in the escrow",
            Error::ReentrantCall => "Reentrancy guard detected a reentrant call",
            Error::ApprovalRequestNotFound => "The large-transfer approval request does not exist",
            Error::ApprovalRequestExpired => "The large-transfer approval request has expired",
            Error::ApprovalRequestAlreadyExecuted => {
                "The large-transfer approval request has already been executed"
            }
            Error::ApprovalRequestCancelled => {
                "The large-transfer approval request has been cancelled"
            }
            Error::LargeTransferApprovalRequired => {
                "Transfer amount exceeds the large-transfer threshold and requires multi-step approval"
            }
        }
    }

    fn error_category(&self) -> ErrorCategory {
        ErrorCategory::Escrow
    }
}
