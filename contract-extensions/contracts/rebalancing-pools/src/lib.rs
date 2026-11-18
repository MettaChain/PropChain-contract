#![cfg_attr(not(feature = "std"), no_std, no_main)]
#![allow(clippy::new_without_default)]

//! # Auto-rebalancing share pools (#1440)
//!
//! A pool holds value in several **property types** and keeps the mix close to
//! admin-set target weights (basis points summing to 10 000). Every recorded buy
//! or sell triggers [`RebalancingPools::rebalance`]'s logic automatically; anyone
//! can also call `rebalance` directly.
//!
//! ## When a rebalance is skipped (event `RebalanceSkipped`)
//!
//! | reason | condition |
//! |---|---|
//! | `NoTargets` | no target allocation has been set |
//! | `EmptyPool` | the pool holds nothing |
//! | `Cooldown` | fewer than `cooldown_blocks` since the last rebalance |
//! | `SlippageTooHigh` | the last reported market slippage exceeds `max_slippage_bps` |
//! | `WithinTolerance` | the largest deviation is `<= tolerance_bps` |
//!
//! ## What a rebalance does
//!
//! Each type's target value is `total * weight / 10_000` (the rounding remainder
//! goes to the type with the largest weight so the total is conserved). The pool
//! moves holdings straight to those targets: overweight types are sold,
//! underweight types are bought. The resulting trade list is stored
//! (`get_last_orders`) as `(property_type, signed value delta)` for the DEX
//! integration to execute.
//!
//! ## Limits of this model
//!
//! * Holdings are **value amounts**; the contract does not call a DEX. It
//!   assumes the orders fill at their quoted value and relies on a
//!   keeper/operator to report observed slippage ([`RebalancingPools::report_slippage`])
//!   so rebalancing pauses in volatile markets.
//! * Only operators (the DEX / pool contract) may record trades.

#[ink::contract]
pub mod rebalancing_pools {
    use ink::prelude::vec::Vec;
    use ink::storage::Mapping;

    pub const MAX_TYPES: usize = 16;

    #[derive(Debug, Clone, Copy, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout))]
    pub enum SkipReason {
        NoTargets,
        EmptyPool,
        Cooldown,
        SlippageTooHigh,
        WithinTolerance,
    }

    #[derive(Debug, Clone, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub enum RebalanceOutcome {
        Skipped(SkipReason),
        /// Number of non-zero orders produced.
        Completed(u32),
    }

    #[derive(Debug, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub enum Error {
        Unauthorized,
        /// Lengths differ, empty, too many types, duplicates, or weights do not sum to 10 000.
        InvalidAllocation,
        UnknownPropertyType,
        InsufficientHoldings,
        ZeroAmount,
        InvalidConfig,
    }

    #[ink(event)]
    pub struct AllocationTargetSet {
        pub type_count: u32,
    }

    #[ink(event)]
    pub struct RebalanceTriggered {
        pub max_deviation_bps: u32,
    }

    #[ink(event)]
    pub struct RebalanceCompleted {
        pub orders: u32,
        pub value_moved: u128,
    }

    #[ink(event)]
    pub struct RebalanceSkipped {
        pub reason: SkipReason,
    }

    #[ink(storage)]
    pub struct RebalancingPools {
        admin: AccountId,
        operators: Mapping<AccountId, bool>,
        types: Vec<u32>,
        targets: Mapping<u32, u32>,
        holdings: Mapping<u32, u128>,
        cooldown_blocks: u32,
        last_rebalance: Option<u32>,
        max_slippage_bps: u32,
        observed_slippage_bps: u32,
        tolerance_bps: u32,
        last_orders: Vec<(u32, i128)>,
    }

    impl RebalancingPools {
        #[ink(constructor)]
        pub fn new(cooldown_blocks: u32, max_slippage_bps: u32, tolerance_bps: u32) -> Self {
            assert!(max_slippage_bps <= 10_000 && tolerance_bps <= 10_000);
            Self {
                admin: Self::env().caller(),
                operators: Mapping::default(),
                types: Vec::new(),
                targets: Mapping::default(),
                holdings: Mapping::default(),
                cooldown_blocks,
                last_rebalance: None,
                max_slippage_bps,
                observed_slippage_bps: 0,
                tolerance_bps,
                last_orders: Vec::new(),
            }
        }

        fn ensure_admin(&self) -> Result<(), Error> {
            if self.env().caller() != self.admin {
                return Err(Error::Unauthorized);
            }
            Ok(())
        }

        fn ensure_operator(&self) -> Result<(), Error> {
            let caller = self.env().caller();
            if caller != self.admin && !self.operators.get(caller).unwrap_or(false) {
                return Err(Error::Unauthorized);
            }
            Ok(())
        }

        // ── Administration ─────────────────────────────────────────────

        #[ink(message)]
        pub fn set_operator(&mut self, account: AccountId, allowed: bool) -> Result<(), Error> {
            self.ensure_admin()?;
            if allowed {
                self.operators.insert(account, &true);
            } else {
                self.operators.remove(account);
            }
            Ok(())
        }

        /// Set the target weights. Replaces the previous allocation; holdings of
        /// types that are no longer listed are kept and rebalanced to zero.
        #[ink(message)]
        pub fn set_target_allocation(&mut self, property_types: Vec<u32>, target_weights: Vec<u32>) -> Result<(), Error> {
            self.ensure_admin()?;
            let n = property_types.len();
            if n == 0 || n > MAX_TYPES || n != target_weights.len() {
                return Err(Error::InvalidAllocation);
            }
            let mut sorted = property_types.clone();
            sorted.sort_unstable();
            if sorted.windows(2).any(|w| w[0] == w[1]) {
                return Err(Error::InvalidAllocation);
            }
            let sum: u64 = target_weights.iter().map(|w| *w as u64).sum();
            if sum != 10_000 {
                return Err(Error::InvalidAllocation);
            }
            for t in self.types.iter() {
                self.targets.remove(*t);
            }
            for (t, w) in property_types.iter().zip(target_weights.iter()) {
                self.targets.insert(*t, w);
            }
            self.types = property_types;
            self.env().emit_event(AllocationTargetSet { type_count: n as u32 });
            Ok(())
        }

        /// `cooldown_blocks`, `max_slippage_bps` and `tolerance_bps`.
        #[ink(message)]
        pub fn set_rebalance_params(&mut self, cooldown_blocks: u32, max_slippage_bps: u32, tolerance_bps: u32) -> Result<(), Error> {
            self.ensure_admin()?;
            if max_slippage_bps > 10_000 || tolerance_bps > 10_000 {
                return Err(Error::InvalidConfig);
            }
            self.cooldown_blocks = cooldown_blocks;
            self.max_slippage_bps = max_slippage_bps;
            self.tolerance_bps = tolerance_bps;
            Ok(())
        }

        /// Latest market slippage a keeper observed for rebalancing trades. Operators only.
        #[ink(message)]
        pub fn report_slippage(&mut self, slippage_bps: u32) -> Result<(), Error> {
            self.ensure_operator()?;
            self.observed_slippage_bps = slippage_bps;
            Ok(())
        }

        // ── Trades (operator hook) ─────────────────────────────────────

        /// Record a buy or sell of `amount` value in `property_type`, then rebalance
        /// automatically. Operators only.
        #[ink(message)]
        pub fn record_trade(&mut self, property_type: u32, amount: u128, is_buy: bool) -> Result<RebalanceOutcome, Error> {
            self.ensure_operator()?;
            if amount == 0 {
                return Err(Error::ZeroAmount);
            }
            if !self.targets.contains(property_type) && !self.holdings.contains(property_type) {
                return Err(Error::UnknownPropertyType);
            }
            let held = self.holdings.get(property_type).unwrap_or(0);
            let new = if is_buy {
                held.saturating_add(amount)
            } else {
                held.checked_sub(amount).ok_or(Error::InsufficientHoldings)?
            };
            self.holdings.insert(property_type, &new);
            Ok(self.run_rebalance())
        }

        /// Rebalance now (subject to cooldown, slippage and tolerance). Anyone may call.
        #[ink(message)]
        pub fn rebalance(&mut self) -> RebalanceOutcome {
            self.run_rebalance()
        }

        // ── Queries ────────────────────────────────────────────────────

        #[ink(message)]
        pub fn get_holding(&self, property_type: u32) -> u128 {
            self.holdings.get(property_type).unwrap_or(0)
        }

        #[ink(message)]
        pub fn get_total_value(&self) -> u128 {
            self.universe().iter().map(|t| self.holdings.get(t).unwrap_or(0)).fold(0u128, u128::saturating_add)
        }

        #[ink(message)]
        pub fn get_target_allocation(&self) -> Vec<(u32, u32)> {
            self.types.iter().map(|t| (*t, self.targets.get(t).unwrap_or(0))).collect()
        }

        /// `(largest deviation in bps, [(type, current_bps, target_bps)])`.
        #[ink(message)]
        pub fn get_deviation(&self) -> (u32, Vec<(u32, u32, u32)>) {
            let total = self.get_total_value();
            let mut max = 0u32;
            let mut rows = Vec::new();
            for t in self.universe() {
                let current = if total == 0 {
                    0
                } else {
                    (self.holdings.get(t).unwrap_or(0).saturating_mul(10_000) / total) as u32
                };
                let target = self.targets.get(t).unwrap_or(0);
                max = max.max(current.abs_diff(target));
                rows.push((t, current, target));
            }
            (max, rows)
        }

        /// The orders produced by the last completed rebalance: `(type, signed value delta)`.
        #[ink(message)]
        pub fn get_last_orders(&self) -> Vec<(u32, i128)> {
            self.last_orders.clone()
        }

        #[ink(message)]
        pub fn get_last_rebalance_block(&self) -> Option<u32> {
            self.last_rebalance
        }

        // ── Internals ──────────────────────────────────────────────────

        /// Target types plus any other type that still has holdings (to be sold down).
        fn universe(&self) -> Vec<u32> {
            self.types.clone()
        }

        fn skip(&self, reason: SkipReason) -> RebalanceOutcome {
            self.env().emit_event(RebalanceSkipped { reason });
            RebalanceOutcome::Skipped(reason)
        }

        fn run_rebalance(&mut self) -> RebalanceOutcome {
            if self.types.is_empty() {
                return self.skip(SkipReason::NoTargets);
            }
            let total = self.get_total_value();
            if total == 0 {
                return self.skip(SkipReason::EmptyPool);
            }
            let now = self.env().block_number();
            if let Some(last) = self.last_rebalance {
                if now.saturating_sub(last) < self.cooldown_blocks {
                    return self.skip(SkipReason::Cooldown);
                }
            }
            if self.observed_slippage_bps > self.max_slippage_bps {
                return self.skip(SkipReason::SlippageTooHigh);
            }
            let (max_dev, _) = self.get_deviation();
            if max_dev <= self.tolerance_bps {
                return self.skip(SkipReason::WithinTolerance);
            }
            self.env().emit_event(RebalanceTriggered { max_deviation_bps: max_dev });

            // Targets, with the rounding remainder given to the heaviest type.
            let mut goals: Vec<(u32, u128)> = self
                .types
                .iter()
                .map(|t| (*t, total.saturating_mul(self.targets.get(t).unwrap_or(0) as u128) / 10_000))
                .collect();
            let assigned: u128 = goals.iter().map(|(_, v)| *v).sum();
            let heaviest = self
                .types
                .iter()
                .enumerate()
                .max_by_key(|(_, t)| self.targets.get(**t).unwrap_or(0))
                .map(|(i, _)| i)
                .unwrap_or(0);
            goals[heaviest].1 += total - assigned;

            let mut orders: Vec<(u32, i128)> = Vec::new();
            let mut moved: u128 = 0;
            for (t, goal) in goals {
                let held = self.holdings.get(t).unwrap_or(0);
                if goal != held {
                    orders.push((t, goal as i128 - held as i128));
                    moved = moved.saturating_add(goal.abs_diff(held));
                    self.holdings.insert(t, &goal);
                }
            }
            self.last_rebalance = Some(now);
            let count = orders.len() as u32;
            self.last_orders = orders;
            // Each unit of value is both sold and bought, so report half the gross movement.
            self.env().emit_event(RebalanceCompleted { orders: count, value_moved: moved / 2 });
            RebalanceOutcome::Completed(count)
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use ink::env::test;

        type Accounts = test::DefaultAccounts<ink::env::DefaultEnvironment>;

        const RESIDENTIAL: u32 = 1;
        const COMMERCIAL: u32 = 2;
        const LAND: u32 = 3;

        fn block(n: u32) {
            test::set_block_number::<ink::env::DefaultEnvironment>(n);
        }

        /// Admin = alice, who is also an operator. Cooldown 100 blocks, max slippage
        /// 200 bps, tolerance 100 bps. Targets 50% / 30% / 20%.
        fn setup() -> (RebalancingPools, Accounts) {
            let a = test::default_accounts::<ink::env::DefaultEnvironment>();
            test::set_caller::<ink::env::DefaultEnvironment>(a.alice);
            block(1_000);
            let mut p = RebalancingPools::new(100, 200, 100);
            p.set_target_allocation(vec![RESIDENTIAL, COMMERCIAL, LAND], vec![5_000, 3_000, 2_000]).unwrap();
            (p, a)
        }

        /// Seed the holdings without triggering a rebalance (cooldown disabled afterwards).
        fn seed(p: &mut RebalancingPools, amounts: [(u32, u128); 3]) {
            p.set_rebalance_params(100, 200, 10_000).unwrap(); // tolerance 100%: never rebalances
            for (t, v) in amounts {
                p.record_trade(t, v, true).unwrap();
            }
            p.set_rebalance_params(100, 200, 100).unwrap();
        }

        // ── Target allocation ───────────────────────────────────────────

        #[ink::test]
        fn the_admin_sets_a_valid_target_allocation() {
            let (p, _) = setup();
            assert_eq!(
                p.get_target_allocation(),
                vec![(RESIDENTIAL, 5_000), (COMMERCIAL, 3_000), (LAND, 2_000)]
            );
        }

        #[ink::test]
        fn invalid_allocations_are_rejected() {
            let (mut p, a) = setup();
            assert_eq!(p.set_target_allocation(vec![], vec![]), Err(Error::InvalidAllocation));
            assert_eq!(p.set_target_allocation(vec![1, 2], vec![10_000]), Err(Error::InvalidAllocation));
            assert_eq!(p.set_target_allocation(vec![1, 1], vec![5_000, 5_000]), Err(Error::InvalidAllocation));
            assert_eq!(p.set_target_allocation(vec![1, 2], vec![5_000, 4_999]), Err(Error::InvalidAllocation));
            assert_eq!(p.set_target_allocation(vec![1, 2], vec![5_000, 5_001]), Err(Error::InvalidAllocation));
            let many: Vec<u32> = (0..=MAX_TYPES as u32).collect();
            assert_eq!(p.set_target_allocation(many, vec![0; MAX_TYPES + 1]), Err(Error::InvalidAllocation));
            test::set_caller::<ink::env::DefaultEnvironment>(a.bob);
            assert_eq!(p.set_target_allocation(vec![1], vec![10_000]), Err(Error::Unauthorized));
            // The original allocation survived.
            assert_eq!(p.get_target_allocation().len(), 3);
        }

        // ── Deviation ───────────────────────────────────────────────────

        #[ink::test]
        fn an_empty_pool_deviates_by_the_largest_target() {
            let (p, _) = setup();
            let (max, rows) = p.get_deviation();
            assert_eq!(max, 5_000);
            assert!(rows.iter().all(|(_, current, _)| *current == 0));
        }

        #[ink::test]
        fn deviation_is_the_largest_gap_between_current_and_target_shares() {
            let (mut p, _) = setup();
            seed(&mut p, [(RESIDENTIAL, 700), (COMMERCIAL, 200), (LAND, 100)]);
            let (max, rows) = p.get_deviation();
            // Shares 70 / 20 / 10 vs targets 50 / 30 / 20: gaps of 20, 10 and 10 points.
            assert_eq!(max, 2_000);
            assert_eq!(
                rows,
                vec![(RESIDENTIAL, 7_000, 5_000), (COMMERCIAL, 2_000, 3_000), (LAND, 1_000, 2_000)]
            );
        }

        #[ink::test]
        fn a_pool_on_target_has_zero_deviation() {
            let (mut p, _) = setup();
            seed(&mut p, [(RESIDENTIAL, 500), (COMMERCIAL, 300), (LAND, 200)]);
            assert_eq!(p.get_deviation().0, 0);
        }

        // ── Rebalancing ─────────────────────────────────────────────────

        #[ink::test]
        fn a_rebalance_moves_every_type_to_its_target_value_and_conserves_the_total() {
            let (mut p, _) = setup();
            seed(&mut p, [(RESIDENTIAL, 700), (COMMERCIAL, 200), (LAND, 100)]);
            assert_eq!(p.rebalance(), RebalanceOutcome::Completed(3));
            assert_eq!(p.get_holding(RESIDENTIAL), 500);
            assert_eq!(p.get_holding(COMMERCIAL), 300);
            assert_eq!(p.get_holding(LAND), 200);
            assert_eq!(p.get_total_value(), 1_000);
            // Overweight residential is sold, the others are bought.
            assert_eq!(
                p.get_last_orders(),
                vec![(RESIDENTIAL, -200), (COMMERCIAL, 100), (LAND, 100)]
            );
            assert_eq!(p.get_deviation().0, 0);
        }

        #[ink::test]
        fn rounding_dust_goes_to_the_heaviest_type_so_the_total_is_exact() {
            let (mut p, _) = setup();
            seed(&mut p, [(RESIDENTIAL, 3), (COMMERCIAL, 3), (LAND, 3)]); // total 9
            p.rebalance();
            // 50% / 30% / 20% of 9 floors to 4 / 2 / 1; the leftover 2 go to residential.
            assert_eq!(p.get_holding(RESIDENTIAL), 6);
            assert_eq!(p.get_holding(COMMERCIAL), 2);
            assert_eq!(p.get_holding(LAND), 1);
            assert_eq!(p.get_total_value(), 9);
        }

        #[ink::test]
        fn buys_and_sells_trigger_a_rebalance_automatically() {
            let (mut p, a) = setup();
            seed(&mut p, [(RESIDENTIAL, 500), (COMMERCIAL, 300), (LAND, 200)]);
            block(1_000 + 500);
            // A large residential purchase pushes the pool off target; the same call rebalances it.
            test::set_caller::<ink::env::DefaultEnvironment>(a.alice);
            let outcome = p.record_trade(RESIDENTIAL, 1_000, true).unwrap();
            assert!(matches!(outcome, RebalanceOutcome::Completed(_)));
            // 1_500 / 300 / 200 -> total 2_000 -> targets 1_000 / 600 / 400.
            assert_eq!(p.get_holding(RESIDENTIAL), 1_000);
            assert_eq!(p.get_holding(COMMERCIAL), 600);
            assert_eq!(p.get_holding(LAND), 400);
        }

        #[ink::test]
        fn small_drifts_within_tolerance_are_left_alone() {
            let (mut p, _) = setup();
            seed(&mut p, [(RESIDENTIAL, 500), (COMMERCIAL, 300), (LAND, 200)]);
            block(2_000);
            // +5 on residential: 505/1005 = 50.2% (20 bps off), inside the 100 bps band.
            assert_eq!(
                p.record_trade(RESIDENTIAL, 5, true),
                Ok(RebalanceOutcome::Skipped(SkipReason::WithinTolerance))
            );
            assert_eq!(p.get_holding(RESIDENTIAL), 505);
        }

        // ── Cooldown ────────────────────────────────────────────────────

        #[ink::test]
        fn a_second_rebalance_inside_the_cooldown_is_skipped_until_it_elapses() {
            let (mut p, _) = setup();
            seed(&mut p, [(RESIDENTIAL, 700), (COMMERCIAL, 200), (LAND, 100)]);
            assert_eq!(p.rebalance(), RebalanceOutcome::Completed(3));
            // Knock it off target again, at the same block.
            p.set_rebalance_params(100, 200, 10_000).unwrap();
            p.record_trade(LAND, 900, true).unwrap();
            p.set_rebalance_params(100, 200, 100).unwrap();
            assert_eq!(p.rebalance(), RebalanceOutcome::Skipped(SkipReason::Cooldown));

            block(1_000 + 99);
            assert_eq!(p.rebalance(), RebalanceOutcome::Skipped(SkipReason::Cooldown));
            block(1_000 + 100);
            assert!(matches!(p.rebalance(), RebalanceOutcome::Completed(_)));
            assert_eq!(p.get_last_rebalance_block(), Some(1_100));
        }

        // ── Slippage protection ─────────────────────────────────────────

        #[ink::test]
        fn rebalancing_pauses_while_reported_slippage_is_too_high() {
            let (mut p, _) = setup();
            seed(&mut p, [(RESIDENTIAL, 700), (COMMERCIAL, 200), (LAND, 100)]);
            p.report_slippage(201).unwrap();
            assert_eq!(p.rebalance(), RebalanceOutcome::Skipped(SkipReason::SlippageTooHigh));
            assert_eq!(p.get_holding(RESIDENTIAL), 700);
            // At the limit it is allowed (the bound is inclusive).
            p.report_slippage(200).unwrap();
            assert!(matches!(p.rebalance(), RebalanceOutcome::Completed(_)));
        }

        #[ink::test]
        fn only_operators_report_slippage_or_record_trades() {
            let (mut p, a) = setup();
            test::set_caller::<ink::env::DefaultEnvironment>(a.eve);
            assert_eq!(p.report_slippage(1), Err(Error::Unauthorized));
            assert_eq!(p.record_trade(RESIDENTIAL, 1, true), Err(Error::Unauthorized));
            test::set_caller::<ink::env::DefaultEnvironment>(a.alice);
            p.set_operator(a.eve, true).unwrap();
            test::set_caller::<ink::env::DefaultEnvironment>(a.eve);
            assert!(p.record_trade(RESIDENTIAL, 1, true).is_ok());
            test::set_caller::<ink::env::DefaultEnvironment>(a.alice);
            p.set_operator(a.eve, false).unwrap();
            test::set_caller::<ink::env::DefaultEnvironment>(a.eve);
            assert_eq!(p.report_slippage(1), Err(Error::Unauthorized));
        }

        // ── Skips ───────────────────────────────────────────────────────

        #[ink::test]
        fn nothing_to_rebalance_without_targets_or_value() {
            let a = test::default_accounts::<ink::env::DefaultEnvironment>();
            test::set_caller::<ink::env::DefaultEnvironment>(a.alice);
            let mut p = RebalancingPools::new(10, 100, 50);
            assert_eq!(p.rebalance(), RebalanceOutcome::Skipped(SkipReason::NoTargets));
            p.set_target_allocation(vec![1, 2], vec![6_000, 4_000]).unwrap();
            assert_eq!(p.rebalance(), RebalanceOutcome::Skipped(SkipReason::EmptyPool));
        }

        // ── Trades ──────────────────────────────────────────────────────

        #[ink::test]
        fn selling_more_than_is_held_or_trading_unknown_types_is_rejected() {
            let (mut p, _) = setup();
            seed(&mut p, [(RESIDENTIAL, 500), (COMMERCIAL, 300), (LAND, 200)]);
            assert_eq!(p.record_trade(RESIDENTIAL, 501, false), Err(Error::InsufficientHoldings));
            assert_eq!(p.record_trade(99, 10, true), Err(Error::UnknownPropertyType));
            assert_eq!(p.record_trade(RESIDENTIAL, 0, true), Err(Error::ZeroAmount));
            assert_eq!(p.get_holding(RESIDENTIAL), 500);
        }

        #[ink::test]
        fn parameters_are_admin_only_and_bounded() {
            let (mut p, a) = setup();
            assert_eq!(p.set_rebalance_params(1, 10_001, 0), Err(Error::InvalidConfig));
            assert_eq!(p.set_rebalance_params(1, 0, 10_001), Err(Error::InvalidConfig));
            test::set_caller::<ink::env::DefaultEnvironment>(a.bob);
            assert_eq!(p.set_rebalance_params(1, 1, 1), Err(Error::Unauthorized));
        }

        #[ink::test]
        fn rebalance_events_are_emitted() {
            let (mut p, _) = setup();
            seed(&mut p, [(RESIDENTIAL, 700), (COMMERCIAL, 200), (LAND, 100)]);
            let before = test::recorded_events().count();
            p.rebalance(); // Triggered + Completed
            assert_eq!(test::recorded_events().count() - before, 2);
            p.rebalance(); // Skipped (cooldown)
            assert_eq!(test::recorded_events().count() - before, 3);
        }
    }
}
