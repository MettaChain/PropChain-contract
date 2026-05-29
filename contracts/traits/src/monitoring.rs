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

/// Errors that can be returned by the monitoring contract.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Encode, Decode)]
#[cfg_attr(feature = "std", derive(TypeInfo))]
pub enum MonitoringError {
    Unauthorized,
    ContractPaused,
    InvalidThreshold,
    SubscriberLimitReached,
    SubscriberNotFound,
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
}
