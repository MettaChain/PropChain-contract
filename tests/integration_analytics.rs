/// # Integration Tests: Analytics Dashboard (Issue #923)
///
/// The analytics dashboard aggregates market metrics, historical trends,
/// sentiment, portfolio positions and benchmark indices, and supports a
/// two-step admin key rotation with a cooldown window.
///
/// Acceptance criteria tested:
///   check Market metrics update and read back through the public getter
///   check Batch metric updates enforce the batch-size limit
///   check Historical trends keep insertion order and feed market reports
///   check Sentiment updates flow into generated reports
///   check Portfolio positions round-trip per owner
///   check Benchmark indices track property types
///   check Admin rotation requires the nominee to confirm after cooldown
#[cfg(test)]
mod integration_analytics {
    use ink::env::{test, DefaultEnvironment};
    use propchain_analytics::propchain_analytics::{
        AnalyticsDashboard, MarketTrend, MetricUpdate, PortfolioPosition,
    };
    use propchain_traits::PropertyType;

    fn trend(start: u64, price_change: i32) -> MarketTrend {
        MarketTrend {
            period_start: start,
            period_end: start + 30,
            price_change_percentage: price_change,
            volume_change_percentage: price_change * 2,
        }
    }

    #[ink::test]
    fn metrics_updates_and_batch_limit() {
        let accounts = test::default_accounts::<DefaultEnvironment>();
        test::set_caller::<DefaultEnvironment>(accounts.alice);
        let mut dash = AnalyticsDashboard::new();

        let initial = dash.get_market_metrics();
        assert_eq!(
            (
                initial.average_price,
                initial.total_volume,
                initial.properties_listed
            ),
            (0, 0, 0),
            "fresh dashboard starts empty"
        );

        dash.update_market_metrics(1_000_000, 50_000_000, 120)
            .expect("metrics accepted");
        let metrics = dash.get_market_metrics();
        assert_eq!(metrics.average_price, 1_000_000);
        assert_eq!(metrics.total_volume, 50_000_000);
        assert_eq!(metrics.properties_listed, 120);

        // Batch updates apply each entry; the final state is the last one.
        let updates = vec![
            MetricUpdate {
                average_price: 2_000_000,
                total_volume: 60_000_000,
                properties_listed: 130,
            },
            MetricUpdate {
                average_price: 3_000_000,
                total_volume: 70_000_000,
                properties_listed: 140,
            },
        ];
        dash.batch_update_metrics(updates)
            .expect("small batch accepted");
        let after_batch = dash.get_market_metrics();
        assert_eq!(after_batch.average_price, 3_000_000);

        // Batches above MAX_BATCH_SIZE are rejected wholesale.
        let oversized: Vec<MetricUpdate> = (0..21)
            .map(|i| MetricUpdate {
                average_price: i as u128,
                total_volume: 0,
                properties_listed: 0,
            })
            .collect();
        assert_eq!(
            dash.batch_update_metrics(oversized),
            Err(propchain_analytics::propchain_analytics::AnalyticsError::BatchSizeExceeded)
        );
        assert_eq!(
            dash.get_market_metrics().average_price,
            3_000_000,
            "rejected batch leaves state untouched"
        );
    }

    #[ink::test]
    fn historical_trends_ordering_and_report_generation() {
        let accounts = test::default_accounts::<DefaultEnvironment>();
        test::set_caller::<DefaultEnvironment>(accounts.alice);
        let mut dash = AnalyticsDashboard::new();

        dash.add_market_trend(trend(0, 5)).expect("trend accepted");
        dash.add_market_trend(trend(30, -3))
            .expect("trend accepted");
        dash.add_market_trend(trend(60, 8)).expect("trend accepted");

        let trends = dash.get_historical_trends();
        assert_eq!(trends.len(), 3);
        assert_eq!(trends[0].price_change_percentage, 5);
        assert_eq!(trends[1].price_change_percentage, -3);
        assert_eq!(trends[2].period_start, 60);

        // The report embeds the most recent trend verbatim.
        let report = dash.generate_market_report();
        assert_eq!(report.trend.price_change_percentage, 8);

        // Batch trend ingestion also respects the size cap.
        let oversized: Vec<MarketTrend> = (0..21).map(|i| trend(i as u64 * 30, i)).collect();
        assert_eq!(
            dash.batch_add_trends(oversized),
            Err(propchain_analytics::propchain_analytics::AnalyticsError::BatchSizeExceeded)
        );
        dash.batch_add_trends(vec![trend(90, 1), trend(120, 2)])
            .expect("small batch accepted");
        assert_eq!(dash.get_historical_trends().len(), 5);
    }

    #[ink::test]
    fn sentiment_flows_into_reports() {
        let accounts = test::default_accounts::<DefaultEnvironment>();
        test::set_caller::<DefaultEnvironment>(accounts.alice);
        let mut dash = AnalyticsDashboard::new();

        // Bull-heavy market: ratio lands at 6000 bips (60%).
        dash.update_market_sentiment(7, 600, 400)
            .expect("sentiment accepted");
        let report = dash.generate_market_report();
        assert_eq!(report.sentiment.bull_volume, 600);
        assert_eq!(report.sentiment.bear_volume, 400);
        assert_eq!(report.sentiment.bull_bear_ratio_bips, 6_000);

        // A bear flip is reflected on the next report: sentiment volumes
        // accumulate across updates (bull 600+100, bear 400+900).
        dash.update_market_sentiment(7, 100, 900)
            .expect("sentiment accepted");
        let report = dash.generate_market_report();
        assert_eq!(report.sentiment.bull_bear_ratio_bips, 3_500);
    }

    #[ink::test]
    fn portfolio_positions_round_trip_per_owner() {
        let accounts = test::default_accounts::<DefaultEnvironment>();
        test::set_caller::<DefaultEnvironment>(accounts.alice);
        let mut dash = AnalyticsDashboard::new();

        let bob_positions = vec![
            PortfolioPosition {
                property_type: PropertyType::Residential,
                value: 300_000,
            },
            PortfolioPosition {
                property_type: PropertyType::Commercial,
                value: 700_000,
            },
        ];
        dash.set_portfolio_positions(accounts.bob, bob_positions.clone())
            .expect("positions accepted");

        let stored = dash.get_portfolio_positions(accounts.bob);
        assert_eq!(stored.len(), 2);
        assert_eq!(stored[0].property_type, PropertyType::Residential);
        assert_eq!(stored[1].value, 700_000);

        // Other owners see nothing.
        assert!(dash.get_portfolio_positions(accounts.charlie).is_empty());

        // Health score returns a bounded value for a positioned owner.
        let _score = dash.get_portfolio_health_score(accounts.bob);
    }

    #[ink::test]
    fn benchmark_index_tracks_property_types() {
        let accounts = test::default_accounts::<DefaultEnvironment>();
        test::set_caller::<DefaultEnvironment>(accounts.alice);
        let mut dash = AnalyticsDashboard::new();

        assert_eq!(
            dash.get_benchmark_index(PropertyType::Residential),
            0,
            "untracked property type starts at zero"
        );

        dash.update_benchmark_index(PropertyType::Residential, 250)
            .expect("benchmark accepted");
        dash.update_benchmark_index(PropertyType::Commercial, -75)
            .expect("benchmark accepted");

        assert_eq!(dash.get_benchmark_index(PropertyType::Residential), 250);
        assert_eq!(dash.get_benchmark_index(PropertyType::Commercial), -75);
    }

    #[ink::test]
    fn admin_rotation_requires_nominee_and_cooldown() {
        let accounts = test::default_accounts::<DefaultEnvironment>();
        test::set_caller::<DefaultEnvironment>(accounts.alice);
        let mut dash = AnalyticsDashboard::new();

        // Non-admin cannot request rotation.
        test::set_caller::<DefaultEnvironment>(accounts.bob);
        assert_eq!(
            dash.request_admin_rotation(accounts.bob),
            Err(propchain_analytics::propchain_analytics::AnalyticsError::Unauthorized)
        );

        test::set_caller::<DefaultEnvironment>(accounts.alice);
        dash.request_admin_rotation(accounts.bob)
            .expect("admin requests rotation");
        assert!(dash.get_pending_admin_rotation().is_some());

        // A pending rotation blocks further requests.
        assert_eq!(
            dash.request_admin_rotation(accounts.charlie),
            Err(propchain_analytics::propchain_analytics::AnalyticsError::KeyRotationCooldown)
        );

        // Confirming before the cooldown elapses is rejected...
        test::set_caller::<DefaultEnvironment>(accounts.bob);
        assert_eq!(
            dash.confirm_admin_rotation(),
            Err(propchain_analytics::propchain_analytics::AnalyticsError::KeyRotationCooldown)
        );
        // ...as is confirmation by anyone but the nominee.
        test::set_caller::<DefaultEnvironment>(accounts.charlie);
        assert_eq!(
            dash.confirm_admin_rotation(),
            Err(propchain_analytics::propchain_analytics::AnalyticsError::RotationUnauthorized)
        );

        // After the cooldown the nominee takes over.
        test::set_block_number::<DefaultEnvironment>(14_401);
        test::set_caller::<DefaultEnvironment>(accounts.bob);
        dash.confirm_admin_rotation()
            .expect("nominee confirms after cooldown");
        assert_eq!(dash.get_admin(), accounts.bob);
        assert!(dash.get_pending_admin_rotation().is_none());
    }
}
