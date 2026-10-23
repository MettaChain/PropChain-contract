#![allow(unexpected_cfgs)]
#![allow(clippy::clone_on_copy)]

//! Overflow tests for the RentalYield arithmetic guards (issue #1372).
//!
//! Every arithmetic site is exercised at or beyond the `u128` boundary and
//! must fail with `Error::ArithmeticOverflow` rather than panic or wrap.
//! Operations that only *just* fit (`u128::MAX / PRECISION`) are accepted to
//! show the guards are exact, not off-by-one, and every failed operation
//! leaves the stored property state untouched.

use ink::env::test::{self, DefaultAccounts};
use ink::env::DefaultEnvironment;
use ink::primitives::AccountId;
use rental_yield::rental_yield::{Error, PropertyYield, RentalYield};

const PROPERTY: u64 = 42;
const PRECISION: u128 = 1_000_000_000_000;
const MAX: u128 = u128::MAX;

/// Largest amount whose `amount * PRECISION` product still fits in `u128`.
const BOUNDARY_AMOUNT: u128 = MAX / PRECISION;

fn accounts() -> DefaultAccounts<DefaultEnvironment> {
    test::default_accounts::<DefaultEnvironment>()
}

/// In the off-chain test environment the constructed contract shares
/// `alice`'s balance slot, so funding alice funds every deposit and claim.
fn fund_alice_for_max_value_transfers(accounts: &DefaultAccounts<DefaultEnvironment>) {
    test::set_account_balance::<DefaultEnvironment>(accounts.alice, MAX);
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

fn snapshot(contract: &RentalYield) -> (PropertyYield, u128, u128) {
    (
        contract.get_property(PROPERTY).expect("property exists"),
        contract.total_deposited(),
        contract.total_claimed(),
    )
}

#[ink::test]
fn overflow_deposit_amount_times_precision_rejects_at_u128_max() {
    let accounts = accounts();
    fund_alice_for_max_value_transfers(&accounts);
    let (mut contract, _) = setup(PRECISION, &[(accounts.alice, PRECISION)]);

    // `amount * PRECISION` exceeds `u128` for any amount above
    // `MAX / PRECISION`; the widest possible deposit must be rejected by the
    // checked path instead of trapping or wrapping.
    let before = snapshot(&contract);
    test::set_caller::<DefaultEnvironment>(accounts.alice);
    test::set_value_transferred::<DefaultEnvironment>(MAX);
    assert_eq!(
        contract.deposit_rent(PROPERTY),
        Err(Error::ArithmeticOverflow)
    );
    test::set_value_transferred::<DefaultEnvironment>(0);

    let after = snapshot(&contract);
    assert_eq!(before, after, "a failed deposit must not mutate any state");
}

#[ink::test]
fn overflow_deposit_boundary_amount_is_accepted_exactly() {
    let accounts = accounts();
    fund_alice_for_max_value_transfers(&accounts);
    let (mut contract, _) = setup(PRECISION, &[(accounts.alice, PRECISION)]);

    // `BOUNDARY_AMOUNT * PRECISION` is the largest product that still fits:
    // splitting it across `PRECISION` shares places the accumulator at its
    // own maximum while the rounding remainder stays zero.
    deposit(&mut contract, accounts.alice, BOUNDARY_AMOUNT);

    let property = contract.get_property(PROPERTY).expect("property exists");
    assert_eq!(property.reward_per_share, BOUNDARY_AMOUNT);
    assert_eq!(property.deposited, BOUNDARY_AMOUNT);
    assert_eq!(property.remainder, 0);
    assert_eq!(contract.total_deposited(), BOUNDARY_AMOUNT);
    assert_eq!(
        contract.claimable(PROPERTY, accounts.alice),
        BOUNDARY_AMOUNT
    );
}

#[ink::test]
fn overflow_set_shares_intermediate_debt_mul_rejects() {
    let accounts = accounts();
    fund_alice_for_max_value_transfers(&accounts);
    let (mut contract, _) = setup(MAX, &[(accounts.alice, PRECISION)]);

    // Two boundary deposits push the accumulator to `2 * MAX / PRECISION`.
    // The debt for a `PRECISION`-share holder is then
    // `PRECISION * reward_per_share > u128::MAX`, and the checked
    // multiplication must reject the assignment instead of wrapping.
    deposit(&mut contract, accounts.alice, BOUNDARY_AMOUNT);
    deposit(&mut contract, accounts.alice, BOUNDARY_AMOUNT);

    let before = snapshot(&contract);
    test::set_caller::<DefaultEnvironment>(accounts.alice);
    assert_eq!(
        contract.set_shares(PROPERTY, accounts.bob, PRECISION),
        Err(Error::ArithmeticOverflow)
    );
    let after = snapshot(&contract);
    assert_eq!(
        before, after,
        "a failed share assignment must not mutate property state"
    );

    // A single-share assignment still fits, so the guard is exact.
    contract
        .set_shares(PROPERTY, accounts.bob, 1)
        .expect("a shares * reward_per_share product within u128 succeeds");
}

#[ink::test]
fn overflow_claim_intermediate_gross_mul_rejects() {
    let accounts = accounts();
    fund_alice_for_max_value_transfers(&accounts);
    let (mut contract, _) = setup(100, &[(accounts.alice, 100)]);

    // One boundary deposit leaves alice owed exactly `BOUNDARY_AMOUNT`; the
    // claim goes through the full settle/claim path without error.
    deposit(&mut contract, accounts.alice, BOUNDARY_AMOUNT);
    assert_eq!(
        contract.claimable(PROPERTY, accounts.alice),
        BOUNDARY_AMOUNT
    );

    // A second deposit doubles the accumulator. The gross product for
    // alice's 100 shares is then `100 * reward_per_share`, which exceeds
    // `u128::MAX` as a single multiplication, so the checked `settle` guard
    // refuses to compute it and the claim fails with `ArithmeticOverflow`
    // instead of wrapping.
    deposit(&mut contract, accounts.alice, BOUNDARY_AMOUNT);
    assert_eq!(
        contract.claimable(PROPERTY, accounts.alice),
        BOUNDARY_AMOUNT,
        "the view clamps the saturating product before dividing"
    );

    let before = snapshot(&contract);
    test::set_caller::<DefaultEnvironment>(accounts.alice);
    assert_eq!(contract.claim(PROPERTY), Err(Error::ArithmeticOverflow));
    let after = snapshot(&contract);
    assert_eq!(
        before, after,
        "a failed claim must not mutate property state"
    );
}

#[ink::test]
fn overflow_repeated_deposits_keep_counters_in_sync_and_invariants_hold() {
    let accounts = accounts();
    fund_alice_for_max_value_transfers(&accounts);
    let (mut contract, _) = setup(PRECISION, &[(accounts.alice, PRECISION)]);

    // Monetary totals (`deposited`, `remainder`, `total_deposited`) only ever
    // grow by overflow-guarded additions, so repeated large deposits keep the
    // property and global counters in sync and the accounting invariants
    // intact instead of wrapping around.
    let total_before = contract.total_deposited();

    test::set_caller::<DefaultEnvironment>(accounts.alice);
    test::set_value_transferred::<DefaultEnvironment>(u64::MAX as u128);
    for _ in 0..8 {
        contract
            .deposit_rent(PROPERTY)
            .expect("u64::MAX-sized deposits are individually in range");
        let property = contract.get_property(PROPERTY).expect("property exists");
        assert!(
            property.claimed <= property.deposited,
            "claims never exceed deposits"
        );
        assert_eq!(
            property.deposited - total_before,
            contract.total_deposited() - total_before,
            "property and global deposit counters stay in sync"
        );
    }
    test::set_value_transferred::<DefaultEnvironment>(0);

    assert_eq!(
        contract.total_deposited() - total_before,
        8 * (u64::MAX as u128),
        "each deposit accumulates into the global counter"
    );
    assert_eq!(
        contract.claimable(PROPERTY, accounts.alice),
        8 * (u64::MAX as u128),
        "the holder is owed every deposited token"
    );
}
