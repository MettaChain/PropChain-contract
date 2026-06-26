# Developer Onboarding Checklist

Welcome to the PropChain development team! Use this checklist to set up your environment and get familiar with our smart contract ecosystem.

## Environment Setup

- [ ] **Install Rust**: Follow the instructions in [DEVELOPMENT.md](../DEVELOPMENT.md) to install the Rust toolchain.
- [ ] **Install Substrate Tools**: Ensure you have `cargo-contract` and other necessary Substrate development tools.
- [ ] **Clone the Repository**: `git clone https://github.com/maryjane/PropChain-contract.git`
- [ ] **Verify Build**: Run `cargo build` in the root directory to ensure all contracts compile correctly.
- [ ] **Run Tests**: Run `cargo test` to verify the current state of the codebase.

## Learning the Architecture

- [ ] **Read the Overview**: Start with the [README.md](../README.md) for a high-level project summary.
- [ ] **Explore the Contracts**: Read [docs/contracts.md](contracts.md) to understand the core APIs.
- [ ] **Review Tutorials**:
  - [ ] [Insurance Integration](tutorials/insurance-integration.md)
  - [ ] [Cross-Chain Bridging](tutorials/cross-chain-bridging.md)
- [ ] **Understand Compliance**: Study the `ComplianceRegistry` and `ZkCompliance` contracts to understand how we handle regulatory requirements.

## Hands-On Exploration

- [ ] **Deploy to Local Node**: Follow the deployment guide in [DEVELOPMENT.md](../DEVELOPMENT.md) to deploy the `PropertyToken` contract to a local dev node.
- [ ] **Interact with Contracts**: Use the `polkadot-js/api` or `subxt` to call a simple method like `register_property`.
- [ ] **Try the Contract Playground**: Run `./scripts/playground.sh` for an interactive menu that walks you through the five most common calls (register a property, create an escrow, stake tokens, vote on a proposal, create an insurance policy) without having to hand-write `cargo-contract` commands. It reads addresses from your local deployment, so deploy first. See `./scripts/playground.sh --help` for details.
- [ ] **Debug a Transaction**: Use the local node logs to trace a contract call.

## Contributing

- [ ] **Read Contribution Guidelines**: Review the guidelines in [README.md](../README.md#contributing).
- [ ] **Setup Pre-commit Hooks**: Follow the instructions in [DEVELOPMENT.md](../DEVELOPMENT.md#pre-commit-hooks).
- [ ] **Create a Feature Branch**: Always work on a separate branch for your changes.
- [ ] **Follow ADRs**: Check existing [Architecture Decision Records](adr/) before making major design choices.

## Docker Test Environment

A pre-built Docker environment is available for running PropChain contract tests without a local Rust/Substrate setup.

- [ ] **Start the test environment**: Run `./scripts/start-test-env.sh` from the repo root. This brings up a `substrate-contracts-node` (dev mode), an IPFS node, and a Subsquid indexer.
- [ ] **Substrate RPC**: WebSocket at `ws://localhost:9944`, HTTP at `http://localhost:9933`
- [ ] **IPFS API**: `http://localhost:5001`
- [ ] **Stop the environment**: `docker compose -f docker-compose.test.yml down`

The test compose file is `docker-compose.test.yml`; the node image is built from `Dockerfile.test-node`.

## Finding Your First Issue

- [ ] **Browse Good First Issues**: Go to [GitHub Issues](https://github.com/MettaChain/PropChain-contract/issues?q=is%3Aopen+label%3A%22good+first+issue%22) and filter by the `good first issue` label.
- [ ] **Understand Complexity Labels**: Issues are tagged `Trivial Complexity`, `Medium Complexity`, or `High Complexity`. Start with `Trivial` or `good first issue` tagged issues.
- [ ] **Claim an Issue**: Leave a comment on the issue saying you'd like to work on it. Wait for a maintainer to assign it to you before starting.
- [ ] **Read Related Code**: Before writing any code, read the files the issue references. Use `grep` or the code search in GitHub to find relevant functions.

## Submitting Your First Pull Request

- [ ] **Sync Your Fork**: Before starting, run `git pull origin main` to ensure your branch is up to date.
- [ ] **Create a Feature Branch**: Branch naming convention is `<type>/<short-description>` (e.g., `fix/escrow-state-check`, `docs/update-faq`).
- [ ] **Make Focused Changes**: Keep PRs small and focused on a single issue. Avoid mixing unrelated changes.
- [ ] **Run Tests Before Pushing**: Always run `cargo test` locally. The CI will also run tests, but catching failures early saves time.
- [ ] **Write a Clear PR Description**: Include:
  - What the PR does
  - Which issue it closes (use `Closes #<issue-number>`)
  - How to test the changes
- [ ] **Respond to Review Feedback**: Maintainers may request changes. Push new commits to the same branch—do not open a new PR.
- [ ] **Squash Only If Asked**: Do not squash commits unless a reviewer explicitly asks you to.

## Code Standards

- [ ] **Rust Formatting**: Run `cargo fmt` before committing. The CI enforces `rustfmt` style.
- [ ] **Linting**: Run `cargo clippy -- -D warnings` and resolve all warnings before submitting.
- [ ] **Documentation**: All public functions must have `///` doc comments. Update existing docs if your change affects them.
- [ ] **Error Codes**: If you add a new error variant with a numeric code, assign it from the correct range in `contracts/traits/src/errors.rs` and update `docs/ERROR_CODES.md`.
- [ ] **ADRs**: For significant design decisions, add an ADR in `docs/adr/` following the template in `0001-record-architecture-decisions.md`.

## Getting Help

- [ ] **Review FAQ**: Check the [Troubleshooting/FAQ](troubleshooting-faq.md) for common issues.
- [ ] **Join the Dev Channel**: Reach out to the team on our designated discord/slack channel for support.
- [ ] **Ask in the Issue**: If you're stuck on a specific issue, ask questions directly in the GitHub issue thread. Maintainers actively monitor open issues.
- [ ] **Check Architecture Docs**: The `docs/adr/` directory explains *why* the system is designed the way it is—read relevant ADRs before proposing architectural changes.
