#![cfg(test)]

use super::*;
use soroban_sdk::Env;
use soroban_sdk::testutils::Budget;

#[test]
fn test_loan_issuance() {
    let env = Env::default();
    let contract_id = env.register_contract(None, LendingAnalyticsContract);
    let client = LendingAnalyticsContractClient::new(&env, &contract_id);

    client.update_stats_on_new_loan(&5000);

    let stats = client.get_dashboard_stats();
    assert_eq!(stats.total_principal_lent, 5000);
    assert_eq!(stats.active_loans_count, 1);
    assert_eq!(stats.completed_loans_count, 0);
    assert_eq!(stats.defaulted_loans_count, 0);
}

#[test]
fn test_settlement() {
    let env = Env::default();
    let contract_id = env.register_contract(None, LendingAnalyticsContract);
    let client = LendingAnalyticsContractClient::new(&env, &contract_id);

    client.update_stats_on_new_loan(&5000);
    client.update_stats_on_repayment(&false); // Simulate successful settlement

    let stats = client.get_dashboard_stats();
    assert_eq!(stats.total_principal_lent, 5000);
    assert_eq!(stats.active_loans_count, 0);
    assert_eq!(stats.completed_loans_count, 1);
    assert_eq!(stats.defaulted_loans_count, 0);
}

#[test]
fn test_default_scenario() {
    let env = Env::default();
    let contract_id = env.register_contract(None, LendingAnalyticsContract);
    let client = LendingAnalyticsContractClient::new(&env, &contract_id);

    client.update_stats_on_new_loan(&5000);
    client.update_stats_on_repayment(&true); // Simulate a loan default

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

    // Simulate recording multiple high-volume loan issuances
    for _ in 0..num_loans {
        client.update_stats_on_new_loan(&loan_amount);
    }

    // Repay half successfully, default the other half
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