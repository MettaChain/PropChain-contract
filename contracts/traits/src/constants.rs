/// Centralized configuration constants for PropChain contracts.
///
/// All magic numbers are extracted here with documentation explaining
/// their purpose and valid ranges. Contracts import from this module
/// instead of using inline literals.

// ── Oracle Constants ─────────────────────────────────────────────────────────

/// Maximum age (in seconds) before a price is considered stale.
/// Default: 3600 (1 hour).
pub const DEFAULT_MAX_PRICE_STALENESS: u64 = 3600;

/// Minimum number of oracle sources required for a valid valuation.
pub const DEFAULT_MIN_SOURCES_REQUIRED: u32 = 2;

/// Number of standard deviations beyond which a price is an outlier.
pub const DEFAULT_OUTLIER_THRESHOLD: u32 = 2;

/// Initial reputation score assigned to new oracle sources (0-1000 scale).
pub const ORACLE_INITIAL_REPUTATION: u32 = 500;

/// Maximum reputation score an oracle source can achieve.
pub const ORACLE_MAX_REPUTATION: u32 = 1000;

/// Minimum reputation required for an oracle source to participate.
pub const ORACLE_MIN_REPUTATION_THRESHOLD: u32 = 200;

/// Reputation points gained on a successful price submission.
pub const ORACLE_REPUTATION_GAIN: u32 = 10;

/// Reputation points lost on a failed/inaccurate submission.
pub const ORACLE_REPUTATION_PENALTY: u32 = 50;

/// Multiplier for coefficient of variance calculations (basis points).
pub const COEFFICIENT_VARIANCE_MULTIPLIER: u32 = 10_000;

// ── Bridge Constants ─────────────────────────────────────────────────────────

/// Default gas multiplier for bridge operations (100 = 1.0x).
/// Expressed as percentage: 100 = 100% = 1x, 150 = 150% = 1.5x.
pub const DEFAULT_GAS_MULTIPLIER: u32 = 100;

/// Default number of block confirmations before a bridge tx is final.
pub const DEFAULT_CONFIRMATION_BLOCKS: u32 = 6;

/// Base gas cost for a bridge operation (in gas units).
pub const BRIDGE_BASE_GAS: u64 = 100_000;

// ── IPFS / Metadata Constants ────────────────────────────────────────────────

/// Maximum length for property location strings.
pub const MAX_LOCATION_LENGTH: u32 = 500;

/// Minimum property size in square meters.
pub const MIN_PROPERTY_SIZE: u64 = 1;

/// Maximum property size in square meters (1 billion).
pub const MAX_PROPERTY_SIZE: u64 = 1_000_000_000;

/// Maximum length for legal description text.
pub const MAX_LEGAL_DESCRIPTION_LENGTH: u32 = 5_000;

/// Minimum valuation amount (in smallest token unit).
pub const MIN_VALUATION: u128 = 1;

/// Maximum file size for IPFS uploads (100 MB).
pub const MAX_FILE_SIZE: u64 = 100_000_000;

/// Maximum number of documents per property.
pub const MAX_DOCUMENTS_PER_PROPERTY: u32 = 100;

/// Maximum total pinned size per property (500 MB).
pub const MAX_PINNED_SIZE_PER_PROPERTY: u64 = 500_000_000;

// ── Token Constants ──────────────────────────────────────────────────────────

/// Precision scaling factor for token amounts (1e12).
pub const TOKEN_SCALING_FACTOR: u128 = 1_000_000_000_000;

// ── Analytics Constants ──────────────────────────────────────────────────────

/// Default bull/bear ratio in basis points (50% = 5000 bps).
pub const DEFAULT_BULL_BEAR_RATIO_BPS: u32 = 5_000;

/// Basis points denominator (100% = 10000 bps).
pub const BASIS_POINTS_DENOMINATOR: u32 = 10_000;

// ── Storage Constants ────────────────────────────────────────────────────────

/// Number of reserved `u128` slots for future storage upgrades.
/// Each contract SHOULD include a `__storage_gap: [u128; STORAGE_GAP_SIZE]`
/// field at the end of its storage struct.
pub const STORAGE_GAP_SIZE: usize = 20;

// ── Governance Constants ─────────────────────────────────────────────────────

/// Minimum number of signers required for a governance multisig.
pub const GOVERNANCE_MIN_SIGNERS: u32 = 2;

/// Maximum number of signers allowed in a governance multisig.
pub const GOVERNANCE_MAX_SIGNERS: u32 = 50;

/// Default timelock duration in blocks before a proposal can be executed.
pub const GOVERNANCE_DEFAULT_TIMELOCK_BLOCKS: u64 = 100;

/// Maximum number of active proposals at once.
pub const GOVERNANCE_MAX_ACTIVE_PROPOSALS: u32 = 100;

// ── Staking Constants ────────────────────────────────────────────────────────

/// Minimum stake amount (in smallest token unit).
pub const STAKING_MIN_AMOUNT: u128 = 1_000_000_000_000; // 1 token at 1e12 precision

/// Lock period: 30 days in blocks (assuming 6-second blocks).
pub const LOCK_PERIOD_30_DAYS: u64 = 432_000;

/// Lock period: 90 days in blocks.
pub const LOCK_PERIOD_90_DAYS: u64 = 1_296_000;

/// Lock period: 1 year in blocks.
pub const LOCK_PERIOD_1_YEAR: u64 = 5_256_000;

/// Reward rate precision (basis points, 10000 = 100%).
pub const REWARD_RATE_PRECISION: u128 = 10_000;

/// Lock-period reward multiplier: flexible = 1x (100 bps).
pub const MULTIPLIER_FLEXIBLE: u128 = 100;

/// Lock-period reward multiplier: 30 days = 1.25x.
pub const MULTIPLIER_30_DAYS: u128 = 125;

/// Lock-period reward multiplier: 90 days = 1.75x.
pub const MULTIPLIER_90_DAYS: u128 = 175;

/// Lock-period reward multiplier: 1 year = 3x.
pub const MULTIPLIER_1_YEAR: u128 = 300;
