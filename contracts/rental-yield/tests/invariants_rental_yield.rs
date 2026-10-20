#![allow(unexpected_cfgs)]
#![allow(clippy::clone_on_copy)]

//! Invariant tests for the RentalYield accounting model.
//!
//! Each invariant is asserted after a mixed sequence of deposits, share
//! changes and claims so that rounding, role changes and the pause switch
//! cannot push the contract into an inconsistent state.

use ink::env::test::{self, DefaultAccounts};
use ink::env::DefaultEnvironment;
use ink::primitives::AccountId;
use rental_yield::rental_yield::{Error, PropertyYield, RentalYield};

const PROPERTY: u64 = 7;

fn accounts() -> DefaultAccounts<DefaultEnvironment> {
    test::default_accounts::<DefaultEnvironment>()
}

fn setup(
    total_shares: u128,
    holdings: &[(AccountId, u128)],
) -> (RentalYield, DefaultAccounts<DefaultEnvironment>) {
    let accounts = accounts();
    test::set_caller::<DefaultEnvironment>(accounts.alice);
    let mut contract = RentalYield::new();
    contract
        .register_property(PROPERTY, total_shares)
        .expect("register property");
    for (holder, shares) in holdings {
        contract
            .set_shares(PROPERTY, *holder, *shares)
            .expect("assign shares");
    }
    (contract, accounts)
}

fn deposit(contract: &mut RentalYield, payer: AccountId, amount: u128) {
    test::set_caller::<DefaultEnvironment>(payer);
    test::set_value_transferred::<DefaultEnvironment>(amount);
    contract.deposit_rent(PROPERTY).expect("deposit rent");
    test::set_value_transferred::<DefaultEnvironment>(0);
}

fn total_claimable(contract: &RentalYield, holders: &[AccountId]) -> u128 {
    holders
        .iter()
        .map(|holder| contract.claimable(PROPERTY, *holder))
        .sum()
}

fn assert_accounting_invariants(
    contract: &RentalYield,
    property: &PropertyYield,
    holders: &[AccountId],
) {
    assert!(
        property.allocated_shares <= property.total_shares,
        "allocated shares must never exceed the property supply"
    );
    assert!(
        property.claimed <= property.deposited,
        "claims must never exceed deposits"
    );
    assert!(
        total_claimable(contract, holders) <= property.deposited - property.claimed,
        "outstanding claims must be backed by unclaimed deposits"
    );
    assert!(
        contract.total_claimed() <= contract.total_deposited(),
        "lifetime claims must never exceed lifetime deposits"
    );

    for holder in holders {
        assert!(
            contract.claimable(PROPERTY, *holder) <= property.deposited - property.claimed,
            "a single holder can never claim more than the remaining pool"
        );
    }
}

#[ink::test]
fn invariant_accounting_holds_after_mixed_flows() {
    let accounts = accounts();
    let holders = [accounts.alice, accounts.bob, accounts.charlie];
    let (mut contract, _) = setup(
        100,
        &[
            (accounts.alice, 50),
            (accounts.bob, 30),
            (accounts.charlie, 20),
        ],
    );
    let property = contract.get_property(PROPERTY).expect("property exists");
    assert_accounting_invariants(&contract, &property, &holders);

    deposit(&mut contract, accounts.alice, 1_000);
    let property = contract.get_property(PROPERTY).expect("property exists");
    assert_accounting_invariants(&contract, &property, &holders);

    contract
        .set_shares(PROPERTY, accounts.charlie, 10)
        .expect("decrease charlie shares");
    contract
        .set_shares(PROPERTY, accounts.bob, 40)
        .expect("increase bob shares");
    let property = contract.get_property(PROPERTY).expect("property exists");
    assert_accounting_invariants(&contract, &property, &holders);

    deposit(&mut contract, accounts.alice, 333);
    test::set_caller::<DefaultEnvironment>(accounts.bob);
    let claimed = contract.claim(PROPERTY).expect("bob claims");
    assert_eq!(claimed, 433);
    let property = contract.get_property(PROPERTY).expect("property exists");
    assert_accounting_invariants(&contract, &property, &holders);

    deposit(&mut contract, accounts.alice, 1);
    let property = contract.get_property(PROPERTY).expect("property exists");
    assert_accounting_invariants(&contract, &property, &holders);
}

#[ink::test]
fn invariant_reward_per_share_is_monotonic() {
    let accounts = accounts();
    let holders = [accounts.alice, accounts.bob];
    let (mut contract, _) = setup(10, &[(accounts.alice, 5), (accounts.bob, 5)]);

    let mut previous = contract
        .get_property(PROPERTY)
        .expect("property exists")
        .reward_per_share;

    for amount in [1u128, 7, 99, 1_000, 3] {
        deposit(&mut contract, accounts.alice, amount);
        let property = contract.get_property(PROPERTY).expect("property exists");
        assert!(
            property.reward_per_share >= previous,
            "the accumulator may only move forward"
        );
        assert_accounting_invariants(&contract, &property, &holders);
        previous = property.reward_per_share;
    }
}

#[ink::test]
fn invariant_failed_operations_do_not_mutate_state() {
    let accounts = accounts();
    let holders = [accounts.alice, accounts.bob];
    let (mut contract, _) = setup(10, &[(accounts.alice, 10)]);
    deposit(&mut contract, accounts.alice, 100);

    let before = contract.get_property(PROPERTY).expect("property exists");
    let before_deposited = contract.total_deposited();
    let before_claimed = contract.total_claimed();
    let before_claimable = contract.claimable(PROPERTY, accounts.alice);

    test::set_caller::<DefaultEnvironment>(accounts.bob);
    test::set_value_transferred::<DefaultEnvironment>(500);
    assert_eq!(contract.deposit_rent(PROPERTY), Err(Error::Unauthorized));
    test::set_value_transferred::<DefaultEnvironment>(0);
    assert_eq!(
        contract.set_shares(PROPERTY, accounts.bob, 1),
        Err(Error::Unauthorized)
    );
    assert_eq!(contract.claim(PROPERTY), Err(Error::NoYield));

    let after = contract.get_property(PROPERTY).expect("property exists");
    assert_eq!(before, after);
    assert_eq!(contract.total_deposited(), before_deposited);
    assert_eq!(contract.total_claimed(), before_claimed);
    assert_eq!(
        contract.claimable(PROPERTY, accounts.alice),
        before_claimable
    );
    assert_accounting_invariants(&contract, &after, &holders);
}

#[ink::test]
fn invariant_paused_blocks_all_value_movements() {
    let accounts = accounts();
    let holders = [accounts.alice];
    let (mut contract, _) = setup(10, &[(accounts.alice, 10)]);
    deposit(&mut contract, accounts.alice, 100);

    let before = contract.get_property(PROPERTY).expect("property exists");
    let before_claimable = contract.claimable(PROPERTY, accounts.alice);

    test::set_caller::<DefaultEnvironment>(accounts.alice);
    contract.pause().expect("pause");
    assert_eq!(contract.pause(), Err(Error::AlreadyPaused));

    test::set_value_transferred::<DefaultEnvironment>(100);
    assert_eq!(contract.deposit_rent(PROPERTY), Err(Error::ContractPaused));
    test::set_value_transferred::<DefaultEnvironment>(0);
    assert_eq!(contract.claim(PROPERTY), Err(Error::ContractPaused));

    let during = contract.get_property(PROPERTY).expect("property exists");
    assert_eq!(before, during);
    assert_eq!(
        contract.claimable(PROPERTY, accounts.alice),
        before_claimable
    );
    assert_accounting_invariants(&contract, &during, &holders);

    contract.unpause().expect("unpause");
    let claimed = contract.claim(PROPERTY).expect("claim after resume");
    assert_eq!(claimed, before_claimable);
    let after = contract.get_property(PROPERTY).expect("property exists");
    assert_accounting_invariants(&contract, &after, &holders);
}

#[ink::test]
fn invariant_share_assignments_cannot_exceed_supply() {
    let accounts = accounts();
    let holders = [accounts.alice, accounts.bob];
    let (mut contract, _) = setup(10, &[(accounts.alice, 6)]);

    assert_eq!(
        contract.set_shares(PROPERTY, accounts.bob, 5),
        Err(Error::InsufficientShares)
    );

    contract
        .set_shares(PROPERTY, accounts.bob, 4)
        .expect("exactly fill the supply");
    let property = contract.get_property(PROPERTY).expect("property exists");
    assert_eq!(property.allocated_shares, 10);
    assert_accounting_invariants(&contract, &property, &holders);

    contract
        .set_shares(PROPERTY, accounts.alice, 0)
        .expect("release shares");
    let property = contract.get_property(PROPERTY).expect("property exists");
    assert_eq!(property.allocated_shares, 4);
    assert_accounting_invariants(&contract, &property, &holders);
}

#[ink::test]
fn invariant_zero_value_deposit_is_rejected_without_state_change() {
    let accounts = accounts();
    let holders = [accounts.alice];
    let (mut contract, _) = setup(10, &[(accounts.alice, 10)]);

    let before = contract.get_property(PROPERTY).expect("property exists");
    test::set_caller::<DefaultEnvironment>(accounts.alice);
    test::set_value_transferred::<DefaultEnvironment>(0);
    assert_eq!(contract.deposit_rent(PROPERTY), Err(Error::InvalidAmount));

    let after = contract.get_property(PROPERTY).expect("property exists");
    assert_eq!(before, after);
    assert_eq!(contract.total_deposited(), 0);
    assert_accounting_invariants(&contract, &after, &holders);
}
