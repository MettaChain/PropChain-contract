# Development Setup

This guide walks you through setting up a local development environment for
PropChain Smart Contracts.

## Prerequisites

| Tool | Minimum Version | Notes |
|------|-----------------|-------|
| Rust | 1.70 (stable) | Install via [rustup](https://rustup.rs/) |
| cargo-contract | 4.x | `cargo install cargo-contract` |
| Docker | 24.x | Required for local Substrate node |
| Git | 2.x | Version control |

### Rust Toolchain

```bash
# Install stable toolchain with required components
rustup toolchain install stable
rustup component add rustfmt clippy

# Install nightly toolchain for macro formatting
rustup toolchain install nightly
rustup component add --toolchain nightly rustfmt

# Add the WASM compilation target
rustup target add wasm32-unknown-unknown
```

## Clone & Build

```bash
git clone https://github.com/MettaChain/PropChain-contract.git
cd PropChain-contract

# Install Rust toolchain and project deps
./scripts/setup.sh

# Build all contracts (debug mode)
./scripts/build.sh

# Build optimised WASM bundles
./scripts/build.sh --release
```

## Running Tests

```bash
# Run the full workspace test suite
cargo test --all-features --workspace

# Run a single contract's tests
cargo test -p propchain-lending

# Run with coverage (requires cargo-llvm-cov)
./scripts/run_tests_with_coverage.sh
```

## Code Quality

```bash
# Check formatting (stable)
cargo fmt --check

# Check formatting including ink! macro matchers (nightly)
cargo +nightly fmt --check

# Run Clippy lints
cargo clippy --all-targets --all-features -- -D warnings
```

### rustfmt Configuration

The project ships a `rustfmt.toml` in the repo root that enables
`format_macro_matchers = true` so that `#[ink::contract]`, `#[ink::test]`,
and `propchain_traits::non_reentrant!` macro bodies are formatted
consistently.

Apply nightly formatting with:

```bash
cargo +nightly fmt
```

## Local Substrate Node

```bash
# Start a local Substrate node via Docker
docker-compose up -d

# Or use the helper script
./scripts/local-node.sh
```

## Pre-commit Hooks

```bash
# Install the pre-commit hook set
./scripts/setup-pre-commit.sh
```

The hooks run `cargo fmt --check` and `cargo clippy` before every commit.

## IDE Setup

### VS Code

Install the **rust-analyzer** extension.  A recommended `.vscode/settings.json`:

```json
{
  "rust-analyzer.checkOnSave.command": "clippy",
  "rust-analyzer.cargo.features": "all"
}
```

### IntelliJ / CLion

Use the official **Rust** plugin.  Enable "Run clippy instead of check" in
the Rust plugin settings.

## Troubleshooting

| Symptom | Fix |
|---------|-----|
| `error: linker 'cc' not found` | `apt-get install build-essential` |
| WASM target missing | `rustup target add wasm32-unknown-unknown` |
| cargo-contract not found | `cargo install cargo-contract --force` |
| Docker node won't start | Check `docker-compose logs substrate-node` |

## Further Reading

- [Contributing Guide](./CONTRIBUTING.md)
- [Architecture Overview](./ARCHITECTURE.md)
- [Security Policy](./SECURITY.md)
