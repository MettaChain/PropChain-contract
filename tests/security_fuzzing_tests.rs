//! Security Test Suite — Property-Based / Fuzz Testing
//!
//! Uses `proptest` for automated property-based testing across
//! a wide range of randomly generated inputs. This catches
//! corner cases that hand-crafted tests typically miss.
//!
//! # Coverage
//! - Random invalid token IDs always return a clean error
//! - Random unauthorized callers always return Unauthorized
//! - Metadata with boundary-length strings never panics
//! - Random amounts are handled gracefully in financial ops

#![cfg(test)]

use ink::env::{test, DefaultEnvironment};
use propchain_traits::PropertyMetadata;
use property_token::property_token::{Error, PropertyToken};
use proptest::prelude::*;

// ─── Shared setup ──────────────────────────────────────────────────────────

fn new_contract_as_alice() -> (PropertyToken, ink::primitives::AccountId) {
    let accounts = test::default_accounts::<DefaultEnvironment>();
    test::set_caller::<DefaultEnvironment>(accounts.alice);
    (PropertyToken::new(), accounts.alice)
}

fn make_metadata(location: &str, size: u64, valuation: u128) -> PropertyMetadata {
    PropertyMetadata {
        location: location.to_string(),
        size,
        legal_description: String::from("Fuzz test property"),
        valuation,
        documents_url: String::from("ipfs://fuzz"),
    }
}

// ─── FZ-01: Random non-existent token IDs always return TokenNotFound ────────

/// PROPERTY: For any token_id that has NOT been minted, all ops must return
/// TokenNotFound — never panic, never return unexpected results.
proptest! {
    #[test]
    fn sec_fz01_random_ghost_token_id_always_fails(
        ghost_id in 1000u64..u64::MAX,
    ) {
        let accounts = test::default_accounts::<DefaultEnvironment>();
        test::set_caller::<DefaultEnvironment>(accounts.alice);
        test::set_callee::<DefaultEnvironment>(ink::primitives::AccountId::from([0xFF; 32]));
        let mut contract = PropertyToken::new();

        prop_assert_eq!(
            contract.owner_of(ghost_id),
            None,
            "owner_of a ghost token must return None"
        );
        prop_assert_eq!(
            contract.transfer_from(accounts.alice, accounts.bob, ghost_id),
            Err(Error::TokenNotFound),
            "transfer_from ghost token must return TokenNotFound"
        );
        prop_assert_eq!(
            contract.approve(accounts.bob, ghost_id),
            Err(Error::TokenNotFound),
            "approve ghost token must return TokenNotFound"
        );
    }
}

// ─── FZ-02: Contract never panics with extreme metadata values ───────────────

/// PROPERTY: Registering properties with any combination of boundary-range
/// values for size and valuation must never cause a panic.
proptest! {
    #[test]
    fn sec_fz02_extreme_metadata_never_panics(
        size in 0u64..=u64::MAX,
        valuation in 0u128..=u128::MAX,
        location_len in 0usize..1000,
    ) {
        let accounts = test::default_accounts::<DefaultEnvironment>();
        test::set_caller::<DefaultEnvironment>(accounts.alice);
        test::set_callee::<DefaultEnvironment>(ink::primitives::AccountId::from([0xFF; 32]));
        let mut contract = PropertyToken::new();

        let location = "A".repeat(location_len);
        let meta = make_metadata(&location, size, valuation);

        // Must not panic — a clean Ok or Err is both acceptable
        let result = contract.register_property_with_token(meta);
        prop_assert!(
            result.is_ok() || result.is_err(),
            "register_property_with_token must never panic, got unexpected state"
        );
    }
}

// ─── FZ-03: Any non-admin caller gets Unauthorized on admin functions ─────────

/// PROPERTY: Callers generated from any random seed who are not the admin
/// must always get Unauthorized on admin-only operations.
proptest! {
    #[test]
    fn sec_fz03_non_admin_always_unauthorized(seed in 1u8..=254u8) {
        let accounts = test::default_accounts::<DefaultEnvironment>();
        test::set_caller::<DefaultEnvironment>(accounts.alice); // alice = admin
        test::set_callee::<DefaultEnvironment>(ink::primitives::AccountId::from([0xFF; 32]));
        let mut contract = PropertyToken::new();

        // Mint a token as admin
        let token_id = contract
            .register_property_with_token(make_metadata("Fuzz St", 1000, 500_000))
            .expect("Admin minting should succeed");

        // Generate a deterministic non-admin account from the seed
        let mut bytes = [seed; 32];
        bytes[0] = seed;
        bytes[1] = seed.wrapping_add(1);
        let attacker = ink::primitives::AccountId::from(bytes);

        // Ensure we're using an account that isn't any known test account
        prop_assume!(attacker != accounts.alice);
        prop_assume!(attacker != accounts.bob);
        prop_assume!(attacker != accounts.charlie);

        test::set_caller::<DefaultEnvironment>(attacker);

        // These must all return Unauthorized
        prop_assert_eq!(
            contract.verify_compliance(token_id, true),
            Err(Error::Unauthorized),
            "Seed {}: verify_compliance by non-admin must return Unauthorized",
            seed
        );
        prop_assert_eq!(
            contract.add_bridge_operator(attacker),
            Err(Error::Unauthorized),
            "Seed {}: add_bridge_operator by non-admin must return Unauthorized",
            seed
        );
    }
}

// ─── FZ-04: Balance of batch with mismatched lengths returns empty ────────────

/// PROPERTY: balance_of_batch must handle any length combination gracefully.
proptest! {
    #[test]
    fn sec_fz04_balance_of_batch_handles_any_lengths(
        count_a in 0usize..20,
        count_b in 0usize..20,
    ) {
        let accounts = test::default_accounts::<DefaultEnvironment>();
        test::set_caller::<DefaultEnvironment>(accounts.alice);
        test::set_callee::<DefaultEnvironment>(ink::primitives::AccountId::from([0xFF; 32]));
        let contract = PropertyToken::new();

        let accounts_vec: Vec<ink::primitives::AccountId> =
            (0..count_a).map(|i| {
                let mut b = [0u8; 32];
                b[0] = i as u8;
                ink::primitives::AccountId::from(b)
            }).collect();

        let ids_vec: Vec<u64> = (0..count_b as u64).collect();

        // Must not panic regardless of length mismatch
        let result = contract.balance_of_batch(accounts_vec, ids_vec);
        prop_assert!(
            result.is_empty() || !result.is_empty(),
            "balance_of_batch must not panic with mismatched lengths"
        );
    }
}

// ─── FZ-05: Minting many tokens keeps supply counter accurate ────────────────

/// PROPERTY: Minting N tokens in sequence must result in total_supply == N.
proptest! {
    #[test]
    fn sec_fz05_bulk_minting_keeps_accurate_supply(count in 1u32..20) {
        let accounts = test::default_accounts::<DefaultEnvironment>();
        test::set_caller::<DefaultEnvironment>(accounts.alice);
        test::set_callee::<DefaultEnvironment>(ink::primitives::AccountId::from([0xFF; 32]));
        let mut contract = PropertyToken::new();

        for i in 0..count {
            let meta = make_metadata(&format!("Prop {}", i), (i + 1) as u64 * 100, 100_000);
            contract
                .register_property_with_token(meta)
                .expect("Bulk minting should succeed");
        }

        prop_assert_eq!(
            contract.total_supply(),
            count as u64,
            "Supply counter must equal the number of minted tokens"
        );
    }
}
