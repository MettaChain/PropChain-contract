#![cfg_attr(not(feature = "std"), no_std)]
#![allow(unexpected_cfgs)]

use ink::prelude::{string::String, vec::Vec};
use ink::storage::Mapping;
use propchain_traits::*;

#[ink::contract]
mod dex {
    use super::*;
    use propchain_traits::{non_reentrant, ReentrancyError, ReentrancyGuard};

    const BIPS_DENOMINATOR: u128 = 10_000;
    const REWARD_PRECISION: u128 = 1_000_000_000;

    // Error types extracted to errors.rs (Issue #101)
    include!("errors.rs");

    impl From<ReentrancyError> for Error {
        fn from(_: ReentrancyError) -> Self {
            Error::ReentrantCall
        }
    }

    #[ink(event)]
    pub struct PoolCreated {
        #[ink(topic)]
        pub pair_id: u64,
        pub base_token: TokenId,
        pub quote_token: TokenId,
    }

    #[ink(event)]
    pub struct LiquidityAdded {
        #[ink(topic)]
        pub pair_id: u64,
        #[ink(topic)]
        pub provider: AccountId,
        pub minted_shares: u128,
    }

    #[ink(event)]
    pub struct SwapExecuted {
        #[ink(topic)]
        pub pair_id: u64,
        #[ink(topic)]
        pub trader: AccountId,
        pub amount_in: u128,
        pub amount_out: u128,
    }

    #[ink(event)]
    pub struct PriceImpactWarning {
        #[ink(topic)]
        pub pair_id: u64,
        #[ink(topic)]
        pub trader: AccountId,
        pub price_impact_bips: u32,
        pub amount_in: u128,
    }

    #[ink(event)]
    pub struct OrderPlaced {
        #[ink(topic)]
        pub order_id: u64,
        #[ink(topic)]
        pub pair_id: u64,
        #[ink(topic)]
        pub trader: AccountId,
    }

    #[ink(event)]
    pub struct CrossChainTradeCreated {
        #[ink(topic)]
        pub trade_id: u64,
        #[ink(topic)]
        pub pair_id: u64,
        pub destination_chain: ChainId,
    }

    #[ink(event)]
    pub struct TradingCompetitionCreated {
        #[ink(topic)]
        pub competition_id: u64,
        pub pair_id: Option<u64>,
        pub title: String,
        pub reward_amount: u128,
    }

    #[ink(event)]
    pub struct CompetitionScoreUpdated {
        #[ink(topic)]
        pub competition_id: u64,
        #[ink(topic)]
        pub trader: AccountId,
        pub score: u128,
    }

    #[ink(event)]
    pub struct CompetitionRewardClaimed {
        #[ink(topic)]
        pub competition_id: u64,
        #[ink(topic)]
        pub trader: AccountId,
        pub reward_amount: u128,
    }

    #[derive(
        Debug, Clone, PartialEq, scale::Encode, scale::Decode, ink::storage::traits::StorageLayout,
    )]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub struct TradingCompetition {
        pub competition_id: u64,
        pub pair_id: Option<u64>,
        pub title: String,
        pub reward_amount: u128,
        pub start_block: u64,
        pub end_block: u64,
        pub min_trade_volume: u128,
        pub top_n: u32,
        pub reward_token_symbol: String,
        pub active: bool,
    }

    #[ink(event)]
    pub struct AdminActionScheduled {
        #[ink(topic)]
        pub action_id: u64,
        #[ink(topic)]
        pub proposer: AccountId,
        pub kind: AdminActionKind,
        pub executable_at: u64,
    }

    #[ink(event)]
    pub struct AdminActionExecuted {
        #[ink(topic)]
        pub action_id: u64,
        pub kind: AdminActionKind,
    }

    #[ink(event)]
    pub struct AdminActionCancelled {
        #[ink(topic)]
        pub action_id: u64,
    }

    #[ink(storage)]
    pub struct PropertyDex {
        admin: AccountId,
        pair_counter: u64,
        order_counter: u64,
        cross_chain_trade_counter: u64,
        proposal_counter: u64,
        pools: Mapping<u64, LiquidityPool>,
        pair_lookup: Mapping<(TokenId, TokenId), u64>,
        positions: Mapping<(u64, AccountId), LiquidityPosition>,
        orders: Mapping<u64, TradingOrder>,
        order_book: Mapping<(u64, u64), u64>,
        order_book_count: Mapping<u64, u64>,
        analytics: Mapping<u64, PairAnalytics>,
        bridge_quotes: Mapping<ChainId, BridgeFeeQuote>,
        cross_chain_trades: Mapping<u64, CrossChainTradeIntent>,
        governance_config: GovernanceTokenConfig,
        governance_balances: Mapping<AccountId, u128>,
        governance_proposals: Mapping<u64, GovernanceProposal>,
        votes_cast: Mapping<(u64, AccountId), bool>,
        liquidity_mining: LiquidityMiningCampaign,
        last_reward_block: Mapping<u64, u64>,
        reentrancy_guard: ReentrancyGuard,
        trade_competition_counter: u64,
        trading_competitions: Mapping<u64, TradingCompetition>,
        competition_scores: Mapping<(u64, AccountId), u128>,
        competition_participants: Mapping<u64, Vec<AccountId>>,
        competition_claimed: Mapping<(u64, AccountId), bool>,
        admin_timelock_delay: u64,
        pending_admin_actions: Mapping<u64, PendingAdminAction>,
        pending_admin_action_counter: u64,
    }

    impl PropertyDex {
        #[ink(constructor)]
        pub fn new(
            governance_symbol: String,
            governance_supply: u128,
            emission_rate: u128,
            quorum_bips: u32,
        ) -> Self {
            let caller = Self::env().caller();
            let mut instance = Self {
                admin: caller,
                pair_counter: 0,
                order_counter: 0,
                cross_chain_trade_counter: 0,
                proposal_counter: 0,
                pools: Mapping::default(),
                pair_lookup: Mapping::default(),
                positions: Mapping::default(),
                orders: Mapping::default(),
                order_book: Mapping::default(),
                order_book_count: Mapping::default(),
                analytics: Mapping::default(),
                bridge_quotes: Mapping::default(),
                cross_chain_trades: Mapping::default(),
                governance_config: GovernanceTokenConfig {
                    symbol: governance_symbol,
                    total_supply: governance_supply,
                    emission_rate,
                    quorum_bips,
                },
                governance_balances: Mapping::default(),
                governance_proposals: Mapping::default(),
                votes_cast: Mapping::default(),
                liquidity_mining: LiquidityMiningCampaign {
                    emission_rate,
                    start_block: 0,
                    end_block: u64::MAX,
                    reward_token_symbol: String::from("GOV"),
                },
                last_reward_block: Mapping::default(),
                reentrancy_guard: ReentrancyGuard::new(),
                trade_competition_counter: 0,
                trading_competitions: Mapping::default(),
                competition_scores: Mapping::default(),
                competition_participants: Mapping::default(),
                competition_claimed: Mapping::default(),
                admin_timelock_delay: 0,
                pending_admin_actions: Mapping::default(),
                pending_admin_action_counter: 0,
            };
            instance
                .governance_balances
                .insert(caller, &governance_supply);
            instance
        }

        #[ink(message)]
        pub fn create_pool(
            &mut self,
            base_token: TokenId,
            quote_token: TokenId,
            fee_bips: u32,
            initial_base: u128,
            initial_quote: u128,
        ) -> Result<u64, Error> {
            non_reentrant!(self, {
                self.ensure_admin_or_pair_creator()?;
                if base_token == quote_token
                    || initial_base == 0
                    || initial_quote == 0
                    || fee_bips >= 1_000
                {
                    return Err(Error::InvalidPair);
                }

                let key = ordered_pair(base_token, quote_token);
                if self.pair_lookup.get(key).unwrap_or(0) != 0 {
                    return Err(Error::InvalidPair);
                }

                self.pair_counter += 1;
                let pair_id = self.pair_counter;
                let last_price = initial_quote
                    .saturating_mul(BIPS_DENOMINATOR)
                    .checked_div(initial_base)
                    .unwrap_or(0);
                let minted = integer_sqrt(initial_base.saturating_mul(initial_quote));
                let pool = LiquidityPool {
                    pair_id,
                    base_token,
                    quote_token,
                    reserve_base: initial_base,
                    reserve_quote: initial_quote,
                    total_lp_shares: minted,
                    fee_bips,
                    reward_index: 0,
                    cumulative_volume: 0,
                    last_price,
                    is_active: true,
                };
                self.pools.insert(pair_id, &pool);
                self.pair_lookup.insert(key, &pair_id);
                self.positions.insert(
                    (pair_id, self.env().caller()),
                    &LiquidityPosition {
                        lp_shares: minted,
                        reward_debt: 0,
                        provided_base: initial_base,
                        provided_quote: initial_quote,
                        pending_rewards: 0,
                    },
                );
                self.analytics.insert(
                    pair_id,
                    &PairAnalytics {
                        pair_id,
                        last_price,
                        twap_price: last_price,
                        reference_price: last_price,
                        cumulative_volume: 0,
                        trade_count: 0,
                        best_bid: 0,
                        best_ask: 0,
                        volatility_bips: 0,
                        last_updated: self.env().block_timestamp(),
                        high_24h: last_price,
                        low_24h: last_price,
                        volume_24h: 0,
                        trade_count_24h: 0,
                    },
                );
                self.last_reward_block
                    .insert(pair_id, &u64::from(self.env().block_number()));

                self.env().emit_event(PoolCreated {
                    pair_id,
                    base_token,
                    quote_token,
                });

                Ok(pair_id)
            })
        }

        #[ink(message)]
        pub fn add_liquidity(
            &mut self,
            pair_id: u64,
            amount_base: u128,
            amount_quote: u128,
        ) -> Result<u128, Error> {
            non_reentrant!(self, {
                if amount_base == 0 || amount_quote == 0 {
                    return Err(Error::InvalidPair);
                }
                self.accrue_rewards(pair_id)?;
                let mut pool = self.pool(pair_id)?;
                let minted_shares = if pool.total_lp_shares == 0 {
                    integer_sqrt(amount_base.saturating_mul(amount_quote))
                } else {
                    let base_shares = amount_base
                        .saturating_mul(pool.total_lp_shares)
                        .checked_div(pool.reserve_base)
                        .unwrap_or(0);
                    let quote_shares = amount_quote
                        .saturating_mul(pool.total_lp_shares)
                        .checked_div(pool.reserve_quote)
                        .unwrap_or(0);
                    core::cmp::min(base_shares, quote_shares)
                };

                pool.reserve_base = pool.reserve_base.saturating_add(amount_base);
                pool.reserve_quote = pool.reserve_quote.saturating_add(amount_quote);
                pool.total_lp_shares = pool.total_lp_shares.saturating_add(minted_shares);
                self.update_pool_price(&mut pool);
                self.pools.insert(pair_id, &pool);

                let caller = self.env().caller();
                let mut position = self.position(pair_id, caller);
                let accrued = pending_from_indices(
                    position.lp_shares,
                    pool.reward_index,
                    position.reward_debt,
                );
                position.pending_rewards = position.pending_rewards.saturating_add(accrued);
                position.reward_debt = scaled_reward_debt(
                    position.lp_shares.saturating_add(minted_shares),
                    pool.reward_index,
                );
                position.lp_shares = position.lp_shares.saturating_add(minted_shares);
                position.provided_base = position.provided_base.saturating_add(amount_base);
                position.provided_quote = position.provided_quote.saturating_add(amount_quote);
                self.positions.insert((pair_id, caller), &position);

                let mut analytics = self.analytics_for(pair_id);
                analytics.last_updated = self.env().block_timestamp();
                self.analytics.insert(pair_id, &analytics);

                self.env().emit_event(LiquidityAdded {
                    pair_id,
                    provider: caller,
                    minted_shares,
                });

                Ok(minted_shares)
            })
        }

        #[ink(message)]
        pub fn remove_liquidity(
            &mut self,
            pair_id: u64,
            shares: u128,
        ) -> Result<(u128, u128), Error> {
            non_reentrant!(self, {
                if shares == 0 {
                    return Err(Error::InvalidPair);
                }
                self.accrue_rewards(pair_id)?;
                let mut pool = self.pool(pair_id)?;
                let caller = self.env().caller();
                let mut position = self.position(pair_id, caller);
                if shares > position.lp_shares {
                    return Err(Error::InsufficientLiquidity);
                }

                let base_out = shares
                    .saturating_mul(pool.reserve_base)
                    .checked_div(pool.total_lp_shares)
                    .unwrap_or(0);
                let quote_out = shares
                    .saturating_mul(pool.reserve_quote)
                    .checked_div(pool.total_lp_shares)
                    .unwrap_or(0);
                pool.reserve_base = pool.reserve_base.saturating_sub(base_out);
                pool.reserve_quote = pool.reserve_quote.saturating_sub(quote_out);
                pool.total_lp_shares = pool.total_lp_shares.saturating_sub(shares);
                self.update_pool_price(&mut pool);
                self.pools.insert(pair_id, &pool);

                let accrued = pending_from_indices(
                    position.lp_shares,
                    pool.reward_index,
                    position.reward_debt,
                );
                position.pending_rewards = position.pending_rewards.saturating_add(accrued);
                position.lp_shares = position.lp_shares.saturating_sub(shares);
                position.reward_debt = scaled_reward_debt(position.lp_shares, pool.reward_index);
                self.positions.insert((pair_id, caller), &position);

                Ok((base_out, quote_out))
            })
        }

        #[ink(message)]
        pub fn swap_exact_base_for_quote(
            &mut self,
            pair_id: u64,
            amount_in: u128,
            min_quote_out: u128,
        ) -> Result<u128, Error> {
            non_reentrant!(self, {
                self.swap(pair_id, OrderSide::Sell, amount_in, min_quote_out)
            })
        }

        #[ink(message)]
        pub fn swap_exact_quote_for_base(
            &mut self,
            pair_id: u64,
            amount_in: u128,
            min_base_out: u128,
        ) -> Result<u128, Error> {
            non_reentrant!(self, {
                self.swap(pair_id, OrderSide::Buy, amount_in, min_base_out)
            })
        }

        #[ink(message)]
        #[allow(clippy::too_many_arguments)]
        pub fn place_order(
            &mut self,
            pair_id: u64,
            side: OrderSide,
            order_type: OrderType,
            time_in_force: TimeInForce,
            price: u128,
            amount: u128,
            trigger_price: Option<u128>,
            twap_interval: Option<u64>,
            reduce_only: bool,
        ) -> Result<u64, Error> {
            non_reentrant!(self, {
                if amount == 0 {
                    return Err(Error::InvalidOrder);
                }
                let _ = self.pool(pair_id)?;
                if matches!(
                    order_type,
                    OrderType::Limit | OrderType::StopLoss | OrderType::TakeProfit
                ) && price == 0
                {
                    return Err(Error::InvalidOrder);
                }

                self.order_counter += 1;
                let now = self.env().block_timestamp();
                let order_id = self.order_counter;
                let order = TradingOrder {
                    order_id,
                    pair_id,
                    trader: self.env().caller(),
                    side,
                    order_type,
                    time_in_force,
                    price,
                    amount,
                    remaining_amount: amount,
                    trigger_price,
                    twap_interval,
                    reduce_only,
                    status: OrderStatus::Open,
                    created_at: now,
                    updated_at: now,
                };
                self.orders.insert(order_id, &order);
                let count = self.order_book_count.get(pair_id).unwrap_or(0);
                self.order_book.insert((pair_id, count), &order_id);
                self.order_book_count.insert(pair_id, &(count + 1));

                self.refresh_best_quotes(pair_id);

                self.env().emit_event(OrderPlaced {
                    order_id,
                    pair_id,
                    trader: self.env().caller(),
                });

                if matches!(
                    time_in_force,
                    TimeInForce::ImmediateOrCancel | TimeInForce::FillOrKill
                ) || matches!(order_type, OrderType::Market)
                {
                    self.execute_order(order_id, amount)?;
                }

                Ok(order_id)
            })
        }

        #[ink(message)]
        pub fn execute_order(
            &mut self,
            order_id: u64,
            requested_amount: u128,
        ) -> Result<u128, Error> {
            non_reentrant!(self, {
                self.execute_order_core(order_id, requested_amount)
            })
        }

        fn execute_order_core(
            &mut self,
            order_id: u64,
            requested_amount: u128,
        ) -> Result<u128, Error> {
            let mut order = self.order(order_id)?;
            if !matches!(
                order.status,
                OrderStatus::Open | OrderStatus::PartiallyFilled | OrderStatus::Triggered
            ) {
                return Err(Error::OrderNotExecutable);
            }

            let executable = self.is_order_executable(&order)?;
            if !executable {
                return Err(Error::OrderNotExecutable);
            }

            let fill_amount = core::cmp::min(requested_amount, order.remaining_amount);
            if fill_amount == 0 {
                return Err(Error::InvalidOrder);
            }

            let pair_id = order.pair_id;
            let output = match order.side {
                OrderSide::Sell => self.swap(pair_id, OrderSide::Sell, fill_amount, 0)?,
                OrderSide::Buy => self.swap(pair_id, OrderSide::Buy, fill_amount, 0)?,
            };

            order.remaining_amount = order.remaining_amount.saturating_sub(fill_amount);
            order.updated_at = self.env().block_timestamp();
            order.status = if order.remaining_amount == 0 {
                OrderStatus::Filled
            } else {
                OrderStatus::PartiallyFilled
            };
            self.orders.insert(order_id, &order);
            self.refresh_best_quotes(pair_id);

            Ok(output)
        }

        #[ink(message)]
        pub fn match_orders(
            &mut self,
            maker_order_id: u64,
            taker_order_id: u64,
            amount: u128,
        ) -> Result<u128, Error> {
            non_reentrant!(self, {
                let mut maker = self.order(maker_order_id)?;
                let mut taker = self.order(taker_order_id)?;
                if maker.pair_id != taker.pair_id || maker.side == taker.side {
                    return Err(Error::InvalidOrder);
                }

                let fill_amount = core::cmp::min(
                    amount,
                    core::cmp::min(maker.remaining_amount, taker.remaining_amount),
                );
                if fill_amount == 0 {
                    return Err(Error::InvalidOrder);
                }

                let execution_price = if maker.price > 0 {
                    maker.price
                } else {
                    taker.price
                };
                let notional = fill_amount
                    .saturating_mul(execution_price)
                    .checked_div(BIPS_DENOMINATOR)
                    .unwrap_or(0);

                maker.remaining_amount = maker.remaining_amount.saturating_sub(fill_amount);
                taker.remaining_amount = taker.remaining_amount.saturating_sub(fill_amount);
                maker.status = if maker.remaining_amount == 0 {
                    OrderStatus::Filled
                } else {
                    OrderStatus::PartiallyFilled
                };
                taker.status = if taker.remaining_amount == 0 {
                    OrderStatus::Filled
                } else {
                    OrderStatus::PartiallyFilled
                };
                maker.updated_at = self.env().block_timestamp();
                taker.updated_at = maker.updated_at;
                self.orders.insert(maker_order_id, &maker);
                self.orders.insert(taker_order_id, &taker);

                let mut analytics = self.analytics_for(maker.pair_id);
                let prev = analytics.last_price;
                analytics.last_price = execution_price;
                analytics.reference_price =
                    weighted_average(execution_price, analytics.twap_price, 7, 3);
                analytics.twap_price =
                    weighted_average(execution_price, analytics.twap_price, 1, 1);
                analytics.cumulative_volume = analytics.cumulative_volume.saturating_add(notional);
                analytics.trade_count = analytics.trade_count.saturating_add(1);
                analytics.volatility_bips = volatility_bips(prev, execution_price);
                analytics.last_updated = self.env().block_timestamp();
                self.analytics.insert(maker.pair_id, &analytics);
                self.refresh_best_quotes(maker.pair_id);

                Ok(notional)
            })
        }

        #[ink(message)]
        pub fn cancel_order(&mut self, order_id: u64) -> Result<(), Error> {
            non_reentrant!(self, {
                let mut order = self.order(order_id)?;
                let caller = self.env().caller();
                if caller != order.trader && caller != self.admin {
                    return Err(Error::Unauthorized);
                }
                order.status = OrderStatus::Cancelled;
                order.updated_at = self.env().block_timestamp();
                self.orders.insert(order_id, &order);
                self.refresh_best_quotes(order.pair_id);
                Ok(())
            })
        }

        #[ink(message)]
        pub fn configure_bridge_route(
            &mut self,
            destination_chain: ChainId,
            gas_estimate: u64,
            protocol_fee: u128,
        ) -> Result<(), Error> {
            if self.env().caller() != self.admin {
                return Err(Error::Unauthorized);
            }
            if self.admin_timelock_delay > 0 {
                return Err(Error::TimelockRequired);
            }
            self.apply_configure_bridge_route(destination_chain, gas_estimate, protocol_fee);
            Ok(())
        }

        #[ink(message)]
        pub fn quote_cross_chain_trade(
            &self,
            destination_chain: ChainId,
        ) -> Result<BridgeFeeQuote, Error> {
            self.bridge_quotes
                .get(destination_chain)
                .ok_or(Error::InvalidBridgeRoute)
        }

        #[ink(message)]
        pub fn create_cross_chain_trade(
            &mut self,
            pair_id: u64,
            order_id: Option<u64>,
            destination_chain: ChainId,
            recipient: AccountId,
            amount_in: u128,
            min_amount_out: u128,
        ) -> Result<u64, Error> {
            non_reentrant!(self, {
                let _ = self.pool(pair_id)?;
                let quote = self.quote_cross_chain_trade(destination_chain)?;
                self.cross_chain_trade_counter += 1;
                let trade_id = self.cross_chain_trade_counter;
                let intent = CrossChainTradeIntent {
                    trade_id,
                    pair_id,
                    order_id,
                    source_chain: 1,
                    destination_chain,
                    trader: self.env().caller(),
                    recipient,
                    amount_in,
                    min_amount_out,
                    bridge_request_id: None,
                    bridge_fee_quote: quote,
                    status: CrossChainTradeStatus::Pending,
                    created_at: self.env().block_timestamp(),
                };
                self.cross_chain_trades.insert(trade_id, &intent);
                self.env().emit_event(CrossChainTradeCreated {
                    trade_id,
                    pair_id,
                    destination_chain,
                });
                Ok(trade_id)
            })
        }

        #[ink(message)]
        pub fn attach_bridge_request(
            &mut self,
            trade_id: u64,
            bridge_request_id: u64,
        ) -> Result<(), Error> {
            non_reentrant!(self, {
                let mut trade = self.cross_chain_trade(trade_id)?;
                if self.env().caller() != trade.trader && self.env().caller() != self.admin {
                    return Err(Error::Unauthorized);
                }
                trade.bridge_request_id = Some(bridge_request_id);
                trade.status = CrossChainTradeStatus::BridgeRequested;
                self.cross_chain_trades.insert(trade_id, &trade);
                Ok(())
            })
        }

        #[ink(message)]
        pub fn finalize_cross_chain_trade(&mut self, trade_id: u64) -> Result<(), Error> {
            non_reentrant!(self, {
                let mut trade = self.cross_chain_trade(trade_id)?;
                if self.env().caller() != self.admin {
                    return Err(Error::Unauthorized);
                }
                trade.status = CrossChainTradeStatus::Settled;
                self.cross_chain_trades.insert(trade_id, &trade);
                Ok(())
            })
        }

        #[ink(message)]
        pub fn set_liquidity_mining_campaign(
            &mut self,
            emission_rate: u128,
            start_block: u64,
            end_block: u64,
            reward_token_symbol: String,
        ) -> Result<(), Error> {
            non_reentrant!(self, {
                if self.env().caller() != self.admin {
                    return Err(Error::Unauthorized);
                }
                if self.admin_timelock_delay > 0 {
                    return Err(Error::TimelockRequired);
                }
                self.liquidity_mining = LiquidityMiningCampaign {
                    emission_rate,
                    start_block,
                    end_block,
                    reward_token_symbol,
                };
                self.governance_config.emission_rate = emission_rate;
                Ok(())
            })
        }

        #[ink(message)]
        pub fn tally_competition_leaderboard(&self, competition_id: u64) -> Vec<(AccountId, u128)> {
            self.get_competition_leaderboard(competition_id)
        }

        #[ink(message)]
        pub fn get_competition_status(&self, competition_id: u64) -> Option<(bool, u64, u64)> {
            self.trading_competitions
                .get(competition_id)
                .map(|competition| {
                    (
                        competition.active,
                        competition.start_block,
                        competition.end_block,
                    )
                })
        }

        #[ink(message)]
        pub fn get_competition_reward_state(&self, competition_id: u64) -> Option<bool> {
            Some(self.is_competition_reward_claimed(competition_id, self.env().caller()))
        }

        #[ink(message)]
        pub fn get_reward_share_for_trader(
            &self,
            competition_id: u64,
            trader: AccountId,
        ) -> Option<u128> {
            let competition = self.trading_competitions.get(competition_id)?;
            let score = self
                .competition_scores
                .get((competition_id, trader))
                .unwrap_or(0);
            if score == 0 {
                return None;
            }
            let participants = self
                .competition_participants
                .get(competition_id)
                .unwrap_or_default();
            let mut total_score = 0u128;
            for participant in participants {
                total_score = total_score.saturating_add(
                    self.competition_scores
                        .get((competition_id, participant))
                        .unwrap_or(0),
                );
            }
            if total_score == 0 {
                return None;
            }
            Some(
                competition
                    .reward_amount
                    .saturating_mul(score)
                    .checked_div(total_score)
                    .unwrap_or(0),
            )
        }

        #[ink(message)]
        pub fn get_competition_total_score(&self, competition_id: u64) -> u128 {
            let participants = self
                .competition_participants
                .get(competition_id)
                .unwrap_or_default();
            let mut total_score = 0u128;
            for participant in participants {
                total_score = total_score.saturating_add(
                    self.competition_scores
                        .get((competition_id, participant))
                        .unwrap_or(0),
                );
            }
            total_score
        }

        #[ink(message)]
        pub fn get_competition_count(&self) -> u64 {
            self.trade_competition_counter
        }

        #[ink(message)]
        pub fn get_competition_participant_reward_status(
            &self,
            competition_id: u64,
            trader: AccountId,
        ) -> bool {
            self.is_competition_reward_claimed(competition_id, trader)
        }

        #[ink(message)]
        pub fn get_competition_reward_balance(&self, trader: AccountId) -> u128 {
            self.governance_balances.get(trader).unwrap_or(0)
        }

        #[ink(message)]
        pub fn get_competition_settings(&self, competition_id: u64) -> Option<(u32, u128)> {
            self.trading_competitions
                .get(competition_id)
                .map(|competition| (competition.top_n, competition.min_trade_volume))
        }

        #[ink(message)]
        pub fn get_competition_details_by_title(&self, title: String) -> Vec<TradingCompetition> {
            let mut results = Vec::new();
            for competition_id in 1..=self.trade_competition_counter {
                if let Some(comp) = self.trading_competitions.get(competition_id) {
                    if comp.title == title {
                        results.push(comp);
                    }
                }
            }
            results
        }

        #[ink(message)]
        pub fn get_competition_rewards_summary(&self, competition_id: u64) -> Option<(u128, bool)> {
            self.trading_competitions
                .get(competition_id)
                .map(|competition| (competition.reward_amount, competition.active))
        }

        #[ink(message)]
        pub fn get_competition_status_summary(
            &self,
            competition_id: u64,
        ) -> Option<(bool, u64, u64, u128)> {
            self.trading_competitions
                .get(competition_id)
                .map(|competition| {
                    (
                        competition.active,
                        competition.start_block,
                        competition.end_block,
                        competition.reward_amount,
                    )
                })
        }

        #[ink(message)]
        pub fn get_competition_report(
            &self,
            competition_id: u64,
        ) -> Option<(String, u128, u64, u64)> {
            self.trading_competitions
                .get(competition_id)
                .map(|competition| {
                    (
                        competition.title,
                        competition.reward_amount,
                        competition.start_block,
                        competition.end_block,
                    )
                })
        }

        #[ink(message)]
        pub fn get_competition_metadata(&self, competition_id: u64) -> Option<String> {
            self.trading_competitions
                .get(competition_id)
                .map(|competition| competition.title)
        }

        #[ink(message)]
        pub fn get_competition_summary_for_user(
            &self,
            competition_id: u64,
            trader: AccountId,
        ) -> Option<(u128, bool)> {
            let score = self.get_competition_score(competition_id, trader);
            let claimed = self.is_competition_reward_claimed(competition_id, trader);
            if score == 0 {
                None
            } else {
                Some((score, claimed))
            }
        }

        #[ink(message)]
        pub fn get_competition_summary_all(&self) -> Vec<(u64, bool, u128)> {
            let mut list = Vec::new();
            for competition_id in 1..=self.trade_competition_counter {
                if let Some(comp) = self.trading_competitions.get(competition_id) {
                    list.push((competition_id, comp.active, comp.reward_amount));
                }
            }
            list
        }

        #[ink(message)]
        pub fn get_competition_final_scores(&self, competition_id: u64) -> Vec<(AccountId, u128)> {
            self.get_competition_leaderboard(competition_id)
        }

        #[ink(message)]
        pub fn get_competition_trade_volume_goal(&self, competition_id: u64) -> Option<u128> {
            self.trading_competitions
                .get(competition_id)
                .map(|competition| competition.min_trade_volume)
        }

        #[ink(message)]
        pub fn get_competition_reward_distribution(
            &self,
            competition_id: u64,
        ) -> Option<(u128, u32)> {
            self.trading_competitions
                .get(competition_id)
                .map(|competition| (competition.reward_amount, competition.top_n))
        }

        #[ink(message)]
        pub fn get_competition_details_for_dashboard(
            &self,
            competition_id: u64,
        ) -> Option<(String, bool, u128)> {
            self.trading_competitions
                .get(competition_id)
                .map(|competition| {
                    (
                        competition.title,
                        competition.active,
                        competition.reward_amount,
                    )
                })
        }

        #[ink(message)]
        pub fn get_competition_history(&self) -> Vec<TradingCompetition> {
            self.get_all_competitions()
        }

        #[ink(message)]
        pub fn get_competition_description(&self, competition_id: u64) -> Option<String> {
            self.trading_competitions
                .get(competition_id)
                .map(|competition| competition.title)
        }

        #[ink(message)]
        pub fn get_competition_rank(&self, competition_id: u64, trader: AccountId) -> Option<u64> {
            let mut leaderboard = self.get_competition_leaderboard(competition_id);
            leaderboard.sort_by(|a, b| b.1.cmp(&a.1));
            for (idx, (account, _score)) in leaderboard.iter().enumerate() {
                if *account == trader {
                    return Some((idx + 1) as u64);
                }
            }
            None
        }

        #[ink(message)]
        pub fn get_competition_top_scores(&self, competition_id: u64) -> Vec<(AccountId, u128)> {
            self.get_competition_leaderboard(competition_id)
        }

        #[ink(message)]
        pub fn get_competition_active_status(&self, competition_id: u64) -> bool {
            self.is_competition_active(competition_id)
        }

        #[ink(message)]
        pub fn get_competition_admin(&self, _competition_id: u64) -> AccountId {
            self.admin
        }

        #[ink(message)]
        pub fn claim_liquidity_rewards(&mut self, pair_id: u64) -> Result<u128, Error> {
            non_reentrant!(self, {
                self.accrue_rewards(pair_id)?;
                let caller = self.env().caller();
                let pool = self.pool(pair_id)?;
                let mut position = self.position(pair_id, caller);
                let accrued = pending_from_indices(
                    position.lp_shares,
                    pool.reward_index,
                    position.reward_debt,
                );
                let reward = position.pending_rewards.saturating_add(accrued);
                if reward == 0 {
                    return Err(Error::RewardUnavailable);
                }
                position.pending_rewards = 0;
                position.reward_debt = scaled_reward_debt(position.lp_shares, pool.reward_index);
                self.positions.insert((pair_id, caller), &position);
                let balance = self.governance_balances.get(caller).unwrap_or(0);
                self.governance_balances
                    .insert(caller, &balance.saturating_add(reward));
                self.governance_config.total_supply =
                    self.governance_config.total_supply.saturating_add(reward);
                Ok(reward)
            })
        }

        #[ink(message)]
        pub fn create_governance_proposal(
            &mut self,
            title: String,
            description_hash: [u8; 32],
            new_fee_bips: Option<u32>,
            new_emission_rate: Option<u128>,
            duration_blocks: u64,
        ) -> Result<u64, Error> {
            non_reentrant!(self, {
                let caller = self.env().caller();
                let balance = self.governance_balances.get(caller).unwrap_or(0);
                if balance == 0 {
                    return Err(Error::InsufficientGovernanceBalance);
                }
                self.proposal_counter += 1;
                let start_block = u64::from(self.env().block_number());
                let proposal_id = self.proposal_counter;
                self.governance_proposals.insert(
                    proposal_id,
                    &GovernanceProposal {
                        proposal_id,
                        proposer: caller,
                        title,
                        description_hash,
                        new_fee_bips,
                        new_emission_rate,
                        votes_for: 0,
                        votes_against: 0,
                        start_block,
                        end_block: start_block.saturating_add(duration_blocks),
                        executed: false,
                    },
                );
                Ok(proposal_id)
            })
        }

        #[ink(message)]
        pub fn vote_on_proposal(&mut self, proposal_id: u64, support: bool) -> Result<(), Error> {
            non_reentrant!(self, {
                let caller = self.env().caller();
                if self.votes_cast.get((proposal_id, caller)).unwrap_or(false) {
                    return Err(Error::AlreadyVoted);
                }
                let mut proposal = self
                    .governance_proposals
                    .get(proposal_id)
                    .ok_or(Error::ProposalNotFound)?;
                let current_block = u64::from(self.env().block_number());
                if current_block > proposal.end_block || proposal.executed {
                    return Err(Error::ProposalClosed);
                }
                let voting_power = self.governance_balances.get(caller).unwrap_or(0);
                if support {
                    proposal.votes_for = proposal.votes_for.saturating_add(voting_power);
                } else {
                    proposal.votes_against = proposal.votes_against.saturating_add(voting_power);
                }
                self.governance_proposals.insert(proposal_id, &proposal);
                self.votes_cast.insert((proposal_id, caller), &true);
                Ok(())
            })
        }

        #[ink(message)]
        pub fn execute_governance_proposal(&mut self, proposal_id: u64) -> Result<bool, Error> {
            non_reentrant!(self, {
                let mut proposal = self
                    .governance_proposals
                    .get(proposal_id)
                    .ok_or(Error::ProposalNotFound)?;
                if proposal.executed {
                    return Err(Error::ProposalClosed);
                }
                let current_block = u64::from(self.env().block_number());
                if current_block <= proposal.end_block {
                    return Err(Error::ProposalClosed);
                }
                let passed = proposal.votes_for > proposal.votes_against;
                proposal.executed = true;
                self.governance_proposals.insert(proposal_id, &proposal);
                if passed {
                    if let Some(new_fee) = proposal.new_fee_bips {
                        self.apply_fee_to_all_pools(new_fee)?;
                    }
                }
                Ok(passed)
            })
        }

        #[ink(message)]
        pub fn get_pool(&self, pair_id: u64) -> Option<LiquidityPool> {
            self.pools.get(pair_id)
        }

        #[ink(message)]
        pub fn get_order(&self, order_id: u64) -> Option<TradingOrder> {
            self.orders.get(order_id)
        }

        #[ink(message)]
        pub fn get_pair_analytics(&self, pair_id: u64) -> Option<PairAnalytics> {
            self.analytics.get(pair_id)
        }

        #[ink(message)]
        pub fn discover_price(&self, pair_id: u64) -> Result<u128, Error> {
            let analytics = self.analytics_for(pair_id);
            let midpoint = if analytics.best_bid > 0 && analytics.best_ask > 0 {
                analytics.best_bid.saturating_add(analytics.best_ask) / 2
            } else {
                analytics.last_price
            };
            Ok(weighted_average(
                analytics.last_price,
                midpoint.max(analytics.reference_price),
                6,
                4,
            ))
        }

        #[ink(message)]
        pub fn get_portfolio_snapshot(&self, account: AccountId) -> PortfolioSnapshot {
            let mut liquidity_positions = 0u64;
            let mut pending_rewards = 0u128;
            let mut estimated_inventory_value = 0u128;
            for pair_id in 1..=self.pair_counter {
                let pool = match self.pools.get(pair_id) {
                    Some(pool) => pool,
                    None => continue,
                };
                let position = self.position(pair_id, account);
                if position.lp_shares > 0 {
                    liquidity_positions = liquidity_positions.saturating_add(1);
                    pending_rewards = pending_rewards.saturating_add(position.pending_rewards);
                    if pool.total_lp_shares > 0 {
                        estimated_inventory_value = estimated_inventory_value.saturating_add(
                            position
                                .lp_shares
                                .saturating_mul(pool.reserve_quote)
                                .checked_div(pool.total_lp_shares)
                                .unwrap_or(0),
                        );
                    }
                }
            }

            let mut open_orders = 0u64;
            for order_id in 1..=self.order_counter {
                if let Some(order) = self.orders.get(order_id) {
                    if order.trader == account
                        && matches!(
                            order.status,
                            OrderStatus::Open
                                | OrderStatus::PartiallyFilled
                                | OrderStatus::Triggered
                        )
                    {
                        open_orders = open_orders.saturating_add(1);
                    }
                }
            }

            let mut cross_chain_positions = 0u64;
            for trade_id in 1..=self.cross_chain_trade_counter {
                if let Some(trade) = self.cross_chain_trades.get(trade_id) {
                    if trade.trader == account
                        && !matches!(
                            trade.status,
                            CrossChainTradeStatus::Settled | CrossChainTradeStatus::Cancelled
                        )
                    {
                        cross_chain_positions = cross_chain_positions.saturating_add(1);
                    }
                }
            }

            PortfolioSnapshot {
                owner: account,
                liquidity_positions,
                open_orders,
                pending_rewards,
                governance_balance: self.governance_balances.get(account).unwrap_or(0),
                estimated_inventory_value,
                cross_chain_positions,
            }
        }

        /// Calculate the expected price impact for a given trade amount.
        /// Returns the price impact in basis points (bips) and the expected output amount.
        /// This allows users to check the impact before executing a trade.
        #[ink(message)]
        pub fn calculate_price_impact(
            &self,
            pair_id: u64,
            side: OrderSide,
            amount_in: u128,
        ) -> Result<(u32, u128), Error> {
            if amount_in == 0 {
                return Err(Error::InvalidOrder);
            }
            let pool = self.pool(pair_id)?;
            let fee_adjusted_in = amount_in
                .saturating_mul(BIPS_DENOMINATOR.saturating_sub(pool.fee_bips as u128))
                .checked_div(BIPS_DENOMINATOR)
                .unwrap_or(0);

            let (reserve_in, reserve_out) = match side {
                OrderSide::Sell => (pool.reserve_base, pool.reserve_quote),
                OrderSide::Buy => (pool.reserve_quote, pool.reserve_base),
            };
            if reserve_in == 0 || reserve_out == 0 {
                return Err(Error::InsufficientLiquidity);
            }

            let amount_out = fee_adjusted_in
                .saturating_mul(reserve_out)
                .checked_div(reserve_in.saturating_add(fee_adjusted_in))
                .unwrap_or(0);

            let price_before = reserve_out
                .saturating_mul(BIPS_DENOMINATOR)
                .checked_div(reserve_in)
                .unwrap_or(0);

            let reserve_in_after = reserve_in.saturating_add(amount_in);
            let reserve_out_after = reserve_out.saturating_sub(amount_out);
            let price_after = if reserve_in_after > 0 {
                reserve_out_after
                    .saturating_mul(BIPS_DENOMINATOR)
                    .checked_div(reserve_in_after)
                    .unwrap_or(0)
            } else {
                0
            };

            let price_impact_bips = if price_before > 0 {
                price_before
                    .abs_diff(price_after)
                    .saturating_mul(BIPS_DENOMINATOR)
                    .checked_div(price_before)
                    .unwrap_or(0) as u32
            } else {
                0
            };

            Ok((price_impact_bips, amount_out))
        }

        #[ink(message)]
        pub fn get_governance_balance(&self, account: AccountId) -> u128 {
            self.governance_balances.get(account).unwrap_or(0)
        }

        #[ink(message)]
        pub fn get_admin_timelock_delay(&self) -> u64 {
            self.admin_timelock_delay
        }

        #[ink(message)]
        pub fn set_admin_timelock_delay(&mut self, delay_blocks: u64) -> Result<(), Error> {
            if self.env().caller() != self.admin {
                return Err(Error::Unauthorized);
            }
            if self.admin_timelock_delay > 0 {
                return Err(Error::TimelockRequired);
            }
            self.admin_timelock_delay = delay_blocks;
            Ok(())
        }

        #[ink(message)]
        pub fn schedule_bridge_route_update(
            &mut self,
            destination_chain: ChainId,
            gas_estimate: u64,
            protocol_fee: u128,
        ) -> Result<u64, Error> {
            let payload = AdminActionPayload {
                destination_chain,
                gas_estimate,
                protocol_fee,
                ..empty_admin_action_payload()
            };
            self.schedule_admin_action_internal(AdminActionKind::ConfigureBridgeRoute, payload)
        }

        #[ink(message)]
        pub fn schedule_liquidity_mining_update(
            &mut self,
            emission_rate: u128,
            start_block: u64,
            end_block: u64,
            reward_token_symbol: String,
        ) -> Result<u64, Error> {
            let payload = AdminActionPayload {
                emission_rate,
                start_block,
                end_block,
                reward_token_symbol,
                ..empty_admin_action_payload()
            };
            self.schedule_admin_action_internal(AdminActionKind::SetLiquidityMining, payload)
        }

        #[ink(message)]
        pub fn schedule_timelock_delay_update(&mut self, delay_blocks: u64) -> Result<u64, Error> {
            let payload = AdminActionPayload {
                timelock_delay_blocks: delay_blocks,
                ..empty_admin_action_payload()
            };
            self.schedule_admin_action_internal(AdminActionKind::UpdateTimelockDelay, payload)
        }

        #[ink(message)]
        pub fn execute_admin_action(&mut self, action_id: u64) -> Result<(), Error> {
            if self.env().caller() != self.admin {
                return Err(Error::Unauthorized);
            }
            let mut action = self
                .pending_admin_actions
                .get(action_id)
                .ok_or(Error::AdminActionNotFound)?;
            if !matches!(action.status, AdminActionStatus::Scheduled) {
                return Err(Error::AdminActionAlreadyFinalized);
            }
            let current_block = u64::from(self.env().block_number());
            if current_block < action.executable_at {
                return Err(Error::TimelockActive);
            }
            match action.kind {
                AdminActionKind::ConfigureBridgeRoute => {
                    self.apply_configure_bridge_route(
                        action.payload.destination_chain,
                        action.payload.gas_estimate,
                        action.payload.protocol_fee,
                    );
                }
                AdminActionKind::SetLiquidityMining => {
                    self.apply_set_liquidity_mining(
                        action.payload.emission_rate,
                        action.payload.start_block,
                        action.payload.end_block,
                        action.payload.reward_token_symbol.clone(),
                    );
                }
                AdminActionKind::UpdateTimelockDelay => {
                    self.admin_timelock_delay = action.payload.timelock_delay_blocks;
                }
            }
            action.status = AdminActionStatus::Executed;
            let kind = action.kind;
            self.pending_admin_actions.insert(action_id, &action);
            self.env()
                .emit_event(AdminActionExecuted { action_id, kind });
            Ok(())
        }

        #[ink(message)]
        pub fn cancel_admin_action(&mut self, action_id: u64) -> Result<(), Error> {
            if self.env().caller() != self.admin {
                return Err(Error::Unauthorized);
            }
            let mut action = self
                .pending_admin_actions
                .get(action_id)
                .ok_or(Error::AdminActionNotFound)?;
            if !matches!(action.status, AdminActionStatus::Scheduled) {
                return Err(Error::AdminActionAlreadyFinalized);
            }
            action.status = AdminActionStatus::Cancelled;
            self.pending_admin_actions.insert(action_id, &action);
            self.env().emit_event(AdminActionCancelled { action_id });
            Ok(())
        }

        #[ink(message)]
        pub fn get_scheduled_admin_action(&self, action_id: u64) -> Option<PendingAdminAction> {
            self.pending_admin_actions.get(action_id)
        }

        fn schedule_admin_action_internal(
            &mut self,
            kind: AdminActionKind,
            payload: AdminActionPayload,
        ) -> Result<u64, Error> {
            let caller = self.env().caller();
            if caller != self.admin {
                return Err(Error::Unauthorized);
            }
            self.pending_admin_action_counter = self.pending_admin_action_counter.saturating_add(1);
            let action_id = self.pending_admin_action_counter;
            let scheduled_at = u64::from(self.env().block_number());
            let executable_at = scheduled_at.saturating_add(self.admin_timelock_delay);
            let action = PendingAdminAction {
                action_id,
                kind,
                payload,
                proposer: caller,
                scheduled_at,
                executable_at,
                status: AdminActionStatus::Scheduled,
            };
            self.pending_admin_actions.insert(action_id, &action);
            self.env().emit_event(AdminActionScheduled {
                action_id,
                proposer: caller,
                kind,
                executable_at,
            });
            Ok(action_id)
        }

        fn apply_configure_bridge_route(
            &mut self,
            destination_chain: ChainId,
            gas_estimate: u64,
            protocol_fee: u128,
        ) {
            self.bridge_quotes.insert(
                destination_chain,
                &BridgeFeeQuote {
                    destination_chain,
                    gas_estimate,
                    protocol_fee,
                    total_fee: protocol_fee.saturating_add(gas_estimate as u128),
                },
            );
        }

        fn apply_set_liquidity_mining(
            &mut self,
            emission_rate: u128,
            start_block: u64,
            end_block: u64,
            reward_token_symbol: String,
        ) {
            self.liquidity_mining = LiquidityMiningCampaign {
                emission_rate,
                start_block,
                end_block,
                reward_token_symbol,
            };
            self.governance_config.emission_rate = emission_rate;
        }

        #[ink(message)]
        pub fn get_order_book_snapshot(
            &self,
            pair_id: u64,
            max_levels: u32,
        ) -> Result<OrderBookSnapshot, Error> {
            let _ = self.pool(pair_id)?;
            let bids = self.collect_order_book_levels(pair_id, OrderSide::Buy, max_levels);
            let asks = self.collect_order_book_levels(pair_id, OrderSide::Sell, max_levels);

            let best_bid = bids.first().map(|level| level.price).unwrap_or(0);
            let best_ask = asks.first().map(|level| level.price).unwrap_or(0);
            let spread = if best_bid > 0 && best_ask > best_bid {
                best_ask - best_bid
            } else {
                0
            };
            let mid_price = if best_bid > 0 && best_ask > 0 {
                best_bid.saturating_add(best_ask) / 2
            } else if best_bid > 0 {
                best_bid
            } else {
                best_ask
            };
            let total_bid_depth = bids
                .iter()
                .fold(0u128, |acc, level| acc.saturating_add(level.total_amount));
            let total_ask_depth = asks
                .iter()
                .fold(0u128, |acc, level| acc.saturating_add(level.total_amount));
            let analytics = self.analytics_for(pair_id);

            Ok(OrderBookSnapshot {
                pair_id,
                bids,
                asks,
                best_bid,
                best_ask,
                spread,
                mid_price,
                total_bid_depth,
                total_ask_depth,
                last_price: analytics.last_price,
                last_updated: analytics.last_updated,
            })
        }

        #[ink(message)]
        pub fn get_order_book_levels(
            &self,
            pair_id: u64,
            side: OrderSide,
            max_levels: u32,
        ) -> Result<Vec<OrderBookLevel>, Error> {
            let _ = self.pool(pair_id)?;
            Ok(self.collect_order_book_levels(pair_id, side, max_levels))
        }

        fn collect_order_book_levels(
            &self,
            pair_id: u64,
            side: OrderSide,
            max_levels: u32,
        ) -> Vec<OrderBookLevel> {
            let count = self.order_book_count.get(pair_id).unwrap_or(0);
            let mut levels: Vec<OrderBookLevel> = Vec::new();
            for idx in 0..count {
                let order_id = match self.order_book.get((pair_id, idx)) {
                    Some(order_id) => order_id,
                    None => continue,
                };
                let order = match self.orders.get(order_id) {
                    Some(order) => order,
                    None => continue,
                };
                if order.side != side {
                    continue;
                }
                if !matches!(
                    order.status,
                    OrderStatus::Open | OrderStatus::PartiallyFilled | OrderStatus::Triggered
                ) {
                    continue;
                }
                if order.remaining_amount == 0 || order.price == 0 {
                    continue;
                }
                if let Some(existing) = levels.iter_mut().find(|level| level.price == order.price) {
                    existing.total_amount =
                        existing.total_amount.saturating_add(order.remaining_amount);
                    existing.order_count = existing.order_count.saturating_add(1);
                } else {
                    levels.push(OrderBookLevel {
                        price: order.price,
                        total_amount: order.remaining_amount,
                        order_count: 1,
                        cumulative_amount: 0,
                        side,
                    });
                }
            }
            match side {
                OrderSide::Buy => levels.sort_by(|a, b| b.price.cmp(&a.price)),
                OrderSide::Sell => levels.sort_by(|a, b| a.price.cmp(&b.price)),
            }
            if max_levels > 0 && (max_levels as usize) < levels.len() {
                levels.truncate(max_levels as usize);
            }
            let mut cumulative = 0u128;
            for level in levels.iter_mut() {
                cumulative = cumulative.saturating_add(level.total_amount);
                level.cumulative_amount = cumulative;
            }
            levels
        }

        /// Get comprehensive trading statistics across all pairs
        #[ink(message)]
        pub fn get_trading_statistics(&self) -> TradingStatistics {
            let mut total_volume_24h = 0u128;
            let mut total_trades_24h = 0u64;
            let mut most_active_pair = None;
            let mut highest_volume_pair = None;
            let mut max_trades = 0u64;
            let mut max_volume = 0u128;
            let mut total_volatility = 0u32;
            let mut pairs_with_volatility = 0u32;

            for pair_id in 1..=self.pair_counter {
                if let Some(analytics) = self.analytics.get(pair_id) {
                    total_volume_24h = total_volume_24h.saturating_add(analytics.volume_24h);
                    total_trades_24h = total_trades_24h.saturating_add(analytics.trade_count_24h);
                    total_volatility = total_volatility.saturating_add(analytics.volatility_bips);
                    pairs_with_volatility = pairs_with_volatility.saturating_add(1);

                    if analytics.trade_count_24h > max_trades {
                        max_trades = analytics.trade_count_24h;
                        most_active_pair = Some(pair_id);
                    }

                    if analytics.volume_24h > max_volume {
                        max_volume = analytics.volume_24h;
                        highest_volume_pair = Some(pair_id);
                    }
                }
            }

            let average_volatility_bips = if pairs_with_volatility > 0 {
                total_volatility / pairs_with_volatility
            } else {
                0
            };

            TradingStatistics {
                total_pairs: self.pair_counter,
                total_volume_24h,
                total_trades_24h,
                most_active_pair,
                highest_volume_pair,
                average_volatility_bips,
            }
        }

        /// Get price history summary for a trading pair
        #[ink(message)]
        pub fn get_price_history(&self, pair_id: u64) -> Option<PriceHistory> {
            let analytics = self.analytics.get(pair_id)?;
            Some(PriceHistory {
                pair_id,
                current_price: analytics.last_price,
                high_24h: analytics.high_24h,
                low_24h: analytics.low_24h,
                twap_price: analytics.twap_price,
                reference_price: analytics.reference_price,
                volatility_bips: analytics.volatility_bips,
            })
        }

        /// Get volume analytics for a trading pair
        #[ink(message)]
        pub fn get_volume_analytics(&self, pair_id: u64) -> Option<VolumeAnalytics> {
            let analytics = self.analytics.get(pair_id)?;
            let pool = self.pools.get(pair_id)?;

            Some(VolumeAnalytics {
                pair_id,
                volume_24h: analytics.volume_24h,
                cumulative_volume: analytics.cumulative_volume,
                trade_count_24h: analytics.trade_count_24h,
                total_trade_count: analytics.trade_count,
                liquidity_base: pool.reserve_base,
                liquidity_quote: pool.reserve_quote,
            })
        }

        /// Get analytics for all trading pairs
        #[ink(message)]
        pub fn get_all_pair_analytics(&self) -> Vec<PairAnalytics> {
            let mut analytics_list = Vec::new();
            for pair_id in 1..=self.pair_counter {
                if let Some(analytics) = self.analytics.get(pair_id) {
                    analytics_list.push(analytics);
                }
            }
            analytics_list
        }

        fn swap(
            &mut self,
            pair_id: u64,
            side: OrderSide,
            amount_in: u128,
            min_amount_out: u128,
        ) -> Result<u128, Error> {
            if amount_in == 0 {
                return Err(Error::InvalidOrder);
            }
            self.accrue_rewards(pair_id)?;
            let mut pool = self.pool(pair_id)?;
            let caller = self.env().caller();
            let fee_adjusted_in = amount_in
                .saturating_mul(BIPS_DENOMINATOR.saturating_sub(pool.fee_bips as u128))
                .checked_div(BIPS_DENOMINATOR)
                .unwrap_or(0);

            let (reserve_in, reserve_out) = match side {
                OrderSide::Sell => (pool.reserve_base, pool.reserve_quote),
                OrderSide::Buy => (pool.reserve_quote, pool.reserve_base),
            };
            if reserve_in == 0 || reserve_out == 0 {
                return Err(Error::InsufficientLiquidity);
            }

            let amount_out = fee_adjusted_in
                .saturating_mul(reserve_out)
                .checked_div(reserve_in.saturating_add(fee_adjusted_in))
                .unwrap_or(0);
            if amount_out == 0 || amount_out < min_amount_out {
                return Err(Error::SlippageExceeded);
            }

            // Calculate price impact before executing the trade
            let price_before = if reserve_in > 0 {
                reserve_out
                    .saturating_mul(BIPS_DENOMINATOR)
                    .checked_div(reserve_in)
                    .unwrap_or(0)
            } else {
                0
            };

            let reserve_in_after = reserve_in.saturating_add(amount_in);
            let reserve_out_after = reserve_out.saturating_sub(amount_out);
            let price_after = if reserve_in_after > 0 {
                reserve_out_after
                    .saturating_mul(BIPS_DENOMINATOR)
                    .checked_div(reserve_in_after)
                    .unwrap_or(0)
            } else {
                0
            };

            let price_impact_bips = if price_before > 0 {
                price_before
                    .abs_diff(price_after)
                    .saturating_mul(BIPS_DENOMINATOR)
                    .checked_div(price_before)
                    .unwrap_or(0) as u32
            } else {
                0
            };

            // Emit price impact warning if impact exceeds 3% (300 bips)
            if price_impact_bips > 300 {
                self.env().emit_event(PriceImpactWarning {
                    pair_id,
                    trader: caller,
                    price_impact_bips,
                    amount_in,
                });
            }

            match side {
                OrderSide::Sell => {
                    pool.reserve_base = pool.reserve_base.saturating_add(amount_in);
                    pool.reserve_quote = pool.reserve_quote.saturating_sub(amount_out);
                }
                OrderSide::Buy => {
                    pool.reserve_quote = pool.reserve_quote.saturating_add(amount_in);
                    pool.reserve_base = pool.reserve_base.saturating_sub(amount_out);
                }
            }
            pool.cumulative_volume = pool.cumulative_volume.saturating_add(amount_in);
            self.update_pool_price(&mut pool);
            self.pools.insert(pair_id, &pool);

            let mut analytics = self.analytics_for(pair_id);
            let previous = analytics.last_price;
            analytics.last_price = pool.last_price;
            analytics.twap_price =
                weighted_average(analytics.last_price, analytics.twap_price, 2, 1);
            analytics.reference_price =
                self.reference_price_from_book(pair_id, analytics.last_price);
            analytics.cumulative_volume = analytics.cumulative_volume.saturating_add(amount_in);
            analytics.trade_count = analytics.trade_count.saturating_add(1);
            analytics.volatility_bips = volatility_bips(previous, analytics.last_price);
            analytics.last_updated = self.env().block_timestamp();

            // Update 24h statistics
            if analytics.high_24h == 0 || pool.last_price > analytics.high_24h {
                analytics.high_24h = pool.last_price;
            }
            if analytics.low_24h == 0 || pool.last_price < analytics.low_24h {
                analytics.low_24h = pool.last_price;
            }
            analytics.volume_24h = analytics.volume_24h.saturating_add(amount_in);
            analytics.trade_count_24h = analytics.trade_count_24h.saturating_add(1);

            self.analytics.insert(pair_id, &analytics);
            self.refresh_best_quotes(pair_id);

            let reward = amount_in
                .saturating_mul(self.liquidity_mining.emission_rate)
                .checked_div(1_000)
                .unwrap_or(0);
            let gov = self.governance_balances.get(caller).unwrap_or(0);
            self.governance_balances
                .insert(caller, &gov.saturating_add(reward));
            self.governance_config.total_supply =
                self.governance_config.total_supply.saturating_add(reward);

            self.env().emit_event(SwapExecuted {
                pair_id,
                trader: caller,
                amount_in,
                amount_out,
            });

            self.update_trade_competition_score(pair_id, caller, amount_out);
            // After swap, check for executable limit orders
            self.process_executable_limit_orders(pair_id)?;

            Ok(amount_out)
        }

        fn get_competition_leaderboard(&self, competition_id: u64) -> Vec<(AccountId, u128)> {
            let participants = self
                .competition_participants
                .get(competition_id)
                .unwrap_or_default();
            participants
                .into_iter()
                .map(|account| {
                    let score = self
                        .competition_scores
                        .get((competition_id, account))
                        .unwrap_or(0);
                    (account, score)
                })
                .collect()
        }

        fn is_competition_reward_claimed(&self, competition_id: u64, trader: AccountId) -> bool {
            self.competition_claimed
                .get((competition_id, trader))
                .unwrap_or(false)
        }

        fn get_competition_score(&self, competition_id: u64, trader: AccountId) -> u128 {
            self.competition_scores
                .get((competition_id, trader))
                .unwrap_or(0)
        }

        fn is_competition_active(&self, competition_id: u64) -> bool {
            self.trading_competitions
                .get(competition_id)
                .map(|c| c.active)
                .unwrap_or(false)
        }

        fn update_trade_competition_score(
            &mut self,
            pair_id: u64,
            trader: AccountId,
            volume: u128,
        ) {
            for competition_id in 1..=self.trade_competition_counter {
                if let Some(competition) = self.trading_competitions.get(competition_id) {
                    if !competition.active {
                        continue;
                    }
                    if competition.pair_id.is_some_and(|p| p != pair_id) {
                        continue;
                    }
                    let mut participants = self
                        .competition_participants
                        .get(competition_id)
                        .unwrap_or_default();
                    if !participants.contains(&trader) {
                        participants.push(trader);
                        self.competition_participants
                            .insert(competition_id, &participants);
                    }
                    let current = self
                        .competition_scores
                        .get((competition_id, trader))
                        .unwrap_or(0);
                    let new_score = current.saturating_add(volume);
                    self.competition_scores
                        .insert((competition_id, trader), &new_score);
                    self.env().emit_event(CompetitionScoreUpdated {
                        competition_id,
                        trader,
                        score: new_score,
                    });
                }
            }
        }

        fn get_all_competitions(&self) -> Vec<TradingCompetition> {
            let mut competitions = Vec::new();
            for competition_id in 1..=self.trade_competition_counter {
                if let Some(comp) = self.trading_competitions.get(competition_id) {
                    competitions.push(comp);
                }
            }
            competitions
        }

        fn is_order_executable(&self, order: &TradingOrder) -> Result<bool, Error> {
            let discovered = self.discover_price(order.pair_id)?;
            let triggered = match order.order_type {
                OrderType::Market | OrderType::Limit => true,
                OrderType::StopLoss => match order.side {
                    OrderSide::Sell => discovered <= order.trigger_price.unwrap_or(order.price),
                    OrderSide::Buy => discovered >= order.trigger_price.unwrap_or(order.price),
                },
                OrderType::TakeProfit => match order.side {
                    OrderSide::Sell => discovered >= order.trigger_price.unwrap_or(order.price),
                    OrderSide::Buy => discovered <= order.trigger_price.unwrap_or(order.price),
                },
                OrderType::Twap => true,
            };
            if !triggered {
                return Ok(false);
            }
            Ok(match order.order_type {
                OrderType::Market
                | OrderType::Twap
                | OrderType::StopLoss
                | OrderType::TakeProfit => true,
                _ => match order.side {
                    OrderSide::Buy => discovered <= order.price,
                    OrderSide::Sell => discovered >= order.price,
                },
            })
        }

        fn accrue_rewards(&mut self, pair_id: u64) -> Result<(), Error> {
            let mut pool = self.pool(pair_id)?;
            if pool.total_lp_shares == 0 {
                return Ok(());
            }
            let current_block = u64::from(self.env().block_number());
            let last_block = self.last_reward_block.get(pair_id).unwrap_or(current_block);
            let start = core::cmp::max(last_block, self.liquidity_mining.start_block);
            let end = core::cmp::min(current_block, self.liquidity_mining.end_block);
            if end <= start {
                self.last_reward_block.insert(pair_id, &current_block);
                return Ok(());
            }
            let blocks = (end - start) as u128;
            let total_reward = blocks.saturating_mul(self.liquidity_mining.emission_rate);
            let increment = total_reward
                .saturating_mul(REWARD_PRECISION)
                .checked_div(pool.total_lp_shares)
                .unwrap_or(0);
            pool.reward_index = pool.reward_index.saturating_add(increment);
            self.pools.insert(pair_id, &pool);
            self.last_reward_block.insert(pair_id, &current_block);
            Ok(())
        }

        fn apply_fee_to_all_pools(&mut self, new_fee_bips: u32) -> Result<(), Error> {
            if new_fee_bips >= 1_000 {
                return Err(Error::InvalidPair);
            }
            for pair_id in 1..=self.pair_counter {
                if let Some(mut pool) = self.pools.get(pair_id) {
                    pool.fee_bips = new_fee_bips;
                    self.pools.insert(pair_id, &pool);
                }
            }
            Ok(())
        }

        fn refresh_best_quotes(&mut self, pair_id: u64) {
            let count = self.order_book_count.get(pair_id).unwrap_or(0);
            let mut best_bid = 0u128;
            let mut best_ask = 0u128;
            for idx in 0..count {
                let order_id = match self.order_book.get((pair_id, idx)) {
                    Some(order_id) => order_id,
                    None => continue,
                };
                let order = match self.orders.get(order_id) {
                    Some(order) => order,
                    None => continue,
                };
                if !matches!(
                    order.status,
                    OrderStatus::Open | OrderStatus::PartiallyFilled | OrderStatus::Triggered
                ) {
                    continue;
                }
                match order.side {
                    OrderSide::Buy => {
                        if order.price > best_bid {
                            best_bid = order.price;
                        }
                    }
                    OrderSide::Sell => {
                        if best_ask == 0 || order.price < best_ask {
                            best_ask = order.price;
                        }
                    }
                }
            }
            let mut analytics = self.analytics_for(pair_id);
            analytics.best_bid = best_bid;
            analytics.best_ask = best_ask;
            analytics.reference_price =
                self.reference_price_from_book(pair_id, analytics.last_price);
            self.analytics.insert(pair_id, &analytics);
        }

        /// Process and execute all limit orders that have become executable after a price change.
        /// This is called after each swap to ensure limit orders are filled when their price
        /// conditions are met.
        fn process_executable_limit_orders(&mut self, pair_id: u64) -> Result<(), Error> {
            let count = self.order_book_count.get(pair_id).unwrap_or(0);
            if count == 0 {
                return Ok(());
            }

            // Collect order IDs that need to be executed
            let mut orders_to_execute: Vec<u64> = Vec::new();

            for idx in 0..count {
                let order_id = match self.order_book.get((pair_id, idx)) {
                    Some(order_id) => order_id,
                    None => continue,
                };

                let order = match self.orders.get(order_id) {
                    Some(order) => order,
                    None => continue,
                };

                // Only process limit orders that are open or partially filled
                if !matches!(order.order_type, OrderType::Limit) {
                    continue;
                }

                if !matches!(
                    order.status,
                    OrderStatus::Open | OrderStatus::PartiallyFilled
                ) {
                    continue;
                }

                // Check if the limit order is now executable
                if self.is_order_executable(&order)? {
                    orders_to_execute.push(order_id);
                }
            }

            // Execute collected orders
            for order_id in orders_to_execute {
                // Reload order to get latest state (may have been partially filled by previous executions)
                let order = match self.orders.get(order_id) {
                    Some(order) => order,
                    None => continue,
                };

                if order.remaining_amount > 0
                    && matches!(
                        order.status,
                        OrderStatus::Open | OrderStatus::PartiallyFilled
                    )
                {
                    let _ = self.execute_order_core(order_id, order.remaining_amount);
                }
            }

            Ok(())
        }

        fn reference_price_from_book(&self, pair_id: u64, fallback: u128) -> u128 {
            let analytics = self.analytics_for(pair_id);
            if analytics.best_bid > 0 && analytics.best_ask > 0 {
                (analytics.best_bid.saturating_add(analytics.best_ask)) / 2
            } else {
                fallback
            }
        }

        fn update_pool_price(&self, pool: &mut LiquidityPool) {
            if pool.reserve_base > 0 {
                pool.last_price = pool
                    .reserve_quote
                    .saturating_mul(BIPS_DENOMINATOR)
                    .checked_div(pool.reserve_base)
                    .unwrap_or(pool.last_price);
            }
        }

        fn ensure_admin_or_pair_creator(&self) -> Result<(), Error> {
            let _ = self.env().caller();
            Ok(())
        }

        fn pool(&self, pair_id: u64) -> Result<LiquidityPool, Error> {
            self.pools.get(pair_id).ok_or(Error::PoolNotFound)
        }

        fn order(&self, order_id: u64) -> Result<TradingOrder, Error> {
            self.orders.get(order_id).ok_or(Error::OrderNotFound)
        }

        fn cross_chain_trade(&self, trade_id: u64) -> Result<CrossChainTradeIntent, Error> {
            self.cross_chain_trades
                .get(trade_id)
                .ok_or(Error::CrossChainTradeNotFound)
        }

        fn position(&self, pair_id: u64, account: AccountId) -> LiquidityPosition {
            self.positions
                .get((pair_id, account))
                .unwrap_or(LiquidityPosition {
                    lp_shares: 0,
                    reward_debt: 0,
                    provided_base: 0,
                    provided_quote: 0,
                    pending_rewards: 0,
                })
        }

        fn analytics_for(&self, pair_id: u64) -> PairAnalytics {
            self.analytics.get(pair_id).unwrap_or(PairAnalytics {
                pair_id,
                last_price: 0,
                twap_price: 0,
                reference_price: 0,
                cumulative_volume: 0,
                trade_count: 0,
                best_bid: 0,
                best_ask: 0,
                volatility_bips: 0,
                last_updated: 0,
                high_24h: 0,
                low_24h: 0,
                volume_24h: 0,
                trade_count_24h: 0,
            })
        }
    }

    fn empty_admin_action_payload() -> AdminActionPayload {
        AdminActionPayload {
            destination_chain: 0,
            gas_estimate: 0,
            protocol_fee: 0,
            emission_rate: 0,
            start_block: 0,
            end_block: 0,
            reward_token_symbol: String::new(),
            timelock_delay_blocks: 0,
        }
    }

    fn ordered_pair(base: TokenId, quote: TokenId) -> (TokenId, TokenId) {
        if base < quote {
            (base, quote)
        } else {
            (quote, base)
        }
    }

    fn integer_sqrt(value: u128) -> u128 {
        if value <= 1 {
            return value;
        }
        let mut x0 = value / 2;
        let mut x1 = (x0 + value / x0) / 2;
        while x1 < x0 {
            x0 = x1;
            x1 = (x0 + value / x0) / 2;
        }
        x0
    }

    fn weighted_average(a: u128, b: u128, a_weight: u128, b_weight: u128) -> u128 {
        if a_weight + b_weight == 0 {
            return 0;
        }
        a.saturating_mul(a_weight)
            .saturating_add(b.saturating_mul(b_weight))
            .checked_div(a_weight + b_weight)
            .unwrap_or(0)
    }

    fn pending_from_indices(lp_shares: u128, reward_index: u128, reward_debt: u128) -> u128 {
        lp_shares
            .saturating_mul(reward_index)
            .checked_div(REWARD_PRECISION)
            .unwrap_or(0)
            .saturating_sub(reward_debt)
    }

    fn scaled_reward_debt(lp_shares: u128, reward_index: u128) -> u128 {
        lp_shares
            .saturating_mul(reward_index)
            .checked_div(REWARD_PRECISION)
            .unwrap_or(0)
    }

    fn volatility_bips(previous: u128, current: u128) -> u32 {
        if previous == 0 || current == 0 {
            return 0;
        }
        let diff = previous.abs_diff(current);
        diff.saturating_mul(BIPS_DENOMINATOR)
            .checked_div(previous)
            .unwrap_or(0) as u32
    }

    // Include unit tests
    #[cfg(test)]
    include!("tests.rs");
}
