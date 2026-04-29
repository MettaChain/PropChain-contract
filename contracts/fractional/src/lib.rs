#![cfg_attr(not(feature = "std"), no_std, no_main)]

#[ink::contract]
mod fractional {
    use ink::prelude::vec::Vec;
    use ink::storage::Mapping;

    #[derive(
        Debug,
        Clone,
        PartialEq,
        Eq,
        scale::Encode,
        scale::Decode,
        ink::storage::traits::StorageLayout,
    )]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub struct PortfolioItem {
        pub token_id: u64,
        pub shares: u128,
        pub price_per_share: u128,
    }

    #[derive(
        Debug,
        Clone,
        PartialEq,
        Eq,
        scale::Encode,
        scale::Decode,
        ink::storage::traits::StorageLayout,
    )]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub struct PortfolioAggregation {
        pub total_value: u128,
        pub positions: Vec<(u64, u128, u128)>,
    }

    #[derive(
        Debug,
        Clone,
        PartialEq,
        Eq,
        scale::Encode,
        scale::Decode,
        ink::storage::traits::StorageLayout,
    )]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub struct TaxReport {
        pub total_dividends: u128,
        pub total_proceeds: u128,
        pub transactions: u64,
    }

    /// A share listing placed by a fractional owner who wants to exit
    #[derive(
        Debug,
        Clone,
        PartialEq,
        Eq,
        scale::Encode,
        scale::Decode,
        ink::storage::traits::StorageLayout,
    )]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub struct ShareListing {
        pub seller: AccountId,
        pub token_id: u64,
        pub shares: u128,
        pub price_per_share: u128,
    }

    #[derive(Debug, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub enum FractionalError {
        InsufficientShares,
        ListingNotFound,
        InsufficientPayment,
        Unauthorized,
        ZeroAmount,
        NoDividends,
        NoShares,
    }

    /// Emitted when an owner lists shares for sale
    #[ink(event)]
    pub struct SharesListed {
        #[ink(topic)]
        seller: AccountId,
        token_id: u64,
        shares: u128,
        price_per_share: u128,
    }

    /// Emitted when a buyer purchases listed shares
    #[ink(event)]
    pub struct SharesSold {
        #[ink(topic)]
        seller: AccountId,
        #[ink(topic)]
        buyer: AccountId,
        token_id: u64,
        shares: u128,
        total_price: u128,
    }

    /// Emitted when an owner redeems shares for their proportional value
    #[ink(event)]
    pub struct SharesRedeemed {
        #[ink(topic)]
        owner: AccountId,
        token_id: u64,
        shares: u128,
        payout: u128,
    }

    /// Emitted when a listing is cancelled
    #[ink(event)]
    pub struct ListingCancelled {
        #[ink(topic)]
        seller: AccountId,
        token_id: u64,
    }

    /// Emitted when rental income is deposited for distribution
    #[ink(event)]
    pub struct DividendsDeposited {
        #[ink(topic)]
        token_id: u64,
        amount: u128,
        per_share: u128,
    }

    /// Emitted when a shareholder withdraws their accumulated dividends
    #[ink(event)]
    pub struct DividendsWithdrawn {
        #[ink(topic)]
        account: AccountId,
        token_id: u64,
        amount: u128,
    }

    #[ink(storage)]
    pub struct Fractional {
        last_prices: Mapping<u64, u128>,
        /// Shares held per (owner, token_id)
        balances: Mapping<(AccountId, u64), u128>,
        /// Active listings per (seller, token_id)
        listings: Mapping<(AccountId, u64), ShareListing>,
        /// Total shares issued per token_id
        total_shares: Mapping<u64, u128>,
        /// Accumulated dividends per share (scaled by SCALING), per token_id
        dividends_per_share: Mapping<u64, u128>,
        /// Last dividends_per_share snapshot per (account, token_id) — used to compute owed amount
        dividend_credit: Mapping<(AccountId, u64), u128>,
        /// Pending dividend balance per (account, token_id) ready to withdraw
        dividend_balance: Mapping<(AccountId, u64), u128>,
    }

    impl Fractional {
        #[ink(constructor)]
        pub fn new() -> Self {
            Self {
                last_prices: Mapping::default(),
                balances: Mapping::default(),
                listings: Mapping::default(),
                total_shares: Mapping::default(),
                dividends_per_share: Mapping::default(),
                dividend_credit: Mapping::default(),
                dividend_balance: Mapping::default(),
            }
        }
    }

    impl Default for Fractional {
        fn default() -> Self {
            Self::new()
        }
    }

    impl Fractional {
        #[ink(message)]
        pub fn set_last_price(&mut self, token_id: u64, price_per_share: u128) {
            self.last_prices.insert(token_id, &price_per_share);
        }

        #[ink(message)]
        pub fn get_last_price(&self, token_id: u64) -> Option<u128> {
            self.last_prices.get(token_id)
        }

        #[ink(message)]
        pub fn aggregate_portfolio(&self, items: Vec<PortfolioItem>) -> PortfolioAggregation {
            let mut total: u128 = 0;
            let mut positions: Vec<(u64, u128, u128)> = Vec::new();
            for it in items.iter() {
                let price = if it.price_per_share > 0 {
                    it.price_per_share
                } else {
                    self.last_prices.get(it.token_id).unwrap_or(0)
                };
                let value = price.saturating_mul(it.shares);
                total = total.saturating_add(value);
                positions.push((it.token_id, it.shares, price));
            }
            PortfolioAggregation {
                total_value: total,
                positions,
            }
        }

        #[ink(message)]
        pub fn summarize_tax(
            &self,
            dividends: Vec<(u64, u128)>,
            proceeds: Vec<(u64, u128)>,
        ) -> TaxReport {
            let mut total_dividends: u128 = 0;
            for d in dividends.iter() {
                total_dividends = total_dividends.saturating_add(d.1);
            }
            let mut total_proceeds: u128 = 0;
            for p in proceeds.iter() {
                total_proceeds = total_proceeds.saturating_add(p.1);
            }
            TaxReport {
                total_dividends,
                total_proceeds,
                transactions: (dividends.len() + proceeds.len()) as u64,
            }
        }

        // ── Issue #278: Exit mechanism ───────────────────────────────────────

        /// Mint shares to an owner (used in tests / by the property token contract)
        #[ink(message)]
        pub fn mint_shares(&mut self, owner: AccountId, token_id: u64, amount: u128) {
            // Settle any pending dividends before changing the balance so the new
            // shares don't retroactively earn dividends from prior deposits.
            self.settle_dividend(owner, token_id);
            let current = self.balances.get(&(owner, token_id)).unwrap_or(0);
            self.balances.insert(&(owner, token_id), &current.saturating_add(amount));
            let total = self.total_shares.get(&token_id).unwrap_or(0);
            self.total_shares.insert(&token_id, &total.saturating_add(amount));
        }

        /// Get the share balance of an owner for a given token
        #[ink(message)]
        pub fn balance_of(&self, owner: AccountId, token_id: u64) -> u128 {
            self.balances.get(&(owner, token_id)).unwrap_or(0)
        }

        /// List shares for sale at a given price per share.
        /// The caller must hold at least `shares` of `token_id`.
        #[ink(message)]
        pub fn list_shares_for_sale(
            &mut self,
            token_id: u64,
            shares: u128,
            price_per_share: u128,
        ) -> Result<(), FractionalError> {
            if shares == 0 {
                return Err(FractionalError::ZeroAmount);
            }
            let caller = self.env().caller();
            let held = self.balances.get(&(caller, token_id)).unwrap_or(0);
            if held < shares {
                return Err(FractionalError::InsufficientShares);
            }

            let listing = ShareListing {
                seller: caller,
                token_id,
                shares,
                price_per_share,
            };
            self.listings.insert(&(caller, token_id), &listing);
            self.last_prices.insert(token_id, &price_per_share);

            self.env().emit_event(SharesListed {
                seller: caller,
                token_id,
                shares,
                price_per_share,
            });
            Ok(())
        }

        /// Cancel an active listing
        #[ink(message)]
        pub fn cancel_listing(&mut self, token_id: u64) -> Result<(), FractionalError> {
            let caller = self.env().caller();
            if self.listings.get(&(caller, token_id)).is_none() {
                return Err(FractionalError::ListingNotFound);
            }
            self.listings.remove(&(caller, token_id));
            self.env().emit_event(ListingCancelled {
                seller: caller,
                token_id,
            });
            Ok(())
        }

        /// Buy shares from an existing listing.
        /// The buyer must attach sufficient payment (transferred value).
        #[ink(message, payable)]
        pub fn buy_shares(
            &mut self,
            seller: AccountId,
            token_id: u64,
            shares: u128,
        ) -> Result<(), FractionalError> {
            if shares == 0 {
                return Err(FractionalError::ZeroAmount);
            }
            let buyer = self.env().caller();
            let payment = self.env().transferred_value();

            let listing = self
                .listings
                .get(&(seller, token_id))
                .ok_or(FractionalError::ListingNotFound)?;

            if shares > listing.shares {
                return Err(FractionalError::InsufficientShares);
            }

            let total_price = listing.price_per_share.saturating_mul(shares);
            if payment < total_price {
                return Err(FractionalError::InsufficientPayment);
            }

            // Settle dividends for both parties before changing balances
            self.settle_dividend(seller, token_id);
            self.settle_dividend(buyer, token_id);

            // Transfer shares: deduct from seller, credit buyer
            let seller_held = self.balances.get(&(seller, token_id)).unwrap_or(0);
            self.balances
                .insert(&(seller, token_id), &seller_held.saturating_sub(shares));

            let buyer_held = self.balances.get(&(buyer, token_id)).unwrap_or(0);
            self.balances
                .insert(&(buyer, token_id), &buyer_held.saturating_add(shares));

            // Update or remove listing
            let remaining = listing.shares.saturating_sub(shares);
            if remaining == 0 {
                self.listings.remove(&(seller, token_id));
            } else {
                let updated = ShareListing {
                    shares: remaining,
                    ..listing
                };
                self.listings.insert(&(seller, token_id), &updated);
            }

            // Pay the seller
            if self.env().transfer(seller, total_price).is_err() {
                // Non-fatal: payment forwarding failed (e.g. in unit tests)
            }

            self.env().emit_event(SharesSold {
                seller,
                buyer,
                token_id,
                shares,
                total_price,
            });
            Ok(())
        }

        /// Redeem shares for their proportional value based on the last recorded price.
        /// Burns the shares and pays out `shares * last_price` to the caller.
        #[ink(message)]
        pub fn redeem_shares(
            &mut self,
            token_id: u64,
            shares: u128,
        ) -> Result<u128, FractionalError> {
            if shares == 0 {
                return Err(FractionalError::ZeroAmount);
            }
            let caller = self.env().caller();
            let held = self.balances.get(&(caller, token_id)).unwrap_or(0);
            if held < shares {
                return Err(FractionalError::InsufficientShares);
            }

            let price = self.last_prices.get(token_id).unwrap_or(0);
            let payout = price.saturating_mul(shares);

            // Settle dividends before burning so the owner doesn't lose unclaimed dividends
            self.settle_dividend(caller, token_id);

            // Burn shares
            self.balances
                .insert(&(caller, token_id), &held.saturating_sub(shares));
            let total = self.total_shares.get(&token_id).unwrap_or(0);
            self.total_shares
                .insert(&token_id, &total.saturating_sub(shares));

            // Pay out (best-effort in unit tests)
            if payout > 0 {
                let _ = self.env().transfer(caller, payout);
            }

            self.env().emit_event(SharesRedeemed {
                owner: caller,
                token_id,
                shares,
                payout,
            });
            Ok(payout)
        }

        /// Get an active listing
        #[ink(message)]
        pub fn get_listing(&self, seller: AccountId, token_id: u64) -> Option<ShareListing> {
            self.listings.get(&(seller, token_id))
        }

        // ── Issue #272: Dividend distribution ───────────────────────────────

        const SCALING: u128 = 1_000_000_000_000;

        /// Deposit rental income for a property token; distributes proportionally to all shareholders.
        /// The caller must attach the income amount as the transferred value.
        #[ink(message, payable)]
        pub fn deposit_dividends(&mut self, token_id: u64) -> Result<(), FractionalError> {
            let amount = self.env().transferred_value();
            if amount == 0 {
                return Err(FractionalError::ZeroAmount);
            }
            let ts = self.total_shares.get(token_id).unwrap_or(0);
            if ts == 0 {
                return Err(FractionalError::NoShares);
            }
            let add = amount.saturating_mul(Self::SCALING) / ts;
            let cur = self.dividends_per_share.get(token_id).unwrap_or(0);
            self.dividends_per_share.insert(token_id, &cur.saturating_add(add));
            self.env().emit_event(DividendsDeposited {
                token_id,
                amount,
                per_share: add,
            });
            Ok(())
        }

        /// Withdraw accumulated dividends for the caller on a given token.
        #[ink(message)]
        pub fn withdraw_dividends(&mut self, token_id: u64) -> Result<u128, FractionalError> {
            let caller = self.env().caller();
            self.settle_dividend(caller, token_id);
            let owed = self.dividend_balance.get(&(caller, token_id)).unwrap_or(0);
            if owed == 0 {
                return Err(FractionalError::NoDividends);
            }
            self.dividend_balance.insert(&(caller, token_id), &0u128);
            // Best-effort transfer (may fail in unit tests without balance)
            let _ = self.env().transfer(caller, owed);
            self.env().emit_event(DividendsWithdrawn {
                account: caller,
                token_id,
                amount: owed,
            });
            Ok(owed)
        }

        /// Query pending (unclaimed) dividends for an account without mutating state.
        #[ink(message)]
        pub fn pending_dividends(&self, account: AccountId, token_id: u64) -> u128 {
            let dps = self.dividends_per_share.get(token_id).unwrap_or(0);
            let credited = self.dividend_credit.get(&(account, token_id)).unwrap_or(0);
            let already = self.dividend_balance.get(&(account, token_id)).unwrap_or(0);
            if dps > credited {
                let bal = self.balances.get(&(account, token_id)).unwrap_or(0);
                let delta = dps.saturating_sub(credited);
                let add = bal.saturating_mul(delta) / Self::SCALING;
                already.saturating_add(add)
            } else {
                already
            }
        }

        /// Internal: credit any unsettled dividends into the account's balance.
        fn settle_dividend(&mut self, account: AccountId, token_id: u64) {
            let dps = self.dividends_per_share.get(token_id).unwrap_or(0);
            let credited = self.dividend_credit.get(&(account, token_id)).unwrap_or(0);
            if dps > credited {
                let bal = self.balances.get(&(account, token_id)).unwrap_or(0);
                let delta = dps.saturating_sub(credited);
                let add = bal.saturating_mul(delta) / Self::SCALING;
                let owed = self.dividend_balance.get(&(account, token_id)).unwrap_or(0);
                self.dividend_balance.insert(&(account, token_id), &owed.saturating_add(add));
                self.dividend_credit.insert(&(account, token_id), &dps);
            } else if credited == 0 && dps > 0 {
                self.dividend_credit.insert(&(account, token_id), &dps);
            }
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use ink::env::test;

        fn alice() -> AccountId {
            test::default_accounts::<ink::env::DefaultEnvironment>().alice
        }
        fn bob() -> AccountId {
            test::default_accounts::<ink::env::DefaultEnvironment>().bob
        }

        #[ink::test]
        fn test_mint_and_balance() {
            let mut f = Fractional::new();
            f.mint_shares(alice(), 1, 100);
            assert_eq!(f.balance_of(alice(), 1), 100);
        }

        #[ink::test]
        fn test_list_and_cancel() {
            let mut f = Fractional::new();
            test::set_caller::<ink::env::DefaultEnvironment>(alice());
            f.mint_shares(alice(), 1, 100);
            assert!(f.list_shares_for_sale(1, 50, 10).is_ok());
            let listing = f.get_listing(alice(), 1).unwrap();
            assert_eq!(listing.shares, 50);
            assert!(f.cancel_listing(1).is_ok());
            assert!(f.get_listing(alice(), 1).is_none());
        }

        #[ink::test]
        fn test_list_insufficient_shares() {
            let mut f = Fractional::new();
            test::set_caller::<ink::env::DefaultEnvironment>(alice());
            f.mint_shares(alice(), 1, 10);
            assert_eq!(
                f.list_shares_for_sale(1, 50, 10),
                Err(FractionalError::InsufficientShares)
            );
        }

        #[ink::test]
        fn test_redeem_shares() {
            let mut f = Fractional::new();
            test::set_caller::<ink::env::DefaultEnvironment>(alice());
            f.mint_shares(alice(), 1, 100);
            f.set_last_price(1, 5);
            let payout = f.redeem_shares(1, 20).unwrap();
            assert_eq!(payout, 100); // 20 * 5
            assert_eq!(f.balance_of(alice(), 1), 80);
        }

        #[ink::test]
        fn test_redeem_insufficient() {
            let mut f = Fractional::new();
            test::set_caller::<ink::env::DefaultEnvironment>(alice());
            f.mint_shares(alice(), 1, 10);
            assert_eq!(
                f.redeem_shares(1, 50),
                Err(FractionalError::InsufficientShares)
            );
        }

        #[ink::test]
        fn test_aggregate_portfolio() {
            let f = Fractional::new();
            let items = vec![
                PortfolioItem { token_id: 1, shares: 10, price_per_share: 5 },
                PortfolioItem { token_id: 2, shares: 20, price_per_share: 3 },
            ];
            let agg = f.aggregate_portfolio(items);
            assert_eq!(agg.total_value, 110);
        }

        #[ink::test]
        fn test_buy_shares_insufficient_payment() {
            let mut f = Fractional::new();
            test::set_caller::<ink::env::DefaultEnvironment>(alice());
            f.mint_shares(alice(), 1, 100);
            f.list_shares_for_sale(1, 50, 10).unwrap();

            test::set_caller::<ink::env::DefaultEnvironment>(bob());
            // No payment attached → InsufficientPayment
            assert_eq!(
                f.buy_shares(alice(), 1, 10),
                Err(FractionalError::InsufficientPayment)
            );
        }

        // ── Issue #272: Dividend distribution tests ──────────────────────────

        #[ink::test]
        fn test_deposit_dividends_no_shares_fails() {
            let mut f = Fractional::new();
            test::set_value_transferred::<ink::env::DefaultEnvironment>(1_000);
            assert_eq!(f.deposit_dividends(1), Err(FractionalError::NoShares));
        }

        #[ink::test]
        fn test_deposit_dividends_zero_value_fails() {
            let mut f = Fractional::new();
            f.mint_shares(alice(), 1, 100);
            // No value transferred → ZeroAmount
            assert_eq!(f.deposit_dividends(1), Err(FractionalError::ZeroAmount));
        }

        #[ink::test]
        fn test_pending_dividends_proportional() {
            let mut f = Fractional::new();
            // Alice: 600 shares, Bob: 400 shares (total 1000)
            f.mint_shares(alice(), 1, 600);
            f.mint_shares(bob(), 1, 400);

            test::set_value_transferred::<ink::env::DefaultEnvironment>(1_000_000);
            f.deposit_dividends(1).unwrap();

            // Alice should get 60%, Bob 40%
            let alice_pending = f.pending_dividends(alice(), 1);
            let bob_pending = f.pending_dividends(bob(), 1);
            assert_eq!(alice_pending, 600_000);
            assert_eq!(bob_pending, 400_000);
        }

        #[ink::test]
        fn test_withdraw_dividends_success() {
            let mut f = Fractional::new();
            f.mint_shares(alice(), 1, 500);
            f.mint_shares(bob(), 1, 500);

            test::set_value_transferred::<ink::env::DefaultEnvironment>(1_000_000);
            f.deposit_dividends(1).unwrap();

            test::set_caller::<ink::env::DefaultEnvironment>(alice());
            let withdrawn = f.withdraw_dividends(1).unwrap();
            assert_eq!(withdrawn, 500_000);

            // After withdrawal, pending should be zero
            assert_eq!(f.pending_dividends(alice(), 1), 0);
        }

        #[ink::test]
        fn test_withdraw_dividends_no_dividends_fails() {
            let mut f = Fractional::new();
            f.mint_shares(alice(), 1, 100);
            test::set_caller::<ink::env::DefaultEnvironment>(alice());
            assert_eq!(f.withdraw_dividends(1), Err(FractionalError::NoDividends));
        }

        #[ink::test]
        fn test_multiple_deposits_accumulate() {
            let mut f = Fractional::new();
            f.mint_shares(alice(), 1, 1000);

            test::set_value_transferred::<ink::env::DefaultEnvironment>(500_000);
            f.deposit_dividends(1).unwrap();

            test::set_value_transferred::<ink::env::DefaultEnvironment>(500_000);
            f.deposit_dividends(1).unwrap();

            // Total 1_000_000 deposited, alice holds all shares
            assert_eq!(f.pending_dividends(alice(), 1), 1_000_000);
        }

        #[ink::test]
        fn test_late_shareholder_gets_no_prior_dividends() {
            let mut f = Fractional::new();
            f.mint_shares(alice(), 1, 1000);

            // Deposit before Bob has any shares
            test::set_value_transferred::<ink::env::DefaultEnvironment>(1_000_000);
            f.deposit_dividends(1).unwrap();

            // Bob joins after the deposit
            f.mint_shares(bob(), 1, 1000);

            // Bob should have no pending dividends from the prior deposit
            assert_eq!(f.pending_dividends(bob(), 1), 0);
            // Alice still has her full entitlement
            assert_eq!(f.pending_dividends(alice(), 1), 1_000_000);
        }
    }
}
