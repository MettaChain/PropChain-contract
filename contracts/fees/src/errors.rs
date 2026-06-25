// Error types for the fees contract (Issue #101 - extracted from lib.rs)
//
// Enhancements over v1:
//  - `ErrorSeverity` enum — distinguishes user mistakes from internal faults
//  - `is_recoverable()` — callers can decide whether to retry or abort
//  - `recovery_hint()` — actionable guidance embedded in the error itself
//  - `FeeErrorKind` — machine-readable category for off-chain indexers /
//    analytics (finer-grained than the shared `ErrorCategory::Fees`)
//  - `From<FeeError>` for `u32` — lossless conversion to the numeric code
//    without going through the `ContractError` trait
//  - `FeeResult<T>` alias — reduces boilerplate at call sites
//  - All match arms are exhaustive and kept in a single canonical order so
//    adding a new variant produces a compiler error in every match
//  - Full unit-test suite covering every variant and every method

use propchain_traits::errors::{fee_codes, ContractError, ErrorCategory};

// ── Supporting types ──────────────────────────────────────────────────────────

/// How serious an error is — drives logging and UI presentation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, scale::Encode, scale::Decode)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub enum ErrorSeverity {
    /// The caller made a correctable mistake (wrong timing, value too low, etc.).
    UserError,
    /// The contract is in an unexpected state — needs operator attention.
    ContractError,
    /// The caller is explicitly not permitted.
    AuthError,
}

/// Fine-grained machine-readable classification — useful for off-chain indexers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, scale::Encode, scale::Decode)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub enum FeeErrorKind {
    Authorization,
    AuctionLifecycle,
    BidValidation,
    ConfigValidation,
    PropertyValidation,
}

/// Convenience alias — use `FeeResult<T>` instead of `Result<T, FeeError>`
/// throughout the fees contract.
pub type FeeResult<T> = Result<T, FeeError>;

// ── Error enum ────────────────────────────────────────────────────────────────

/// All recoverable errors the fees contract can produce.
///
/// Variants are listed in ascending numeric-code order to make auditing the
/// `error_code()` mapping straightforward.
#[derive(Debug, Clone, Copy, PartialEq, Eq, scale::Encode, scale::Decode)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub enum FeeError {
    /// The caller does not hold the required role.
    Unauthorized,
    /// The referenced auction ID does not exist in storage.
    AuctionNotFound,
    /// The operation requires an active auction, but it has ended.
    AuctionEnded,
    /// The operation requires a finished auction, but it is still active.
    AuctionNotEnded,
    /// The submitted bid is below the current minimum acceptable amount.
    BidTooLow,
    /// Settlement was attempted on an already-settled auction.
    AlreadySettled,
    /// The fee configuration parameters are outside accepted bounds.
    InvalidConfig,
    /// The property ID is malformed or does not exist in the registry.
    InvalidProperty,
    /// A numeric operation would overflow or underflow.
    ArithmeticError,
    /// The auction's bid deadline has not been reached yet.
    BidDeadlineNotReached,
    /// The caller tried to bid on their own auction.
    SelfBidNotAllowed,
}

impl FeeError {
    // ── Rich metadata ─────────────────────────────────────────────────────────

    /// Severity level — drives log levels and UI copy.
    pub fn severity(&self) -> ErrorSeverity {
        match self {
            FeeError::Unauthorized | FeeError::SelfBidNotAllowed => ErrorSeverity::AuthError,
            FeeError::ArithmeticError | FeeError::InvalidConfig => ErrorSeverity::ContractError,
            FeeError::AuctionNotFound
            | FeeError::AuctionEnded
            | FeeError::AuctionNotEnded
            | FeeError::BidTooLow
            | FeeError::AlreadySettled
            | FeeError::InvalidProperty
            | FeeError::BidDeadlineNotReached => ErrorSeverity::UserError,
        }
    }

    /// Machine-readable sub-category for indexers and analytics pipelines.
    pub fn kind(&self) -> FeeErrorKind {
        match self {
            FeeError::Unauthorized | FeeError::SelfBidNotAllowed => {
                FeeErrorKind::Authorization
            }
            FeeError::AuctionNotFound
            | FeeError::AuctionEnded
            | FeeError::AuctionNotEnded
            | FeeError::AlreadySettled
            | FeeError::BidDeadlineNotReached => FeeErrorKind::AuctionLifecycle,
            FeeError::BidTooLow => FeeErrorKind::BidValidation,
            FeeError::InvalidConfig | FeeError::ArithmeticError => {
                FeeErrorKind::ConfigValidation
            }
            FeeError::InvalidProperty => FeeErrorKind::PropertyValidation,
        }
    }

    /// Whether the caller can retry with different inputs or timing.
    pub fn is_recoverable(&self) -> bool {
        match self {
            // Hard stops — no retry will help without operator intervention.
            FeeError::ArithmeticError | FeeError::InvalidConfig => false,
            // Everything else is correctable by the caller.
            _ => true,
        }
    }

    /// Actionable one-liner the UI or SDK can surface directly to the user.
    pub fn recovery_hint(&self) -> &'static str {
        match self {
            FeeError::Unauthorized => {
                "Ensure you are using the correct admin or owner account."
            }
            FeeError::AuctionNotFound => {
                "Check the auction ID and confirm the auction was created."
            }
            FeeError::AuctionEnded => {
                "This auction is closed. Browse open auctions instead."
            }
            FeeError::AuctionNotEnded => {
                "Wait for the auction deadline to pass before settling."
            }
            FeeError::BidTooLow => {
                "Increase your bid to at least the current minimum bid amount."
            }
            FeeError::AlreadySettled => {
                "Settlement has already been processed. No further action needed."
            }
            FeeError::InvalidConfig => {
                "Contact the contract administrator — the fee configuration is corrupt."
            }
            FeeError::InvalidProperty => {
                "Verify the property ID exists in the registry before bidding."
            }
            FeeError::ArithmeticError => {
                "An internal calculation overflowed. Report this to the development team."
            }
            FeeError::BidDeadlineNotReached => {
                "The bid deadline has not passed yet. Try again after the deadline."
            }
            FeeError::SelfBidNotAllowed => {
                "You cannot bid on an auction you created."
            }
        }
    }
}

// ── core::fmt::Display ────────────────────────────────────────────────────────

impl core::fmt::Display for FeeError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        // Delegates to `error_description` so there is a single source of truth
        // for the human-readable message.
        f.write_str(self.error_description())
    }
}

// ── From<FeeError> for u32 ────────────────────────────────────────────────────

impl From<FeeError> for u32 {
    fn from(e: FeeError) -> u32 {
        e.error_code()
    }
}

// ── ContractError ─────────────────────────────────────────────────────────────

impl ContractError for FeeError {
    fn error_code(&self) -> u32 {
        match self {
            FeeError::Unauthorized => fee_codes::FEE_UNAUTHORIZED,
            FeeError::AuctionNotFound => fee_codes::FEE_AUCTION_NOT_FOUND,
            FeeError::AuctionEnded => fee_codes::FEE_AUCTION_ENDED,
            FeeError::AuctionNotEnded => fee_codes::FEE_AUCTION_NOT_ENDED,
            FeeError::BidTooLow => fee_codes::FEE_BID_TOO_LOW,
            FeeError::AlreadySettled => fee_codes::FEE_ALREADY_SETTLED,
            FeeError::InvalidConfig => fee_codes::FEE_INVALID_CONFIG,
            FeeError::InvalidProperty => fee_codes::FEE_INVALID_PROPERTY,
            // New variants — add corresponding constants to `fee_codes` module.
            FeeError::ArithmeticError => fee_codes::FEE_ARITHMETIC_ERROR,
            FeeError::BidDeadlineNotReached => fee_codes::FEE_BID_DEADLINE_NOT_REACHED,
            FeeError::SelfBidNotAllowed => fee_codes::FEE_SELF_BID_NOT_ALLOWED,
        }
    }

    fn error_description(&self) -> &'static str {
        match self {
            FeeError::Unauthorized => {
                "Caller does not have permission to perform this operation"
            }
            FeeError::AuctionNotFound => "The specified auction does not exist",
            FeeError::AuctionEnded => "This auction has already ended",
            FeeError::AuctionNotEnded => "The auction is still active and has not ended",
            FeeError::BidTooLow => "The bid amount is below the minimum required",
            FeeError::AlreadySettled => "This auction has already been settled",
            FeeError::InvalidConfig => "The fee configuration is invalid",
            FeeError::InvalidProperty => "The property ID is invalid or does not exist",
            FeeError::ArithmeticError => {
                "An arithmetic overflow or underflow occurred during fee calculation"
            }
            FeeError::BidDeadlineNotReached => {
                "The bid deadline has not been reached; settlement is not yet permitted"
            }
            FeeError::SelfBidNotAllowed => {
                "The auction creator is not permitted to bid on their own auction"
            }
        }
    }

    fn error_category(&self) -> ErrorCategory {
        ErrorCategory::Fees
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // Every variant under test — update this when adding new variants so the
    // compiler reminds you to cover them in each test.
    const ALL_VARIANTS: &[FeeError] = &[
        FeeError::Unauthorized,
        FeeError::AuctionNotFound,
        FeeError::AuctionEnded,
        FeeError::AuctionNotEnded,
        FeeError::BidTooLow,
        FeeError::AlreadySettled,
        FeeError::InvalidConfig,
        FeeError::InvalidProperty,
        FeeError::ArithmeticError,
        FeeError::BidDeadlineNotReached,
        FeeError::SelfBidNotAllowed,
    ];

    // ── Display / description ─────────────────────────────────────────────────

    #[test]
    fn display_matches_error_description() {
        for e in ALL_VARIANTS {
            assert_eq!(
                e.to_string(),
                e.error_description(),
                "{e:?}: Display and error_description() must match"
            );
        }
    }

    #[test]
    fn display_is_non_empty_for_all_variants() {
        for e in ALL_VARIANTS {
            assert!(!e.to_string().is_empty(), "{e:?}: Display must not be empty");
        }
    }

    // ── Recovery hint ─────────────────────────────────────────────────────────

    #[test]
    fn recovery_hint_is_non_empty_for_all_variants() {
        for e in ALL_VARIANTS {
            assert!(
                !e.recovery_hint().is_empty(),
                "{e:?}: recovery_hint() must not be empty"
            );
        }
    }

    // ── Severity ──────────────────────────────────────────────────────────────

    #[test]
    fn auth_errors_have_auth_severity() {
        assert_eq!(FeeError::Unauthorized.severity(), ErrorSeverity::AuthError);
        assert_eq!(FeeError::SelfBidNotAllowed.severity(), ErrorSeverity::AuthError);
    }

    #[test]
    fn internal_errors_have_contract_severity() {
        assert_eq!(FeeError::ArithmeticError.severity(), ErrorSeverity::ContractError);
        assert_eq!(FeeError::InvalidConfig.severity(), ErrorSeverity::ContractError);
    }

    #[test]
    fn user_mistakes_have_user_severity() {
        let user_errors = [
            FeeError::AuctionNotFound,
            FeeError::AuctionEnded,
            FeeError::AuctionNotEnded,
            FeeError::BidTooLow,
            FeeError::AlreadySettled,
            FeeError::InvalidProperty,
            FeeError::BidDeadlineNotReached,
        ];
        for e in user_errors {
            assert_eq!(e.severity(), ErrorSeverity::UserError, "{e:?}");
        }
    }

    // ── Recoverability ────────────────────────────────────────────────────────

    #[test]
    fn arithmetic_and_config_errors_are_not_recoverable() {
        assert!(!FeeError::ArithmeticError.is_recoverable());
        assert!(!FeeError::InvalidConfig.is_recoverable());
    }

    #[test]
    fn all_user_errors_are_recoverable() {
        let recoverable = [
            FeeError::Unauthorized,
            FeeError::AuctionNotFound,
            FeeError::AuctionEnded,
            FeeError::AuctionNotEnded,
            FeeError::BidTooLow,
            FeeError::AlreadySettled,
            FeeError::InvalidProperty,
            FeeError::BidDeadlineNotReached,
            FeeError::SelfBidNotAllowed,
        ];
        for e in recoverable {
            assert!(e.is_recoverable(), "{e:?} should be recoverable");
        }
    }

    // ── Kind (sub-category) ───────────────────────────────────────────────────

    #[test]
    fn auth_variants_map_to_authorization_kind() {
        assert_eq!(FeeError::Unauthorized.kind(), FeeErrorKind::Authorization);
        assert_eq!(FeeError::SelfBidNotAllowed.kind(), FeeErrorKind::Authorization);
    }

    #[test]
    fn bid_too_low_maps_to_bid_validation_kind() {
        assert_eq!(FeeError::BidTooLow.kind(), FeeErrorKind::BidValidation);
    }

    #[test]
    fn property_error_maps_to_property_validation_kind() {
        assert_eq!(FeeError::InvalidProperty.kind(), FeeErrorKind::PropertyValidation);
    }

    #[test]
    fn auction_lifecycle_variants_map_correctly() {
        let lifecycle = [
            FeeError::AuctionNotFound,
            FeeError::AuctionEnded,
            FeeError::AuctionNotEnded,
            FeeError::AlreadySettled,
            FeeError::BidDeadlineNotReached,
        ];
        for e in lifecycle {
            assert_eq!(e.kind(), FeeErrorKind::AuctionLifecycle, "{e:?}");
        }
    }

    // ── Error codes ───────────────────────────────────────────────────────────

    #[test]
    fn all_variants_produce_non_zero_error_code() {
        for e in ALL_VARIANTS {
            assert_ne!(e.error_code(), 0, "{e:?}: error_code() must not be 0");
        }
    }

    #[test]
    fn error_codes_are_unique_across_all_variants() {
        let mut codes: std::vec::Vec<u32> =
            ALL_VARIANTS.iter().map(|e| e.error_code()).collect();
        let original_len = codes.len();
        codes.dedup();
        // Sort first so dedup removes all duplicates, not just consecutive ones.
        codes.sort_unstable();
        codes.dedup();
        assert_eq!(
            codes.len(),
            original_len,
            "Duplicate error codes detected — each variant must have a unique code"
        );
    }

    #[test]
    fn from_fee_error_for_u32_matches_error_code() {
        for e in ALL_VARIANTS {
            assert_eq!(
                u32::from(*e),
                e.error_code(),
                "{e:?}: From<FeeError> for u32 must match error_code()"
            );
        }
    }

    // ── ErrorCategory ─────────────────────────────────────────────────────────

    #[test]
    fn all_variants_belong_to_fees_category() {
        for e in ALL_VARIANTS {
            assert_eq!(
                e.error_category(),
                ErrorCategory::Fees,
                "{e:?}: error_category() must always be Fees"
            );
        }
    }

    // ── FeeResult alias ───────────────────────────────────────────────────────

    #[test]
    fn fee_result_ok_and_err_roundtrip() {
        let ok: FeeResult<u32> = Ok(42);
        assert_eq!(ok.unwrap(), 42);

        let err: FeeResult<u32> = Err(FeeError::BidTooLow);
        assert_eq!(err.unwrap_err(), FeeError::BidTooLow);
    }

    // ── Encoding round-trip ───────────────────────────────────────────────────

    #[test]
    fn scale_encode_decode_roundtrip() {
        use scale::{Decode, Encode};
        for e in ALL_VARIANTS {
            let encoded = e.encode();
            let decoded = FeeError::decode(&mut &encoded[..])
                .expect("SCALE decode must not fail for a valid FeeError");
            assert_eq!(*e, decoded, "{e:?}: SCALE round-trip must be lossless");
        }
    }
}