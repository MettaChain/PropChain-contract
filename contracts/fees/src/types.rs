// Data types for the fees contract (Issue #101 - extracted from lib.rs)

#[derive(Debug, Clone, Copy, PartialEq, Eq, scale::Encode, scale::Decode)]
#[cfg_attr(
    feature = "std",
    derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
)]
pub enum FeeCalculationMethod {
    Fixed,
    Dynamic,
    Tiered,
    Exponential,
}

#[derive(Debug, Clone, PartialEq, scale::Encode, scale::Decode)]
#[cfg_attr(
    feature = "std",
    derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
)]
pub struct FeeConfig {
    pub base_fee: u128,
    pub min_fee: u128,
    pub max_fee: u128,
    pub congestion_sensitivity: u32,
    pub demand_factor_bp: u32,
    pub calculation_method: FeeCalculationMethod,
    pub last_updated: u64,
}

/// Dynamic fee configuration driven by pool utilization / market congestion.
///
/// At 0 % utilisation the effective fee equals `base_fee_bps`.
/// At 100 % utilisation the effective fee equals
/// `base_fee_bps * congestion_multiplier / 100`, capped at `max_fee_bps`.
/// When activity drops the rate falls back towards `base_fee_bps`.
#[derive(Debug, Clone, PartialEq, scale::Encode, scale::Decode)]
#[cfg_attr(
    feature = "std",
    derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
)]
pub struct DynamicFeeConfig {
    /// Base fee rate in basis points (e.g. 30 = 0.30 %)
    pub base_fee_bps: u32,
    /// Multiplier applied at 100 % utilisation, expressed as a percentage
    /// of the base (e.g. 300 means 3× base at full utilisation).
    pub congestion_multiplier: u32,
    /// Hard cap on the effective fee rate in basis points.
    pub max_fee_bps: u32,
}

pub struct FeeContext {
    pub congestion_index: u32,
    pub demand_factor_bp: u32,
    pub operation: FeeOperation,
}

#[derive(Debug, Clone, scale::Encode, scale::Decode)]
#[cfg_attr(
    feature = "std",
    derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
)]
#[allow(dead_code)]
pub struct FeeHistoryEntry {
    pub timestamp: u64,
    pub operation_count: u32,
    pub total_fees_collected: u128,
}

#[derive(Debug, Clone, PartialEq, scale::Encode, scale::Decode)]
#[cfg_attr(
    feature = "std",
    derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
)]
pub struct PremiumAuction {
    pub property_id: u64,
    pub seller: AccountId,
    pub min_bid: u128,
    pub current_bid: u128,
    pub current_bidder: Option<AccountId>,
    pub end_time: u64,
    pub settled: bool,
    pub fee_paid: u128,
}

#[derive(Debug, Clone, scale::Encode, scale::Decode)]
#[cfg_attr(
    feature = "std",
    derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
)]
pub struct AuctionBid {
    pub bidder: AccountId,
    pub amount: u128,
    pub timestamp: u64,
}

#[derive(Debug, Clone, scale::Encode, scale::Decode)]
#[cfg_attr(
    feature = "std",
    derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
)]
pub struct RewardRecord {
    pub account: AccountId,
    pub amount: u128,
    pub reason: RewardReason,
    pub timestamp: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, scale::Encode, scale::Decode)]
#[cfg_attr(
    feature = "std",
    derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
)]
pub enum RewardReason {
    ValidatorReward,
    LiquidityProvider,
    PremiumListingFee,
    ParticipationIncentive,
}

#[derive(Debug, Clone, PartialEq, scale::Encode, scale::Decode)]
#[cfg_attr(
    feature = "std",
    derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
)]
pub struct FeeReport {
    pub config: FeeConfig,
    pub congestion_index: u32,
    pub recommended_fee: u128,
    pub total_fees_collected: u128,
    pub total_distributed: u128,
    pub operation_count_24h: u64,
    pub premium_auctions_active: u32,
    pub timestamp: u64,
}

#[derive(Debug, Clone, PartialEq, scale::Encode, scale::Decode)]
#[cfg_attr(
    feature = "std",
    derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
)]
pub struct FeeEstimate {
    pub operation: FeeOperation,
    pub estimated_fee: u128,
    pub min_fee: u128,
    pub max_fee: u128,
    pub congestion_level: String,
    pub recommendation: String,
}
