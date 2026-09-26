#![allow(clippy::clone_on_copy)] // fires inside ink! generated storage code
#![cfg_attr(not(feature = "std"), no_std, no_main)]

//! # PropChain Multicall Contract
//!
//! Dispatches multiple cross-contract calls in a single transaction.
//!
//! ## Usage
//!
//! Build a `Vec<CallRequest>` where each entry specifies:
//! - `callee`              – target contract `AccountId`
//! - `selector_and_input` – 4-byte selector + SCALE-encoded args
//! - `transferred_value`  – native tokens to forward (usually 0)
//! - `gas_limit`          – per-call gas cap (0 = forward remaining gas)
//! - `allow_revert`       – if `false`, one failure reverts the whole batch
//!
//! Call `aggregate` for strict (all-or-nothing) execution, or
//! `try_aggregate` to collect per-call results without reverting.

use ink::prelude::vec::Vec;
use propchain_traits::constants::MAX_BATCH_SIZE;
use propchain_traits::multicall::{
    CallRequest, CallResult, MulticallError, CALL_SELECTOR_LEN,
};

/// Hard cap on calls per multicall transaction.
const MAX_MULTICALL_SIZE: u32 = MAX_BATCH_SIZE;

#[ink::contract]
pub mod propchain_multicall {
    use super::*;

    // ── Events ────────────────────────────────────────────────────────────

    /// Emitted once per successful `aggregate` / `try_aggregate` invocation.
    #[ink(event)]
    pub struct MulticallExecuted {
        /// Caller that submitted the batch.
        #[ink(topic)]
        pub caller: AccountId,
        /// Total calls in the batch.
        pub total: u32,
        /// Number of calls that succeeded.
        pub succeeded: u32,
        /// Number of calls that failed (only non-zero for `try_aggregate`).
        pub failed: u32,
        pub timestamp: u64,
    }

    /// Emitted when the contract pause state changes.
    #[ink(event)]
    pub struct PauseToggled {
        #[ink(topic)]
        pub by: AccountId,
        pub paused: bool,
    }

    // ── Storage ───────────────────────────────────────────────────────────

    #[ink(storage)]
    pub struct MulticallContract {
        /// Contract admin – can pause/unpause.
        admin: AccountId,
        /// When `true` all dispatch calls are rejected.
        paused: bool,
    }

    // ── Implementation ────────────────────────────────────────────────────

    impl MulticallContract {
        /// Deploy the multicall contract.
        #[ink(constructor)]
        #[allow(clippy::new_without_default)]
        pub fn new() -> Self {
            Self {
                admin: Self::env().caller(),
                paused: false,
            }
        }

        // ── Public messages ───────────────────────────────────────────────

        /// Execute all calls atomically.
        ///
        /// Reverts the entire transaction if **any** call fails, regardless
        /// of the individual `allow_revert` flags.
        ///
        /// Attaching native tokens to this call is not supported. Use
        /// `CallRequest.transferred_value` to forward value per-call.
        #[ink(message, payable)]
        pub fn aggregate(
            &mut self,
            calls: Vec<CallRequest>,
        ) -> Result<Vec<CallResult>, MulticallError> {
            if self.env().transferred_value() > 0 {
                return Err(MulticallError::UnexpectedValue);
            }
            self.ensure_not_paused()?;
            self.validate_calls(&calls)?;

            let mut results = Vec::with_capacity(calls.len());

            for (i, call) in calls.iter().enumerate() {
                let result = self.dispatch(i as u32, call);
                if !result.success {
                    // Strict mode: any failure reverts everything.
                    return Err(MulticallError::CallReverted(i as u32));
                }
                results.push(result);
            }

            self.emit_executed(results.len() as u32, 0);
            Ok(results)
        }

        /// Execute all calls, collecting results without reverting on failure.
        ///
        /// Individual calls that have `allow_revert = false` still cause a
        /// full revert; calls with `allow_revert = true` record the failure
        /// and continue.
        ///
        /// Attaching native tokens to this call is not supported. Use
        /// `CallRequest.transferred_value` to forward value per-call.
        #[ink(message, payable)]
        pub fn try_aggregate_calls(
            &mut self,
            calls: Vec<CallRequest>,
        ) -> Result<Vec<CallResult>, MulticallError> {
            if self.env().transferred_value() > 0 {
                return Err(MulticallError::UnexpectedValue);
            }
            self.ensure_not_paused()?;
            self.validate_calls(&calls)?;

            let mut results = Vec::with_capacity(calls.len());
            let mut failed: u32 = 0;

            for (i, call) in calls.iter().enumerate() {
                let result = self.dispatch(i as u32, call);

                if !result.success && !call.allow_revert {
                    // Caller marked this call as must-succeed.
                    return Err(MulticallError::CallReverted(i as u32));
                }

                if !result.success {
                    failed += 1;
                }

                results.push(result);
            }

            let succeeded = results.len() as u32 - failed;
            self.emit_executed(succeeded, failed);
            Ok(results)
        }

        /// Pause the contract (admin only).
        #[ink(message)]
        pub fn pause(&mut self) -> Result<(), MulticallError> {
            self.ensure_admin()?;
            self.paused = true;
            let caller = self.env().caller();
            self.env().emit_event(PauseToggled {
                by: caller,
                paused: true,
            });
            Ok(())
        }

        /// Unpause the contract (admin only).
        #[ink(message)]
        pub fn unpause(&mut self) -> Result<(), MulticallError> {
            self.ensure_admin()?;
            self.paused = false;
            let caller = self.env().caller();
            self.env().emit_event(PauseToggled {
                by: caller,
                paused: false,
            });
            Ok(())
        }

        /// Transfer admin role to a new account.
        #[ink(message)]
        pub fn transfer_admin(&mut self, new_admin: AccountId) -> Result<(), MulticallError> {
            self.ensure_admin()?;
            self.admin = new_admin;
            Ok(())
        }

        // ── Queries ───────────────────────────────────────────────────────

        #[ink(message)]
        pub fn admin(&self) -> AccountId {
            self.admin
        }

        #[ink(message)]
        pub fn is_paused(&self) -> bool {
            self.paused
        }

        #[ink(message)]
        pub fn max_calls(&self) -> u32 {
            MAX_MULTICALL_SIZE
        }

        // ── Internal helpers ──────────────────────────────────────────────

        /// Dispatch a single `CallRequest` and return its `CallResult`.
        fn dispatch(&self, index: u32, req: &CallRequest) -> CallResult {
            // `validate_calls` rejects undersized selectors for the whole batch
            // before any call is dispatched, so reaching here with fewer than
            // `CALL_SELECTOR_LEN` bytes means the invariant was bypassed. It is
            // still checked rather than assumed: the previous `[..4]` slice
            // panicked on a short buffer, and the `try_into().unwrap_or([0;4])`
            // fallback it was wrapped in would instead have invoked selector
            // `0x00000000` against the callee. Neither outcome is acceptable, so
            // the call is reported as a failure and no invocation is attempted.
            if req.selector_and_input.len() < CALL_SELECTOR_LEN {
                return CallResult {
                    index,
                    success: false,
                    return_data: scale::Encode::encode(&MulticallError::SelectorTooShort {
                        index,
                        len: req.selector_and_input.len() as u32,
                    }),
                };
            }

            let gas_limit = if req.gas_limit == 0 {
                self.env().gas_left()
            } else {
                req.gas_limit
            };

            // selector_and_input layout: [..4] = 4-byte selector, [4..] = encoded args.
            // `copy_from_slice` cannot fail here: the length was checked above,
            // and both sides are exactly `CALL_SELECTOR_LEN` bytes.
            let mut selector = [0u8; CALL_SELECTOR_LEN];
            selector.copy_from_slice(&req.selector_and_input[..CALL_SELECTOR_LEN]);

            let outcome = ink::env::call::build_call::<ink::env::DefaultEnvironment>()
                .call_v1(req.callee)
                .gas_limit(gas_limit)
                .transferred_value(req.transferred_value)
                .call_flags(ink::env::CallFlags::empty())
                .exec_input(
                    ink::env::call::ExecutionInput::new(ink::env::call::Selector::new(selector))
                        .push_arg(&req.selector_and_input[CALL_SELECTOR_LEN..]),
                )
                .returns::<Vec<u8>>()
                .try_invoke();

            match outcome {
                Ok(Ok(data)) => CallResult {
                    index,
                    success: true,
                    return_data: data,
                },
                Ok(Err(lang_err)) => CallResult {
                    index,
                    success: false,
                    return_data: scale::Encode::encode(&lang_err),
                },
                Err(env_err) => CallResult {
                    index,
                    success: false,
                    return_data: ink::prelude::format!("{:?}", env_err).into_bytes(),
                },
            }
        }

        fn validate_calls(&self, calls: &[CallRequest]) -> Result<(), MulticallError> {
            if calls.is_empty() {
                return Err(MulticallError::EmptyCalls);
            }
            if calls.len() > MAX_MULTICALL_SIZE as usize {
                return Err(MulticallError::TooManyCalls);
            }
            // Every entry must carry a complete 4-byte selector. Checked here,
            // over the whole batch, so a malformed entry is reported by index
            // before any gas is spent on the calls ahead of it (#1164).
            for (index, call) in calls.iter().enumerate() {
                if call.selector_and_input.len() < CALL_SELECTOR_LEN {
                    return Err(MulticallError::SelectorTooShort {
                        index: index as u32,
                        len: call.selector_and_input.len() as u32,
                    });
                }
            }
            Ok(())
        }

        fn ensure_not_paused(&self) -> Result<(), MulticallError> {
            if self.paused {
                return Err(MulticallError::Paused);
            }
            Ok(())
        }

        fn ensure_admin(&self) -> Result<(), MulticallError> {
            if self.env().caller() != self.admin {
                return Err(MulticallError::Unauthorized);
            }
            Ok(())
        }

        fn emit_executed(&self, succeeded: u32, failed: u32) {
            self.env().emit_event(MulticallExecuted {
                caller: self.env().caller(),
                total: succeeded + failed,
                succeeded,
                failed,
                timestamp: self.env().block_timestamp(),
            });
        }
    }

    // ── Tests ─────────────────────────────────────────────────────────────

    #[cfg(test)]
    mod tests {
        use ink::env::{test, DefaultEnvironment};
        use scale::Decode;

        use super::*;

        fn setup() -> MulticallContract {
            let accounts = test::default_accounts::<DefaultEnvironment>();
            test::set_caller::<DefaultEnvironment>(accounts.alice);
            MulticallContract::new()
        }

        /// A well-formed request with the given selector bytes; `gas_limit` and
        /// `transferred_value` are left at zero so nothing is forwarded.
        fn req(callee: AccountId, selector_and_input: Vec<u8>) -> CallRequest {
            CallRequest {
                callee,
                selector_and_input,
                transferred_value: 0,
                gas_limit: 0,
                allow_revert: true,
            }
        }

        fn alice_bob() -> (AccountId, AccountId) {
            let accounts = test::default_accounts::<DefaultEnvironment>();
            (accounts.alice, accounts.bob)
        }

        // ---- #1164: undersized selectors ----

        #[ink::test]
        fn validate_calls_rejects_every_length_below_the_selector_width() {
            let contract = setup();
            let (_, bob) = alice_bob();

            // 0, 1, 2 and 3 bytes are all shorter than CALL_SELECTOR_LEN.
            for len in 0..CALL_SELECTOR_LEN {
                let calls = vec![req(bob, vec![0xAAu8; len])];
                assert_eq!(
                    contract.validate_calls(&calls),
                    Err(MulticallError::SelectorTooShort {
                        index: 0,
                        len: len as u32
                    }),
                    "a {len}-byte selector_and_input must be rejected"
                );
            }
        }

        #[ink::test]
        fn validate_calls_accepts_exactly_four_bytes() {
            let contract = setup();
            let (_, bob) = alice_bob();

            // A selector with no arguments is the shortest legal request.
            let calls = vec![req(bob, vec![0xAAu8; CALL_SELECTOR_LEN])];
            assert!(contract.validate_calls(&calls).is_ok());
        }

        #[ink::test]
        fn validate_calls_accepts_a_selector_with_arguments() {
            let contract = setup();
            let (_, bob) = alice_bob();

            let mut input = vec![0xAAu8; CALL_SELECTOR_LEN];
            input.extend_from_slice(&[1, 2, 3, 4, 5, 6, 7, 8]);
            assert!(contract.validate_calls(&[req(bob, input)]).is_ok());
        }

        #[ink::test]
        fn validate_calls_reports_the_index_of_the_malformed_call() {
            let contract = setup();
            let (_, bob) = alice_bob();

            // Three well-formed calls, then a truncated one at index 3.
            let calls = vec![
                req(bob, vec![0xAAu8; CALL_SELECTOR_LEN]),
                req(bob, vec![0xBBu8; CALL_SELECTOR_LEN + 4]),
                req(bob, vec![0xCCu8; CALL_SELECTOR_LEN]),
                req(bob, vec![0xDDu8; 2]),
            ];
            assert_eq!(
                contract.validate_calls(&calls),
                Err(MulticallError::SelectorTooShort { index: 3, len: 2 })
            );
        }

        #[ink::test]
        fn validate_calls_reports_the_first_malformed_index() {
            let contract = setup();
            let (_, bob) = alice_bob();

            let calls = vec![
                req(bob, vec![0u8; 1]),
                req(bob, vec![0u8; 3]),
                req(bob, vec![0u8; 0]),
            ];
            assert_eq!(
                contract.validate_calls(&calls),
                Err(MulticallError::SelectorTooShort { index: 0, len: 1 }),
                "the lowest offending index must win so the caller is pointed at the first fix"
            );
        }

        #[ink::test]
        fn empty_and_oversized_batches_still_take_precedence() {
            let contract = setup();
            let (_, bob) = alice_bob();

            // The new selector check runs last, so the pre-existing errors keep
            // their original meaning and a malformed entry cannot mask them.
            assert_eq!(
                contract.validate_calls(&[]),
                Err(MulticallError::EmptyCalls)
            );

            let malformed: Vec<CallRequest> = (0..=MAX_MULTICALL_SIZE)
                .map(|_| req(bob, vec![0u8; 1]))
                .collect();
            assert_eq!(
                contract.validate_calls(&malformed),
                Err(MulticallError::TooManyCalls)
            );
        }

        // ---- #1164: dispatch no longer falls back to selector 0x00000000 ----

        #[ink::test]
        fn dispatch_never_invokes_selector_zero_for_an_undersized_request() {
            let contract = setup();
            let (_, bob) = alice_bob();

            // Reaching `dispatch` directly simulates the invariant being
            // bypassed. The request must come back as a failure that decodes to
            // `SelectorTooShort`, not as a successful call to selector
            // 0x00000000.
            let result = contract.dispatch(7, &req(bob, vec![0xAAu8; 3]));
            assert!(!result.success);
            assert_eq!(result.index, 7);

            let decoded = MulticallError::decode(&mut &result.return_data[..])
                .expect("return_data should be a SCALE-encoded MulticallError");
            assert_eq!(decoded, MulticallError::SelectorTooShort { index: 7, len: 3 });
        }

        #[ink::test]
        fn dispatch_reports_an_empty_selector_rather_than_truncating() {
            let contract = setup();
            let (_, bob) = alice_bob();

            let result = contract.dispatch(0, &req(bob, Vec::new()));
            assert!(!result.success);
            let decoded = MulticallError::decode(&mut &result.return_data[..])
                .expect("return_data should be a SCALE-encoded MulticallError");
            assert_eq!(decoded, MulticallError::SelectorTooShort { index: 0, len: 0 });
        }

        // ---- #1164: the public surface rejects the whole batch ----

        #[ink::test]
        fn aggregate_reverts_with_a_clear_error_on_an_undersized_selector() {
            let mut contract = setup();
            let (_, bob) = alice_bob();

            assert_eq!(
                contract.aggregate(vec![req(bob, vec![0xAAu8; 3])]),
                Err(MulticallError::SelectorTooShort { index: 0, len: 3 })
            );
        }

        #[ink::test]
        fn aggregate_rejects_an_empty_selector() {
            let mut contract = setup();
            let (_, bob) = alice_bob();

            assert_eq!(
                contract.aggregate(vec![req(bob, Vec::new())]),
                Err(MulticallError::SelectorTooShort { index: 0, len: 0 })
            );
        }

        #[ink::test]
        fn aggregate_names_the_offending_index_in_a_larger_batch() {
            let mut contract = setup();
            let (_, bob) = alice_bob();

            let calls = vec![
                req(bob, vec![0xAAu8; CALL_SELECTOR_LEN]),
                req(bob, vec![0xBBu8; CALL_SELECTOR_LEN]),
                req(bob, vec![0xCCu8; 1]),
            ];
            assert_eq!(
                contract.aggregate(calls),
                Err(MulticallError::SelectorTooShort { index: 2, len: 1 })
            );
        }

        #[ink::test]
        fn try_aggregate_rejects_a_malformed_batch_before_dispatching_anything() {
            let mut contract = setup();
            let (_, bob) = alice_bob();

            // `allow_revert = true` would normally let this call be recorded and
            // skipped. It does not help here: the batch is rejected by
            // `validate_calls` before the dispatch loop starts, so a malformed
            // entry cannot be used to smuggle a bad call past the check.
            let mut malformed = req(bob, vec![0xAAu8; 1]);
            malformed.allow_revert = true;
            assert_eq!(
                contract.try_aggregate_calls(vec![malformed]),
                Err(MulticallError::SelectorTooShort { index: 0, len: 1 })
            );
        }

        #[ink::test]
        fn a_one_byte_truncation_of_a_valid_selector_is_caught() {
            let mut contract = setup();
            let (_, bob) = alice_bob();

            // The realistic form of this bug: a selector that was meant to be 4
            // bytes and lost one.
            let calls = vec![req(bob, vec![0x9C, 0xE4, 0x8A])];
            assert_eq!(
                contract.aggregate(calls),
                Err(MulticallError::SelectorTooShort { index: 0, len: 3 })
            );
        }

        // ---- pre-existing coverage ----

        #[ink::test]
        fn constructor_sets_admin() {
            let accounts = test::default_accounts::<DefaultEnvironment>();
            test::set_caller::<DefaultEnvironment>(accounts.alice);
            let contract = MulticallContract::new();
            assert_eq!(contract.admin(), accounts.alice);
            assert!(!contract.is_paused());
            assert_eq!(contract.max_calls(), MAX_MULTICALL_SIZE);
        }

        #[ink::test]
        fn aggregate_rejects_empty_calls() {
            let mut contract = setup();
            let result = contract.aggregate(Vec::new());
            assert_eq!(result, Err(MulticallError::EmptyCalls));
        }

        #[ink::test]
        fn aggregate_rejects_too_many_calls() {
            let mut contract = setup();
            let accounts = test::default_accounts::<DefaultEnvironment>();
            let calls: Vec<CallRequest> = (0..=MAX_MULTICALL_SIZE)
                .map(|_| CallRequest {
                    callee: accounts.bob,
                    selector_and_input: vec![0u8; 4],
                    transferred_value: 0,
                    gas_limit: 0,
                    allow_revert: true,
                })
                .collect();
            let result = contract.aggregate(calls);
            assert_eq!(result, Err(MulticallError::TooManyCalls));
        }

        #[ink::test]
        fn try_aggregate_rejects_empty_calls() {
            let mut contract = setup();
            let result = contract.try_aggregate_calls(Vec::new());
            assert_eq!(result, Err(MulticallError::EmptyCalls));
        }

        #[ink::test]
        fn pause_and_unpause_works() {
            let mut contract = setup();
            assert!(contract.pause().is_ok());
            assert!(contract.is_paused());
            assert!(contract.unpause().is_ok());
            assert!(!contract.is_paused());
        }

        #[ink::test]
        fn pause_rejects_non_admin() {
            let mut contract = setup();
            let accounts = test::default_accounts::<DefaultEnvironment>();
            test::set_caller::<DefaultEnvironment>(accounts.bob);
            assert_eq!(contract.pause(), Err(MulticallError::Unauthorized));
        }

        #[ink::test]
        fn aggregate_rejects_when_paused() {
            let mut contract = setup();
            contract.pause().unwrap();
            let accounts = test::default_accounts::<DefaultEnvironment>();
            let calls = vec![CallRequest {
                callee: accounts.bob,
                selector_and_input: vec![0u8; 4],
                transferred_value: 0,
                gas_limit: 0,
                allow_revert: false,
            }];
            assert_eq!(contract.aggregate(calls), Err(MulticallError::Paused));
        }

        #[ink::test]
        fn transfer_admin_works() {
            let mut contract = setup();
            let accounts = test::default_accounts::<DefaultEnvironment>();
            assert!(contract.transfer_admin(accounts.bob).is_ok());
            assert_eq!(contract.admin(), accounts.bob);
        }
    }
}
