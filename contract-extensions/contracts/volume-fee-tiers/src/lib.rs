#![cfg_attr(not(feature = "std"), no_std, no_main)]
#![allow(clippy::new_without_default)]

//! # Volume-based fee tiers (#1444)
//!
//! Traders that move more volume pay lower fees. A DEX (an authorised
//! *recorder*) calls [`VolumeFeeManager::record_volume`] on each swap; anyone can
//! read the applicable rate with [`VolumeFeeManager::get_fee_rate`].
//!
//! ## Default schedule (basis points; lower bound inclusive)
//!
//! | rolling volume | fee |
//! |---|---|
//! | 0 – 9 999 | 30 bps |
//! | 10 000 – 49 999 | 25 bps |
//! | 50 000 – 99 999 | 20 bps |
//! | 100 000 + | 15 bps |
//!
//! ## Rolling window without a transaction list
//!
//! Storing every trade would be unbounded. Each account instead keeps **two
//! buckets** of `window_blocks` (default 14 400 blocks, one day at 6 s):
//! `current` and `previous`, plus the block the current bucket started at. The
//! rolling volume is the *sliding-window-counter* estimate
//!
//! ```text
//! rolling = current + previous * (window - elapsed_in_current_bucket) / window
//! ```
//!
//! so volume ages out smoothly and is exactly zero once two full windows have
//! passed. It is an **approximation** of a true rolling sum (it assumes the
//! previous bucket's volume was spread evenly), chosen for O(1) storage and gas.
//!
//! ## Wiring
//!
//! The DEX is expected to call `record_volume`; this contract does not call the
//! `fees` contract or the DEX itself, and the `fees` contract does not read it.

#[ink::contract]
pub mod volume_fee_tiers {
    use ink::prelude::vec::Vec;
    use ink::storage::Mapping;

    /// Default rolling window: one day at 6-second blocks.
    pub const DEFAULT_WINDOW_BLOCKS: u32 = 14_400;
    /// Most tiers a schedule may hold.
    pub const MAX_TIERS: usize = 8;

    #[derive(Debug, Clone, Copy, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout))]
    pub struct FeeTier {
        /// Smallest rolling volume (inclusive) at which this tier applies.
        pub min_volume: u128,
        pub fee_bps: u32,
    }

    #[derive(Debug, Clone, Copy, Default, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout))]
    pub struct VolumeRecord {
        pub bucket_start: u32,
        pub current: u128,
        pub previous: u128,
        /// The fee last reported for this account (to detect tier changes).
        pub last_fee_bps: u32,
    }

    #[derive(Debug, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub enum Error {
        Unauthorized,
        AlreadyRegistered,
        NotRegistered,
        ZeroAmount,
        /// The tier list is empty, too long, does not start at volume 0, is not
        /// strictly increasing in volume, rises in fee, or has a fee above 100%.
        InvalidTiers,
        InvalidWindow,
    }

    #[ink(event)]
    pub struct VolumeTrackerRegistered {
        #[ink(topic)]
        pub account: AccountId,
    }

    #[ink(event)]
    pub struct VolumeRecorded {
        #[ink(topic)]
        pub account: AccountId,
        pub amount: u128,
        pub rolling_volume: u128,
    }

    #[ink(event)]
    pub struct FeeTierChanged {
        #[ink(topic)]
        pub account: AccountId,
        pub old_fee_bps: u32,
        pub new_fee_bps: u32,
    }

    #[ink(event)]
    pub struct FeeTierUpdated {
        pub tier_count: u32,
    }

    #[ink(storage)]
    pub struct VolumeFeeManager {
        admin: AccountId,
        recorders: Mapping<AccountId, bool>,
        tiers: Vec<FeeTier>,
        window_blocks: u32,
        records: Mapping<AccountId, VolumeRecord>,
    }

    impl VolumeFeeManager {
        #[ink(constructor)]
        pub fn new() -> Self {
            Self {
                admin: Self::env().caller(),
                recorders: Mapping::default(),
                tiers: Self::default_tiers(),
                window_blocks: DEFAULT_WINDOW_BLOCKS,
                records: Mapping::default(),
            }
        }

        fn default_tiers() -> Vec<FeeTier> {
            ink::prelude::vec![
                FeeTier { min_volume: 0, fee_bps: 30 },
                FeeTier { min_volume: 10_000, fee_bps: 25 },
                FeeTier { min_volume: 50_000, fee_bps: 20 },
                FeeTier { min_volume: 100_000, fee_bps: 15 },
            ]
        }

        // ── Registration ───────────────────────────────────────────────

        /// Start tracking the caller's volume.
        #[ink(message)]
        pub fn register_volume_tracker(&mut self) -> Result<(), Error> {
            let caller = self.env().caller();
            if self.records.contains(caller) {
                return Err(Error::AlreadyRegistered);
            }
            let record = VolumeRecord {
                bucket_start: self.env().block_number(),
                last_fee_bps: self.tiers.first().map(|t| t.fee_bps).unwrap_or(0),
                ..Default::default()
            };
            self.records.insert(caller, &record);
            self.env().emit_event(VolumeTrackerRegistered { account: caller });
            Ok(())
        }

        // ── Recording (DEX hook) ───────────────────────────────────────

        /// Add `amount` of volume for `account`. Admin or an authorised recorder only.
        #[ink(message)]
        pub fn record_volume(&mut self, account: AccountId, amount: u128) -> Result<(), Error> {
            let caller = self.env().caller();
            if caller != self.admin && !self.recorders.get(caller).unwrap_or(false) {
                return Err(Error::Unauthorized);
            }
            if amount == 0 {
                return Err(Error::ZeroAmount);
            }
            let mut record = self.records.get(account).ok_or(Error::NotRegistered)?;
            let now = self.env().block_number();
            Self::roll(&mut record, now, self.window_blocks);
            record.current = record.current.saturating_add(amount);

            let rolling = Self::rolling_volume(&record, now, self.window_blocks);
            let new_fee = self.fee_for(rolling);
            let old_fee = record.last_fee_bps;
            record.last_fee_bps = new_fee;
            self.records.insert(account, &record);

            self.env().emit_event(VolumeRecorded { account, amount, rolling_volume: rolling });
            if new_fee != old_fee {
                self.env().emit_event(FeeTierChanged { account, old_fee_bps: old_fee, new_fee_bps: new_fee });
            }
            Ok(())
        }

        // ── Queries ────────────────────────────────────────────────────

        /// The fee (bps) that applies to `account` right now. An account that is
        /// not tracked pays the base (first-tier) rate.
        #[ink(message)]
        pub fn get_fee_rate(&self, account: AccountId) -> u32 {
            self.fee_for(self.get_rolling_volume(account))
        }

        /// Estimated volume of the last window for `account` (see module docs).
        #[ink(message)]
        pub fn get_rolling_volume(&self, account: AccountId) -> u128 {
            match self.records.get(account) {
                Some(mut r) => {
                    let now = self.env().block_number();
                    Self::roll(&mut r, now, self.window_blocks);
                    Self::rolling_volume(&r, now, self.window_blocks)
                }
                None => 0,
            }
        }

        #[ink(message)]
        pub fn get_fee_tiers(&self) -> Vec<FeeTier> {
            self.tiers.clone()
        }

        #[ink(message)]
        pub fn get_window_blocks(&self) -> u32 {
            self.window_blocks
        }

        #[ink(message)]
        pub fn is_registered(&self, account: AccountId) -> bool {
            self.records.contains(account)
        }

        // ── Administration ─────────────────────────────────────────────

        fn ensure_admin(&self) -> Result<(), Error> {
            if self.env().caller() != self.admin {
                return Err(Error::Unauthorized);
            }
            Ok(())
        }

        /// Replace the schedule. Admin only.
        ///
        /// Tiers must be non-empty, at most [`MAX_TIERS`], start at volume `0`,
        /// be strictly increasing in `min_volume`, never *increase* the fee as
        /// volume grows, and keep every fee at or below 10 000 bps.
        #[ink(message)]
        pub fn set_fee_tiers(&mut self, tiers: Vec<FeeTier>) -> Result<(), Error> {
            self.ensure_admin()?;
            let ok = !tiers.is_empty()
                && tiers.len() <= MAX_TIERS
                && tiers[0].min_volume == 0
                && tiers.iter().all(|t| t.fee_bps <= 10_000)
                && tiers.windows(2).all(|w| w[0].min_volume < w[1].min_volume && w[1].fee_bps <= w[0].fee_bps);
            if !ok {
                return Err(Error::InvalidTiers);
            }
            let count = tiers.len() as u32;
            self.tiers = tiers;
            self.env().emit_event(FeeTierUpdated { tier_count: count });
            Ok(())
        }

        /// Change the rolling window length. Admin only. Existing buckets are
        /// reinterpreted under the new length.
        #[ink(message)]
        pub fn set_window_blocks(&mut self, window_blocks: u32) -> Result<(), Error> {
            self.ensure_admin()?;
            if window_blocks == 0 {
                return Err(Error::InvalidWindow);
            }
            self.window_blocks = window_blocks;
            Ok(())
        }

        /// Authorise (or revoke) an account that may call `record_volume` (e.g. the DEX).
        #[ink(message)]
        pub fn set_recorder(&mut self, account: AccountId, allowed: bool) -> Result<(), Error> {
            self.ensure_admin()?;
            if allowed {
                self.recorders.insert(account, &true);
            } else {
                self.recorders.remove(account);
            }
            Ok(())
        }

        // ── Internals ──────────────────────────────────────────────────

        fn fee_for(&self, volume: u128) -> u32 {
            let mut fee = self.tiers.first().map(|t| t.fee_bps).unwrap_or(0);
            for tier in self.tiers.iter() {
                if volume >= tier.min_volume {
                    fee = tier.fee_bps;
                }
            }
            fee
        }

        /// Advance the buckets so `bucket_start <= now < bucket_start + window`.
        fn roll(record: &mut VolumeRecord, now: u32, window: u32) {
            let elapsed = now.saturating_sub(record.bucket_start);
            if elapsed < window {
                return;
            }
            let buckets_passed = elapsed / window;
            record.previous = if buckets_passed == 1 { record.current } else { 0 };
            record.current = 0;
            record.bucket_start = record.bucket_start.saturating_add(buckets_passed.saturating_mul(window));
        }

        fn rolling_volume(record: &VolumeRecord, now: u32, window: u32) -> u128 {
            let elapsed = now.saturating_sub(record.bucket_start).min(window) as u128;
            let remaining = window as u128 - elapsed;
            let carried = record.previous.saturating_mul(remaining) / window as u128;
            record.current.saturating_add(carried)
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use ink::env::test;

        type Accounts = test::DefaultAccounts<ink::env::DefaultEnvironment>;

        fn setup() -> (VolumeFeeManager, Accounts) {
            let accounts = test::default_accounts::<ink::env::DefaultEnvironment>();
            test::set_caller::<ink::env::DefaultEnvironment>(accounts.alice);
            test::set_block_number::<ink::env::DefaultEnvironment>(0);
            (VolumeFeeManager::new(), accounts)
        }

        fn block(n: u32) {
            test::set_block_number::<ink::env::DefaultEnvironment>(n);
        }

        /// Register `who` as a tracked trader; the admin (alice) records volume.
        fn tracked(m: &mut VolumeFeeManager, a: &Accounts, who: AccountId) {
            test::set_caller::<ink::env::DefaultEnvironment>(who);
            m.register_volume_tracker().unwrap();
            test::set_caller::<ink::env::DefaultEnvironment>(a.alice);
        }

        fn add(m: &mut VolumeFeeManager, who: AccountId, amount: u128) {
            m.record_volume(who, amount).unwrap();
        }

        // ── Registration ────────────────────────────────────────────────

        #[ink::test]
        fn an_account_registers_once() {
            let (mut m, a) = setup();
            assert!(!m.is_registered(a.bob));
            tracked(&mut m, &a, a.bob);
            assert!(m.is_registered(a.bob));
            test::set_caller::<ink::env::DefaultEnvironment>(a.bob);
            assert_eq!(m.register_volume_tracker(), Err(Error::AlreadyRegistered));
        }

        #[ink::test]
        fn recording_needs_a_registered_account_a_positive_amount_and_authorisation() {
            let (mut m, a) = setup();
            assert_eq!(m.record_volume(a.bob, 1), Err(Error::NotRegistered));
            tracked(&mut m, &a, a.bob);
            assert_eq!(m.record_volume(a.bob, 0), Err(Error::ZeroAmount));
            test::set_caller::<ink::env::DefaultEnvironment>(a.charlie);
            assert_eq!(m.record_volume(a.bob, 5), Err(Error::Unauthorized));
            test::set_caller::<ink::env::DefaultEnvironment>(a.alice);
            m.set_recorder(a.charlie, true).unwrap();
            test::set_caller::<ink::env::DefaultEnvironment>(a.charlie);
            assert!(m.record_volume(a.bob, 5).is_ok());
            test::set_caller::<ink::env::DefaultEnvironment>(a.alice);
            m.set_recorder(a.charlie, false).unwrap();
            test::set_caller::<ink::env::DefaultEnvironment>(a.charlie);
            assert_eq!(m.record_volume(a.bob, 5), Err(Error::Unauthorized));
        }

        // ── Tiers ───────────────────────────────────────────────────────

        #[ink::test]
        fn the_default_schedule_is_30_25_20_15_bps() {
            let (m, _) = setup();
            let t = m.get_fee_tiers();
            assert_eq!(
                t.iter().map(|x| (x.min_volume, x.fee_bps)).collect::<Vec<_>>(),
                vec![(0, 30), (10_000, 25), (50_000, 20), (100_000, 15)]
            );
            assert_eq!(m.get_window_blocks(), 14_400);
        }

        #[ink::test]
        fn untracked_accounts_pay_the_base_rate() {
            let (m, a) = setup();
            assert_eq!(m.get_fee_rate(a.eve), 30);
            assert_eq!(m.get_rolling_volume(a.eve), 0);
        }

        #[ink::test]
        fn tier_boundaries_are_inclusive_at_the_lower_bound() {
            let (mut m, a) = setup();
            tracked(&mut m, &a, a.bob);
            assert_eq!(m.get_fee_rate(a.bob), 30);
            add(&mut m, a.bob, 9_999);
            assert_eq!(m.get_fee_rate(a.bob), 30);
            add(&mut m, a.bob, 1); // 10_000
            assert_eq!(m.get_fee_rate(a.bob), 25);
            add(&mut m, a.bob, 39_999); // 49_999
            assert_eq!(m.get_fee_rate(a.bob), 25);
            add(&mut m, a.bob, 1); // 50_000
            assert_eq!(m.get_fee_rate(a.bob), 20);
            add(&mut m, a.bob, 50_000); // 100_000
            assert_eq!(m.get_fee_rate(a.bob), 15);
            add(&mut m, a.bob, 1_000_000_000);
            assert_eq!(m.get_fee_rate(a.bob), 15);
        }

        #[ink::test]
        fn accounts_have_independent_volume() {
            let (mut m, a) = setup();
            tracked(&mut m, &a, a.bob);
            tracked(&mut m, &a, a.charlie);
            add(&mut m, a.bob, 200_000);
            assert_eq!(m.get_fee_rate(a.bob), 15);
            assert_eq!(m.get_fee_rate(a.charlie), 30);
        }

        #[ink::test]
        fn saturating_volume_does_not_overflow() {
            let (mut m, a) = setup();
            tracked(&mut m, &a, a.bob);
            add(&mut m, a.bob, u128::MAX);
            add(&mut m, a.bob, 10);
            assert_eq!(m.get_rolling_volume(a.bob), u128::MAX);
            assert_eq!(m.get_fee_rate(a.bob), 15);
        }

        // ── Rolling window ──────────────────────────────────────────────

        #[ink::test]
        fn volume_ages_out_and_the_fee_returns_to_the_base_rate() {
            let (mut m, a) = setup();
            tracked(&mut m, &a, a.bob);
            add(&mut m, a.bob, 100_000);
            assert_eq!(m.get_fee_rate(a.bob), 15);

            // Half a window after the next bucket starts, half of it is still counted.
            block(14_400 + 7_200);
            assert_eq!(m.get_rolling_volume(a.bob), 50_000);
            assert_eq!(m.get_fee_rate(a.bob), 20);

            // Two full windows later nothing remains.
            block(2 * 14_400);
            assert_eq!(m.get_rolling_volume(a.bob), 0);
            assert_eq!(m.get_fee_rate(a.bob), 30);
        }

        #[ink::test]
        fn volume_decays_smoothly_across_the_window_boundary() {
            let (mut m, a) = setup();
            tracked(&mut m, &a, a.bob);
            add(&mut m, a.bob, 40_000);
            // At the exact start of the next bucket the whole previous bucket still counts.
            block(14_400);
            assert_eq!(m.get_rolling_volume(a.bob), 40_000);
            // A quarter of a window later three quarters remain.
            block(14_400 + 3_600);
            assert_eq!(m.get_rolling_volume(a.bob), 30_000);
        }

        #[ink::test]
        fn new_volume_adds_to_the_decayed_total() {
            let (mut m, a) = setup();
            tracked(&mut m, &a, a.bob);
            add(&mut m, a.bob, 40_000);
            block(14_400 + 7_200); // 20_000 carried over
            add(&mut m, a.bob, 5_000);
            assert_eq!(m.get_rolling_volume(a.bob), 25_000);
        }

        #[ink::test]
        fn a_gap_of_more_than_two_windows_clears_everything() {
            let (mut m, a) = setup();
            tracked(&mut m, &a, a.bob);
            add(&mut m, a.bob, 500_000);
            block(3 * 14_400);
            add(&mut m, a.bob, 1);
            assert_eq!(m.get_rolling_volume(a.bob), 1);
            assert_eq!(m.get_fee_rate(a.bob), 30);
        }

        #[ink::test]
        fn the_window_length_is_configurable() {
            let (mut m, a) = setup();
            assert_eq!(m.set_window_blocks(0), Err(Error::InvalidWindow));
            m.set_window_blocks(100).unwrap();
            tracked(&mut m, &a, a.bob);
            add(&mut m, a.bob, 100_000);
            block(200);
            assert_eq!(m.get_rolling_volume(a.bob), 0);
            test::set_caller::<ink::env::DefaultEnvironment>(a.bob);
            assert_eq!(m.set_window_blocks(10), Err(Error::Unauthorized));
        }

        // ── Tier configuration ──────────────────────────────────────────

        #[ink::test]
        fn the_admin_replaces_the_schedule_and_rates_follow() {
            let (mut m, a) = setup();
            tracked(&mut m, &a, a.bob);
            add(&mut m, a.bob, 1_000);
            assert_eq!(m.get_fee_rate(a.bob), 30);
            m.set_fee_tiers(vec![
                FeeTier { min_volume: 0, fee_bps: 50 },
                FeeTier { min_volume: 500, fee_bps: 10 },
            ])
            .unwrap();
            assert_eq!(m.get_fee_rate(a.bob), 10);
            assert_eq!(m.get_fee_tiers().len(), 2);
        }

        #[ink::test]
        fn invalid_schedules_are_rejected_and_leave_the_old_one() {
            let (mut m, a) = setup();
            let before = m.get_fee_tiers();
            let bad: Vec<Vec<FeeTier>> = vec![
                vec![],
                vec![FeeTier { min_volume: 1, fee_bps: 30 }], // must start at 0
                vec![FeeTier { min_volume: 0, fee_bps: 30 }, FeeTier { min_volume: 0, fee_bps: 20 }], // not increasing
                vec![FeeTier { min_volume: 0, fee_bps: 20 }, FeeTier { min_volume: 10, fee_bps: 30 }], // fee rises
                vec![FeeTier { min_volume: 0, fee_bps: 10_001 }],
                (0..9u128).map(|i| FeeTier { min_volume: i * 10, fee_bps: 30 - i as u32 }).collect(), // too many
            ];
            for tiers in bad {
                assert_eq!(m.set_fee_tiers(tiers), Err(Error::InvalidTiers));
            }
            assert_eq!(m.get_fee_tiers(), before);

            test::set_caller::<ink::env::DefaultEnvironment>(a.bob);
            assert_eq!(
                m.set_fee_tiers(vec![FeeTier { min_volume: 0, fee_bps: 1 }]),
                Err(Error::Unauthorized)
            );
        }

        // ── Events ──────────────────────────────────────────────────────

        #[ink::test]
        fn events_report_volume_and_tier_changes_only_when_the_tier_moves() {
            let (mut m, a) = setup();
            tracked(&mut m, &a, a.bob);
            let base = test::recorded_events().count();
            add(&mut m, a.bob, 100); // VolumeRecorded only
            assert_eq!(test::recorded_events().count() - base, 1);
            add(&mut m, a.bob, 10_000); // VolumeRecorded + FeeTierChanged
            assert_eq!(test::recorded_events().count() - base, 3);
            m.set_fee_tiers(vec![FeeTier { min_volume: 0, fee_bps: 5 }]).unwrap(); // FeeTierUpdated
            assert_eq!(test::recorded_events().count() - base, 4);
        }
    }
}
