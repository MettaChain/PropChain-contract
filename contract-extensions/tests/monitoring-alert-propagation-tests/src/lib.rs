//! Integration tests for monitoring alert propagation (#1532).
//!
//! The monitoring contract aggregates per-operation metrics reported by
//! authorized contracts and derives alerts from the resulting error rate.
//! There is no cross-contract dispatcher and no lending-specific
//! `OperationType`, so the staking, lending and bridge sources are simulated by
//! three authorized reporter accounts: staking and bridge use their native
//! `OperationType` variants and lending reports as `Generic`.
//!
//! The contract exposes no alert-acknowledgement flag and no per-alert
//! escalation timer. The closest behaviours are tested instead:
//! * acknowledgement → the admin resetting the stored health status,
//! * escalation → the automatic Healthy → Degraded/Critical status update and
//!   the re-notification that occurs once the per-alert cooldown elapses,
//! * deduplication → the per-alert cooldown (`MONITORING_ALERT_COOLDOWN_MS`).

#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(test)]
mod tests {
    use ink::env::{test, DefaultEnvironment};
    use ink::primitives::AccountId;
    use propchain_monitoring::monitoring::{AlertTriggered, MonitoringContract};
    use propchain_traits::monitoring::{
        AlertType, HealthStatus, MonitoringError, MonitoringSystem, OperationType,
    };
    use scale::Decode as _;

    type Accounts = test::DefaultAccounts<DefaultEnvironment>;

    const START: u64 = 1_000_000;
    /// `MONITORING_ALERT_COOLDOWN_MS` in `propchain_traits::constants`.
    const COOLDOWN_MS: u64 = 300_000;

    fn setup() -> (MonitoringContract, Accounts) {
        let accounts = test::default_accounts::<DefaultEnvironment>();
        test::set_caller::<DefaultEnvironment>(accounts.alice);
        test::set_block_timestamp::<DefaultEnvironment>(START);
        (MonitoringContract::new(), accounts)
    }

    /// Authorize the three simulated source contracts (staking, lending, bridge).
    fn authorize_sources(m: &mut MonitoringContract, accounts: &Accounts) {
        test::set_caller::<DefaultEnvironment>(accounts.alice);
        m.add_reporter(accounts.bob).expect("staking reporter");
        m.add_reporter(accounts.charlie).expect("lending reporter");
        m.add_reporter(accounts.django).expect("bridge reporter");
    }

    fn report_as(
        from: AccountId,
        m: &mut MonitoringContract,
        operation: OperationType,
        success: bool,
    ) {
        test::set_caller::<DefaultEnvironment>(from);
        m.record_operation(operation, success)
            .expect("authorized reporter can record");
    }

    /// Operations reported from the staking source.
    fn staking(m: &mut MonitoringContract, from: AccountId, successes: u32, failures: u32) {
        for _ in 0..successes {
            report_as(from, m, OperationType::Stake, true);
        }
        for _ in 0..failures {
            report_as(from, m, OperationType::Stake, false);
        }
    }

    /// Decode the `AlertTriggered` events recorded so far.
    ///
    /// `alert_type` is an `#[ink(topic)]` field, so it is read from the event
    /// topics (after the signature topic). ink! also includes the topic field
    /// at the start of the SCALE payload, so it is consumed there before
    /// decoding the remaining fields. Events are matched by their signature
    /// topic so other monitoring events with a two-topic layout are ignored.
    fn recorded_alerts() -> Vec<(AlertType, u32, u32, u64)> {
        let signature = <AlertTriggered as ink::env::Event>::SIGNATURE_TOPIC;
        test::recorded_events()
            .filter_map(
                |event| match (event.topics.first(), event.topics.get(1), signature) {
                    (Some(topic), Some(encoded_alert_type), Some(expected))
                        if topic.as_slice() == &expected[..] =>
                    {
                        let alert_type = AlertType::decode(&mut &encoded_alert_type[..]).ok()?;
                        let mut data = &event.data[..];
                        let _encoded_alert_type = AlertType::decode(&mut data).ok()?;
                        let (current_value, threshold, triggered_at) =
                            <(u32, u32, u64)>::decode(&mut data).ok()?;
                        Some((alert_type, current_value, threshold, triggered_at))
                    }
                    _ => None,
                },
            )
            .collect()
    }

    // ── Data collection from multiple sources ───────────────────────────

    #[ink::test]
    fn operations_from_each_source_are_aggregated_per_operation_type() {
        let (mut m, accounts) = setup();
        authorize_sources(&mut m, &accounts);

        report_as(accounts.bob, &mut m, OperationType::Stake, true);
        report_as(accounts.bob, &mut m, OperationType::Stake, false);
        report_as(accounts.charlie, &mut m, OperationType::Generic, true);
        report_as(accounts.django, &mut m, OperationType::BridgeTransfer, true);
        report_as(accounts.django, &mut m, OperationType::BridgeTransfer, true);

        let stake = m.get_performance_metrics(OperationType::Stake);
        assert_eq!(
            (stake.total_calls, stake.success_count, stake.error_count),
            (2, 1, 1)
        );
        assert_eq!(stake.error_rate_bips, 5_000);
        assert_eq!(stake.last_error_at, START);

        let lending = m.get_performance_metrics(OperationType::Generic);
        assert_eq!((lending.total_calls, lending.error_count), (1, 0));

        let bridge = m.get_performance_metrics(OperationType::BridgeTransfer);
        assert_eq!((bridge.total_calls, bridge.error_count), (2, 0));

        let total: u64 = m.get_all_metrics().iter().map(|m| m.total_calls).sum();
        assert_eq!(total, 5);
        assert_eq!(m.health_check().total_operations, 5);
        // One failure in five operations = 2_000 bips.
        assert_eq!(m.health_check().overall_error_rate_bips, 2_000);
    }

    #[ink::test]
    fn a_source_that_never_reports_contributes_zero_metrics() {
        let (mut m, accounts) = setup();
        authorize_sources(&mut m, &accounts);

        // Only staking reports; lending and bridge stay silent.
        staking(&mut m, accounts.bob, 3, 0);

        assert_eq!(
            m.get_performance_metrics(OperationType::Generic)
                .total_calls,
            0
        );
        assert_eq!(
            m.get_performance_metrics(OperationType::BridgeTransfer)
                .total_calls,
            0
        );
        assert_eq!(m.health_check().total_operations, 3);
        assert_eq!(m.health_check().status, HealthStatus::Healthy);
    }

    // ── Alert recording and severity ────────────────────────────────────

    #[ink::test]
    fn an_alert_records_the_aggregate_threshold_breach_from_several_sources() {
        let (mut m, accounts) = setup();
        m.set_alert_config(AlertType::HighErrorRate, 500, true)
            .expect("admin activates the alert");
        authorize_sources(&mut m, &accounts);

        staking(&mut m, accounts.bob, 2, 0);
        report_as(accounts.charlie, &mut m, OperationType::Generic, true);
        report_as(accounts.django, &mut m, OperationType::BridgeTransfer, true);
        // Fifth operation, first failure: 2_000 bips > 500 bips.
        report_as(
            accounts.django,
            &mut m,
            OperationType::BridgeTransfer,
            false,
        );

        assert_eq!(
            m.get_alert_config(AlertType::HighErrorRate)
                .last_triggered_at,
            START
        );
        let alerts = recorded_alerts();
        assert_eq!(alerts, vec![(AlertType::HighErrorRate, 2_000, 500, START)]);
    }

    #[ink::test]
    fn both_alert_types_are_recorded_independently() {
        let (mut m, accounts) = setup();
        m.set_alert_config(AlertType::HighErrorRate, 500, true)
            .unwrap();
        m.set_alert_config(AlertType::SystemDegraded, 0, true)
            .unwrap();
        authorize_sources(&mut m, &accounts);

        staking(&mut m, accounts.bob, 3, 0);
        report_as(
            accounts.django,
            &mut m,
            OperationType::BridgeTransfer,
            false,
        );

        let alerts = recorded_alerts();
        assert_eq!(
            alerts,
            vec![
                (AlertType::HighErrorRate, 2_500, 500, START),
                (AlertType::SystemDegraded, 2_500, 0, START),
            ]
        );
        // 1 failure in 4 operations = 2_500 bips, the critical boundary.
        assert_eq!(m.get_system_status(), HealthStatus::Critical);
    }

    #[ink::test]
    fn alert_type_is_decoded_from_topics_and_unrelated_events_are_ignored() {
        let (mut m, accounts) = setup();
        m.set_alert_config(AlertType::HighErrorRate, 500, true)
            .unwrap();
        authorize_sources(&mut m, &accounts);

        // `SnapshotTaken` carries 24 bytes of data, which would also decode as
        // (u32, u32, u64); it must not be mistaken for an alert.
        m.take_metrics_snapshot().unwrap();
        report_as(accounts.bob, &mut m, OperationType::Stake, false);

        let signature = <AlertTriggered as ink::env::Event>::SIGNATURE_TOPIC.unwrap();
        let alert_event = test::recorded_events()
            .find(|event| {
                event
                    .topics
                    .first()
                    .map(|topic| topic.as_slice() == &signature[..])
                    .unwrap_or(false)
            })
            .expect("AlertTriggered event");
        assert_eq!(alert_event.topics.len(), 2);
        assert_eq!(
            AlertType::decode(&mut &alert_event.topics[1][..]).unwrap(),
            AlertType::HighErrorRate
        );
        assert_eq!(
            <(AlertType, u32, u32, u64)>::decode(&mut &alert_event.data[..]).unwrap(),
            (AlertType::HighErrorRate, 10_000, 500, START)
        );
        assert_eq!(
            recorded_alerts(),
            vec![(AlertType::HighErrorRate, 10_000, 500, START)]
        );
    }

    #[ink::test]
    fn a_degraded_error_rate_is_classified_as_degraded() {
        let (mut m, accounts) = setup();
        m.set_alert_config(AlertType::SystemDegraded, 0, true)
            .unwrap();
        authorize_sources(&mut m, &accounts);

        staking(&mut m, accounts.bob, 8, 0);
        report_as(accounts.charlie, &mut m, OperationType::Generic, false);

        // One failure in nine operations = 1_111 bips: >= 1_000, < 2_500.
        let health = m.health_check();
        assert_eq!(health.overall_error_rate_bips, 1_111);
        assert_eq!(health.status, HealthStatus::Degraded);
        assert!(health.is_accepting_calls);
        assert_eq!(m.get_system_status(), HealthStatus::Degraded);
        assert_eq!(
            recorded_alerts(),
            vec![(AlertType::SystemDegraded, 1_111, 0, START)]
        );
    }

    #[ink::test]
    fn a_critical_error_rate_is_classified_as_critical() {
        let (mut m, accounts) = setup();
        m.set_alert_config(AlertType::SystemDegraded, 0, true)
            .unwrap();
        authorize_sources(&mut m, &accounts);

        staking(&mut m, accounts.bob, 2, 0);
        report_as(accounts.charlie, &mut m, OperationType::Generic, false);

        // One failure in three operations = 3_333 bips: >= 2_500.
        let health = m.health_check();
        assert_eq!(health.overall_error_rate_bips, 3_333);
        assert_eq!(health.status, HealthStatus::Critical);
        assert_eq!(m.get_system_status(), HealthStatus::Critical);
    }

    // ── Deduplication and re-notification (escalation) ──────────────────

    #[ink::test]
    fn repeated_failures_within_the_cooldown_do_not_duplicate_the_alert() {
        let (mut m, accounts) = setup();
        m.set_alert_config(AlertType::HighErrorRate, 500, true)
            .unwrap();
        authorize_sources(&mut m, &accounts);

        report_as(accounts.bob, &mut m, OperationType::Stake, false);
        assert_eq!(recorded_alerts().len(), 1);

        // Other sources keep failing, but the cooldown has not elapsed.
        test::set_block_timestamp::<DefaultEnvironment>(START + 60_000);
        report_as(accounts.charlie, &mut m, OperationType::Generic, false);
        report_as(
            accounts.django,
            &mut m,
            OperationType::BridgeTransfer,
            false,
        );

        assert_eq!(
            m.get_alert_config(AlertType::HighErrorRate)
                .last_triggered_at,
            START
        );
        assert_eq!(recorded_alerts().len(), 1);
    }

    #[ink::test]
    fn a_still_degraded_system_is_re_alerted_after_the_cooldown() {
        let (mut m, accounts) = setup();
        m.set_alert_config(AlertType::HighErrorRate, 500, true)
            .unwrap();
        authorize_sources(&mut m, &accounts);

        report_as(accounts.bob, &mut m, OperationType::Stake, false);
        assert_eq!(recorded_alerts().len(), 1);

        // Exactly at the cooldown boundary the aggregate breach re-fires.
        test::set_block_timestamp::<DefaultEnvironment>(START + COOLDOWN_MS);
        report_as(
            accounts.django,
            &mut m,
            OperationType::BridgeTransfer,
            false,
        );

        let alerts = recorded_alerts();
        assert_eq!(alerts.len(), 2);
        assert_eq!(
            alerts[1],
            (AlertType::HighErrorRate, 10_000, 500, START + COOLDOWN_MS)
        );
        assert_eq!(
            m.get_alert_config(AlertType::HighErrorRate)
                .last_triggered_at,
            START + COOLDOWN_MS
        );
    }

    // ── Acknowledgement and operational controls ────────────────────────

    #[ink::test]
    fn only_the_admin_can_acknowledge_by_resetting_the_health_status() {
        let (mut m, accounts) = setup();
        m.set_alert_config(AlertType::SystemDegraded, 0, true)
            .unwrap();
        authorize_sources(&mut m, &accounts);
        staking(&mut m, accounts.bob, 4, 0);
        report_as(accounts.charlie, &mut m, OperationType::Generic, false);
        // 1 failure in 5 operations = 2_000 bips: degraded.
        assert_eq!(m.get_system_status(), HealthStatus::Degraded);

        // There is no dedicated acknowledge flag; the admin reset is the
        // closest supported action.
        test::set_caller::<DefaultEnvironment>(accounts.charlie);
        assert_eq!(
            m.set_health_status(HealthStatus::Healthy),
            Err(MonitoringError::Unauthorized)
        );
        assert_eq!(m.get_system_status(), HealthStatus::Degraded);

        test::set_caller::<DefaultEnvironment>(accounts.alice);
        let before = test::recorded_events().count();
        m.set_health_status(HealthStatus::Healthy).unwrap();
        assert_eq!(m.get_system_status(), HealthStatus::Healthy);
        // One `HealthStatusChanged` event is emitted by the reset.
        assert_eq!(test::recorded_events().count() - before, 1);
    }

    #[ink::test]
    fn pausing_prevents_recordings_and_alerts_until_resumed() {
        let (mut m, accounts) = setup();
        m.set_alert_config(AlertType::HighErrorRate, 500, true)
            .unwrap();
        authorize_sources(&mut m, &accounts);

        test::set_caller::<DefaultEnvironment>(accounts.alice);
        m.pause().unwrap();
        assert_eq!(m.get_system_status(), HealthStatus::Paused);
        assert_eq!(m.health_check().status, HealthStatus::Paused);
        assert!(!m.health_check().is_accepting_calls);

        test::set_caller::<DefaultEnvironment>(accounts.django);
        assert_eq!(
            m.record_operation(OperationType::BridgeTransfer, false),
            Err(MonitoringError::ContractPaused)
        );
        assert_eq!(
            m.get_alert_config(AlertType::HighErrorRate)
                .last_triggered_at,
            0
        );
        assert!(recorded_alerts().is_empty());

        test::set_caller::<DefaultEnvironment>(accounts.alice);
        m.resume().unwrap();
        assert_eq!(m.get_system_status(), HealthStatus::Healthy);

        test::set_caller::<DefaultEnvironment>(accounts.django);
        m.record_operation(OperationType::BridgeTransfer, false)
            .unwrap();
        assert_eq!(recorded_alerts().len(), 1);
    }

    // ── Source authorization and subscriber management ──────────────────

    #[ink::test]
    fn only_authorized_sources_can_report() {
        let (mut m, accounts) = setup();

        test::set_caller::<DefaultEnvironment>(accounts.eve);
        assert_eq!(
            m.record_operation(OperationType::Generic, true),
            Err(MonitoringError::Unauthorized)
        );
        assert_eq!(m.health_check().total_operations, 0);

        test::set_caller::<DefaultEnvironment>(accounts.alice);
        m.add_reporter(accounts.bob).unwrap();
        assert!(m.is_authorized_reporter(accounts.bob));
        report_as(accounts.bob, &mut m, OperationType::Stake, true);

        test::set_caller::<DefaultEnvironment>(accounts.alice);
        m.remove_reporter(accounts.bob).unwrap();
        assert!(!m.is_authorized_reporter(accounts.bob));
        test::set_caller::<DefaultEnvironment>(accounts.bob);
        assert_eq!(
            m.record_operation(OperationType::Stake, true),
            Err(MonitoringError::Unauthorized)
        );
    }

    #[ink::test]
    fn alert_subscribers_are_managed_by_the_admin_and_survive_an_alert() {
        let (mut m, accounts) = setup();

        test::set_caller::<DefaultEnvironment>(accounts.bob);
        assert_eq!(
            m.subscribe_alerts(accounts.charlie),
            Err(MonitoringError::Unauthorized)
        );

        test::set_caller::<DefaultEnvironment>(accounts.alice);
        m.subscribe_alerts(accounts.bob).unwrap();
        m.subscribe_alerts(accounts.charlie).unwrap();
        // Duplicate subscriptions are ignored.
        m.subscribe_alerts(accounts.bob).unwrap();
        assert_eq!(
            m.get_alert_subscribers(),
            vec![accounts.bob, accounts.charlie]
        );

        m.set_alert_config(AlertType::HighErrorRate, 500, true)
            .unwrap();
        authorize_sources(&mut m, &accounts);
        report_as(
            accounts.django,
            &mut m,
            OperationType::BridgeTransfer,
            false,
        );
        assert_eq!(recorded_alerts().len(), 1);
        // The trigger does not disturb the subscriber list.
        assert_eq!(
            m.get_alert_subscribers(),
            vec![accounts.bob, accounts.charlie]
        );

        test::set_caller::<DefaultEnvironment>(accounts.alice);
        m.unsubscribe_alerts(accounts.charlie).unwrap();
        assert_eq!(m.get_alert_subscribers(), vec![accounts.bob]);
    }
}
