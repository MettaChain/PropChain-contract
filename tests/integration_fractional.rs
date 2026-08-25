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
            .list_shares_for_sale(TOKEN_ID, 400, PRICE_PER_SHARE)
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
            .list_shares_for_sale(TOKEN_ID, 300, PRICE_PER_SHARE)
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
            .list_shares_for_sale(TOKEN_ID, 100, PRICE_PER_SHARE)
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
            .list_shares_for_sale(TOKEN_ID, 50, PRICE_PER_SHARE)
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
            .list_shares_for_sale(TOKEN_ID, 150, PRICE_PER_SHARE)
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
            fractional.list_shares_for_sale(TOKEN_ID, 51, PRICE_PER_SHARE),
            Err(FractionalError::InsufficientShares),
            "Listing more than held must be rejected"
        );

        // Zero-share listing rejected
        assert_eq!(
            fractional.list_shares_for_sale(TOKEN_ID, 0, PRICE_PER_SHARE),
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
}
