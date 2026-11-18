//! Unit tests for the monitoring contract's alert thresholds, trigger evaluation
//! and notification emission (#1465).
//!
//! The contract has no explicit alert *acknowledgement* or *escalation timer*;
//! the nearest behaviours it does have are tested instead: a per-alert cooldown
//! (re-trigger suppression), automatic health-status escalation, the metrics
//! snapshot ring buffer (as alert/metric history), and pause/resume.

#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(test)]
mod tests {
    use ink::env::{test, DefaultEnvironment};
    use propchain_monitoring::monitoring::MonitoringContract;
    use propchain_traits::monitoring::{
        AlertType, HealthStatus, MonitoringError, MonitoringSystem, OperationType,
    };

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

    fn ok_ops(m: &mut MonitoringContract, n: u32) {
        for _ in 0..n {
            m.record_operation(OperationType::RegisterProperty, true).unwrap();
        }
    }

    fn fail_op(m: &mut MonitoringContract) {
        m.record_operation(OperationType::RegisterProperty, false).unwrap();
    }

    // ── Threshold configuration ─────────────────────────────────────────

    #[ink::test]
    fn admin_configures_an_alert_threshold_and_reads_it_back() {
        let (mut m, _) = setup();
        let before = m.get_alert_config(AlertType::HighErrorRate);
        assert!(!before.is_active);
        assert_eq!(before.threshold_bips, 1_000); // contract default
        assert_eq!(before.last_triggered_at, 0);

        m.set_alert_config(AlertType::HighErrorRate, 500, true).unwrap();
        let after = m.get_alert_config(AlertType::HighErrorRate);
        assert_eq!(after.alert_type, AlertType::HighErrorRate);
        assert_eq!(after.threshold_bips, 500);
        assert!(after.is_active);
    }

    #[ink::test]
    fn only_the_admin_can_configure_thresholds() {
        let (mut m, accounts) = setup();
        test::set_caller::<DefaultEnvironment>(accounts.bob);
        assert_eq!(
            m.set_alert_config(AlertType::HighErrorRate, 500, true),
            Err(MonitoringError::Unauthorized)
        );
        assert!(!m.get_alert_config(AlertType::HighErrorRate).is_active);
    }

    #[ink::test]
    fn thresholds_above_one_hundred_percent_are_rejected() {
        let (mut m, _) = setup();
        assert_eq!(
            m.set_alert_config(AlertType::HighErrorRate, 10_001, true),
            Err(MonitoringError::InvalidThreshold)
        );
        // The boundary value itself is accepted.
        assert!(m.set_alert_config(AlertType::HighErrorRate, 10_000, true).is_ok());
    }

    #[ink::test]
    fn alert_types_are_configured_independently() {
        let (mut m, _) = setup();
        m.set_alert_config(AlertType::HighErrorRate, 700, true).unwrap();
        let degraded = m.get_alert_config(AlertType::SystemDegraded);
        assert!(!degraded.is_active);
        assert_ne!(degraded.threshold_bips, 700);
    }

    // ── Trigger evaluation ──────────────────────────────────────────────

    #[ink::test]
    fn alert_triggers_when_the_error_rate_exceeds_the_threshold() {
        let (mut m, _) = setup();
        m.set_alert_config(AlertType::HighErrorRate, 500, true).unwrap();
        ok_ops(&mut m, 9);
        assert_eq!(m.get_alert_config(AlertType::HighErrorRate).last_triggered_at, 0);

        // 1 failure in 10 calls = 1_000 bips, above the 500 bip threshold.
        fail_op(&mut m);
        assert_eq!(
            m.get_alert_config(AlertType::HighErrorRate).last_triggered_at,
            START
        );
    }

    #[ink::test]
    fn no_alert_when_the_error_rate_is_within_bounds() {
        let (mut m, _) = setup();
        m.set_alert_config(AlertType::HighErrorRate, 2_000, true).unwrap();
        ok_ops(&mut m, 9);
        fail_op(&mut m); // 1_000 bips, below 2_000
        assert_eq!(m.get_alert_config(AlertType::HighErrorRate).last_triggered_at, 0);
        assert_eq!(m.get_system_status(), HealthStatus::Healthy);
    }

    #[ink::test]
    fn an_inactive_alert_never_fires() {
        let (mut m, _) = setup();
        m.set_alert_config(AlertType::HighErrorRate, 100, false).unwrap();
        fail_op(&mut m);
        assert_eq!(m.get_alert_config(AlertType::HighErrorRate).last_triggered_at, 0);
    }

    #[ink::test]
    fn the_cooldown_suppresses_repeat_alerts_until_it_elapses() {
        let (mut m, _) = setup();
        m.set_alert_config(AlertType::HighErrorRate, 500, true).unwrap();
        fail_op(&mut m);
        assert_eq!(m.get_alert_config(AlertType::HighErrorRate).last_triggered_at, START);

        // Still inside the cooldown window: no new trigger.
        test::set_block_timestamp::<DefaultEnvironment>(START + COOLDOWN_MS - 1);
        fail_op(&mut m);
        assert_eq!(m.get_alert_config(AlertType::HighErrorRate).last_triggered_at, START);

        // Exactly at the cooldown: triggers again.
        test::set_block_timestamp::<DefaultEnvironment>(START + COOLDOWN_MS);
        fail_op(&mut m);
        assert_eq!(
            m.get_alert_config(AlertType::HighErrorRate).last_triggered_at,
            START + COOLDOWN_MS
        );
    }

    #[ink::test]
    fn system_degraded_alert_escalates_the_stored_health_status() {
        let (mut m, _) = setup();
        m.set_alert_config(AlertType::SystemDegraded, 0, true).unwrap();
        ok_ops(&mut m, 4);
        assert_eq!(m.get_system_status(), HealthStatus::Healthy);

        // 1 failure in 5 calls = 2_000 bips: degraded (>= 1_000, < 2_500).
        fail_op(&mut m);
        assert_eq!(m.get_alert_config(AlertType::SystemDegraded).last_triggered_at, START);
        assert_eq!(m.get_system_status(), HealthStatus::Degraded);
        // The other alert type was not configured, so it did not fire.
        assert_eq!(m.get_alert_config(AlertType::HighErrorRate).last_triggered_at, 0);
    }

    #[ink::test]
    fn health_check_reflects_the_live_error_rate() {
        let (mut m, _) = setup();
        ok_ops(&mut m, 9);
        assert_eq!(m.health_check().status, HealthStatus::Healthy);
        fail_op(&mut m);
        let result = m.health_check();
        assert_eq!(result.total_operations, 10);
        assert_eq!(result.overall_error_rate_bips, 1_000);
        assert_eq!(result.status, HealthStatus::Degraded);
        assert!(result.is_accepting_calls);
    }

    // ── Notification emission ───────────────────────────────────────────

    #[ink::test]
    fn recording_an_operation_emits_an_event_and_an_alert_adds_another() {
        let (mut m, _) = setup();
        m.record_operation(OperationType::MintToken, true).unwrap();
        assert_eq!(test::recorded_events().count(), 1);

        m.set_alert_config(AlertType::HighErrorRate, 500, true).unwrap();
        let before = test::recorded_events().count();
        fail_op(&mut m);
        // OperationRecorded + AlertTriggered.
        assert_eq!(test::recorded_events().count() - before, 2);
    }

    // ── Pause / resume ──────────────────────────────────────────────────

    #[ink::test]
    fn pausing_blocks_recording_and_resuming_restores_it() {
        let (mut m, _) = setup();
        m.pause().unwrap();
        assert_eq!(m.get_system_status(), HealthStatus::Paused);
        assert_eq!(m.health_check().status, HealthStatus::Paused);
        assert!(!m.health_check().is_accepting_calls);
        assert_eq!(
            m.record_operation(OperationType::MintToken, true),
            Err(MonitoringError::ContractPaused)
        );

        m.resume().unwrap();
        assert_eq!(m.get_system_status(), HealthStatus::Healthy);
        assert!(m.record_operation(OperationType::MintToken, true).is_ok());
    }

    #[ink::test]
    fn only_the_admin_can_pause_or_resume() {
        let (mut m, accounts) = setup();
        test::set_caller::<DefaultEnvironment>(accounts.bob);
        assert_eq!(m.pause(), Err(MonitoringError::Unauthorized));
        assert_eq!(m.resume(), Err(MonitoringError::Unauthorized));
    }

    // ── History (metrics snapshots) ─────────────────────────────────────

    #[ink::test]
    fn snapshots_form_an_ordered_history() {
        let (mut m, _) = setup();
        ok_ops(&mut m, 3);
        m.take_metrics_snapshot().unwrap();
        fail_op(&mut m);
        test::set_block_timestamp::<DefaultEnvironment>(START + 10);
        m.take_metrics_snapshot().unwrap();

        let first = m.get_metrics_snapshot(0).expect("slot 0");
        let second = m.get_metrics_snapshot(1).expect("slot 1");
        assert_eq!((first.snapshot_id, first.total_calls, first.total_errors), (0, 3, 0));
        assert_eq!((second.snapshot_id, second.total_calls, second.total_errors), (1, 4, 1));
        assert_eq!(second.error_rate_bips, 2_500);
        assert!(second.timestamp > first.timestamp);
        assert!(m.get_metrics_snapshot(2).is_none());
    }

    #[ink::test]
    fn snapshots_need_authorization_and_an_unpaused_contract() {
        let (mut m, accounts) = setup();
        test::set_caller::<DefaultEnvironment>(accounts.bob);
        assert_eq!(m.take_metrics_snapshot(), Err(MonitoringError::Unauthorized));

        test::set_caller::<DefaultEnvironment>(accounts.alice);
        m.pause().unwrap();
        assert_eq!(m.take_metrics_snapshot(), Err(MonitoringError::ContractPaused));
    }

    #[ink::test]
    fn authorized_reporters_can_record_but_strangers_cannot() {
        let (mut m, accounts) = setup();
        test::set_caller::<DefaultEnvironment>(accounts.bob);
        assert_eq!(
            m.record_operation(OperationType::Stake, true),
            Err(MonitoringError::Unauthorized)
        );

        test::set_caller::<DefaultEnvironment>(accounts.alice);
        m.add_reporter(accounts.bob).unwrap();
        test::set_caller::<DefaultEnvironment>(accounts.bob);
        assert!(m.record_operation(OperationType::Stake, true).is_ok());
    }
}
