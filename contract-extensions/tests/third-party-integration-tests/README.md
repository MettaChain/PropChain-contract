# Third-Party Integration Tests (issue #1535)

Integration tests for the adapter layer that connects PropChain's third-party
service registry to external contracts. Everything in this crate lives under
`contract-extensions/`; the compiled `propchain-third-party` contract is used as
a path dependency so the tests run against real, compiled contract code rather
than a hand-written stub of the registry.

## What is covered

| Acceptance criterion | Test |
|----------------------|------|
| Adapter wraps an external contract interface | `adapter_wraps_external_contract_interface` |
| View functions are forwarded to the external contract | `view_functions_are_forwarded_to_the_external_contract` |
| Write functions are executed on the external contract | `write_functions_are_forwarded_to_the_external_contract` |
| Errors returned by the external contract are handled | `view_errors_are_mapped_to_adapter_errors`, `write_errors_are_mapped_to_adapter_errors` |
| Compatibility check blocks incompatible contracts | `compatibility_check_rejects_incompatible_contracts` |
| Integration event emission for indexers | `registering_an_external_contract_emits_an_indexable_event` |
| Only authorized integrators may add contracts | `only_the_configured_integrator_can_add_external_contracts` |
| Runs against actual compiled contracts | the registry is `propchain_third_party::propchain_third_party::ThirdPartyIntegration` |

## Layout

- `src/mock_external.rs` - the `ExternalContractInterface` abstraction and a
  deterministic in-memory `MockExternalContract` implementation.
- `src/adapter.rs` - `ThirdPartyAdapter`, the compatibility gate and the
  adapter error type.
- `src/tests.rs` - the `#[ink::test]` suite exercising the whole flow.

## Running

```sh
cd contract-extensions/tests/third-party-integration-tests
cargo test
```

The crate is intentionally a standalone workspace (`[workspace]` in
`Cargo.toml`) so adding it does not require changing the root workspace
manifest.
