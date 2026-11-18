#![cfg_attr(not(feature = "std"), no_std, no_main)]
#![allow(clippy::new_without_default)]

//! # Concentrated liquidity DEX (#1435)
//!
//! Liquidity providers choose a price range `[lower_tick, upper_tick)`; only
//! positions whose range contains the pool's **current tick** take part in swaps
//! and earn fees. Each tick is a 0.1% price step: `price(tick) = 1.001^tick`
//! (token1 per token0, fixed point with 12 decimals).
//!
//! ## What this model does and does not do
//!
//! This is a deliberately **simplified** concentrated-liquidity ledger, not a
//! Uniswap-v3 port:
//!
//! * A swap executes entirely at the **current tick's price**, against the
//!   *active* positions only, and is rejected if their combined output-side
//!   reserve cannot cover it. There is **no price impact and no tick crossing
//!   inside a swap**; the price moves only when the admin/keeper calls
//!   [`ConcentratedLiquidityDex::set_current_tick`].
//! * Balances are **internal accounting**; no token contract is called. Wiring
//!   deposits/withdrawals to real tokens is left to the integration layer.
//! * A position is a non-fungible id owned by an account
//!   ([`ConcentratedLiquidityDex::transfer_position`] moves it); it is not a
//!   PSP34 token.
//! * Fees are charged on the input token and credited, at swap time, to each
//!   active position in proportion to its liquidity.
//!
//! ## Position rules
//!
//! Ticks must be multiples of `tick_spacing` and within `±MAX_TICK`. If the pool
//! is below the range only token0 may be provided; above it, only token1; inside
//! it, both. `liquidity = amount0 * price + amount1` (token1 terms).

#[ink::contract]
pub mod concentrated_liquidity_dex {
    use ink::prelude::vec::Vec;
    use ink::storage::Mapping;

    /// Fixed-point scale of `tick_to_price` (12 decimals).
    pub const PRICE_SCALE: u128 = 1_000_000_000_000;
    /// 1.001 in `PRICE_SCALE` fixed point.
    const TICK_BASE: u128 = 1_001_000_000_000;
    /// Largest `|tick|` (price range roughly 0.135x .. 7.4x).
    pub const MAX_TICK: i32 = 2_000;
    pub const MAX_POSITIONS: usize = 128;

    /// `1.001^tick` in `PRICE_SCALE` fixed point, by square-and-multiply.
    pub fn tick_to_price(tick: i32) -> u128 {
        let mut exp = tick.unsigned_abs();
        let mut base = TICK_BASE;
        let mut acc = PRICE_SCALE;
        while exp > 0 {
            if exp & 1 == 1 {
                acc = acc * base / PRICE_SCALE;
            }
            base = base * base / PRICE_SCALE;
            exp >>= 1;
        }
        if tick < 0 {
            PRICE_SCALE * PRICE_SCALE / acc
        } else {
            acc
        }
    }

    #[derive(Debug, Clone, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout))]
    pub struct Position {
        pub owner: AccountId,
        pub lower_tick: i32,
        pub upper_tick: i32,
        pub amount0: u128,
        pub amount1: u128,
        pub liquidity: u128,
        pub owed0: u128,
        pub owed1: u128,
    }

    #[derive(Debug, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub enum Error {
        Unauthorized,
        InvalidTickRange,
        TickNotAligned,
        /// Amounts do not match where the current price sits relative to the range.
        InvalidAmounts,
        PositionNotFound,
        TooManyPositions,
        ZeroAmount,
        /// The active positions cannot pay out the swap.
        InsufficientLiquidity,
        InvalidFee,
    }

    #[ink(event)]
    pub struct ConcentratedLiquidityAdded {
        #[ink(topic)]
        pub position_id: u64,
        #[ink(topic)]
        pub owner: AccountId,
        pub lower_tick: i32,
        pub upper_tick: i32,
        pub amount0: u128,
        pub amount1: u128,
    }

    #[ink(event)]
    pub struct ConcentratedLiquidityRemoved {
        #[ink(topic)]
        pub position_id: u64,
        pub amount0: u128,
        pub amount1: u128,
    }

    #[ink(event)]
    pub struct FeesCollected {
        #[ink(topic)]
        pub position_id: u64,
        pub fees0: u128,
        pub fees1: u128,
    }

    #[ink(event)]
    pub struct Swapped {
        pub zero_for_one: bool,
        pub amount_in: u128,
        pub amount_out: u128,
        pub tick: i32,
    }

    #[ink(storage)]
    pub struct ConcentratedLiquidityDex {
        admin: AccountId,
        fee_bps: u32,
        tick_spacing: i32,
        current_tick: i32,
        next_position_id: u64,
        positions: Mapping<u64, Position>,
        position_ids: Vec<u64>,
    }

    impl ConcentratedLiquidityDex {
        #[ink(constructor)]
        pub fn new(fee_bps: u32, tick_spacing: i32, initial_tick: i32) -> Self {
            assert!(fee_bps < 10_000, "fee must be below 100%");
            assert!(tick_spacing > 0 && initial_tick.abs() <= MAX_TICK);
            Self {
                admin: Self::env().caller(),
                fee_bps,
                tick_spacing,
                current_tick: initial_tick,
                next_position_id: 1,
                positions: Mapping::default(),
                position_ids: Vec::new(),
            }
        }

        // ── Queries ────────────────────────────────────────────────────

        #[ink(message)]
        pub fn get_current_tick(&self) -> i32 {
            self.current_tick
        }

        #[ink(message)]
        pub fn get_price(&self) -> u128 {
            tick_to_price(self.current_tick)
        }

        #[ink(message)]
        pub fn price_at_tick(&self, tick: i32) -> u128 {
            tick_to_price(tick)
        }

        #[ink(message)]
        pub fn get_position(&self, position_id: u64) -> Option<Position> {
            self.positions.get(position_id)
        }

        #[ink(message)]
        pub fn get_position_ids(&self) -> Vec<u64> {
            self.position_ids.clone()
        }

        /// Total liquidity of positions whose range contains the current tick.
        #[ink(message)]
        pub fn get_active_liquidity(&self) -> u128 {
            self.position_ids
                .iter()
                .filter_map(|id| self.positions.get(id))
                .filter(|p| Self::in_range(p, self.current_tick))
                .fold(0u128, |acc, p| acc.saturating_add(p.liquidity))
        }

        /// Fees `position_id` could collect right now.
        #[ink(message)]
        pub fn pending_fees(&self, position_id: u64) -> Option<(u128, u128)> {
            self.positions.get(position_id).map(|p| (p.owed0, p.owed1))
        }

        // ── Liquidity ──────────────────────────────────────────────────

        #[ink(message)]
        pub fn add_concentrated_liquidity(
            &mut self,
            lower_tick: i32,
            upper_tick: i32,
            amount0: u128,
            amount1: u128,
        ) -> Result<u64, Error> {
            if lower_tick >= upper_tick || lower_tick < -MAX_TICK || upper_tick > MAX_TICK {
                return Err(Error::InvalidTickRange);
            }
            if lower_tick % self.tick_spacing != 0 || upper_tick % self.tick_spacing != 0 {
                return Err(Error::TickNotAligned);
            }
            if amount0 == 0 && amount1 == 0 {
                return Err(Error::ZeroAmount);
            }
            // Where is the price relative to the range?
            let ok = if self.current_tick < lower_tick {
                amount1 == 0
            } else if self.current_tick >= upper_tick {
                amount0 == 0
            } else {
                amount0 > 0 && amount1 > 0
            };
            if !ok {
                return Err(Error::InvalidAmounts);
            }
            if self.position_ids.len() >= MAX_POSITIONS {
                return Err(Error::TooManyPositions);
            }
            let price = tick_to_price(self.current_tick);
            let liquidity = amount0.saturating_mul(price) / PRICE_SCALE;
            let liquidity = liquidity.saturating_add(amount1);
            let owner = self.env().caller();
            let id = self.next_position_id;
            self.next_position_id += 1;
            self.positions.insert(
                id,
                &Position {
                    owner,
                    lower_tick,
                    upper_tick,
                    amount0,
                    amount1,
                    liquidity,
                    owed0: 0,
                    owed1: 0,
                },
            );
            self.position_ids.push(id);
            self.env().emit_event(ConcentratedLiquidityAdded {
                position_id: id,
                owner,
                lower_tick,
                upper_tick,
                amount0,
                amount1,
            });
            Ok(id)
        }

        /// Close a position and return `(amount0, amount1)` plus any uncollected fees.
        #[ink(message)]
        pub fn remove_concentrated_liquidity(&mut self, position_id: u64) -> Result<(u128, u128), Error> {
            let p = self.positions.get(position_id).ok_or(Error::PositionNotFound)?;
            if p.owner != self.env().caller() {
                return Err(Error::Unauthorized);
            }
            let out0 = p.amount0.saturating_add(p.owed0);
            let out1 = p.amount1.saturating_add(p.owed1);
            self.positions.remove(position_id);
            self.position_ids.retain(|id| *id != position_id);
            self.env().emit_event(ConcentratedLiquidityRemoved { position_id, amount0: out0, amount1: out1 });
            Ok((out0, out1))
        }

        #[ink(message)]
        pub fn collect_fees(&mut self, position_id: u64) -> Result<(u128, u128), Error> {
            let mut p = self.positions.get(position_id).ok_or(Error::PositionNotFound)?;
            if p.owner != self.env().caller() {
                return Err(Error::Unauthorized);
            }
            let (f0, f1) = (p.owed0, p.owed1);
            p.owed0 = 0;
            p.owed1 = 0;
            self.positions.insert(position_id, &p);
            self.env().emit_event(FeesCollected { position_id, fees0: f0, fees1: f1 });
            Ok((f0, f1))
        }

        #[ink(message)]
        pub fn transfer_position(&mut self, position_id: u64, to: AccountId) -> Result<(), Error> {
            let mut p = self.positions.get(position_id).ok_or(Error::PositionNotFound)?;
            if p.owner != self.env().caller() {
                return Err(Error::Unauthorized);
            }
            p.owner = to;
            self.positions.insert(position_id, &p);
            Ok(())
        }

        // ── Swapping ───────────────────────────────────────────────────

        /// Swap `amount_in` at the current tick's price using active positions only.
        /// `zero_for_one` sells token0 for token1. Returns the output amount.
        #[ink(message)]
        pub fn swap(&mut self, zero_for_one: bool, amount_in: u128) -> Result<u128, Error> {
            if amount_in == 0 {
                return Err(Error::ZeroAmount);
            }
            let price = tick_to_price(self.current_tick);
            let fee = amount_in.saturating_mul(self.fee_bps as u128) / 10_000;
            let net = amount_in - fee;
            let amount_out = if zero_for_one {
                net.saturating_mul(price) / PRICE_SCALE
            } else {
                net.saturating_mul(PRICE_SCALE) / price
            };

            // Active positions and their output-side reserves.
            let mut active: Vec<(u64, Position)> = Vec::new();
            let mut reserve_out: u128 = 0;
            let mut active_liquidity: u128 = 0;
            for id in self.position_ids.iter() {
                if let Some(p) = self.positions.get(id) {
                    if Self::in_range(&p, self.current_tick) {
                        reserve_out = reserve_out.saturating_add(if zero_for_one { p.amount1 } else { p.amount0 });
                        active_liquidity = active_liquidity.saturating_add(p.liquidity);
                        active.push((*id, p));
                    }
                }
            }
            if amount_out == 0 || reserve_out < amount_out || active_liquidity == 0 {
                return Err(Error::InsufficientLiquidity);
            }

            // Move reserves: each active position gives out in proportion to its
            // output-side reserve and receives input in the same proportion; the fee
            // is credited in proportion to liquidity.
            for (id, mut p) in active {
                let own_out = if zero_for_one { p.amount1 } else { p.amount0 };
                let out_i = amount_out.saturating_mul(own_out) / reserve_out;
                let in_i = net.saturating_mul(own_out) / reserve_out;
                let fee_i = fee.saturating_mul(p.liquidity) / active_liquidity;
                if zero_for_one {
                    p.amount1 -= out_i.min(p.amount1);
                    p.amount0 = p.amount0.saturating_add(in_i);
                    p.owed0 = p.owed0.saturating_add(fee_i);
                } else {
                    p.amount0 -= out_i.min(p.amount0);
                    p.amount1 = p.amount1.saturating_add(in_i);
                    p.owed1 = p.owed1.saturating_add(fee_i);
                }
                self.positions.insert(id, &p);
            }
            self.env().emit_event(Swapped {
                zero_for_one,
                amount_in,
                amount_out,
                tick: self.current_tick,
            });
            Ok(amount_out)
        }

        /// Move the pool price. Admin only (stands in for price discovery).
        #[ink(message)]
        pub fn set_current_tick(&mut self, tick: i32) -> Result<(), Error> {
            if self.env().caller() != self.admin {
                return Err(Error::Unauthorized);
            }
            if tick.abs() > MAX_TICK {
                return Err(Error::InvalidTickRange);
            }
            self.current_tick = tick;
            Ok(())
        }

        #[ink(message)]
        pub fn set_fee_bps(&mut self, fee_bps: u32) -> Result<(), Error> {
            if self.env().caller() != self.admin {
                return Err(Error::Unauthorized);
            }
            if fee_bps >= 10_000 {
                return Err(Error::InvalidFee);
            }
            self.fee_bps = fee_bps;
            Ok(())
        }

        // ── Internals ──────────────────────────────────────────────────

        fn in_range(p: &Position, tick: i32) -> bool {
            p.lower_tick <= tick && tick < p.upper_tick
        }

    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use ink::env::test;

        type Accounts = test::DefaultAccounts<ink::env::DefaultEnvironment>;

        fn setup() -> (ConcentratedLiquidityDex, Accounts) {
            let accounts = test::default_accounts::<ink::env::DefaultEnvironment>();
            test::set_caller::<ink::env::DefaultEnvironment>(accounts.alice);
            // 0.30% fee, ticks 10 apart, price starts at tick 0 (1.0).
            (ConcentratedLiquidityDex::new(30, 10, 0), accounts)
        }

        fn as_user(who: AccountId) {
            test::set_caller::<ink::env::DefaultEnvironment>(who);
        }

        // ── Ticks ───────────────────────────────────────────────────────

        #[test]
        fn tick_zero_is_one_and_prices_step_by_a_tenth_of_a_percent() {
            assert_eq!(tick_to_price(0), PRICE_SCALE);
            let one = tick_to_price(1);
            assert!((1_000_999_000_000..=1_001_001_000_000).contains(&one), "{one}");
            assert!(tick_to_price(100) > tick_to_price(99));
            assert!(tick_to_price(-100) < tick_to_price(-99));
            // 1.001^100 is about 1.10512.
            let p100 = tick_to_price(100);
            assert!((1_105_000_000_000..=1_105_300_000_000).contains(&p100), "{p100}");
        }

        #[test]
        fn opposite_ticks_have_reciprocal_prices() {
            for tick in [1, 10, 250, 1_999] {
                let product = tick_to_price(tick) * tick_to_price(-tick) / PRICE_SCALE;
                let drift = product.abs_diff(PRICE_SCALE);
                assert!(drift * 1_000_000 <= PRICE_SCALE, "tick {tick}: {product}");
            }
        }

        // ── Providing liquidity ─────────────────────────────────────────

        #[ink::test]
        fn an_in_range_position_takes_both_tokens_and_is_active() {
            let (mut d, a) = setup();
            as_user(a.bob);
            let id = d.add_concentrated_liquidity(-100, 100, 1_000, 1_000).unwrap();
            assert_eq!(id, 1);
            let p = d.get_position(id).unwrap();
            assert_eq!((p.owner, p.lower_tick, p.upper_tick), (a.bob, -100, 100));
            // price 1.0: liquidity = 1_000 * 1 + 1_000.
            assert_eq!(p.liquidity, 2_000);
            assert_eq!(d.get_active_liquidity(), 2_000);
            assert_eq!(d.get_position_ids(), vec![1]);
        }

        #[ink::test]
        fn single_sided_positions_must_sit_on_the_correct_side_of_the_price() {
            let (mut d, a) = setup();
            as_user(a.bob);
            // Range above the price: token0 only.
            assert!(d.add_concentrated_liquidity(100, 200, 500, 0).is_ok());
            assert_eq!(d.add_concentrated_liquidity(100, 200, 500, 5), Err(Error::InvalidAmounts));
            // Range below the price: token1 only.
            assert!(d.add_concentrated_liquidity(-200, -100, 0, 500).is_ok());
            assert_eq!(d.add_concentrated_liquidity(-200, -100, 5, 500), Err(Error::InvalidAmounts));
            // In range needs both.
            assert_eq!(d.add_concentrated_liquidity(-10, 10, 500, 0), Err(Error::InvalidAmounts));
            // Out-of-range positions are not active.
            assert_eq!(d.get_active_liquidity(), 0);
        }

        #[ink::test]
        fn tick_ranges_are_validated() {
            let (mut d, a) = setup();
            as_user(a.bob);
            assert_eq!(d.add_concentrated_liquidity(10, 10, 1, 1), Err(Error::InvalidTickRange));
            assert_eq!(d.add_concentrated_liquidity(20, 10, 1, 1), Err(Error::InvalidTickRange));
            assert_eq!(d.add_concentrated_liquidity(-3_000, 10, 1, 1), Err(Error::InvalidTickRange));
            assert_eq!(d.add_concentrated_liquidity(-10, 3_000, 1, 1), Err(Error::InvalidTickRange));
            assert_eq!(d.add_concentrated_liquidity(-15, 10, 1, 1), Err(Error::TickNotAligned));
            assert_eq!(d.add_concentrated_liquidity(-10, 10, 0, 0), Err(Error::ZeroAmount));
        }

        // ── Swap routing ────────────────────────────────────────────────

        #[ink::test]
        fn swaps_only_use_positions_whose_range_contains_the_price() {
            let (mut d, a) = setup();
            as_user(a.bob);
            let active = d.add_concentrated_liquidity(-100, 100, 10_000, 10_000).unwrap();
            // A position far above the price holds token0 only and is inactive.
            let inactive = d.add_concentrated_liquidity(500, 600, 50_000, 0).unwrap();
            let before = d.get_position(inactive).unwrap();

            // Sell token0 for token1 at price 1.0 with a 0.30% fee.
            let out = d.swap(true, 1_000).unwrap();
            assert_eq!(out, 997); // 1_000 - 3 fee, at price 1.0
            assert_eq!(d.get_position(inactive).unwrap(), before);
            let p = d.get_position(active).unwrap();
            assert_eq!(p.amount1, 10_000 - 997);
            assert_eq!(p.amount0, 10_000 + 997);
        }

        #[ink::test]
        fn a_swap_larger_than_the_active_reserve_is_rejected() {
            let (mut d, a) = setup();
            as_user(a.bob);
            d.add_concentrated_liquidity(-100, 100, 1_000, 1_000).unwrap();
            assert_eq!(d.swap(true, 5_000), Err(Error::InsufficientLiquidity));
            assert_eq!(d.swap(true, 0), Err(Error::ZeroAmount));
            // No active liquidity at all.
            let (mut empty, _) = setup();
            assert_eq!(empty.swap(false, 10), Err(Error::InsufficientLiquidity));
        }

        #[ink::test]
        fn moving_the_price_changes_which_positions_are_active() {
            let (mut d, a) = setup();
            as_user(a.bob);
            d.add_concentrated_liquidity(-100, 100, 1_000, 1_000).unwrap();
            as_user(a.alice);
            d.set_current_tick(150).unwrap();
            assert_eq!(d.get_active_liquidity(), 0);
            d.set_current_tick(99).unwrap();
            assert!(d.get_active_liquidity() > 0);
            // The upper tick is exclusive.
            d.set_current_tick(100).unwrap();
            assert_eq!(d.get_active_liquidity(), 0);
            as_user(a.bob);
            assert_eq!(d.set_current_tick(0), Err(Error::Unauthorized));
        }

        // ── Fees ────────────────────────────────────────────────────────

        #[ink::test]
        fn fees_are_split_between_active_positions_by_liquidity() {
            let (mut d, a) = setup();
            as_user(a.bob);
            let small = d.add_concentrated_liquidity(-100, 100, 10_000, 10_000).unwrap(); // L = 20_000
            as_user(a.charlie);
            let large = d.add_concentrated_liquidity(-100, 100, 30_000, 30_000).unwrap(); // L = 60_000
            d.swap(true, 10_000).unwrap(); // fee = 30 token0
            // 30 split 1:3 by liquidity: 30 * 20_000 / 80_000 = 7 and 30 * 60_000 / 80_000 = 22.
            assert_eq!(d.pending_fees(small), Some((7, 0)));
            assert_eq!(d.pending_fees(large), Some((22, 0)));
        }

        #[ink::test]
        fn inactive_and_late_positions_do_not_earn_earlier_fees() {
            let (mut d, a) = setup();
            as_user(a.bob);
            d.add_concentrated_liquidity(-100, 100, 100_000, 100_000).unwrap();
            let inactive = d.add_concentrated_liquidity(500, 600, 10_000, 0).unwrap();
            d.swap(true, 10_000).unwrap(); // fee 30
            assert_eq!(d.pending_fees(inactive), Some((0, 0)));
            // A position opened after the swap starts from the current growth level.
            let late = d.add_concentrated_liquidity(-100, 100, 1_000, 1_000).unwrap();
            assert_eq!(d.pending_fees(late), Some((0, 0)));
        }

        #[ink::test]
        fn collecting_pays_accrued_fees_once() {
            let (mut d, a) = setup();
            as_user(a.bob);
            let id = d.add_concentrated_liquidity(-100, 100, 100_000, 100_000).unwrap();
            d.swap(true, 10_000).unwrap(); // fee 30 on token0
            assert_eq!(d.collect_fees(id), Ok((30, 0)));
            assert_eq!(d.collect_fees(id), Ok((0, 0)));
            d.swap(false, 10_000).unwrap(); // fee 30 on token1
            assert_eq!(d.collect_fees(id), Ok((0, 30)));
        }

        #[ink::test]
        fn only_the_owner_collects_removes_or_transfers() {
            let (mut d, a) = setup();
            as_user(a.bob);
            let id = d.add_concentrated_liquidity(-100, 100, 1_000, 1_000).unwrap();
            as_user(a.charlie);
            assert_eq!(d.collect_fees(id), Err(Error::Unauthorized));
            assert_eq!(d.remove_concentrated_liquidity(id), Err(Error::Unauthorized));
            assert_eq!(d.transfer_position(id, a.charlie), Err(Error::Unauthorized));
            as_user(a.bob);
            d.transfer_position(id, a.charlie).unwrap();
            as_user(a.charlie);
            assert!(d.collect_fees(id).is_ok());
        }

        // ── Removal ─────────────────────────────────────────────────────

        #[ink::test]
        fn removal_returns_current_amounts_plus_fees_and_deletes_the_position() {
            let (mut d, a) = setup();
            as_user(a.bob);
            let id = d.add_concentrated_liquidity(-100, 100, 100_000, 100_000).unwrap();
            d.swap(true, 10_000).unwrap(); // out 9_970, fee 30 token0
            // Reserves now: token0 100_000 + 9_970, token1 100_000 - 9_970; plus 30 fees.
            let out = d.remove_concentrated_liquidity(id).unwrap();
            assert_eq!(out, (100_000 + 9_970 + 30, 100_000 - 9_970));
            assert!(d.get_position(id).is_none());
            assert!(d.get_position_ids().is_empty());
            assert_eq!(d.remove_concentrated_liquidity(id), Err(Error::PositionNotFound));
        }

        #[ink::test]
        fn events_are_emitted_for_add_collect_and_remove() {
            let (mut d, a) = setup();
            as_user(a.bob);
            let id = d.add_concentrated_liquidity(-100, 100, 1_000, 1_000).unwrap();
            assert_eq!(test::recorded_events().count(), 1);
            d.collect_fees(id).unwrap();
            assert_eq!(test::recorded_events().count(), 2);
            d.remove_concentrated_liquidity(id).unwrap();
            assert_eq!(test::recorded_events().count(), 3);
        }
    }
}
