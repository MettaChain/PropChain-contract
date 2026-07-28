// SPDX-License-Identifier: MIT

/// Identifies an external dependency that the circuit breaker monitors.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Encode, Decode, SpreadLayout, PackedLayout, StorageLayout)]
#[cfg_attr(feature = "std", derive(TypeInfo))]
pub enum ExternalDependency {
    Oracle,
    ComplianceRegistry,
    FeeManager,
    IdentityRegistry,
}

/// Tracks failure history and open/closed state of the circuit breaker for a dependency.
#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode, Default, SpreadLayout, PackedLayout, StorageLayout)]
#[cfg_attr(feature = "std", derive(TypeInfo))]
pub struct CircuitBreakerState {
    /// Consecutive failure count since last success.
    pub failure_count: u8,
    /// Total cumulative failures recorded.
    pub total_failures: u64,
    /// Timestamp of the most recent failure.
    pub last_failure_at: Option<u64>,
    /// If set, the breaker is open until this timestamp.
    pub open_until: Option<u64>,
}

/// Configuration parameters for circuit breaker behaviour.
#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode, SpreadLayout, PackedLayout, StorageLayout)]
#[cfg_attr(feature = "std", derive(TypeInfo))]
pub struct CircuitBreakerConfig {
    /// Number of consecutive failures before the breaker opens.
    pub failure_threshold: u8,
    /// Cooldown period in seconds before the breaker auto-closes.
    pub cooldown_period_secs: u64,
}

impl Default for CircuitBreakerConfig {
    fn default() -> Self {
        Self {
            failure_threshold: 3,
            cooldown_period_secs: 300, // 5 minutes
        }
    }
}

