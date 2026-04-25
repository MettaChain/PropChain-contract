//! Shared error handling framework for PropChain contracts
//!
//! This module provides a unified error handling system with:
//! - Base error trait that all contract errors implement
//! - Common error variants reusable across contracts
//! - Numeric error codes for external API integration
//! - Full Debug, Display, and From trait implementations
//! - [`ErrorMessage`]: structured error snapshot combining code, category, message, and i18n key
//! - [`ContractError::to_error_message()`]: default method to produce an `ErrorMessage`
//! - [`ContractError::error_i18n_key()`]: default method returning a localization key

use core::fmt;
use scale::{Decode, Encode};

#[cfg(feature = "std")]
use scale_info::TypeInfo;

// =============================================================================
// Standardized Error Message
// =============================================================================

/// Structured snapshot of all error information for a single error instance.
///
/// Suitable for logging and client-side display. All string fields are `&'static str`
/// for `no_std` / no-heap compatibility. This type is not SCALE-encoded since
/// `&'static str` does not implement `Decode`; use it purely in-memory.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ErrorMessage {
    /// Numeric error code, globally unique across all PropChain contracts.
    pub code: u32,
    /// Top-level domain that produced this error.
    pub category: ErrorCategory,
    /// Short human-readable message (matches `error_description`).
    pub message: &'static str,
    /// Longer technical description suitable for logs and developer tooling.
    pub description: &'static str,
    /// Dot-separated localization key for client-side message lookup.
    /// Format: `"<category>.<variant_snake_case>"`, e.g. `"compliance.not_verified"`.
    pub i18n_key: &'static str,
}

// =============================================================================
// Base Error Trait
// =============================================================================

/// Base trait for all PropChain contract errors.
/// All contract-specific error enums must implement this trait.
pub trait ContractError: fmt::Debug + fmt::Display + Encode + Decode {
    /// Returns the numeric error code for this error variant.
    /// Used for external API integration and monitoring.
    fn error_code(&self) -> u32;

    /// Returns a human-readable description of the error.
    fn error_description(&self) -> &'static str;

    /// Returns the category of this error.
    fn error_category(&self) -> ErrorCategory {
        match self.error_code() {
            1..=999 => ErrorCategory::Common,
            1000..=1999 => ErrorCategory::PropertyToken,
            2000..=2999 => ErrorCategory::Escrow,
            3000..=3999 => ErrorCategory::Bridge,
            4000..=4999 => ErrorCategory::Oracle,
            5000..=5999 => ErrorCategory::Fees,
            6000..=6999 => ErrorCategory::Compliance,
            7000..=7999 => ErrorCategory::Dex,
            8000..=8999 => ErrorCategory::Governance,
            9000..=9999 => ErrorCategory::Staking,
            10000..=10999 => ErrorCategory::Monitoring,
            11000..=11999 => ErrorCategory::EventBus,
            _ => ErrorCategory::Unknown,
        }
    }

    /// Returns a dot-separated localization key for client-side message lookup.
    ///
    /// Format: `"<category>.<variant_snake_case>"`, e.g. `"compliance.not_verified"`.
    /// Override this in each error type to provide a precise key.
    fn error_i18n_key(&self) -> &'static str {
        "unknown.error"
    }

    /// Constructs a complete [`ErrorMessage`] snapshot from this error.
    /// No allocation is performed; all fields are `'static`.
    fn to_error_message(&self) -> ErrorMessage {
        ErrorMessage {
            code: self.error_code(),
            category: self.error_category(),
            message: self.error_description(),
            description: self.error_description(),
            i18n_key: self.error_i18n_key(),
        }
    }
}

/// Error categories for classification and monitoring
#[derive(Debug, Clone, Copy, PartialEq, Eq, Encode, Decode)]
#[cfg_attr(feature = "std", derive(TypeInfo))]
pub enum ErrorCategory {
    Common,
    PropertyToken,
    Escrow,
    Bridge,
    Oracle,
    Fees,
    Compliance,
    Dex,
    Governance,
    Staking,
    Monitoring,
    EventBus,
    Unknown,
}

impl fmt::Display for ErrorCategory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ErrorCategory::Common => write!(f, "Common"),
            ErrorCategory::PropertyToken => write!(f, "PropertyToken"),
            ErrorCategory::Escrow => write!(f, "Escrow"),
            ErrorCategory::Bridge => write!(f, "Bridge"),
            ErrorCategory::Oracle => write!(f, "Oracle"),
            ErrorCategory::Fees => write!(f, "Fees"),
            ErrorCategory::Compliance => write!(f, "Compliance"),
            ErrorCategory::Dex => write!(f, "Dex"),
            ErrorCategory::Governance => write!(f, "Governance"),
            ErrorCategory::Staking => write!(f, "Staking"),
            ErrorCategory::Monitoring => write!(f, "Monitoring"),
            ErrorCategory::EventBus => write!(f, "EventBus"),
            ErrorCategory::Unknown => write!(f, "Unknown"),
        }
    }
}

// =============================================================================
// Common Error Variants
// =============================================================================

/// Common error variants that can be used across multiple contracts
#[derive(Debug, Clone, Copy, PartialEq, Eq, Encode, Decode)]
#[cfg_attr(feature = "std", derive(TypeInfo))]
pub enum CommonError {
    /// Unauthorized access - caller lacks required permissions
    Unauthorized = 1,
    /// Invalid parameters provided to function
    InvalidParameters = 2,
    /// Resource not found (generic)
    NotFound = 3,
    /// Insufficient funds or balance
    InsufficientFunds = 4,
    /// Operation not allowed in current state
    InvalidState = 5,
    /// Internal contract error
    InternalError = 6,
    /// Serialization/deserialization error
    CodecError = 7,
    /// Feature not yet implemented
    NotImplemented = 8,
    /// Operation timed out
    Timeout = 9,
    /// Duplicate operation or resource
    Duplicate = 10,
}

impl fmt::Display for CommonError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CommonError::Unauthorized => {
                write!(f, "Unauthorized: caller lacks required permissions")
            }
            CommonError::InvalidParameters => write!(f, "Invalid parameters provided to function"),
            CommonError::NotFound => write!(f, "Resource not found"),
            CommonError::InsufficientFunds => write!(f, "Insufficient funds or balance"),
            CommonError::InvalidState => write!(f, "Operation not allowed in current state"),
            CommonError::InternalError => write!(f, "Internal contract error occurred"),
            CommonError::CodecError => write!(f, "Serialization/deserialization error"),
            CommonError::NotImplemented => write!(f, "Feature not yet implemented"),
            CommonError::Timeout => write!(f, "Operation timed out"),
            CommonError::Duplicate => write!(f, "Duplicate operation or resource"),
        }
    }
}

impl ContractError for CommonError {
    fn error_code(&self) -> u32 {
        *self as u32
    }

    fn error_description(&self) -> &'static str {
        match self {
            CommonError::Unauthorized => {
                "Caller does not have permission to perform this operation"
            }
            CommonError::InvalidParameters => "One or more function parameters are invalid",
            CommonError::NotFound => "The requested resource does not exist",
            CommonError::InsufficientFunds => "Account has insufficient balance for this operation",
            CommonError::InvalidState => "Cannot perform this operation in the current state",
            CommonError::InternalError => "An internal error occurred in the contract",
            CommonError::CodecError => "Failed to encode or decode data",
            CommonError::NotImplemented => "This feature is not yet implemented",
            CommonError::Timeout => "The operation exceeded its time limit",
            CommonError::Duplicate => "This operation or resource already exists",
        }
    }

    fn error_category(&self) -> ErrorCategory {
        ErrorCategory::Common
    }

    fn error_i18n_key(&self) -> &'static str {
        match self {
            CommonError::Unauthorized => "common.unauthorized",
            CommonError::InvalidParameters => "common.invalid_parameters",
            CommonError::NotFound => "common.not_found",
            CommonError::InsufficientFunds => "common.insufficient_funds",
            CommonError::InvalidState => "common.invalid_state",
            CommonError::InternalError => "common.internal_error",
            CommonError::CodecError => "common.codec_error",
            CommonError::NotImplemented => "common.not_implemented",
            CommonError::Timeout => "common.timeout",
            CommonError::Duplicate => "common.duplicate",
        }
    }
}

// =============================================================================
// Error Code Constants
// =============================================================================

/// Common error codes (1-999)
pub mod common_codes {
    pub const UNAUTHORIZED: u32 = 1;
    pub const INVALID_PARAMETERS: u32 = 2;
    pub const NOT_FOUND: u32 = 3;
    pub const INSUFFICIENT_FUNDS: u32 = 4;
    pub const INVALID_STATE: u32 = 5;
    pub const INTERNAL_ERROR: u32 = 6;
    pub const CODEC_ERROR: u32 = 7;
    pub const NOT_IMPLEMENTED: u32 = 8;
    pub const TIMEOUT: u32 = 9;
    pub const DUPLICATE: u32 = 10;
}

/// PropertyToken error codes (1000-1999)
pub mod property_token_codes {
    pub const TOKEN_NOT_FOUND: u32 = 1001;
    pub const UNAUTHORIZED_TRANSFER: u32 = 1002;
    pub const PROPERTY_NOT_FOUND: u32 = 1003;
    pub const INVALID_METADATA: u32 = 1004;
    pub const DOCUMENT_NOT_FOUND: u32 = 1005;
    pub const COMPLIANCE_FAILED: u32 = 1006;
    pub const BRIDGE_NOT_SUPPORTED: u32 = 1007;
    pub const INVALID_CHAIN: u32 = 1008;
    pub const BRIDGE_LOCKED: u32 = 1009;
    pub const INSUFFICIENT_SIGNATURES: u32 = 1010;
    pub const REQUEST_EXPIRED: u32 = 1011;
    pub const INVALID_REQUEST: u32 = 1012;
    pub const BRIDGE_PAUSED: u32 = 1013;
    pub const GAS_LIMIT_EXCEEDED: u32 = 1014;
    pub const METADATA_CORRUPTION: u32 = 1015;
    pub const INVALID_BRIDGE_OPERATOR: u32 = 1016;
    pub const DUPLICATE_BRIDGE_REQUEST: u32 = 1017;
    pub const BRIDGE_TIMEOUT: u32 = 1018;
    pub const ALREADY_SIGNED: u32 = 1019;
    pub const INSUFFICIENT_BALANCE: u32 = 1020;
    pub const INVALID_AMOUNT: u32 = 1021;
    pub const PROPOSAL_NOT_FOUND: u32 = 1022;
    pub const PROPOSAL_CLOSED: u32 = 1023;
    pub const ASK_NOT_FOUND: u32 = 1024;
    pub const BATCH_SIZE_EXCEEDED: u32 = 1025;
    pub const STAKE_NOT_FOUND: u32 = 1026;
    pub const LOCK_ACTIVE: u32 = 1027;
    pub const NO_REWARDS: u32 = 1028;
    pub const INSUFFICIENT_REWARD_POOL: u32 = 1029;
    pub const ALREADY_STAKED: u32 = 1030;
    pub const REENTRANT_CALL: u32 = 1031;
}

/// Escrow error codes (2000-2999)
pub mod escrow_codes {
    pub const ESCROW_NOT_FOUND: u32 = 2001;
    pub const UNAUTHORIZED_ACCESS: u32 = 2002;
    pub const INVALID_STATUS: u32 = 2003;
    pub const INSUFFICIENT_ESCROW_FUNDS: u32 = 2004;
    pub const CONDITIONS_NOT_MET: u32 = 2005;
    pub const SIGNATURE_THRESHOLD_NOT_MET: u32 = 2006;
    pub const ALREADY_SIGNED_ESCROW: u32 = 2007;
    pub const DOCUMENT_NOT_FOUND: u32 = 2008;
    pub const DISPUTE_ACTIVE: u32 = 2009;
    pub const TIME_LOCK_ACTIVE: u32 = 2010;
    pub const INVALID_CONFIGURATION: u32 = 2011;
    pub const ESCROW_ALREADY_FUNDED: u32 = 2012;
    pub const PARTICIPANT_NOT_FOUND: u32 = 2013;
    pub const REENTRANT_CALL: u32 = 2014;
}

/// Bridge error codes (3000-3999)
pub mod bridge_codes {
    pub const BRIDGE_UNAUTHORIZED: u32 = 3001;
    pub const BRIDGE_TOKEN_NOT_FOUND: u32 = 3002;
    pub const BRIDGE_INVALID_CHAIN: u32 = 3003;
    pub const BRIDGE_NOT_SUPPORTED: u32 = 3004;
    pub const BRIDGE_INSUFFICIENT_SIGNATURES: u32 = 3005;
    pub const BRIDGE_REQUEST_EXPIRED: u32 = 3006;
    pub const BRIDGE_ALREADY_SIGNED: u32 = 3007;
    pub const BRIDGE_INVALID_REQUEST: u32 = 3008;
    pub const BRIDGE_PAUSED: u32 = 3009;
    pub const BRIDGE_INVALID_METADATA: u32 = 3010;
    pub const BRIDGE_DUPLICATE_REQUEST: u32 = 3011;
    pub const BRIDGE_GAS_LIMIT_EXCEEDED: u32 = 3012;
    pub const BRIDGE_RATE_LIMIT_EXCEEDED: u32 = 3013;
    pub const REENTRANT_CALL: u32 = 3014;
}

/// Oracle error codes (4000-4999)
pub mod oracle_codes {
    pub const ORACLE_PROPERTY_NOT_FOUND: u32 = 4001;
    pub const ORACLE_INSUFFICIENT_SOURCES: u32 = 4002;
    pub const ORACLE_INVALID_VALUATION: u32 = 4003;
    pub const ORACLE_UNAUTHORIZED: u32 = 4004;
    pub const ORACLE_SOURCE_NOT_FOUND: u32 = 4005;
    pub const ORACLE_INVALID_PARAMETERS: u32 = 4006;
    pub const ORACLE_PRICE_FEED_ERROR: u32 = 4007;
    pub const ORACLE_ALERT_NOT_FOUND: u32 = 4008;
    pub const ORACLE_INSUFFICIENT_REPUTATION: u32 = 4009;
    pub const ORACLE_SOURCE_ALREADY_EXISTS: u32 = 4010;
    pub const ORACLE_REQUEST_PENDING: u32 = 4011;
    pub const ORACLE_BATCH_SIZE_EXCEEDED: u32 = 4012;
}

/// Fee error codes (5000-5999)
pub mod fee_codes {
    pub const FEE_UNAUTHORIZED: u32 = 5001;
    pub const FEE_AUCTION_NOT_FOUND: u32 = 5002;
    pub const FEE_AUCTION_ENDED: u32 = 5003;
    pub const FEE_AUCTION_NOT_ENDED: u32 = 5004;
    pub const FEE_BID_TOO_LOW: u32 = 5005;
    pub const FEE_ALREADY_SETTLED: u32 = 5006;
    pub const FEE_INVALID_CONFIG: u32 = 5007;
    pub const FEE_INVALID_PROPERTY: u32 = 5008;
}

/// Compliance error codes (6000-6999)
pub mod compliance_codes {
    pub const COMPLIANCE_UNAUTHORIZED: u32 = 6001;
    pub const COMPLIANCE_CHECK_FAILED: u32 = 6002;
    pub const COMPLIANCE_NOT_VERIFIED: u32 = 6003;
    pub const COMPLIANCE_DOCUMENT_MISSING: u32 = 6004;
    pub const COMPLIANCE_EXPIRED: u32 = 6005;
    pub const COMPLIANCE_HIGH_RISK: u32 = 6006;
    pub const COMPLIANCE_PROHIBITED_JURISDICTION: u32 = 6007;
    pub const COMPLIANCE_ALREADY_VERIFIED: u32 = 6008;
    pub const COMPLIANCE_CONSENT_NOT_GIVEN: u32 = 6009;
    pub const COMPLIANCE_INVALID_RISK_SCORE: u32 = 6010;
    pub const COMPLIANCE_JURISDICTION_NOT_SUPPORTED: u32 = 6011;
    pub const COMPLIANCE_INVALID_DOCUMENT_TYPE: u32 = 6012;
    pub const COMPLIANCE_DATA_RETENTION_EXPIRED: u32 = 6013;
    pub const REENTRANT_CALL: u32 = 6014;
}

/// DEX error codes (7000-7999)
pub mod dex_codes {
    pub const DEX_UNAUTHORIZED: u32 = 7001;
    pub const DEX_INVALID_PAIR: u32 = 7002;
    pub const DEX_POOL_NOT_FOUND: u32 = 7003;
    pub const DEX_INSUFFICIENT_LIQUIDITY: u32 = 7004;
    pub const DEX_SLIPPAGE_EXCEEDED: u32 = 7005;
    pub const DEX_ORDER_NOT_FOUND: u32 = 7006;
    pub const DEX_INVALID_ORDER: u32 = 7007;
    pub const DEX_ORDER_NOT_EXECUTABLE: u32 = 7008;
    pub const DEX_REWARD_UNAVAILABLE: u32 = 7009;
    pub const DEX_PROPOSAL_NOT_FOUND: u32 = 7010;
    pub const DEX_PROPOSAL_CLOSED: u32 = 7011;
    pub const DEX_ALREADY_VOTED: u32 = 7012;
    pub const DEX_INVALID_BRIDGE_ROUTE: u32 = 7013;
    pub const DEX_CROSS_CHAIN_TRADE_NOT_FOUND: u32 = 7014;
    pub const DEX_INSUFFICIENT_GOVERNANCE_BALANCE: u32 = 7015;
    pub const REENTRANT_CALL: u32 = 7016;
    pub const DEX_INVALID_REQUEST: u32 = 7016;
    pub const DEX_TIMELOCK_REQUIRED: u32 = 7016;
    pub const DEX_TIMELOCK_ACTIVE: u32 = 7017;
    pub const DEX_ADMIN_ACTION_NOT_FOUND: u32 = 7018;
    pub const DEX_ADMIN_ACTION_ALREADY_FINALIZED: u32 = 7019;
}

/// Governance error codes (8000-8999)
pub mod governance_codes {
    pub const GOVERNANCE_UNAUTHORIZED: u32 = 8001;
    pub const GOVERNANCE_PROPOSAL_NOT_FOUND: u32 = 8002;
    pub const GOVERNANCE_ALREADY_VOTED: u32 = 8003;
    pub const GOVERNANCE_PROPOSAL_CLOSED: u32 = 8004;
    pub const GOVERNANCE_THRESHOLD_NOT_MET: u32 = 8005;
    pub const GOVERNANCE_TIMELOCK_ACTIVE: u32 = 8006;
    pub const GOVERNANCE_INVALID_THRESHOLD: u32 = 8007;
    pub const GOVERNANCE_SIGNER_EXISTS: u32 = 8008;
    pub const GOVERNANCE_SIGNER_NOT_FOUND: u32 = 8009;
    pub const GOVERNANCE_MIN_SIGNERS: u32 = 8010;
    pub const GOVERNANCE_MAX_PROPOSALS: u32 = 8011;
    pub const GOVERNANCE_NOT_A_SIGNER: u32 = 8012;
    pub const GOVERNANCE_PROPOSAL_EXPIRED: u32 = 8013;
}

/// Staking error codes (9000-9999)
pub mod staking_codes {
    pub const STAKING_UNAUTHORIZED: u32 = 9001;
    pub const STAKING_INSUFFICIENT_AMOUNT: u32 = 9002;
    pub const STAKING_NOT_FOUND: u32 = 9003;
    pub const STAKING_LOCK_ACTIVE: u32 = 9004;
    pub const STAKING_NO_REWARDS: u32 = 9005;
    pub const STAKING_INSUFFICIENT_POOL: u32 = 9006;
    pub const STAKING_INVALID_CONFIG: u32 = 9007;
    pub const STAKING_ALREADY_STAKED: u32 = 9008;
    pub const STAKING_INVALID_DELEGATE: u32 = 9009;
    pub const STAKING_ZERO_AMOUNT: u32 = 9010;
    pub const REENTRANT_CALL: u32 = 9011;
}

/// Monitoring error codes (10000-10999)
pub mod monitoring_codes {
    pub const MONITORING_UNAUTHORIZED: u32 = 10001;
    pub const MONITORING_CONTRACT_PAUSED: u32 = 10002;
    pub const MONITORING_INVALID_THRESHOLD: u32 = 10003;
    pub const MONITORING_SUBSCRIBER_LIMIT_REACHED: u32 = 10004;
    pub const MONITORING_SUBSCRIBER_NOT_FOUND: u32 = 10005;
}

/// EventBus error codes (11000-11999)
pub mod event_bus_codes {
    pub const EVENT_BUS_UNAUTHORIZED: u32 = 11001;
    pub const EVENT_BUS_TOPIC_NOT_FOUND: u32 = 11002;
    pub const EVENT_BUS_ALREADY_SUBSCRIBED: u32 = 11003;
    pub const EVENT_BUS_NOT_SUBSCRIBED: u32 = 11004;
    pub const EVENT_BUS_MAX_SUBSCRIBERS_REACHED: u32 = 11005;
    pub const EVENT_BUS_SUBSCRIBER_CALL_FAILED: u32 = 11006;
    pub const EVENT_BUS_REENTRANT_CALL: u32 = 11007;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn common_error_i18n_keys_are_correct() {
        assert_eq!(
            CommonError::Unauthorized.error_i18n_key(),
            "common.unauthorized"
        );
        assert_eq!(CommonError::NotFound.error_i18n_key(), "common.not_found");
        assert_eq!(CommonError::Duplicate.error_i18n_key(), "common.duplicate");
    }

    #[test]
    fn to_error_message_populates_all_fields() {
        let msg = CommonError::Unauthorized.to_error_message();
        assert_eq!(msg.code, common_codes::UNAUTHORIZED);
        assert_eq!(msg.category, ErrorCategory::Common);
        assert_eq!(msg.i18n_key, "common.unauthorized");
        assert!(!msg.description.is_empty());
    }

    #[test]
    fn oracle_batch_size_exceeded_constant_matches_value() {
        assert_eq!(oracle_codes::ORACLE_BATCH_SIZE_EXCEEDED, 4012);
    }

    #[test]
    fn compliance_codes_are_unique() {
        let mut codes = vec![
            compliance_codes::COMPLIANCE_UNAUTHORIZED,
            compliance_codes::COMPLIANCE_NOT_VERIFIED,
            compliance_codes::COMPLIANCE_CHECK_FAILED,
            compliance_codes::COMPLIANCE_DOCUMENT_MISSING,
            compliance_codes::COMPLIANCE_EXPIRED,
            compliance_codes::COMPLIANCE_HIGH_RISK,
            compliance_codes::COMPLIANCE_PROHIBITED_JURISDICTION,
            compliance_codes::COMPLIANCE_ALREADY_VERIFIED,
            compliance_codes::COMPLIANCE_CONSENT_NOT_GIVEN,
            compliance_codes::COMPLIANCE_INVALID_RISK_SCORE,
            compliance_codes::COMPLIANCE_JURISDICTION_NOT_SUPPORTED,
            compliance_codes::COMPLIANCE_INVALID_DOCUMENT_TYPE,
            compliance_codes::COMPLIANCE_DATA_RETENTION_EXPIRED,
            compliance_codes::REENTRANT_CALL,
        ];
        let len = codes.len();
        codes.sort();
        codes.dedup();
        assert_eq!(
            codes.len(),
            len,
            "duplicate compliance error codes detected"
        );
    }
}
