// Local types for the oracle contract (Issue #101 - extracted from lib.rs)

/// Result of an oracle batch operation
#[derive(Debug, Clone, PartialEq, Eq, scale::Encode, scale::Decode)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub struct OracleBatchResult {
    pub successes: Vec<u64>,
    pub failures: Vec<OracleBatchItemFailure>,
    pub total_items: u32,
    pub successful_items: u32,
    pub failed_items: u32,
    pub early_terminated: bool,
}

/// A single item failure in an oracle batch operation
#[derive(Debug, Clone, PartialEq, Eq, scale::Encode, scale::Decode)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub struct OracleBatchItemFailure {
    pub index: u32,
    pub item_id: u64,
    pub error: OracleError,
}
