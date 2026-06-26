#![cfg(test)]

use super::*;
use soroban_sdk::testutils::Budget;
use soroban_sdk::Env;

/// Shared setup so every test gets a fresh env + client without repeating
/// the same three lines. Returns the client directly since the env/contract
/// id aren't needed standalone in most tests.
fn setup() -> LendingAnalyticsContractClient<'static> {
    let env = Env::default();
    let contract_id = env.register_contract(None, LendingAnalyticsContract);
    LendingAnalyticsContractClient::new(&env, &contract_id)
}

#[test]
fn test_loan_issuance() {
    let client = setup();

    client.update_stats_on_new_loan(&5000);

    let stats = client.get_dashboard_stats();
    assert_eq!(stats.total_principal_lent, 5000);
    assert_eq!(stats.active_loans_count, 1);
    assert_eq!(stats.completed_loans_count, 0);
    assert_eq!(stats.defaulted_loans_count, 0);
}

#[test]
fn test_settlement() {
    let client = setup();

    client.update_stats_on_new_loan(&5000);
    client.update_stats_on_repayment(&false); // successful settlement

    let stats = client.get_dashboard_stats();
    assert_eq!(stats.total_principal_lent, 5000);
    assert_eq!(stats.active_loans_count, 0);
    assert_eq!(stats.completed_loans_count, 1);
    assert_eq!(stats.defaulted_loans_count, 0);
}

#[test]
fn test_default_scenario() {
    let client = setup();

    client.update_stats_on_new_loan(&5000);
    client.update_stats_on_repayment(&true); // loan default

    let stats = client.get_dashboard_stats();
    assert_eq!(stats.total_principal_lent, 5000);
    assert_eq!(stats.active_loans_count, 0);
    assert_eq!(stats.completed_loans_count, 0);
    assert_eq!(stats.defaulted_loans_count, 1);
}

#[test]
fn test_multiple_loan_records_overflow_check() {
    let env = Env::default();
    env.budget().reset_unlimited();
    let contract_id = env.register_contract(None, LendingAnalyticsContract);
    let client = LendingAnalyticsContractClient::new(&env, &contract_id);

    let num_loans: u32 = 150;
    let loan_amount: i128 = 1000;

    for _ in 0..num_loans {
        client.update_stats_on_new_loan(&loan_amount);
    }

    for _ in 0..(num_loans / 2) {
        client.update_stats_on_repayment(&false);
        client.update_stats_on_repayment(&true);
    }

    let final_stats = client.get_dashboard_stats();
    assert_eq!(final_stats.total_principal_lent, (num_loans as i128) * loan_amount);
    assert_eq!(final_stats.active_loans_count, 0);
    assert_eq!(final_stats.completed_loans_count, num_loans / 2);
    assert_eq!(final_stats.defaulted_loans_count, num_loans / 2);
}

// ---- New coverage below ----

#[test]
fn test_dashboard_stats_default_to_zero_before_any_loan() {
    // Guards against stats storage being uninitialized/garbage rather
    // than a clean zero state on a fresh contract.
    let client = setup();

    let stats = client.get_dashboard_stats();
    assert_eq!(stats.total_principal_lent, 0);
    assert_eq!(stats.active_loans_count, 0);
    assert_eq!(stats.completed_loans_count, 0);
    assert_eq!(stats.defaulted_loans_count, 0);
}

#[test]
fn test_zero_amount_loan_does_not_affect_principal() {
    // Edge case: a zero-amount loan should still count as an active loan
    // (assuming that's a valid call) but must not corrupt the principal sum.
    let client = setup();

    client.update_stats_on_new_loan(&0);

    let stats = client.get_dashboard_stats();
    assert_eq!(stats.total_principal_lent, 0);
    assert_eq!(stats.active_loans_count, 1);
}

#[test]
fn test_multiple_active_loans_accumulate_principal_independently() {
    // Two loans of different sizes should both stay "active" and sum
    // correctly, distinguishing this from the single-loan happy path.
    let client = setup();

    client.update_stats_on_new_loan(&3000);
    client.update_stats_on_new_loan(&7000);

    let stats = client.get_dashboard_stats();
    assert_eq!(stats.total_principal_lent, 10_000);
    assert_eq!(stats.active_loans_count, 2);
    assert_eq!(stats.completed_loans_count, 0);
    assert_eq!(stats.defaulted_loans_count, 0);
}

#[test]
fn test_repayment_after_settlement_and_default_in_sequence() {
    // Interleaves settlement and default across multiple loans rather than
    // testing them in isolation, closer to how loans would resolve in
    // practice (out of order, mixed outcomes).
    let client = setup();

    client.update_stats_on_new_loan(&1000);
    client.update_stats_on_new_loan(&2000);
    client.update_stats_on_new_loan(&3000);

    client.update_stats_on_repayment(&false); // loan 1 settles
    client.update_stats_on_repayment(&true);  // loan 2 defaults
    client.update_stats_on_repayment(&false); // loan 3 settles

    let stats = client.get_dashboard_stats();
    assert_eq!(stats.total_principal_lent, 6000);
    assert_eq!(stats.active_loans_count, 0);
    assert_eq!(stats.completed_loans_count, 2);
    assert_eq!(stats.defaulted_loans_count, 1);
}

#[test]
fn test_large_principal_amount_near_i128_range() {
    // Checks that a single very large principal is stored/summed correctly
    // without truncation, distinct from the *count* overflow test below.
    let client = setup();

    let large_amount: i128 = 1_000_000_000_000_000_000;
    client.update_stats_on_new_loan(&large_amount);

    let stats = client.get_dashboard_stats();
    assert_eq!(stats.total_principal_lent, large_amount);
    assert_eq!(stats.active_loans_count, 1);
}

#[test]
#[should_panic]
fn test_repayment_without_any_active_loan_panics() {
    // If there's no active loan to settle, calling repayment should not
    // silently underflow active_loans_count to a bogus state — it should
    // panic (or otherwise be rejected). If the real contract instead
    // returns a Result/error code, replace this with an explicit check
    // against that error rather than `should_panic`.
    let client = setup();

    client.update_stats_on_repayment(&false);
}