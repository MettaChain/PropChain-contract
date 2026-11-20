# ink! Testing Strategies: Unit, Integration and E2E

This guide describes the test layers present in this workspace and how to choose between them. The examples and paths are grounded in the test crate, contract unit tests and scripts; adapt imports and constructors to the exact crate and message under test.

## Choose the right test layer

| Layer | What it exercises | Typical location | Requires a running node? |
|---|---|---|---|
| Contract unit test (`#[ink::test]`) | One contract instance in ink!'s off-chain test environment; Rust message calls, storage and mocked environment values | Contract `src/` tests, such as `contracts/staking/src/tests.rs` | No |
| Workspace integration-style test | Several contract crates and state machines composed in a Rust test process | `tests/integration_*.rs` | Usually no; these tests call contract Rust APIs directly |
| ink! E2E test (`#[ink_e2e::test]`) | Deployed Wasm contract messages, runtime behavior and cross-contract calls against a node | `tests/integration_tests.rs`, when built with the E2E feature | Yes |
| Performance/gas analysis | Workload behavior and execution cost in a specified environment | `tests/performance_benchmarks.rs`, contract tooling | Depends on the measurement method |

“Integration” in a filename does not necessarily mean a local-node test. For example, `tests/integration_fractional.rs` and `tests/integration_dex.rs` instantiate Rust contract structs and call their methods directly; they are valuable multi-step contract behavior tests but do not validate Wasm deployment or cross-contract ABI wiring.

## Unit tests with `#[ink::test]`

The `#[ink::test]` macro sets up an off-chain ink! environment and runs the test synchronously. Instantiate the contract directly, set the relevant environment values, call messages as Rust methods, and assert on return values, getters, emitted events or resulting state. Tests should make each external environment dependency explicit rather than relying on test defaults.

### Mocking accounts, balances and block context

`ink::env::test` provides helpers used in this repository:

```rust
use ink::env::{test, DefaultEnvironment};

#[ink::test]
fn example_offchain_setup() {
    let accounts = test::default_accounts::<DefaultEnvironment>();
    test::set_caller::<DefaultEnvironment>(accounts.alice);

    // Set the environment context used by payable messages and time logic.
    test::set_value_transferred::<DefaultEnvironment>(250);
    test::set_block_number::<DefaultEnvironment>(12);
    test::set_block_timestamp::<DefaultEnvironment>(10_000);
    test::set_account_balance::<DefaultEnvironment>(accounts.alice, 1_000_000);

    // Construct the contract after selecting its constructor caller.
    let _caller = ink::env::caller::<DefaultEnvironment>();
}
```

Use `set_caller` before the operation whose authorization is under test. `set_value_transferred` sets the call's attached value; it is not a substitute for setting an account's balance. Reset transferred value/caller between calls where the scenario changes. For time-gated behavior, advance or set block number/timestamp deliberately and assert behavior both before and after the threshold.

The helper APIs are environment mocks. They do not model transaction fees, node consensus, or real chain balance accounting in every detail.

### Test a rejected call as well as a happy path

For state-changing messages, assert that invalid input/authorization returns the expected error and leaves all relevant state unchanged. A useful structure is:

```rust
#[ink::test]
fn rejected_operation_preserves_state() {
    let accounts = test::default_accounts::<DefaultEnvironment>();
    test::set_caller::<DefaultEnvironment>(accounts.alice);
    let mut contract = ExampleContract::new();

    let before = contract.balance_of(accounts.alice);
    test::set_caller::<DefaultEnvironment>(accounts.bob);

    assert_eq!(
        contract.withdraw(u128::MAX),
        Err(ExampleError::InsufficientBalance)
    );
    assert_eq!(contract.balance_of(accounts.alice), before);
}
```

Replace the example types/messages with the actual contract API. Also test boundary values, duplicate operations, role restrictions, time windows, attached value, rounding and failure paths where applicable.

## Repository examples by contract type

### Staking

[`contracts/staking/src/tests.rs`](../../contracts/staking/src/tests.rs) defines account helpers, selects the constructor caller, sets block number and creates a configured `Staking` instance. It tests validator registration, minimum-stake checks, commissions and authorization:

```rust
let accounts = ink::env::test::default_accounts::<ink::env::DefaultEnvironment>();
ink::env::test::set_caller::<ink::env::DefaultEnvironment>(accounts.alice);
let mut staking = Staking::new(500, 1_000);

ink::env::test::set_caller::<ink::env::DefaultEnvironment>(accounts.bob);
assert!(staking.register_validator(MIN_VALIDATOR_STAKE, 500).is_ok());
assert_eq!(
    staking.get_validator_info(accounts.bob).unwrap().self_stake,
    MIN_VALIDATOR_STAKE
);
```

Add separate tests for reward accrual/claiming, lock periods and delegation. When testing time- or block-dependent behavior, explicitly advance the environment; do not infer elapsed time from the order of Rust calls.

### Governance

[`tests/integration_governance.rs`](../../tests/integration_governance.rs) covers a proposal lifecycle: configure signers and threshold, create proposal, vote, reject execution before approval/timelock, advance block number, and execute. Include non-signer and double-vote cases. Governance tests should check both the proposal's status and the unauthorized caller's failure result.

For example, the repository advances the mocked block beyond the proposal timelock before execution:

```rust
let proposal_id = create_proposal(&mut governance);
test::set_caller::<DefaultEnvironment>(accounts.alice);
governance.vote(proposal_id, true).expect("first vote");
test::set_caller::<DefaultEnvironment>(accounts.bob);
governance.vote(proposal_id, true).expect("threshold vote");

let proposal = governance.get_proposal(proposal_id).unwrap();
let current_block = ink::env::block_number::<DefaultEnvironment>() as u64;
test::set_block_number::<DefaultEnvironment>(
    (current_block.max(proposal.timelock_until) + 1) as u32,
);
governance.execute_proposal(proposal_id).expect("timelock elapsed");
assert_eq!(
    governance.get_proposal(proposal_id).unwrap().status,
    ProposalStatus::Executed
);
```

### DEX

[`tests/integration_dex.rs`](../../tests/integration_dex.rs) covers the DEX's constant-product AMM: invalid/duplicate pools, LP share calculations, reserve updates, fee-adjusted swaps and slippage rejection. A solid swap test calculates or otherwise bounds expected output, verifies reserve changes, and checks reverse direction and insufficient-liquidity behavior. Do not assert only that the message returned `Ok`.

The test sets up a pool, checks the fee-adjusted quote and then verifies updated reserves:

```rust
let pair = dex.create_pool(1, 2, 100, 1_000_000, 4_000_000).expect("pool");
let amount_in = 100_000u128;
let fee_adjusted = amount_in * 9_900 / 10_000;
let expected_out = fee_adjusted * 4_000_000 / (1_000_000 + fee_adjusted);

let out = dex
    .swap_exact_base_for_quote(pair, amount_in, 0)
    .expect("swap succeeds");
assert_eq!(out, expected_out);
let pool = dex.get_pool(pair).unwrap();
assert_eq!(pool.reserve_base, 1_100_000);
assert_eq!(pool.reserve_quote, 4_000_000 - expected_out);
```

### Cross-contract-sensitive scenarios

Use the direct-contract suite for business logic and unit-testable errors. Use a true node-backed test when the scenario depends on contract addresses, ABI compatibility, deployed Wasm, callback behavior or runtime rollback. `tests/cross_contract_integration.rs` and the bridge/oracle tests provide examples of workspace-level integration composition, but inspect each test's setup to determine whether it uses deployed contracts or direct Rust calls.

## Access control and reentrancy coverage

### Access control

For each privileged message, test:

- the authorized admin/role succeeds;
- an unrelated account receives the expected error;
- failed access does not mutate balances, roles, configuration or counters;
- role grants/revocations affect subsequent calls as intended.

`test::set_caller` is the standard off-chain way to switch identities between direct calls. Do not treat changing the caller in an off-chain test as cryptographic identity verification; it only sets the mocked environment caller.

### Reentrancy

Use a fast unit test to verify `ReentrancyGuard`'s `enter`/`exit` semantics and that a protected operation remains unlocked after normal and error returns. `contracts/traits/tests/reentrancy_guard_integration.rs` contains a vault-style self-reentry model. For an actual callback path, deploy a test callee that attempts to call the protected message during the external interaction and assert that:

1. the inner call fails with the mapped `ReentrantCall` error;
2. no second payout or duplicate state transition occurs;
3. the outer call's final state is consistent; and
4. a later valid call can proceed.

A model of calling `enter()` twice is not itself proof that an external callback is wired correctly.

## E2E tests with a local Substrate node

An E2E test deploys Wasm and submits messages through `ink_e2e`. `tests/integration_tests.rs` uses the `#[ink_e2e::test]` attribute, a client, a constructor, `instantiate`, `build_message`, `call`, and `call_dry_run`. This layer catches issues absent from direct Rust method calls, such as constructor metadata, message encoding, deployed addresses and runtime call behavior.

The test crate's `Cargo.toml` explicitly declares test targets because its package root is already the `tests/` directory. It enables `ink_e2e` through the `e2e-tests` feature, but the current manifest does not declare `integration_tests.rs` or `integration_governance.rs` as `[[test]]` targets. Register a target before expecting Cargo to discover or run either source; documentation-only work should not silently imply that an undeclared file is runnable.

General sequence:

1. Start a node configured for ink! contracts and make sure contract execution is enabled.
2. Build the target contract artifact using the repository's documented tooling.
3. Enable the test crate's `e2e-tests` feature, which enables `ink_e2e`.
4. Instantiate the contract using a funded development signer.
5. Submit state-changing calls with `client.call`; use `call_dry_run` for read-only/preflight checks.
6. Assert the decoded return value and inspect follow-up reads/events.
7. For cross-contract tests, instantiate/configure all participating contracts and test both success and failure/rollback paths.

`ink_e2e::Client` and node/configuration APIs can differ by ink! release. Match the repository's ink! 5.0.0 dependencies and current E2E test setup rather than copying examples for another major version. Do not describe a direct Rust integration test as an E2E test merely because it covers multiple steps.

## Gas and performance testing

`tests/performance_benchmarks.rs` labels its tests as performance benchmarks, but the shown thresholds measure the difference in block timestamp units around direct off-chain calls. They are useful as workload/regression examples; they are not node-measured gas or weight readings.

The repository's [`scripts/test.sh`](../../scripts/test.sh) documents separate test and gas paths. Its gas path invokes `cargo contract build` and `cargo contract test --execution gas` for contracts. For meaningful gas comparisons:

- measure the same artifact, runtime/node version, optimization profile and message inputs;
- record the measured gas/weight and chain context, not only a test timestamp;
- include worst-case storage size, loops and external calls;
- compare the guarded and unguarded operation only in a controlled environment;
- do not promote off-chain timestamp thresholds as on-chain gas guarantees.

## Example command selection

These are examples of the repository's Rust testing conventions; adjust package and target names to the selected contract. E2E invocation also requires a configured node and the E2E feature.

```sh
# One contract crate's unit tests
cargo test -p staking --lib

# A workspace integration-style test target
cargo test -p propchain-tests --test integration_dex

# E2E invocation after registering integration_tests.rs as a [[test]] target
cargo test -p propchain-tests --features e2e-tests --test integration_tests

# Repository's test script (runs its configured unit/integration/contract suites)
./scripts/test.sh
```

The script may execute several Cargo test/build commands; choose the smallest focused target for local iteration, and use the repository script/CI suite when broad validation is appropriate.

## PR testing checklist

- [ ] Tests cover the success path and relevant rejection paths.
- [ ] Caller, transferred value, balances and block/time are set explicitly where relevant.
- [ ] Assertions verify state invariants, not only returned success.
- [ ] Access-control behavior is tested with authorized and unauthorized callers.
- [ ] Cross-contract behavior is tested at the appropriate layer (direct composition versus deployed E2E).
- [ ] Reentrancy-sensitive paths include a nested-call/callback attempt and post-call unlock/state checks.
- [ ] Performance claims distinguish timestamp proxies from measured gas/weight.
- [ ] Relevant focused tests and repository CI checks are run and their exact outcomes are reported.
