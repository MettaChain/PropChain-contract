// Unit tests for the DEX contract (Issue #101 - extracted from lib.rs)

#[cfg(test)]
mod tests {
    use super::*;
    use ink::env::{test, DefaultEnvironment};

    fn setup_dex() -> PropertyDex {
        let mut dex = PropertyDex::new(String::from("PCG"), 1_000_000, 25, 1_000);
        dex.configure_bridge_route(2, 120_000, 400)
            .expect("bridge route config should work");
        dex
    }

    fn create_pool(dex: &mut PropertyDex) -> u64 {
        dex.create_pool(1, 2, 30, 10_000, 20_000)
            .expect("pool creation should work")
    }

    #[ink::test]
    fn amm_swap_updates_pool_state() {
        let mut dex = setup_dex();
        let pair_id = create_pool(&mut dex);
        let quote_out = dex
            .swap_exact_base_for_quote(pair_id, 1_000, 1)
            .expect("swap should succeed");
        assert!(quote_out > 0);

        let pool = dex.get_pool(pair_id).expect("pool must exist");
        assert_eq!(pool.reserve_base, 11_000);
        assert!(pool.reserve_quote < 20_000);

        let analytics = dex
            .get_pair_analytics(pair_id)
            .expect("analytics must exist");
        assert_eq!(analytics.trade_count, 1);
        assert!(analytics.last_price > 0);
    }

    #[ink::test]
    fn limit_orders_can_be_matched() {
        let mut dex = setup_dex();
        let pair_id = create_pool(&mut dex);
        let accounts = test::default_accounts::<DefaultEnvironment>();

        test::set_caller::<DefaultEnvironment>(accounts.bob);
        let maker = dex
            .place_order(
                pair_id,
                OrderSide::Sell,
                OrderType::Limit,
                TimeInForce::GoodTillCancelled,
                2_000,
                500,
                None,
                None,
                false,
            )
            .expect("maker order");

        test::set_caller::<DefaultEnvironment>(accounts.charlie);
        let taker = dex
            .place_order(
                pair_id,
                OrderSide::Buy,
                OrderType::Limit,
                TimeInForce::GoodTillCancelled,
                2_000,
                500,
                None,
                None,
                false,
            )
            .expect("taker order");

        let notional = dex.match_orders(maker, taker, 300).expect("match");
        assert_eq!(notional, 60);

        let maker_order = dex.get_order(maker).expect("maker order exists");
        let taker_order = dex.get_order(taker).expect("taker order exists");
        assert_eq!(maker_order.remaining_amount, 200);
        assert_eq!(taker_order.remaining_amount, 200);
    }

    #[ink::test]
    fn stop_loss_orders_require_trigger() {
        let mut dex = setup_dex();
        let pair_id = create_pool(&mut dex);
        let order_id = dex
            .place_order(
                pair_id,
                OrderSide::Sell,
                OrderType::StopLoss,
                TimeInForce::GoodTillCancelled,
                15_000,
                400,
                Some(15_000),
                None,
                false,
            )
            .expect("order");
        let result = dex.execute_order(order_id, 100);
        assert_eq!(result, Err(Error::OrderNotExecutable));

        dex.swap_exact_base_for_quote(pair_id, 4_000, 1)
            .expect("large sell to move price");
        let output = dex
            .execute_order(order_id, 100)
            .expect("triggered order executes");
        assert!(output > 0);
    }

    #[ink::test]
    fn liquidity_rewards_and_governance_accrue() {
        let mut dex = setup_dex();
        let pair_id = create_pool(&mut dex);
        test::set_block_number::<DefaultEnvironment>(25);
        let reward = dex
            .claim_liquidity_rewards(pair_id)
            .expect("reward should accrue");
        assert!(reward > 0);
        assert!(
            dex.get_governance_balance(test::default_accounts::<DefaultEnvironment>().alice)
                > 1_000_000
        );
    }

    #[ink::test]
    fn governance_can_update_fees() {
        let mut dex = setup_dex();
        let pair_id = create_pool(&mut dex);
        let proposal_id = dex
            .create_governance_proposal(
                String::from("Lower fees"),
                [7u8; 32],
                Some(20),
                None,
                5,
            )
            .expect("proposal");
        dex.vote_on_proposal(proposal_id, true).expect("vote");
        test::set_block_number::<DefaultEnvironment>(10);
        let passed = dex
            .execute_governance_proposal(proposal_id)
            .expect("execute");
        assert!(passed);
        let pool = dex.get_pool(pair_id).expect("pool exists");
        assert_eq!(pool.fee_bips, 20);
    }

    #[ink::test]
    fn order_book_snapshot_aggregates_levels_for_visualization() {
        let mut dex = setup_dex();
        let pair_id = create_pool(&mut dex);
        let accounts = test::default_accounts::<DefaultEnvironment>();

        test::set_caller::<DefaultEnvironment>(accounts.bob);
        dex.place_order(
            pair_id,
            OrderSide::Sell,
            OrderType::Limit,
            TimeInForce::GoodTillCancelled,
            2_100,
            400,
            None,
            None,
            false,
        )
        .expect("ask 1");
        dex.place_order(
            pair_id,
            OrderSide::Sell,
            OrderType::Limit,
            TimeInForce::GoodTillCancelled,
            2_100,
            300,
            None,
            None,
            false,
        )
        .expect("ask 2 same price");
        dex.place_order(
            pair_id,
            OrderSide::Sell,
            OrderType::Limit,
            TimeInForce::GoodTillCancelled,
            2_200,
            100,
            None,
            None,
            false,
        )
        .expect("ask 3");

        test::set_caller::<DefaultEnvironment>(accounts.charlie);
        dex.place_order(
            pair_id,
            OrderSide::Buy,
            OrderType::Limit,
            TimeInForce::GoodTillCancelled,
            1_900,
            250,
            None,
            None,
            false,
        )
        .expect("bid 1");
        dex.place_order(
            pair_id,
            OrderSide::Buy,
            OrderType::Limit,
            TimeInForce::GoodTillCancelled,
            1_950,
            500,
            None,
            None,
            false,
        )
        .expect("bid 2");

        let snapshot = dex
            .get_order_book_snapshot(pair_id, 10)
            .expect("snapshot should load");
        assert_eq!(snapshot.pair_id, pair_id);
        assert_eq!(snapshot.bids.len(), 2);
        assert_eq!(snapshot.asks.len(), 2);

        assert_eq!(snapshot.bids[0].price, 1_950);
        assert_eq!(snapshot.bids[0].total_amount, 500);
        assert_eq!(snapshot.bids[0].order_count, 1);
        assert_eq!(snapshot.bids[0].cumulative_amount, 500);
        assert_eq!(snapshot.bids[1].price, 1_900);
        assert_eq!(snapshot.bids[1].cumulative_amount, 750);

        assert_eq!(snapshot.asks[0].price, 2_100);
        assert_eq!(snapshot.asks[0].total_amount, 700);
        assert_eq!(snapshot.asks[0].order_count, 2);
        assert_eq!(snapshot.asks[0].cumulative_amount, 700);
        assert_eq!(snapshot.asks[1].price, 2_200);
        assert_eq!(snapshot.asks[1].cumulative_amount, 800);

        assert_eq!(snapshot.best_bid, 1_950);
        assert_eq!(snapshot.best_ask, 2_100);
        assert_eq!(snapshot.spread, 150);
        assert_eq!(snapshot.mid_price, 2_025);
        assert_eq!(snapshot.total_bid_depth, 750);
        assert_eq!(snapshot.total_ask_depth, 800);

        let cancel_id = dex
            .place_order(
                pair_id,
                OrderSide::Buy,
                OrderType::Limit,
                TimeInForce::GoodTillCancelled,
                1_800,
                100,
                None,
                None,
                false,
            )
            .expect("bid to cancel");
        dex.cancel_order(cancel_id).expect("cancel should work");
        let after_cancel = dex
            .get_order_book_snapshot(pair_id, 10)
            .expect("post-cancel snapshot");
        assert_eq!(
            after_cancel.bids.len(),
            2,
            "cancelled orders must not appear in the visualization"
        );

        let top = dex
            .get_order_book_snapshot(pair_id, 1)
            .expect("top-of-book");
        assert_eq!(top.bids.len(), 1);
        assert_eq!(top.asks.len(), 1);
        assert_eq!(top.bids[0].price, 1_950);
        assert_eq!(top.asks[0].price, 2_100);

        let bids_only = dex
            .get_order_book_levels(pair_id, OrderSide::Buy, 10)
            .expect("bids only");
        assert_eq!(bids_only.len(), 2);
        assert_eq!(bids_only[0].price, 1_950);

        assert_eq!(
            dex.get_order_book_snapshot(999, 10),
            Err(Error::PoolNotFound)
        );
    }

    #[ink::test]
    fn cross_chain_trade_and_portfolio_tracking_work() {
        let mut dex = setup_dex();
        let pair_id = create_pool(&mut dex);
        let accounts = test::default_accounts::<DefaultEnvironment>();

        test::set_caller::<DefaultEnvironment>(accounts.bob);
        dex.add_liquidity(pair_id, 5_000, 10_000)
            .expect("add liquidity");
        let order_id = dex
            .place_order(
                pair_id,
                OrderSide::Buy,
                OrderType::Twap,
                TimeInForce::GoodTillCancelled,
                0,
                250,
                None,
                Some(60),
                false,
            )
            .expect("place twap");
        let trade_id = dex
            .create_cross_chain_trade(pair_id, Some(order_id), 2, accounts.charlie, 700, 500)
            .expect("cross-chain trade");
        dex.attach_bridge_request(trade_id, 77)
            .expect("attach bridge request");

        let snapshot = dex.get_portfolio_snapshot(accounts.bob);
        assert_eq!(snapshot.liquidity_positions, 1);
        assert_eq!(snapshot.open_orders, 1);
        assert_eq!(snapshot.cross_chain_positions, 1);

        test::set_caller::<DefaultEnvironment>(accounts.alice);
        dex.finalize_cross_chain_trade(trade_id)
            .expect("admin finalizes");

        let trade = dex.cross_chain_trade(trade_id).expect("trade exists");
        assert_eq!(trade.status, CrossChainTradeStatus::Settled);
    }
}
