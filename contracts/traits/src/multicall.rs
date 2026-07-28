//! Multicall types shared across the workspace.
//!
//! A `CallRequest` describes a single cross-contract call to be dispatched
//! by the Multicall contract.  `CallResult` carries the outcome of each
//! individual call so callers can inspect partial failures.

use ink::prelude::vec::Vec;

/// A single call to be dispatched inside a multicall transaction.
#[derive(Debug, Clone, PartialEq, Eq, scale::Encode, scale::Decode)]
#[cfg_attr(
    feature = "std",
    derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
)]
pub struct CallRequest {
    /// Target contract address.
    pub callee: ink::primitives::AccountId,
    /// 4-byte selector followed by SCALE-encoded arguments.
    pub selector_and_input: Vec<u8>,
    /// Native token value to forward with the call (0 for most calls).
    pub transferred_value: u128,
    /// Gas limit for this individual call (0 = use remaining gas).
    pub gas_limit: u64,
    /// When `true` the entire multicall reverts if this call fails.
    /// When `false` the failure is recorded and execution continues.
    pub allow_revert: bool,
}

/// Outcome of a single dispatched call.
#[derive(Debug, Clone, PartialEq, Eq, scale::Encode, scale::Decode)]
#[cfg_attr(
    feature = "std",
    derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
)]
pub struct CallResult {
    /// Index of the originating `CallRequest` in the input slice.
    pub index: u32,
    /// Whether the call succeeded.
    pub success: bool,
    /// SCALE-encoded return data on success, or error bytes on failure.
    pub return_data: Vec<u8>,
}

/// Errors returned by the Multicall contract.
#[derive(Debug, Clone, PartialEq, Eq, scale::Encode, scale::Decode)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub enum MulticallError {
    /// The calls vector was empty.
    EmptyCalls,
    /// The number of calls exceeds `MAX_MULTICALL_SIZE`.
    TooManyCalls,
    /// A call with `allow_revert = false` failed; index of the failing call
    /// is embedded so the caller knows which one caused the revert.
    CallReverted(u32),
    /// The contract is paused.
    Paused,
    /// Caller is not the admin.
    Unauthorized,
}
