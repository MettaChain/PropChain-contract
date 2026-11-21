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
