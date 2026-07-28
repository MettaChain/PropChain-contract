//! DEX and trading type definitions.
//!
//! This module contains all types related to the decentralized exchange,
//! order book, liquidity pools, governance, and cross-chain trading.

use crate::bridge::BridgeFeeQuote;
use crate::property::{ChainId, TokenId};
use ink::prelude::string::String;
use ink::prelude::vec::Vec;
use ink::primitives::AccountId;

// =========================================================================
// Order and Trading Types
// =========================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, scale::Encode, scale::Decode)]
#[cfg_attr(
    feature = "std",
    derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
)]
pub enum OrderSide {
    Buy,
    Sell,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, scale::Encode, scale::Decode)]
#[cfg_attr(
    feature = "std",
    derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
)]
pub enum OrderType {
    Market,
    Limit,
    StopLoss,
    TakeProfit,
    Twap,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, scale::Encode, scale::Decode)]
#[cfg_attr(
    feature = "std",
    derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
)]
pub enum TimeInForce {
    GoodTillCancelled,
    ImmediateOrCancel,
    FillOrKill,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, scale::Encode, scale::Decode)]
#[cfg_attr(
    feature = "std",
    derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
)]
pub enum OrderStatus {
    Open,
    PartiallyFilled,
    Filled,
    Cancelled,
    Triggered,
    Expired,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, scale::Encode, scale::Decode)]
#[cfg_attr(
    feature = "std",
    derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
)]
pub enum CrossChainTradeStatus {
    Pending,
    BridgeRequested,
    InFlight,
    Settled,
    Cancelled,
    Failed,
}

// =========================================================================
// Liquidity Pool Types
// =========================================================================

#[derive(Debug, Clone, PartialEq, scale::Encode, scale::Decode)]
#[cfg_attr(
    feature = "std",
    derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
)]
pub struct LiquidityPool {
    pub pair_id: u64,
    pub base_token: TokenId,
    pub quote_token: TokenId,
    pub reserve_base: u128,
    pub reserve_quote: u128,
    pub total_lp_shares: u128,
    pub fee_bips: u32,
    pub reward_index: u128,
    pub cumulative_volume: u128,
    pub last_price: u128,
    pub is_active: bool,
}

#[derive(Debug, Clone, PartialEq, scale::Encode, scale::Decode)]
#[cfg_attr(
    feature = "std",
    derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
)]
pub struct LiquidityPosition {
    pub lp_shares: u128,
    pub reward_debt: u128,
    pub provided_base: u128,
    pub provided_quote: u128,
    pub pending_rewards: u128,
}

// =========================================================================
// Order Types
// =========================================================================

#[derive(Debug, Clone, PartialEq, scale::Encode, scale::Decode)]
#[cfg_attr(
    feature = "std",
    derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
)]
pub struct TradingOrder {
    pub order_id: u64,
    pub pair_id: u64,
    pub trader: AccountId,
    pub side: OrderSide,
    pub order_type: OrderType,
    pub time_in_force: TimeInForce,
    pub price: u128,
    pub amount: u128,
    pub remaining_amount: u128,
    pub trigger_price: Option<u128>,
    pub twap_interval: Option<u64>,
    pub reduce_only: bool,
    pub status: OrderStatus,
    pub created_at: u64,
    pub updated_at: u64,
}

// =========================================================================
// Analytics Types
// =========================================================================

#[derive(Debug, Clone, PartialEq, scale::Encode, scale::Decode)]
#[cfg_attr(
    feature = "std",
    derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
)]
pub struct PairAnalytics {
    pub pair_id: u64,
    pub last_price: u128,
    pub twap_price: u128,
    pub reference_price: u128,
    pub cumulative_volume: u128,
    pub trade_count: u64,
    pub best_bid: u128,
    pub best_ask: u128,
    pub volatility_bips: u32,
    pub last_updated: u64,
    pub high_24h: u128,
    pub low_24h: u128,
    pub volume_24h: u128,
    pub trade_count_24h: u64,
}

#[derive(Debug, Clone, PartialEq, scale::Encode, scale::Decode)]
#[cfg_attr(
    feature = "std",
    derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
)]
pub struct TradingStatistics {
    pub total_pairs: u64,
    pub total_volume_24h: u128,
    pub total_trades_24h: u64,
    pub most_active_pair: Option<u64>,
    pub highest_volume_pair: Option<u64>,
    pub average_volatility_bips: u32,
}

#[derive(Debug, Clone, PartialEq, scale::Encode, scale::Decode)]
#[cfg_attr(
    feature = "std",
    derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
)]
pub struct PriceHistory {
    pub pair_id: u64,
    pub current_price: u128,
    pub high_24h: u128,
    pub low_24h: u128,
    pub twap_price: u128,
    pub reference_price: u128,
    pub volatility_bips: u32,
}

#[derive(Debug, Clone, PartialEq, scale::Encode, scale::Decode)]
#[cfg_attr(
    feature = "std",
    derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
)]
pub struct VolumeAnalytics {
    pub pair_id: u64,
    pub volume_24h: u128,
    pub cumulative_volume: u128,
    pub trade_count_24h: u64,
    pub total_trade_count: u64,
    pub liquidity_base: u128,
    pub liquidity_quote: u128,
}

// =========================================================================
// Admin Timelock Types
// =========================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, scale::Encode, scale::Decode)]
#[cfg_attr(
    feature = "std",
    derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
)]
pub enum AdminActionKind {
    ConfigureBridgeRoute,
    SetLiquidityMining,
    UpdateTimelockDelay,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, scale::Encode, scale::Decode)]
#[cfg_attr(
    feature = "std",
    derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
)]
pub enum AdminActionStatus {
    Scheduled,
    Executed,
    Cancelled,
}

#[derive(Debug, Clone, PartialEq, scale::Encode, scale::Decode)]
#[cfg_attr(
    feature = "std",
    derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
)]
pub struct AdminActionPayload {
    pub destination_chain: ChainId,
    pub gas_estimate: u64,
    pub protocol_fee: u128,
    pub emission_rate: u128,
    pub start_block: u64,
    pub end_block: u64,
    pub reward_token_symbol: String,
    pub timelock_delay_blocks: u64,
}

#[derive(Debug, Clone, PartialEq, scale::Encode, scale::Decode)]
#[cfg_attr(
    feature = "std",
    derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
)]
pub struct PendingAdminAction {
    pub action_id: u64,
    pub kind: AdminActionKind,
    pub payload: AdminActionPayload,
    pub proposer: AccountId,
    pub scheduled_at: u64,
    pub executable_at: u64,
    pub status: AdminActionStatus,
}

// =========================================================================
// Order Book Visualization Types
// =========================================================================

#[derive(Debug, Clone, PartialEq, scale::Encode, scale::Decode)]
#[cfg_attr(
    feature = "std",
    derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
)]
pub struct OrderBookLevel {
    pub price: u128,
    pub total_amount: u128,
    pub order_count: u32,
    pub cumulative_amount: u128,
    pub side: OrderSide,
}

#[derive(Debug, Clone, PartialEq, scale::Encode, scale::Decode)]
#[cfg_attr(
    feature = "std",
    derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
)]
pub struct OrderBookSnapshot {
    pub pair_id: u64,
    pub bids: Vec<OrderBookLevel>,
    pub asks: Vec<OrderBookLevel>,
    pub best_bid: u128,
    pub best_ask: u128,
    pub spread: u128,
    pub mid_price: u128,
    pub total_bid_depth: u128,
    pub total_ask_depth: u128,
    pub last_price: u128,
    pub last_updated: u64,
}

// =========================================================================
// Governance & Mining Types
// =========================================================================

#[derive(Debug, Clone, PartialEq, scale::Encode, scale::Decode)]
#[cfg_attr(
    feature = "std",
    derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
)]
pub struct LiquidityMiningCampaign {
    pub emission_rate: u128,
    pub start_block: u64,
    pub end_block: u64,
    pub reward_token_symbol: String,
}

#[derive(Debug, Clone, PartialEq, scale::Encode, scale::Decode)]
#[cfg_attr(
    feature = "std",
    derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
)]
pub struct GovernanceProposal {
    pub proposal_id: u64,
    pub proposer: AccountId,
    pub title: String,
    pub description_hash: [u8; 32],
    pub new_fee_bips: Option<u32>,
    pub new_emission_rate: Option<u128>,
    pub votes_for: u128,
    pub votes_against: u128,
    pub start_block: u64,
    pub end_block: u64,
    pub executed: bool,
}

#[derive(Debug, Clone, PartialEq, scale::Encode, scale::Decode)]
#[cfg_attr(
    feature = "std",
    derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
)]
pub struct GovernanceTokenConfig {
    pub symbol: String,
    pub total_supply: u128,
    pub emission_rate: u128,
    pub quorum_bips: u32,
}

// =========================================================================
// Portfolio & Cross-Chain Types
// =========================================================================

#[derive(Debug, Clone, PartialEq, scale::Encode, scale::Decode)]
#[cfg_attr(
    feature = "std",
    derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
)]
pub struct PortfolioSnapshot {
    pub owner: AccountId,
    pub liquidity_positions: u64,
    pub open_orders: u64,
    pub pending_rewards: u128,
    pub governance_balance: u128,
    pub estimated_inventory_value: u128,
    pub cross_chain_positions: u64,
}

#[derive(Debug, Clone, PartialEq, scale::Encode, scale::Decode)]
#[cfg_attr(
    feature = "std",
    derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
)]
pub struct CrossChainTradeIntent {
    pub trade_id: u64,
    pub pair_id: u64,
    pub order_id: Option<u64>,
    pub source_chain: ChainId,
    pub destination_chain: ChainId,
    pub trader: AccountId,
    pub recipient: AccountId,
    pub amount_in: u128,
    pub min_amount_out: u128,
    pub bridge_request_id: Option<u64>,
    pub bridge_fee_quote: BridgeFeeQuote,
    pub status: CrossChainTradeStatus,
    pub created_at: u64,
}
