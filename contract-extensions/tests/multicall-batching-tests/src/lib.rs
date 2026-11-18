//! Unit tests for multicall batch validation and administration (#1461).
//!
//! The off-chain test engine cannot execute cross-contract calls, so these tests
//! cover everything that happens *before* a call would be dispatched: batch
//! shape, selector length, value handling, pause and admin control. Correct
//! forwarding of real calls (result ordering, partial failure with
//! `allow_revert`) needs an on-chain / e2e environment and is not covered.

#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(test)]
mod tests {
    use ink::env::{test, DefaultEnvironment};
    use ink::primitives::AccountId;
    use propchain_multicall::propchain_multicall::MulticallContract;
    use propchain_traits::constants::MAX_BATCH_SIZE;
    use propchain_traits::multicall::{CallRequest, MulticallError, CALL_SELECTOR_LEN};

    type Accounts = test::DefaultAccounts<DefaultEnvironment>;

    fn setup() -> (MulticallContract, Accounts) {
        let accounts = test::default_accounts::<DefaultEnvironment>();
        test::set_caller::<DefaultEnvironment>(accounts.alice);
        test::set_value_transferred::<DefaultEnvironment>(0);
        (MulticallContract::new(), accounts)
    }

    fn call(callee: AccountId, input: Vec<u8>) -> CallRequest {
        CallRequest {
            callee,
            selector_and_input: input,
            transferred_value: 0,
            gas_limit: 100_000,
            allow_revert: false,
        }
    }

    // ── Batch shape ─────────────────────────────────────────────────────

    #[ink::test]
    fn an_empty_batch_is_rejected_by_both_entry_points() {
        let (mut m, _) = setup();
        assert_eq!(m.aggregate(Vec::new()), Err(MulticallError::EmptyCalls));
        assert_eq!(m.try_aggregate_calls(Vec::new()), Err(MulticallError::EmptyCalls));
    }

    #[ink::test]
    fn a_batch_over_the_size_limit_is_rejected_before_dispatch() {
        let (mut m, accounts) = setup();
        let too_many: Vec<CallRequest> = (0..=MAX_BATCH_SIZE)
            .map(|_| call(accounts.bob, vec![0u8; CALL_SELECTOR_LEN]))
            .collect();
        assert_eq!(too_many.len() as u32, MAX_BATCH_SIZE + 1);
        assert_eq!(m.aggregate(too_many.clone()), Err(MulticallError::TooManyCalls));
        assert_eq!(m.try_aggregate_calls(too_many), Err(MulticallError::TooManyCalls));
    }

    #[ink::test]
    fn the_size_limit_check_comes_before_selector_validation() {
        let (mut m, accounts) = setup();
        let malformed_and_oversized: Vec<CallRequest> = (0..=MAX_BATCH_SIZE)
            .map(|_| call(accounts.bob, vec![0u8; 1]))
            .collect();
        assert_eq!(m.aggregate(malformed_and_oversized), Err(MulticallError::TooManyCalls));
    }

    #[ink::test]
    fn max_calls_reports_the_configured_limit() {
        let (m, _) = setup();
        assert_eq!(m.max_calls(), MAX_BATCH_SIZE);
    }

    // ── Selector validation ─────────────────────────────────────────────

    #[ink::test]
    fn every_selector_shorter_than_four_bytes_is_rejected() {
        let (mut m, accounts) = setup();
        for len in 0..CALL_SELECTOR_LEN {
            let calls = vec![call(accounts.bob, vec![0xAA; len])];
            assert_eq!(
                m.aggregate(calls.clone()),
                Err(MulticallError::SelectorTooShort { index: 0, len: len as u32 }),
                "len {len}"
            );
            assert_eq!(
                m.try_aggregate_calls(calls),
                Err(MulticallError::SelectorTooShort { index: 0, len: len as u32 }),
                "len {len}"
            );
        }
    }

    #[ink::test]
    fn the_first_malformed_call_in_a_batch_is_reported_by_index() {
        let (mut m, accounts) = setup();
        let calls = vec![
            call(accounts.bob, vec![0xAA; CALL_SELECTOR_LEN]),
            call(accounts.bob, vec![0xBB; 2]),
            call(accounts.bob, vec![0xCC; 1]),
        ];
        assert_eq!(
            m.aggregate(calls),
            Err(MulticallError::SelectorTooShort { index: 1, len: 2 })
        );
    }

    // ── Value handling ──────────────────────────────────────────────────

    #[ink::test]
    fn attached_native_value_is_rejected_before_anything_else() {
        let (mut m, accounts) = setup();
        test::set_value_transferred::<DefaultEnvironment>(1);
        let valid = vec![call(accounts.bob, vec![0; CALL_SELECTOR_LEN])];
        assert_eq!(m.aggregate(valid.clone()), Err(MulticallError::UnexpectedValue));
        assert_eq!(m.try_aggregate_calls(valid), Err(MulticallError::UnexpectedValue));
        // Value is checked before the empty-batch check too.
        assert_eq!(m.aggregate(Vec::new()), Err(MulticallError::UnexpectedValue));
    }

    // ── Pause ───────────────────────────────────────────────────────────

    #[ink::test]
    fn a_paused_contract_rejects_batches_until_unpaused() {
        let (mut m, accounts) = setup();
        m.pause().unwrap();
        assert!(m.is_paused());
        let valid = vec![call(accounts.bob, vec![0; CALL_SELECTOR_LEN])];
        assert_eq!(m.aggregate(valid.clone()), Err(MulticallError::Paused));
        assert_eq!(m.try_aggregate_calls(valid), Err(MulticallError::Paused));

        m.unpause().unwrap();
        assert!(!m.is_paused());
        // After unpausing, an empty batch is again reported as such (not as paused).
        assert_eq!(m.aggregate(Vec::new()), Err(MulticallError::EmptyCalls));
    }

    #[ink::test]
    fn pause_takes_precedence_over_batch_validation() {
        let (mut m, _) = setup();
        m.pause().unwrap();
        assert_eq!(m.aggregate(Vec::new()), Err(MulticallError::Paused));
    }

    // ── Administration ──────────────────────────────────────────────────

    #[ink::test]
    fn only_the_admin_can_pause_unpause_or_transfer_admin() {
        let (mut m, accounts) = setup();
        test::set_caller::<DefaultEnvironment>(accounts.bob);
        assert_eq!(m.pause(), Err(MulticallError::Unauthorized));
        assert_eq!(m.unpause(), Err(MulticallError::Unauthorized));
        assert_eq!(m.transfer_admin(accounts.bob), Err(MulticallError::Unauthorized));
        assert!(!m.is_paused());
        assert_eq!(m.admin(), accounts.alice);
    }

    #[ink::test]
    fn admin_rights_move_with_transfer_admin() {
        let (mut m, accounts) = setup();
        m.transfer_admin(accounts.bob).unwrap();
        assert_eq!(m.admin(), accounts.bob);
        assert_eq!(m.pause(), Err(MulticallError::Unauthorized)); // alice, no longer admin
        test::set_caller::<DefaultEnvironment>(accounts.bob);
        assert!(m.pause().is_ok());
    }

    #[ink::test]
    fn pausing_and_unpausing_emit_one_event_each() {
        let (mut m, _) = setup();
        m.pause().unwrap();
        assert_eq!(test::recorded_events().count(), 1);
        m.unpause().unwrap();
        assert_eq!(test::recorded_events().count(), 2);
    }
}
