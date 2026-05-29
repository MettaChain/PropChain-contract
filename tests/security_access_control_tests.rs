//! Security Test Suite — Access Control & Authorization
//!
//! Tests that sensitive contract operations enforce proper role-based access
//! control and cannot be called by unauthorized parties.
//!
//! # Coverage
//! - Non-admin cannot execute admin-only functions
//! - Token owner constraints on transfer and approval
//! - Bridge operator privilege enforcement
//! - Compliance verifier role enforcement

#![cfg(test)]

use ink::env::{test, DefaultEnvironment};
use propchain_traits::PropertyMetadata;
use property_token::property_token::{Error, PropertyToken};

// ─── Helper ────────────────────────────────────────────────────────────────

fn default_metadata() -> PropertyMetadata {
    PropertyMetadata {
        location: String::from("1 Security Lane"),
        size: 1500,
        legal_description: String::from("Security test property"),
        valuation: 300_000,
        documents_url: String::from("ipfs://security-docs"),
    }
}

fn setup() -> (
    PropertyToken,
    ink::primitives::AccountId,
    ink::primitives::AccountId,
) {
    let accounts = test::default_accounts::<DefaultEnvironment>();
    test::set_caller::<DefaultEnvironment>(accounts.alice); // alice = admin
    let contract = PropertyToken::new();
    (contract, accounts.alice, accounts.bob)
}

// ─── AC-01: Unauthorized bridge operator addition ───────────────────────────

/// SECURITY: A regular user (non-admin) must NOT be able to add a bridge operator.
#[ink::test]
fn sec_ac01_non_admin_cannot_add_bridge_operator() {
    let (mut contract, _alice, bob) = setup();
    let accounts = test::default_accounts::<DefaultEnvironment>();

    // Switch caller to charlie (not admin)
    test::set_caller::<DefaultEnvironment>(accounts.charlie);
    let result = contract.add_bridge_operator(bob);

    assert_eq!(
        result,
        Err(Error::Unauthorized),
        "SECURITY FINDING [CRITICAL]: Non-admin was able to add bridge operator"
    );
}

// ─── AC-02: Unauthorized compliance verification ────────────────────────────

/// SECURITY: Only the admin should be able to verify compliance on a token.
#[ink::test]
fn sec_ac02_non_admin_cannot_verify_compliance() {
    let (mut contract, alice, _bob) = setup();
    let accounts = test::default_accounts::<DefaultEnvironment>();

    // Mint a token as alice (admin)
    test::set_caller::<DefaultEnvironment>(alice);
    let token_id = contract
        .register_property_with_token(default_metadata())
        .expect("Minting should succeed for admin");

    // Try to verify compliance as charlie (non-admin)
    test::set_caller::<DefaultEnvironment>(accounts.charlie);
    let result = contract.verify_compliance(token_id, true);

    assert_eq!(
        result,
        Err(Error::Unauthorized),
        "SECURITY FINDING [HIGH]: Non-admin was able to verify token compliance"
    );
}

// ─── AC-03: Unauthorized transfer (not owner, not approved) ─────────────────

/// SECURITY: A third party with no approval must NOT be able to transfer a token.
#[ink::test]
fn sec_ac03_unapproved_caller_cannot_transfer_token() {
    let (mut contract, alice, _bob) = setup();
    let accounts = test::default_accounts::<DefaultEnvironment>();

    test::set_caller::<DefaultEnvironment>(alice);
    let token_id = contract
        .register_property_with_token(default_metadata())
        .expect("Minting should succeed");

    // charlie has no approval — must be rejected
    test::set_caller::<DefaultEnvironment>(accounts.charlie);
    let result = contract.transfer_from(alice, accounts.django, token_id);

    assert_eq!(
        result,
        Err(Error::Unauthorized),
        "SECURITY FINDING [CRITICAL]: Unapproved caller was able to transfer a token"
    );
}

// ─── AC-04: Transfer by incorrect 'from' address ────────────────────────────

/// SECURITY: Even the token owner cannot call transfer_from with a wrong 'from'.
#[ink::test]
fn sec_ac04_transfer_with_wrong_from_fails() {
    let (mut contract, alice, bob) = setup();
    let accounts = test::default_accounts::<DefaultEnvironment>();

    test::set_caller::<DefaultEnvironment>(alice);
    let token_id = contract
        .register_property_with_token(default_metadata())
        .expect("Minting should succeed");

    // Claim the token was owned by bob (false), while alice actually owns it
    test::set_caller::<DefaultEnvironment>(alice);
    let result = contract.transfer_from(bob, accounts.charlie, token_id);

    assert_eq!(
        result,
        Err(Error::Unauthorized),
        "SECURITY FINDING [CRITICAL]: transfer_from accepted an incorrect 'from' address"
    );
}

// ─── AC-05: Approval by non-owner ───────────────────────────────────────────

/// SECURITY: Only the token owner or an approved-for-all operator can call approve().
#[ink::test]
fn sec_ac05_non_owner_cannot_approve_token() {
    let (mut contract, alice, bob) = setup();
    let accounts = test::default_accounts::<DefaultEnvironment>();

    test::set_caller::<DefaultEnvironment>(alice);
    let token_id = contract
        .register_property_with_token(default_metadata())
        .expect("Minting should succeed");

    // Switch to charlie who doesn't own the token
    test::set_caller::<DefaultEnvironment>(accounts.charlie);
    let result = contract.approve(bob, token_id);

    assert_eq!(
        result,
        Err(Error::Unauthorized),
        "SECURITY FINDING [HIGH]: Non-owner was able to approve a token transfer"
    );
}

// ─── AC-06: Approved delegation is scoped ───────────────────────────────────

/// SECURITY: An account approved for *one* token cannot transfer a *different* token.
#[ink::test]
fn sec_ac06_single_token_approval_cannot_transfer_other_tokens() {
    let (mut contract, alice, bob) = setup();
    let accounts = test::default_accounts::<DefaultEnvironment>();

    test::set_caller::<DefaultEnvironment>(alice);
    let token_id_1 = contract
        .register_property_with_token(default_metadata())
        .expect("Minting token 1 should succeed");
    let token_id_2 = contract
        .register_property_with_token(default_metadata())
        .expect("Minting token 2 should succeed");

    // Approve bob for token 1 only
    contract
        .approve(bob, token_id_1)
        .expect("Approval should succeed");

    // Bob tries to transfer token 2 — must fail
    test::set_caller::<DefaultEnvironment>(bob);
    let result = contract.transfer_from(alice, accounts.charlie, token_id_2);

    assert_eq!(
        result,
        Err(Error::Unauthorized),
        "SECURITY FINDING [HIGH]: Single-token approval granted access to a different token"
    );
}

// ─── AC-07: Operator approval is correctly scoped to one owner ──────────────

/// SECURITY: `set_approval_for_all` must only apply to the caller's own tokens.
#[ink::test]
fn sec_ac07_operator_approval_scoped_to_owner() {
    let (mut contract, alice, bob) = setup();
    let accounts = test::default_accounts::<DefaultEnvironment>();

    // Alice mints token
    test::set_caller::<DefaultEnvironment>(alice);
    let token_id = contract
        .register_property_with_token(default_metadata())
        .expect("Minting should succeed");

    // Bob approves charlie as his operator — should NOT grant access to alice's tokens
    test::set_caller::<DefaultEnvironment>(bob);
    contract
        .set_approval_for_all(accounts.charlie, true)
        .expect("Setting approval should succeed");

    // Charlie tries to transfer alice's token — must fail
    test::set_caller::<DefaultEnvironment>(accounts.charlie);
    let result = contract.transfer_from(alice, accounts.django, token_id);

    assert_eq!(
        result,
        Err(Error::Unauthorized),
        "SECURITY FINDING [CRITICAL]: Operator approval from Bob gave access to Alice's token"
    );
}

// ─── AC-08: Operations on non-existent tokens ───────────────────────────────

/// SECURITY: All operations on non-existent token IDs must return TokenNotFound.
#[ink::test]
fn sec_ac08_operations_on_nonexistent_token_return_not_found() {
    let (mut contract, alice, bob) = setup();
    let ghost_token_id: u64 = 999_999;

    test::set_caller::<DefaultEnvironment>(alice);

    assert_eq!(
        contract.transfer_from(alice, bob, ghost_token_id),
        Err(Error::TokenNotFound),
        "transfer_from on ghost token should return TokenNotFound"
    );
    assert_eq!(
        contract.approve(bob, ghost_token_id),
        Err(Error::TokenNotFound),
        "approve on ghost token should return TokenNotFound"
    );
    assert_eq!(
        contract.verify_compliance(ghost_token_id, true),
        Err(Error::TokenNotFound),
        "verify_compliance on ghost token should return TokenNotFound"
    );
    assert_eq!(
        contract.attach_legal_document(
            ghost_token_id,
            ink::primitives::Hash::from([0u8; 32]),
            String::from("Deed")
        ),
        Err(Error::TokenNotFound),
        "attach_legal_document on ghost token should return TokenNotFound"
    );
}
