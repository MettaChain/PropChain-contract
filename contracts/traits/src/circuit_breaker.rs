use ink::prelude::vec::Vec;
use ink::storage::Mapping;
use ink::primitives::Hash;
use ink::traits::{SpreadLayout, PackedLayout};
use scale::{Encode, Decode};
#[cfg(feature = "std")]
use scale_info::TypeInfo;
use ink::storage::traits::{StorageLayout};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Encode, Decode, SpreadLayout, PackedLayout, StorageLayout)]
#[cfg_attr(feature = "std", derive(TypeInfo))]
pub enum ExternalDependency {
    Oracle,
    ComplianceRegistry,
    FeeManager,
    IdentityRegistry,
}

#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode, Default, SpreadLayout, PackedLayout, StorageLayout)]
#[cfg_attr(feature = "std", derive(TypeInfo))]
pub struct CircuitBreakerState {
    pub failure_count: u8,
    pub total_failures: u64,
    pub last_failure_at: Option<u64>,
    pub open_until: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode, SpreadLayout, PackedLayout, StorageLayout)]
#[cfg_attr(feature = "std", derive(TypeInfo))]
pub struct CircuitBreakerConfig {
    pub failure_threshold: u8,
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

