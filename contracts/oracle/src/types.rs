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
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout))]
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
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout))]
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
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout))]
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


/// Direction of property price trend.
#[derive(Debug, Clone, PartialEq, Eq, scale::Encode, scale::Decode)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout))]
pub enum TrendDirection {
    Up,
    Down,
    Stable,
}

/// Property trend metrics for valuation analysis.
#[derive(Debug, Clone, PartialEq, scale::Encode, scale::Decode)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout))]
pub struct TrendMetrics {
    pub current_price: u128,
    pub ema_7d: u128,
    pub sma_7d: u128,
    pub sma_30d: u128,
    pub trend_direction: TrendDirection,
}


// ── Aggregation Method (existing infrastructure) ──────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, scale::Encode, scale::Decode)]
#[cfg_attr(
    feature = "std",
    derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
)]
pub enum AggregationMethod {
    WeightedMean,
    Median,
    TrimmedMean(u32),
}

impl Default for AggregationMethod {
    fn default() -> Self {
        AggregationMethod::WeightedMean
    }
}

// ── Slashing Infrastructure (existing infrastructure) ─────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, scale::Encode, scale::Decode)]
#[cfg_attr(
    feature = "std",
    derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
)]
pub enum SlashingSeverity {
    Minor,
    Moderate,
    Severe,
    Critical,
}

#[derive(Debug, Clone, PartialEq, scale::Encode, scale::Decode)]
#[cfg_attr(
    feature = "std",
    derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
)]
pub struct SlashingRecord {
    pub block: u32,
    pub severity: SlashingSeverity,
    pub amount_slashed: u128,
    pub reason: String,
    pub banned: bool,
}

#[derive(Debug, Clone, PartialEq, scale::Encode, scale::Decode)]
#[cfg_attr(
    feature = "std",
    derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
)]
pub struct SlashingConfig {
    pub minor_slash_bps: u32,
    pub moderate_slash_bps: u32,
    pub severe_slash_bps: u32,
    pub critical_slash_bps: u32,
    pub minor_reputation_penalty: u32,
    pub moderate_reputation_penalty: u32,
    pub severe_reputation_penalty: u32,
    pub critical_reputation_penalty: u32,
    pub suspension_threshold: u32,
    pub ban_duration_blocks: u32,
}

impl Default for SlashingConfig {
    fn default() -> Self {
        Self {
            minor_slash_bps: 500,       // 5%
            moderate_slash_bps: 1500,   // 15%
            severe_slash_bps: 3000,     // 30%
            critical_slash_bps: 5000,   // 50%
            minor_reputation_penalty: 50,
            moderate_reputation_penalty: 150,
            severe_reputation_penalty: 300,
            critical_reputation_penalty: 500,
            suspension_threshold: 3,
            ban_duration_blocks: 43_200, // ~3 days at 6s blocks
        }
    }
}

#[derive(Debug, Clone, PartialEq, scale::Encode, scale::Decode)]
#[cfg_attr(
    feature = "std",
    derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
)]
pub struct SourceStatus {
    pub reputation: u32,
    pub stake: u128,
    pub is_active: bool,
    pub is_banned: bool,
    pub ban_expires_at: u32,
    pub total_slashes: u32,
    pub total_amount_slashed: u128,
}

#[derive(Debug, Clone, PartialEq, scale::Encode, scale::Decode)]
#[cfg_attr(
    feature = "std",
    derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
)]
pub struct SlashingSummary {
    pub recent_slashes: Vec<SlashingRecord>,
    pub total_slashes: u32,
    pub total_amount_slashed: u128,
}

// ── Fallback Mechanism (existing infrastructure) ──────────────────────────────

#[derive(Debug, Clone, PartialEq, scale::Encode, scale::Decode)]
#[cfg_attr(
    feature = "std",
    derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
)]
pub struct FallbackConfig {
    pub enabled: bool,
    pub fallback_delay_blocks: u64,
    pub max_fallback_attempts: u32,
}

impl Default for FallbackConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            fallback_delay_blocks: 3,
            max_fallback_attempts: 3,
        }
    }
}

#[derive(Debug, Clone, PartialEq, scale::Encode, scale::Decode)]
#[cfg_attr(
    feature = "std",
    derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
)]
pub struct FallbackSource {
    pub id: String,
    pub priority: u32,
    pub active: bool,
    pub success_count: u32,
    pub failure_count: u32,
}

#[derive(Debug, Clone, PartialEq, scale::Encode, scale::Decode)]
#[cfg_attr(
    feature = "std",
    derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
)]
pub struct FallbackQueryResult {
    pub success: bool,
    pub price: u128,
    pub source_id: String,
    pub attempts: u32,
    pub timestamp: u64,
    pub error: String,
}

// ── Oracle Source Management Proposals (Issue #495) ───────────────────────────

/// The action type for a pending oracle source management proposal.
#[derive(Debug, Clone, PartialEq, Eq, scale::Encode, scale::Decode)]
#[cfg_attr(
    feature = "std",
    derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
)]
pub enum OracleSourceAction {
    /// Propose adding a new oracle source
    AddSource,
    /// Propose removing an existing oracle source by ID
    RemoveSource,
}

/// A multi-sig proposal for adding or removing an oracle source.
#[derive(Debug, Clone, PartialEq, scale::Encode, scale::Decode)]
#[cfg_attr(
    feature = "std",
    derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
)]
pub struct OracleSourceProposal {
    pub proposal_id: u64,
    pub action: OracleSourceAction,
    /// For AddSource: the source to add. For RemoveSource: only `id` is used.
    pub source: OracleSource,
    /// Accounts that have already approved this proposal.
    pub approvals: Vec<AccountId>,
    /// Whether this proposal has been executed.
    pub executed: bool,
    /// Block number when this proposal was created.
    pub created_at: u32,
}

// ── Oracle Slashing Events (Issue #319 / #497) ────────────────────────────────

// (These are referenced in oracle lib.rs via self.env().emit_event — they are
//  defined as #[ink(event)] structs inside the mod propchain_oracle block.
//  We define helper types here for the UpdateThrottled event.)

#[derive(Debug, Clone, PartialEq, scale::Encode, scale::Decode)]
#[cfg_attr(
    feature = "std",
    derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
)]
pub struct AutoSlashReason {
    /// Human-readable reason code: "Staleness" | "Deviation" | "MissedUpdates"
    pub reason: String,
    /// Threshold that was violated
    pub threshold: u64,
    /// Observed value that triggered the slash
    pub observed: u64,
}

// ── Oracle Data History Tracking Types ─────────────────────────────────────────

/// Minimum retention period for history data (7 days in milliseconds)
pub const HISTORY_MIN_RETENTION_MS: u64 = 7 * 24 * 60 * 60 * 1000;

/// Maximum retention period for history data (2 years in milliseconds)
pub const HISTORY_MAX_RETENTION_MS: u64 = 730 * 24 * 60 * 60 * 1000;

/// Default retention period for history data (90 days in milliseconds)
pub const HISTORY_DEFAULT_RETENTION_MS: u64 = 90 * 24 * 60 * 60 * 1000;



