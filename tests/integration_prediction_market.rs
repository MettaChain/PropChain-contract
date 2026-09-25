//! # Integration Tests: Prediction market lifecycle and settlement (Issue #927)
//!
//! Drives the full value flow of `propchain-prediction-market` at suite
//! level: market creation → staking (real balance movement via
//! `transfer_in`) → settlement against a price source → payout claims with
//! exact math assertions. Covers both the admin-resolved plain markets and
//! the oracle-settled markets introduced by issue #505.
//!
//! Payout formula under test:
//!   total_reward = stake + stake * losing_pool / winning_pool
//!   payout       = total_reward - total_reward * fee_bips / 10_000
//!
//! Since #1148 a manual resolution is a proposal: the market parks in
//! `PendingResolution` for a `DISPUTE_WINDOW`, claims are refused while it is
//! open, and `finalize_resolution` is what unlocks settlement.

#[cfg(test)]
mod integration_prediction_market {
    use ink::env::{test, DefaultEnvironment};
    use propchain_prediction_market::propchain_prediction_market::{
        Error, MarketStatus, PredictionDirection, PredictionMarket, DISPUTE_WINDOW,
    };

    fn setup() -> (
        PredictionMarket,
        ink::env::test::DefaultAccounts<DefaultEnvironment>,
    ) {
        let accounts = test::default_accounts::<DefaultEnvironment>();
        // The off-chain engine starts every account at zero balance; fund
        // the actors so payable stakes can actually move funds.
        for account in [accounts.alice, accounts.bob, accounts.charlie, accounts.eve] {
            test::set_account_balance::<DefaultEnvironment>(account, 1_000_000);
        }
        test::set_caller::<DefaultEnvironment>(accounts.alice);
        let contract = PredictionMarket::new(accounts.alice, 100); // 1% fee
        (contract, accounts)
    }

    #[ink::test]
    fn full_market_lifecycle_resolves_and_pays_winners() {
        let (mut contract, accounts) = setup();

        // Admin-only market creation.
        test::set_caller::<DefaultEnvironment>(accounts.bob);
        assert_eq!(
            contract.create_market(1, 500_000, 10_000),
            Err(Error::Unauthorized)
        );
        test::set_caller::<DefaultEnvironment>(accounts.alice);
        let market_id = contract.create_market(1, 500_000, 10_000).expect("create");

        // Bob goes Long with 1_000, Charlie hedges the other side with 3_000.
        let bob_balance_before =
            test::get_account_balance::<DefaultEnvironment>(accounts.bob).expect("bob has balance");
        test::set_caller::<DefaultEnvironment>(accounts.bob);
        test::transfer_in::<DefaultEnvironment>(1_000);
        contract
            .stake_prediction(market_id, PredictionDirection::Long)
            .expect("bob stakes long");
        assert_eq!(
            test::get_account_balance::<DefaultEnvironment>(accounts.bob).expect("bob has balance"),
            bob_balance_before - 1_000
        );

        test::set_caller::<DefaultEnvironment>(accounts.charlie);
        test::transfer_in::<DefaultEnvironment>(3_000);
        contract
            .stake_prediction(market_id, PredictionDirection::Short)
            .expect("charlie stakes short");

        // Zero-value and post-deadline stakes are rejected.
        test::set_value_transferred::<DefaultEnvironment>(0);
        assert_eq!(
            contract.stake_prediction(market_id, PredictionDirection::Short),
            Err(Error::InvalidAmount)
        );
        test::set_block_timestamp::<DefaultEnvironment>(10_001);
        test::set_value_transferred::<DefaultEnvironment>(1);
        assert_eq!(
            contract.stake_prediction(market_id, PredictionDirection::Long),
            Err(Error::MarketNotActive)
        );

        // Resolution before the deadline is rejected; after it, the price
        // source value 600_000 >= target 500_000 makes Long win. The outcome
        // is only proposed, so the market parks in PendingResolution until the
        // dispute window elapses (#1148).
        test::set_caller::<DefaultEnvironment>(accounts.alice);
        test::set_block_timestamp::<DefaultEnvironment>(5_000);
        assert_eq!(
            contract.resolve_market(market_id, 600_000),
            Err(Error::MarketNotReadyForResolution)
        );
        test::set_block_timestamp::<DefaultEnvironment>(10_001);
        test::set_caller::<DefaultEnvironment>(accounts.alice);
        contract
            .resolve_market(market_id, 600_000)
            .expect("resolve");
        assert_eq!(
            contract.resolve_market(market_id, 600_000),
            Err(Error::MarketAlreadyResolved)
        );

        let market = contract.get_market(market_id).expect("market exists");
        assert_eq!(market.status, MarketStatus::PendingResolution);
        assert_eq!(market.winning_direction, Some(PredictionDirection::Long));
        assert_eq!(market.resolved_value, Some(600_000));
        assert_eq!(
            contract.get_dispute_deadline(market_id),
            10_001 + DISPUTE_WINDOW
        );
        assert_eq!(
            contract.finalize_resolution(market_id),
            Err(Error::DisputeWindowStillOpen),
            "payouts stay locked until the window elapses"
        );
        assert_eq!(
            contract.claim_reward(market_id),
            Err(Error::MarketNotActive),
            "a contested resolution is not claimable"
        );

        // Window elapsed: anyone can finalize, and the payout path is unchanged.
        test::set_block_timestamp::<DefaultEnvironment>(10_001 + DISPUTE_WINDOW);
        test::set_caller::<DefaultEnvironment>(accounts.django);
        contract
            .finalize_resolution(market_id)
            .expect("finalize is permissionless");
        let market = contract.get_market(market_id).expect("market exists");
        assert_eq!(market.status, MarketStatus::Resolved);

        // Bob's winning payout: 1_000 + 1_000 * 3_000 / 1_000 = 4_000 gross;
        // fee 4_000 * 100 / 10_000 = 40 → exactly 3_960 net.
        let bob_before_claim =
            test::get_account_balance::<DefaultEnvironment>(accounts.bob).expect("bob has balance");
        test::set_caller::<DefaultEnvironment>(accounts.bob);
        contract.claim_reward(market_id).expect("bob claims");
        assert_eq!(
            test::get_account_balance::<DefaultEnvironment>(accounts.bob).expect("bob has balance"),
            bob_before_claim + 3_960
        );
        assert_eq!(
            contract.claim_reward(market_id),
            Err(Error::RewardAlreadyClaimed)
        );

        // Losers cannot claim and lose reputation.
        test::set_caller::<DefaultEnvironment>(accounts.charlie);
        assert_eq!(
            contract.claim_reward(market_id),
            Err(Error::LoserCannotClaim)
        );
        assert_eq!(
            contract
                .get_user_reputation(accounts.charlie)
                .successful_predictions,
            0
        );
        assert_eq!(
            contract
                .get_user_reputation(accounts.bob)
                .successful_predictions,
            1
        );
    }

    #[ink::test]
    fn oracle_settled_market_pays_out_against_price_source() {
        let (mut contract, accounts) = setup();
        contract.set_oracle(accounts.eve).expect("oracle set");

        // Only the oracle (or admin) may submit price data.
        let market_id = contract
            .create_oracle_market(7, String::from("property.valuation"), 500_000, 1_000)
            .expect("create oracle market");
        assert_eq!(
            contract
                .get_oracle_market(market_id)
                .expect("exists")
                .threshold,
            500_000
        );

        // Equal pools: winner doubles their stake minus fees.
        test::set_caller::<DefaultEnvironment>(accounts.bob);
        test::transfer_in::<DefaultEnvironment>(2_000);
        contract
            .stake_oracle_market(market_id, PredictionDirection::Long)
            .expect("bob long");
        test::set_caller::<DefaultEnvironment>(accounts.charlie);
        test::transfer_in::<DefaultEnvironment>(2_000);
        contract
            .stake_oracle_market(market_id, PredictionDirection::Short)
            .expect("charlie short");

        // Oracle submission before resolution_time is rejected.
        test::set_caller::<DefaultEnvironment>(accounts.eve);
        assert_eq!(
            contract.submit_oracle_data(market_id, 600_000),
            Err(Error::OracleMarketNotReady)
        );

        test::set_block_timestamp::<DefaultEnvironment>(1_001);
        contract
            .submit_oracle_data(market_id, 600_000)
            .expect("resolve");
        assert_eq!(
            contract.submit_oracle_data(market_id, 600_000),
            Err(Error::OracleMarketAlreadyResolved)
        );

        let market = contract.get_oracle_market(market_id).expect("exists");
        assert!(market.resolved);
        assert_eq!(market.winning_direction, Some(PredictionDirection::Long));
        assert_eq!(market.resolved_oracle_value, Some(600_000));

        // Bob: 2_000 + 2_000 * 2_000 / 2_000 = 4_000 gross; 40 fee → 3_960.
        let bob_before_claim =
            test::get_account_balance::<DefaultEnvironment>(accounts.bob).expect("bob has balance");
        test::set_caller::<DefaultEnvironment>(accounts.bob);
        contract
            .claim_winnings(market_id)
            .expect("bob claims winnings");
        assert_eq!(
            test::get_account_balance::<DefaultEnvironment>(accounts.bob).expect("bob has balance"),
            bob_before_claim + 3_960
        );

        // Unresolvable-before-resolution and loser paths stay pinned.
        test::set_caller::<DefaultEnvironment>(accounts.charlie);
        assert_eq!(
            contract.claim_winnings(market_id),
            Err(Error::LoserCannotClaim)
        );
        test::set_caller::<DefaultEnvironment>(accounts.eve);
        assert!(contract.get_oracle_market(999).is_none());
    }
}
