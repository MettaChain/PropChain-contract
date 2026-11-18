//! Unit tests for prediction market resolution (#1457): proposal, dispute
//! window, finalization, challenges and payouts.
//!
//! Resolution is two-step: `resolve_market` *proposes* an outcome and parks the
//! market in `PendingResolution`; after `DISPUTE_WINDOW` anyone can
//! `finalize_resolution`, and winners then `claim_reward`.

#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(test)]
mod tests {
    use ink::env::{test, DefaultEnvironment};
    use propchain_prediction_market::propchain_prediction_market::{
        Error, MarketStatus, PredictionDirection, PredictionMarket, DISPUTE_WINDOW,
    };

    type Accounts = test::DefaultAccounts<DefaultEnvironment>;

    const TARGET: u128 = 500_000;
    const RESOLUTION_TIME: u64 = 10_000;

    /// 1% fee. Market 0 targets `TARGET` and resolves at `RESOLUTION_TIME`.
    fn setup() -> (PredictionMarket, Accounts, u64) {
        let accounts = test::default_accounts::<DefaultEnvironment>();
        for a in [accounts.alice, accounts.bob, accounts.charlie, accounts.eve] {
            test::set_account_balance::<DefaultEnvironment>(a, 1_000_000);
        }
        test::set_caller::<DefaultEnvironment>(accounts.alice);
        let mut m = PredictionMarket::new(accounts.alice, 100);
        let id = m.create_market(1, TARGET, RESOLUTION_TIME).unwrap();
        (m, accounts, id)
    }

    fn stake(m: &mut PredictionMarket, who: ink::primitives::AccountId, id: u64, dir: PredictionDirection, amount: u128) {
        test::set_caller::<DefaultEnvironment>(who);
        test::transfer_in::<DefaultEnvironment>(amount);
        m.stake_prediction(id, dir).expect("stake accepted");
    }

    /// Bob 1_000 long, Charlie 3_000 short.
    fn staked() -> (PredictionMarket, Accounts, u64) {
        let (mut m, a, id) = setup();
        stake(&mut m, a.bob, id, PredictionDirection::Long, 1_000);
        stake(&mut m, a.charlie, id, PredictionDirection::Short, 3_000);
        (m, a, id)
    }

    fn resolve_at(m: &mut PredictionMarket, a: &Accounts, id: u64, value: u128) {
        test::set_block_timestamp::<DefaultEnvironment>(RESOLUTION_TIME);
        test::set_caller::<DefaultEnvironment>(a.alice);
        m.resolve_market(id, value).unwrap();
    }

    fn finalize(m: &mut PredictionMarket, a: &Accounts, id: u64) {
        test::set_block_timestamp::<DefaultEnvironment>(RESOLUTION_TIME + DISPUTE_WINDOW);
        test::set_caller::<DefaultEnvironment>(a.django);
        m.finalize_resolution(id).unwrap();
    }

    // ── Proposing a resolution ──────────────────────────────────────────

    #[ink::test]
    fn the_outcome_follows_the_resolved_value_at_the_target_boundary() {
        for (value, expected) in [
            (TARGET, PredictionDirection::Long),          // >= target is Long
            (TARGET - 1, PredictionDirection::Short),
            (TARGET + 1, PredictionDirection::Long),
            (0, PredictionDirection::Short),
        ] {
            let (mut m, a, id) = staked();
            resolve_at(&mut m, &a, id, value);
            let market = m.get_market(id).unwrap();
            assert_eq!(market.winning_direction, Some(expected), "value {value}");
            assert_eq!(market.resolved_value, Some(value));
            assert_eq!(market.status, MarketStatus::PendingResolution);
        }
    }

    #[ink::test]
    fn resolution_is_admin_only_and_not_before_the_resolution_time() {
        let (mut m, a, id) = staked();
        test::set_block_timestamp::<DefaultEnvironment>(RESOLUTION_TIME - 1);
        test::set_caller::<DefaultEnvironment>(a.alice);
        assert_eq!(m.resolve_market(id, TARGET), Err(Error::MarketNotReadyForResolution));

        test::set_block_timestamp::<DefaultEnvironment>(RESOLUTION_TIME);
        test::set_caller::<DefaultEnvironment>(a.bob);
        assert_eq!(m.resolve_market(id, TARGET), Err(Error::Unauthorized));

        test::set_caller::<DefaultEnvironment>(a.alice);
        assert!(m.resolve_market(id, TARGET).is_ok());
    }

    #[ink::test]
    fn a_market_cannot_be_resolved_twice_or_when_unknown() {
        let (mut m, a, id) = staked();
        resolve_at(&mut m, &a, id, TARGET);
        assert_eq!(m.resolve_market(id, TARGET), Err(Error::MarketAlreadyResolved));
        assert_eq!(m.resolve_market(99, TARGET), Err(Error::MarketNotFound));
    }

    #[ink::test]
    fn the_dispute_deadline_is_the_resolution_moment_plus_the_window() {
        let (mut m, a, id) = staked();
        assert_eq!(m.get_dispute_deadline(id), 0);
        resolve_at(&mut m, &a, id, TARGET);
        assert_eq!(m.get_dispute_deadline(id), RESOLUTION_TIME + DISPUTE_WINDOW);
    }

    // ── Finalization ────────────────────────────────────────────────────

    #[ink::test]
    fn finalization_waits_for_the_window_and_is_inclusive_at_the_deadline() {
        let (mut m, a, id) = staked();
        resolve_at(&mut m, &a, id, TARGET);

        test::set_block_timestamp::<DefaultEnvironment>(RESOLUTION_TIME + DISPUTE_WINDOW - 1);
        assert_eq!(m.finalize_resolution(id), Err(Error::DisputeWindowStillOpen));
        assert_eq!(m.get_market(id).unwrap().status, MarketStatus::PendingResolution);

        finalize(&mut m, &a, id);
        assert_eq!(m.get_market(id).unwrap().status, MarketStatus::Resolved);
        assert_eq!(m.get_dispute_deadline(id), 0);
    }

    #[ink::test]
    fn finalization_requires_a_pending_resolution() {
        let (mut m, a, id) = staked();
        // Still Active.
        assert_eq!(m.finalize_resolution(id), Err(Error::MarketNotPendingResolution));
        resolve_at(&mut m, &a, id, TARGET);
        finalize(&mut m, &a, id);
        // Already Resolved.
        assert_eq!(m.finalize_resolution(id), Err(Error::MarketNotPendingResolution));
        assert_eq!(m.finalize_resolution(99), Err(Error::MarketNotFound));
    }

    // ── Challenges ──────────────────────────────────────────────────────

    #[ink::test]
    fn a_challenge_inside_the_window_with_a_sufficient_bond_reopens_the_market() {
        let (mut m, a, id) = staked();
        resolve_at(&mut m, &a, id, TARGET);
        let bond = m.required_challenge_bond(id);
        assert!(bond > 0);

        test::set_caller::<DefaultEnvironment>(a.eve);
        test::transfer_in::<DefaultEnvironment>(bond);
        m.challenge_resolution(id).unwrap();

        let market = m.get_market(id).unwrap();
        assert_eq!(market.status, MarketStatus::Active);
        assert_eq!(market.resolved_value, None);
        assert_eq!(market.winning_direction, None);
        assert_eq!(m.get_dispute_deadline(id), 0);
    }

    #[ink::test]
    fn an_underfunded_challenge_is_rejected() {
        let (mut m, a, id) = staked();
        resolve_at(&mut m, &a, id, TARGET);
        let bond = m.required_challenge_bond(id);
        test::set_caller::<DefaultEnvironment>(a.eve);
        test::set_value_transferred::<DefaultEnvironment>(bond - 1);
        assert_eq!(m.challenge_resolution(id), Err(Error::InsufficientChallengeBond));
        assert_eq!(m.get_market(id).unwrap().status, MarketStatus::PendingResolution);
    }

    #[ink::test]
    fn a_challenge_after_the_window_or_on_an_unresolved_market_is_rejected() {
        let (mut m, a, id) = staked();
        test::set_caller::<DefaultEnvironment>(a.eve);
        test::set_value_transferred::<DefaultEnvironment>(1_000_000);
        assert_eq!(m.challenge_resolution(id), Err(Error::MarketNotPendingResolution));

        resolve_at(&mut m, &a, id, TARGET);
        test::set_block_timestamp::<DefaultEnvironment>(RESOLUTION_TIME + DISPUTE_WINDOW);
        test::set_caller::<DefaultEnvironment>(a.eve);
        test::set_value_transferred::<DefaultEnvironment>(1_000_000);
        assert_eq!(m.challenge_resolution(id), Err(Error::DisputeWindowClosed));
    }

    // ── Payouts ─────────────────────────────────────────────────────────

    #[ink::test]
    fn a_winner_is_paid_their_stake_plus_a_pro_rata_share_of_the_losing_pool_minus_the_fee() {
        let (mut m, a, id) = staked();
        resolve_at(&mut m, &a, id, TARGET); // Long wins
        finalize(&mut m, &a, id);

        let before = test::get_account_balance::<DefaultEnvironment>(a.bob).unwrap();
        test::set_caller::<DefaultEnvironment>(a.bob);
        m.claim_reward(id).unwrap();
        // 1_000 + 1_000 * 3_000 / 1_000 = 4_000; 1% fee = 40.
        let after = test::get_account_balance::<DefaultEnvironment>(a.bob).unwrap();
        assert_eq!(after - before, 3_960);
    }

    #[ink::test]
    fn losers_cannot_claim_and_winners_cannot_claim_twice() {
        let (mut m, a, id) = staked();
        resolve_at(&mut m, &a, id, TARGET);
        finalize(&mut m, &a, id);

        test::set_caller::<DefaultEnvironment>(a.charlie);
        assert_eq!(m.claim_reward(id), Err(Error::LoserCannotClaim));

        test::set_caller::<DefaultEnvironment>(a.bob);
        m.claim_reward(id).unwrap();
        assert_eq!(m.claim_reward(id), Err(Error::RewardAlreadyClaimed));

        test::set_caller::<DefaultEnvironment>(a.eve); // never staked
        assert_eq!(m.claim_reward(id), Err(Error::StakeNotFound));
    }

    #[ink::test]
    fn no_payout_is_possible_before_finalization() {
        let (mut m, a, id) = staked();
        test::set_caller::<DefaultEnvironment>(a.bob);
        assert_eq!(m.claim_reward(id), Err(Error::MarketNotActive)); // still Active
        resolve_at(&mut m, &a, id, TARGET);
        test::set_caller::<DefaultEnvironment>(a.bob);
        assert_eq!(m.claim_reward(id), Err(Error::MarketNotActive)); // PendingResolution
    }

    #[ink::test]
    fn a_challenged_market_can_be_resolved_again_with_a_different_outcome() {
        let (mut m, a, id) = staked();
        resolve_at(&mut m, &a, id, TARGET); // Long
        let bond = m.required_challenge_bond(id);
        test::set_caller::<DefaultEnvironment>(a.eve);
        test::transfer_in::<DefaultEnvironment>(bond);
        m.challenge_resolution(id).unwrap();

        test::set_caller::<DefaultEnvironment>(a.alice);
        m.resolve_market(id, TARGET - 1).unwrap(); // Short this time
        assert_eq!(m.get_market(id).unwrap().winning_direction, Some(PredictionDirection::Short));
    }

    // ── Staking rules relevant to settlement ────────────────────────────

    #[ink::test]
    fn stakes_are_rejected_at_and_after_the_resolution_time_and_cannot_hedge() {
        let (mut m, a, id) = setup();
        stake(&mut m, a.bob, id, PredictionDirection::Long, 100);
        // Hedging the other way is refused.
        test::set_caller::<DefaultEnvironment>(a.bob);
        test::transfer_in::<DefaultEnvironment>(100);
        assert_eq!(m.stake_prediction(id, PredictionDirection::Short), Err(Error::InvalidAmount));

        test::set_block_timestamp::<DefaultEnvironment>(RESOLUTION_TIME);
        test::set_caller::<DefaultEnvironment>(a.charlie);
        test::set_value_transferred::<DefaultEnvironment>(100);
        assert_eq!(m.stake_prediction(id, PredictionDirection::Long), Err(Error::MarketNotActive));
    }

    #[ink::test]
    fn the_required_bond_is_zero_for_unknown_markets() {
        let (m, _, _) = setup();
        assert_eq!(m.required_challenge_bond(99), 0);
    }
}
