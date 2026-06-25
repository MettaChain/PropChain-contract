#![cfg(test)]

use super::*;
use soroban_sdk::{
    testutils::{Address as _, Ledger},
    token, Address, Env,
};

fn setup() -> (Env, Address, Address, Address, Address) {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, SubscriptionContract);
    let admin = Address::generate(&env);
    let token_id = env.register_stellar_asset_contract_v2(admin.clone()).address();

    let subscriber = Address::generate(&env);
    let merchant = Address::generate(&env);

    let token_admin = token::StellarAssetClient::new(&env, &token_id);
    token_admin.mint(&subscriber, &10_000);

    (env, contract_id, token_id, subscriber, merchant)
}

// ---------------------------------------------------------------------------
// Issue #188 – execute_payment fails with PaymentNotDue before next_payment
// ---------------------------------------------------------------------------
#[test]
fn test_execute_payment_before_due_time_returns_error() {
    let (env, contract_id, token_id, subscriber, merchant) = setup();
    let client = SubscriptionContractClient::new(&env, &contract_id);

    env.ledger().with_mut(|l| l.timestamp = 1000);
    client.subscribe(&subscriber, &merchant, &token_id, &100, &3600).unwrap();

    env.ledger().with_mut(|l| l.timestamp = 1001);
    client.execute_payment(&subscriber).unwrap();

    // next_payment is now 1001 + 3600 = 4601; calling again at T=1001 must fail
    let err = client.try_execute_payment(&subscriber).unwrap_err().unwrap();
    assert_eq!(err, Error::PaymentNotDue);

    let sub = client.get_subscription(&subscriber).unwrap();
    assert_eq!(sub.next_payment, 4601);
    assert_eq!(sub.amount, 100);
}

#[test]
fn test_execute_payment_succeeds_at_due_time() {
    let (env, contract_id, token_id, subscriber, merchant) = setup();
    let client = SubscriptionContractClient::new(&env, &contract_id);

    env.ledger().with_mut(|l| l.timestamp = 500);
    client.subscribe(&subscriber, &merchant, &token_id, &200, &3600).unwrap();
    client.execute_payment(&subscriber).unwrap();

    let sub = client.get_subscription(&subscriber).unwrap();
    assert_eq!(sub.next_payment, 500 + 3600);

    let token_client = token::Client::new(&env, &token_id);
    assert_eq!(token_client.balance(&merchant), 200);
}

// ---------------------------------------------------------------------------
// Issue #189 – cancel then re-subscribe replaces the storage record cleanly
// ---------------------------------------------------------------------------
#[test]
fn test_cancel_and_resubscribe_replaces_record() {
    let (env, contract_id, token_id, subscriber, merchant) = setup();
    let client = SubscriptionContractClient::new(&env, &contract_id);

    env.ledger().with_mut(|l| l.timestamp = 0);
    client.subscribe(&subscriber, &merchant, &token_id, &50, &600).unwrap();
    assert_eq!(client.get_subscription(&subscriber).unwrap().amount, 50);

    client.cancel(&subscriber).unwrap();
    assert!(client.get_subscription(&subscriber).is_none());

    env.ledger().with_mut(|l| l.timestamp = 100);
    client.subscribe(&subscriber, &merchant, &token_id, &99, &1200).unwrap();

    let sub = client.get_subscription(&subscriber).unwrap();
    assert_eq!(sub.amount, 99);
    assert_eq!(sub.interval, 1200);
    assert_eq!(sub.next_payment, 100);
}

#[test]
fn test_cancel_nonexistent_subscription_returns_error() {
    let (env, contract_id, _, subscriber, _) = setup();
    let client = SubscriptionContractClient::new(&env, &contract_id);

    let err = client.try_cancel(&subscriber).unwrap_err().unwrap();
    assert_eq!(err, Error::NotSubscribed);
}

#[test]
fn test_double_subscribe_returns_error() {
    let (env, contract_id, token_id, subscriber, merchant) = setup();
    let client = SubscriptionContractClient::new(&env, &contract_id);

    client.subscribe(&subscriber, &merchant, &token_id, &10, &100).unwrap();

    let err = client.try_subscribe(&subscriber, &merchant, &token_id, &10, &100)
        .unwrap_err().unwrap();
    assert_eq!(err, Error::AlreadySubscribed);
}
