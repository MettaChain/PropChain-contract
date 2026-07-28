/// Macro to implement `From<ReentrancyError>` for a contract's error type.
///
/// Provides a canonical, single-line mapping that replaces the repetitive
/// `impl From<ReentrancyError> for Error { ... }` blocks that were duplicated
/// across every contract crate.
///
/// # Usage
///
/// ```ignore
/// // Inside a contract module where `Error` has a `ReentrantCall` variant:
/// map_reentrancy!(Error => ReentrantCall);
///
/// // For custom-named error enums:
/// map_reentrancy!(LendingError => ReentrantCall);
/// ```
///
/// The macro expands to:
///
/// ```ignore
/// impl From<propchain_traits::ReentrancyError> for Error {
///     fn from(_: propchain_traits::ReentrancyError) -> Self {
///         Error::ReentrantCall
///     }
/// }
/// ```
#[macro_export]
macro_rules! map_reentrancy {
    ($err_ty:ty => $variant:ident) => {
        impl From<$crate::ReentrancyError> for $err_ty {
            fn from(_: $crate::ReentrancyError) -> Self {
                <$err_ty>::$variant
            }
        }
    };
}
