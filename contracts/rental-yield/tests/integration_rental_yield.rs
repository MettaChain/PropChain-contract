#![allow(unexpected_cfgs)]
#![allow(clippy::clone_on_copy)]

//! Integration tests for the end-to-end RentalYield flows.
//!
//! These exercise the same public messages an off-chain caller would use:
//! property registration, share assignment, rent deposits, yield claims,
//! role management and the pause switch.

use ink::env::test::{self, DefaultAccounts};
use ink::env::DefaultEnvironment;
use ink::primitives::AccountId;
use rental_yield::rental_yield::{Error, RentalYield};
use rental_yield::Role;

const PROPERTY: u64 = 1;

fn accounts() -> DefaultAccounts<DefaultEnvironment> {
    test::default_accounts::<DefaultEnvironment>()
}

fn setup() -> (RentalYield, DefaultAccounts<DefaultEnvironment>) {
    let accounts = accounts();
    test::set_caller::<DefaultEnvironment>(accounts.alice);
    (RentalYield::new(), accounts)
}

fn register(contract: &mut RentalYield, total_shares: u128) {
    let accounts = accounts();
    test::set_caller::<DefaultEnvironment>(accounts.alice);
    contract
        .register_property(PROPERTY, total_shares)
        .expect("admin can register a property");
}

fn set_shares(contract: &mut RentalYield, holder: AccountId, shares: u128) {
    let accounts = accounts();
    test::set_caller::<DefaultEnvironment>(accounts.alice);
    contract
        .set_shares(PROPERTY, holder, shares)
        .expect("admin can assign shares");
}

fn deposit(contract: &mut RentalYield, payer: AccountId, amount: u128) {
    test::set_caller::<DefaultEnvironment>(payer);
    test::set_value_transferred::<DefaultEnvironment>(amount);
    contract
        .deposit_rent(PROPERTY)
        .expect("operator can deposit rent");
    test::set_value_transferred::<DefaultEnvironment>(0);
}

#[ink::test]
fn integration_proportional_yield_flow() {
    let (mut contract, accounts) = setup();
    register(&mut contract, 100);
    set_shares(&mut contract, accounts.alice, 60);
    set_shares(&mut contract, accounts.bob, 40);

    deposit(&mut contract, accounts.alice, 1_000);

    assert_eq!(contract.claimable(PROPERTY, accounts.alice), 600);
    assert_eq!(contract.claimable(PROPERTY, accounts.bob), 400);
    assert_eq!(contract.total_deposited(), 1_000);

    test::set_caller::<DefaultEnvironment>(accounts.bob);
    let claimed = contract.claim(PROPERTY).expect("bob can claim");
    assert_eq!(claimed, 400);
    assert_eq!(contract.claimable(PROPERTY, accounts.bob), 0);
    assert_eq!(contract.total_claimed(), 400);

    test::set_caller::<DefaultEnvironment>(accounts.alice);
    let claimed = contract.claim(PROPERTY).expect("alice can claim");
    assert_eq!(claimed, 600);
    assert_eq!(contract.total_claimed(), 1_000);

    let property = contract.get_property(PROPERTY).expect("property exists");
    assert_eq!(property.claimed, 1_000);
}

#[ink::test]
fn integration_rounding_never_over_distributes() {
    let (mut contract, accounts) = setup();
    register(&mut contract, 3);
    set_shares(&mut contract, accounts.alice, 1);
    set_shares(&mut contract, accounts.bob, 1);
    set_shares(&mut contract, accounts.charlie, 1);

    deposit(&mut contract, accounts.alice, 100);

    let alice = contract.claimable(PROPERTY, accounts.alice);
    let bob = contract.claimable(PROPERTY, accounts.bob);
    let charlie = contract.claimable(PROPERTY, accounts.charlie);

    assert_eq!(alice, 33);
    assert_eq!(bob, 33);
    assert_eq!(charlie, 33);
    assert!(alice + bob + charlie <= contract.total_deposited());

    let property = contract.get_property(PROPERTY).expect("property exists");
    assert_eq!(property.remainder, 1);
}

#[ink::test]
fn integration_access_control_rejects_non_operators() {
    let (mut contract, accounts) = setup();
    register(&mut contract, 10);
    set_shares(&mut contract, accounts.alice, 10);

    test::set_caller::<DefaultEnvironment>(accounts.bob);
    assert_eq!(contract.register_property(2, 10), Err(Error::Unauthorized));
    assert_eq!(
        contract.set_shares(PROPERTY, accounts.bob, 1),
        Err(Error::Unauthorized)
    );
    test::set_value_transferred::<DefaultEnvironment>(100);
    assert_eq!(contract.deposit_rent(PROPERTY), Err(Error::Unauthorized));
    test::set_value_transferred::<DefaultEnvironment>(0);
    assert_eq!(contract.pause(), Err(Error::Unauthorized));
    assert!(!contract.is_paused());
}

#[ink::test]
fn integration_manager_role_can_operate_and_is_revocable() {
    let (mut contract, accounts) = setup();
    register(&mut contract, 10);

    test::set_caller::<DefaultEnvironment>(accounts.alice);
    contract
        .grant_role(accounts.bob, Role::Manager)
        .expect("admin can grant Manager");
    assert!(contract.has_role(accounts.bob, Role::Manager));

    test::set_caller::<DefaultEnvironment>(accounts.bob);
    contract
        .register_property(2, 10)
        .expect("manager can register a property");
    contract
        .set_shares(2, accounts.bob, 10)
        .expect("manager can assign shares");
    test::set_value_transferred::<DefaultEnvironment>(100);
    contract.deposit_rent(2).expect("manager can deposit rent");
    test::set_value_transferred::<DefaultEnvironment>(0);

    test::set_caller::<DefaultEnvironment>(accounts.alice);
    contract
        .revoke_role(accounts.bob, Role::Manager)
        .expect("admin can revoke Manager");
    assert!(!contract.has_role(accounts.bob, Role::Manager));

    test::set_caller::<DefaultEnvironment>(accounts.bob);
    assert_eq!(contract.register_property(3, 10), Err(Error::Unauthorized));
}

#[ink::test]
fn integration_pause_blocks_and_resume_restores_flows() {
    let (mut contract, accounts) = setup();
    register(&mut contract, 10);
    set_shares(&mut contract, accounts.alice, 10);
    deposit(&mut contract, accounts.alice, 100);

    test::set_caller::<DefaultEnvironment>(accounts.alice);
    contract.pause().expect("admin can pause");
    assert!(contract.is_paused());

    test::set_caller::<DefaultEnvironment>(accounts.alice);
    test::set_value_transferred::<DefaultEnvironment>(100);
    assert_eq!(contract.deposit_rent(PROPERTY), Err(Error::ContractPaused));
    test::set_value_transferred::<DefaultEnvironment>(0);
    assert_eq!(contract.claim(PROPERTY), Err(Error::ContractPaused));
    assert_eq!(contract.claimable(PROPERTY, accounts.alice), 100);

    contract.unpause().expect("admin can unpause");
    assert!(!contract.is_paused());
    let claimed = contract.claim(PROPERTY).expect("claim works after resume");
    assert_eq!(claimed, 100);
}

#[ink::test]
fn integration_share_change_does_not_earn_retroactively() {
    let (mut contract, accounts) = setup();
    register(&mut contract, 100);
    set_shares(&mut contract, accounts.alice, 50);

    deposit(&mut contract, accounts.alice, 100);
    assert_eq!(contract.claimable(PROPERTY, accounts.alice), 100);

    set_shares(&mut contract, accounts.alice, 100);
    assert_eq!(contract.claimable(PROPERTY, accounts.alice), 100);

    deposit(&mut contract, accounts.alice, 100);
    assert_eq!(contract.claimable(PROPERTY, accounts.alice), 200);

    test::set_caller::<DefaultEnvironment>(accounts.alice);
    assert_eq!(contract.claim(PROPERTY), Ok(200));
    assert_eq!(contract.total_claimed(), 200);
}

#[ink::test]
fn integration_claim_is_idempotent_after_balance_is_exhausted() {
    let (mut contract, accounts) = setup();
    register(&mut contract, 10);
    set_shares(&mut contract, accounts.alice, 10);
    deposit(&mut contract, accounts.alice, 50);

    test::set_caller::<DefaultEnvironment>(accounts.bob);
    assert_eq!(contract.claim(PROPERTY), Err(Error::NoYield));

    test::set_caller::<DefaultEnvironment>(accounts.alice);
    assert_eq!(contract.claim(PROPERTY), Ok(50));
    assert_eq!(contract.claim(PROPERTY), Err(Error::NoYield));
    assert_eq!(contract.total_claimed(), 50);
}

#[ink::test]
fn integration_properties_are_isolated() {
    let (mut contract, accounts) = setup();
    register(&mut contract, 10);
    set_shares(&mut contract, accounts.alice, 10);

    test::set_caller::<DefaultEnvironment>(accounts.alice);
    contract
        .register_property(2, 10)
        .expect("register second property");
    contract
        .set_shares(2, accounts.bob, 10)
        .expect("assign second property shares");

    test::set_caller::<DefaultEnvironment>(accounts.alice);
    test::set_value_transferred::<DefaultEnvironment>(100);
    contract.deposit_rent(1).expect("deposit property one");
    test::set_value_transferred::<DefaultEnvironment>(0);

    assert_eq!(contract.claimable(1, accounts.alice), 100);
    assert_eq!(contract.claimable(2, accounts.bob), 0);
    assert_eq!(contract.claimable(2, accounts.alice), 0);
}
