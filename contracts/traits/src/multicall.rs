//! Multicall types shared across the workspace.
//!
//! A `CallRequest` describes a single cross-contract call to be dispatched
//! by the Multicall contract.  `CallResult` carries the outcome of each
//! individual call so callers can inspect partial failures.
//!
//! Issue #737 substrate lives below in addition to the existing multicall
//! types: `VerificationKind` and the pure-rust `aggregate_verifications`
//! helper exist to support batching Identity/Compliance/Sanctions/Oracle
//! checks through a single multicall. The actual on-chain dispatch
//! remains in `contracts/multicall/src/lib.rs`; this file owns the
//! types and the request-construction helper only.

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

// ---------------------------------------------------------------------------
// Issue #737 substrate: aggregation of onboarding verification checks.
// ---------------------------------------------------------------------------

/// Categories of verification checks that onboarding flows may want to
/// batch through `Multicall::aggregate`.
///
/// `Identity`   — verify the actor is a real, registered identity.
/// `Compliance` — verify the actor is compliant with jurisdictional rules.
/// `Sanctions`  — verify the actor is not on any sanctions list.
/// `Oracle`     — verify the property payload against an oracle feed.
///
/// These map 1:1 onto separate contracts in the workspace today. The
/// intent of `aggregate_verifications` is to give any onboarding flow
/// a single batched entry point so a caller composes one
/// `Vec<CallRequest>` and hands it to `Multicall::aggregate`, instead of
/// issuing N independent round-trip messages.
#[derive(Debug, Clone, Copy, PartialEq, Eq, scale::Encode, scale::Decode)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub enum VerificationKind {
    Identity,
    Compliance,
    Sanctions,
    Oracle,
}

/// Length of a SCALE function selector preamble.
pub const CALL_SELECTOR_LEN: usize = 4;

/// Stub 4-byte SCALE selectors per `VerificationKind`. Replace with
/// production selectors (e.g. `ink::selector_bytes!("verify")`) once
/// the underlying verification contract messages are finalised; the
/// stubs are deliberately distinct from each other so a future
/// subcontract can replace one selector without breaking the others.
pub fn verification_selector(kind: VerificationKind) -> [u8; CALL_SELECTOR_LEN] {
    match kind {
        VerificationKind::Identity => [0x01, 0x00, 0x00, 0x00],
        VerificationKind::Compliance => [0x02, 0x00, 0x00, 0x00],
        VerificationKind::Sanctions => [0x03, 0x00, 0x00, 0x00],
        VerificationKind::Oracle => [0x04, 0x00, 0x00, 0x00],
    }
}

/// Build one `CallRequest` for a verification kind targeting `callee`.
///
/// The 4-byte selector goes first; the caller's SCALE-encoded argument
/// bytes follow verbatim as `selector_and_input`. The returned
/// `CallRequest` is suitable for handing to `Multicall::aggregate`.
pub fn build_verification_call(
    callee: ink::primitives::AccountId,
    kind: VerificationKind,
    input: &[u8],
) -> CallRequest {
    let selector = verification_selector(kind);
    let mut selector_and_input = Vec::with_capacity(CALL_SELECTOR_LEN + input.len());
    selector_and_input.extend_from_slice(&selector);
    selector_and_input.extend_from_slice(input);
    CallRequest {
        callee,
        selector_and_input,
        transferred_value: 0,
        gas_limit: 0,
        // Verification checks during onboarding should never abort the
        // whole batch — the aggregator decides policy on partial failures.
        allow_revert: true,
    }
}

/// Build the slice of `CallRequest`s for an onboarding batch of
/// verification checks. The result is intended to be passed directly
/// into the `Multicall::aggregate` entry point (see
/// `contracts/multicall/src/lib.rs`).
///
/// This function is a pure-rust constructor: it does NOT perform
/// cross-contract calls itself. Caller dispatches `Vec<CallRequest>`
/// to `Multicall::aggregate` at transaction time. The win over the
/// previous four-message per-check pattern is that onboarding flows
/// only have to compose one batch in memory instead of issuing N
/// independent messages.
pub fn aggregate_verifications(
    callee: ink::primitives::AccountId,
    requests: &[VerificationKind],
    input: &[u8],
) -> Vec<CallRequest> {
    requests
        .iter()
        .map(|kind| build_verification_call(callee, *kind, input))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use ink::primitives::AccountId;

    #[test]
    fn selectors_are_distinct_across_kinds() {
        let kinds = [
            VerificationKind::Identity,
            VerificationKind::Compliance,
            VerificationKind::Sanctions,
            VerificationKind::Oracle,
        ];
        for pair in kinds.windows(2) {
            assert_ne!(
                verification_selector(pair[0]),
                verification_selector(pair[1]),
                "selectors must be distinct across kinds so a multicall can disambiguate them"
            );
        }
    }

    #[test]
    fn aggregate_verifications_yields_one_call_per_kind() {
        let callee = AccountId::from([0xab; 32]);
        let input = [0xa0u8, 0xa1, 0xa2];
        let kinds = [
            VerificationKind::Identity,
            VerificationKind::Compliance,
            VerificationKind::Sanctions,
            VerificationKind::Oracle,
        ];
        let calls = aggregate_verifications(callee, &kinds, &input);
        assert_eq!(calls.len(), kinds.len());
        for call in &calls {
            assert_eq!(call.callee, callee);
            assert_eq!(call.transferred_value, 0);
            assert_eq!(call.gas_limit, 0);
            assert!(call.allow_revert);
        }
    }

    #[test]
    fn aggregate_verifications_appends_input_after_selector_bytes() {
        let callee = AccountId::from([0xab; 32]);
        let input = [0xf0u8, 0xf1, 0xf2, 0xf3];
        let kinds = [VerificationKind::Identity];
        let calls = aggregate_verifications(callee, &kinds, &input);
        assert_eq!(calls.len(), 1);
        let call = &calls[0];
        assert_eq!(
            call.selector_and_input.len(),
            CALL_SELECTOR_LEN + input.len()
        );
        // Identity selector bytes are first.
        assert_eq!(
            &call.selector_and_input[..CALL_SELECTOR_LEN],
            &[0x01u8, 0x00, 0x00, 0x00][..]
        );
        // Caller's SCALE-encoded args follow verbatim.
        assert_eq!(&call.selector_and_input[CALL_SELECTOR_LEN..], &input[..]);
    }

    #[test]
    fn empty_kinds_slice_yields_no_calls() {
        let callee = AccountId::from([0xab; 32]);
        let calls = aggregate_verifications(callee, &[], &[1, 2, 3]);
        assert!(calls.is_empty());
    }
}
