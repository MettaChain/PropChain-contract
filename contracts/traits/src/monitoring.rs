use core::fmt;

use ink::prelude::vec::Vec;
use scale::{Decode, Encode};
#[cfg(feature = "std")]
use scale_info::TypeInfo;

use crate::errors::{monitoring_codes, ContractError, ErrorCategory};

/// Classifies which contract operation is being recorded.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Encode, Decode, ink::storage::traits::StorageLayout,
)]
#[cfg_attr(feature = "std", derive(TypeInfo))]
pub enum OperationType {
    RegisterProperty,
    TransferProperty,
    UpdateMetadata,
    CreateEscrow,
    ReleaseEscrow,
    RefundEscrow,
    MintToken,
    BurnToken,
    BridgeTransfer,
    Stake,
    Unstake,
    GovernanceVote,
    OracleUpdate,
    ComplianceCheck,
    FeeCollection,
    Generic,
}

/// Overall health of the monitored system.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Encode, Decode, ink::storage::traits::StorageLayout,
)]
#[cfg_attr(feature = "std", derive(TypeInfo))]
pub enum HealthStatus {
    Healthy,
    Degraded,
    Critical,
    Paused,
}

/// Category of alert condition.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Encode, Decode, ink::storage::traits::StorageLayout,
)]
#[cfg_attr(feature = "std", derive(TypeInfo))]
pub enum AlertType {
    /// Fires when the overall error rate (in bips) exceeds the configured threshold.
    HighErrorRate,
    /// Fires when the computed health status is Degraded or Critical.
    SystemDegraded,
}

/// Per-operation performance snapshot returned to callers.
#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode)]
#[cfg_attr(feature = "std", derive(TypeInfo, ink::storage::traits::StorageLayout))]
pub struct PerformanceMetrics {
    pub operation: OperationType,
    pub total_calls: u64,
    pub success_count: u64,
    pub error_count: u64,
    /// Error rate expressed in basis points (10 000 = 100 %).
    pub error_rate_bips: u32,
    pub last_called_at: u64,
    pub last_error_at: u64,
}

/// Point-in-time aggregate metrics stored in the circular snapshot buffer.
#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode)]
#[cfg_attr(feature = "std", derive(TypeInfo, ink::storage::traits::StorageLayout))]
pub struct MetricsSnapshot {
    pub snapshot_id: u64,
    pub timestamp: u64,
    pub total_calls: u64,
    pub total_errors: u64,
    pub error_rate_bips: u32,
}

/// Result returned by the health-check endpoint.
#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode)]
#[cfg_attr(feature = "std", derive(TypeInfo, ink::storage::traits::StorageLayout))]
pub struct HealthCheckResult {
    pub status: HealthStatus,
    pub checked_at: u64,
    pub total_operations: u64,
    pub overall_error_rate_bips: u32,
    pub uptime_blocks: u64,
    pub is_accepting_calls: bool,
}

/// Current configuration for a single alert type.
#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode)]
#[cfg_attr(feature = "std", derive(TypeInfo, ink::storage::traits::StorageLayout))]
pub struct AlertConfig {
    pub alert_type: AlertType,
    pub threshold_bips: u32,
    pub is_active: bool,
    pub last_triggered_at: u64,
}

/// Stable, machine-readable name for an alert type.
///
/// Used to render self-contained delivery payloads without requiring the
/// consumer to decode the SCALE enum discriminant.
pub const fn alert_type_name(alert_type: &AlertType) -> &'static str {
    match alert_type {
        AlertType::HighErrorRate => "HighErrorRate",
        AlertType::SystemDegraded => "SystemDegraded",
    }
}

/// Severity ranking for an alert type, used by delivery consumers to triage.
///
/// Higher is more urgent. The on-chain record remains authoritative; this only
/// orders a batch for the operator.
pub const fn alert_type_severity(alert_type: &AlertType) -> u8 {
    match alert_type {
        AlertType::HighErrorRate => 1,
        AlertType::SystemDegraded => 2,
    }
}

/// A single alert as recorded on chain.
///
/// Alerts are appended to a bounded ring buffer (see
/// [`MONITORING_MAX_ALERT_LOG`](crate::constants::MONITORING_MAX_ALERT_LOG)) so
/// that an off-chain delivery worker can read recent alerts without tailing
/// every `AlertTriggered` event, and so a critical alert that could not be
/// delivered is still readable after the fact.
#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode)]
#[cfg_attr(feature = "std", derive(TypeInfo, ink::storage::traits::StorageLayout))]
pub struct AlertRecord {
    /// Monotonic identifier, also the ring-buffer slot.
    pub alert_id: u64,
    pub alert_type: AlertType,
    /// Observed value that breached the threshold, in bips.
    pub current_value: u32,
    /// Configured threshold that was breached, in bips.
    pub threshold: u32,
    /// Ledger timestamp at which the alert fired.
    pub triggered_at: u64,
    /// Set once a delivery worker confirms receipt. Drives retry: an alert left
    /// unacknowledged is one the worker has not yet confirmed.
    pub acknowledged: bool,
}

/// Self-contained alert blob intended for out-of-band delivery.
///
/// Contains everything a webhook/indexer needs in a single read: the structured
/// fields plus a canonical JSON rendering. The consumer never has to make
/// follow-up contract calls to interpret the alert, which is what lets
/// delivery be retried reliably.
#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode)]
#[cfg_attr(feature = "std", derive(TypeInfo, ink::storage::traits::StorageLayout))]
pub struct AlertPayload {
    pub alert_id: u64,
    pub alert_type: AlertType,
    pub current_value: u32,
    pub threshold: u32,
    pub triggered_at: u64,
    pub acknowledged: bool,
    /// Stable name of `alert_type`, so consumers need not decode the enum.
    pub alert_type_name: ink::prelude::string::String,
    /// Severity ranking; higher is more urgent.
    pub severity: u8,
    /// Canonical JSON object with all of the above, safe to POST as a webhook
    /// body. Field names are stable and the alert id is the delivery key, so a
    /// redelivery is idempotent on the consumer side.
    pub json: ink::prelude::string::String,
}

/// Renders an [`AlertRecord`] into its self-contained [`AlertPayload`].
///
/// The JSON is assembled from a closed set of type names and integers, so no
/// escaping is required: the only strings in the object are the fixed
/// `alertType` value and the fixed object keys. `alertId` is the delivery key,
/// which lets a consumer drop a redelivered alert idempotently.
pub fn build_alert_payload(record: &AlertRecord) -> AlertPayload {
    use ink::prelude::format;
    use ink::prelude::string::String;

    let name = alert_type_name(&record.alert_type);
    let severity = alert_type_severity(&record.alert_type);

    let json = format!(
        concat!(
            "{{\"alertId\":{},\"alertType\":\"{}\",\"severity\":{},",
            "\"currentValue\":{},\"threshold\":{},\"triggeredAt\":{},",
            "\"acknowledged\":{}}}"
        ),
        record.alert_id,
        name,
        severity,
        record.current_value,
        record.threshold,
        record.triggered_at,
        record.acknowledged,
    );

    AlertPayload {
        alert_id: record.alert_id,
        alert_type: record.alert_type,
        current_value: record.current_value,
        threshold: record.threshold,
        triggered_at: record.triggered_at,
        acknowledged: record.acknowledged,
        alert_type_name: String::from(name),
        severity,
        json,
    }
}

/// Errors that can be returned by the monitoring contract.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Encode, Decode)]
#[cfg_attr(feature = "std", derive(TypeInfo))]
pub enum MonitoringError {
    Unauthorized,
    ContractPaused,
    InvalidThreshold,
    SubscriberLimitReached,
    SubscriberNotFound,
    HealthCheckFailed,
    /// The requested alert id was never recorded or has been evicted from the
    /// bounded alert log.
    AlertNotFound,
}

impl fmt::Display for MonitoringError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MonitoringError::Unauthorized => write!(f, "Caller is not authorized"),
            MonitoringError::ContractPaused => write!(f, "Monitoring contract is paused"),
            MonitoringError::InvalidThreshold => write!(f, "Alert threshold value is invalid"),
            MonitoringError::SubscriberLimitReached => {
                write!(f, "Maximum subscriber limit reached")
            }
            MonitoringError::SubscriberNotFound => write!(f, "Subscriber not found"),
            MonitoringError::HealthCheckFailed => write!(f, "Health check endpoint failed"),
            MonitoringError::AlertNotFound => {
                write!(f, "Alert not found in the retained alert log")
            }
        }
    }
}

impl ContractError for MonitoringError {
    fn error_code(&self) -> u32 {
        match self {
            MonitoringError::Unauthorized => monitoring_codes::MONITORING_UNAUTHORIZED,
            MonitoringError::ContractPaused => monitoring_codes::MONITORING_CONTRACT_PAUSED,
            MonitoringError::InvalidThreshold => monitoring_codes::MONITORING_INVALID_THRESHOLD,
            MonitoringError::SubscriberLimitReached => {
                monitoring_codes::MONITORING_SUBSCRIBER_LIMIT_REACHED
            }
            MonitoringError::SubscriberNotFound => {
                monitoring_codes::MONITORING_SUBSCRIBER_NOT_FOUND
            }
            MonitoringError::HealthCheckFailed => monitoring_codes::MONITORING_HEALTH_CHECK_FAILED,
            MonitoringError::AlertNotFound => monitoring_codes::MONITORING_ALERT_NOT_FOUND,
        }
    }

    fn error_description(&self) -> &'static str {
        match self {
            MonitoringError::Unauthorized => "Caller does not have monitoring permissions",
            MonitoringError::ContractPaused => "Monitoring contract is currently paused",
            MonitoringError::InvalidThreshold => {
                "Threshold value must be between 0 and 10 000 bips"
            }
            MonitoringError::SubscriberLimitReached => {
                "Cannot add more subscribers, maximum limit reached"
            }
            MonitoringError::SubscriberNotFound => "The subscriber account is not registered",
            MonitoringError::HealthCheckFailed => "Failed to retrieve health status from contract",
            MonitoringError::AlertNotFound => {
                "Alert id is unknown or has been evicted from the alert log"
            }
        }
    }

    fn error_category(&self) -> ErrorCategory {
        ErrorCategory::Monitoring
    }

    fn error_i18n_key(&self) -> &'static str {
        match self {
            MonitoringError::Unauthorized => "monitoring.unauthorized",
            MonitoringError::ContractPaused => "monitoring.contract_paused",
            MonitoringError::InvalidThreshold => "monitoring.invalid_threshold",
            MonitoringError::SubscriberLimitReached => "monitoring.subscriber_limit_reached",
            MonitoringError::SubscriberNotFound => "monitoring.subscriber_not_found",
            MonitoringError::HealthCheckFailed => "monitoring.health_check_failed",
            MonitoringError::AlertNotFound => "monitoring.alert_not_found",
        }
    }
}

/// Cross-contract interface for the monitoring system.
#[ink::trait_definition]
pub trait MonitoringSystem {
    /// Record a single operation outcome. Callable by admin or authorized reporters.
    #[ink(message)]
    fn record_operation(
        &mut self,
        operation: OperationType,
        success: bool,
    ) -> Result<(), MonitoringError>;

    /// Return accumulated metrics for a specific operation type.
    #[ink(message)]
    fn get_performance_metrics(&self, operation: OperationType) -> PerformanceMetrics;

    /// Return metrics for all known operation types.
    #[ink(message)]
    fn get_all_metrics(&self) -> Vec<PerformanceMetrics>;

    /// Compute and return a live health-check result based on current metrics.
    #[ink(message)]
    fn health_check(&self) -> HealthCheckResult;

    /// Return the currently stored health status (admin-controlled).
    #[ink(message)]
    fn get_system_status(&self) -> HealthStatus;

    /// Persist a point-in-time snapshot of aggregate metrics (circular buffer).
    #[ink(message)]
    fn take_metrics_snapshot(&mut self) -> Result<(), MonitoringError>;

    /// Retrieve a previously stored snapshot by its buffer slot index.
    #[ink(message)]
    fn get_metrics_snapshot(&self, slot: u64) -> Option<MetricsSnapshot>;

    /// Retrieve a self-contained delivery payload for one recorded alert.
    ///
    /// Returns `None` if `alert_id` has been evicted from the bounded alert log
    /// or was never recorded.
    #[ink(message)]
    fn alert_payload(&self, alert_id: u64) -> Option<AlertPayload>;

    /// Batch recent alerts, oldest first, starting after `since_alert_id`.
    ///
    /// `limit` is clamped to [`MONITORING_MAX_ALERT_LOG`](crate::constants::MONITORING_MAX_ALERT_LOG)
    /// so a single call cannot exceed the retained window. Pass
    /// `since_alert_id = 0` to start from the oldest retained alert; the
    /// contract also accepts a value older than the window, which is clamped
    /// to the oldest retained entry, so a worker that fell behind resumes
    /// without gaps rather than silently skipping alerts.
    #[ink(message)]
    fn get_recent_alerts(&self, since_alert_id: u64, limit: u32) -> Vec<AlertRecord>;

    /// Mark an alert as delivered, clearing it from the retry set.
    ///
    /// Admin only. Idempotent: acknowledging an already-acknowledged alert
    /// succeeds without changing state. Returns an error for an unknown id.
    #[ink(message)]
    fn acknowledge_alert(&mut self, alert_id: u64) -> Result<(), MonitoringError>;

    /// Number of retained alerts that have not been acknowledged.
    ///
    /// A non-zero value that is not falling means delivery is stuck; this is
    /// the signal a worker or an operator monitor should alert on.
    #[ink(message)]
    fn pending_alert_count(&self) -> u32;
}

/// On-chain health report from a contract.
#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode)]
#[cfg_attr(feature = "std", derive(TypeInfo, ink::storage::traits::StorageLayout))]
pub struct HealthReport {
    /// The name or identifier of the contract reporting health
    pub contract_name: ink::prelude::string::String,
    /// Overall health status of the contract
    pub status: HealthStatus,
    /// Timestamp when this report was generated
    pub reported_at: u64,
    /// Number of operations processed by the contract
    pub total_operations: u64,
    /// Number of operations that resulted in errors
    pub error_count: u64,
    /// Error rate in basis points (10_000 = 100%)
    pub error_rate_bips: u32,
    /// Whether the contract is accepting new calls
    pub is_accepting_calls: bool,
}

/// Trait for contracts to expose health-check endpoints.
#[ink::trait_definition]
pub trait HealthEndpoint {
    /// Return the current health status of this contract.
    #[ink(message)]
    fn health(&self) -> HealthReport;
}
