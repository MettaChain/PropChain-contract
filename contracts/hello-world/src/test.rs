#![cfg(test)]

use super::*;
use soroban_sdk::Env;

#[test]
fn test_loan_lifecycle() {
    let env = Env::default();
    let contract_id = env.register_contract(None, LoanAnalyticsContract);
    let client = LoanAnalyticsContractClient::new(&env, &contract_id);

    client.record_loan(&5000);

    let stats = client.get_stats();
    assert_eq!(stats.total_loaned, 5000);
    assert_eq!(stats.active_loans, 1);
    assert_eq!(stats.defaults, 0);
}

#[test]
fn test_default_handling() {
    let env = Env::default();
    let contract_id = env.register_contract(None, LoanAnalyticsContract);
    let client = LoanAnalyticsContractClient::new(&env, &contract_id);

    client.record_loan(&5000);
    client.record_default();

    let stats = client.get_stats();
    assert_eq!(stats.total_loaned, 5000);
    assert_eq!(stats.active_loans, 0);
    assert_eq!(stats.defaults, 1);
}
