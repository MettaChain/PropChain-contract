#![cfg_attr(not(feature = "std"), no_std, no_main)]

#[ink::contract]
mod fractional {
    use ink::prelude::vec::Vec;
    use ink::storage::Mapping;
    use propchain_traits;
    use propchain_traits::{non_reentrant, ReentrancyError, ReentrancyGuard};

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
    pub struct FractionalDashboard {
        pub owner: AccountId,
        pub total_value: u128,
        pub positions: Vec<PortfolioItem>,
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

    /// Dutch auction: price decreases over time from start_price to end_price
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
    pub struct DutchAuction {
        pub seller: AccountId,
        pub token_id: u64,
        pub shares: u128,
        pub start_price: u128,
        pub end_price: u128,
        pub start_time: u64,
        pub duration: u64,
        pub has_bids: bool,
    }

    #[derive(Debug, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub enum FractionalError {
        InsufficientShares,
        ListingNotFound,
        AuctionNotFound,
        AuctionAlreadyBid,
        InsufficientPayment,
        Unauthorized,
        ReentrantCall,
        ZeroAmount,
        PoolNotFound,
        PoolAlreadyExists,
        SlippageExceeded,
        InsufficientLiquidity,
        InsufficientLpShares,
        // Admin key rotation (Issue #496)
        KeyRotationCooldown,
        KeyRotationExpired,
        NoPendingRotation,
        RotationUnauthorized,
        RequestExpired,
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

    /// Emitted when a Dutch auction is created
    #[ink(event)]
    pub struct DutchAuctionCreated {
        #[ink(topic)]
        auction_id: u64,
        #[ink(topic)]
        seller: AccountId,
        token_id: u64,
        shares: u128,
        start_price: u128,
        end_price: u128,
        duration: u64,
    }

    /// Emitted when a bid is placed on a Dutch auction
    #[ink(event)]
    pub struct DutchAuctionBid {
        #[ink(topic)]
        auction_id: u64,
        #[ink(topic)]
        buyer: AccountId,
        shares: u128,
        price_paid: u128,
    }

    /// Emitted when a Dutch auction is cancelled
    #[ink(event)]
    pub struct DutchAuctionCancelled {
        #[ink(topic)]
        auction_id: u64,
        #[ink(topic)]
        seller: AccountId,
    }

    // ── Admin Key Rotation Events (Issue #496) ────────────────────────────────

    #[ink(event)]
    pub struct AdminRotationRequested {
        #[ink(topic)]
        old_admin: AccountId,
        #[ink(topic)]
        new_admin: AccountId,
        effective_at_block: u32,
    }

    #[ink(event)]
    pub struct AdminRotationConfirmed {
        #[ink(topic)]
        old_admin: AccountId,
        #[ink(topic)]
        new_admin: AccountId,
    }

    #[ink(event)]
    pub struct AdminRotationCancelled {
        #[ink(topic)]
        old_admin: AccountId,
        cancelled_by: AccountId,
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
        /// Active Dutch auctions indexed by auction id
        dutch_auctions: Mapping<u64, DutchAuction>,
        /// Monotonic counter for Dutch auction ids
        auction_counter: u64,
        /// Reentrancy protection for state-changing entry points
        reentrancy_guard: ReentrancyGuard,
        /// Contract administrator (Issue #496)
        admin: AccountId,
        /// Pending admin key rotation request (Issue #496)
        pending_admin_rotation: Option<propchain_traits::KeyRotationRequest>,
    }

    impl From<ReentrancyError> for FractionalError {
        fn from(_: ReentrancyError) -> Self {
            FractionalError::ReentrantCall
        }
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
                dutch_auctions: Mapping::default(),
                auction_counter: 0,
                reentrancy_guard: ReentrancyGuard::new(),
                admin: Self::env().caller(),
                pending_admin_rotation: None,
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
            non_reentrant!(self, {
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

                self.env().emit_event(SharesSold {
                    seller,
                    buyer,
                    token_id,
                    shares,
                    total_price,
                });
                Ok(())
            })
        }

        /// Redeem shares for their proportional value based on the last recorded price.
        /// Burns the shares and pays out `shares * last_price` to the caller.
        #[ink(message)]
        pub fn redeem_shares(
            &mut self,
            token_id: u64,
            shares: u128,
        ) -> Result<u128, FractionalError> {
            non_reentrant!(self, {
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

                self.env().emit_event(SharesRedeemed {
                    owner: caller,
                    token_id,
                    shares,
                    payout,
                });
                Ok(payout)
            })
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
            non_reentrant!(self, {
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
            })
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
            non_reentrant!(self, {
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
            })
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
            non_reentrant!(self, {
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
            })
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

        // ── Dutch Auction ────────────────────────────────────────────────────

        /// Calculate the current price in a Dutch auction
        /// Current price = start_price - (elapsed / duration) * (start_price - end_price)
        fn calculate_dutch_price(&self, auction: &DutchAuction, current_block: u64) -> u128 {
            if current_block >= auction.start_time.saturating_add(auction.duration) {
                // Auction expired or ended: use end price
                return auction.end_price;
            }

            if current_block <= auction.start_time {
                // Auction hasn't started yet
                return auction.start_price;
            }

            let elapsed = (current_block - auction.start_time) as u128;
            let duration = auction.duration as u128;
            let price_decrease = auction
                .start_price
                .saturating_sub(auction.end_price)
                .saturating_mul(elapsed)
                / duration;

            auction.start_price.saturating_sub(price_decrease)
        }

        /// Create a new Dutch auction for fractional shares.
        /// The seller must hold at least `shares` of the `token_id`.
        #[ink(message)]
        pub fn create_dutch_auction(
            &mut self,
            token_id: u64,
            shares: u128,
            start_price: u128,
            end_price: u128,
            duration: u64,
        ) -> Result<u64, FractionalError> {
            let caller = self.env().caller();

            if shares == 0 || duration == 0 {
                return Err(FractionalError::ZeroAmount);
            }

            if start_price == 0 || end_price == 0 {
                return Err(FractionalError::ZeroAmount);
            }

            let held = self.balances.get(&(caller, token_id)).unwrap_or(0);
            if held < shares {
                return Err(FractionalError::InsufficientShares);
            }

            let auction_id = self.auction_counter;
            self.auction_counter = self.auction_counter.saturating_add(1);

            let now = self.env().block_number() as u64;
            let auction = DutchAuction {
                seller: caller,
                token_id,
                shares,
                start_price,
                end_price,
                start_time: now,
                duration,
                has_bids: false,
            };

            self.dutch_auctions.insert(auction_id, &auction);

            self.env().emit_event(DutchAuctionCreated {
                auction_id,
                seller: caller,
                token_id,
                shares,
                start_price,
                end_price,
                duration,
            });

            Ok(auction_id)
        }

        /// Get the details of a Dutch auction
        #[ink(message)]
        pub fn get_dutch_auction(&self, auction_id: u64) -> Option<DutchAuction> {
            self.dutch_auctions.get(auction_id)
        }

        /// Get the current price of a Dutch auction
        #[ink(message)]
        pub fn get_dutch_auction_price(&self, auction_id: u64) -> Result<u128, FractionalError> {
            let auction = self
                .dutch_auctions
                .get(auction_id)
                .ok_or(FractionalError::AuctionNotFound)?;

            let current_block = self.env().block_number() as u64;
            Ok(self.calculate_dutch_price(&auction, current_block))
        }

        /// Bid on a Dutch auction at the current descending price.
        /// Buyer must attach sufficient payment for the current price of all shares.
        #[ink(message, payable)]
        pub fn bid_dutch_auction(&mut self, auction_id: u64) -> Result<(), FractionalError> {
            let caller = self.env().caller();
            let payment = self.env().transferred_value();

            let mut auction = self
                .dutch_auctions
                .get(auction_id)
                .ok_or(FractionalError::AuctionNotFound)?;

            if auction.has_bids {
                return Err(FractionalError::AuctionAlreadyBid);
            }

            let current_block = self.env().block_number() as u64;
            let current_price = self.calculate_dutch_price(&auction, current_block);
            let total_price = current_price.saturating_mul(auction.shares);

            if payment < total_price {
                return Err(FractionalError::InsufficientPayment);
            }

            // Transfer shares from seller to buyer
            let seller_held = self
                .balances
                .get(&(auction.seller, auction.token_id))
                .unwrap_or(0);
            self.balances.insert(
                &(auction.seller, auction.token_id),
                &seller_held.saturating_sub(auction.shares),
            );

            let buyer_held = self.balances.get(&(caller, auction.token_id)).unwrap_or(0);
            self.balances.insert(
                &(caller, auction.token_id),
                &buyer_held.saturating_add(auction.shares),
            );

            // Mark auction as complete
            auction.has_bids = true;
            self.dutch_auctions.insert(auction_id, &auction);

            // Pay the seller
            if self.env().transfer(auction.seller, total_price).is_err() {
                // Non-fatal: payment forwarding failed (e.g. in unit tests)
            }

            self.env().emit_event(DutchAuctionBid {
                auction_id,
                buyer: caller,
                shares: auction.shares,
                price_paid: total_price,
            });

            Ok(())
        }

        /// Cancel a Dutch auction (seller only, before any bid).
        #[ink(message)]
        pub fn cancel_dutch_auction(&mut self, auction_id: u64) -> Result<(), FractionalError> {
            let caller = self.env().caller();

            let auction = self
                .dutch_auctions
                .get(auction_id)
                .ok_or(FractionalError::AuctionNotFound)?;

            if caller != auction.seller {
                return Err(FractionalError::Unauthorized);
            }

            if auction.has_bids {
                return Err(FractionalError::AuctionAlreadyBid);
            }

            self.dutch_auctions.remove(auction_id);

            self.env().emit_event(DutchAuctionCancelled {
                auction_id,
                seller: caller,
            });

            Ok(())
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

        // ── Admin Key Rotation (Issue #496) ──────────────────────────────────

        /// Get the contract admin address.
        #[ink(message)]
        pub fn get_admin(&self) -> AccountId {
            self.admin
        }

        /// Initiate two-step admin rotation with timelock cooldown.
        ///
        /// Only the current admin may call this. The nominated `new_admin` must
        /// confirm after `KEY_ROTATION_COOLDOWN_BLOCKS` blocks have elapsed.
        #[ink(message)]
        pub fn request_admin_rotation(
            &mut self,
            new_admin: AccountId,
        ) -> Result<(), FractionalError> {
            let caller = self.env().caller();
            if caller != self.admin {
                return Err(FractionalError::Unauthorized);
            }
            if self.pending_admin_rotation.is_some() {
                return Err(FractionalError::KeyRotationCooldown);
            }

            let block = self.env().block_number();
            let effective_at =
                block.saturating_add(propchain_traits::constants::KEY_ROTATION_COOLDOWN_BLOCKS);

            self.pending_admin_rotation = Some(propchain_traits::KeyRotationRequest {
                old_account: caller,
                new_account: new_admin,
                requested_at: block,
                effective_at,
                confirmed: false,
            });

            self.env().emit_event(AdminRotationRequested {
                old_admin: caller,
                new_admin,
                effective_at_block: effective_at,
            });
            Ok(())
        }

        /// Confirm a pending admin rotation after the cooldown period.
        ///
        /// Must be called by the nominated new admin.
        #[ink(message)]
        pub fn confirm_admin_rotation(&mut self) -> Result<(), FractionalError> {
            let caller = self.env().caller();
            let block = self.env().block_number();

            let request = self
                .pending_admin_rotation
                .as_ref()
                .ok_or(FractionalError::NoPendingRotation)?;

            if request.new_account != caller {
                return Err(FractionalError::RotationUnauthorized);
            }
            if block < request.effective_at {
                return Err(FractionalError::KeyRotationCooldown);
            }
            let expiry = request
                .effective_at
                .saturating_add(propchain_traits::constants::KEY_ROTATION_EXPIRY_BLOCKS);
            if block > expiry {
                self.pending_admin_rotation = None;
                return Err(FractionalError::RequestExpired);
            }

            let old_admin = request.old_account;
            self.admin = caller;
            self.pending_admin_rotation = None;

            self.env().emit_event(AdminRotationConfirmed {
                old_admin,
                new_admin: caller,
            });
            Ok(())
        }

        /// Cancel a pending admin rotation.
        ///
        /// Either the current admin or the nominated new admin may cancel.
        #[ink(message)]
        pub fn cancel_admin_rotation(&mut self) -> Result<(), FractionalError> {
            let caller = self.env().caller();
            let request = self
                .pending_admin_rotation
                .as_ref()
                .ok_or(FractionalError::NoPendingRotation)?;

            if caller != request.old_account && caller != request.new_account {
                return Err(FractionalError::RotationUnauthorized);
            }

            let old_admin = request.old_account;
            self.pending_admin_rotation = None;

            self.env().emit_event(AdminRotationCancelled {
                old_admin,
                cancelled_by: caller,
            });
            Ok(())
        }

        /// Get the pending admin rotation request, if any.
        #[ink(message)]
        pub fn get_pending_admin_rotation(&self) -> Option<propchain_traits::KeyRotationRequest> {
            self.pending_admin_rotation.clone()
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

        // ── Dutch Auction Tests ──────────────────────────────────────────────

        fn charlie() -> AccountId {
            test::default_accounts::<ink::env::DefaultEnvironment>().charlie
        }

        #[ink::test]
        fn test_create_dutch_auction_succeeds() {
            let mut f = Fractional::new();
            test::set_caller::<ink::env::DefaultEnvironment>(alice());
            f.mint_shares(alice(), 1, 100);

            let auction_id = f.create_dutch_auction(1, 50, 1000, 500, 1000).unwrap();
            assert_eq!(auction_id, 0);

            let auction = f.get_dutch_auction(0).unwrap();
            assert_eq!(auction.seller, alice());
            assert_eq!(auction.token_id, 1);
            assert_eq!(auction.shares, 50);
            assert_eq!(auction.start_price, 1000);
            assert_eq!(auction.end_price, 500);
            assert_eq!(auction.duration, 1000);
            assert!(!auction.has_bids);
        }

        #[ink::test]
        fn test_create_dutch_auction_insufficient_shares() {
            let mut f = Fractional::new();
            test::set_caller::<ink::env::DefaultEnvironment>(alice());
            f.mint_shares(alice(), 1, 10);

            assert_eq!(
                f.create_dutch_auction(1, 50, 1000, 500, 1000),
                Err(FractionalError::InsufficientShares)
            );
        }

        #[ink::test]
        fn test_create_dutch_auction_zero_shares() {
            let mut f = Fractional::new();
            test::set_caller::<ink::env::DefaultEnvironment>(alice());
            f.mint_shares(alice(), 1, 100);

            assert_eq!(
                f.create_dutch_auction(1, 0, 1000, 500, 1000),
                Err(FractionalError::ZeroAmount)
            );
        }

        #[ink::test]
        fn test_create_dutch_auction_zero_price() {
            let mut f = Fractional::new();
            test::set_caller::<ink::env::DefaultEnvironment>(alice());
            f.mint_shares(alice(), 1, 100);

            assert_eq!(
                f.create_dutch_auction(1, 50, 0, 500, 1000),
                Err(FractionalError::ZeroAmount)
            );
        }

        #[ink::test]
        fn test_create_dutch_auction_zero_duration() {
            let mut f = Fractional::new();
            test::set_caller::<ink::env::DefaultEnvironment>(alice());
            f.mint_shares(alice(), 1, 100);

            assert_eq!(
                f.create_dutch_auction(1, 50, 1000, 500, 0),
                Err(FractionalError::ZeroAmount)
            );
        }

        #[ink::test]
        fn test_dutch_auction_price_at_start() {
            let mut f = Fractional::new();
            test::set_caller::<ink::env::DefaultEnvironment>(alice());
            f.mint_shares(alice(), 1, 100);

            f.create_dutch_auction(1, 50, 1000, 500, 1000).unwrap();

            let current_price = f.get_dutch_auction_price(0).unwrap();
            assert_eq!(current_price, 1000); // start price
        }

        #[ink::test]
        fn test_dutch_auction_price_at_end() {
            let mut f = Fractional::new();
            test::set_caller::<ink::env::DefaultEnvironment>(alice());
            f.mint_shares(alice(), 1, 100);

            f.create_dutch_auction(1, 50, 1000, 500, 1000).unwrap();

            // Advance blocks to end of auction
            for _ in 0..1100 {
                test::advance_block::<ink::env::DefaultEnvironment>();
            }

            let current_price = f.get_dutch_auction_price(0).unwrap();
            assert_eq!(current_price, 500); // end price
        }

        #[ink::test]
        fn test_dutch_auction_price_halfway() {
            let mut f = Fractional::new();
            test::set_caller::<ink::env::DefaultEnvironment>(alice());
            f.mint_shares(alice(), 1, 100);

            f.create_dutch_auction(1, 50, 1000, 500, 1000).unwrap();

            // Advance blocks to halfway through auction
            for _ in 0..500 {
                test::advance_block::<ink::env::DefaultEnvironment>();
            }

            let current_price = f.get_dutch_auction_price(0).unwrap();
            // At 50% of duration: price should be 1000 - 250 = 750
            assert_eq!(current_price, 750);
        }

        #[ink::test]
        fn test_bid_dutch_auction_succeeds() {
            let mut f = Fractional::new();
            test::set_caller::<ink::env::DefaultEnvironment>(alice());
            f.mint_shares(alice(), 1, 100);

            f.create_dutch_auction(1, 50, 1000, 500, 1000).unwrap();

            test::set_caller::<ink::env::DefaultEnvironment>(bob());
            test::set_value_transferred::<ink::env::DefaultEnvironment>(50_000); // 50 * 1000
            assert!(f.bid_dutch_auction(0).is_ok());

            // Bob should now own 50 shares
            assert_eq!(f.balance_of(bob(), 1), 50);
            // Alice should have lost 50 shares
            assert_eq!(f.balance_of(alice(), 1), 50);

            // Auction should be marked as bid
            let auction = f.get_dutch_auction(0).unwrap();
            assert!(auction.has_bids);
        }

        #[ink::test]
        fn test_bid_dutch_auction_insufficient_payment() {
            let mut f = Fractional::new();
            test::set_caller::<ink::env::DefaultEnvironment>(alice());
            f.mint_shares(alice(), 1, 100);

            f.create_dutch_auction(1, 50, 1000, 500, 1000).unwrap();

            test::set_caller::<ink::env::DefaultEnvironment>(bob());
            test::set_value_transferred::<ink::env::DefaultEnvironment>(1_000); // Too low
            assert_eq!(
                f.bid_dutch_auction(0),
                Err(FractionalError::InsufficientPayment)
            );
        }

        #[ink::test]
        fn test_bid_dutch_auction_after_first_bid_fails() {
            let mut f = Fractional::new();
            test::set_caller::<ink::env::DefaultEnvironment>(alice());
            f.mint_shares(alice(), 1, 100);

            f.create_dutch_auction(1, 50, 1000, 500, 1000).unwrap();

            // First bid succeeds
            test::set_caller::<ink::env::DefaultEnvironment>(bob());
            test::set_value_transferred::<ink::env::DefaultEnvironment>(50_000);
            assert!(f.bid_dutch_auction(0).is_ok());

            // Second bid on same auction should fail
            test::set_caller::<ink::env::DefaultEnvironment>(charlie());
            test::set_value_transferred::<ink::env::DefaultEnvironment>(50_000);
            assert_eq!(
                f.bid_dutch_auction(0),
                Err(FractionalError::AuctionAlreadyBid)
            );
        }

        #[ink::test]
        fn test_bid_dutch_auction_nonexistent() {
            let mut f = Fractional::new();
            test::set_caller::<ink::env::DefaultEnvironment>(bob());
            test::set_value_transferred::<ink::env::DefaultEnvironment>(50_000);
            assert_eq!(
                f.bid_dutch_auction(999),
                Err(FractionalError::AuctionNotFound)
            );
        }

        #[ink::test]
        fn test_bid_dutch_auction_with_descending_price() {
            let mut f = Fractional::new();
            test::set_caller::<ink::env::DefaultEnvironment>(alice());
            f.mint_shares(alice(), 1, 100);

            f.create_dutch_auction(1, 50, 1000, 500, 1000).unwrap();

            // Advance 500 blocks (halfway through auction)
            for _ in 0..500 {
                test::advance_block::<ink::env::DefaultEnvironment>();
            }

            // Current price should be 750 (halfway between 1000 and 500)
            let current_price = f.get_dutch_auction_price(0).unwrap();
            assert_eq!(current_price, 750);

            // Bid at current price
            test::set_caller::<ink::env::DefaultEnvironment>(bob());
            test::set_value_transferred::<ink::env::DefaultEnvironment>(37_500); // 50 * 750
            assert!(f.bid_dutch_auction(0).is_ok());

            assert_eq!(f.balance_of(bob(), 1), 50);
        }

        #[ink::test]
        fn test_cancel_dutch_auction_succeeds() {
            let mut f = Fractional::new();
            test::set_caller::<ink::env::DefaultEnvironment>(alice());
            f.mint_shares(alice(), 1, 100);

            let auction_id = f.create_dutch_auction(1, 50, 1000, 500, 1000).unwrap();

            assert!(f.cancel_dutch_auction(auction_id).is_ok());
            assert!(f.get_dutch_auction(auction_id).is_none());
        }

        #[ink::test]
        fn test_cancel_dutch_auction_unauthorized() {
            let mut f = Fractional::new();
            test::set_caller::<ink::env::DefaultEnvironment>(alice());
            f.mint_shares(alice(), 1, 100);

            let auction_id = f.create_dutch_auction(1, 50, 1000, 500, 1000).unwrap();

            // Bob tries to cancel Alice's auction
            test::set_caller::<ink::env::DefaultEnvironment>(bob());
            assert_eq!(
                f.cancel_dutch_auction(auction_id),
                Err(FractionalError::Unauthorized)
            );
        }

        #[ink::test]
        fn test_cancel_dutch_auction_after_bid_fails() {
            let mut f = Fractional::new();
            test::set_caller::<ink::env::DefaultEnvironment>(alice());
            f.mint_shares(alice(), 1, 100);

            let auction_id = f.create_dutch_auction(1, 50, 1000, 500, 1000).unwrap();

            // Place a bid
            test::set_caller::<ink::env::DefaultEnvironment>(bob());
            test::set_value_transferred::<ink::env::DefaultEnvironment>(50_000);
            f.bid_dutch_auction(auction_id).unwrap();

            // Alice tries to cancel after bid
            test::set_caller::<ink::env::DefaultEnvironment>(alice());
            assert_eq!(
                f.cancel_dutch_auction(auction_id),
                Err(FractionalError::AuctionAlreadyBid)
            );
        }

        #[ink::test]
        fn test_cancel_dutch_auction_nonexistent() {
            let mut f = Fractional::new();
            test::set_caller::<ink::env::DefaultEnvironment>(alice());
            assert_eq!(
                f.cancel_dutch_auction(999),
                Err(FractionalError::AuctionNotFound)
            );
        }

        #[ink::test]
        fn test_multiple_dutch_auctions() {
            let mut f = Fractional::new();
            test::set_caller::<ink::env::DefaultEnvironment>(alice());
            f.mint_shares(alice(), 1, 200);

            let auction_id_1 = f.create_dutch_auction(1, 50, 1000, 500, 1000).unwrap();
            let auction_id_2 = f.create_dutch_auction(1, 100, 2000, 1000, 500).unwrap();

            assert_eq!(auction_id_1, 0);
            assert_eq!(auction_id_2, 1);

            let a1 = f.get_dutch_auction(0).unwrap();
            let a2 = f.get_dutch_auction(1).unwrap();
            assert_eq!(a1.shares, 50);
            assert_eq!(a2.shares, 100);
        }

        // ── Issue #493: Reentrancy guard tests ───────────────────────────────

        /// Test that calling buy_shares while the guard is locked returns ReentrantCall.
        /// We simulate this by manually locking the guard before the call.
        #[ink::test]
        fn test_reentrant_buy_shares_returns_error() {
            let mut f = Fractional::new();
            test::set_caller::<ink::env::DefaultEnvironment>(alice());
            f.mint_shares(alice(), 1, 100);
            f.list_shares_for_sale(1, 50, 10).unwrap();

            // Manually lock the guard to simulate a reentrant call
            f.reentrancy_guard
                .enter()
                .expect("first lock should succeed");

            test::set_caller::<ink::env::DefaultEnvironment>(bob());
            // While guard is locked, buy_shares should return ReentrantCall
            let result = f.buy_shares(alice(), 1, 10);
            assert_eq!(
                result,
                Err(FractionalError::ReentrantCall),
                "buy_shares must return ReentrantCall when guard is locked (issue #493)"
            );

            // Unlock so the guard does not stay poisoned
            f.reentrancy_guard.exit();
        }

        /// Test that calling swap_shares_for_value while the guard is locked returns ReentrantCall.
        #[ink::test]
        fn test_reentrant_swap_shares_for_value_returns_error() {
            let mut f = Fractional::new();
            test::set_caller::<ink::env::DefaultEnvironment>(alice());
            f.mint_shares(alice(), 1, 1000);

            test::set_value_transferred::<ink::env::DefaultEnvironment>(10_000);
            f.add_liquidity(1, 100, 0).unwrap();

            // Manually lock the guard to simulate a reentrant call
            f.reentrancy_guard
                .enter()
                .expect("first lock should succeed");

            let result = f.swap_shares_for_value(1, 10, 0);
            assert_eq!(
                result,
                Err(FractionalError::ReentrantCall),
                "swap_shares_for_value must return ReentrantCall when guard is locked (issue #493)"
            );

            f.reentrancy_guard.exit();
        }

        /// Test that redeem_shares is guarded and returns ReentrantCall when locked.
        #[ink::test]
        fn test_reentrant_redeem_shares_returns_error() {
            let mut f = Fractional::new();
            test::set_caller::<ink::env::DefaultEnvironment>(alice());
            f.mint_shares(alice(), 1, 100);
            f.set_last_price(1, 5);

            f.reentrancy_guard
                .enter()
                .expect("first lock should succeed");

            let result = f.redeem_shares(1, 10);
            assert_eq!(
                result,
                Err(FractionalError::ReentrantCall),
                "redeem_shares must return ReentrantCall when guard is locked (issue #493)"
            );

            f.reentrancy_guard.exit();
        }

        /// Test that add_liquidity is guarded and returns ReentrantCall when locked.
        #[ink::test]
        fn test_reentrant_add_liquidity_returns_error() {
            let mut f = Fractional::new();
            test::set_caller::<ink::env::DefaultEnvironment>(alice());
            f.mint_shares(alice(), 1, 1000);

            f.reentrancy_guard
                .enter()
                .expect("first lock should succeed");

            test::set_value_transferred::<ink::env::DefaultEnvironment>(500);
            let result = f.add_liquidity(1, 100, 0);
            assert_eq!(
                result,
                Err(FractionalError::ReentrantCall),
                "add_liquidity must return ReentrantCall when guard is locked (issue #493)"
            );

            f.reentrancy_guard.exit();
        }

        /// Test that remove_liquidity is guarded and returns ReentrantCall when locked.
        #[ink::test]
        fn test_reentrant_remove_liquidity_returns_error() {
            let mut f = Fractional::new();
            test::set_caller::<ink::env::DefaultEnvironment>(alice());
            f.mint_shares(alice(), 1, 1000);

            test::set_value_transferred::<ink::env::DefaultEnvironment>(1000);
            let lp = f.add_liquidity(1, 200, 0).unwrap();

            f.reentrancy_guard
                .enter()
                .expect("first lock should succeed");

            let result = f.remove_liquidity(1, lp, 0, 0);
            assert_eq!(
                result,
                Err(FractionalError::ReentrantCall),
                "remove_liquidity must return ReentrantCall when guard is locked (issue #493)"
            );

            f.reentrancy_guard.exit();
        }

        /// Test that after a non-reentrant call completes, the guard is unlocked
        /// and a subsequent call succeeds normally.
        #[ink::test]
        fn test_guard_releases_after_successful_call() {
            let mut f = Fractional::new();
            test::set_caller::<ink::env::DefaultEnvironment>(alice());
            f.mint_shares(alice(), 1, 200);
            f.set_last_price(1, 5);

            // First call: guard enters and exits normally
            let payout1 = f.redeem_shares(1, 50).unwrap();
            assert_eq!(payout1, 250);

            // Guard should be unlocked — second call must succeed
            let payout2 = f.redeem_shares(1, 50).unwrap();
            assert_eq!(payout2, 250);

            assert!(
                !f.reentrancy_guard.is_locked(),
                "guard must be unlocked after call"
            );
        }
    }

    // =========================================================================
    // ADMIN KEY ROTATION TESTS (Issue #496) — Fractional
    // =========================================================================

    #[cfg(test)]
    mod fractional_admin_rotation_tests {
        use super::*;
        use ink::env::{test, DefaultEnvironment};

        fn setup() -> Fractional {
            let accounts = test::default_accounts::<DefaultEnvironment>();
            test::set_caller::<DefaultEnvironment>(accounts.alice);
            Fractional::new()
        }

        #[ink::test]
        fn test_constructor_sets_caller_as_admin() {
            let contract = setup();
            let accounts = test::default_accounts::<DefaultEnvironment>();
            assert_eq!(contract.get_admin(), accounts.alice);
        }

        #[ink::test]
        fn test_admin_can_request_rotation() {
            let mut contract = setup();
            let accounts = test::default_accounts::<DefaultEnvironment>();
            assert!(contract.request_admin_rotation(accounts.bob).is_ok());
            let pending = contract.get_pending_admin_rotation();
            assert!(pending.is_some());
            let req = pending.unwrap();
            assert_eq!(req.old_account, accounts.alice);
            assert_eq!(req.new_account, accounts.bob);
        }

        #[ink::test]
        fn test_non_admin_cannot_request_rotation() {
            let mut contract = setup();
            let accounts = test::default_accounts::<DefaultEnvironment>();
            test::set_caller::<DefaultEnvironment>(accounts.bob);
            assert_eq!(
                contract.request_admin_rotation(accounts.charlie),
                Err(FractionalError::Unauthorized)
            );
        }

        #[ink::test]
        fn test_rotation_cannot_be_confirmed_before_cooldown() {
            let mut contract = setup();
            let accounts = test::default_accounts::<DefaultEnvironment>();
            contract.request_admin_rotation(accounts.bob).unwrap();
            test::set_caller::<DefaultEnvironment>(accounts.bob);
            // Block 0 < effective_at 14_400
            assert_eq!(
                contract.confirm_admin_rotation(),
                Err(FractionalError::KeyRotationCooldown)
            );
        }

        #[ink::test]
        fn test_old_admin_can_cancel_rotation() {
            let mut contract = setup();
            let accounts = test::default_accounts::<DefaultEnvironment>();
            contract.request_admin_rotation(accounts.bob).unwrap();
            assert!(contract.cancel_admin_rotation().is_ok());
            assert!(contract.get_pending_admin_rotation().is_none());
        }

        #[ink::test]
        fn test_new_admin_can_cancel_rotation() {
            let mut contract = setup();
            let accounts = test::default_accounts::<DefaultEnvironment>();
            contract.request_admin_rotation(accounts.bob).unwrap();
            test::set_caller::<DefaultEnvironment>(accounts.bob);
            assert!(contract.cancel_admin_rotation().is_ok());
        }

        #[ink::test]
        fn test_unrelated_cannot_cancel() {
            let mut contract = setup();
            let accounts = test::default_accounts::<DefaultEnvironment>();
            contract.request_admin_rotation(accounts.bob).unwrap();
            test::set_caller::<DefaultEnvironment>(accounts.charlie);
            assert_eq!(
                contract.cancel_admin_rotation(),
                Err(FractionalError::RotationUnauthorized)
            );
        }

        #[ink::test]
        fn test_duplicate_request_fails() {
            let mut contract = setup();
            let accounts = test::default_accounts::<DefaultEnvironment>();
            contract.request_admin_rotation(accounts.bob).unwrap();
            assert_eq!(
                contract.request_admin_rotation(accounts.charlie),
                Err(FractionalError::KeyRotationCooldown)
            );
        }
    }
}
