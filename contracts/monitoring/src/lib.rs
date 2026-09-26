#![allow(clippy::clone_on_copy)] // fires inside ink! generated storage code
#![cfg_attr(not(feature = "std"), no_std, no_main)]

/// Standalone quorum-guard helper tracking governance participation drops
/// across consecutive proposals.
///
/// The module is compiled and exported as part of this crate so its logic is
/// unit-tested and reusable by integrators; automatic recording hooks into
/// governance events are out of scope for this crate.
pub mod quorum_guard;

#[ink::contract]
pub mod monitoring {
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
        // Alert log (circular buffer, size = MONITORING_MAX_ALERT_LOG).
        // Mirrors AlertTriggered events so an off-chain delivery worker can
        // batch-read and retry alerts rather than tailing events.
        alert_log: Mapping<u64, AlertRecord>,
        /// Lifetime count of alerts appended, used as the ring-buffer cursor.
        alert_log_count: u64,
        // Metrics snapshots (circular buffer, size = MONITORING_MAX_SNAPSHOTS)
        snapshots: Mapping<u64, MetricsSnapshot>,
        snapshot_count: u64,
        // Registered health-check contracts
        health_check_contracts: Vec<AccountId>,
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

        /// Returns a self-contained delivery payload for one recorded alert.
        #[ink(message)]
        fn alert_payload(&self, alert_id: u64) -> Option<AlertPayload> {
            let record = self.retained_alert(alert_id)?;
            Some(build_alert_payload(&record))
        }

        /// Batches retained alerts, oldest first, starting after `since_alert_id`.
        #[ink(message)]
        fn get_recent_alerts(&self, since_alert_id: u64, limit: u32) -> Vec<AlertRecord> {
            let cap = constants::MONITORING_MAX_ALERT_LOG;

            // `limit` of 0 would silently return nothing and look like an empty
            // log; treat it as "everything retained" so a caller that passes an
            // unset limit still gets the batch it asked for.
            let limit = if limit == 0 {
                cap
            } else {
                core::cmp::min(limit as u64, cap)
            };

            // Oldest id still on chain. A cursor older than the window (a worker
            // that was offline) is clamped forward to the oldest retained entry
            // so it resumes without gaps rather than skipping alerts.
            let oldest = self.alert_log_count.saturating_sub(cap);

            // `since_alert_id` is the last id the caller already handled, so the
            // batch starts strictly after it. Alert ids begin at 0, which leaves
            // no value that means "delivered nothing yet"; 0 is used as that
            // sentinel and starts the batch at the oldest retained alert.
            let cursor = if since_alert_id == 0 {
                oldest
            } else {
                core::cmp::max(since_alert_id.saturating_add(1), oldest)
            };

            let end = core::cmp::min(self.alert_log_count, cursor.saturating_add(limit));

            let mut out = Vec::new();
            let mut id = cursor;
            while id < end {
                if let Some(record) = self.retained_alert(id) {
                    out.push(record);
                }
                id = id.saturating_add(1);
            }
            out
        }

        /// Marks an alert as delivered, clearing it from the retry set.
        #[ink(message)]
        fn acknowledge_alert(&mut self, alert_id: u64) -> Result<(), MonitoringError> {
            self.ensure_admin()?;

            let mut record = self
                .retained_alert(alert_id)
                .ok_or(MonitoringError::AlertNotFound)?;

            // Idempotent: a redelivery that races with a previous ack succeeds.
            if !record.acknowledged {
                record.acknowledged = true;
                let slot = alert_id % constants::MONITORING_MAX_ALERT_LOG;
                self.alert_log.insert(slot, &record);
            }
            Ok(())
        }

        /// Counts retained alerts that a delivery worker has not confirmed.
        #[ink(message)]
        fn pending_alert_count(&self) -> u32 {
            let cap = constants::MONITORING_MAX_ALERT_LOG;
            let oldest = self.alert_log_count.saturating_sub(cap);
            let mut pending = 0u32;
            let mut id = oldest;
            while id < self.alert_log_count {
                if let Some(record) = self.retained_alert(id) {
                    if !record.acknowledged {
                        pending = pending.saturating_add(1);
                    }
                }
                id = id.saturating_add(1);
            }
            pending
        }
    }

    // =========================================================================
    // Implementation — admin & configuration messages
    // =========================================================================

    impl MonitoringContract {
        /// Appends an alert to the bounded ring buffer, evicting the oldest
        /// entry once the buffer is full.
        ///
        /// Called alongside the `AlertTriggered` event so the event stream stays
        /// the authoritative record while the log gives a delivery worker
        /// something to batch-read and retry from.
        fn append_alert_record(
            &mut self,
            alert_type: AlertType,
            current_value: u32,
            threshold: u32,
            triggered_at: u64,
        ) {
            let slot = self.alert_log_count % constants::MONITORING_MAX_ALERT_LOG;
            self.alert_log.insert(
                slot,
                &AlertRecord {
                    alert_id: self.alert_log_count,
                    alert_type,
                    current_value,
                    threshold,
                    triggered_at,
                    acknowledged: false,
                },
            );
            self.alert_log_count = self.alert_log_count.saturating_add(1);
        }

        /// Looks up a retained alert by its id, or `None` if it was never
        /// recorded or has since been evicted from the ring buffer.
        fn retained_alert(&self, alert_id: u64) -> Option<AlertRecord> {
            let cap = constants::MONITORING_MAX_ALERT_LOG;
            let oldest = self.alert_log_count.saturating_sub(cap);
            if alert_id < oldest || alert_id >= self.alert_log_count {
                return None;
            }
            let record = self.alert_log.get(alert_id % cap)?;
            // The slot may have been recycled onto a newer alert by now; only
            // return it if it is still the alert that was asked for.
            if record.alert_id != alert_id {
                return None;
            }
            Some(record)
        }

        /// Deploys the monitoring contract. The caller becomes admin.
        #[ink(constructor)]
        #[allow(clippy::new_without_default)]
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
                alert_log: Mapping::default(),
                alert_log_count: 0,
                snapshots: Mapping::default(),
                snapshot_count: 0,
                health_check_contracts: Vec::new(),
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

        /// Register a contract for health-check aggregation. Admin only.
        #[ink(message)]
        pub fn register_health_contract(
            &mut self,
            contract: AccountId,
        ) -> Result<(), MonitoringError> {
            self.ensure_admin()?;
            if !self.health_check_contracts.contains(&contract) {
                self.health_check_contracts.push(contract);
            }
            Ok(())
        }

        /// Unregister a contract from health-check aggregation. Admin only.
        #[ink(message)]
        pub fn unregister_health_contract(
            &mut self,
            contract: AccountId,
        ) -> Result<(), MonitoringError> {
            self.ensure_admin()?;
            self.health_check_contracts.retain(|c| c != &contract);
            Ok(())
        }

        /// Get list of registered health-check contracts.
        #[ink(message)]
        pub fn get_health_contracts(&self) -> Vec<AccountId> {
            self.health_check_contracts.clone()
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
                        self.append_alert_record(
                            AlertType::HighErrorRate,
                            error_rate_bips,
                            threshold,
                            now,
                        );
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
                        self.append_alert_record(
                            AlertType::SystemDegraded,
                            error_rate_bips,
                            0,
                            now,
                        );
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

        // ─────────────────────────────────────────────────────────────────
        // Issue #1015: subscriber lifecycle & snapshot coverage
        // ─────────────────────────────────────────────────────────────────

        fn new_contract_with_admin(admin: AccountId) -> MonitoringContract {
            ink::env::test::set_caller::<ink::env::DefaultEnvironment>(admin);
            MonitoringContract::new()
        }

        #[ink::test]
        fn subscribe_alerts_registers_subscriber_once() {
            let accounts = ink::env::test::default_accounts::<ink::env::DefaultEnvironment>();
            let mut c = new_contract_with_admin(accounts.alice);

            ink::env::test::set_caller::<ink::env::DefaultEnvironment>(accounts.alice);
            c.subscribe_alerts(accounts.bob).expect("first subscribe");

            // Duplicate subscribe is a silent no-op, not an error
            c.subscribe_alerts(accounts.bob)
                .expect("duplicate subscribe must succeed silently");
            let subs = c.get_alert_subscribers();
            assert_eq!(subs.len(), 1, "duplicate subscribe must not add twice");
            assert_eq!(subs[0], accounts.bob);
        }

        #[ink::test]
        fn unsubscribe_unknown_subscriber_returns_error() {
            let accounts = ink::env::test::default_accounts::<ink::env::DefaultEnvironment>();
            let mut c = new_contract_with_admin(accounts.alice);

            ink::env::test::set_caller::<ink::env::DefaultEnvironment>(accounts.alice);
            let result = c.unsubscribe_alerts(accounts.django);
            assert_eq!(result, Err(MonitoringError::SubscriberNotFound));
        }

        #[ink::test]
        fn get_alert_subscribers_reflects_content_and_removal_order() {
            let accounts = ink::env::test::default_accounts::<ink::env::DefaultEnvironment>();
            let mut c = new_contract_with_admin(accounts.alice);

            ink::env::test::set_caller::<ink::env::DefaultEnvironment>(accounts.alice);
            c.subscribe_alerts(accounts.bob).unwrap();
            c.subscribe_alerts(accounts.charlie).unwrap();
            c.subscribe_alerts(accounts.django).unwrap();

            assert_eq!(
                c.get_alert_subscribers(),
                vec![accounts.bob, accounts.charlie, accounts.django],
                "subscribers must appear in registration order"
            );

            // Removing the middle entry swaps the last one into its place
            c.unsubscribe_alerts(accounts.charlie).unwrap();
            assert_eq!(
                c.get_alert_subscribers(),
                vec![accounts.bob, accounts.django],
                "swap_remove moves the last subscriber into the freed slot"
            );
        }

        #[ink::test]
        fn non_admin_cannot_manage_subscribers() {
            let accounts = ink::env::test::default_accounts::<ink::env::DefaultEnvironment>();
            let mut c = new_contract_with_admin(accounts.alice);

            ink::env::test::set_caller::<ink::env::DefaultEnvironment>(accounts.bob);
            assert_eq!(
                c.subscribe_alerts(accounts.charlie),
                Err(MonitoringError::Unauthorized)
            );
            assert_eq!(
                c.unsubscribe_alerts(accounts.charlie),
                Err(MonitoringError::Unauthorized)
            );
            assert!(c.get_alert_subscribers().is_empty());
        }

        #[ink::test]
        fn snapshot_round_trip_records_expected_aggregates_from_reporter() {
            let accounts = ink::env::test::default_accounts::<ink::env::DefaultEnvironment>();
            let mut c = new_contract_with_admin(accounts.alice);

            // Admin authorizes bob as a reporter
            ink::env::test::set_caller::<ink::env::DefaultEnvironment>(accounts.alice);
            c.add_reporter(accounts.bob).unwrap();
            assert!(c.is_authorized_reporter(accounts.bob));

            // Authorized reporter records operations: 3 successes + 1 failure
            ink::env::test::set_caller::<ink::env::DefaultEnvironment>(accounts.bob);
            c.record_operation(OperationType::RegisterProperty, true)
                .unwrap();
            c.record_operation(OperationType::TransferProperty, true)
                .unwrap();
            c.record_operation(OperationType::GovernanceVote, true)
                .unwrap();
            c.record_operation(OperationType::BridgeTransfer, false)
                .unwrap();

            // Snapshot lands at slot 0
            c.take_metrics_snapshot().unwrap();
            let snap = c
                .get_metrics_snapshot(0)
                .expect("first snapshot must be at slot 0");
            assert_eq!(snap.snapshot_id, 0);
            assert_eq!(snap.total_calls, 4);
            assert_eq!(snap.total_errors, 1);
            assert_eq!(snap.error_rate_bips, 2_500); // 25%
        }

        #[ink::test]
        fn consecutive_snapshots_use_distinct_slots_and_ids() {
            let accounts = ink::env::test::default_accounts::<ink::env::DefaultEnvironment>();
            let mut c = new_contract_with_admin(accounts.alice);

            ink::env::test::set_caller::<ink::env::DefaultEnvironment>(accounts.alice);
            c.record_operation(OperationType::Generic, true).unwrap();
            c.take_metrics_snapshot().unwrap();

            c.record_operation(OperationType::Stake, false).unwrap();
            c.take_metrics_snapshot().unwrap();

            let first = c.get_metrics_snapshot(0).expect("slot 0");
            let second = c
                .get_metrics_snapshot(1)
                .expect("second snapshot at slot 1");
            assert_eq!(first.snapshot_id, 0);
            assert_eq!(second.snapshot_id, 1);
            assert_eq!(second.total_calls, 2);
            assert_eq!(second.total_errors, 1);
            assert_eq!(second.error_rate_bips, 5_000); // 50%
        }

        #[ink::test]
        fn snapshot_buffer_wraps_and_overwrites_oldest_slot() {
            let accounts = ink::env::test::default_accounts::<ink::env::DefaultEnvironment>();
            let mut c = new_contract_with_admin(accounts.alice);

            let max = constants::MONITORING_MAX_SNAPSHOTS;
            let extra = 3u64;
            let total_snapshots = max + extra;

            ink::env::test::set_caller::<ink::env::DefaultEnvironment>(accounts.alice);
            for _ in 0..total_snapshots {
                c.take_metrics_snapshot().unwrap();
            }
            assert_eq!(c.snapshot_count, total_snapshots);

            // Slot reuse: slot 0 first held snapshot 0, now holds the newest
            // snapshot whose id maps onto slot 0 (id = max).
            let wrapped = c.get_metrics_snapshot(0).expect("slot 0 rewritten");
            assert_eq!(
                wrapped.snapshot_id, max,
                "oldest snapshot in slot 0 must be overwritten"
            );
            // Slot 1 now holds id max+1, slot 2 holds max+2
            assert_eq!(c.get_metrics_snapshot(1).unwrap().snapshot_id, max + 1);
            assert_eq!(c.get_metrics_snapshot(2).unwrap().snapshot_id, max + 2);

            // Slots untouched during the second pass keep their original data
            assert_eq!(c.get_metrics_snapshot(extra).unwrap().snapshot_id, extra);
        }

        #[ink::test]
        fn snapshot_beyond_written_count_returns_none() {
            let mut c = new_contract();
            assert!(c.get_metrics_snapshot(0).is_none(), "empty buffer");

            c.take_metrics_snapshot().unwrap();
            // Slot indices are bounded by MONITORING_MAX_SNAPSHOTS; anything
            // at or above it was never written.
            assert!(c
                .get_metrics_snapshot(constants::MONITORING_MAX_SNAPSHOTS)
                .is_none());
        }

        // =====================================================================
        // Alert delivery log (issue #1197)
        // =====================================================================

        /// Moves the test chain clock forward, which the alert cooldown reads.
        fn set_time(timestamp: u64) {
            ink::env::test::set_block_timestamp::<ink::env::DefaultEnvironment>(timestamp);
        }

        /// Arms the HighErrorRate alert and records one failing operation, which
        /// drives the error rate to 100% and fires the alert.
        ///
        /// Moves the clock past the cooldown first: the cooldown is measured
        /// against a last-triggered default of 0, so an alert evaluated at
        /// timestamp 0 would always be suppressed.
        fn fire_high_error_alert(c: &mut MonitoringContract) {
            c.set_alert_config(AlertType::HighErrorRate, 500, true)
                .unwrap();
            set_time(1_000_000);
            c.record_operation(OperationType::Generic, false).unwrap();
        }

        /// Fires `count` alerts, stepping past the cooldown between each so the
        /// cooldown does not suppress them.
        fn fire_alerts(c: &mut MonitoringContract, count: u64) {
            c.set_alert_config(AlertType::HighErrorRate, 500, true)
                .unwrap();
            let mut t = 1_000_000;
            for _ in 0..count {
                set_time(t);
                c.record_operation(OperationType::Generic, false).unwrap();
                t = t.saturating_add(constants::MONITORING_ALERT_COOLDOWN_MS + 1);
            }
        }

        #[ink::test]
        fn no_alerts_are_recorded_before_one_fires() {
            let c = new_contract();
            assert!(c.get_recent_alerts(0, 10).is_empty());
            assert_eq!(c.pending_alert_count(), 0);
            assert!(c.alert_payload(0).is_none());
        }

        #[ink::test]
        fn alert_payload_is_none_for_unknown_id() {
            let c = new_contract();
            assert!(c.alert_payload(0).is_none());
            assert!(c.alert_payload(999).is_none());
        }

        #[ink::test]
        fn firing_an_alert_records_it_in_the_log() {
            let mut c = new_contract();
            fire_high_error_alert(&mut c);

            let records = c.get_recent_alerts(0, 10);
            assert_eq!(records.len(), 1);
            assert_eq!(records[0].alert_id, 0);
            assert_eq!(records[0].alert_type, AlertType::HighErrorRate);
            assert_eq!(records[0].current_value, 10_000);
            assert_eq!(records[0].threshold, 500);
            assert!(!records[0].acknowledged, "new alerts start unacknowledged");
        }

        #[ink::test]
        fn alert_payload_is_self_contained() {
            let mut c = new_contract();
            set_time(1_000_000);
            fire_high_error_alert(&mut c);

            let payload = c.alert_payload(0).expect("alert 0 retained");
            assert_eq!(payload.alert_id, 0);
            assert_eq!(payload.alert_type, AlertType::HighErrorRate);
            assert_eq!(payload.current_value, 10_000);
            assert_eq!(payload.threshold, 500);
            assert_eq!(payload.triggered_at, 1_000_000);
            assert!(!payload.acknowledged);
            // Named type and severity so a consumer needs no SCALE decoding.
            assert_eq!(payload.alert_type_name, "HighErrorRate");
            assert_eq!(payload.severity, 1);
        }

        #[ink::test]
        fn alert_payload_json_carries_every_field() {
            let mut c = new_contract();
            set_time(1_000_000);
            fire_high_error_alert(&mut c);

            let json = c.alert_payload(0).unwrap().json;
            assert_eq!(
                json,
                concat!(
                    r#"{"alertId":0,"alertType":"HighErrorRate","severity":1,"#,
                    r#""currentValue":10000,"threshold":500,"triggeredAt":1000000,"#,
                    r#""acknowledged":false}"#
                )
            );
        }

        #[ink::test]
        fn system_degraded_alert_is_recorded_with_its_own_severity() {
            let mut c = new_contract();
            c.set_alert_config(AlertType::SystemDegraded, 0, true)
                .unwrap();
            set_time(1_000_000);
            c.record_operation(OperationType::Generic, false).unwrap();

            let records = c.get_recent_alerts(0, 10);
            assert_eq!(records.len(), 1);
            assert_eq!(records[0].alert_type, AlertType::SystemDegraded);

            let payload = c.alert_payload(0).unwrap();
            assert_eq!(payload.alert_type_name, "SystemDegraded");
            // Higher severity than HighErrorRate, so a batch can be triaged.
            assert_eq!(payload.severity, 2);
        }

        #[ink::test]
        fn get_recent_alerts_batches_oldest_first() {
            let mut c = new_contract();
            fire_alerts(&mut c, 5);

            let all = c.get_recent_alerts(0, 100);
            assert_eq!(all.len(), 5);
            for (index, record) in all.iter().enumerate() {
                assert_eq!(record.alert_id, index as u64, "oldest first");
            }
        }

        #[ink::test]
        fn get_recent_alerts_resumes_from_cursor() {
            let mut c = new_contract();
            fire_alerts(&mut c, 5);

            // Cursor is exclusive: everything strictly after alert 2.
            let after_two = c.get_recent_alerts(2, 100);
            assert_eq!(after_two.len(), 2);
            assert_eq!(after_two[0].alert_id, 3);
            assert_eq!(after_two[1].alert_id, 4);
        }

        #[ink::test]
        fn get_recent_alerts_honours_limit() {
            let mut c = new_contract();
            fire_alerts(&mut c, 5);

            assert_eq!(c.get_recent_alerts(0, 2).len(), 2);
        }

        #[ink::test]
        fn get_recent_alerts_clamps_limit_to_the_window() {
            let mut c = new_contract();
            fire_alerts(&mut c, 3);

            // A limit above the retained window cannot exceed what is retained.
            let huge = c.get_recent_alerts(0, u32::MAX);
            assert_eq!(huge.len(), 3);
        }

        #[ink::test]
        fn get_recent_alerts_treats_zero_limit_as_everything() {
            let mut c = new_contract();
            fire_alerts(&mut c, 3);
            // An unset limit must not look like an empty log.
            assert_eq!(c.get_recent_alerts(0, 0).len(), 3);
        }

        #[ink::test]
        fn cursor_older_than_the_window_resumes_without_gaps() {
            let mut c = new_contract();
            let cap = constants::MONITORING_MAX_ALERT_LOG;
            fire_alerts(&mut c, cap + 10);

            // A worker that was offline holds a stale cursor. It must start at
            // the oldest retained alert, not silently return nothing.
            let batch = c.get_recent_alerts(0, 5);
            assert_eq!(batch.len(), 5);
            assert_eq!(
                batch[0].alert_id, 10,
                "resumes at the oldest retained alert"
            );
        }

        #[ink::test]
        fn acknowledge_alert_marks_it_delivered() {
            let mut c = new_contract();
            fire_high_error_alert(&mut c);

            c.acknowledge_alert(0).unwrap();
            assert!(c.alert_payload(0).unwrap().acknowledged);
            assert!(c.get_recent_alerts(0, 10)[0].acknowledged);
        }

        #[ink::test]
        fn acknowledge_alert_is_idempotent() {
            let mut c = new_contract();
            fire_high_error_alert(&mut c);

            // A redelivery that races a previous ack must not error.
            c.acknowledge_alert(0).unwrap();
            c.acknowledge_alert(0).unwrap();
            assert!(c.alert_payload(0).unwrap().acknowledged);
        }

        #[ink::test]
        fn acknowledge_alert_rejects_unknown_id() {
            let mut c = new_contract();
            assert_eq!(c.acknowledge_alert(42), Err(MonitoringError::AlertNotFound));
        }

        #[ink::test]
        fn pending_alert_count_tracks_the_retry_set() {
            let mut c = new_contract();
            fire_alerts(&mut c, 3);
            assert_eq!(c.pending_alert_count(), 3);

            c.acknowledge_alert(0).unwrap();
            assert_eq!(c.pending_alert_count(), 2);
            c.acknowledge_alert(2).unwrap();
            assert_eq!(c.pending_alert_count(), 1);
        }

        #[ink::test]
        fn alert_log_evicts_oldest_beyond_the_cap() {
            let mut c = new_contract();
            let cap = constants::MONITORING_MAX_ALERT_LOG;
            fire_alerts(&mut c, cap + 5);

            // The five oldest are gone and report absence rather than stale data.
            for id in 0..5u64 {
                assert!(
                    c.alert_payload(id).is_none(),
                    "alert {id} should have been evicted"
                );
            }
            // The newest are retained and still readable.
            assert!(c.alert_payload(cap + 4).is_some());
            assert_eq!(c.alert_payload(cap + 4).unwrap().alert_id, cap + 4);
        }

        #[ink::test]
        fn evicted_slots_do_not_leak_older_alerts() {
            let mut c = new_contract();
            let cap = constants::MONITORING_MAX_ALERT_LOG;
            fire_alerts(&mut c, cap + 1);

            // Slot 0 now holds alert `cap`, not alert 0. Asking for alert 0 must
            // not return the newer record that recycled its slot.
            assert!(c.alert_payload(0).is_none());
            assert_eq!(c.alert_payload(cap).unwrap().alert_id, cap);
        }

        #[ink::test]
        fn pending_count_stays_bounded_after_eviction() {
            let mut c = new_contract();
            let cap = constants::MONITORING_MAX_ALERT_LOG;
            fire_alerts(&mut c, cap + 20);

            // Acknowledged alerts that are later evicted stop counting, so the
            // retry set can never exceed the retained window.
            assert_eq!(c.pending_alert_count(), cap as u32);
        }

        #[ink::test]
        fn batch_read_after_eviction_returns_only_retained_alerts() {
            let mut c = new_contract();
            let cap = constants::MONITORING_MAX_ALERT_LOG;
            fire_alerts(&mut c, cap + 5);

            let batch = c.get_recent_alerts(0, u32::MAX);
            assert_eq!(batch.len(), cap as usize);
            assert_eq!(batch[0].alert_id, 5);
            assert_eq!(batch[batch.len() - 1].alert_id, cap + 4);
        }
    }
}
