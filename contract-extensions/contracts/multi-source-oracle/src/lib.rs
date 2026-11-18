#![cfg_attr(not(feature = "std"), no_std, no_main)]
#![allow(clippy::new_without_default)]

//! # Multi-source oracle (#1436)
//!
//! Registered sources push prices per asset; [`MultiSourceOracle::get_aggregated_price`]
//! combines the fresh ones into a median and a confidence score.
//!
//! ## Aggregation, step by step
//!
//! 1. **Stale filter.** A submission older than `max_age_blocks` (measured from
//!    the block it was submitted in) is ignored.
//! 2. **Outlier filter.** With the fresh prices, compute the weighted mean and
//!    weighted standard deviation `σ`, and the median. A price farther than
//!    `2σ` from the median is dropped (not applied when `σ = 0`).
//! 3. **Result.** The median of the remaining prices (average of the two middle
//!    values for an even count), and
//!    `confidence = 10_000 - min(10_000, σ * 10_000 / mean)` in basis points,
//!    computed over the remaining prices (so `10_000` means perfect agreement).
//!
//! Source **weights** are used in the mean / standard deviation behind the
//! confidence score and the outlier cut-off; the reported price is the plain
//! median, as the issue specifies.
//!
//! A result needs at least `min_sources` fresh prices, otherwise `None`.
//!
//! The submitting account *is* the source (ink authenticates the caller), so
//! [`MultiSourceOracle::submit_price`] takes no separate source address.

#[ink::contract]
pub mod multi_source_oracle {
    use ink::prelude::vec::Vec;
    use ink::storage::Mapping;

    pub const MAX_SOURCES: usize = 16;

    #[derive(Debug, Clone, Copy, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout))]
    pub struct PriceEntry {
        pub price: u128,
        /// Timestamp the source claims for the price (informational).
        pub timestamp: u64,
        /// Block in which it was submitted (used for staleness).
        pub block: u32,
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub struct AggregatedPrice {
        pub median: u128,
        /// 0..=10_000.
        pub confidence_bps: u32,
        /// Prices that took part after the stale and outlier filters.
        pub sources_used: u32,
        /// Fresh prices dropped as outliers.
        pub outliers_removed: u32,
    }

    #[derive(Debug, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub enum Error {
        Unauthorized,
        SourceAlreadyRegistered,
        SourceNotFound,
        TooManySources,
        InvalidWeight,
        InvalidPrice,
        InvalidConfig,
    }

    #[ink(event)]
    pub struct SourceRegistered {
        #[ink(topic)]
        pub source: AccountId,
        pub weight: u32,
    }

    #[ink(event)]
    pub struct SourceRemoved {
        #[ink(topic)]
        pub source: AccountId,
    }

    #[ink(event)]
    pub struct PriceSubmitted {
        #[ink(topic)]
        pub asset: u64,
        #[ink(topic)]
        pub source: AccountId,
        pub price: u128,
    }

    #[ink(event)]
    pub struct PriceAggregated {
        #[ink(topic)]
        pub asset: u64,
        pub median: u128,
        pub confidence_bps: u32,
        pub sources_used: u32,
    }

    #[ink(storage)]
    pub struct MultiSourceOracle {
        admin: AccountId,
        sources: Vec<AccountId>,
        weights: Mapping<AccountId, u32>,
        prices: Mapping<(u64, AccountId), PriceEntry>,
        last_aggregate: Mapping<u64, AggregatedPrice>,
        max_age_blocks: u32,
        min_sources: u32,
    }

    /// Integer square root (floor).
    fn isqrt(n: u128) -> u128 {
        if n < 2 {
            return n;
        }
        let mut x = n;
        let mut y = x.div_ceil(2);
        while y < x {
            x = y;
            y = (x + n / x) / 2;
        }
        x
    }

    /// Median of a non-empty slice (average of the two middle values when even).
    fn median(sorted: &[u128]) -> u128 {
        let n = sorted.len();
        if n % 2 == 1 {
            sorted[n / 2]
        } else {
            let (a, b) = (sorted[n / 2 - 1], sorted[n / 2]);
            a / 2 + b / 2 + (a % 2 + b % 2) / 2
        }
    }

    /// Weighted mean and standard deviation of `(price, weight)` pairs.
    fn weighted_stats(items: &[(u128, u32)]) -> (u128, u128) {
        let total_w: u128 = items.iter().map(|(_, w)| *w as u128).sum();
        if total_w == 0 {
            return (0, 0);
        }
        let mean = items.iter().map(|(p, w)| p.saturating_mul(*w as u128)).fold(0u128, u128::saturating_add) / total_w;
        let var = items
            .iter()
            .map(|(p, w)| {
                let d = p.abs_diff(mean);
                d.saturating_mul(d).saturating_mul(*w as u128)
            })
            .fold(0u128, u128::saturating_add)
            / total_w;
        (mean, isqrt(var))
    }

    impl MultiSourceOracle {
        #[ink(constructor)]
        pub fn new(max_age_blocks: u32, min_sources: u32) -> Self {
            assert!(max_age_blocks > 0 && min_sources > 0);
            Self {
                admin: Self::env().caller(),
                sources: Vec::new(),
                weights: Mapping::default(),
                prices: Mapping::default(),
                last_aggregate: Mapping::default(),
                max_age_blocks,
                min_sources,
            }
        }

        fn ensure_admin(&self) -> Result<(), Error> {
            if self.env().caller() != self.admin {
                return Err(Error::Unauthorized);
            }
            Ok(())
        }

        // ── Sources ────────────────────────────────────────────────────

        #[ink(message)]
        pub fn register_source(&mut self, source: AccountId, weight: u32) -> Result<(), Error> {
            self.ensure_admin()?;
            if weight == 0 {
                return Err(Error::InvalidWeight);
            }
            if self.weights.contains(source) {
                return Err(Error::SourceAlreadyRegistered);
            }
            if self.sources.len() >= MAX_SOURCES {
                return Err(Error::TooManySources);
            }
            self.sources.push(source);
            self.weights.insert(source, &weight);
            self.env().emit_event(SourceRegistered { source, weight });
            Ok(())
        }

        #[ink(message)]
        pub fn remove_source(&mut self, source: AccountId) -> Result<(), Error> {
            self.ensure_admin()?;
            if !self.weights.contains(source) {
                return Err(Error::SourceNotFound);
            }
            self.weights.remove(source);
            self.sources.retain(|s| *s != source);
            self.env().emit_event(SourceRemoved { source });
            Ok(())
        }

        #[ink(message)]
        pub fn get_sources(&self) -> Vec<(AccountId, u32)> {
            self.sources.iter().map(|s| (*s, self.weights.get(s).unwrap_or(0))).collect()
        }

        #[ink(message)]
        pub fn set_staleness(&mut self, max_age_blocks: u32, min_sources: u32) -> Result<(), Error> {
            self.ensure_admin()?;
            if max_age_blocks == 0 || min_sources == 0 {
                return Err(Error::InvalidConfig);
            }
            self.max_age_blocks = max_age_blocks;
            self.min_sources = min_sources;
            Ok(())
        }

        // ── Prices ─────────────────────────────────────────────────────

        /// A registered source submits its latest price for `asset`.
        #[ink(message)]
        pub fn submit_price(&mut self, asset: u64, price: u128, timestamp: u64) -> Result<(), Error> {
            let source = self.env().caller();
            if !self.weights.contains(source) {
                return Err(Error::Unauthorized);
            }
            if price == 0 {
                return Err(Error::InvalidPrice);
            }
            self.prices.insert(
                (asset, source),
                &PriceEntry { price, timestamp, block: self.env().block_number() },
            );
            self.env().emit_event(PriceSubmitted { asset, source, price });
            Ok(())
        }

        /// The latest submission of `source` for `asset`.
        #[ink(message)]
        pub fn get_price_of(&self, asset: u64, source: AccountId) -> Option<PriceEntry> {
            self.prices.get((asset, source))
        }

        /// Median price and confidence of the fresh, non-outlier submissions.
        #[ink(message)]
        pub fn get_aggregated_price(&self, asset: u64) -> Option<AggregatedPrice> {
            self.aggregate(asset)
        }

        /// Compute the aggregate, store it and emit `PriceAggregated`. Anyone may call.
        #[ink(message)]
        pub fn aggregate_price(&mut self, asset: u64) -> Option<AggregatedPrice> {
            let result = self.aggregate(asset)?;
            self.last_aggregate.insert(asset, &result);
            self.env().emit_event(PriceAggregated {
                asset,
                median: result.median,
                confidence_bps: result.confidence_bps,
                sources_used: result.sources_used,
            });
            Some(result)
        }

        /// The aggregate stored by the last `aggregate_price` call.
        #[ink(message)]
        pub fn get_last_aggregate(&self, asset: u64) -> Option<AggregatedPrice> {
            self.last_aggregate.get(asset)
        }

        fn aggregate(&self, asset: u64) -> Option<AggregatedPrice> {
            let now = self.env().block_number();
            let mut fresh: Vec<(u128, u32)> = Vec::new();
            for source in self.sources.iter() {
                if let Some(entry) = self.prices.get((asset, *source)) {
                    if now.saturating_sub(entry.block) <= self.max_age_blocks {
                        fresh.push((entry.price, self.weights.get(source).unwrap_or(1)));
                    }
                }
            }
            if (fresh.len() as u32) < self.min_sources {
                return None;
            }
            let mut sorted: Vec<u128> = fresh.iter().map(|(p, _)| *p).collect();
            sorted.sort_unstable();
            let first_median = median(&sorted);
            let (_, sigma) = weighted_stats(&fresh);

            // Outliers: farther than two standard deviations from the median.
            let kept: Vec<(u128, u32)> = if sigma == 0 {
                fresh.clone()
            } else {
                fresh
                    .iter()
                    .copied()
                    .filter(|(p, _)| p.abs_diff(first_median) <= sigma.saturating_mul(2))
                    .collect()
            };
            let outliers_removed = (fresh.len() - kept.len()) as u32;
            let mut kept_prices: Vec<u128> = kept.iter().map(|(p, _)| *p).collect();
            kept_prices.sort_unstable();
            let result_median = median(&kept_prices);
            let (mean, sigma2) = weighted_stats(&kept);
            let confidence_bps = if mean == 0 {
                0
            } else {
                10_000u128.saturating_sub((sigma2.saturating_mul(10_000) / mean).min(10_000)) as u32
            };
            Some(AggregatedPrice {
                median: result_median,
                confidence_bps,
                sources_used: kept.len() as u32,
                outliers_removed,
            })
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use ink::env::test;

        type Accounts = test::DefaultAccounts<ink::env::DefaultEnvironment>;
        const ASSET: u64 = 1;

        fn setup() -> (MultiSourceOracle, Accounts) {
            let accounts = test::default_accounts::<ink::env::DefaultEnvironment>();
            test::set_caller::<ink::env::DefaultEnvironment>(accounts.alice);
            test::set_block_number::<ink::env::DefaultEnvironment>(100);
            (MultiSourceOracle::new(50, 1), accounts)
        }

        fn sources(o: &mut MultiSourceOracle, who: &[AccountId]) {
            for s in who {
                o.register_source(*s, 1).unwrap();
            }
        }

        fn submit(o: &mut MultiSourceOracle, who: AccountId, price: u128) {
            test::set_caller::<ink::env::DefaultEnvironment>(who);
            o.submit_price(ASSET, price, 0).unwrap();
        }

        fn block(n: u32) {
            test::set_block_number::<ink::env::DefaultEnvironment>(n);
        }

        // ── Pure math ───────────────────────────────────────────────────

        #[test]
        fn integer_square_root_is_exact_on_squares_and_floors_otherwise() {
            for n in [0u128, 1, 2, 3, 4, 15, 16, 17, 1_000_000, 999_999, u64::MAX as u128] {
                let r = isqrt(n);
                assert!(r * r <= n && (r + 1) * (r + 1) > n, "n = {n}, r = {r}");
            }
        }

        #[test]
        fn the_median_handles_odd_even_and_single_inputs() {
            assert_eq!(median(&[7]), 7);
            assert_eq!(median(&[1, 5, 9]), 5);
            assert_eq!(median(&[1, 5, 9, 11]), 7);
            assert_eq!(median(&[2, 3]), 2); // (2 + 3) / 2 floors
            assert_eq!(median(&[u128::MAX, u128::MAX]), u128::MAX);
        }

        // ── Source management ───────────────────────────────────────────

        #[ink::test]
        fn the_admin_registers_and_removes_sources() {
            let (mut o, a) = setup();
            o.register_source(a.bob, 3).unwrap();
            o.register_source(a.charlie, 1).unwrap();
            assert_eq!(o.get_sources(), vec![(a.bob, 3), (a.charlie, 1)]);
            assert_eq!(o.register_source(a.bob, 1), Err(Error::SourceAlreadyRegistered));
            assert_eq!(o.register_source(a.eve, 0), Err(Error::InvalidWeight));
            o.remove_source(a.bob).unwrap();
            assert_eq!(o.get_sources(), vec![(a.charlie, 1)]);
            assert_eq!(o.remove_source(a.bob), Err(Error::SourceNotFound));

            test::set_caller::<ink::env::DefaultEnvironment>(a.bob);
            assert_eq!(o.register_source(a.eve, 1), Err(Error::Unauthorized));
            assert_eq!(o.remove_source(a.charlie), Err(Error::Unauthorized));
        }

        #[ink::test]
        fn the_number_of_sources_is_bounded() {
            let (mut o, _) = setup();
            for i in 0..MAX_SOURCES {
                o.register_source(AccountId::from([i as u8 + 1; 32]), 1).unwrap();
            }
            assert_eq!(
                o.register_source(AccountId::from([200; 32]), 1),
                Err(Error::TooManySources)
            );
        }

        #[ink::test]
        fn only_registered_sources_can_submit_valid_prices() {
            let (mut o, a) = setup();
            sources(&mut o, &[a.bob]);
            test::set_caller::<ink::env::DefaultEnvironment>(a.eve);
            assert_eq!(o.submit_price(ASSET, 100, 0), Err(Error::Unauthorized));
            test::set_caller::<ink::env::DefaultEnvironment>(a.bob);
            assert_eq!(o.submit_price(ASSET, 0, 0), Err(Error::InvalidPrice));
            o.submit_price(ASSET, 100, 42).unwrap();
            let entry = o.get_price_of(ASSET, a.bob).unwrap();
            assert_eq!((entry.price, entry.timestamp, entry.block), (100, 42, 100));
        }

        // ── Aggregation ─────────────────────────────────────────────────

        #[ink::test]
        fn the_median_of_three_sources_is_the_middle_price() {
            let (mut o, a) = setup();
            sources(&mut o, &[a.bob, a.charlie, a.django]);
            submit(&mut o, a.bob, 100);
            submit(&mut o, a.charlie, 102);
            submit(&mut o, a.django, 101);
            let agg = o.get_aggregated_price(ASSET).unwrap();
            assert_eq!(agg.median, 101);
            assert_eq!(agg.sources_used, 3);
            assert_eq!(agg.outliers_removed, 0);
        }

        #[ink::test]
        fn tightly_agreeing_sources_have_high_confidence_and_wide_spread_lowers_it() {
            let (mut tight, a) = setup();
            sources(&mut tight, &[a.bob, a.charlie, a.django]);
            for (who, p) in [(a.bob, 10_000), (a.charlie, 10_001), (a.django, 9_999)] {
                submit(&mut tight, who, p);
            }
            let (mut wide, _) = setup();
            sources(&mut wide, &[a.bob, a.charlie, a.django]);
            for (who, p) in [(a.bob, 8_000), (a.charlie, 10_000), (a.django, 12_000)] {
                submit(&mut wide, who, p);
            }
            let t = tight.get_aggregated_price(ASSET).unwrap().confidence_bps;
            let w = wide.get_aggregated_price(ASSET).unwrap().confidence_bps;
            assert!(t > 9_990, "{t}");
            assert!(w < t, "{w} vs {t}");
        }

        #[ink::test]
        fn identical_prices_give_full_confidence() {
            let (mut o, a) = setup();
            sources(&mut o, &[a.bob, a.charlie]);
            submit(&mut o, a.bob, 500);
            submit(&mut o, a.charlie, 500);
            let agg = o.get_aggregated_price(ASSET).unwrap();
            assert_eq!((agg.median, agg.confidence_bps), (500, 10_000));
        }

        #[ink::test]
        fn an_outlier_beyond_two_standard_deviations_is_excluded() {
            let (mut o, a) = setup();
            let eight: Vec<AccountId> = (1..=8u8).map(|i| AccountId::from([i; 32])).collect();
            sources(&mut o, &eight);
            for who in &eight[..7] {
                submit(&mut o, *who, 100);
            }
            submit(&mut o, eight[7], 10_000); // wildly off
            let agg = o.get_aggregated_price(ASSET).unwrap();
            assert_eq!(agg.outliers_removed, 1);
            assert_eq!(agg.sources_used, 7);
            assert_eq!(agg.median, 100);
            assert_eq!(agg.confidence_bps, 10_000);
            let _ = a;
        }

        #[ink::test]
        fn with_two_sources_neither_is_treated_as_an_outlier() {
            let (mut o, a) = setup();
            sources(&mut o, &[a.bob, a.charlie]);
            submit(&mut o, a.bob, 100);
            submit(&mut o, a.charlie, 200);
            let agg = o.get_aggregated_price(ASSET).unwrap();
            assert_eq!(agg.sources_used, 2);
            assert_eq!(agg.median, 150);
        }

        // ── Staleness ───────────────────────────────────────────────────

        #[ink::test]
        fn stale_submissions_are_excluded_and_the_boundary_is_inclusive() {
            let (mut o, a) = setup();
            sources(&mut o, &[a.bob, a.charlie, a.django]);
            submit(&mut o, a.bob, 100);
            block(110);
            submit(&mut o, a.charlie, 200);
            submit(&mut o, a.django, 210);
            // Bob's price is 50 blocks old at block 150: still fresh.
            block(150);
            assert_eq!(o.get_aggregated_price(ASSET).unwrap().sources_used, 3);
            // 51 blocks old: stale and ignored.
            block(151);
            let agg = o.get_aggregated_price(ASSET).unwrap();
            assert_eq!(agg.sources_used, 2);
            assert_eq!(agg.median, 205);
        }

        #[ink::test]
        fn too_few_fresh_prices_give_no_result() {
            let (mut o, a) = setup();
            o.set_staleness(50, 2).unwrap();
            sources(&mut o, &[a.bob, a.charlie]);
            assert!(o.get_aggregated_price(ASSET).is_none());
            submit(&mut o, a.bob, 100);
            assert!(o.get_aggregated_price(ASSET).is_none());
            submit(&mut o, a.charlie, 100);
            assert!(o.get_aggregated_price(ASSET).is_some());
            block(1_000);
            assert!(o.get_aggregated_price(ASSET).is_none());
        }

        #[ink::test]
        fn removing_a_source_removes_its_price_from_the_aggregate() {
            let (mut o, a) = setup();
            sources(&mut o, &[a.bob, a.charlie, a.django]);
            submit(&mut o, a.bob, 100);
            submit(&mut o, a.charlie, 110);
            submit(&mut o, a.django, 120);
            test::set_caller::<ink::env::DefaultEnvironment>(a.alice);
            o.remove_source(a.django).unwrap();
            let agg = o.get_aggregated_price(ASSET).unwrap();
            assert_eq!((agg.sources_used, agg.median), (2, 105));
        }

        #[ink::test]
        fn assets_are_aggregated_independently() {
            let (mut o, a) = setup();
            sources(&mut o, &[a.bob]);
            test::set_caller::<ink::env::DefaultEnvironment>(a.bob);
            o.submit_price(1, 100, 0).unwrap();
            o.submit_price(2, 900, 0).unwrap();
            assert_eq!(o.get_aggregated_price(1).unwrap().median, 100);
            assert_eq!(o.get_aggregated_price(2).unwrap().median, 900);
            assert!(o.get_aggregated_price(3).is_none());
        }

        // ── Events and storage of the result ────────────────────────────

        #[ink::test]
        fn aggregating_stores_the_result_and_emits_an_event() {
            let (mut o, a) = setup();
            sources(&mut o, &[a.bob]);
            submit(&mut o, a.bob, 777);
            let before = test::recorded_events().count();
            let result = o.aggregate_price(ASSET).unwrap();
            assert_eq!(result.median, 777);
            assert_eq!(o.get_last_aggregate(ASSET), Some(result));
            assert_eq!(test::recorded_events().count() - before, 1);
            assert!(o.aggregate_price(99).is_none());
        }

        #[ink::test]
        fn staleness_settings_are_admin_only_and_validated() {
            let (mut o, a) = setup();
            assert_eq!(o.set_staleness(0, 1), Err(Error::InvalidConfig));
            assert_eq!(o.set_staleness(10, 0), Err(Error::InvalidConfig));
            test::set_caller::<ink::env::DefaultEnvironment>(a.bob);
            assert_eq!(o.set_staleness(10, 1), Err(Error::Unauthorized));
        }
    }
}
