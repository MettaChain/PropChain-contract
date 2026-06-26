
#![cfg(all(test, feature = "e2e-tests"))]

use ink::primitives::AccountId;
use propchain_contract::propchain_contract::PropchainContract;
use propchain_contract::propchain_contract::PropchainContractRef;
use propchain_traits::{Balance, Error, Jurisdiction};

type DefaultEnvironment = ink::env::DefaultEnvironment;

const VALID_JURISDICTION: Jurisdiction = Jurisdiction::Usa;

fn make_contract() -> (PropchainContract, AccountId) {
    let accounts = ink::env::test::default_accounts::<DefaultEnvironment>();
    ink::env::test::set_caller::<DefaultEnvironment>(accounts.alice);

    let contract = PropchainContract::new(
        accounts.alice,
        accounts.bob,
        accounts.charlie,
        1_000_000,
        10,
    );

    (contract, accounts.alice)
}

#[ink::test]
fn sec_gov_escrow_creation_with_invalid_jurisdiction_is_rejected() {
    let (mut contract, alice) = make_contract();
    let accounts = ink::env::test::default_accounts::<DefaultEnvironment>();

    // 1. Define an invalid jurisdiction
    const INVALID_JURISDICTION: Jurisdiction = Jurisdiction::Other(99);

    // 2. Attempt to create an escrow with the invalid jurisdiction
    let result = contract.create_escrow_advanced(
        1,
        1_000_000,
        accounts.bob,
        accounts.charlie,
        vec![alice, accounts.bob, accounts.charlie],
        2,
        None,
        INVALID_JURISDICTION,
    );

    // 3. Verify that the operation is rejected with the correct error
    assert_eq!(result, Err(Error::JurisdictionNotAllowed));
}