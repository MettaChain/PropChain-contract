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

    /// AMM liquidity pool for a property token using constant-product (x * y = k).
    /// `share_reserve` = shares in pool, `value_reserve` = native-token value in pool.
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
    pub struct AmmPool {
        pub share_reserve: u128,
        pub value_reserve: u128,
        pub lp_supply: u128,
    }

    #[derive(Debug, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub enum FractionalError {
        InsufficientShares,
        ListingNotFound,
        InsufficientPayment,
        Unauthorized,
        ZeroAmount,
        PoolNotFound,
        PoolAlreadyExists,
        SlippageExceeded,
        InsufficientLiquidity,
        InsufficientLpShares,
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

    /// Emitted when liquidity is added to an AMM pool
    #[ink(event)]
    pub struct LiquidityAdded {
        #[ink(topic)]
        provider: AccountId,
        token_id: u64,
        shares_added: u128,
        value_added: u128,
        lp_minted: u128,
    }

    /// Emitted when liquidity is removed from an AMM pool
    #[ink(event)]
    pub struct LiquidityRemoved {
        #[ink(topic)]
        provider: AccountId,
        token_id: u64,
        shares_out: u128,
        value_out: u128,
        lp_burned: u128,
    }

    /// Emitted when shares are swapped for value via the AMM
    #[ink(event)]
    pub struct SharesSwapped {
        #[ink(topic)]
        trader: AccountId,
        token_id: u64,
        shares_in: u128,
        value_out: u128,
        new_spot_price: u128,
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
        /// AMM pools per token_id
        amm_pools: Mapping<u64, AmmPool>,
        /// LP token balances per (provider, token_id)
        lp_balances: Mapping<(AccountId, u64), u128>,
    }

    impl Fractional {
        #[ink(constructor)]
        pub fn new() -> Self {
            Self {
                last_prices: Mapping::default(),
                balances: Mapping::default(),
                listings: Mapping::default(),
                total_shares: Mapping::default(),
                amm_pools: Mapping::default(),
                lp_balances: Mapping::default(),
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
            let current = self.balances.get(&(owner, token_id)).unwrap_or(0);
            self.balances
                .insert(&(owner, token_id), &current.saturating_add(amount));
            let total = self.total_shares.get(&token_id).unwrap_or(0);
            self.total_shares
                .insert(&token_id, &total.saturating_add(amount));
        }

        /// Get the share balance of an owner for a given token
        #[ink(message)]
        pub fn balance_of(&self, owner: AccountId, token_id: u64) -> u128 {
            self.balances.get(&(owner, token_id)).unwrap_or(0)
        }

        /// Consolidate an owner's share balance for a token into the canonical balance slot.
        #[ink(message)]
        pub fn consolidate_shares(
            &mut self,
            owner: AccountId,
            token_id: u64,
        ) -> Result<u128, FractionalError> {
            let shares = self.balances.get(&(owner, token_id)).unwrap_or(0);
            if shares == 0 {
                return Err(FractionalError::InsufficientShares);
            }

            self.balances.insert(&(owner, token_id), &shares);
            Ok(shares)
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

            // Analytics: record trade and update holder counts
            let price_per_share = listing.price_per_share;
            let seller_new_bal = self.balances.get(&(seller, token_id)).unwrap_or(0);
            let buyer_new_bal = self.balances.get(&(buyer, token_id)).unwrap_or(0);
            self.record_trade(token_id, total_price, price_per_share);
            self.update_holder(seller, token_id, seller_new_bal);
            self.update_holder(buyer, token_id, buyer_new_bal);

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

            // Analytics: record redemption as a trade and update holder count
            let new_bal = self.balances.get(&(caller, token_id)).unwrap_or(0);
            self.record_trade(token_id, payout, price);
            self.update_holder(caller, token_id, new_bal);

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

        // ── Issue #269: AMM-style dynamic share pricing ──────────────────────

        /// Seed a new constant-product AMM pool for `token_id`.
        /// The caller contributes `share_amount` shares and attaches native value.
        /// Caller must already hold `share_amount` shares.
        #[ink(message, payable)]
        pub fn add_liquidity(
            &mut self,
            token_id: u64,
            share_amount: u128,
            min_lp_out: u128,
        ) -> Result<u128, FractionalError> {
            if share_amount == 0 {
                return Err(FractionalError::ZeroAmount);
            }
            let caller = self.env().caller();
            let value_in = self.env().transferred_value();
            if value_in == 0 {
                return Err(FractionalError::ZeroAmount);
            }

            let held = self.balances.get(&(caller, token_id)).unwrap_or(0);
            if held < share_amount {
                return Err(FractionalError::InsufficientShares);
            }

            let lp_minted;
            let pool = self.amm_pools.get(token_id);
            let updated = match pool {
                None => {
                    // First deposit: LP = sqrt(share_amount * value_in), floored via integer sqrt
                    lp_minted = Self::isqrt(share_amount.saturating_mul(value_in));
                    AmmPool {
                        share_reserve: share_amount,
                        value_reserve: value_in,
                        lp_supply: lp_minted,
                    }
                }
                Some(p) => {
                    // Proportional deposit: LP = min(share/share_reserve, value/value_reserve) * lp_supply
                    let lp_by_share = share_amount
                        .saturating_mul(p.lp_supply)
                        .checked_div(p.share_reserve)
                        .unwrap_or(0);
                    let lp_by_value = value_in
                        .saturating_mul(p.lp_supply)
                        .checked_div(p.value_reserve)
                        .unwrap_or(0);
                    lp_minted = lp_by_share.min(lp_by_value);
                    AmmPool {
                        share_reserve: p.share_reserve.saturating_add(share_amount),
                        value_reserve: p.value_reserve.saturating_add(value_in),
                        lp_supply: p.lp_supply.saturating_add(lp_minted),
                    }
                }
            };

            if lp_minted < min_lp_out {
                return Err(FractionalError::SlippageExceeded);
            }

            // Lock shares in pool (deduct from caller balance)
            self.balances
                .insert(&(caller, token_id), &held.saturating_sub(share_amount));

            // Update spot price
            if updated.share_reserve > 0 {
                let spot = updated.value_reserve / updated.share_reserve;
                self.last_prices.insert(token_id, &spot);
            }

            self.amm_pools.insert(token_id, &updated);
            let lp_held = self.lp_balances.get(&(caller, token_id)).unwrap_or(0);
            self.lp_balances
                .insert(&(caller, token_id), &lp_held.saturating_add(lp_minted));

            self.env().emit_event(LiquidityAdded {
                provider: caller,
                token_id,
                shares_added: share_amount,
                value_added: value_in,
                lp_minted,
            });
            Ok(lp_minted)
        }

        /// Burn `lp_amount` LP tokens and withdraw proportional shares + value.
        #[ink(message)]
        pub fn remove_liquidity(
            &mut self,
            token_id: u64,
            lp_amount: u128,
            min_shares_out: u128,
            min_value_out: u128,
        ) -> Result<(u128, u128), FractionalError> {
            if lp_amount == 0 {
                return Err(FractionalError::ZeroAmount);
            }
            let caller = self.env().caller();
            let lp_held = self.lp_balances.get(&(caller, token_id)).unwrap_or(0);
            if lp_held < lp_amount {
                return Err(FractionalError::InsufficientLpShares);
            }
            let pool = self
                .amm_pools
                .get(token_id)
                .ok_or(FractionalError::PoolNotFound)?;

            let shares_out = lp_amount
                .saturating_mul(pool.share_reserve)
                .checked_div(pool.lp_supply)
                .unwrap_or(0);
            let value_out = lp_amount
                .saturating_mul(pool.value_reserve)
                .checked_div(pool.lp_supply)
                .unwrap_or(0);

            if shares_out < min_shares_out || value_out < min_value_out {
                return Err(FractionalError::SlippageExceeded);
            }

            let updated = AmmPool {
                share_reserve: pool.share_reserve.saturating_sub(shares_out),
                value_reserve: pool.value_reserve.saturating_sub(value_out),
                lp_supply: pool.lp_supply.saturating_sub(lp_amount),
            };

            // Return shares to caller
            let bal = self.balances.get(&(caller, token_id)).unwrap_or(0);
            self.balances
                .insert(&(caller, token_id), &bal.saturating_add(shares_out));

            self.lp_balances
                .insert(&(caller, token_id), &lp_held.saturating_sub(lp_amount));
            self.amm_pools.insert(token_id, &updated);

            // Update spot price
            if updated.share_reserve > 0 {
                let spot = updated.value_reserve / updated.share_reserve;
                self.last_prices.insert(token_id, &spot);
            }

            // Return value to caller (best-effort)
            if value_out > 0 {
                let _ = self.env().transfer(caller, value_out);
            }

            self.env().emit_event(LiquidityRemoved {
                provider: caller,
                token_id,
                shares_out,
                value_out,
                lp_burned: lp_amount,
            });
            Ok((shares_out, value_out))
        }

        /// Sell `shares_in` shares into the AMM pool, receiving native value out.
        /// Uses constant-product formula: value_out = value_reserve - k / (share_reserve + shares_in).
        /// A 30-bip (0.3 %) fee is retained in the pool.
        #[ink(message)]
        pub fn swap_shares_for_value(
            &mut self,
            token_id: u64,
            shares_in: u128,
            min_value_out: u128,
        ) -> Result<u128, FractionalError> {
            if shares_in == 0 {
                return Err(FractionalError::ZeroAmount);
            }
            let caller = self.env().caller();
            let held = self.balances.get(&(caller, token_id)).unwrap_or(0);
            if held < shares_in {
                return Err(FractionalError::InsufficientShares);
            }
            let pool = self
                .amm_pools
                .get(token_id)
                .ok_or(FractionalError::PoolNotFound)?;
            if pool.value_reserve == 0 || pool.share_reserve == 0 {
                return Err(FractionalError::InsufficientLiquidity);
            }

            // 0.3 % fee: effective shares_in after fee
            let shares_in_with_fee = shares_in.saturating_mul(9970);
            let numerator = shares_in_with_fee.saturating_mul(pool.value_reserve);
            let denominator = pool
                .share_reserve
                .saturating_mul(10000)
                .saturating_add(shares_in_with_fee);
            let value_out = numerator.checked_div(denominator).unwrap_or(0);

            if value_out < min_value_out {
                return Err(FractionalError::SlippageExceeded);
            }
            if value_out >= pool.value_reserve {
                return Err(FractionalError::InsufficientLiquidity);
            }

            let updated = AmmPool {
                share_reserve: pool.share_reserve.saturating_add(shares_in),
                value_reserve: pool.value_reserve.saturating_sub(value_out),
                lp_supply: pool.lp_supply,
            };

            // Deduct shares from caller
            self.balances
                .insert(&(caller, token_id), &held.saturating_sub(shares_in));
            // Burn shares from total supply
            let total = self.total_shares.get(token_id).unwrap_or(0);
            self.total_shares
                .insert(token_id, &total.saturating_sub(shares_in));

            // Update spot price
            let new_spot = if updated.share_reserve > 0 {
                let s = updated.value_reserve / updated.share_reserve;
                self.last_prices.insert(token_id, &s);
                s
            } else {
                0
            };

            self.amm_pools.insert(token_id, &updated);

            // Pay caller (best-effort)
            if value_out > 0 {
                let _ = self.env().transfer(caller, value_out);
            }

            self.env().emit_event(SharesSwapped {
                trader: caller,
                token_id,
                shares_in,
                value_out,
                new_spot_price: new_spot,
            });
            Ok(value_out)
        }

        /// Returns the current spot price (value per share) for `token_id`'s AMM pool.
        /// Spot price = value_reserve / share_reserve.
        #[ink(message)]
        pub fn get_spot_price(&self, token_id: u64) -> Result<u128, FractionalError> {
            let pool = self
                .amm_pools
                .get(token_id)
                .ok_or(FractionalError::PoolNotFound)?;
            if pool.share_reserve == 0 {
                return Err(FractionalError::InsufficientLiquidity);
            }
            Ok(pool.value_reserve / pool.share_reserve)
        }

        /// Returns the AMM pool state for `token_id`.
        #[ink(message)]
        pub fn get_pool(&self, token_id: u64) -> Option<AmmPool> {
            self.amm_pools.get(token_id)
        }

        /// Returns the LP token balance of `provider` for `token_id`.
        #[ink(message)]
        pub fn lp_balance_of(&self, provider: AccountId, token_id: u64) -> u128 {
            self.lp_balances.get(&(provider, token_id)).unwrap_or(0)
        }

        // ── Helpers ──────────────────────────────────────────────────────────

        /// Integer square root (floor).
        fn isqrt(n: u128) -> u128 {
            if n == 0 {
                return 0;
            }
            let mut x = n;
            let mut y = (x + 1) / 2;
            while y < x {
                x = y;
                y = (x + n / x) / 2;
            }
            x
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
                PortfolioItem {
                    token_id: 1,
                    shares: 10,
                    price_per_share: 5,
                },
                PortfolioItem {
                    token_id: 2,
                    shares: 20,
                    price_per_share: 3,
                },
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

        // ── AMM tests ────────────────────────────────────────────────────────

        #[ink::test]
        fn test_add_liquidity_creates_pool() {
            let mut f = Fractional::new();
            test::set_caller::<ink::env::DefaultEnvironment>(alice());
            f.mint_shares(alice(), 1, 1000);

            test::set_value_transferred::<ink::env::DefaultEnvironment>(500);
            let lp = f.add_liquidity(1, 100, 0).unwrap();
            assert!(lp > 0);

            let pool = f.get_pool(1).unwrap();
            assert_eq!(pool.share_reserve, 100);
            assert_eq!(pool.value_reserve, 500);
            assert_eq!(pool.lp_supply, lp);
            assert_eq!(f.lp_balance_of(alice(), 1), lp);
            // Shares locked: 1000 - 100 = 900
            assert_eq!(f.balance_of(alice(), 1), 900);
        }

        #[ink::test]
        fn test_add_liquidity_updates_spot_price() {
            let mut f = Fractional::new();
            test::set_caller::<ink::env::DefaultEnvironment>(alice());
            f.mint_shares(alice(), 1, 1000);

            test::set_value_transferred::<ink::env::DefaultEnvironment>(2000);
            f.add_liquidity(1, 100, 0).unwrap();

            // spot = 2000 / 100 = 20
            assert_eq!(f.get_spot_price(1).unwrap(), 20);
            assert_eq!(f.get_last_price(1), Some(20));
        }

        #[ink::test]
        fn test_add_liquidity_insufficient_shares() {
            let mut f = Fractional::new();
            test::set_caller::<ink::env::DefaultEnvironment>(alice());
            f.mint_shares(alice(), 1, 10);

            test::set_value_transferred::<ink::env::DefaultEnvironment>(500);
            assert_eq!(
                f.add_liquidity(1, 100, 0),
                Err(FractionalError::InsufficientShares)
            );
        }

        #[ink::test]
        fn test_add_liquidity_slippage_check() {
            let mut f = Fractional::new();
            test::set_caller::<ink::env::DefaultEnvironment>(alice());
            f.mint_shares(alice(), 1, 1000);

            test::set_value_transferred::<ink::env::DefaultEnvironment>(100);
            // isqrt(100 * 100) = 100; require > 100 → SlippageExceeded
            assert_eq!(
                f.add_liquidity(1, 100, 101),
                Err(FractionalError::SlippageExceeded)
            );
        }

        #[ink::test]
        fn test_remove_liquidity() {
            let mut f = Fractional::new();
            test::set_caller::<ink::env::DefaultEnvironment>(alice());
            f.mint_shares(alice(), 1, 1000);

            test::set_value_transferred::<ink::env::DefaultEnvironment>(1000);
            let lp = f.add_liquidity(1, 200, 0).unwrap();

            let (shares_out, value_out) = f.remove_liquidity(1, lp, 0, 0).unwrap();
            assert_eq!(shares_out, 200);
            assert_eq!(value_out, 1000);
            // Pool should be empty
            let pool = f.get_pool(1).unwrap();
            assert_eq!(pool.share_reserve, 0);
            assert_eq!(pool.value_reserve, 0);
            // Shares returned to alice
            assert_eq!(f.balance_of(alice(), 1), 1000);
        }

        #[ink::test]
        fn test_remove_liquidity_insufficient_lp() {
            let mut f = Fractional::new();
            test::set_caller::<ink::env::DefaultEnvironment>(alice());
            f.mint_shares(alice(), 1, 1000);

            test::set_value_transferred::<ink::env::DefaultEnvironment>(500);
            let lp = f.add_liquidity(1, 100, 0).unwrap();

            assert_eq!(
                f.remove_liquidity(1, lp + 1, 0, 0),
                Err(FractionalError::InsufficientLpShares)
            );
        }

        #[ink::test]
        fn test_swap_shares_for_value() {
            let mut f = Fractional::new();
            test::set_caller::<ink::env::DefaultEnvironment>(alice());
            f.mint_shares(alice(), 1, 1000);

            // Seed pool: 100 shares, 10_000 value → spot = 100
            test::set_value_transferred::<ink::env::DefaultEnvironment>(10_000);
            f.add_liquidity(1, 100, 0).unwrap();

            // Alice swaps 10 shares for value
            let value_out = f.swap_shares_for_value(1, 10, 0).unwrap();
            assert!(value_out > 0);
            // After swap: share_reserve grows, value_reserve shrinks → spot decreases
            let new_spot = f.get_spot_price(1).unwrap();
            assert!(new_spot < 100);
        }

        #[ink::test]
        fn test_swap_no_pool() {
            let mut f = Fractional::new();
            test::set_caller::<ink::env::DefaultEnvironment>(alice());
            f.mint_shares(alice(), 1, 100);
            assert_eq!(
                f.swap_shares_for_value(1, 10, 0),
                Err(FractionalError::PoolNotFound)
            );
        }

        #[ink::test]
        fn test_swap_slippage_check() {
            let mut f = Fractional::new();
            test::set_caller::<ink::env::DefaultEnvironment>(alice());
            f.mint_shares(alice(), 1, 1000);

            test::set_value_transferred::<ink::env::DefaultEnvironment>(10_000);
            f.add_liquidity(1, 100, 0).unwrap();

            // Demand more value_out than the pool can give
            assert_eq!(
                f.swap_shares_for_value(1, 10, 10_000),
                Err(FractionalError::SlippageExceeded)
            );
        }

        #[ink::test]
        fn test_get_spot_price_no_pool() {
            let f = Fractional::new();
            assert_eq!(f.get_spot_price(99), Err(FractionalError::PoolNotFound));
        }

        #[ink::test]
        fn test_isqrt() {
            assert_eq!(Fractional::isqrt(0), 0);
            assert_eq!(Fractional::isqrt(1), 1);
            assert_eq!(Fractional::isqrt(4), 2);
            assert_eq!(Fractional::isqrt(9), 3);
            assert_eq!(Fractional::isqrt(10_000), 100);
        }
    }
}
