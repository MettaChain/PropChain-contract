/// # Integration Tests: Fractional Share Trading (Issue #1008)
///
/// These tests verify the end-to-end exit-liquidity pipeline of the
/// `fractional` contract:
///   mint -> list -> buy (with transferred value) -> settlement
///
/// Acceptance criteria tested:
///   check mint_shares credits the owner and is visible via balance_of
///   check list_shares_for_sale records a listing priced per share
///   check buy_shares with sufficient attached value settles ownership
///   check Seller and buyer balances update after a full purchase
///   check Partial purchases keep the listing alive with reduced quantity
///   check Underpayment is rejected with InsufficientPayment
///   check cancel_listing removes the listing and blocks subsequent buys
///   check Sellers cannot list more shares than they hold
///   check A listing records its creation timestamp and optional deadline
///   check A listing without a deadline never expires
///   check An expired listing cannot be purchased and is swept (Issue #1146)
///   check Renewal restamps the clock and makes the listing buyable again
///   check Only the seller can renew, and only an existing listing
///   check Cancelling a live listing still works unchanged
#[cfg(test)]
#[allow(clippy::module_inception)]
mod integration_fractional {
    // Fractional share contract
    use fractional::fractional::{Fractional, FractionalError};
    use ink::env::{test, DefaultEnvironment};

    const TOKEN_ID: u64 = 1;
    const PRICE_PER_SHARE: u128 = 25;

    /// Deploy the contract; constructor caller becomes admin (alice).
    fn setup() -> Fractional {
        let accounts = test::default_accounts::<DefaultEnvironment>();
        test::set_caller::<DefaultEnvironment>(accounts.alice);
        Fractional::new()
    }

    /// Mint `amount` shares of TOKEN_ID to `owner`.
    fn mint(contract: &mut Fractional, owner: ink::primitives::AccountId, amount: u128) {
        contract.mint_shares(owner, TOKEN_ID, amount);
    }

    /// Full happy path: mint -> list -> full purchase.
    /// Ownership moves entirely for the sold quantity and the listing closes.
    #[ink::test]
    fn test_mint_list_buy_full_settlement() {
        let accounts = test::default_accounts::<DefaultEnvironment>();
        let mut fractional = setup();

        // Mint to alice
        mint(&mut fractional, accounts.alice, 1_000);
        assert_eq!(fractional.balance_of(accounts.alice, TOKEN_ID), 1_000);

        // Alice lists 400 shares at PRICE_PER_SHARE each
        test::set_caller::<DefaultEnvironment>(accounts.alice);
        fractional
            .list_shares_for_sale(TOKEN_ID, 400, PRICE_PER_SHARE, None)
            .expect("Owner should list shares for sale");

        let listing = fractional
            .get_listing(accounts.alice, TOKEN_ID)
            .expect("Listing should be recorded");
        assert_eq!(listing.seller, accounts.alice);
        assert_eq!(listing.shares, 400);
        assert_eq!(listing.price_per_share, PRICE_PER_SHARE);

        // Bob buys the whole listing with exact payment
        let total_price = PRICE_PER_SHARE * 400;
        test::set_caller::<DefaultEnvironment>(accounts.bob);
        test::set_value_transferred::<DefaultEnvironment>(total_price);
        fractional
            .buy_shares(accounts.alice, TOKEN_ID, 400)
            .expect("Exact payment must settle the purchase");

        // Balances moved accordingly
        assert_eq!(fractional.balance_of(accounts.bob, TOKEN_ID), 400);
        assert_eq!(fractional.balance_of(accounts.alice, TOKEN_ID), 600);

        // Fully consumed listing is removed
        assert!(
            fractional.get_listing(accounts.alice, TOKEN_ID).is_none(),
            "A fully bought-out listing must be closed"
        );
    }

    /// Partial purchase keeps the listing alive with reduced quantity.
    #[ink::test]
    fn test_partial_purchase_updates_listing() {
        let accounts = test::default_accounts::<DefaultEnvironment>();
        let mut fractional = setup();

        mint(&mut fractional, accounts.alice, 500);
        test::set_caller::<DefaultEnvironment>(accounts.alice);
        fractional
            .list_shares_for_sale(TOKEN_ID, 300, PRICE_PER_SHARE, None)
            .expect("Listing should succeed");

        // Charlie buys 120 shares
        test::set_caller::<DefaultEnvironment>(accounts.charlie);
        test::set_value_transferred::<DefaultEnvironment>(PRICE_PER_SHARE * 120);
        fractional
            .buy_shares(accounts.alice, TOKEN_ID, 120)
            .expect("Partial purchase should succeed");

        assert_eq!(fractional.balance_of(accounts.charlie, TOKEN_ID), 120);
        assert_eq!(fractional.balance_of(accounts.alice, TOKEN_ID), 380);

        // Listing survives with reduced quantity
        let remaining = fractional
            .get_listing(accounts.alice, TOKEN_ID)
            .expect("Partial purchase must keep the listing open");
        assert_eq!(remaining.shares, 180);
    }

    /// Underpayment is rejected and leaves all state untouched.
    #[ink::test]
    fn test_underpayment_rejected() {
        let accounts = test::default_accounts::<DefaultEnvironment>();
        let mut fractional = setup();

        mint(&mut fractional, accounts.alice, 200);
        test::set_caller::<DefaultEnvironment>(accounts.alice);
        fractional
            .list_shares_for_sale(TOKEN_ID, 100, PRICE_PER_SHARE, None)
            .expect("Listing should succeed");

        // Buyer attaches too little value
        test::set_caller::<DefaultEnvironment>(accounts.bob);
        test::set_value_transferred::<DefaultEnvironment>(PRICE_PER_SHARE * 100 - 1);
        assert_eq!(
            fractional.buy_shares(accounts.alice, TOKEN_ID, 100),
            Err(FractionalError::InsufficientPayment),
            "Underpaid purchases must be rejected"
        );

        // Nothing moved: seller still holds everything, listing still open
        assert_eq!(fractional.balance_of(accounts.bob, TOKEN_ID), 0);
        assert_eq!(fractional.balance_of(accounts.alice, TOKEN_ID), 200);
        assert!(fractional.get_listing(accounts.alice, TOKEN_ID).is_some());
    }

    /// Buying more shares than listed is rejected even with ample payment.
    #[ink::test]
    fn test_oversized_purchase_rejected() {
        let accounts = test::default_accounts::<DefaultEnvironment>();
        let mut fractional = setup();

        mint(&mut fractional, accounts.alice, 100);
        test::set_caller::<DefaultEnvironment>(accounts.alice);
        fractional
            .list_shares_for_sale(TOKEN_ID, 50, PRICE_PER_SHARE, None)
            .expect("Listing should succeed");

        test::set_caller::<DefaultEnvironment>(accounts.bob);
        test::set_value_transferred::<DefaultEnvironment>(PRICE_PER_SHARE * 60);
        assert_eq!(
            fractional.buy_shares(accounts.alice, TOKEN_ID, 60),
            Err(FractionalError::InsufficientShares),
            "Purchases exceeding the listing size must be rejected"
        );
    }

    /// Cancel path: seller cancels the listing, afterwards buys are impossible.
    #[ink::test]
    fn test_cancel_listing_blocks_purchase() {
        let accounts = test::default_accounts::<DefaultEnvironment>();
        let mut fractional = setup();

        mint(&mut fractional, accounts.alice, 300);
        test::set_caller::<DefaultEnvironment>(accounts.alice);
        fractional
            .list_shares_for_sale(TOKEN_ID, 150, PRICE_PER_SHARE, None)
            .expect("Listing should succeed");

        // Non-seller cannot cancel
        test::set_caller::<DefaultEnvironment>(accounts.bob);
        assert_eq!(
            fractional.cancel_listing(TOKEN_ID),
            Err(FractionalError::ListingNotFound),
            "Only the listing owner may cancel (bob has no listing)"
        );

        // Seller cancels successfully
        test::set_caller::<DefaultEnvironment>(accounts.alice);
        fractional
            .cancel_listing(TOKEN_ID)
            .expect("Seller should cancel own listing");
        assert!(fractional.get_listing(accounts.alice, TOKEN_ID).is_none());

        // A buyer can no longer purchase against the cancelled listing
        test::set_caller::<DefaultEnvironment>(accounts.bob);
        test::set_value_transferred::<DefaultEnvironment>(PRICE_PER_SHARE * 10);
        assert_eq!(
            fractional.buy_shares(accounts.alice, TOKEN_ID, 10),
            Err(FractionalError::ListingNotFound),
            "Cancelled listings must not be purchasable"
        );
    }

    /// Listing guard: sellers cannot offer more than they hold; zero amounts
    /// are rejected outright.
    #[ink::test]
    fn test_listing_guards() {
        let accounts = test::default_accounts::<DefaultEnvironment>();
        let mut fractional = setup();

        mint(&mut fractional, accounts.alice, 50);

        // Over-listing rejected
        test::set_caller::<DefaultEnvironment>(accounts.alice);
        assert_eq!(
            fractional.list_shares_for_sale(TOKEN_ID, 51, PRICE_PER_SHARE, None),
            Err(FractionalError::InsufficientShares),
            "Listing more than held must be rejected"
        );

        // Zero-share listing rejected
        assert_eq!(
            fractional.list_shares_for_sale(TOKEN_ID, 0, PRICE_PER_SHARE, None),
            Err(FractionalError::ZeroAmount),
            "Zero-quantity listings must be rejected"
        );

        // Zero-share purchase rejected
        test::set_value_transferred::<DefaultEnvironment>(PRICE_PER_SHARE);
        assert_eq!(
            fractional.buy_shares(accounts.alice, TOKEN_ID, 0),
            Err(FractionalError::ZeroAmount),
            "Zero-quantity purchases must be rejected"
        );
    }

    // ---- Listing expiry (Issue #1146) ----

    /// A new listing records the creation timestamp and the requested deadline.
    #[ink::test]
    fn test_listing_records_timestamp_and_deadline() {
        let accounts = test::default_accounts::<DefaultEnvironment>();
        let mut fractional = setup();

        mint(&mut fractional, accounts.alice, 200);
        test::set_caller::<DefaultEnvironment>(accounts.alice);
        test::set_block_timestamp::<DefaultEnvironment>(1_000);
        fractional
            .list_shares_for_sale(TOKEN_ID, 200, PRICE_PER_SHARE, Some(1_600))
            .expect("Listing with a deadline should succeed");

        let listing = fractional
            .get_listing(accounts.alice, TOKEN_ID)
            .expect("Listing should be recorded");
        assert_eq!(listing.listed_at, 1_000);
        assert_eq!(listing.expires_at, Some(1_600));
        assert!(
            fractional.is_listing_active(accounts.alice, TOKEN_ID),
            "A listing before its deadline is active"
        );
    }

    /// A listing without a deadline never expires, however far time moves on.
    #[ink::test]
    fn test_listing_without_deadline_never_expires() {
        let accounts = test::default_accounts::<DefaultEnvironment>();
        let mut fractional = setup();

        mint(&mut fractional, accounts.alice, 100);
        test::set_caller::<DefaultEnvironment>(accounts.alice);
        fractional
            .list_shares_for_sale(TOKEN_ID, 100, PRICE_PER_SHARE, None)
            .expect("Non-expiring listing should succeed");
        assert_eq!(
            fractional
                .get_listing(accounts.alice, TOKEN_ID)
                .expect("Listing should be recorded")
                .expires_at,
            None,
            "A non-expiring listing must carry no deadline"
        );

        test::set_block_timestamp::<DefaultEnvironment>(9_999_999);
        assert!(
            fractional.is_listing_active(accounts.alice, TOKEN_ID),
            "A deadline-free listing stays active forever"
        );

        test::set_caller::<DefaultEnvironment>(accounts.bob);
        test::set_value_transferred::<DefaultEnvironment>(PRICE_PER_SHARE * 40);
        fractional
            .buy_shares(accounts.alice, TOKEN_ID, 40)
            .expect("A non-expiring listing remains purchasable");
        assert_eq!(fractional.balance_of(accounts.bob, TOKEN_ID), 40);
    }

    /// Core acceptance criterion: once the deadline passes the listing cannot be
    /// bought, the stale ask is swept, and no shares or payments move.
    #[ink::test]
    fn test_expired_listing_cannot_be_purchased() {
        let accounts = test::default_accounts::<DefaultEnvironment>();
        let mut fractional = setup();

        mint(&mut fractional, accounts.alice, 150);
        test::set_caller::<DefaultEnvironment>(accounts.alice);
        test::set_block_timestamp::<DefaultEnvironment>(1_000);
        fractional
            .list_shares_for_sale(TOKEN_ID, 150, PRICE_PER_SHARE, Some(1_100))
            .expect("Listing should succeed");

        // Buyer arrives after the deadline, with ample payment attached.
        test::set_block_timestamp::<DefaultEnvironment>(1_100);
        assert!(
            !fractional.is_listing_active(accounts.alice, TOKEN_ID),
            "A listing at its deadline is no longer active"
        );

        test::set_caller::<DefaultEnvironment>(accounts.bob);
        test::set_value_transferred::<DefaultEnvironment>(PRICE_PER_SHARE * 150);
        assert_eq!(
            fractional.buy_shares(accounts.alice, TOKEN_ID, 150),
            Err(FractionalError::ListingExpired),
            "An expired listing must not be fillable at the stale price"
        );

        assert_eq!(
            fractional.balance_of(accounts.alice, TOKEN_ID),
            150,
            "The seller's shares must be untouched by the failed purchase"
        );
        assert_eq!(
            fractional.balance_of(accounts.bob, TOKEN_ID),
            0,
            "The buyer must receive nothing from a stale listing"
        );
        assert!(
            fractional.get_listing(accounts.alice, TOKEN_ID).is_none(),
            "The stale listing must be swept so the shares are free again"
        );
    }

    /// Renewal resets the clock: a listing that had expired becomes buyable
    /// again once the seller renews it with a future deadline.
    #[ink::test]
    fn test_renewal_resets_the_clock() {
        let accounts = test::default_accounts::<DefaultEnvironment>();
        let mut fractional = setup();

        mint(&mut fractional, accounts.alice, 120);
        test::set_caller::<DefaultEnvironment>(accounts.alice);
        test::set_block_timestamp::<DefaultEnvironment>(500);
        fractional
            .list_shares_for_sale(TOKEN_ID, 120, PRICE_PER_SHARE, Some(600))
            .expect("Listing should succeed");

        // Let it lapse, then confirm it is really dead before renewing.
        test::set_block_timestamp::<DefaultEnvironment>(700);
        test::set_caller::<DefaultEnvironment>(accounts.bob);
        test::set_value_transferred::<DefaultEnvironment>(PRICE_PER_SHARE * 120);
        assert_eq!(
            fractional.buy_shares(accounts.alice, TOKEN_ID, 120),
            Err(FractionalError::ListingExpired),
            "The listing must be dead before it can be renewed"
        );

        // A lapsed listing is swept on contact, so re-list to exercise renewal
        // on a live listing: renew, then let the new deadline pass, then renew
        // again and confirm the purchase settles.
        test::set_caller::<DefaultEnvironment>(accounts.alice);
        fractional
            .list_shares_for_sale(TOKEN_ID, 120, PRICE_PER_SHARE, Some(800))
            .expect("Re-listing should succeed");
        test::set_block_timestamp::<DefaultEnvironment>(800);
        fractional
            .renew_listing(TOKEN_ID, Some(1_500))
            .expect("Renewal should reset the deadline");

        let renewed = fractional
            .get_listing(accounts.alice, TOKEN_ID)
            .expect("Renewed listing should still exist");
        assert_eq!(renewed.expires_at, Some(1_500));
        assert_eq!(
            renewed.listed_at, 800,
            "Renewal must restamp the listing clock"
        );
        assert_eq!(renewed.shares, 120, "Renewal must not touch the quantity");
        assert_eq!(
            renewed.price_per_share, PRICE_PER_SHARE,
            "Renewal must not touch the price"
        );

        test::set_caller::<DefaultEnvironment>(accounts.bob);
        test::set_value_transferred::<DefaultEnvironment>(PRICE_PER_SHARE * 120);
        fractional
            .buy_shares(accounts.alice, TOKEN_ID, 120)
            .expect("A renewed listing is purchasable again");
        assert_eq!(fractional.balance_of(accounts.bob, TOKEN_ID), 120);
    }

    /// Renewal is a seller-only action on an existing listing.
    #[ink::test]
    fn test_renew_listing_guards() {
        let accounts = test::default_accounts::<DefaultEnvironment>();
        let mut fractional = setup();

        // No listing yet -> ListingNotFound
        test::set_caller::<DefaultEnvironment>(accounts.alice);
        assert_eq!(
            fractional.renew_listing(TOKEN_ID, Some(10_000)),
            Err(FractionalError::ListingNotFound),
            "Renewing a missing listing must fail"
        );

        // A listing belonging to alice is invisible to bob's renew call.
        mint(&mut fractional, accounts.alice, 60);
        fractional
            .list_shares_for_sale(TOKEN_ID, 60, PRICE_PER_SHARE, Some(10_000))
            .expect("Listing should succeed");

        test::set_caller::<DefaultEnvironment>(accounts.bob);
        assert_eq!(
            fractional.renew_listing(TOKEN_ID, Some(20_000)),
            Err(FractionalError::ListingNotFound),
            "A third party must not be able to renew someone else's listing"
        );
        assert_eq!(
            fractional
                .get_listing(accounts.alice, TOKEN_ID)
                .expect("Alice's listing should be untouched")
                .expires_at,
            Some(10_000),
            "A rejected renewal must not modify the deadline"
        );
    }

    /// Cancelling still works for a listing that has not expired, and the
    /// deadline-free form is cancelled the same way.
    #[ink::test]
    fn test_cancel_listing_unchanged_by_expiry_work() {
        let accounts = test::default_accounts::<DefaultEnvironment>();
        let mut fractional = setup();

        mint(&mut fractional, accounts.alice, 90);
        test::set_caller::<DefaultEnvironment>(accounts.alice);
        fractional
            .list_shares_for_sale(TOKEN_ID, 90, PRICE_PER_SHARE, Some(9_000))
            .expect("Listing should succeed");
        fractional
            .cancel_listing(TOKEN_ID)
            .expect("A live listing can still be cancelled");
        assert!(fractional.get_listing(accounts.alice, TOKEN_ID).is_none());
        assert!(
            !fractional.is_listing_active(accounts.alice, TOKEN_ID),
            "A cancelled listing is never active"
        );
    }
}
