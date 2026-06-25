// Error types for the bridge contract (Issue #101 - extracted from lib.rs)

// ---------------------------------------------------------------------------
// Severity — how serious is this error?
// ---------------------------------------------------------------------------

/// Broad classification of how severe a bridge error is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    /// A caller mistake — bad input, wrong permissions, etc.
    User,
    /// Transient infrastructure issue that may resolve on retry.
    Transient,
    /// A hard system fault; retrying will not help.
    Fatal,
}

// ---------------------------------------------------------------------------
// Error enum
// ---------------------------------------------------------------------------

/// All error variants that the bridge contract can return.
///
/// Marked `#[non_exhaustive]` so that adding future variants is never a
/// breaking change for downstream crates that pattern-match on this type.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq, scale::Encode, scale::Decode)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub enum Error {
    // ── Authorization ───────────────────────────────────────────────────────
    /// Caller does not hold the required role.
    Unauthorized,
    /// Caller is not a registered guardian (and not the admin).
    NotGuardian,

    // ── Token / chain validation ────────────────────────────────────────────
    /// The specified token does not exist in the registry.
    TokenNotFound,
    /// The destination chain ID is not recognised.
    InvalidChain,
    /// Cross-chain bridging is not supported for this token/chain pair.
    BridgeNotSupported,

    // ── Request lifecycle ───────────────────────────────────────────────────
    /// The bridge request is malformed or logically inconsistent.
    InvalidRequest,
    /// A bridge request with these parameters was already submitted.
    DuplicateRequest,
    /// The bridge request has expired and can no longer be executed.
    RequestExpired,
    /// Caller has already signed this bridge request.
    AlreadySigned,
    /// Not enough guardian signatures have been collected yet.
    InsufficientSignatures,

    // ── Compliance ──────────────────────────────────────────────────────────
    /// FATF travel-rule data must be submitted before execution.
    TravelRuleDataRequired,
    /// Travel-rule data was already submitted for this request.
    TravelRuleDataAlreadySubmitted,
    /// Metadata attached to the token or request is invalid.
    InvalidMetadata,

    // ── Operational / infrastructure ────────────────────────────────────────
    /// Bridge operations (or a specific operation class) are paused.
    BridgePaused,
    /// The targeted operation class is emergency-paused.
    OperationPaused,
    /// The operation exceeded its gas allowance.
    GasLimitExceeded,
    /// The caller has exceeded the daily rate limit.
    RateLimitExceeded,
    /// Reentrancy guard detected a reentrant call.
    ReentrantCall,

    // ── Cross-chain tracking ────────────────────────────────────────────────
    /// No status record exists for the given cross-chain transaction ID.
    TransactionNotFound,
    /// The requested per-chain status transition is not allowed from the
    /// current state.
    InvalidStatusTransition,
}

impl Error {
    /// Returns `true` when the operation *might* succeed if retried later
    /// (e.g. transient infrastructure issues, rate limits).
    ///
    /// Callers can use this to drive automatic retry logic without
    /// exhaustively pattern-matching every variant.
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            Error::InsufficientSignatures
                | Error::RateLimitExceeded
                | Error::BridgePaused
                | Error::OperationPaused
        )
    }

    /// Returns `true` for errors that are permanent — retrying will never help.
    pub fn is_fatal(&self) -> bool {
        matches!(
            self,
            Error::Unauthorized
                | Error::NotGuardian
                | Error::TokenNotFound
                | Error::InvalidChain
                | Error::BridgeNotSupported
                | Error::DuplicateRequest
                | Error::AlreadySigned
                | Error::ReentrantCall
                | Error::InvalidStatusTransition
                | Error::TravelRuleDataAlreadySubmitted
        )
    }

    /// Broad severity classification for logging and alerting pipelines.
    pub fn severity(&self) -> Severity {
        match self {
            // Caller mistakes — not our problem to alert on
            Error::Unauthorized
            | Error::NotGuardian
            | Error::TokenNotFound
            | Error::InvalidChain
            | Error::BridgeNotSupported
            | Error::InvalidRequest
            | Error::DuplicateRequest
            | Error::RequestExpired
            | Error::AlreadySigned
            | Error::TravelRuleDataRequired
            | Error::TravelRuleDataAlreadySubmitted
            | Error::InvalidMetadata
            | Error::TransactionNotFound
            | Error::InvalidStatusTransition => Severity::User,

            // May resolve without intervention
            Error::InsufficientSignatures
            | Error::BridgePaused
            | Error::OperationPaused
            | Error::RateLimitExceeded => Severity::Transient,

            // Hard system faults — page someone
            Error::GasLimitExceeded | Error::ReentrantCall => Severity::Fatal,
        }
    }

    /// Returns `true` when this error is compliance-related.
    ///
    /// Useful for routing errors to a compliance audit log separately from
    /// the main operational log.
    pub fn is_compliance_error(&self) -> bool {
        matches!(
            self,
            Error::TravelRuleDataRequired
                | Error::TravelRuleDataAlreadySubmitted
                | Error::InvalidMetadata
        )
    }
}

// ---------------------------------------------------------------------------
// Display
// ---------------------------------------------------------------------------

impl core::fmt::Display for Error {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        // Delegate to the richer description rather than duplicating strings.
        f.write_str(ContractError::error_description(self))
    }
}

// ---------------------------------------------------------------------------
// ContractError
// ---------------------------------------------------------------------------

impl ContractError for Error {
    fn error_code(&self) -> u32 {
        use bridge_codes::*;
        match self {
            Error::Unauthorized => BRIDGE_UNAUTHORIZED,
            Error::NotGuardian => BRIDGE_NOT_GUARDIAN,
            Error::TokenNotFound => BRIDGE_TOKEN_NOT_FOUND,
            Error::InvalidChain => BRIDGE_INVALID_CHAIN,
            Error::BridgeNotSupported => BRIDGE_NOT_SUPPORTED,
            Error::InvalidRequest => BRIDGE_INVALID_REQUEST,
            Error::DuplicateRequest => BRIDGE_DUPLICATE_REQUEST,
            Error::RequestExpired => BRIDGE_REQUEST_EXPIRED,
            Error::AlreadySigned => BRIDGE_ALREADY_SIGNED,
            Error::InsufficientSignatures => BRIDGE_INSUFFICIENT_SIGNATURES,
            Error::TravelRuleDataRequired => BRIDGE_TRAVEL_RULE_DATA_REQUIRED,
            Error::TravelRuleDataAlreadySubmitted => BRIDGE_TRAVEL_RULE_DATA_ALREADY_SUBMITTED,
            Error::InvalidMetadata => BRIDGE_INVALID_METADATA,
            Error::BridgePaused => BRIDGE_PAUSED,
            Error::OperationPaused => BRIDGE_OPERATION_PAUSED,
            Error::GasLimitExceeded => BRIDGE_GAS_LIMIT_EXCEEDED,
            Error::RateLimitExceeded => BRIDGE_RATE_LIMIT_EXCEEDED,
            Error::ReentrantCall => REENTRANT_CALL,
            Error::TransactionNotFound => BRIDGE_TRANSACTION_NOT_FOUND,
            Error::InvalidStatusTransition => BRIDGE_INVALID_STATUS_TRANSITION,
        }
    }

    fn error_description(&self) -> &'static str {
        match self {
            Error::Unauthorized =>
                "Caller does not have permission to perform this operation",
            Error::NotGuardian =>
                "Caller is not registered as a guardian and is not the admin",
            Error::TokenNotFound =>
                "The specified token does not exist in the registry",
            Error::InvalidChain =>
                "The destination chain ID is not recognised",
            Error::BridgeNotSupported =>
                "Cross-chain bridging is not supported for this token/chain pair",
            Error::InvalidRequest =>
                "The bridge request is invalid or malformed",
            Error::DuplicateRequest =>
                "A bridge request with these parameters already exists",
            Error::RequestExpired =>
                "The bridge request has expired and can no longer be executed",
            Error::AlreadySigned =>
                "You have already signed this bridge request",
            Error::InsufficientSignatures =>
                "Not enough guardian signatures collected for bridge execution",
            Error::TravelRuleDataRequired =>
                "FATF travel-rule data must be submitted before this bridge request can be executed",
            Error::TravelRuleDataAlreadySubmitted =>
                "Travel-rule data has already been submitted for this bridge request",
            Error::InvalidMetadata =>
                "The token or request metadata is invalid",
            Error::BridgePaused =>
                "Bridge operations are temporarily paused",
            Error::OperationPaused =>
                "The targeted bridge operation class has been emergency-paused",
            Error::GasLimitExceeded =>
                "The operation exceeded its gas allowance",
            Error::RateLimitExceeded =>
                "The caller has exceeded the daily rate limit",
            Error::ReentrantCall =>
                "Reentrancy guard detected a reentrant call — this is a contract bug",
            Error::TransactionNotFound =>
                "No cross-chain transaction status record exists for the given identifier",
            Error::InvalidStatusTransition =>
                "The requested per-chain status transition is not allowed from the current status",
        }
    }

    fn error_category(&self) -> ErrorCategory {
        ErrorCategory::Bridge
    }
}

// ---------------------------------------------------------------------------
// ErrorContext — attach runtime detail without bloating the enum
// ---------------------------------------------------------------------------

/// Wraps a [`Error`] with optional contextual information gathered at the
/// call site (request ID, chain ID, caller address, etc.).
///
/// Use this inside contract internals for richer logging; return the plain
/// [`Error`] across the ABI boundary.
///
/// # Example
///
/// ```rust
/// let ctx = ErrorContext::new(Error::RateLimitExceeded)
///     .with_request_id(request_id)
///     .with_detail("limit=100,window=86400s");
/// log::warn!("{ctx}");
/// return Err(ctx.into_error());
/// ```
#[derive(Debug)]
pub struct ErrorContext {
    error: Error,
    request_id: Option<[u8; 32]>,
    chain_id: Option<u64>,
    detail: Option<&'static str>,
}

impl ErrorContext {
    /// Wrap `error` with no additional context.
    pub fn new(error: Error) -> Self {
        Self { error, request_id: None, chain_id: None, detail: None }
    }

    /// Attach the cross-chain request ID (32-byte hash).
    pub fn with_request_id(mut self, id: [u8; 32]) -> Self {
        self.request_id = Some(id);
        self
    }

    /// Attach the destination chain ID.
    pub fn with_chain_id(mut self, id: u64) -> Self {
        self.chain_id = Some(id);
        self
    }

    /// Attach a short, `'static` free-text annotation.
    pub fn with_detail(mut self, detail: &'static str) -> Self {
        self.detail = Some(detail);
        self
    }

    /// Consume the context and return the inner [`Error`] for the ABI boundary.
    pub fn into_error(self) -> Error {
        self.error
    }

    /// Borrow the inner error without consuming the context.
    pub fn error(&self) -> &Error {
        &self.error
    }
}

impl core::fmt::Display for ErrorContext {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "[bridge_error code={}", ContractError::error_code(&self.error))?;
        if let Some(ref id) = self.request_id {
            write!(f, " request_id={}", hex::encode(id))?;
        }
        if let Some(chain) = self.chain_id {
            write!(f, " chain_id={chain}")?;
        }
        write!(f, "] {}", self.error)?;
        if let Some(detail) = self.detail {
            write!(f, " ({detail})")?;
        }
        Ok(())
    }
}

impl From<ErrorContext> for Error {
    fn from(ctx: ErrorContext) -> Self {
        ctx.into_error()
    }
}

// ---------------------------------------------------------------------------
// From impls for common upstream error types
// ---------------------------------------------------------------------------

/// Map scale codec errors to [`Error::InvalidRequest`].
///
/// Lets you use `?` when decoding ABI-encoded payloads inside a function that
/// returns `Result<_, Error>`.
impl From<scale::Error> for Error {
    fn from(_: scale::Error) -> Self {
        Error::InvalidRequest
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn retryable_variants_are_correct() {
        assert!(Error::RateLimitExceeded.is_retryable());
        assert!(Error::BridgePaused.is_retryable());
        assert!(Error::OperationPaused.is_retryable());
        assert!(Error::InsufficientSignatures.is_retryable());

        assert!(!Error::Unauthorized.is_retryable());
        assert!(!Error::ReentrantCall.is_retryable());
        assert!(!Error::DuplicateRequest.is_retryable());
    }

    #[test]
    fn fatal_variants_are_correct() {
        assert!(Error::Unauthorized.is_fatal());
        assert!(Error::ReentrantCall.is_fatal());
        assert!(Error::AlreadySigned.is_fatal());

        assert!(!Error::RateLimitExceeded.is_fatal());
        assert!(!Error::BridgePaused.is_fatal());
    }

    #[test]
    fn retryable_and_fatal_are_mutually_exclusive() {
        let all = [
            Error::Unauthorized, Error::NotGuardian, Error::TokenNotFound,
            Error::InvalidChain, Error::BridgeNotSupported, Error::InvalidRequest,
            Error::DuplicateRequest, Error::RequestExpired, Error::AlreadySigned,
            Error::InsufficientSignatures, Error::TravelRuleDataRequired,
            Error::TravelRuleDataAlreadySubmitted, Error::InvalidMetadata,
            Error::BridgePaused, Error::OperationPaused, Error::GasLimitExceeded,
            Error::RateLimitExceeded, Error::ReentrantCall,
            Error::TransactionNotFound, Error::InvalidStatusTransition,
        ];
        for e in &all {
            assert!(
                !(e.is_retryable() && e.is_fatal()),
                "{e:?} cannot be both retryable and fatal"
            );
        }
    }

    #[test]
    fn compliance_errors_flagged_correctly() {
        assert!(Error::TravelRuleDataRequired.is_compliance_error());
        assert!(Error::TravelRuleDataAlreadySubmitted.is_compliance_error());
        assert!(Error::InvalidMetadata.is_compliance_error());
        assert!(!Error::Unauthorized.is_compliance_error());
    }

    #[test]
    fn severity_spot_checks() {
        assert_eq!(Error::Unauthorized.severity(), Severity::User);
        assert_eq!(Error::RateLimitExceeded.severity(), Severity::Transient);
        assert_eq!(Error::ReentrantCall.severity(), Severity::Fatal);
    }

    #[test]
    fn display_delegates_to_description() {
        let e = Error::RequestExpired;
        assert_eq!(
            e.to_string(),
            ContractError::error_description(&e)
        );
    }

    #[test]
    fn error_context_display_contains_code_and_detail() {
        let ctx = ErrorContext::new(Error::RateLimitExceeded)
            .with_chain_id(42)
            .with_detail("limit=100");
        let s = ctx.to_string();
        assert!(s.contains("chain_id=42"));
        assert!(s.contains("limit=100"));
        assert!(s.contains("rate limit"));
    }

    #[test]
    fn error_context_into_error_roundtrip() {
        let ctx = ErrorContext::new(Error::DuplicateRequest);
        assert_eq!(ctx.into_error(), Error::DuplicateRequest);
    }
}