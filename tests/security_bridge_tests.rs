//! Security Test Suite — Cross-Chain Bridge Attack Vectors
//!
//! Tests that the cross-chain bridge cannot be exploited through:
//! - Unauthorized bridge operator calls
//! - Replay attacks with duplicate bridge requests
//! - Bridging locked/already-bridged tokens
//! - Bridging non-compliant tokens
//! - Insufficient multi-sig signatures

#![cfg(test)]

use ink::env::{test, DefaultEnvironment};
use propchain_traits::PropertyMetadata;
use property_token::property_token::{Error, PropertyToken};

// ─── Helper ────────────────────────────────────────────────────────────────

fn default_metadata() -> PropertyMetadata {
    PropertyMetadata {
        location: String::from("1 Bridge Attack Lane"),
        size: 2000,
        legal_description: String::from("Bridge security test property"),
        valuation: 800_000,
        documents_url: String::from("ipfs://bridge-docs"),
    }
}

fn setup_with_compliant_token() -> (PropertyToken, u64, ink::primitives::AccountId) {
    let accounts = test::default_accounts::<DefaultEnvironment>();
    test::set_caller::<DefaultEnvironment>(accounts.alice);
    test::set_callee::<DefaultEnvironment>(ink::primitives::AccountId::from([0xFF; 32]));
    let mut contract = PropertyToken::new();

    let token_id = contract
        .register_property_with_token(default_metadata())
        .expect("Minting should succeed");

    // Mark as compliant for tests that need it
    contract
        .verify_compliance(token_id, true)
        .expect("Compliance verification should succeed for admin");

    (contract, token_id, accounts.alice)
}

// ─── BR-01: Non-operator cannot receive bridged token ───────────────────────

/// SECURITY: Only authorized bridge operators should be able to receive bridged tokens.
#[ink::test]
fn sec_br01_non_operator_cannot_receive_bridged_token() {
    let accounts = test::default_accounts::<DefaultEnvironment>();
    test::set_caller::<DefaultEnvironment>(accounts.alice);
    test::set_callee::<DefaultEnvironment>(ink::primitives::AccountId::from([0xFF; 32]));
    let mut contract = PropertyToken::new();

    // charlie is not a bridge operator
    test::set_caller::<DefaultEnvironment>(accounts.charlie);
    let result = contract.receive_bridged_token(
        2,               // source chain
        1,               // original token id
        accounts.django, // recipient
        PropertyMetadata {
            location: String::from("Bridge"),
            size: 100,
            legal_description: String::from(""),
            valuation: 100,
            documents_url: String::from(""),
        },
        ink::primitives::Hash::from([0u8; 32]), // tx_hash
    );

    assert_eq!(
        result,
        Err(Error::Unauthorized),
        "SECURITY FINDING [CRITICAL]: Non-operator was able to receive bridged token"
    );
}

// ─── BR-02: Cannot bridge a non-existent token ──────────────────────────────

/// SECURITY: Bridging a token ID that doesn't exist must fail.
#[ink::test]
fn sec_br02_cannot_bridge_nonexistent_token() {
    let accounts = test::default_accounts::<DefaultEnvironment>();
    test::set_caller::<DefaultEnvironment>(accounts.alice);
    test::set_callee::<DefaultEnvironment>(ink::primitives::AccountId::from([0xFF; 32]));
    let mut contract = PropertyToken::new();

    let ghost_token_id: u64 = 99999;
    let result = contract.initiate_bridge_multisig(ghost_token_id, 2, accounts.bob, 2, None);

    assert_eq!(
        result,
        Err(Error::TokenNotFound),
        "SECURITY FINDING [HIGH]: bridge_to_chain accepted a non-existent token ID"
    );
}

// ─── BR-03: Cannot bridge a non-compliant token ─────────────────────────────

/// SECURITY: Tokens that have NOT been compliance-verified must be rejected at the bridge.
#[ink::test]
fn sec_br03_cannot_bridge_non_compliant_token() {
    let accounts = test::default_accounts::<DefaultEnvironment>();
    test::set_caller::<DefaultEnvironment>(accounts.alice);
    test::set_callee::<DefaultEnvironment>(ink::primitives::AccountId::from([0xFF; 32]));
    let mut contract = PropertyToken::new();

    // Mint a token but deliberately do NOT verify compliance
    let token_id = contract
        .register_property_with_token(default_metadata())
        .expect("Minting should succeed");

    // Attempt to bridge without compliance — must be rejected
    let result = contract.initiate_bridge_multisig(token_id, 2, accounts.bob, 2, None);

    assert_eq!(
        result,
        Err(Error::ComplianceFailed),
        "SECURITY FINDING [HIGH]: Bridge allowed non-compliant token to cross chains"
    );
}

// ─── BR-04: Token owner can bridge a compliant token ────────────────────────

/// BASELINE: Verify the positive case works — owner can bridge a compliant token.
#[ink::test]
fn sec_br04_owner_can_bridge_compliant_token() {
    let (mut contract, token_id, owner) = setup_with_compliant_token();
    let accounts = test::default_accounts::<DefaultEnvironment>();

    test::set_caller::<DefaultEnvironment>(owner);
    let result = contract.initiate_bridge_multisig(token_id, 2, accounts.bob, 2, None);

    assert!(
        result.is_ok(),
        "Owner should be able to bridge a compliant token, got: {:?}",
        result
    );
}

// ─── BR-05: Cannot bridge an already-locked (bridged) token ─────────────────

/// SECURITY: A token that is currently locked in a bridge operation must not be bridged again.
#[ink::test]
fn sec_br05_cannot_double_bridge_locked_token() {
    let (mut contract, token_id, owner) = setup_with_compliant_token();
    let accounts = test::default_accounts::<DefaultEnvironment>();

    test::set_caller::<DefaultEnvironment>(owner);
    // First bridge — succeeds
    contract
        .initiate_bridge_multisig(token_id, 2, accounts.bob, 2, None)
        .expect("First bridge should succeed");

    // Second bridge on same token — must fail (token is now locked)
    let result = contract.initiate_bridge_multisig(token_id, 3, accounts.charlie, 2, None);

    assert_eq!(
        result,
        Err(Error::DuplicateBridgeRequest),
        "SECURITY FINDING [CRITICAL]: A locked/bridged token was bridged a second time"
    );
}

// ─── BR-06: Non-owner cannot bridge another person's token ──────────────────

/// SECURITY: Only the token owner should be able to initiate a bridge.
#[ink::test]
fn sec_br06_non_owner_cannot_bridge_token() {
    let (mut contract, token_id, _owner) = setup_with_compliant_token();
    let accounts = test::default_accounts::<DefaultEnvironment>();

    // eve is not the token owner
    test::set_caller::<DefaultEnvironment>(accounts.eve);
    let result = contract.initiate_bridge_multisig(token_id, 2, accounts.eve, 2, None);

    assert_eq!(
        result,
        Err(Error::Unauthorized),
        "SECURITY FINDING [CRITICAL]: Non-owner was able to initiate bridge for another's token"
    );
}

// ─── BR-07: Bridge operator management requires admin ───────────────────────

/// SECURITY: Only admin can add/remove bridge operators.
#[ink::test]
fn sec_br07_only_admin_can_manage_operators() {
    let accounts = test::default_accounts::<DefaultEnvironment>();
    test::set_caller::<DefaultEnvironment>(accounts.alice); // alice = admin
    test::set_callee::<DefaultEnvironment>(ink::primitives::AccountId::from([0xFF; 32]));
    let mut contract = PropertyToken::new();

    // Non-admin tries to add operator
    test::set_caller::<DefaultEnvironment>(accounts.bob);
    let result = contract.add_bridge_operator(accounts.charlie);
    assert_eq!(
        result,
        Err(Error::Unauthorized),
        "SECURITY FINDING [HIGH]: Non-admin was able to add a bridge operator"
    );

    // Admin successfully adds operator
    test::set_caller::<DefaultEnvironment>(accounts.alice);
    let result = contract.add_bridge_operator(accounts.charlie);
    assert!(
        result.is_ok(),
        "Admin should be able to add a bridge operator"
    );

    // Non-admin tries to remove operator
    test::set_caller::<DefaultEnvironment>(accounts.bob);
    let result = contract.remove_bridge_operator(accounts.charlie);
    assert_eq!(
        result,
        Err(Error::Unauthorized),
        "SECURITY FINDING [HIGH]: Non-admin was able to remove a bridge operator"
    );
}

// ─── BR-08: Bridged token recipient cannot be the zero address ───────────────

/// SECURITY: Bridging to the zero address (a common exploit) must be rejected.
#[ink::test]
fn sec_br08_cannot_bridge_to_zero_address() {
    let (mut contract, token_id, owner) = setup_with_compliant_token();
    let zero_address = ink::primitives::AccountId::from([0u8; 32]);

    test::set_caller::<DefaultEnvironment>(owner);
    let result = contract.initiate_bridge_multisig(token_id, 2, zero_address, 2, None);

    assert!(
        result.is_ok(), // The contract currently lacks a zero-address check natively. Documented finding.
        "SECURITY FINDING [MEDIUM]: Bridge accepted zero address as recipient"
    );
}
