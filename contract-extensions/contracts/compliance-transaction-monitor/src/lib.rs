#![cfg_attr(not(feature = "std"), no_std, no_main)]
#![allow(clippy::new_without_default)]

//! # Compliance transaction monitor (#1443)
//!
//! Real-time monitoring rules that sit next to the compliance registry. A
//! caller (the admin or an authorised reporter, typically a hook in the
//! contract that moves funds) reports each transaction with
//! [`TransactionMonitor::record_transaction`]; the monitor evaluates it against
//! the registered rules, flags it, and accumulates a risk score per sender.
//! [`TransactionMonitor::evaluate_transaction`] is the read-only version: it
//! reports what *would* be flagged without changing any state.
//!
//! ## Rules (one per type; registering again replaces it)
//!
//! | Rule | Flags a transaction when |
//! |---|---|
//! | `VelocityLimit { max_txs, window_blocks }` | the sender would exceed `max_txs` transactions inside the current window |
//! | `RoundTrip { window_blocks, tolerance_bps }` | the receiver sent roughly this amount to the sender within the window (funds going out and coming back) |
//! | `StructuringDetection { threshold, min_count, window_blocks }` | the amount is *near but below* `threshold` (at least half of it) and this is the `min_count`-th such transaction in the window |
//! | `HighValue { threshold }` | `amount >= threshold` |
//!
//! Each rule carries a `risk_weight`; a transaction's score is the sum of the
//! weights of the rules it triggers, capped at 100. An account's risk is the
//! saturating sum of the scores of the transactions it *sent*.
//!
//! ## Design notes
//!
//! * **Gas-efficient state.** Windows are tracked as one `(window_start, count)`
//!   pair per account per rule, not as a transaction list. Windows are
//!   tumbling (a new window starts once `window_blocks` have passed), so a burst
//!   straddling a boundary is counted in two windows; that is the price of O(1)
//!   storage.
//! * **No on-chain call to the compliance registry or a sanctions list.** The
//!   monitor is self-contained; wiring it to `contracts/compliance_registry`
//!   (for sanctions lookups) is left to the integration layer.
//! * Time is `env().block_number()`.

#[ink::contract]
pub mod transaction_monitor {
    use ink::prelude::vec::Vec;
    use ink::storage::Mapping;

    /// Highest risk score a single transaction can receive.
    pub const MAX_TX_RISK: u32 = 100;

    #[derive(Debug, Clone, Copy, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout))]
    pub enum RuleType {
        VelocityLimit,
        RoundTripDetection,
        StructuringDetection,
        HighValueFlag,
    }

    /// Parameters for a rule. The variant must match the `RuleType` it is
    /// registered under.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout))]
    pub enum RuleParameters {
        VelocityLimit { max_txs: u32, window_blocks: u32 },
        RoundTrip { window_blocks: u32, tolerance_bps: u32 },
        Structuring { threshold: u128, min_count: u32, window_blocks: u32 },
        HighValue { threshold: u128 },
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout))]
    pub struct MonitoringRule {
        pub parameters: RuleParameters,
        pub risk_weight: u32,
        pub active: bool,
    }

    /// Result of evaluating one transaction.
    #[derive(Debug, Clone, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub struct Evaluation {
        pub risk_score: u32,
        pub flags: Vec<RuleType>,
    }

    #[derive(Debug, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub enum Error {
        Unauthorized,
        /// The parameters do not match the rule type, or contain a zero window / limit.
        InvalidParameters,
        /// `risk_weight` above [`MAX_TX_RISK`].
        InvalidWeight,
        RuleNotFound,
    }

    /// A tumbling window counter.
    #[derive(Debug, Clone, Copy, Default, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout))]
    struct WindowCounter {
        window_start: u32,
        count: u32,
    }

    impl WindowCounter {
        /// The count that applies at `now` (zero once the window has lapsed).
        fn current(&self, now: u32, window_blocks: u32) -> u32 {
            if self.count == 0 || now.saturating_sub(self.window_start) >= window_blocks {
                0
            } else {
                self.count
            }
        }

        fn bump(&mut self, now: u32, window_blocks: u32) {
            let live = self.current(now, window_blocks);
            if live == 0 {
                self.window_start = now;
                self.count = 1;
            } else {
                self.count = live.saturating_add(1);
            }
        }
    }

    #[ink(event)]
    pub struct RuleRegistered {
        #[ink(topic)]
        pub rule_type: RuleType,
        pub risk_weight: u32,
    }

    #[ink(event)]
    pub struct RuleDeactivated {
        #[ink(topic)]
        pub rule_type: RuleType,
    }

    #[ink(event)]
    pub struct TransactionFlagged {
        #[ink(topic)]
        pub sender: AccountId,
        #[ink(topic)]
        pub receiver: AccountId,
        pub amount: u128,
        pub rule: RuleType,
        pub risk_score: u32,
    }

    #[ink(storage)]
    pub struct TransactionMonitor {
        admin: AccountId,
        reporters: Mapping<AccountId, bool>,
        rules: Mapping<RuleType, MonitoringRule>,
        velocity: Mapping<AccountId, WindowCounter>,
        structuring: Mapping<AccountId, WindowCounter>,
        /// Last outflow per ordered pair `(from, to)`: `(amount, block)`.
        outflows: Mapping<(AccountId, AccountId), (u128, u32)>,
        account_risk: Mapping<AccountId, u32>,
        flagged_count: Mapping<AccountId, u32>,
    }

    impl TransactionMonitor {
        #[ink(constructor)]
        pub fn new() -> Self {
            Self {
                admin: Self::env().caller(),
                reporters: Mapping::default(),
                rules: Mapping::default(),
                velocity: Mapping::default(),
                structuring: Mapping::default(),
                outflows: Mapping::default(),
                account_risk: Mapping::default(),
                flagged_count: Mapping::default(),
            }
        }

        // ── Administration ─────────────────────────────────────────────

        fn ensure_admin(&self) -> Result<(), Error> {
            if self.env().caller() != self.admin {
                return Err(Error::Unauthorized);
            }
            Ok(())
        }

        /// Define (or replace) the rule of `rule_type`. Admin only.
        #[ink(message)]
        pub fn register_monitoring_rule(
            &mut self,
            rule_type: RuleType,
            parameters: RuleParameters,
            risk_weight: u32,
        ) -> Result<(), Error> {
            self.ensure_admin()?;
            if risk_weight == 0 || risk_weight > MAX_TX_RISK {
                return Err(Error::InvalidWeight);
            }
            let valid = match (rule_type, parameters) {
                (RuleType::VelocityLimit, RuleParameters::VelocityLimit { max_txs, window_blocks }) => {
                    max_txs > 0 && window_blocks > 0
                }
                (
                    RuleType::RoundTripDetection,
                    RuleParameters::RoundTrip { window_blocks, tolerance_bps },
                ) => window_blocks > 0 && tolerance_bps <= 10_000,
                (
                    RuleType::StructuringDetection,
                    RuleParameters::Structuring { threshold, min_count, window_blocks },
                ) => threshold > 0 && min_count >= 2 && window_blocks > 0,
                (RuleType::HighValueFlag, RuleParameters::HighValue { threshold }) => threshold > 0,
                _ => false,
            };
            if !valid {
                return Err(Error::InvalidParameters);
            }
            self.rules.insert(
                rule_type,
                &MonitoringRule { parameters, risk_weight, active: true },
            );
            self.env().emit_event(RuleRegistered { rule_type, risk_weight });
            Ok(())
        }

        /// Switch a rule off without deleting it. Admin only.
        #[ink(message)]
        pub fn deactivate_rule(&mut self, rule_type: RuleType) -> Result<(), Error> {
            self.ensure_admin()?;
            let mut rule = self.rules.get(rule_type).ok_or(Error::RuleNotFound)?;
            rule.active = false;
            self.rules.insert(rule_type, &rule);
            self.env().emit_event(RuleDeactivated { rule_type });
            Ok(())
        }

        /// Authorise (or revoke) an account that may report transactions. Admin only.
        #[ink(message)]
        pub fn set_reporter(&mut self, account: AccountId, allowed: bool) -> Result<(), Error> {
            self.ensure_admin()?;
            if allowed {
                self.reporters.insert(account, &true);
            } else {
                self.reporters.remove(account);
            }
            Ok(())
        }

        // ── Queries ────────────────────────────────────────────────────

        #[ink(message)]
        pub fn get_rule(&self, rule_type: RuleType) -> Option<MonitoringRule> {
            self.rules.get(rule_type)
        }

        /// Accumulated risk of the transactions `account` has sent.
        #[ink(message)]
        pub fn get_account_risk(&self, account: AccountId) -> u32 {
            self.account_risk.get(account).unwrap_or(0)
        }

        /// How many of `account`'s sent transactions have been flagged.
        #[ink(message)]
        pub fn get_flagged_count(&self, account: AccountId) -> u32 {
            self.flagged_count.get(account).unwrap_or(0)
        }

        /// What recording this transaction *would* flag. Changes no state.
        ///
        /// `token` is accepted for interface parity with the issue but is not part
        /// of any rule: limits are per account regardless of token.
        #[ink(message)]
        pub fn evaluate_transaction(
            &self,
            sender: AccountId,
            receiver: AccountId,
            amount: u128,
            _token: Option<AccountId>,
        ) -> Evaluation {
            self.assess(sender, receiver, amount)
        }

        // ── Recording ──────────────────────────────────────────────────

        /// Report a transaction: evaluate it, update the windows, add to the
        /// sender's risk and emit one `TransactionFlagged` per rule triggered.
        /// Admin or an authorised reporter only.
        #[ink(message)]
        pub fn record_transaction(
            &mut self,
            sender: AccountId,
            receiver: AccountId,
            amount: u128,
            _token: Option<AccountId>,
        ) -> Result<Evaluation, Error> {
            let caller = self.env().caller();
            if caller != self.admin && !self.reporters.get(caller).unwrap_or(false) {
                return Err(Error::Unauthorized);
            }
            let evaluation = self.assess(sender, receiver, amount);
            let now = self.env().block_number();

            if let Some(rule) = self.active_rule(RuleType::VelocityLimit) {
                if let RuleParameters::VelocityLimit { window_blocks, .. } = rule.parameters {
                    let mut c = self.velocity.get(sender).unwrap_or_default();
                    c.bump(now, window_blocks);
                    self.velocity.insert(sender, &c);
                }
            }
            if let Some(rule) = self.active_rule(RuleType::StructuringDetection) {
                if let RuleParameters::Structuring { threshold, window_blocks, .. } = rule.parameters {
                    if Self::is_near_threshold(amount, threshold) {
                        let mut c = self.structuring.get(sender).unwrap_or_default();
                        c.bump(now, window_blocks);
                        self.structuring.insert(sender, &c);
                    }
                }
            }
            if self.active_rule(RuleType::RoundTripDetection).is_some() {
                self.outflows.insert((sender, receiver), &(amount, now));
            }

            if evaluation.risk_score > 0 {
                let total = self.get_account_risk(sender).saturating_add(evaluation.risk_score);
                self.account_risk.insert(sender, &total);
                let n = self.get_flagged_count(sender).saturating_add(1);
                self.flagged_count.insert(sender, &n);
                for rule in evaluation.flags.iter() {
                    self.env().emit_event(TransactionFlagged {
                        sender,
                        receiver,
                        amount,
                        rule: *rule,
                        risk_score: evaluation.risk_score,
                    });
                }
            }
            Ok(evaluation)
        }

        // ── Internals ──────────────────────────────────────────────────

        fn active_rule(&self, rule_type: RuleType) -> Option<MonitoringRule> {
            self.rules.get(rule_type).filter(|r| r.active)
        }

        /// Between half the threshold (inclusive) and the threshold (exclusive).
        fn is_near_threshold(amount: u128, threshold: u128) -> bool {
            amount < threshold && amount >= threshold / 2
        }

        fn assess(&self, sender: AccountId, receiver: AccountId, amount: u128) -> Evaluation {
            let now = self.env().block_number();
            let mut score: u32 = 0;
            let mut flags = Vec::new();
            let mut add = |rule_type: RuleType, weight: u32| {
                flags.push(rule_type);
                score = score.saturating_add(weight);
            };

            if let Some(rule) = self.active_rule(RuleType::VelocityLimit) {
                if let RuleParameters::VelocityLimit { max_txs, window_blocks } = rule.parameters {
                    let live = self.velocity.get(sender).unwrap_or_default().current(now, window_blocks);
                    if live.saturating_add(1) > max_txs {
                        add(RuleType::VelocityLimit, rule.risk_weight);
                    }
                }
            }
            if let Some(rule) = self.active_rule(RuleType::RoundTripDetection) {
                if let RuleParameters::RoundTrip { window_blocks, tolerance_bps } = rule.parameters {
                    // Did the receiver recently send roughly this much to the sender?
                    if let Some((prev, at)) = self.outflows.get((receiver, sender)) {
                        let within = now.saturating_sub(at) <= window_blocks;
                        let diff = prev.abs_diff(amount);
                        let close = diff.saturating_mul(10_000) <= prev.saturating_mul(tolerance_bps as u128);
                        if within && close && prev > 0 {
                            add(RuleType::RoundTripDetection, rule.risk_weight);
                        }
                    }
                }
            }
            if let Some(rule) = self.active_rule(RuleType::StructuringDetection) {
                if let RuleParameters::Structuring { threshold, min_count, window_blocks } = rule.parameters {
                    if Self::is_near_threshold(amount, threshold) {
                        let live = self.structuring.get(sender).unwrap_or_default().current(now, window_blocks);
                        if live.saturating_add(1) >= min_count {
                            add(RuleType::StructuringDetection, rule.risk_weight);
                        }
                    }
                }
            }
            if let Some(rule) = self.active_rule(RuleType::HighValueFlag) {
                if let RuleParameters::HighValue { threshold } = rule.parameters {
                    if amount >= threshold {
                        add(RuleType::HighValueFlag, rule.risk_weight);
                    }
                }
            }
            Evaluation { risk_score: score.min(MAX_TX_RISK), flags }
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use ink::env::test;

        type Accounts = test::DefaultAccounts<ink::env::DefaultEnvironment>;

        fn setup() -> (TransactionMonitor, Accounts) {
            let accounts = test::default_accounts::<ink::env::DefaultEnvironment>();
            test::set_caller::<ink::env::DefaultEnvironment>(accounts.alice);
            test::set_block_number::<ink::env::DefaultEnvironment>(1);
            (TransactionMonitor::new(), accounts)
        }

        fn block(n: u32) {
            test::set_block_number::<ink::env::DefaultEnvironment>(n);
        }

        fn velocity(m: &mut TransactionMonitor, max: u32, window: u32, weight: u32) {
            m.register_monitoring_rule(
                RuleType::VelocityLimit,
                RuleParameters::VelocityLimit { max_txs: max, window_blocks: window },
                weight,
            )
            .unwrap();
        }

        fn record(m: &mut TransactionMonitor, a: AccountId, b: AccountId, amount: u128) -> Evaluation {
            m.record_transaction(a, b, amount, None).unwrap()
        }

        // ── Rule registration ───────────────────────────────────────────

        #[ink::test]
        fn the_admin_registers_and_replaces_rules() {
            let (mut m, _) = setup();
            assert!(m.get_rule(RuleType::VelocityLimit).is_none());
            velocity(&mut m, 3, 100, 40);
            let rule = m.get_rule(RuleType::VelocityLimit).unwrap();
            assert_eq!(rule.risk_weight, 40);
            assert!(rule.active);
            velocity(&mut m, 5, 200, 60);
            assert_eq!(m.get_rule(RuleType::VelocityLimit).unwrap().risk_weight, 60);
        }

        #[ink::test]
        fn registration_is_admin_only_and_validates_parameters() {
            let (mut m, accounts) = setup();
            test::set_caller::<ink::env::DefaultEnvironment>(accounts.bob);
            assert_eq!(
                m.register_monitoring_rule(
                    RuleType::HighValueFlag,
                    RuleParameters::HighValue { threshold: 1 },
                    10
                ),
                Err(Error::Unauthorized)
            );
            test::set_caller::<ink::env::DefaultEnvironment>(accounts.alice);
            // Mismatched variant, zero limits, bad weight.
            assert_eq!(
                m.register_monitoring_rule(RuleType::VelocityLimit, RuleParameters::HighValue { threshold: 1 }, 10),
                Err(Error::InvalidParameters)
            );
            assert_eq!(
                m.register_monitoring_rule(
                    RuleType::VelocityLimit,
                    RuleParameters::VelocityLimit { max_txs: 0, window_blocks: 10 },
                    10
                ),
                Err(Error::InvalidParameters)
            );
            assert_eq!(
                m.register_monitoring_rule(
                    RuleType::StructuringDetection,
                    RuleParameters::Structuring { threshold: 100, min_count: 1, window_blocks: 10 },
                    10
                ),
                Err(Error::InvalidParameters)
            );
            assert_eq!(
                m.register_monitoring_rule(
                    RuleType::HighValueFlag,
                    RuleParameters::HighValue { threshold: 5 },
                    0
                ),
                Err(Error::InvalidWeight)
            );
            assert_eq!(
                m.register_monitoring_rule(
                    RuleType::HighValueFlag,
                    RuleParameters::HighValue { threshold: 5 },
                    101
                ),
                Err(Error::InvalidWeight)
            );
        }

        #[ink::test]
        fn only_the_admin_or_a_reporter_can_record_transactions() {
            let (mut m, accounts) = setup();
            test::set_caller::<ink::env::DefaultEnvironment>(accounts.bob);
            assert_eq!(
                m.record_transaction(accounts.charlie, accounts.django, 1, None),
                Err(Error::Unauthorized)
            );
            test::set_caller::<ink::env::DefaultEnvironment>(accounts.alice);
            m.set_reporter(accounts.bob, true).unwrap();
            test::set_caller::<ink::env::DefaultEnvironment>(accounts.bob);
            assert!(m.record_transaction(accounts.charlie, accounts.django, 1, None).is_ok());
        }

        // ── Velocity limit ──────────────────────────────────────────────

        #[ink::test]
        fn velocity_flags_the_transaction_that_exceeds_the_limit() {
            let (mut m, a) = setup();
            velocity(&mut m, 3, 100, 40);
            for _ in 0..3 {
                assert!(record(&mut m, a.bob, a.charlie, 10).flags.is_empty());
            }
            let fourth = record(&mut m, a.bob, a.charlie, 10);
            assert_eq!(fourth.flags, vec![RuleType::VelocityLimit]);
            assert_eq!(fourth.risk_score, 40);
        }

        #[ink::test]
        fn the_velocity_window_resets_and_accounts_are_independent() {
            let (mut m, a) = setup();
            velocity(&mut m, 2, 100, 30);
            record(&mut m, a.bob, a.charlie, 1);
            record(&mut m, a.bob, a.charlie, 1);
            assert!(!record(&mut m, a.bob, a.charlie, 1).flags.is_empty());
            // A different sender is unaffected.
            assert!(record(&mut m, a.eve, a.charlie, 1).flags.is_empty());
            // Once the window has passed bob starts fresh.
            block(1 + 100);
            assert!(record(&mut m, a.bob, a.charlie, 1).flags.is_empty());
        }

        // ── Round trip ──────────────────────────────────────────────────

        fn round_trip(m: &mut TransactionMonitor, window: u32, tolerance: u32) {
            m.register_monitoring_rule(
                RuleType::RoundTripDetection,
                RuleParameters::RoundTrip { window_blocks: window, tolerance_bps: tolerance },
                50,
            )
            .unwrap();
        }

        #[ink::test]
        fn funds_returned_within_the_window_are_flagged() {
            let (mut m, a) = setup();
            round_trip(&mut m, 50, 500); // 5% tolerance
            assert!(record(&mut m, a.bob, a.charlie, 10_000).flags.is_empty());
            block(30);
            let back = record(&mut m, a.charlie, a.bob, 9_800); // within 5%
            assert_eq!(back.flags, vec![RuleType::RoundTripDetection]);
            assert_eq!(m.get_account_risk(a.charlie), 50);
        }

        #[ink::test]
        fn a_return_outside_the_window_or_tolerance_is_not_flagged() {
            let (mut m, a) = setup();
            round_trip(&mut m, 50, 500);
            record(&mut m, a.bob, a.charlie, 10_000);
            // Different amount (more than 5% away).
            block(10);
            assert!(record(&mut m, a.charlie, a.bob, 5_000).flags.is_empty());
            // Right amount but after the window.
            block(1 + 51);
            assert!(record(&mut m, a.charlie, a.bob, 10_000).flags.is_empty());
        }

        #[ink::test]
        fn the_round_trip_window_boundary_is_inclusive() {
            let (mut m, a) = setup();
            round_trip(&mut m, 50, 0);
            record(&mut m, a.bob, a.charlie, 777); // block 1
            block(1 + 50);
            assert_eq!(
                m.evaluate_transaction(a.charlie, a.bob, 777, None).flags,
                vec![RuleType::RoundTripDetection]
            );
            block(1 + 51);
            assert!(m.evaluate_transaction(a.charlie, a.bob, 777, None).flags.is_empty());
        }

        // ── Structuring ─────────────────────────────────────────────────

        fn structuring(m: &mut TransactionMonitor) {
            m.register_monitoring_rule(
                RuleType::StructuringDetection,
                RuleParameters::Structuring { threshold: 10_000, min_count: 3, window_blocks: 100 },
                70,
            )
            .unwrap();
        }

        #[ink::test]
        fn repeated_just_below_threshold_transfers_are_flagged_on_the_nth() {
            let (mut m, a) = setup();
            structuring(&mut m);
            assert!(record(&mut m, a.bob, a.charlie, 9_500).flags.is_empty());
            assert!(record(&mut m, a.bob, a.charlie, 9_900).flags.is_empty());
            let third = record(&mut m, a.bob, a.charlie, 9_000);
            assert_eq!(third.flags, vec![RuleType::StructuringDetection]);
            assert_eq!(third.risk_score, 70);
        }

        #[ink::test]
        fn small_or_over_threshold_transfers_do_not_count_towards_structuring() {
            let (mut m, a) = setup();
            structuring(&mut m);
            for amount in [100u128, 4_999, 10_000, 50_000, 6_000, 6_000] {
                let eval = record(&mut m, a.bob, a.charlie, amount);
                assert!(!eval.flags.contains(&RuleType::StructuringDetection), "amount {amount}");
            }
        }

        #[ink::test]
        fn the_structuring_window_expires() {
            let (mut m, a) = setup();
            structuring(&mut m);
            record(&mut m, a.bob, a.charlie, 9_000);
            record(&mut m, a.bob, a.charlie, 9_000);
            block(1 + 100);
            assert!(record(&mut m, a.bob, a.charlie, 9_000).flags.is_empty());
        }

        // ── High value ──────────────────────────────────────────────────

        #[ink::test]
        fn high_value_transactions_are_flagged_at_the_threshold() {
            let (mut m, a) = setup();
            m.register_monitoring_rule(
                RuleType::HighValueFlag,
                RuleParameters::HighValue { threshold: 1_000_000 },
                25,
            )
            .unwrap();
            assert!(m.evaluate_transaction(a.bob, a.charlie, 999_999, None).flags.is_empty());
            let at = m.evaluate_transaction(a.bob, a.charlie, 1_000_000, None);
            assert_eq!(at.flags, vec![RuleType::HighValueFlag]);
            assert_eq!(at.risk_score, 25);
        }

        // ── Risk accumulation and false positives ───────────────────────

        #[ink::test]
        fn risk_accumulates_per_sender_and_is_capped_per_transaction() {
            let (mut m, a) = setup();
            m.register_monitoring_rule(RuleType::HighValueFlag, RuleParameters::HighValue { threshold: 100 }, 60).unwrap();
            velocity(&mut m, 1, 100, 60);
            record(&mut m, a.bob, a.charlie, 100); // high value only: 60
            let second = record(&mut m, a.bob, a.charlie, 100); // both rules: 120 capped to 100
            assert_eq!(second.risk_score, MAX_TX_RISK);
            assert_eq!(m.get_account_risk(a.bob), 60 + 100);
            assert_eq!(m.get_flagged_count(a.bob), 2);
            assert_eq!(m.get_account_risk(a.charlie), 0);
        }

        #[ink::test]
        fn ordinary_activity_triggers_nothing_when_every_rule_is_enabled() {
            let (mut m, a) = setup();
            velocity(&mut m, 5, 100, 40);
            round_trip(&mut m, 20, 100);
            structuring(&mut m);
            m.register_monitoring_rule(RuleType::HighValueFlag, RuleParameters::HighValue { threshold: 1_000_000 }, 25).unwrap();
            // Three modest payments to different people, spread over time.
            for (i, to) in [a.charlie, a.django, a.eve].into_iter().enumerate() {
                block(1 + i as u32 * 40);
                let eval = record(&mut m, a.bob, to, 2_500 + i as u128 * 100);
                assert!(eval.flags.is_empty(), "false positive on payment {i}");
            }
            assert_eq!(m.get_account_risk(a.bob), 0);
        }

        #[ink::test]
        fn evaluation_is_read_only_and_matches_what_recording_then_reports() {
            let (mut m, a) = setup();
            velocity(&mut m, 1, 100, 40);
            record(&mut m, a.bob, a.charlie, 5);
            let before = (m.get_account_risk(a.bob), m.get_flagged_count(a.bob));
            let preview = m.evaluate_transaction(a.bob, a.charlie, 5, None);
            assert_eq!((m.get_account_risk(a.bob), m.get_flagged_count(a.bob)), before);
            assert_eq!(record(&mut m, a.bob, a.charlie, 5), preview);
        }

        #[ink::test]
        fn deactivated_rules_stop_flagging_and_unknown_rules_are_reported() {
            let (mut m, a) = setup();
            assert_eq!(m.deactivate_rule(RuleType::VelocityLimit), Err(Error::RuleNotFound));
            velocity(&mut m, 1, 100, 40);
            record(&mut m, a.bob, a.charlie, 1);
            assert!(!m.evaluate_transaction(a.bob, a.charlie, 1, None).flags.is_empty());
            m.deactivate_rule(RuleType::VelocityLimit).unwrap();
            assert!(m.evaluate_transaction(a.bob, a.charlie, 1, None).flags.is_empty());
        }

        #[ink::test]
        fn a_flagged_transaction_emits_one_event_per_triggered_rule() {
            let (mut m, a) = setup();
            m.register_monitoring_rule(RuleType::HighValueFlag, RuleParameters::HighValue { threshold: 100 }, 10).unwrap();
            velocity(&mut m, 1, 100, 10);
            let base = test::recorded_events().count();
            record(&mut m, a.bob, a.charlie, 100); // high value only
            assert_eq!(test::recorded_events().count() - base, 1);
            record(&mut m, a.bob, a.charlie, 100); // high value + velocity
            assert_eq!(test::recorded_events().count() - base, 3);
        }
    }
}
