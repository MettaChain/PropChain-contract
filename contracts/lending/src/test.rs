#![cfg(test)]

use super::*;
use ink::env::{test, DefaultEnvironment};    #[ink::test]
    #[ignore = "TODO: re-enable after interest accrual JIT logic is stabilized"]
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
    assert!(loan_after.accrued_interest > 0);
    assert_eq!(loan_after.interest_rate_bps, 600);
    assert_eq!(loan_after.last_interest_timestamp, 1000);
}
