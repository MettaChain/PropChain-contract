//! Localization infrastructure for PropChain error messages.
//!
//! All lookups are static match expressions that allocate nothing, making this
//! module fully compatible with `no_std` / WASM contract environments.
//!
//! # Key format
//! Keys follow the pattern `"<category>.<variant>"` in snake_case, for example:
//! - `"common.unauthorized"`
//! - `"compliance.not_verified"`
//! - `"oracle.batch_size_exceeded"`
//!
//! # Adding a new locale
//! 1. Add a variant to [`SupportedLocale`].
//! 2. In the `lookup` function, add a `SupportedLocale::YourLocale => "..."` arm
//!    inside each key's match block, following the English arm as the template.

use scale::{Decode, Encode};

#[cfg(feature = "std")]
use scale_info::TypeInfo;

/// Supported display locales.
///
/// Only English is provided by default. The enum is designed for extension:
/// adding a new locale only requires a new variant and translation strings
/// inside [`lookup`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Encode, Decode)]
#[cfg_attr(feature = "std", derive(TypeInfo))]
pub enum SupportedLocale {
    /// English (default)
    En,
}

/// A resolved localized message with its original key, locale, and translated text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LocalizedMessage {
    /// The original i18n key that was looked up.
    pub key: &'static str,
    /// The locale used for the resolution.
    pub locale: SupportedLocale,
    /// The resolved, human-readable text in the requested locale.
    pub text: &'static str,
}

/// Look up the localized message for `key` in the given `locale`.
///
/// Returns a [`LocalizedMessage`] with static string fields. Falls back to
/// `"unknown.error"` if the key is not recognized.
pub fn lookup(key: &str, locale: SupportedLocale) -> LocalizedMessage {
    let (resolved_key, text): (&'static str, &'static str) = match key {
        // ---- common --------------------------------------------------------
        "common.unauthorized" => (
            "common.unauthorized",
            match locale {
                SupportedLocale::En => "Caller does not have permission to perform this operation",
            },
        ),
        "common.invalid_parameters" => (
            "common.invalid_parameters",
            match locale {
                SupportedLocale::En => "One or more function parameters are invalid",
            },
        ),
        "common.not_found" => (
            "common.not_found",
            match locale {
                SupportedLocale::En => "The requested resource does not exist",
            },
        ),
        "common.insufficient_funds" => (
            "common.insufficient_funds",
            match locale {
                SupportedLocale::En => "Account has insufficient balance for this operation",
            },
        ),
        "common.invalid_state" => (
            "common.invalid_state",
            match locale {
                SupportedLocale::En => "Cannot perform this operation in the current state",
            },
        ),
        "common.internal_error" => (
            "common.internal_error",
            match locale {
                SupportedLocale::En => "An internal error occurred in the contract",
            },
        ),
        "common.codec_error" => (
            "common.codec_error",
            match locale {
                SupportedLocale::En => "Failed to encode or decode data",
            },
        ),
        "common.not_implemented" => (
            "common.not_implemented",
            match locale {
                SupportedLocale::En => "This feature is not yet implemented",
            },
        ),
        "common.timeout" => (
            "common.timeout",
            match locale {
                SupportedLocale::En => "The operation exceeded its time limit",
            },
        ),
        "common.duplicate" => (
            "common.duplicate",
            match locale {
                SupportedLocale::En => "This operation or resource already exists",
            },
        ),
        // ---- oracle --------------------------------------------------------
        "oracle.property_not_found" => (
            "oracle.property_not_found",
            match locale {
                SupportedLocale::En => "The requested property does not exist in the oracle system",
            },
        ),
        "oracle.insufficient_sources" => (
            "oracle.insufficient_sources",
            match locale {
                SupportedLocale::En => {
                    "Not enough oracle sources are available to provide a reliable valuation"
                }
            },
        ),
        "oracle.invalid_valuation" => (
            "oracle.invalid_valuation",
            match locale {
                SupportedLocale::En => {
                    "The valuation data is invalid, zero, or out of acceptable range"
                }
            },
        ),
        "oracle.unauthorized" => (
            "oracle.unauthorized",
            match locale {
                SupportedLocale::En => {
                    "Caller does not have permission to perform this oracle operation"
                }
            },
        ),
        "oracle.source_not_found" => (
            "oracle.source_not_found",
            match locale {
                SupportedLocale::En => "The specified oracle source does not exist",
            },
        ),
        "oracle.invalid_parameters" => (
            "oracle.invalid_parameters",
            match locale {
                SupportedLocale::En => "One or more oracle function parameters are invalid",
            },
        ),
        "oracle.price_feed_error" => (
            "oracle.price_feed_error",
            match locale {
                SupportedLocale::En => "Failed to retrieve data from external price feed",
            },
        ),
        "oracle.alert_not_found" => (
            "oracle.alert_not_found",
            match locale {
                SupportedLocale::En => "The requested price alert does not exist",
            },
        ),
        "oracle.insufficient_reputation" => (
            "oracle.insufficient_reputation",
            match locale {
                SupportedLocale::En => "Oracle source reputation is below required threshold",
            },
        ),
        "oracle.source_already_exists" => (
            "oracle.source_already_exists",
            match locale {
                SupportedLocale::En => "An oracle source with this identifier already exists",
            },
        ),
        "oracle.request_pending" => (
            "oracle.request_pending",
            match locale {
                SupportedLocale::En => "A valuation request for this property is already pending",
            },
        ),
        "oracle.batch_size_exceeded" => (
            "oracle.batch_size_exceeded",
            match locale {
                SupportedLocale::En => {
                    "The number of items in the batch exceeds the configured maximum"
                }
            },
        ),
        // ---- compliance ----------------------------------------------------
        "compliance.unauthorized" => (
            "compliance.unauthorized",
            match locale {
                SupportedLocale::En => {
                    "Caller does not have permission to perform this compliance operation"
                }
            },
        ),
        "compliance.not_verified" => (
            "compliance.not_verified",
            match locale {
                SupportedLocale::En => "The user has not completed verification",
            },
        ),
        "compliance.verification_expired" => (
            "compliance.verification_expired",
            match locale {
                SupportedLocale::En => "The user's verification has expired and needs renewal",
            },
        ),
        "compliance.high_risk" => (
            "compliance.high_risk",
            match locale {
                SupportedLocale::En => {
                    "The user has been assessed as high risk and is not permitted"
                }
            },
        ),
        "compliance.prohibited_jurisdiction" => (
            "compliance.prohibited_jurisdiction",
            match locale {
                SupportedLocale::En => "The user's jurisdiction is prohibited from this operation",
            },
        ),
        "compliance.already_verified" => (
            "compliance.already_verified",
            match locale {
                SupportedLocale::En => "The user is already verified and cannot be re-verified",
            },
        ),
        "compliance.consent_not_given" => (
            "compliance.consent_not_given",
            match locale {
                SupportedLocale::En => "The user has not provided the required consent",
            },
        ),
        "compliance.data_retention_expired" => (
            "compliance.data_retention_expired",
            match locale {
                SupportedLocale::En => "The data retention period for this record has expired",
            },
        ),
        "compliance.invalid_risk_score" => (
            "compliance.invalid_risk_score",
            match locale {
                SupportedLocale::En => {
                    "The risk score provided is invalid or out of acceptable range"
                }
            },
        ),
        "compliance.invalid_document_type" => (
            "compliance.invalid_document_type",
            match locale {
                SupportedLocale::En => "The document type is invalid or not accepted",
            },
        ),
        "compliance.jurisdiction_not_supported" => (
            "compliance.jurisdiction_not_supported",
            match locale {
                SupportedLocale::En => "The specified jurisdiction is not currently supported",
            },
        ),
        // ---- monitoring ----------------------------------------------------
        "monitoring.unauthorized" => (
            "monitoring.unauthorized",
            match locale {
                SupportedLocale::En => "Caller does not have monitoring permissions",
            },
        ),
        "monitoring.contract_paused" => (
            "monitoring.contract_paused",
            match locale {
                SupportedLocale::En => "Monitoring contract is currently paused",
            },
        ),
        "monitoring.invalid_threshold" => (
            "monitoring.invalid_threshold",
            match locale {
                SupportedLocale::En => "Threshold value must be between 0 and 10 000 basis points",
            },
        ),
        "monitoring.subscriber_limit_reached" => (
            "monitoring.subscriber_limit_reached",
            match locale {
                SupportedLocale::En => "Cannot add more subscribers, maximum limit reached",
            },
        ),
        "monitoring.subscriber_not_found" => (
            "monitoring.subscriber_not_found",
            match locale {
                SupportedLocale::En => "The subscriber account is not registered",
            },
        ),
        // ---- fallback ------------------------------------------------------
        _ => (
            "unknown.error",
            match locale {
                SupportedLocale::En => "An unknown error occurred",
            },
        ),
    };

    LocalizedMessage {
        key: resolved_key,
        locale,
        text,
    }
}

/// Look up using the default locale (English).
pub fn lookup_default(key: &str) -> LocalizedMessage {
    lookup(key, SupportedLocale::En)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_key_returns_non_empty_text() {
        let msg = lookup("compliance.not_verified", SupportedLocale::En);
        assert!(!msg.text.is_empty());
        assert_eq!(msg.key, "compliance.not_verified");
    }

    #[test]
    fn unknown_key_falls_back_to_unknown_error() {
        let msg = lookup("this.key.does.not.exist", SupportedLocale::En);
        assert_eq!(msg.key, "unknown.error");
        assert_eq!(msg.text, "An unknown error occurred");
    }

    #[test]
    fn lookup_default_matches_english_lookup() {
        let a = lookup_default("oracle.batch_size_exceeded");
        let b = lookup("oracle.batch_size_exceeded", SupportedLocale::En);
        assert_eq!(a.text, b.text);
    }

    #[test]
    fn all_oracle_keys_resolve() {
        let keys = [
            "oracle.property_not_found",
            "oracle.insufficient_sources",
            "oracle.invalid_valuation",
            "oracle.unauthorized",
            "oracle.source_not_found",
            "oracle.invalid_parameters",
            "oracle.price_feed_error",
            "oracle.alert_not_found",
            "oracle.insufficient_reputation",
            "oracle.source_already_exists",
            "oracle.request_pending",
            "oracle.batch_size_exceeded",
        ];
        for key in keys {
            let msg = lookup_default(key);
            assert_ne!(msg.key, "unknown.error", "key '{key}' resolved to fallback");
        }
    }

    #[test]
    fn all_compliance_keys_resolve() {
        let keys = [
            "compliance.unauthorized",
            "compliance.not_verified",
            "compliance.verification_expired",
            "compliance.high_risk",
            "compliance.prohibited_jurisdiction",
            "compliance.already_verified",
            "compliance.consent_not_given",
            "compliance.data_retention_expired",
            "compliance.invalid_risk_score",
            "compliance.invalid_document_type",
            "compliance.jurisdiction_not_supported",
        ];
        for key in keys {
            let msg = lookup_default(key);
            assert_ne!(msg.key, "unknown.error", "key '{key}' resolved to fallback");
        }
    }
}
