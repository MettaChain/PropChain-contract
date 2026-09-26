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
    /// Native tokens were attached to the call but the multicall contract
    /// does not forward message-level value. Per-call value should be set
    /// via `CallRequest.transferred_value`.
    UnexpectedValue,
    /// A `CallRequest.selector_and_input` was shorter than
    /// `CALL_SELECTOR_LEN`, so it does not even contain a complete 4-byte
    /// selector (Issue #1164).
    ///
    /// Reported by `validate_calls` before any dispatch is attempted, and
    /// carries both the offending `index` and the `len` that was supplied so a
    /// caller can tell a truncated selector from an empty one. Previously the
    /// contract sliced `[..4]` on a possibly-shorter buffer, which panics, and
    /// whose `unwrap_or([0u8; 4])` fallback would have invoked selector
    /// `0x00000000` against the callee had it ever been reached.
    SelectorTooShort { index: u32, len: u32 },
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
///
/// Note that the four targets do **not** share a signature, so the
/// `input` accepted by `build_verification_call` is not uniformly
/// meaningful across kinds: `verify_identity` and `is_compliant` are
/// keyed by `AccountId`, `is_property_screened` by `u64` property id,
/// and `get_property_valuation` takes a property id and returns a
/// valuation struct rather than a boolean. Real selectors make the
/// dispatch land on the right message; they do not make the arguments
/// line up. See `verification_message` for what each kind calls.
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

/// The 4-byte selector of the live verifier message each `VerificationKind`
/// dispatches to.
///
/// These are derived with `ink::selector_bytes!` from the message names in
/// `contracts/identity`, `contracts/compliance_registry`,
/// `contracts/sanctions` and `contracts/oracle`, so they are the same four
/// bytes the target contract computes for that message rather than a local
/// invention. Delegating to the macro is deliberate: the alternative is
/// pasting in four byte literals, which would be correct only for as long as
/// nobody renamed a message on the other side, and a stale selector fails
/// silently by dispatching to the wrong message.
///
/// All four targets are top-level messages in their contract module (not
/// nested in a submodule), so the macro's un-namespaced form is the one that
/// matches.
pub fn verification_selector(kind: VerificationKind) -> [u8; CALL_SELECTOR_LEN] {
    match kind {
        // `identity::IdentityRegistry::verify_identity`
        VerificationKind::Identity => ink::selector_bytes!("verify_identity"),
        // `compliance_registry::ComplianceRegistry::is_compliant`
        VerificationKind::Compliance => ink::selector_bytes!("is_compliant"),
        // `sanctions::SanctionsScreening::is_property_screened`
        VerificationKind::Sanctions => ink::selector_bytes!("is_property_screened"),
        // `oracle::PropertyOracle::get_property_valuation`
        VerificationKind::Oracle => ink::selector_bytes!("get_property_valuation"),
    }
}

/// The verifier message each `VerificationKind` dispatches to.
///
/// Returned as a name rather than only as bytes so off-chain tooling and
/// error messages can name the message they are actually calling, instead of
/// presenting an opaque four-byte selector. The names here are the literals
/// `verification_selector` feeds to `ink::selector_bytes!`; the two are kept
/// in step by a test.
pub fn verification_message(kind: VerificationKind) -> &'static str {
    match kind {
        VerificationKind::Identity => "verify_identity",
        VerificationKind::Compliance => "is_compliant",
        VerificationKind::Sanctions => "is_property_screened",
        VerificationKind::Oracle => "get_property_valuation",
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
    use ink::primitives::AccountId;

    use super::*;

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
        // The real Identity selector bytes are first, not a placeholder.
        assert_eq!(
            &call.selector_and_input[..CALL_SELECTOR_LEN],
            &verification_selector(VerificationKind::Identity)[..]
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

    // ---- #1182: real verifier selectors, not placeholders ----

    const ALL_KINDS: [VerificationKind; 4] = [
        VerificationKind::Identity,
        VerificationKind::Compliance,
        VerificationKind::Sanctions,
        VerificationKind::Oracle,
    ];

    /// Binds each kind to the selector of the live verifier message it names.
    ///
    /// The expected values come from the same macro the implementation uses, so
    /// this cannot prove the *message* is the right one — it pins the literal,
    /// so that editing `verification_selector` without editing the documented
    /// name (or vice versa) is caught here instead of failing silently
    /// on-chain against a wrong selector.
    #[test]
    fn each_kind_binds_to_its_live_verifier_selector() {
        let expected: [([u8; 4], &str); 4] = [
            (
                ink::selector_bytes!("verify_identity"),
                verification_message(VerificationKind::Identity),
            ),
            (
                ink::selector_bytes!("is_compliant"),
                verification_message(VerificationKind::Compliance),
            ),
            (
                ink::selector_bytes!("is_property_screened"),
                verification_message(VerificationKind::Sanctions),
            ),
            (
                ink::selector_bytes!("get_property_valuation"),
                verification_message(VerificationKind::Oracle),
            ),
        ];

        for (kind, (selector, message)) in ALL_KINDS.iter().zip(expected.iter()) {
            assert_eq!(
                verification_selector(*kind),
                *selector,
                "{:?} must dispatch to `{message}`",
                kind
            );
        }
    }

    /// Each documented name must itself hash to the selector that kind uses,
    /// so the name and the bytes cannot drift apart.
    #[test]
    fn documented_message_names_hash_to_their_selectors() {
        assert_eq!(
            verification_selector(VerificationKind::Identity),
            ink::selector_bytes!("verify_identity")
        );
        assert_eq!(
            verification_selector(VerificationKind::Compliance),
            ink::selector_bytes!("is_compliant")
        );
        assert_eq!(
            verification_selector(VerificationKind::Sanctions),
            ink::selector_bytes!("is_property_screened")
        );
        assert_eq!(
            verification_selector(VerificationKind::Oracle),
            ink::selector_bytes!("get_property_valuation")
        );
    }

    /// The old placeholders were `[0x01..=0x04, 0, 0, 0]`. If any reappears,
    /// the stubs are back and every dispatch for that kind targets a
    /// non-existent message.
    #[test]
    fn no_selector_is_a_placeholder() {
        for (i, kind) in ALL_KINDS.iter().enumerate() {
            let placeholder = [(i as u8) + 1, 0x00, 0x00, 0x00];
            assert_ne!(
                verification_selector(*kind),
                placeholder,
                "{:?} still resolves to its placeholder bytes",
                kind
            );
        }
    }

    /// A real selector is a hash prefix, so it should not be a small integer
    /// padded with zeros. This catches a stub introduced with a different
    /// numbering than the original four.
    #[test]
    fn no_selector_is_zero_padded() {
        for kind in ALL_KINDS {
            let selector = verification_selector(kind);
            let is_small_counter = selector[1..] == [0x00, 0x00, 0x00];
            assert!(
                !is_small_counter,
                "{:?} looks like a counter padded to four bytes: {selector:?}",
                kind
            );
        }
    }

    #[test]
    fn real_selectors_are_still_pairwise_distinct() {
        // The stubs guaranteed this by hand-picking 0x01..=0x04; real
        // hash-derived selectors have to genuinely not collide, so re-assert
        // the property against real values.
        for i in 0..ALL_KINDS.len() {
            for j in (i + 1)..ALL_KINDS.len() {
                assert_ne!(
                    verification_selector(ALL_KINDS[i]),
                    verification_selector(ALL_KINDS[j]),
                    "{:?} and {:?} must not share a selector",
                    ALL_KINDS[i],
                    ALL_KINDS[j]
                );
            }
        }
    }

    #[test]
    fn every_kind_names_its_verifier_message() {
        for kind in ALL_KINDS {
            assert!(
                !verification_message(kind).is_empty(),
                "{:?} must name its verifier message",
                kind
            );
        }
    }
}
