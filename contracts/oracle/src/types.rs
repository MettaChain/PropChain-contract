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

// ── Oracle Governance Types (Issue #228) ──────────────────────────────────────

/// Actions that can be governed by oracle governance proposals.
#[derive(Debug, Clone, PartialEq, Eq, scale::Encode, scale::Decode)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub enum GovernanceAction {
    /// Update the minimum stake required for oracle sources
    UpdateMinStake(u128),
    /// Update the minimum number of sources required for valuation
    UpdateMinSources(u32),
    /// Update the maximum staleness for price feeds
    UpdateMaxStaleness(u64),
    /// Update the volatility threshold for circuit breaker
    UpdateVolatilityThreshold(u32),
    /// Add a new oracle source type
    AddSourceType(String, u32),
    /// Remove an oracle source
    RemoveSource(String),
}

/// Governance-controlled oracle parameters.
#[derive(Debug, Clone, PartialEq, Eq, scale::Encode, scale::Decode)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub struct GovernanceParams {
    pub min_oracle_stake: u128,
    pub max_sources: u32,
    pub governance_quorum_bps: u32,
    pub governance_voting_period_blocks: u64,
}

impl Default for GovernanceParams {
    fn default() -> Self {
        Self {
            min_oracle_stake: 1_000_000,
            max_sources: 20,
            governance_quorum_bps: 5000, // 50%
            governance_voting_period_blocks: 28_800, // ~2 days at 6s blocks
        }
    }
}

/// A governance proposal for changing oracle parameters.
#[derive(Debug, Clone, PartialEq, Eq, scale::Encode, scale::Decode)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub struct GovernanceProposal {
    pub id: u64,
    pub proposer: AccountId,
    pub action: GovernanceAction,
    pub votes_for: u128,
    pub votes_against: u128,
    pub voting_end: u64,
    pub executed: bool,
    pub created_at: u64,
}
