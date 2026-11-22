//! End-to-end tests for the third-party adapter (#1535).

use ink::env::{test, DefaultEnvironment};
use ink::prelude::string::String;
use ink::primitives::AccountId;
use propchain_third_party::propchain_third_party::{ServiceStatus, ServiceType};

use crate::adapter::{is_compatible, AdapterError, ThirdPartyAdapter};
use crate::mock_external::{ExternalContractInterface, MockExternalContract};

type Accounts = test::DefaultAccounts<DefaultEnvironment>;

fn setup() -> (ThirdPartyAdapter, Accounts) {
    let accounts = test::default_accounts::<DefaultEnvironment>();
    test::set_caller::<DefaultEnvironment>(accounts.alice);
    (ThirdPartyAdapter::new(accounts.alice), accounts)
}

fn register(adapter: &mut ThirdPartyAdapter, accounts: &Accounts, version: &str) -> u32 {
    test::set_caller::<DefaultEnvironment>(accounts.alice);
    let external = MockExternalContract::new(accounts.bob, version);
    adapter
        .add_external_contract(
            accounts.alice,
            ServiceType::DataOracle,
            String::from("External Oracle"),
            &external,
            150,
        )
        .expect("compatible service should be registered")
}

#[ink::test]
fn adapter_wraps_external_contract_interface() {
    let (mut adapter, accounts) = setup();
    let service_id = register(&mut adapter, &accounts, "v1.2");

    let registration = &adapter.registrations()[0];
    assert_eq!(registration.service_id, service_id);
    assert_eq!(registration.account, accounts.bob);
    assert_eq!(registration.api_version, "v1.2");
    assert!(registration.compatible);
    assert!(adapter.is_registered(service_id));
}

#[ink::test]
fn registration_is_written_to_the_compiled_registry_contract() {
    let (mut adapter, accounts) = setup();
    let service_id = register(&mut adapter, &accounts, "v1");

    let config = adapter
        .service_config(service_id)
        .expect("registry must contain the registered service");
    assert_eq!(config.service_id, service_id);
    assert_eq!(config.service_type, ServiceType::DataOracle);
    assert_eq!(config.provider_account, accounts.bob);
    assert_eq!(config.api_version, "v1");
    assert_eq!(config.status, ServiceStatus::Active);
    assert_eq!(config.fee_percentage, 150);
}

#[ink::test]
fn compatibility_check_accepts_the_supported_api_family() {
    assert!(is_compatible("v1"));
    assert!(is_compatible("v1.0"));
    assert!(is_compatible("v1.9.3"));
}

#[ink::test]
fn compatibility_check_rejects_incompatible_contracts() {
    assert!(!is_compatible("v0"));
    assert!(!is_compatible("v2"));
    assert!(!is_compatible("legacy-v1"));

    let (mut adapter, accounts) = setup();
    test::set_caller::<DefaultEnvironment>(accounts.alice);
    let external = MockExternalContract::new(accounts.bob, "v0-legacy");

    let result = adapter.add_external_contract(
        accounts.alice,
        ServiceType::DataOracle,
        String::from("Old Oracle"),
        &external,
        100,
    );

    assert_eq!(result, Err(AdapterError::IncompatibleContract));
    assert!(adapter.registrations().is_empty());
    assert_eq!(test::recorded_events().count(), 0);
}

#[ink::test]
fn view_functions_are_forwarded_to_the_external_contract() {
    let (mut adapter, accounts) = setup();
    let service_id = register(&mut adapter, &accounts, "v1");
    let mut external = MockExternalContract::new(accounts.bob, "v1");

    external.write_value(7, 42, accounts.bob).unwrap();
    assert_eq!(adapter.view(service_id, &external, 7), Ok(42));
    assert_eq!(adapter.view(service_id, &external, 8), Ok(0));
}

#[ink::test]
fn view_errors_are_mapped_to_adapter_errors() {
    let (mut adapter, accounts) = setup();
    let service_id = register(&mut adapter, &accounts, "v1");
    let mut external = MockExternalContract::new(accounts.bob, "v1");
    external.fail_next_view();

    assert_eq!(
        adapter.view(service_id, &external, 7),
        Err(AdapterError::ExternalCallFailed)
    );
}

#[ink::test]
fn write_functions_are_forwarded_to_the_external_contract() {
    let (mut adapter, accounts) = setup();
    let service_id = register(&mut adapter, &accounts, "v1");
    let mut external = MockExternalContract::new(accounts.bob, "v1");

    assert_eq!(adapter.write(service_id, &mut external, accounts.bob, 7, 99), Ok(()));
    assert_eq!(adapter.view(service_id, &external, 7), Ok(99));
}

#[ink::test]
fn write_errors_are_mapped_to_adapter_errors() {
    let (mut adapter, accounts) = setup();
    let service_id = register(&mut adapter, &accounts, "v1");
    let mut external = MockExternalContract::new(accounts.bob, "v1");
    external.fail_next_write();

    assert_eq!(
        adapter.write(service_id, &mut external, accounts.bob, 7, 99),
        Err(AdapterError::ExternalWriteRejected)
    );
}

#[ink::test]
fn unauthorized_write_caller_is_rejected_by_the_external_contract() {
    let (mut adapter, accounts) = setup();
    let service_id = register(&mut adapter, &accounts, "v1");
    let mut external = MockExternalContract::new(accounts.bob, "v1");

    assert_eq!(
        adapter.write(service_id, &mut external, accounts.charlie, 7, 99),
        Err(AdapterError::ExternalWriteRejected)
    );
    assert_eq!(adapter.view(service_id, &external, 7), Ok(0));
}

#[ink::test]
fn registering_an_external_contract_emits_an_indexable_event() {
    let (mut adapter, accounts) = setup();
    let before = test::recorded_events().count();
    register(&mut adapter, &accounts, "v1");

    assert_eq!(test::recorded_events().count() - before, 1);
}

#[ink::test]
fn only_the_configured_integrator_can_add_external_contracts() {
    let (mut adapter, accounts) = setup();
    test::set_caller::<DefaultEnvironment>(accounts.bob);
    let external = MockExternalContract::new(accounts.bob, "v1");

    let result = adapter.add_external_contract(
        accounts.bob,
        ServiceType::DataOracle,
        String::from("Rogue Oracle"),
        &external,
        100,
    );

    assert_eq!(result, Err(AdapterError::UnauthorizedIntegrator));
    assert!(adapter.registrations().is_empty());
}

#[ink::test]
fn calls_against_unregistered_contracts_are_rejected() {
    let (adapter, accounts) = setup();
    let external = MockExternalContract::new(accounts.bob, "v1");

    assert_eq!(adapter.view(999, &external, 7), Err(AdapterError::NotRegistered));
}

#[ink::test]
fn multiple_contracts_are_tracked_independently() {
    let (mut adapter, accounts) = setup();
    let first = register(&mut adapter, &accounts, "v1");
    test::set_caller::<DefaultEnvironment>(accounts.alice);
    let second_external = MockExternalContract::new(accounts.charlie, "v1.4");
    let second = adapter
        .add_external_contract(
            accounts.alice,
            ServiceType::PaymentGateway,
            String::from("Second Gateway"),
            &second_external,
            250,
        )
        .unwrap();

    assert_eq!((first, second), (1, 2));
    assert_eq!(adapter.registrations().len(), 2);
    assert_eq!(
        adapter.service_config(second).unwrap().provider_account,
        accounts.charlie
    );
    assert_eq!(adapter.service_config(first).unwrap().provider_account, accounts.bob);
}
