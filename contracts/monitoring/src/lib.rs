#![cfg_attr(not(feature = "std"), no_std, no_main)]

#[ink::contract]
mod monitoring {
    use ink::prelude::vec::Vec;
    use ink::storage::Mapping;
    use propchain_traits::constants;
    use propchain_traits::monitoring::*;

    // =========================================================================
    // Internal storage type (not part of cross-contract interface)
    // =========================================================================

    #[derive(
        Debug, Clone, Default, scale::Encode, scale::Decode, ink::storage::traits::StorageLayout,
    )]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    struct OperationRecord {
        total_calls: u64,
        success_count: u64,
        error_count: u64,
        last_called_at: u64,
        last_error_at: u64,
    }

    // =========================================================================
    // Events
    // =========================================================================

    #[ink(event)]
    pub struct OperationRecorded {
        #[ink(topic)]
        pub operation: OperationType,
        pub success: bool,
        pub timestamp: u64,
    }

    #[ink(event)]
    pub struct AlertTriggered {
        #[ink(topic)]
        pub alert_type: AlertType,
        pub current_value: u32,
        pub threshold: u32,
        pub triggered_at: u64,
    }

    #[ink(event)]
    pub struct HealthStatusChanged {
        pub old_status: HealthStatus,
        pub new_status: HealthStatus,
        pub changed_at: u64,
    }

    #[ink(event)]
    pub struct SnapshotTaken {
        pub snapshot_id: u64,
        pub slot: u64,
        pub timestamp: u64,
    }

    #[ink(event)]
    pub struct ReporterAdded {
        #[ink(topic)]
        pub reporter: AccountId,
        pub added_by: AccountId,
    }

    #[ink(event)]
    pub struct ReporterRemoved {
        #[ink(topic)]
        pub reporter: AccountId,
        pub removed_by: AccountId,
    }

    // =========================================================================
    // Storage
    // =========================================================================

    #[ink(storage)]
    pub struct MonitoringContract {
        admin: AccountId,
        authorized_reporters: Mapping<AccountId, bool>,
        health_status: HealthStatus,
        deployed_at: u64,
        is_paused: bool,
        // Aggregate counters
        total_calls: u64,
        total_errors: u64,
        // Per-operation metrics
        operation_records: Mapping<OperationType, OperationRecord>,
        // Alert configuration
        alert_thresholds: Mapping<AlertType, u32>,
        alert_active: Mapping<AlertType, bool>,
        alert_last_triggered: Mapping<AlertType, u64>,
        alert_subscribers: Vec<AccountId>,
        // Metrics snapshots (circular buffer, size = MONITORING_MAX_SNAPSHOTS)
        snapshots: Mapping<u64, MetricsSnapshot>,
        snapshot_count: u64,
    }

    // =========================================================================
    // MonitoringSystem trait implementation
    // =========================================================================

    impl MonitoringSystem for MonitoringContract {
        /// Records a single operation outcome. Restricted to admin and authorized reporters.
        #[ink(message)]
        fn record_operation(
            &mut self,
            operation: OperationType,
            success: bool,
        ) -> Result<(), MonitoringError> {
            if self.is_paused {
                return Err(MonitoringError::ContractPaused);
            }
            self.ensure_authorized()?;

            let now = self.env().block_timestamp();
            let mut record = self.operation_records.get(operation).unwrap_or_default();

            record.total_calls = record.total_calls.saturating_add(1);
            record.last_called_at = now;
            if success {
                record.success_count = record.success_count.saturating_add(1);
            } else {
                record.error_count = record.error_count.saturating_add(1);
                record.last_error_at = now;
            }
            self.operation_records.insert(operation, &record);

            self.total_calls = self.total_calls.saturating_add(1);
            if !success {
                self.total_errors = self.total_errors.saturating_add(1);
            }

            self.check_and_trigger_alerts();

            self.env().emit_event(OperationRecorded {
                operation,
                success,
                timestamp: now,
            });

            Ok(())
        }

        /// Returns accumulated metrics for a specific operation type.
        #[ink(message)]
        fn get_performance_metrics(&self, operation: OperationType) -> PerformanceMetrics {
            let record = self.operation_records.get(operation).unwrap_or_default();
            let error_rate_bips =
                Self::compute_error_rate_bips(record.error_count, record.total_calls);
            PerformanceMetrics {
                operation,
                total_calls: record.total_calls,
                success_count: record.success_count,
                error_count: record.error_count,
                error_rate_bips,
                last_called_at: record.last_called_at,
                last_error_at: record.last_error_at,
            }
        }

        /// Returns metrics for all known operation types.
        #[ink(message)]
        fn get_all_metrics(&self) -> Vec<PerformanceMetrics> {
            Self::all_operation_types()
                .into_iter()
                .map(|op| self.get_performance_metrics(op))
                .collect()
        }

        /// Computes and returns a live health-check result based on current metrics.
        #[ink(message)]
        fn health_check(&self) -> HealthCheckResult {
            let error_rate_bips =
                Self::compute_error_rate_bips(self.total_errors, self.total_calls);
            let computed = Self::compute_health_status(error_rate_bips);
            let uptime_blocks = (self.env().block_number() as u64).saturating_sub(self.deployed_at);

            HealthCheckResult {
                status: if self.is_paused {
                    HealthStatus::Paused
                } else {
                    computed
                },
                checked_at: self.env().block_timestamp(),
                total_operations: self.total_calls,
                overall_error_rate_bips: error_rate_bips,
                uptime_blocks,
                is_accepting_calls: !self.is_paused,
            }
        }

        /// Returns the currently stored (admin-controlled) health status.
        #[ink(message)]
        fn get_system_status(&self) -> HealthStatus {
            self.health_status
        }

        /// Persists a point-in-time aggregate snapshot in the circular buffer.
        #[ink(message)]
        fn take_metrics_snapshot(&mut self) -> Result<(), MonitoringError> {
            if self.is_paused {
                return Err(MonitoringError::ContractPaused);
            }
            self.ensure_authorized()?;

            let slot = self.snapshot_count % constants::MONITORING_MAX_SNAPSHOTS;
            let error_rate_bips =
                Self::compute_error_rate_bips(self.total_errors, self.total_calls);
            let now = self.env().block_timestamp();

            self.snapshots.insert(
                slot,
                &MetricsSnapshot {
                    snapshot_id: self.snapshot_count,
                    timestamp: now,
                    total_calls: self.total_calls,
                    total_errors: self.total_errors,
                    error_rate_bips,
                },
            );

            self.env().emit_event(SnapshotTaken {
                snapshot_id: self.snapshot_count,
                slot,
                timestamp: now,
            });

            self.snapshot_count = self.snapshot_count.saturating_add(1);
            Ok(())
        }

        /// Retrieves a previously stored snapshot by its circular-buffer slot index.
        #[ink(message)]
        fn get_metrics_snapshot(&self, slot: u64) -> Option<MetricsSnapshot> {
            self.snapshots.get(slot)
        }
    }

    // =========================================================================
    // Implementation — admin & configuration messages
    // =========================================================================

    impl MonitoringContract {
        /// Deploys the monitoring contract. The caller becomes admin.
        #[ink(constructor)]
        pub fn new() -> Self {
            let caller = Self::env().caller();
            Self {
                admin: caller,
                authorized_reporters: Mapping::default(),
                health_status: HealthStatus::Healthy,
                deployed_at: Self::env().block_number() as u64,
                is_paused: false,
                total_calls: 0,
                total_errors: 0,
                operation_records: Mapping::default(),
                alert_thresholds: Mapping::default(),
                alert_active: Mapping::default(),
                alert_last_triggered: Mapping::default(),
                alert_subscribers: Vec::new(),
                snapshots: Mapping::default(),
                snapshot_count: 0,
            }
        }

        /// Manually override the stored health status. Admin only.
        #[ink(message)]
        pub fn set_health_status(&mut self, status: HealthStatus) -> Result<(), MonitoringError> {
            self.ensure_admin()?;
            let old = self.health_status;
            self.health_status = status;
            if old != status {
                self.env().emit_event(HealthStatusChanged {
                    old_status: old,
                    new_status: status,
                    changed_at: self.env().block_timestamp(),
                });
            }
            Ok(())
        }

        /// Configure an alert type. `threshold_bips` = 0 means "use default". Admin only.
        ///
        /// For HighErrorRate: threshold_bips is the error-rate trigger level.
        /// For SystemDegraded: threshold_bips is ignored.
        #[ink(message)]
        pub fn set_alert_config(
            &mut self,
            alert_type: AlertType,
            threshold_bips: u32,
            active: bool,
        ) -> Result<(), MonitoringError> {
            self.ensure_admin()?;
            if threshold_bips > constants::BASIS_POINTS_DENOMINATOR {
                return Err(MonitoringError::InvalidThreshold);
            }
            self.alert_thresholds.insert(alert_type, &threshold_bips);
            self.alert_active.insert(alert_type, &active);
            Ok(())
        }

        /// Returns the current configuration for a given alert type.
        #[ink(message)]
        pub fn get_alert_config(&self, alert_type: AlertType) -> AlertConfig {
            AlertConfig {
                alert_type,
                threshold_bips: self
                    .alert_thresholds
                    .get(alert_type)
                    .unwrap_or(constants::MONITORING_DEFAULT_ERROR_RATE_THRESHOLD_BIPS),
                is_active: self.alert_active.get(alert_type).unwrap_or(false),
                last_triggered_at: self.alert_last_triggered.get(alert_type).unwrap_or(0),
            }
        }

        /// Add an account to the alert subscriber list. Admin only.
        #[ink(message)]
        pub fn subscribe_alerts(&mut self, subscriber: AccountId) -> Result<(), MonitoringError> {
            self.ensure_admin()?;
            if self.alert_subscribers.len() >= constants::MONITORING_MAX_SUBSCRIBERS {
                return Err(MonitoringError::SubscriberLimitReached);
            }
            if !self.alert_subscribers.contains(&subscriber) {
                self.alert_subscribers.push(subscriber);
            }
            Ok(())
        }

        /// Remove an account from the alert subscriber list. Admin only.
        #[ink(message)]
        pub fn unsubscribe_alerts(&mut self, subscriber: AccountId) -> Result<(), MonitoringError> {
            self.ensure_admin()?;
            let pos = self
                .alert_subscribers
                .iter()
                .position(|s| *s == subscriber)
                .ok_or(MonitoringError::SubscriberNotFound)?;
            self.alert_subscribers.swap_remove(pos);
            Ok(())
        }

        /// Returns the list of registered alert subscribers.
        #[ink(message)]
        pub fn get_alert_subscribers(&self) -> Vec<AccountId> {
            self.alert_subscribers.clone()
        }

        /// Authorize an external account or contract to call `record_operation`. Admin only.
        #[ink(message)]
        pub fn add_reporter(&mut self, reporter: AccountId) -> Result<(), MonitoringError> {
            self.ensure_admin()?;
            self.authorized_reporters.insert(reporter, &true);
            self.env().emit_event(ReporterAdded {
                reporter,
                added_by: self.env().caller(),
            });
            Ok(())
        }

        /// Revoke a previously authorized reporter. Admin only.
        #[ink(message)]
        pub fn remove_reporter(&mut self, reporter: AccountId) -> Result<(), MonitoringError> {
            self.ensure_admin()?;
            self.authorized_reporters.insert(reporter, &false);
            self.env().emit_event(ReporterRemoved {
                reporter,
                removed_by: self.env().caller(),
            });
            Ok(())
        }

        /// Returns whether `account` is an authorized reporter.
        #[ink(message)]
        pub fn is_authorized_reporter(&self, account: AccountId) -> bool {
            self.authorized_reporters.get(account).unwrap_or(false)
        }

        /// Pause the contract, blocking new operation recordings and snapshots. Admin only.
        #[ink(message)]
        pub fn pause(&mut self) -> Result<(), MonitoringError> {
            self.ensure_admin()?;
            if !self.is_paused {
                self.is_paused = true;
                let old = self.health_status;
                self.health_status = HealthStatus::Paused;
                self.env().emit_event(HealthStatusChanged {
                    old_status: old,
                    new_status: HealthStatus::Paused,
                    changed_at: self.env().block_timestamp(),
                });
            }
            Ok(())
        }

        /// Resume a paused contract and restore the health status to Healthy. Admin only.
        #[ink(message)]
        pub fn resume(&mut self) -> Result<(), MonitoringError> {
            self.ensure_admin()?;
            if self.is_paused {
                self.is_paused = false;
                self.health_status = HealthStatus::Healthy;
                self.env().emit_event(HealthStatusChanged {
                    old_status: HealthStatus::Paused,
                    new_status: HealthStatus::Healthy,
                    changed_at: self.env().block_timestamp(),
                });
            }
            Ok(())
        }

        /// Returns the admin account.
        #[ink(message)]
        pub fn get_admin(&self) -> AccountId {
            self.admin
        }

        /// Transfer admin rights to a new account. Admin only.
        #[ink(message)]
        pub fn transfer_admin(&mut self, new_admin: AccountId) -> Result<(), MonitoringError> {
            self.ensure_admin()?;
            self.admin = new_admin;
            Ok(())
        }

        // =====================================================================
        // Private helpers
        // =====================================================================

        fn ensure_admin(&self) -> Result<(), MonitoringError> {
            if self.env().caller() != self.admin {
                return Err(MonitoringError::Unauthorized);
            }
            Ok(())
        }

        fn ensure_authorized(&self) -> Result<(), MonitoringError> {
            let caller = self.env().caller();
            if caller == self.admin || self.authorized_reporters.get(caller).unwrap_or(false) {
                return Ok(());
            }
            Err(MonitoringError::Unauthorized)
        }

        /// error_rate_bips = (errors * 10_000) / total, saturating at 10_000.
        fn compute_error_rate_bips(errors: u64, total: u64) -> u32 {
            if total == 0 {
                return 0;
            }
            let bips = errors
                .saturating_mul(constants::BASIS_POINTS_DENOMINATOR as u64)
                .checked_div(total)
                .unwrap_or(0)
                .min(constants::BASIS_POINTS_DENOMINATOR as u64);
            // Safety: value is clamped to BASIS_POINTS_DENOMINATOR (10_000) which fits in u32
            #[allow(clippy::cast_possible_truncation)]
            {
                bips as u32
            }
        }

        fn compute_health_status(error_rate_bips: u32) -> HealthStatus {
            if error_rate_bips >= constants::MONITORING_CRITICAL_THRESHOLD_BIPS {
                HealthStatus::Critical
            } else if error_rate_bips >= constants::MONITORING_DEGRADED_THRESHOLD_BIPS {
                HealthStatus::Degraded
            } else {
                HealthStatus::Healthy
            }
        }

        /// Check both alert types and emit `AlertTriggered` events when thresholds are breached.
        /// Also updates `health_status` automatically on SystemDegraded.
        fn check_and_trigger_alerts(&mut self) {
            let now = self.env().block_timestamp();
            let error_rate_bips =
                Self::compute_error_rate_bips(self.total_errors, self.total_calls);

            // ── HighErrorRate ────────────────────────────────────────────────
            if self
                .alert_active
                .get(AlertType::HighErrorRate)
                .unwrap_or(false)
            {
                let threshold = self
                    .alert_thresholds
                    .get(AlertType::HighErrorRate)
                    .unwrap_or(constants::MONITORING_DEFAULT_ERROR_RATE_THRESHOLD_BIPS);

                if error_rate_bips > threshold {
                    let last = self
                        .alert_last_triggered
                        .get(AlertType::HighErrorRate)
                        .unwrap_or(0);
                    if now.saturating_sub(last) >= constants::MONITORING_ALERT_COOLDOWN_MS {
                        self.alert_last_triggered
                            .insert(AlertType::HighErrorRate, &now);
                        self.env().emit_event(AlertTriggered {
                            alert_type: AlertType::HighErrorRate,
                            current_value: error_rate_bips,
                            threshold,
                            triggered_at: now,
                        });
                    }
                }
            }

            // ── SystemDegraded ───────────────────────────────────────────────
            if self
                .alert_active
                .get(AlertType::SystemDegraded)
                .unwrap_or(false)
            {
                let computed = Self::compute_health_status(error_rate_bips);
                if computed != HealthStatus::Healthy {
                    let last = self
                        .alert_last_triggered
                        .get(AlertType::SystemDegraded)
                        .unwrap_or(0);
                    if now.saturating_sub(last) >= constants::MONITORING_ALERT_COOLDOWN_MS {
                        self.alert_last_triggered
                            .insert(AlertType::SystemDegraded, &now);
                        self.env().emit_event(AlertTriggered {
                            alert_type: AlertType::SystemDegraded,
                            current_value: error_rate_bips,
                            threshold: 0,
                            triggered_at: now,
                        });

                        // Automatically escalate stored health status (never de-escalate here).
                        if self.health_status == HealthStatus::Healthy {
                            let old = self.health_status;
                            self.health_status = computed;
                            self.env().emit_event(HealthStatusChanged {
                                old_status: old,
                                new_status: computed,
                                changed_at: now,
                            });
                        }
                    }
                }
            }
        }

        fn all_operation_types() -> Vec<OperationType> {
            ink::prelude::vec![
                OperationType::RegisterProperty,
                OperationType::TransferProperty,
                OperationType::UpdateMetadata,
                OperationType::CreateEscrow,
                OperationType::ReleaseEscrow,
                OperationType::RefundEscrow,
                OperationType::MintToken,
                OperationType::BurnToken,
                OperationType::BridgeTransfer,
                OperationType::Stake,
                OperationType::Unstake,
                OperationType::GovernanceVote,
                OperationType::OracleUpdate,
                OperationType::ComplianceCheck,
                OperationType::FeeCollection,
                OperationType::Generic,
            ]
        }
    }

    // =========================================================================
    // Unit tests
    // =========================================================================

    #[cfg(test)]
    mod tests {
        use super::*;

        fn new_contract() -> MonitoringContract {
            MonitoringContract::new()
        }

        #[ink::test]
        fn constructor_sets_defaults() {
            let c = new_contract();
            assert_eq!(c.get_system_status(), HealthStatus::Healthy);
            assert!(!c.is_paused);
            assert_eq!(c.total_calls, 0);
            assert_eq!(c.total_errors, 0);
        }

        #[ink::test]
        fn record_operation_success_increments_counters() {
            let mut c = new_contract();
            c.record_operation(OperationType::RegisterProperty, true)
                .unwrap();
            let m = c.get_performance_metrics(OperationType::RegisterProperty);
            assert_eq!(m.total_calls, 1);
            assert_eq!(m.success_count, 1);
            assert_eq!(m.error_count, 0);
            assert_eq!(m.error_rate_bips, 0);
        }

        #[ink::test]
        fn record_operation_failure_increments_error_counters() {
            let mut c = new_contract();
            c.record_operation(OperationType::TransferProperty, false)
                .unwrap();
            let m = c.get_performance_metrics(OperationType::TransferProperty);
            assert_eq!(m.total_calls, 1);
            assert_eq!(m.error_count, 1);
            assert_eq!(m.error_rate_bips, 10_000); // 100%
        }

        #[ink::test]
        fn error_rate_bips_calculation() {
            let mut c = new_contract();
            // 1 success, 1 failure → 50%
            c.record_operation(OperationType::Generic, true).unwrap();
            c.record_operation(OperationType::Generic, false).unwrap();
            let m = c.get_performance_metrics(OperationType::Generic);
            assert_eq!(m.error_rate_bips, 5_000);
        }

        #[ink::test]
        fn get_all_metrics_returns_all_operations() {
            let c = new_contract();
            let all = c.get_all_metrics();
            assert_eq!(all.len(), 16);
        }

        #[ink::test]
        fn health_check_returns_healthy_on_no_errors() {
            let c = new_contract();
            let result = c.health_check();
            assert_eq!(result.status, HealthStatus::Healthy);
            assert!(result.is_accepting_calls);
            assert_eq!(result.overall_error_rate_bips, 0);
        }

        #[ink::test]
        fn health_check_reflects_high_error_rate() {
            let mut c = new_contract();
            // 3 errors out of 4 calls = 75% → Critical
            for _ in 0..3 {
                c.record_operation(OperationType::Generic, false).unwrap();
            }
            c.record_operation(OperationType::Generic, true).unwrap();
            let result = c.health_check();
            assert_eq!(result.status, HealthStatus::Critical);
        }

        #[ink::test]
        fn take_and_retrieve_snapshot() {
            let mut c = new_contract();
            c.record_operation(OperationType::Generic, true).unwrap();
            c.take_metrics_snapshot().unwrap();
            let snap = c.get_metrics_snapshot(0).expect("snapshot at slot 0");
            assert_eq!(snap.snapshot_id, 0);
            assert_eq!(snap.total_calls, 1);
            assert_eq!(snap.total_errors, 0);
        }

        #[ink::test]
        fn snapshot_circular_buffer_wraps() {
            let mut c = new_contract();
            for _ in 0..=constants::MONITORING_MAX_SNAPSHOTS {
                c.take_metrics_snapshot().unwrap();
            }
            // slot 0 should hold the last overwritten snapshot
            assert!(c.get_metrics_snapshot(0).is_some());
        }

        #[ink::test]
        fn pause_and_resume() {
            let mut c = new_contract();
            c.pause().unwrap();
            assert_eq!(c.get_system_status(), HealthStatus::Paused);
            assert!(c.record_operation(OperationType::Generic, true).is_err());
            c.resume().unwrap();
            assert_eq!(c.get_system_status(), HealthStatus::Healthy);
            assert!(c.record_operation(OperationType::Generic, true).is_ok());
        }

        #[ink::test]
        fn set_health_status_emits_event() {
            let mut c = new_contract();
            c.set_health_status(HealthStatus::Degraded).unwrap();
            assert_eq!(c.get_system_status(), HealthStatus::Degraded);
        }

        #[ink::test]
        fn alert_config_defaults_to_inactive() {
            let c = new_contract();
            let cfg = c.get_alert_config(AlertType::HighErrorRate);
            assert!(!cfg.is_active);
            assert_eq!(
                cfg.threshold_bips,
                constants::MONITORING_DEFAULT_ERROR_RATE_THRESHOLD_BIPS
            );
        }

        #[ink::test]
        fn set_alert_config_stores_values() {
            let mut c = new_contract();
            c.set_alert_config(AlertType::HighErrorRate, 500, true)
                .unwrap();
            let cfg = c.get_alert_config(AlertType::HighErrorRate);
            assert!(cfg.is_active);
            assert_eq!(cfg.threshold_bips, 500);
        }

        #[ink::test]
        fn set_alert_config_rejects_invalid_threshold() {
            let mut c = new_contract();
            assert!(c
                .set_alert_config(AlertType::HighErrorRate, 10_001, true)
                .is_err());
        }

        #[ink::test]
        fn subscribe_and_unsubscribe_alerts() {
            let mut c = new_contract();
            let sub = AccountId::from([0x02; 32]);
            c.subscribe_alerts(sub).unwrap();
            assert_eq!(c.get_alert_subscribers().len(), 1);
            c.unsubscribe_alerts(sub).unwrap();
            assert_eq!(c.get_alert_subscribers().len(), 0);
        }

        #[ink::test]
        fn unsubscribe_nonexistent_returns_error() {
            let mut c = new_contract();
            let sub = AccountId::from([0x03; 32]);
            assert!(c.unsubscribe_alerts(sub).is_err());
        }

        #[ink::test]
        fn add_and_remove_reporter() {
            let mut c = new_contract();
            let reporter = AccountId::from([0x04; 32]);
            assert!(!c.is_authorized_reporter(reporter));
            c.add_reporter(reporter).unwrap();
            assert!(c.is_authorized_reporter(reporter));
            c.remove_reporter(reporter).unwrap();
            assert!(!c.is_authorized_reporter(reporter));
        }

        #[ink::test]
        fn transfer_admin() {
            let mut c = new_contract();
            let new_admin = AccountId::from([0x05; 32]);
            c.transfer_admin(new_admin).unwrap();
            assert_eq!(c.get_admin(), new_admin);
        }
    }
}
