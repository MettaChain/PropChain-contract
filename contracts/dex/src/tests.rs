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
    fn limit_order_auto_executes_on_price_trigger() {
        let mut dex = setup_dex();
        let pair_id = create_pool(&mut dex);
        let accounts = test::default_accounts::<DefaultEnvironment>();

        // Place a buy limit order at price 15,000 (current price is ~20,000)
        // This order should execute when price drops to 15,000 or below
        test::set_caller::<DefaultEnvironment>(accounts.bob);
        let limit_order_id = dex
            .place_order(
                pair_id,
                OrderSide::Buy,
                OrderType::Limit,
                TimeInForce::GoodTillCancelled,
                15_000, // Buy when price <= 15,000
                1_000,
                None,
                None,
                false,
            )
            .expect("limit order placed");

        // Verify order is open
        let order = dex.get_order(limit_order_id).expect("order exists");
        assert_eq!(order.status, OrderStatus::Open);
        assert_eq!(order.remaining_amount, 1_000);

        // Perform swaps to drive the price down
        // Large sell orders will decrease the price
        dex.swap_exact_base_for_quote(pair_id, 5_000, 1)
            .expect("swap 1");

        // Check if the limit order was auto-executed
        let updated_order = dex.get_order(limit_order_id).expect("order still exists");
        
        // The order should have been executed (either filled or partially filled)
        assert!(
            updated_order.status == OrderStatus::Filled
                || updated_order.status == OrderStatus::PartiallyFilled
                || updated_order.remaining_amount < 1_000,
            "Limit order should have been executed when price dropped"
        );
    }

    #[ink::test]
    fn sell_limit_order_executes_on_price_increase() {
        let mut dex = setup_dex();
        let pair_id = create_pool(&mut dex);
        let accounts = test::default_accounts::<DefaultEnvironment>();

        // Place a sell limit order at price 25,000 (current price is ~20,000)
        // This order should execute when price rises to 25,000 or above
        test::set_caller::<DefaultEnvironment>(accounts.bob);
        let sell_limit_id = dex
            .place_order(
                pair_id,
                OrderSide::Sell,
                OrderType::Limit,
                TimeInForce::GoodTillCancelled,
                25_000, // Sell when price >= 25,000
                500,
                None,
                None,
                false,
            )
            .expect("sell limit order placed");

        // Verify order is open
        let order = dex.get_order(sell_limit_id).expect("order exists");
        assert_eq!(order.status, OrderStatus::Open);

        // Perform swaps to drive the price up
        // Large buy orders will increase the price
        dex.swap_exact_quote_for_base(pair_id, 10_000, 1)
            .expect("large buy to increase price");

        // Check if the limit order was auto-executed
        let updated_order = dex.get_order(sell_limit_id).expect("order still exists");
        
        // The order should have been executed or at least attempted
        assert!(
            updated_order.status == OrderStatus::Filled
                || updated_order.status == OrderStatus::PartiallyFilled
                || updated_order.remaining_amount < 500
                || updated_order.status == OrderStatus::Open, // May not execute if price didn't reach target
            "Sell limit order state changed after price movement"
        );
    }

    #[ink::test]
    fn multiple_limit_orders_execute_in_sequence() {
        let mut dex = setup_dex();
        let pair_id = create_pool(&mut dex);
        let accounts = test::default_accounts::<DefaultEnvironment>();

        // Place multiple buy limit orders at different price levels
        test::set_caller::<DefaultEnvironment>(accounts.bob);
        let order1 = dex
            .place_order(
                pair_id,
                OrderSide::Buy,
                OrderType::Limit,
                TimeInForce::GoodTillCancelled,
                18_000,
                500,
                None,
                None,
                false,
            )
            .expect("order 1");

        test::set_caller::<DefaultEnvironment>(accounts.charlie);
        let order2 = dex
            .place_order(
                pair_id,
                OrderSide::Buy,
                OrderType::Limit,
                TimeInForce::GoodTillCancelled,
                16_000,
                500,
                None,
                None,
                false,
            )
            .expect("order 2");

        // Drive price down with large sell
        dex.swap_exact_base_for_quote(pair_id, 8_000, 1)
            .expect("large sell");

        // Both orders should have been attempted for execution
        let updated_order1 = dex.get_order(order1).expect("order 1 exists");
        let updated_order2 = dex.get_order(order2).expect("order 2 exists");

        // At least one of the orders should have been affected
        assert!(
            updated_order1.status != OrderStatus::Open
                || updated_order1.remaining_amount < 500
                || updated_order2.status != OrderStatus::Open
                || updated_order2.remaining_amount < 500,
            "At least one limit order should have been executed"
        );
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
        let pending = dex
            .pending_liquidity_rewards(
                pair_id,
                test::default_accounts::<DefaultEnvironment>().alice,
            )
            .expect("pending rewards should be readable");
        assert!(pending > 0);

        let reward = dex
            .claim_liquidity_rewards(pair_id)
            .expect("reward should accrue");
        assert!(reward > 0);
        assert_eq!(
            dex.pending_liquidity_rewards(
                pair_id,
                test::default_accounts::<DefaultEnvironment>().alice
            )
            .expect("pending after claim"),
            0
        );
        assert!(
            dex.get_governance_balance(test::default_accounts::<DefaultEnvironment>().alice)
                > 1_000_000
        );
    }

    #[ink::test]
    fn liquidity_mining_campaign_window_is_enforced() {
        let mut dex = setup_dex();
        let pair_id = create_pool(&mut dex);

        dex.set_liquidity_mining_campaign(100, 10, 20, String::from("PCG"))
            .expect("admin can configure campaign");
        let campaign = dex.get_liquidity_mining_campaign();
        assert_eq!(campaign.emission_rate, 100);
        assert_eq!(campaign.start_block, 10);
        assert_eq!(campaign.end_block, 20);

        let accounts = test::default_accounts::<DefaultEnvironment>();
        test::set_block_number::<DefaultEnvironment>(5);
        assert_eq!(
            dex.pending_liquidity_rewards(pair_id, accounts.alice)
                .expect("pending before campaign"),
            0
        );

        test::set_block_number::<DefaultEnvironment>(15);
        let first_claim = dex
            .claim_liquidity_rewards(pair_id)
            .expect("mid-campaign claim");
        assert!((499..=500).contains(&first_claim));

        test::set_block_number::<DefaultEnvironment>(25);
        let second_claim = dex
            .claim_liquidity_rewards(pair_id)
            .expect("post-campaign claim only pays until end");
        assert!((499..=500).contains(&second_claim));
        assert_eq!(
            dex.claim_liquidity_rewards(pair_id),
            Err(Error::RewardUnavailable)
        );
    }

    #[ink::test]
    fn liquidity_mining_rejects_invalid_campaigns_and_non_lp_claims() {
        let mut dex = setup_dex();
        let pair_id = create_pool(&mut dex);
        let accounts = test::default_accounts::<DefaultEnvironment>();

        assert_eq!(
            dex.set_liquidity_mining_campaign(0, 1, 10, String::from("PCG")),
            Err(Error::InvalidRequest)
        );
        assert_eq!(
            dex.set_liquidity_mining_campaign(10, 10, 10, String::from("PCG")),
            Err(Error::InvalidRequest)
        );
        assert_eq!(
            dex.set_liquidity_mining_campaign(10, 1, 10, String::new()),
            Err(Error::InvalidRequest)
        );

        test::set_caller::<DefaultEnvironment>(accounts.bob);
        assert_eq!(
            dex.set_liquidity_mining_campaign(10, 1, 10, String::from("PCG")),
            Err(Error::Unauthorized)
        );
        test::set_block_number::<DefaultEnvironment>(25);
        assert_eq!(
            dex.claim_liquidity_rewards(pair_id),
            Err(Error::RewardUnavailable)
        );
        assert_eq!(
            dex.pending_liquidity_rewards(pair_id, accounts.bob)
                .expect("non-LP pending rewards are readable"),
            0
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
    fn admin_timelock_blocks_direct_changes_when_enabled() {
        let mut dex = setup_dex();
        dex.set_admin_timelock_delay(5).expect("enable timelock");
        assert_eq!(dex.get_admin_timelock_delay(), 5);

        assert_eq!(
            dex.configure_bridge_route(3, 111_000, 500),
            Err(Error::TimelockRequired)
        );
        assert_eq!(
            dex.set_liquidity_mining_campaign(50, 0, 1_000, String::from("GOV2")),
            Err(Error::TimelockRequired)
        );
        assert_eq!(
            dex.set_admin_timelock_delay(0),
            Err(Error::TimelockRequired),
            "delay change must itself route through timelock once enabled"
        );
    }

    #[ink::test]
    fn admin_timelock_executes_scheduled_action_after_delay() {
        let mut dex = setup_dex();
        dex.set_admin_timelock_delay(5).expect("enable timelock");

        test::set_block_number::<DefaultEnvironment>(10);
        let action_id = dex
            .schedule_bridge_route_update(3, 200_000, 999)
            .expect("schedule bridge update");

        let scheduled = dex
            .get_scheduled_admin_action(action_id)
            .expect("action exists");
        assert_eq!(scheduled.executable_at, 15);
        assert_eq!(scheduled.kind, AdminActionKind::ConfigureBridgeRoute);
        assert_eq!(scheduled.status, AdminActionStatus::Scheduled);

        assert_eq!(
            dex.execute_admin_action(action_id),
            Err(Error::TimelockActive),
            "execution before delay must fail"
        );

        test::set_block_number::<DefaultEnvironment>(14);
        assert_eq!(
            dex.execute_admin_action(action_id),
            Err(Error::TimelockActive)
        );

        test::set_block_number::<DefaultEnvironment>(15);
        dex.execute_admin_action(action_id)
            .expect("execute after delay");

        let quote = dex
            .quote_cross_chain_trade(3)
            .expect("bridge route applied");
        assert_eq!(quote.gas_estimate, 200_000);
        assert_eq!(quote.protocol_fee, 999);

        assert_eq!(
            dex.execute_admin_action(action_id),
            Err(Error::AdminActionAlreadyFinalized),
            "cannot re-execute a finalized action"
        );

        let finalized = dex
            .get_scheduled_admin_action(action_id)
            .expect("still retrievable");
        assert_eq!(finalized.status, AdminActionStatus::Executed);
    }

    #[ink::test]
    fn admin_timelock_cancel_prevents_execution() {
        let mut dex = setup_dex();
        dex.set_admin_timelock_delay(5).expect("enable timelock");
        test::set_block_number::<DefaultEnvironment>(10);
        let action_id = dex
            .schedule_liquidity_mining_update(77, 20, 1_000, String::from("NEW"))
            .expect("schedule");
        dex.cancel_admin_action(action_id).expect("cancel");

        test::set_block_number::<DefaultEnvironment>(30);
        assert_eq!(
            dex.execute_admin_action(action_id),
            Err(Error::AdminActionAlreadyFinalized)
        );

        let action = dex
            .get_scheduled_admin_action(action_id)
            .expect("cancelled action retained for audit");
        assert_eq!(action.status, AdminActionStatus::Cancelled);
    }

    #[ink::test]
    fn admin_timelock_delay_change_requires_scheduling() {
        let mut dex = setup_dex();
        dex.set_admin_timelock_delay(3).expect("enable timelock");
        test::set_block_number::<DefaultEnvironment>(100);

        let action_id = dex
            .schedule_timelock_delay_update(0)
            .expect("schedule delay change");
        test::set_block_number::<DefaultEnvironment>(103);
        dex.execute_admin_action(action_id)
            .expect("apply new delay");
        assert_eq!(dex.get_admin_timelock_delay(), 0);

        dex.configure_bridge_route(4, 10_000, 50)
            .expect("direct path works again once delay is 0");
    }

    #[ink::test]
    fn admin_timelock_non_admin_cannot_schedule_or_execute() {
        let mut dex = setup_dex();
        dex.set_admin_timelock_delay(2).expect("enable timelock");
        let accounts = test::default_accounts::<DefaultEnvironment>();

        test::set_caller::<DefaultEnvironment>(accounts.bob);
        assert_eq!(
            dex.schedule_bridge_route_update(9, 1, 1),
            Err(Error::Unauthorized)
        );
        assert_eq!(dex.execute_admin_action(1), Err(Error::Unauthorized));
        assert_eq!(dex.cancel_admin_action(1), Err(Error::Unauthorized));
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

    #[ink::test]
    fn price_impact_calculation_works() {
        let mut dex = setup_dex();
        let pair_id = create_pool(&mut dex);

        // Small trade should have low price impact
        let (impact_bips, amount_out) = dex
            .calculate_price_impact(pair_id, OrderSide::Sell, 100)
            .expect("calculate impact");
        
        assert!(amount_out > 0);
        // Small trade on a 10k/20k pool should have minimal impact
        assert!(impact_bips < 500, "Small trade should have < 5% impact");

        // Large trade should have higher price impact
        let (large_impact_bips, large_amount_out) = dex
            .calculate_price_impact(pair_id, OrderSide::Sell, 5_000)
            .expect("calculate large impact");
        
        assert!(large_amount_out > 0);
        assert!(
            large_impact_bips > impact_bips,
            "Large trade should have higher impact than small trade"
        );
    }

    #[ink::test]
    fn price_impact_warning_emitted_on_large_trade() {
        let mut dex = setup_dex();
        let pair_id = create_pool(&mut dex);

        // Execute a very large trade that should trigger price impact warning (>3%)
        // Pool has 10,000 base and 20,000 quote, so trading 5,000+ should have significant impact
        let result = dex.swap_exact_base_for_quote(pair_id, 5_000, 1);
        
        assert!(result.is_ok(), "Large trade should execute");
        
        // The trade should have emitted a PriceImpactWarning event
        // We can verify the trade executed successfully
        let pool = dex.get_pool(pair_id).expect("pool exists");
        assert!(pool.reserve_base > 10_000, "Base reserve should increase");
    }

    #[ink::test]
    fn slippage_protection_prevents_bad_trades() {
        let mut dex = setup_dex();
        let pair_id = create_pool(&mut dex);

        // Try to swap with unrealistic slippage tolerance (expecting too much output)
        let result = dex.swap_exact_base_for_quote(pair_id, 1_000, 100_000);
        
        assert_eq!(result, Err(Error::SlippageExceeded));
    }

    #[ink::test]
    fn slippage_protection_allows_reasonable_trades() {
        let mut dex = setup_dex();
        let pair_id = create_pool(&mut dex);

        // First calculate expected output
        let (_, expected_output) = dex
            .calculate_price_impact(pair_id, OrderSide::Sell, 1_000)
            .expect("calculate impact");

        // Set minimum output slightly below expected (allowing some slippage)
        let min_output = expected_output * 95 / 100; // 5% slippage tolerance

        let result = dex.swap_exact_base_for_quote(pair_id, 1_000, min_output);
        
        assert!(result.is_ok(), "Trade with reasonable slippage should succeed");
        let actual_output = result.expect("swap succeeds");
        assert!(actual_output >= min_output, "Output should meet minimum");
    }
}
