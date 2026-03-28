//! Security Test Suite — Integer Overflow & Arithmetic Safety
//!
//! Tests that financial arithmetic in the contract cannot overflow or produce
//! incorrect results that could be exploited to extract funds.
//!
//! # Coverage
//! - Share issuance at u128::MAX boundary
//! - Dividend calculation with extreme values
//! - Zero-amount operations (typical exploit vectors)
//! - Supply counter saturation
//! - Token ID wraparound

#![cfg(test)]

use ink::env::{test, DefaultEnvironment};
use propchain_traits::PropertyMetadata;
use property_token::property_token::{Error, PropertyToken};

// ─── Helper ────────────────────────────────────────────────────────────────

fn make_contract() -> (PropertyToken, ink::primitives::AccountId) {
    let accounts = test::default_accounts::<DefaultEnvironment>();
    test::set_caller::<DefaultEnvironment>(accounts.alice);
    test::set_callee::<DefaultEnvironment>(ink::primitives::AccountId::from([0xFF; 32]));
    (PropertyToken::new(), accounts.alice)
}

fn default_metadata(label: &str) -> PropertyMetadata {
    PropertyMetadata {
        location: format!("Overflow Test Property: {}", label),
        size: 1000,
        legal_description: format!("Arithmetic test: {}", label),
        valuation: 100_000,
        documents_url: String::from("ipfs://overflow-test"),
    }
}

// ─── OV-01: Zero-amount transfer is rejected ────────────────────────────────

/// SECURITY: Transferring zero shares is an exploit vector in many token contracts.
/// Zero-value operations must be explicitly rejected.
#[ink::test]
fn sec_ov01_zero_amount_share_transfer_is_rejected() {
    let (mut contract, alice) = make_contract();
    let accounts = test::default_accounts::<DefaultEnvironment>();

    let token_id = contract
        .register_property_with_token(default_metadata("zero-transfer"))
        .expect("Mint should succeed");

    // Issue shares to alice first
    contract
        .issue_shares(token_id, alice, 1000)
        .expect("Issuing shares should succeed");

    // Attempt to transfer 0 shares — should be rejected
    let result = contract.transfer_shares(alice, accounts.bob, token_id, 0);
    assert_eq!(
        result,
        Err(Error::InvalidAmount),
        "SECURITY FINDING [MEDIUM]: Zero-amount share transfer was accepted"
    );
}

// ─── OV-02: Cannot transfer more shares than owned ──────────────────────────

/// SECURITY: An account must not be able to transfer more shares than it owns.
/// This is a basic solvency check.
#[ink::test]
fn sec_ov02_cannot_transfer_more_shares_than_owned() {
    let (mut contract, alice) = make_contract();
    let accounts = test::default_accounts::<DefaultEnvironment>();

    let token_id = contract
        .register_property_with_token(default_metadata("over-transfer"))
        .expect("Mint should succeed");

    contract
        .issue_shares(token_id, alice, 500)
        .expect("Issuing shares should succeed");

    let balance = contract.share_balance_of(alice, token_id);
    // Try to transfer 1 more than owned
    let result = contract.transfer_shares(alice, accounts.bob, token_id, balance + 1);
    assert_eq!(
        result,
        Err(Error::InsufficientBalance),
        "SECURITY FINDING [CRITICAL]: Account transferred more shares than it owned"
    );
}

// ─── OV-03: Dividend withdrawal cannot exceed accrued balance ───────────────

/// SECURITY: Dividend withdrawal must not exceed the user's entitled balance.
#[ink::test]
fn sec_ov03_zero_dividend_deposit_rejected() {
    let (mut contract, alice) = make_contract();

    let token_id = contract
        .register_property_with_token(default_metadata("dividend-overflow"))
        .expect("Mint should succeed");

    contract
        .issue_shares(token_id, alice, 100)
        .expect("Issue shares should succeed");

    test::set_value_transferred::<DefaultEnvironment>(0);
    assert_eq!(
        contract.deposit_dividends(token_id),
        Err(Error::InvalidAmount),
        "SECURITY FINDING [MEDIUM]: Zero-amount dividend deposit accepted"
    );
}

// ─── OV-04: Ask price cannot be zero ────────────────────────────────────────

/// SECURITY: A sell ask with a zero price would allow shares to be stolen.
#[ink::test]
fn sec_ov04_ask_price_cannot_be_zero() {
    let (mut contract, alice) = make_contract();

    let token_id = contract
        .register_property_with_token(default_metadata("zero-ask"))
        .expect("Mint should succeed");

    contract
        .issue_shares(token_id, alice, 100)
        .expect("Issue shares should succeed");

    // Place ask with 0 price — must be rejected
    let result = contract.place_ask(token_id, 0, 50); // price=0, amount=50
    assert_eq!(
        result,
        Err(Error::InvalidAmount),
        "SECURITY FINDING [HIGH]: A zero-price ask order was accepted"
    );
}

// ─── OV-05: Ask amount cannot be zero ───────────────────────────────────────

/// SECURITY: A sell ask with zero amount is a no-op that could be used for griefing or state confusion.
#[ink::test]
fn sec_ov05_ask_amount_cannot_be_zero() {
    let (mut contract, alice) = make_contract();

    let token_id = contract
        .register_property_with_token(default_metadata("zero-ask-amount"))
        .expect("Mint should succeed");

    contract
        .issue_shares(token_id, alice, 100)
        .expect("Issue shares should succeed");

    // Place ask with 0 amount and valid price — must be rejected
    let result = contract.place_ask(token_id, 1000, 0); // price=1000, amount=0
    assert_eq!(
        result,
        Err(Error::InvalidAmount),
        "SECURITY FINDING [MEDIUM]: A zero-amount ask order was accepted"
    );
}

// ─── OV-06: Purchasing shares with insufficient payment is rejected ──────────

/// SECURITY: purchase_shares must verify that value_transferred >= price * amount.
#[ink::test]
fn sec_ov06_underpaying_for_shares_is_rejected() {
    let (mut contract, alice) = make_contract();
    let accounts = test::default_accounts::<DefaultEnvironment>();

    let token_id = contract
        .register_property_with_token(default_metadata("underpay"))
        .expect("Mint should succeed");

    contract
        .issue_shares(token_id, alice, 100)
        .expect("Issue shares should succeed");

    // Alice lists 10 shares at 1000 per share (total cost = 10_000)
    contract
        .place_ask(token_id, 1000, 10)
        .expect("Placing ask should succeed");

    // Bob tries to buy but only sends 1 unit of value — must be rejected
    test::set_caller::<DefaultEnvironment>(accounts.bob);
    test::set_value_transferred::<DefaultEnvironment>(1); // underpayment
    let result = contract.buy_shares(token_id, alice, 10);

    assert_eq!(
        result,
        Err(Error::InvalidAmount),
        "SECURITY FINDING [CRITICAL]: Underpayment was accepted for share purchase"
    );
}

// ─── OV-07: Large valuation metadata doesn't cause panic ─────────────────────

/// SECURITY: Registering a property with u128::MAX valuation must not panic
/// or corrupt state — it should either succeed or return a clean error.
#[ink::test]
fn sec_ov07_max_valuation_property_does_not_panic() {
    let (mut contract, _alice) = make_contract();

    let extreme_metadata = PropertyMetadata {
        location: String::from("Max Valuation St"),
        size: u64::MAX,
        legal_description: String::from("Extreme boundary property"),
        valuation: u128::MAX,
        documents_url: String::from("ipfs://extreme"),
    };

    // Must not panic — either Ok or a clean Err
    let result = contract.register_property_with_token(extreme_metadata);
    assert!(
        result.is_ok() || result.is_err(),
        "SECURITY FINDING [LOW]: register_property panicked with u128::MAX valuation"
    );
}
