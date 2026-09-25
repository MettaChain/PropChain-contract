#![allow(clippy::duplicated_attributes)]
#![cfg(test)]

use ink::env::{test, DefaultEnvironment};

use super::*;
use crate::propchain_lending::{
    CollateralKind, ListingStatus, LoanStatus, LoanType, PaymentScheduleStatus, Schedule,
};

#[ink::test]
fn test_loan_interest_accrual_is_jit_only_on_loan_modification() {
    let accounts = test::default_accounts::<DefaultEnvironment>();
    test::set_caller::<DefaultEnvironment>(accounts.alice);
    let mut contract = PropertyLending::new(accounts.alice);

    test::set_block_timestamp::<DefaultEnvironment>(100);
    test::set_caller::<DefaultEnvironment>(accounts.bob);
    let loan_id = contract
        .apply_for_loan_with_terms(1, 700_000, 1_000_000, 0, 12, 650)
        .unwrap();

    test::set_caller::<DefaultEnvironment>(accounts.alice);
    for _ in 0..6 {
        contract.record_repayment(accounts.bob).unwrap();
    }
    assert!(contract.underwrite_loan(loan_id).unwrap());

    let loan_before = contract.get_loan(loan_id).unwrap();
    assert_eq!(loan_before.accrued_interest, 0);
    assert_eq!(loan_before.last_interest_timestamp, 100);

    test::set_block_timestamp::<DefaultEnvironment>(1000);
    let loan_during_idle = contract.get_loan(loan_id).unwrap();
    assert_eq!(loan_during_idle.accrued_interest, 0);

    test::set_caller::<DefaultEnvironment>(accounts.bob);
    contract
        .propose_loan_restructuring(loan_id, 24, 600)
        .unwrap();
    test::set_caller::<DefaultEnvironment>(accounts.alice);
    assert!(contract.approve_loan_restructuring(loan_id).unwrap());

    let loan_after = contract.get_loan(loan_id).unwrap();
    // After the fix, the interest snapshot is correctly preserved: the
    // stale-write bug has been resolved by reloading `app` from storage
    // after update_interest_snapshot.
    assert_eq!(loan_after.accrued_interest, 1);
    assert_eq!(loan_after.interest_rate_bps, 600);
    assert_eq!(loan_after.last_interest_timestamp, 1000);
}

// ── #827: Multi-token collateral basket tests ─────────────────────────────

#[ink::test]
fn create_loan_listing_with_collateral_basket() {
    let accounts = test::default_accounts::<DefaultEnvironment>();
    test::set_caller::<DefaultEnvironment>(accounts.alice);
    let mut contract = PropertyLending::new(accounts.alice);

    test::set_caller::<DefaultEnvironment>(accounts.bob);
    let basket = vec![(1u64, 100_000u128), (2u64, 200_000u128)];
    let listing_id = contract
        .create_loan_listing(
            1,
            1_000_000,
            800,
            12,
            CollateralKind::Unsecured,
            basket.clone(),
        )
        .unwrap();

    let listing = contract.get_loan_listing(listing_id).unwrap();
    assert_eq!(listing.collateral_basket.len(), 2);
    assert_eq!(listing.collateral_basket[0], (1, 100_000));
    assert_eq!(listing.collateral_basket[1], (2, 200_000));
    assert_eq!(listing.requested_amount, 1_000_000);
}

#[ink::test]
fn create_loan_listing_with_empty_basket() {
    let accounts = test::default_accounts::<DefaultEnvironment>();
    test::set_caller::<DefaultEnvironment>(accounts.alice);
    let mut contract = PropertyLending::new(accounts.alice);

    test::set_caller::<DefaultEnvironment>(accounts.bob);
    let listing_id = contract
        .create_loan_listing(1, 1_000_000, 800, 12, CollateralKind::Unsecured, vec![])
        .unwrap();

    let listing = contract.get_loan_listing(listing_id).unwrap();
    assert!(listing.collateral_basket.is_empty());
}

// ── #829: Variable amortization schedule tests ────────────────────────────

#[ink::test]
fn create_bullet_payment_schedule() {
    let accounts = test::default_accounts::<DefaultEnvironment>();
    test::set_caller::<DefaultEnvironment>(accounts.alice);
    let mut contract = PropertyLending::new(accounts.alice);

    test::set_caller::<DefaultEnvironment>(accounts.bob);
    let loan_id = contract
        .apply_for_loan_with_terms(1, 1_000_000, 2_000_000, 600, 12, 800)
        .unwrap();

    test::set_caller::<DefaultEnvironment>(accounts.alice);
    let schedule_id = contract
        .create_payment_schedule(loan_id, Schedule::Bullet, 432_000)
        .unwrap();

    let schedule = contract.get_payment_schedule_by_loan(loan_id).unwrap();
    assert_eq!(schedule.schedule_id, schedule_id);
    assert_eq!(schedule.schedule_type, Schedule::Bullet);
    assert_eq!(schedule.installment_amount, 1_000_000);
    assert_eq!(schedule.status, PaymentScheduleStatus::Active);
}

#[ink::test]
fn create_annuity_payment_schedule() {
    let accounts = test::default_accounts::<DefaultEnvironment>();
    test::set_caller::<DefaultEnvironment>(accounts.alice);
    let mut contract = PropertyLending::new(accounts.alice);

    test::set_caller::<DefaultEnvironment>(accounts.bob);
    let loan_id = contract
        .apply_for_loan_with_terms(1, 100_000, 200_000, 600, 6, 500)
        .unwrap();

    test::set_caller::<DefaultEnvironment>(accounts.alice);
    let _schedule_id = contract
        .create_payment_schedule(loan_id, Schedule::Annuity, 216_000)
        .unwrap();

    let schedule = contract.get_payment_schedule_by_loan(loan_id).unwrap();
    assert_eq!(schedule.schedule_type, Schedule::Annuity);
    assert!(schedule.installment_amount > 0);
    assert_eq!(schedule.total_installments, 12); // 6 months * 432_000 / 216_000
}

#[ink::test]
fn create_linear_payment_schedule() {
    let accounts = test::default_accounts::<DefaultEnvironment>();
    test::set_caller::<DefaultEnvironment>(accounts.alice);
    let mut contract = PropertyLending::new(accounts.alice);

    test::set_caller::<DefaultEnvironment>(accounts.bob);
    let loan_id = contract
        .apply_for_loan_with_terms(1, 300_000, 600_000, 700, 12, 600)
        .unwrap();

    test::set_caller::<DefaultEnvironment>(accounts.alice);
    let _schedule_id = contract
        .create_payment_schedule(loan_id, Schedule::Linear, 216_000)
        .unwrap();

    let schedule = contract.get_payment_schedule_by_loan(loan_id).unwrap();
    assert_eq!(schedule.schedule_type, Schedule::Linear);
    assert!(schedule.installment_amount > 0);
    // 12 months * 432_000 blocks/month / 216_000 blocks/installment = 24
    assert_eq!(schedule.total_installments, 24);
}

#[ink::test]
fn create_custom_payment_schedule() {
    let accounts = test::default_accounts::<DefaultEnvironment>();
    test::set_caller::<DefaultEnvironment>(accounts.alice);
    let mut contract = PropertyLending::new(accounts.alice);

    test::set_caller::<DefaultEnvironment>(accounts.bob);
    let loan_id = contract
        .apply_for_loan_with_terms(1, 200_000, 400_000, 700, 12, 600)
        .unwrap();
    test::set_caller::<DefaultEnvironment>(accounts.alice);
    let _schedule_id = contract
        .create_payment_schedule(
            loan_id,
            Schedule::Custom {
                num_installments: 24,
                interval_blocks: 216_000,
                principal_per_payment: 10_000,
            },
            216_000,
        )
        .unwrap();

    let schedule = contract.get_payment_schedule_by_loan(loan_id).unwrap();
    assert_eq!(
        schedule.schedule_type,
        Schedule::Custom {
            num_installments: 24,
            interval_blocks: 216_000,
            principal_per_payment: 10_000,
        }
    );
    assert_eq!(schedule.installment_amount, 10_000);
}

#[ink::test]
fn create_payment_schedule_unauthorized_fails() {
    let accounts = test::default_accounts::<DefaultEnvironment>();
    test::set_caller::<DefaultEnvironment>(accounts.alice);
    let mut contract = PropertyLending::new(accounts.alice);

    test::set_caller::<DefaultEnvironment>(accounts.bob);
    let loan_id = contract
        .apply_for_loan_with_terms(1, 100_000, 200_000, 600, 12, 500)
        .unwrap();

    // Charlie (not admin or borrower) tries to create a schedule
    test::set_caller::<DefaultEnvironment>(accounts.charlie);
    let result = contract.create_payment_schedule(loan_id, Schedule::Bullet, 432_000);
    assert_eq!(result, Err(LendingError::Unauthorized));
}

// ── Risk-path coverage: position PnL, liquidation, restructuring (Issue #978)

/// Active loan fixture: admin = alice, borrower = bob, collateral assessed
/// for property 10 (assessed 1_000_000, liq threshold 8000 bps). Bob's credit
/// profile is built with six on-time repayments so underwriting approves.
fn setup_active_loan(
    contract: &mut PropertyLending,
    accounts: &test::DefaultAccounts<DefaultEnvironment>,
) -> u64 {
    test::set_block_timestamp::<DefaultEnvironment>(100);
    contract
        .assess_collateral(10, 1_000_000, 7_000, 8_000)
        .unwrap();

    test::set_caller::<DefaultEnvironment>(accounts.bob);
    let loan_id = contract
        .apply_for_loan(10, 500_000, 1_000_000, 999)
        .unwrap();

    // Build credit score (500 base + 6 * 20 = 620 >= 600) as the admin.
    test::set_caller::<DefaultEnvironment>(accounts.alice);
    for _ in 0..6 {
        contract.record_repayment(accounts.bob).unwrap();
    }
    assert!(contract.underwrite_loan(loan_id).unwrap());
    loan_id
}

#[ink::test]
fn position_pnl_boundaries_long_and_short() {
    let accounts = test::default_accounts::<DefaultEnvironment>();
    test::set_caller::<DefaultEnvironment>(accounts.alice);
    let mut contract = PropertyLending::new(accounts.alice);

    test::set_caller::<DefaultEnvironment>(accounts.bob);
    let long_id = contract.open_position(1_000, 150, false, 100).unwrap();
    let short_id = contract.open_position(1_000, 150, true, 100).unwrap();

    // Boundary: no price movement means zero PnL regardless of direction.
    assert_eq!(contract.position_pnl(long_id, 100), Ok(0));
    assert_eq!(contract.position_pnl(short_id, 100), Ok(0));

    // +50% move: long gains (50 * 150) / 100, short loses symmetrically.
    assert_eq!(contract.position_pnl(long_id, 150), Ok(75));
    assert_eq!(contract.position_pnl(short_id, 150), Ok(-75));

    // -50% move flips the signs.
    assert_eq!(contract.position_pnl(long_id, 50), Ok(-75));
    assert_eq!(contract.position_pnl(short_id, 50), Ok(75));

    // Unknown positions report PositionNotFound instead of panicking.
    assert_eq!(
        contract.position_pnl(999, 100),
        Err(LendingError::PositionNotFound)
    );
}

#[ink::test]
fn liquidation_enforces_threshold_and_leaves_consistent_state() {
    let accounts = test::default_accounts::<DefaultEnvironment>();
    test::set_caller::<DefaultEnvironment>(accounts.alice);
    let mut contract = PropertyLending::new(accounts.alice);
    let loan_id = setup_active_loan(&mut contract, &accounts);

    // Healthy market: debt/current LTV stays far below the threshold. The
    // liquidation message prices collateral via the oracle feed (Issue #1088);
    // caller-supplied values are ignored.
    assert_eq!(
        contract.should_liquidate_loan(loan_id, vec![(10, 2_000_000)]),
        Ok(false)
    );
    contract.set_oracle_price(10, 2_000_000).unwrap();
    assert_eq!(
        contract.liquidate_loan(loan_id, vec![(10, 2_000_000)]),
        Err(LendingError::LiquidationThresholdNotMet)
    );

    // Collateral collapse: 500k debt against 500k value => 10_000 bps LTV,
    // above the 8_000 bps threshold. Both view and message agree.
    contract.set_oracle_price(10, 500_000).unwrap();
    assert_eq!(
        contract.should_liquidate_loan(loan_id, vec![(10, 500_000)]),
        Ok(true)
    );
    contract
        .liquidate_loan(loan_id, vec![(10, 500_000)])
        .expect("liquidation must succeed once the threshold is breached");

    // Post-liquidation state is consistent: status persisted to storage,
    // no double-liquidation possible, and the loan no longer reports as
    // liquidatable (non-Active loans short-circuit).
    let loan = contract.get_loan(loan_id).unwrap();
    assert_eq!(loan.status, LoanStatus::Liquidated);
    assert_eq!(
        contract.liquidate_loan(loan_id, vec![(10, 500_000)]),
        Err(LendingError::LoanNotActive)
    );
    assert_eq!(
        contract.should_liquidate_loan(loan_id, vec![(10, 500_000)]),
        Ok(false)
    );
}

#[ink::test]
fn restructuring_requires_both_parties_and_cleans_up_record() {
    let accounts = test::default_accounts::<DefaultEnvironment>();
    test::set_caller::<DefaultEnvironment>(accounts.alice);
    let mut contract = PropertyLending::new(accounts.alice);

    test::set_block_timestamp::<DefaultEnvironment>(100);
    contract
        .assess_collateral(20, 2_000_000, 7_000, 8_000)
        .unwrap();

    test::set_caller::<DefaultEnvironment>(accounts.bob);
    let loan_id = contract
        .apply_for_loan_with_terms(20, 600_000, 2_000_000, 0, 24, 900)
        .unwrap();

    test::set_caller::<DefaultEnvironment>(accounts.alice);
    for _ in 0..6 {
        contract.record_repayment(accounts.bob).unwrap();
    }
    assert!(contract.underwrite_loan(loan_id).unwrap());

    // A third party cannot propose a restructuring for someone else's loan.
    test::set_caller::<DefaultEnvironment>(accounts.charlie);
    assert_eq!(
        contract.propose_loan_restructuring(loan_id, 12, 650),
        Err(LendingError::Unauthorized)
    );

    // Borrower proposes; approval is pending until the lender agrees.
    test::set_caller::<DefaultEnvironment>(accounts.bob);
    contract
        .propose_loan_restructuring(loan_id, 12, 650)
        .unwrap();
    assert_eq!(
        contract.get_loan(loan_id).unwrap().status,
        LoanStatus::RestructuringProposed
    );

    // A third party can neither advance nor complete the pending approval.
    test::set_caller::<DefaultEnvironment>(accounts.charlie);
    assert_eq!(
        contract.approve_loan_restructuring(loan_id),
        Err(LendingError::Unauthorized)
    );
    assert!(
        !contract
            .get_loan_restructuring(loan_id)
            .unwrap()
            .lender_approved
    );

    test::set_caller::<DefaultEnvironment>(accounts.bob);
    assert_eq!(contract.approve_loan_restructuring(loan_id), Ok(false));

    // Lender completes the flow: terms applied and record removed.
    test::set_caller::<DefaultEnvironment>(accounts.alice);
    assert_eq!(contract.approve_loan_restructuring(loan_id), Ok(true));
    let restructured = contract.get_loan(loan_id).unwrap();
    assert_eq!(restructured.status, LoanStatus::Restructured);
    assert_eq!(restructured.term_months, 12);
    assert_eq!(restructured.interest_rate_bps, 650);
    assert!(contract.get_loan_restructuring(loan_id).is_none());

    // No residue: approving again finds no pending restructuring record.
    assert_eq!(
        contract.approve_loan_restructuring(loan_id),
        Err(LendingError::RestructuringNotFound)
    );
}

// ── Issue #1095: regression coverage for unexercised mutating messages ──────

#[ink::test]
fn fixed_rate_loan_application_persists_all_terms() {
    let accounts = test::default_accounts::<DefaultEnvironment>();
    test::set_caller::<DefaultEnvironment>(accounts.alice);
    let mut contract = PropertyLending::new(accounts.alice);

    test::set_caller::<DefaultEnvironment>(accounts.bob);
    let loan_id = contract
        .apply_for_fixed_rate_loan(1, 500_000, 1_000_000, 700, 24, 800)
        .unwrap();

    let loan = contract.get_loan(loan_id).unwrap();
    assert_eq!(loan.loan_id, loan_id);
    assert_eq!(loan.applicant, accounts.bob);
    assert_eq!(loan.property_id, 1);
    assert_eq!(loan.requested_amount, 500_000);
    assert_eq!(loan.collateral_value, 1_000_000);
    assert_eq!(loan.credit_score, 700);
    assert_eq!(loan.term_months, 24);
    assert_eq!(loan.interest_rate_bps, 800);
    assert_eq!(loan.loan_type, LoanType::FixedRate);
    assert_eq!(loan.status, LoanStatus::Pending);
    assert!(!loan.approved);
}

#[ink::test]
fn fixed_rate_loan_application_rejects_zero_terms() {
    let accounts = test::default_accounts::<DefaultEnvironment>();
    test::set_caller::<DefaultEnvironment>(accounts.alice);
    let mut contract = PropertyLending::new(accounts.alice);

    test::set_caller::<DefaultEnvironment>(accounts.bob);
    // Zero principal or zero rate must be rejected up front.
    assert_eq!(
        contract.apply_for_fixed_rate_loan(1, 0, 1_000_000, 700, 24, 800),
        Err(LendingError::InvalidParameters)
    );
    assert_eq!(
        contract.apply_for_fixed_rate_loan(1, 500_000, 1_000_000, 700, 24, 0),
        Err(LendingError::InvalidParameters)
    );
    // No loan was created for the rejected applications.
    assert_eq!(contract.get_loan(1), None);
}

#[ink::test]
fn lender_submits_offer_against_open_listing() {
    let accounts = test::default_accounts::<DefaultEnvironment>();
    test::set_caller::<DefaultEnvironment>(accounts.alice);
    let mut contract = PropertyLending::new(accounts.alice);

    test::set_caller::<DefaultEnvironment>(accounts.bob);
    let listing_id = contract
        .create_loan_listing(1, 1_000_000, 800, 12, CollateralKind::Unsecured, vec![])
        .unwrap();

    // A lender (charlie) submits an offer at or below the max rate.
    test::set_caller::<DefaultEnvironment>(accounts.charlie);
    let offer_id = contract
        .submit_loan_offer(listing_id, 900_000, 750, 12)
        .unwrap();

    let offer = contract.get_loan_offer(offer_id).unwrap();
    assert_eq!(offer.offer_id, offer_id);
    assert_eq!(offer.listing_id, listing_id);
    assert_eq!(offer.lender, accounts.charlie);
    assert_eq!(offer.offered_amount, 900_000);
    assert_eq!(offer.rate_bps, 750);
    assert_eq!(offer.term_months, 12);
    assert!(!offer.is_accepted);
}

#[ink::test]
fn submit_loan_offer_enforces_rate_cap_and_listing_state() {
    let accounts = test::default_accounts::<DefaultEnvironment>();
    test::set_caller::<DefaultEnvironment>(accounts.alice);
    let mut contract = PropertyLending::new(accounts.alice);

    test::set_caller::<DefaultEnvironment>(accounts.bob);
    let listing_id = contract
        .create_loan_listing(1, 1_000_000, 800, 12, CollateralKind::Unsecured, vec![])
        .unwrap();

    // Rate above the borrower's cap is refused.
    assert_eq!(
        contract.submit_loan_offer(listing_id, 900_000, 900, 12),
        Err(LendingError::InvalidParameters)
    );
    // Zero amount or zero term is refused.
    assert_eq!(
        contract.submit_loan_offer(listing_id, 0, 700, 12),
        Err(LendingError::InvalidParameters)
    );
    assert_eq!(
        contract.submit_loan_offer(listing_id, 900_000, 700, 0),
        Err(LendingError::InvalidParameters)
    );

    // Unknown listing: loan not found.
    assert_eq!(
        contract.submit_loan_offer(9_999, 900_000, 700, 12),
        Err(LendingError::LoanNotFound)
    );
}

#[ink::test]
fn accept_loan_offer_originates_loan_and_updates_state() {
    let accounts = test::default_accounts::<DefaultEnvironment>();
    test::set_caller::<DefaultEnvironment>(accounts.alice);
    let mut contract = PropertyLending::new(accounts.alice);

    test::set_caller::<DefaultEnvironment>(accounts.bob);
    let listing_id = contract
        .create_loan_listing(1, 1_000_000, 800, 12, CollateralKind::Unsecured, vec![])
        .unwrap();

    test::set_caller::<DefaultEnvironment>(accounts.charlie);
    let offer_id = contract
        .submit_loan_offer(listing_id, 900_000, 750, 12)
        .unwrap();

    // A third party cannot accept the borrower's listing.
    test::set_caller::<DefaultEnvironment>(accounts.charlie);
    assert_eq!(
        contract.accept_loan_offer(offer_id),
        Err(LendingError::Unauthorized)
    );

    // The borrower accepts and the loan originates.
    test::set_caller::<DefaultEnvironment>(accounts.bob);
    let loan_id = contract.accept_loan_offer(offer_id).unwrap();

    let loan = contract.get_loan(loan_id).unwrap();
    assert_eq!(loan.applicant, accounts.bob);
    assert_eq!(loan.property_id, 1);
    assert_eq!(loan.requested_amount, 900_000);
    assert_eq!(loan.loan_type, LoanType::Variable);
    assert_eq!(loan.status, LoanStatus::Active);
    assert!(loan.approved);

    // Listing and offer reflect origination.
    let listing = contract.get_loan_listing(listing_id).unwrap();
    assert_eq!(listing.status, ListingStatus::Originated);
    assert_eq!(listing.accepted_offer_id, Some(offer_id));
    assert!(contract.get_loan_offer(offer_id).unwrap().is_accepted);

    // The accepted offer cannot originate a second loan: the listing is
    // no longer Open.
    assert_eq!(
        contract.accept_loan_offer(offer_id),
        Err(LendingError::LoanNotActive)
    );
}

#[ink::test]
fn cancel_loan_listing_marks_listing_cancelled() {
    let accounts = test::default_accounts::<DefaultEnvironment>();
    test::set_caller::<DefaultEnvironment>(accounts.alice);
    let mut contract = PropertyLending::new(accounts.alice);

    test::set_caller::<DefaultEnvironment>(accounts.bob);
    let listing_id = contract
        .create_loan_listing(1, 1_000_000, 800, 12, CollateralKind::Unsecured, vec![])
        .unwrap();

    // Only the borrower may withdraw the listing.
    test::set_caller::<DefaultEnvironment>(accounts.charlie);
    assert_eq!(
        contract.cancel_loan_listing(listing_id),
        Err(LendingError::Unauthorized)
    );

    test::set_caller::<DefaultEnvironment>(accounts.bob);
    assert!(contract.cancel_loan_listing(listing_id).is_ok());
    assert_eq!(
        contract.get_loan_listing(listing_id).unwrap().status,
        ListingStatus::Cancelled
    );

    // A cancelled listing cannot be cancelled again.
    assert_eq!(
        contract.cancel_loan_listing(listing_id),
        Err(LendingError::LoanNotActive)
    );

    // Unknown listing.
    assert_eq!(
        contract.cancel_loan_listing(9_999),
        Err(LendingError::LoanNotFound)
    );
}
