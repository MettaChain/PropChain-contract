#![cfg_attr(not(feature = "std"), no_std, no_main)]
#![allow(clippy::new_without_default)]

//! # LMSR prediction market (#1442)
//!
//! An automated market maker using **Hanson's Logarithmic Market Scoring Rule**.
//! With outstanding shares `q_i` per outcome and liquidity parameter `b`:
//!
//! ```text
//! C(q)  = b * ln( sum_i exp(q_i / b) )          cost function
//! p_i   = exp(q_i / b) / sum_j exp(q_j / b)     instantaneous price (probability)
//! cost of moving q -> q' is C(q') - C(q)
//! ```
//!
//! The market maker's worst-case loss is `b * ln(n)` for `n` outcomes, so the
//! liquidity parameter is derived from the funding: `b = funding / ln(n)`.
//!
//! * **Buying** with a budget `inv` solves `C(q + d*e_i) - C(q) = inv` for the
//!   shares `d` in closed form:
//!   `exp((q_i + d)/b) = exp((C(q) + inv)/b) - sum_{j != i} exp(q_j/b)`.
//! * **Selling** `d` shares pays `C(q) - C(q - d*e_i)`.
//! * After resolution a winning share redeems for **1 unit**, losing shares for 0.
//!
//! ## `exp` and `ln` without floating point (documented approximation)
//!
//! `no_std` contracts have no `f64::exp`/`ln`, so both are fixed-point with scale
//! [`SCALE`] = 10^9:
//!
//! * `exp(x)`: reduce `x = k*ln2 + r` with `|r| <= ln2/2`, evaluate `exp(r)` with a
//!   13-term Taylor series, then scale by `2^k` (shift).
//! * `ln(y)`: normalise `y = m * 2^e` with `m` in `[1, 2)`, then
//!   `ln(m) = 2 * atanh((m-1)/(m+1))` with an 11-term series, and add `e*ln2`.
//!
//! Both are accurate to roughly 1e-8 relative error over the working range (the
//! tests assert 1e-6). To avoid overflow, all exponentials are **normalised by the
//! largest outstanding quantity** (log-sum-exp), so every `exp` argument is
//! `<= 0`, and the buy budget is capped at `MAX_BUDGET_OVER_B * b`.
//!
//! ## Money model
//!
//! Money and shares share one integer unit (a share pays one unit). Cash is the
//! native value sent with payable calls and paid out with `transfer`.
//!
//! ## Limits
//!
//! Between 2 and 4 outcomes. Liquidity can only be added before the first trade
//! (changing `b` mid-market would reprice outstanding shares). Rounding can leave
//! a few units of dust in the pool.

#[ink::contract]
pub mod lmsr_prediction_market {
    use ink::prelude::vec::Vec;
    use ink::storage::Mapping;

    /// Fixed-point scale of the math helpers.
    pub const SCALE: u128 = 1_000_000_000;
    /// `ln 2` at `SCALE`.
    const LN2: i128 = 693_147_181;
    /// Largest `budget / b` ratio accepted by `buy_shares`.
    pub const MAX_BUDGET_OVER_B: u128 = 25;
    pub const MAX_OUTCOMES: u32 = 4;

    /// `e^x` for `x` (scaled by `SCALE`) in `[-40, 40]`; returns a `SCALE`-scaled value.
    pub fn exp_fp(x: i128) -> u128 {
        let limit = 40 * SCALE as i128;
        let x = x.clamp(-limit, limit);
        // x = k*ln2 + r, nearest integer k.
        let k = if x >= 0 { (x + LN2 / 2) / LN2 } else { (x - LN2 / 2) / LN2 };
        let r = x - k * LN2;
        let s = SCALE as i128;
        let mut term = s;
        let mut sum = s;
        for n in 1..=13i128 {
            term = term * r / (n * s);
            sum += term;
        }
        let sum = sum.max(0) as u128;
        if k >= 0 {
            sum << (k as u32)
        } else {
            sum >> ((-k) as u32)
        }
    }

    /// `ln(y / SCALE)` scaled by `SCALE`, for `y > 0`.
    pub fn ln_fp(y: u128) -> i128 {
        assert!(y > 0, "ln of zero");
        let s = SCALE;
        let mut m = y;
        let mut e: i128 = 0;
        while m >= 2 * s {
            m /= 2;
            e += 1;
        }
        while m < s {
            m *= 2;
            e -= 1;
        }
        // z = (m - 1) / (m + 1) in SCALE.
        let z = ((m - s) * s / (m + s)) as i128;
        let si = SCALE as i128;
        let z2 = z * z / si;
        let mut term = z;
        let mut sum = z;
        for k in 1..=11i128 {
            term = term * z2 / si;
            sum += term / (2 * k + 1);
        }
        2 * sum + e * LN2
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout))]
    pub enum MarketStatus {
        Open,
        Resolved,
    }

    #[derive(Debug, Clone, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout))]
    pub struct Market {
        pub creator: AccountId,
        pub question_hash: Hash,
        pub outcomes: u32,
        /// Liquidity parameter, in money units.
        pub b: u128,
        pub funding: u128,
        /// Native value held for this market.
        pub cash: u128,
        pub shares: Vec<u128>,
        pub resolution_block: u32,
        pub status: MarketStatus,
        pub winning_outcome: Option<u32>,
        pub trades: u32,
    }

    #[derive(Debug, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub enum Error {
        Unauthorized,
        InvalidParameters,
        MarketNotFound,
        MarketNotOpen,
        MarketNotResolved,
        InvalidOutcome,
        ZeroAmount,
        /// Liquidity can only be added before any trade.
        TradingStarted,
        /// The budget is more than `MAX_BUDGET_OVER_B` times the liquidity parameter.
        BudgetTooLarge,
        SlippageExceeded,
        InsufficientShares,
        NothingToRedeem,
        TransferFailed,
        ResolutionTooEarly,
    }

    #[ink(event)]
    pub struct LiquidityAdded {
        #[ink(topic)]
        pub market_id: u64,
        pub provider: AccountId,
        pub amount: u128,
        pub b: u128,
    }

    #[ink(event)]
    pub struct SharesBought {
        #[ink(topic)]
        pub market_id: u64,
        #[ink(topic)]
        pub buyer: AccountId,
        pub outcome: u32,
        pub cost: u128,
        pub shares: u128,
    }

    #[ink(event)]
    pub struct SharesSold {
        #[ink(topic)]
        pub market_id: u64,
        #[ink(topic)]
        pub seller: AccountId,
        pub outcome: u32,
        pub shares: u128,
        pub payout: u128,
    }

    #[ink(event)]
    pub struct LiquidityRemoved {
        #[ink(topic)]
        pub market_id: u64,
        pub provider: AccountId,
        pub amount: u128,
    }

    #[ink(event)]
    pub struct MarketResolved {
        #[ink(topic)]
        pub market_id: u64,
        pub winning_outcome: u32,
    }

    #[ink(storage)]
    pub struct LmsrPredictionMarket {
        admin: AccountId,
        next_id: u64,
        markets: Mapping<u64, Market>,
        holdings: Mapping<(u64, AccountId, u32), u128>,
        lp: Mapping<(u64, AccountId), u128>,
        lp_withdrawn: Mapping<(u64, AccountId), bool>,
    }

    /// `b = funding / ln(n)` (rounded down, so `b * ln n <= funding`).
    fn liquidity_for(funding: u128, outcomes: u32) -> u128 {
        let ln_n = ln_fp(outcomes as u128 * SCALE) as u128;
        funding.saturating_mul(SCALE) / ln_n
    }

    /// `C(q) = b * ln(sum exp(q_i/b))`, computed as `m + b*ln(sum exp((q_i - m)/b))`.
    fn cost(shares: &[u128], b: u128) -> u128 {
        let m = shares.iter().copied().max().unwrap_or(0);
        let sum: u128 = shares
            .iter()
            .map(|q| exp_fp(-(((m - q).saturating_mul(SCALE) / b) as i128)))
            .sum();
        let ln_sum = ln_fp(sum) as u128; // sum >= SCALE (the max term is exp(0))
        m + b.saturating_mul(ln_sum) / SCALE
    }

    /// Prices (probabilities) at `SCALE`, summing to about `SCALE`.
    fn prices(shares: &[u128], b: u128) -> Vec<u128> {
        let m = shares.iter().copied().max().unwrap_or(0);
        let exps: Vec<u128> = shares
            .iter()
            .map(|q| exp_fp(-(((m - q).saturating_mul(SCALE) / b) as i128)))
            .collect();
        let sum: u128 = exps.iter().sum();
        exps.iter().map(|e| e * SCALE / sum).collect()
    }

    /// Shares obtainable for `budget`, in closed form (see the module docs).
    fn shares_for_budget(shares: &[u128], b: u128, outcome: usize, budget: u128) -> u128 {
        let m = shares.iter().copied().max().unwrap_or(0);
        let a: Vec<u128> = shares
            .iter()
            .map(|q| exp_fp(-(((m - q).saturating_mul(SCALE) / b) as i128)))
            .collect();
        let total: u128 = a.iter().sum();
        let growth = exp_fp((budget.saturating_mul(SCALE) / b) as i128);
        let others: u128 = total - a[outcome];
        let new_term = total.saturating_mul(growth) / SCALE - others;
        // q_i + d = m + b * ln(new_term / SCALE)
        let target = m as i128 + (b as i128).saturating_mul(ln_fp(new_term)) / SCALE as i128;
        (target - shares[outcome] as i128).max(0) as u128
    }

    impl LmsrPredictionMarket {
        #[ink(constructor)]
        pub fn new() -> Self {
            Self {
                admin: Self::env().caller(),
                next_id: 1,
                markets: Mapping::default(),
                holdings: Mapping::default(),
                lp: Mapping::default(),
                lp_withdrawn: Mapping::default(),
            }
        }

        // ── Markets and liquidity ──────────────────────────────────────

        /// Create a market funded with the value sent. The funding fixes `b`.
        #[ink(message, payable)]
        pub fn create_market(&mut self, question_hash: Hash, outcomes: u32, resolution_block: u32) -> Result<u64, Error> {
            let funding = self.env().transferred_value();
            if funding == 0 {
                return Err(Error::ZeroAmount);
            }
            if outcomes < 2 || outcomes > MAX_OUTCOMES || resolution_block <= self.env().block_number() {
                return Err(Error::InvalidParameters);
            }
            let b = liquidity_for(funding, outcomes);
            if b == 0 {
                return Err(Error::InvalidParameters);
            }
            let creator = self.env().caller();
            let id = self.next_id;
            self.next_id += 1;
            self.markets.insert(
                id,
                &Market {
                    creator,
                    question_hash,
                    outcomes,
                    b,
                    funding,
                    cash: funding,
                    shares: ink::prelude::vec![0; outcomes as usize],
                    resolution_block,
                    status: MarketStatus::Open,
                    winning_outcome: None,
                    trades: 0,
                },
            );
            self.lp.insert((id, creator), &funding);
            self.env().emit_event(LiquidityAdded { market_id: id, provider: creator, amount: funding, b });
            Ok(id)
        }

        /// Add liquidity (the value sent) before any trade has happened.
        #[ink(message, payable)]
        pub fn add_liquidity(&mut self, market_id: u64) -> Result<u128, Error> {
            let amount = self.env().transferred_value();
            if amount == 0 {
                return Err(Error::ZeroAmount);
            }
            let mut m = self.open_market(market_id)?;
            if m.trades > 0 {
                return Err(Error::TradingStarted);
            }
            let provider = self.env().caller();
            m.funding = m.funding.saturating_add(amount);
            m.cash = m.cash.saturating_add(amount);
            m.b = liquidity_for(m.funding, m.outcomes);
            let held = self.lp.get((market_id, provider)).unwrap_or(0);
            self.lp.insert((market_id, provider), &held.saturating_add(amount));
            let b = m.b;
            self.markets.insert(market_id, &m);
            self.env().emit_event(LiquidityAdded { market_id, provider, amount, b });
            Ok(b)
        }

        /// After resolution a provider withdraws their pro-rata share of what is left
        /// once winning shares are covered. Paid once.
        #[ink(message)]
        pub fn remove_liquidity(&mut self, market_id: u64) -> Result<u128, Error> {
            let m = self.markets.get(market_id).ok_or(Error::MarketNotFound)?;
            if m.status != MarketStatus::Resolved {
                return Err(Error::MarketNotResolved);
            }
            let provider = self.env().caller();
            let contributed = self.lp.get((market_id, provider)).unwrap_or(0);
            if contributed == 0 || self.lp_withdrawn.get((market_id, provider)).unwrap_or(false) {
                return Err(Error::NothingToRedeem);
            }
            let winning = m.winning_outcome.unwrap_or(0) as usize;
            let residual = m.cash.saturating_sub(m.shares[winning]);
            let amount = residual.saturating_mul(contributed) / m.funding;
            if amount == 0 {
                return Err(Error::NothingToRedeem);
            }
            self.lp_withdrawn.insert((market_id, provider), &true);
            self.env().transfer(provider, amount).map_err(|_| Error::TransferFailed)?;
            self.env().emit_event(LiquidityRemoved { market_id, provider, amount });
            Ok(amount)
        }

        // ── Trading ────────────────────────────────────────────────────

        /// Spend the value sent on `outcome`; reverts if fewer than `min_shares` result.
        #[ink(message, payable)]
        pub fn buy_shares(&mut self, market_id: u64, outcome: u32, min_shares: u128) -> Result<u128, Error> {
            let budget = self.env().transferred_value();
            if budget == 0 {
                return Err(Error::ZeroAmount);
            }
            let mut m = self.open_market(market_id)?;
            if outcome >= m.outcomes {
                return Err(Error::InvalidOutcome);
            }
            if budget > MAX_BUDGET_OVER_B.saturating_mul(m.b) {
                return Err(Error::BudgetTooLarge);
            }
            let got = shares_for_budget(&m.shares, m.b, outcome as usize, budget);
            if got == 0 || got < min_shares {
                return Err(Error::SlippageExceeded);
            }
            m.shares[outcome as usize] += got;
            m.cash = m.cash.saturating_add(budget);
            m.trades += 1;
            self.markets.insert(market_id, &m);
            let buyer = self.env().caller();
            let held = self.holdings.get((market_id, buyer, outcome)).unwrap_or(0);
            self.holdings.insert((market_id, buyer, outcome), &(held + got));
            self.env().emit_event(SharesBought { market_id, buyer, outcome, cost: budget, shares: got });
            Ok(got)
        }

        /// Sell `shares` of `outcome` back to the market maker.
        #[ink(message)]
        pub fn sell_shares(&mut self, market_id: u64, outcome: u32, shares: u128, min_payout: u128) -> Result<u128, Error> {
            if shares == 0 {
                return Err(Error::ZeroAmount);
            }
            let mut m = self.open_market(market_id)?;
            if outcome >= m.outcomes {
                return Err(Error::InvalidOutcome);
            }
            let seller = self.env().caller();
            let held = self.holdings.get((market_id, seller, outcome)).unwrap_or(0);
            if held < shares {
                return Err(Error::InsufficientShares);
            }
            let before = cost(&m.shares, m.b);
            let mut after_shares = m.shares.clone();
            after_shares[outcome as usize] -= shares;
            let payout = before.saturating_sub(cost(&after_shares, m.b)).min(m.cash);
            if payout < min_payout {
                return Err(Error::SlippageExceeded);
            }
            m.shares = after_shares;
            m.cash -= payout;
            m.trades += 1;
            self.markets.insert(market_id, &m);
            self.holdings.insert((market_id, seller, outcome), &(held - shares));
            if payout > 0 {
                self.env().transfer(seller, payout).map_err(|_| Error::TransferFailed)?;
            }
            self.env().emit_event(SharesSold { market_id, seller, outcome, shares, payout });
            Ok(payout)
        }

        // ── Resolution ─────────────────────────────────────────────────

        /// Admin sets the winning outcome at or after the resolution block.
        #[ink(message)]
        pub fn resolve_market(&mut self, market_id: u64, winning_outcome: u32) -> Result<(), Error> {
            if self.env().caller() != self.admin {
                return Err(Error::Unauthorized);
            }
            let mut m = self.open_market(market_id)?;
            if self.env().block_number() < m.resolution_block {
                return Err(Error::ResolutionTooEarly);
            }
            if winning_outcome >= m.outcomes {
                return Err(Error::InvalidOutcome);
            }
            m.status = MarketStatus::Resolved;
            m.winning_outcome = Some(winning_outcome);
            self.markets.insert(market_id, &m);
            self.env().emit_event(MarketResolved { market_id, winning_outcome });
            Ok(())
        }

        /// Redeem winning shares at 1 unit each. Losing shares are worth nothing.
        #[ink(message)]
        pub fn redeem_winnings(&mut self, market_id: u64) -> Result<u128, Error> {
            let mut m = self.markets.get(market_id).ok_or(Error::MarketNotFound)?;
            let winning = m.winning_outcome.ok_or(Error::MarketNotResolved)?;
            let holder = self.env().caller();
            let held = self.holdings.get((market_id, holder, winning)).unwrap_or(0);
            if held == 0 {
                return Err(Error::NothingToRedeem);
            }
            self.holdings.insert((market_id, holder, winning), &0);
            let pay = held.min(m.cash);
            m.cash -= pay;
            m.shares[winning as usize] -= held.min(m.shares[winning as usize]);
            self.markets.insert(market_id, &m);
            self.env().transfer(holder, pay).map_err(|_| Error::TransferFailed)?;
            Ok(pay)
        }

        // ── Queries ────────────────────────────────────────────────────

        #[ink(message)]
        pub fn get_market(&self, market_id: u64) -> Option<Market> {
            self.markets.get(market_id)
        }

        /// Current price of each outcome at `SCALE` (so `SCALE / 2` is 50%).
        #[ink(message)]
        pub fn get_prices(&self, market_id: u64) -> Option<Vec<u128>> {
            self.markets.get(market_id).map(|m| prices(&m.shares, m.b))
        }

        /// Shares `budget` would buy right now.
        #[ink(message)]
        pub fn quote_buy(&self, market_id: u64, outcome: u32, budget: u128) -> Option<u128> {
            let m = self.markets.get(market_id)?;
            if outcome >= m.outcomes || budget == 0 || budget > MAX_BUDGET_OVER_B.saturating_mul(m.b) {
                return None;
            }
            Some(shares_for_budget(&m.shares, m.b, outcome as usize, budget))
        }

        /// What selling `shares` of `outcome` would pay right now.
        #[ink(message)]
        pub fn quote_sell(&self, market_id: u64, outcome: u32, shares: u128) -> Option<u128> {
            let m = self.markets.get(market_id)?;
            if outcome >= m.outcomes || shares > m.shares[outcome as usize] {
                return None;
            }
            let mut after = m.shares.clone();
            after[outcome as usize] -= shares;
            Some(cost(&m.shares, m.b).saturating_sub(cost(&after, m.b)))
        }

        #[ink(message)]
        pub fn get_holding(&self, market_id: u64, holder: AccountId, outcome: u32) -> u128 {
            self.holdings.get((market_id, holder, outcome)).unwrap_or(0)
        }

        fn open_market(&self, id: u64) -> Result<Market, Error> {
            let m = self.markets.get(id).ok_or(Error::MarketNotFound)?;
            if m.status != MarketStatus::Open {
                return Err(Error::MarketNotOpen);
            }
            Ok(m)
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use ink::env::test;

        type Accounts = test::DefaultAccounts<ink::env::DefaultEnvironment>;

        const FUNDING: u128 = 1_000_000;

        fn h() -> Hash {
            Hash::from([1; 32])
        }

        fn as_user(who: AccountId) {
            test::set_caller::<ink::env::DefaultEnvironment>(who);
        }

        fn close(a: u128, b: u128, tol: u128) -> bool {
            a.abs_diff(b) <= tol
        }

        // ── Fixed-point math ────────────────────────────────────────────

        #[test]
        fn exp_matches_known_values() {
            assert_eq!(exp_fp(0), SCALE);
            assert!(close(exp_fp(SCALE as i128), 2_718_281_828, 100), "{}", exp_fp(SCALE as i128));
            assert!(close(exp_fp(-(SCALE as i128)), 367_879_441, 100));
            assert!(close(exp_fp(2 * SCALE as i128), 7_389_056_099, 400));
            assert!(close(exp_fp(5 * SCALE as i128 / 10), 1_648_721_271, 100));
        }

        #[test]
        fn exp_is_monotonic_and_inverts_with_ln() {
            let mut last = 0;
            for x in (-20..=20i128).map(|i| i * SCALE as i128 / 4) {
                let v = exp_fp(x);
                assert!(v >= last, "x = {x}");
                last = v;
            }
            for x in [-3i128, -1, 0, 1, 2, 5, 10] {
                let x = x * SCALE as i128 / 3;
                let back = ln_fp(exp_fp(x).max(1));
                assert!(back.abs_diff(x) <= 5_000, "x = {x}, back = {back}");
            }
        }

        #[test]
        fn ln_matches_known_values() {
            assert_eq!(ln_fp(SCALE), 0);
            assert!(ln_fp(2 * SCALE).abs_diff(693_147_181) <= 20);
            assert!(ln_fp(4 * SCALE).abs_diff(1_386_294_361) <= 40);
            assert!(ln_fp(SCALE / 2).abs_diff(-693_147_181) <= 20);
            assert!(ln_fp(2_718_281_828).abs_diff(SCALE as i128) <= 40);
            assert!(ln_fp(10 * SCALE).abs_diff(2_302_585_093) <= 60);
        }

        // ── Cost function and prices ────────────────────────────────────

        #[test]
        fn the_initial_cost_is_b_ln_n_and_prices_are_uniform() {
            let b = liquidity_for(FUNDING, 2);
            assert!(close(b, 1_442_695, 2), "{b}");
            // C(0) = b ln 2 = funding (within rounding).
            assert!(close(cost(&[0, 0], b), FUNDING, 2));
            let p = prices(&[0, 0], b);
            assert!(close(p[0], SCALE / 2, 5) && close(p[1], SCALE / 2, 5));
            let p3 = prices(&[0, 0, 0], liquidity_for(FUNDING, 3));
            assert!(p3.iter().all(|x| close(*x, SCALE / 3, 5)), "{p3:?}");
        }

        #[test]
        fn more_outcomes_need_a_smaller_b_for_the_same_funding() {
            assert!(liquidity_for(FUNDING, 4) < liquidity_for(FUNDING, 3));
            assert!(liquidity_for(FUNDING, 3) < liquidity_for(FUNDING, 2));
        }

        #[test]
        fn cost_is_monotonic_and_prices_always_sum_to_one() {
            let b = liquidity_for(FUNDING, 2);
            let mut q = [0u128, 0u128];
            let mut last = cost(&q, b);
            for step in 0..8u128 {
                q[(step % 2) as usize] += 200_000 + step * 10_000;
                let now = cost(&q, b);
                assert!(now >= last);
                last = now;
                let p = prices(&q, b);
                assert!(close(p.iter().sum::<u128>(), SCALE, 10), "{p:?}");
            }
        }

        #[test]
        fn the_closed_form_buy_matches_the_cost_function() {
            let b = liquidity_for(FUNDING, 3);
            let q = [150_000u128, 40_000, 0];
            for budget in [1_000u128, 50_000, 400_000] {
                let d = shares_for_budget(&q, b, 1, budget);
                let mut after = q;
                after[1] += d;
                let spent = cost(&after, b) - cost(&q, b);
                assert!(close(spent, budget, 3 + b / 10_000_000), "budget {budget}: spent {spent}");
            }
        }

        // ── Market lifecycle ────────────────────────────────────────────

        fn setup() -> (LmsrPredictionMarket, Accounts, u64) {
            let a = test::default_accounts::<ink::env::DefaultEnvironment>();
            for who in [a.alice, a.bob, a.charlie, a.django] {
                test::set_account_balance::<ink::env::DefaultEnvironment>(who, 100_000_000);
            }
            as_user(a.alice);
            test::set_block_number::<ink::env::DefaultEnvironment>(10);
            let mut m = LmsrPredictionMarket::new();
            test::transfer_in::<ink::env::DefaultEnvironment>(FUNDING);
            let id = m.create_market(h(), 2, 100).unwrap();
            (m, a, id)
        }

        fn buy(m: &mut LmsrPredictionMarket, who: AccountId, id: u64, outcome: u32, budget: u128) -> u128 {
            as_user(who);
            test::transfer_in::<ink::env::DefaultEnvironment>(budget);
            m.buy_shares(id, outcome, 0).unwrap()
        }

        #[ink::test]
        fn creating_a_market_derives_b_from_the_funding() {
            let (m, a, id) = setup();
            let mk = m.get_market(id).unwrap();
            assert_eq!((mk.creator, mk.funding, mk.cash, mk.outcomes), (a.alice, FUNDING, FUNDING, 2));
            assert!(close(mk.b, 1_442_695, 2));
            assert_eq!(mk.shares, vec![0, 0]);
            let p = m.get_prices(id).unwrap();
            assert!(close(p[0], SCALE / 2, 5));
        }

        #[ink::test]
        fn market_parameters_are_validated() {
            let (mut m, _, _) = setup();
            as_user(m_caller());
            test::transfer_in::<ink::env::DefaultEnvironment>(1_000);
            assert_eq!(m.create_market(h(), 1, 100), Err(Error::InvalidParameters));
            assert_eq!(m.create_market(h(), 5, 100), Err(Error::InvalidParameters));
            assert_eq!(m.create_market(h(), 2, 10), Err(Error::InvalidParameters)); // not after now
            test::set_value_transferred::<ink::env::DefaultEnvironment>(0);
            assert_eq!(m.create_market(h(), 2, 100), Err(Error::ZeroAmount));
        }

        fn m_caller() -> AccountId {
            test::default_accounts::<ink::env::DefaultEnvironment>().alice
        }

        #[ink::test]
        fn buying_raises_that_outcomes_price_and_lowers_the_others() {
            let (mut m, a, id) = setup();
            let before = m.get_prices(id).unwrap();
            let got = buy(&mut m, a.bob, id, 0, 200_000);
            assert!(got > 200_000, "below 100% the price is under 1, so shares exceed the budget: {got}");
            let after = m.get_prices(id).unwrap();
            assert!(after[0] > before[0] && after[1] < before[1]);
            assert!(close(after.iter().sum::<u128>(), SCALE, 10));
            assert_eq!(m.get_holding(id, a.bob, 0), got);
            assert_eq!(m.get_market(id).unwrap().cash, FUNDING + 200_000);
        }

        #[ink::test]
        fn a_small_purchase_costs_about_the_spot_price() {
            let (mut m, a, id) = setup();
            // At 50% a budget of 1_000 buys just under 2_000 shares.
            let got = buy(&mut m, a.bob, id, 1, 1_000);
            assert!((1_990..=2_000).contains(&got), "{got}");
        }

        #[ink::test]
        fn selling_back_returns_about_what_was_paid() {
            let (mut m, a, id) = setup();
            let got = buy(&mut m, a.bob, id, 0, 100_000);
            as_user(a.bob);
            let before = test::get_account_balance::<ink::env::DefaultEnvironment>(a.bob).unwrap();
            let payout = m.sell_shares(id, 0, got, 0).unwrap();
            assert!(close(payout, 100_000, 5), "{payout}");
            let after = test::get_account_balance::<ink::env::DefaultEnvironment>(a.bob).unwrap();
            assert_eq!(after - before, payout);
            assert_eq!(m.get_holding(id, a.bob, 0), 0);
            let p = m.get_prices(id).unwrap();
            assert!(close(p[0], SCALE / 2, 50));
        }

        #[ink::test]
        fn a_partial_sell_pays_in_proportion_and_cannot_exceed_holdings() {
            let (mut m, a, id) = setup();
            let got = buy(&mut m, a.bob, id, 0, 100_000);
            as_user(a.bob);
            assert_eq!(m.sell_shares(id, 0, got + 1, 0), Err(Error::InsufficientShares));
            assert_eq!(m.sell_shares(id, 0, 0, 0), Err(Error::ZeroAmount));
            as_user(a.charlie);
            assert_eq!(m.sell_shares(id, 0, 1, 0), Err(Error::InsufficientShares));
            as_user(a.bob);
            let half = m.sell_shares(id, 0, got / 2, 0).unwrap();
            assert!(half > 0 && half < 100_000);
        }

        #[ink::test]
        fn slippage_bounds_and_budget_limits_apply() {
            let (mut m, a, id) = setup();
            as_user(a.bob);
            test::transfer_in::<ink::env::DefaultEnvironment>(1_000);
            assert_eq!(m.buy_shares(id, 0, u128::MAX), Err(Error::SlippageExceeded));
            let b = m.get_market(id).unwrap().b;
            test::set_value_transferred::<ink::env::DefaultEnvironment>(b * MAX_BUDGET_OVER_B + 1);
            assert_eq!(m.buy_shares(id, 0, 0), Err(Error::BudgetTooLarge));
            test::set_value_transferred::<ink::env::DefaultEnvironment>(0);
            assert_eq!(m.buy_shares(id, 0, 0), Err(Error::ZeroAmount));
            test::set_value_transferred::<ink::env::DefaultEnvironment>(10);
            assert_eq!(m.buy_shares(id, 2, 0), Err(Error::InvalidOutcome));
            assert_eq!(m.buy_shares(99, 0, 0), Err(Error::MarketNotFound));
        }

        #[ink::test]
        fn the_market_makers_loss_is_bounded_by_the_funding() {
            let (mut m, a, id) = setup();
            // Buy a lot of outcome 0 in steps; payouts if 0 wins must stay within the cash held.
            for _ in 0..6 {
                buy(&mut m, a.bob, id, 0, 1_000_000);
            }
            let mk = m.get_market(id).unwrap();
            // The worst case for the pool is that outcome 0 wins: it owes `shares[0]`.
            assert!(mk.shares[0] <= mk.cash + 10, "owes {} but holds {}", mk.shares[0], mk.cash);
        }

        // ── Liquidity ───────────────────────────────────────────────────

        #[ink::test]
        fn liquidity_can_be_added_only_before_trading_and_raises_b() {
            let (mut m, a, id) = setup();
            let b0 = m.get_market(id).unwrap().b;
            as_user(a.charlie);
            test::transfer_in::<ink::env::DefaultEnvironment>(FUNDING);
            let b1 = m.add_liquidity(id).unwrap();
            assert!(close(b1, 2 * b0, 4));
            assert_eq!(m.get_market(id).unwrap().funding, 2 * FUNDING);
            buy(&mut m, a.bob, id, 0, 1_000);
            as_user(a.charlie);
            test::transfer_in::<ink::env::DefaultEnvironment>(10);
            assert_eq!(m.add_liquidity(id), Err(Error::TradingStarted));
            test::set_value_transferred::<ink::env::DefaultEnvironment>(0);
            assert_eq!(m.add_liquidity(id), Err(Error::ZeroAmount));
        }

        // ── Resolution and payouts ──────────────────────────────────────

        #[ink::test]
        fn only_the_admin_resolves_and_not_before_the_resolution_block() {
            let (mut m, a, id) = setup();
            as_user(a.bob);
            assert_eq!(m.resolve_market(id, 0), Err(Error::Unauthorized));
            as_user(a.alice);
            assert_eq!(m.resolve_market(id, 0), Err(Error::ResolutionTooEarly));
            test::set_block_number::<ink::env::DefaultEnvironment>(100);
            assert_eq!(m.resolve_market(id, 2), Err(Error::InvalidOutcome));
            m.resolve_market(id, 0).unwrap();
            assert_eq!(m.get_market(id).unwrap().status, MarketStatus::Resolved);
            assert_eq!(m.resolve_market(id, 0), Err(Error::MarketNotOpen));
        }

        #[ink::test]
        fn winners_redeem_one_unit_per_share_and_losers_get_nothing() {
            let (mut m, a, id) = setup();
            let win = buy(&mut m, a.bob, id, 0, 100_000);
            buy(&mut m, a.charlie, id, 1, 100_000);
            // No trading or redemption once resolved / before resolution.
            as_user(a.bob);
            assert_eq!(m.redeem_winnings(id), Err(Error::MarketNotResolved));
            test::set_block_number::<ink::env::DefaultEnvironment>(100);
            as_user(a.alice);
            m.resolve_market(id, 0).unwrap();

            as_user(a.bob);
            let before = test::get_account_balance::<ink::env::DefaultEnvironment>(a.bob).unwrap();
            assert_eq!(m.redeem_winnings(id), Ok(win));
            assert_eq!(test::get_account_balance::<ink::env::DefaultEnvironment>(a.bob).unwrap() - before, win);
            assert_eq!(m.redeem_winnings(id), Err(Error::NothingToRedeem));
            as_user(a.charlie);
            assert_eq!(m.redeem_winnings(id), Err(Error::NothingToRedeem));
            as_user(a.bob);
            test::transfer_in::<ink::env::DefaultEnvironment>(10);
            assert_eq!(m.buy_shares(id, 0, 0), Err(Error::MarketNotOpen));
        }

        #[ink::test]
        fn providers_withdraw_what_is_left_after_winners_are_covered() {
            let (mut m, a, id) = setup();
            buy(&mut m, a.bob, id, 0, 100_000);
            test::set_block_number::<ink::env::DefaultEnvironment>(100);
            as_user(a.alice);
            assert_eq!(m.remove_liquidity(id), Err(Error::MarketNotResolved));
            m.resolve_market(id, 1).unwrap(); // the bought outcome loses: the pool keeps the money
            let mk = m.get_market(id).unwrap();
            let before = test::get_account_balance::<ink::env::DefaultEnvironment>(a.alice).unwrap();
            let out = m.remove_liquidity(id).unwrap();
            assert_eq!(out, mk.cash.saturating_sub(mk.shares[1]));
            assert!(out >= FUNDING, "the loser's stake accrues to the pool: {out}");
            assert_eq!(test::get_account_balance::<ink::env::DefaultEnvironment>(a.alice).unwrap() - before, out);
            assert_eq!(m.remove_liquidity(id), Err(Error::NothingToRedeem));
            as_user(a.bob);
            assert_eq!(m.remove_liquidity(id), Err(Error::NothingToRedeem));
        }

        #[ink::test]
        fn trades_and_resolution_emit_events() {
            let (mut m, a, id) = setup();
            let before = test::recorded_events().count();
            let got = buy(&mut m, a.bob, id, 0, 1_000); // SharesBought
            as_user(a.bob);
            m.sell_shares(id, 0, got, 0).unwrap(); // SharesSold
            test::set_block_number::<ink::env::DefaultEnvironment>(100);
            as_user(a.alice);
            m.resolve_market(id, 0).unwrap(); // MarketResolved
            assert_eq!(test::recorded_events().count() - before, 3);
        }
    }
}
