/// # Integration Tests: Monitoring & Sanctions Screening (Issues #1003, #1004)
///
/// These tests verify the public message surface of both contracts directly,
/// mirroring the interaction semantics of ink! unit tests.
///
/// Monitoring acceptance criteria tested (Issue #1003):
///   check Alert subscriber subscribe/unsubscribe round-trip
///   check Duplicate subscription is a no-op per contract source
///   check Reporter authorization management incl. non-admin rejection
///   check Health status override + alert config round-trip
///   check Pause/resume gating of operation recording
///   check Admin transfer gate revokes old admin rights
///
/// Sanctions acceptance criteria tested (Issue #1004):
///   check Sanctioned entity/property administration is admin-gated
///   check Screening FAILs while a property is linked to sanctioned state
///   check Screening PASSes after clearing; results are queryable
///   check Entity-based screening honors jurisdiction matching
#[cfg(test)]
#[allow(clippy::module_inception)]
mod integration_monitoring_sanctions {
    // ── Contracts under test ─────────────────────────────────────────────
    use ink::env::{test, DefaultEnvironment};
    use propchain_monitoring::monitoring::MonitoringContract;
    use propchain_sanctions::sanctions_screening::{
        EntityType, Error as SanctionsError, SanctionLevel, SanctionsScreening,
    };
    // Shared types live in the traits crate.
    use propchain_traits::monitoring::{
        AlertType, HealthStatus, MonitoringError, MonitoringSystem, OperationType,
    };

    // ═════════════════════════════════════════════════════════════════════
    // Issue #1003 — Monitoring
    // ═════════════════════════════════════════════════════════════════════

    fn setup_monitoring() -> MonitoringContract {
        let accounts = test::default_accounts::<DefaultEnvironment>();
        test::set_caller::<DefaultEnvironment>(accounts.alice);
        MonitoringContract::new()
    }

    /// Subscriber round-trip: subscribe → listed → unsubscribe → empty.
    #[ink::test]
    fn test_alert_subscriber_round_trip() {
        let accounts = test::default_accounts::<DefaultEnvironment>();
        let mut monitoring = setup_monitoring();

        monitoring
            .subscribe_alerts(accounts.bob)
            .expect("Admin should subscribe an alert recipient");
        assert_eq!(
            monitoring.get_alert_subscribers(),
            vec![accounts.bob],
            "Subscriber must be listed after subscription"
        );

        // Non-admin cannot mutate the subscriber list.
        test::set_caller::<DefaultEnvironment>(accounts.charlie);
        assert_eq!(
            monitoring.unsubscribe_alerts(accounts.bob),
            Err(MonitoringError::Unauthorized),
            "Non-admin must not unsubscribe accounts"
        );

        test::set_caller::<DefaultEnvironment>(accounts.alice);
        monitoring
            .unsubscribe_alerts(accounts.bob)
            .expect("Admin should unsubscribe the recipient");
        assert!(
            monitoring.get_alert_subscribers().is_empty(),
            "Subscriber list must be empty after unsubscription"
        );

        assert_eq!(
            monitoring.unsubscribe_alerts(accounts.bob),
            Err(MonitoringError::SubscriberNotFound),
            "Unsubscribing an unknown account must fail"
        );
    }

    /// Duplicate subscribe: the contract source treats this as an idempotent
    /// no-op (`Ok`, list unchanged) rather than an error variant.
    #[ink::test]
    fn test_duplicate_subscribe_is_idempotent_noop() {
        let accounts = test::default_accounts::<DefaultEnvironment>();
        let mut monitoring = setup_monitoring();

        monitoring
            .subscribe_alerts(accounts.bob)
            .expect("First subscription should succeed");
        monitoring
            .subscribe_alerts(accounts.bob)
            .expect("Source accepts duplicate subscriptions as a no-op");

        assert_eq!(
            monitoring.get_alert_subscribers(),
            vec![accounts.bob],
            "Duplicates must not appear twice in the subscriber list"
        );
    }

    /// Reporter authorization management incl. non-admin rejection.
    #[ink::test]
    fn test_reporter_authorization_management() {
        let accounts = test::default_accounts::<DefaultEnvironment>();
        let mut monitoring = setup_monitoring();

        assert!(
            !monitoring.is_authorized_reporter(accounts.bob),
            "Unknown accounts must not be reporters"
        );

        test::set_caller::<DefaultEnvironment>(accounts.charlie);
        assert_eq!(
            monitoring.add_reporter(accounts.bob),
            Err(MonitoringError::Unauthorized),
            "Non-admin must not add reporters"
        );
        assert_eq!(
            monitoring.remove_reporter(accounts.bob),
            Err(MonitoringError::Unauthorized),
            "Non-admin must not remove reporters"
        );

        test::set_caller::<DefaultEnvironment>(accounts.alice);
        monitoring
            .add_reporter(accounts.bob)
            .expect("Admin should authorize a reporter");
        assert!(monitoring.is_authorized_reporter(accounts.bob));

        monitoring
            .remove_reporter(accounts.bob)
            .expect("Admin should revoke the reporter");
        assert!(!monitoring.is_authorized_reporter(accounts.bob));
    }

    /// Health status override + alert configuration round-trip.
    #[ink::test]
    fn test_health_status_override_and_alert_config_round_trip() {
        let accounts = test::default_accounts::<DefaultEnvironment>();
        let mut monitoring = setup_monitoring();

        assert_eq!(
            monitoring.get_system_status(),
            HealthStatus::Healthy,
            "Fresh deployments must report healthy"
        );

        // Non-admin cannot override health status nor alert config.
        test::set_caller::<DefaultEnvironment>(accounts.charlie);
        assert_eq!(
            monitoring.set_health_status(HealthStatus::Degraded),
            Err(MonitoringError::Unauthorized),
            "Non-admin must not override health status"
        );
        assert_eq!(
            monitoring.set_alert_config(AlertType::HighErrorRate, 500, true),
            Err(MonitoringError::Unauthorized),
            "Non-admin must not change alert config"
        );

        test::set_caller::<DefaultEnvironment>(accounts.alice);
        monitoring
            .set_health_status(HealthStatus::Degraded)
            .expect("Admin should override the health status");
        assert_eq!(monitoring.get_system_status(), HealthStatus::Degraded);

        monitoring
            .set_alert_config(AlertType::HighErrorRate, 500, true)
            .expect("Admin should configure the alert");
        let cfg = monitoring.get_alert_config(AlertType::HighErrorRate);
        assert!(cfg.is_active);
        assert_eq!(cfg.threshold_bips, 500);

        assert_eq!(
            monitoring.set_alert_config(AlertType::HighErrorRate, 10_001, true),
            Err(MonitoringError::InvalidThreshold),
            "Thresholds above 10 000 bips must be rejected"
        );

        // Default config for untouched alert types is inactive.
        assert!(
            !monitoring
                .get_alert_config(AlertType::SystemDegraded)
                .is_active
        );
    }

    /// Pause blocks operation recording; resume restores normal operation.
    ///
    /// `record_operation` is a trait-defined public message of the
    /// `MonitoringSystem` interface and is the operation gated by the pause
    /// switch in the contract source.
    #[ink::test]
    fn test_pause_resume_gates_operation_recording() {
        let accounts = test::default_accounts::<DefaultEnvironment>();
        let mut monitoring = setup_monitoring();

        test::set_caller::<DefaultEnvironment>(accounts.charlie);
        assert_eq!(
            monitoring.pause(),
            Err(MonitoringError::Unauthorized),
            "Non-admin must not pause the contract"
        );

        test::set_caller::<DefaultEnvironment>(accounts.alice);
        monitoring.pause().expect("Admin should pause the contract");
        assert_eq!(monitoring.get_system_status(), HealthStatus::Paused);

        assert_eq!(
            monitoring.record_operation(OperationType::Generic, true),
            Err(MonitoringError::ContractPaused),
            "Operation recording must be gated while paused"
        );

        monitoring
            .resume()
            .expect("Admin should resume the contract");
        assert_eq!(monitoring.get_system_status(), HealthStatus::Healthy);
        monitoring
            .record_operation(OperationType::Generic, true)
            .expect("Recording must work again after resume");

        let metrics = monitoring.get_performance_metrics(OperationType::Generic);
        assert_eq!(metrics.total_calls, 1);
        assert_eq!(metrics.success_count, 1);
    }

    /// Admin transfer revokes the previous admin's privileges.
    #[ink::test]
    fn test_transfer_admin_gate_revokes_old_admin() {
        let accounts = test::default_accounts::<DefaultEnvironment>();
        let mut monitoring = setup_monitoring();

        test::set_caller::<DefaultEnvironment>(accounts.charlie);
        assert_eq!(
            monitoring.transfer_admin(accounts.charlie),
            Err(MonitoringError::Unauthorized),
            "Non-admin must not transfer admin rights"
        );

        test::set_caller::<DefaultEnvironment>(accounts.alice);
        monitoring
            .transfer_admin(accounts.bob)
            .expect("Admin should transfer admin rights");
        assert_eq!(monitoring.get_admin(), accounts.bob);

        // Old admin loses privileges...
        assert_eq!(
            monitoring.pause(),
            Err(MonitoringError::Unauthorized),
            "Old admin must lose admin-gated capabilities"
        );

        // ...while the new admin can exercise them.
        test::set_caller::<DefaultEnvironment>(accounts.bob);
        monitoring
            .pause()
            .expect("New admin should be able to pause");
        assert_eq!(monitoring.get_system_status(), HealthStatus::Paused);
    }

    // ═════════════════════════════════════════════════════════════════════
    // Issue #1004 — Sanctions screening
    // ═════════════════════════════════════════════════════════════════════

    const JURISDICTION_A: u32 = 1001;
    const JURISDICTION_B: u32 = 2002;
    const PROPERTY_ID: u64 = 42;

    fn setup_sanctions() -> SanctionsScreening {
        let accounts = test::default_accounts::<DefaultEnvironment>();
        test::set_caller::<DefaultEnvironment>(accounts.alice);
        SanctionsScreening::new()
    }

    /// Entity + property management is strictly admin-gated.
    #[ink::test]
    fn test_sanctions_administration_is_gated() {
        let accounts = test::default_accounts::<DefaultEnvironment>();
        let mut sanctions = setup_sanctions();
        assert_eq!(sanctions.admin(), accounts.alice);

        test::set_caller::<DefaultEnvironment>(accounts.charlie);
        assert_eq!(
            sanctions.add_sanctioned_entity(
                b"Bad Actor Corp".to_vec(),
                EntityType::Corporation,
                JURISDICTION_A,
                SanctionLevel::Prohibited,
            ),
            Err(SanctionsError::NotAuthorized),
            "Non-admin must not add sanctioned entities"
        );
        assert_eq!(
            sanctions.add_sanctioned_property(
                PROPERTY_ID,
                JURISDICTION_A,
                SanctionLevel::Prohibited,
                b"OFAC list".to_vec(),
            ),
            Err(SanctionsError::NotAuthorized),
            "Non-admin must not sanction properties"
        );
        assert_eq!(
            sanctions.clear_sanctioned_property(PROPERTY_ID),
            Err(SanctionsError::NotAuthorized),
            "Non-admin must not clear sanctioned properties"
        );
        assert_eq!(
            sanctions.screen_property(PROPERTY_ID, JURISDICTION_A, None),
            Err(SanctionsError::NotAuthorized),
            "Screening itself is restricted to the admin"
        );

        // Nothing was persisted by the rejected calls.
        assert!(!sanctions.is_property_screened(PROPERTY_ID));
        assert!(sanctions.get_sanctioned_entity(1).is_none());
    }

    /// Full screening lifecycle: clean pass → sanctioned FAIL → cleared PASS,
    /// with queryable results throughout.
    #[ink::test]
    fn test_property_screening_lifecycle_fail_then_pass() {
        let mut sanctions = setup_sanctions();

        let entity_id = sanctions
            .add_sanctioned_entity(
                b"Bad Actor Corp".to_vec(),
                EntityType::Corporation,
                JURISDICTION_A,
                SanctionLevel::Prohibited,
            )
            .expect("Admin should add a sanctioned entity");
        assert_eq!(entity_id, 1);

        // Clean property passes screening.
        let clean = sanctions
            .screen_property(PROPERTY_ID, JURISDICTION_A, None)
            .expect("Clean screening should succeed");
        assert!(clean.passed);
        assert_eq!(clean.sanction_level, SanctionLevel::None);

        // Link the property to sanctioned state → screening must FAIL.
        sanctions
            .add_sanctioned_property(
                PROPERTY_ID,
                JURISDICTION_A,
                SanctionLevel::Prohibited,
                b"OFAC list".to_vec(),
            )
            .expect("Admin should sanction the property");
        let failed = sanctions
            .screen_property(PROPERTY_ID, JURISDICTION_A, Some(entity_id))
            .expect("Sanctioned screening should record a failure result");
        assert!(!failed.passed, "Sanctioned property must FAIL screening");
        assert_eq!(failed.sanction_level, SanctionLevel::Prohibited);

        // Results remain queryable afterwards.
        let stored = sanctions
            .get_screening_result(failed.screening_id)
            .expect("Failed screening result must be retrievable");
        assert_eq!(stored.property_id, PROPERTY_ID);
        assert_eq!(stored.entity_id, Some(entity_id));
        assert!(!stored.passed);
        assert!(sanctions.is_property_screened(PROPERTY_ID));

        // Clearing the property flips subsequent screenings to PASS.
        // The entity itself remains sanctioned, so it must not be linked
        // here; the issue flow screens the bare property after clearing.
        sanctions
            .clear_sanctioned_property(PROPERTY_ID)
            .expect("Admin should clear the sanctioned property");
        let cleared = sanctions
            .screen_property(PROPERTY_ID, JURISDICTION_A, None)
            .expect("Post-clear screening should succeed");
        assert!(cleared.passed, "Cleared property must PASS screening");
        assert_eq!(cleared.sanction_level, SanctionLevel::None);

        // Every screening against this property is tracked.
        let history = sanctions.get_property_screenings(PROPERTY_ID);
        assert_eq!(
            history.len(),
            3,
            "Clean + failed + cleared screenings must be recorded"
        );
        assert!(history[0].passed);
        assert!(!history[1].passed);
        assert!(history[2].passed);
    }

    /// Entity-based screening matches only within the sanctioned
    /// jurisdiction and stops once the entity is removed.
    #[ink::test]
    fn test_entity_screening_honors_jurisdiction_and_removal() {
        let mut sanctions = setup_sanctions();

        let entity_id = sanctions
            .add_sanctioned_entity(
                b"Restricted Trust".to_vec(),
                EntityType::Trust,
                JURISDICTION_B,
                SanctionLevel::Restricted,
            )
            .expect("Admin should add the entity");

        // Jurisdiction match → FAIL with the entity's sanction level.
        let matched = sanctions
            .screen_property(PROPERTY_ID, JURISDICTION_B, Some(entity_id))
            .expect("Jurisdiction-matched screening should record a result");
        assert!(!matched.passed);
        assert_eq!(matched.sanction_level, SanctionLevel::Restricted);
        assert_eq!(matched.entity_id, Some(entity_id));

        // Different jurisdiction → no match, property passes.
        let other = sanctions
            .screen_property(PROPERTY_ID, JURISDICTION_A, Some(entity_id))
            .expect("Other-jurisdiction screening should succeed");
        assert!(other.passed);
        assert_eq!(other.sanction_level, SanctionLevel::None);

        // Removing the entity clears future matches entirely.
        sanctions
            .remove_sanctioned_entity(entity_id)
            .expect("Admin should remove the sanctioned entity");
        let after_removal = sanctions
            .screen_property(PROPERTY_ID, JURISDICTION_B, Some(entity_id))
            .expect("Screening after entity removal should succeed");
        assert!(after_removal.passed);
    }
}
