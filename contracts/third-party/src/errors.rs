// Error types for the third-party contract (Issue #101 - extracted from lib.rs)

#[derive(Debug, PartialEq, Eq, scale::Encode, scale::Decode)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub enum Error {
    Unauthorized,
    ServiceNotFound,
    ServiceInactive,
    RequestNotFound,
    InvalidStatusTransition,
    InvalidFeePercentage,
    KycExpired,
    PaymentProcessingFailed,
}
