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
use propchain_traits::multicall::{CallRequest, CallResult, MulticallError};
use propchain_traits::constants::MAX_BATCH_SIZE;

/// Hard cap on calls per multicall transaction.
const MAX_MULTICALL_SIZE: u32 = MAX_BATCH_SIZE;

#[ink::contract]
mod propchain_multicall {
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
        #[ink(message, payable)]
        pub fn aggregate(
            &mut self,
            calls: Vec<CallRequest>,
        ) -> Result<Vec<CallResult>, MulticallError> {
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
        #[ink(message, payable)]
        pub fn try_aggregate_calls(
            &mut self,
            calls: Vec<CallRequest>,
        ) -> Result<Vec<CallResult>, MulticallError> {
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
            self.env().emit_event(PauseToggled { by: caller, paused: true });
            Ok(())
        }

        /// Unpause the contract (admin only).
        #[ink(message)]
        pub fn unpause(&mut self) -> Result<(), MulticallError> {
            self.ensure_admin()?;
            self.paused = false;
            let caller = self.env().caller();
            self.env().emit_event(PauseToggled { by: caller, paused: false });
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
            let gas_limit = if req.gas_limit == 0 {
                self.env().gas_left()
            } else {
                req.gas_limit
            };

            // Build a raw cross-contract call using ink!'s CallV1 builder.
            // selector_and_input layout: [0..4] = 4-byte selector, [4..] = encoded args.
            let selector: [u8; 4] = req.selector_and_input[..4]
                .try_into()
                .unwrap_or([0u8; 4]);

            let outcome = ink::env::call::build_call::<ink::env::DefaultEnvironment>()
                .call_v1(req.callee)
                .gas_limit(gas_limit)
                .transferred_value(req.transferred_value)
                .call_flags(ink::env::CallFlags::empty())
                .exec_input(
                    ink::env::call::ExecutionInput::new(
                        ink::env::call::Selector::new(selector),
                    )
                    .push_arg(&req.selector_and_input[4..]),
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
        use super::*;
        use ink::env::test;
        use ink::env::DefaultEnvironment;

        fn setup() -> MulticallContract {
            let accounts = test::default_accounts::<DefaultEnvironment>();
            test::set_caller::<DefaultEnvironment>(accounts.alice);
            MulticallContract::new()
        }

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
