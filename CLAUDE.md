# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

PropChain is a Rust-based smart contract system for decentralized real estate infrastructure on Substrate/Polkadot. Contracts are written in ink! 5.0 and compiled to WASM.

## Common Commands

### Build
```bash
./scripts/build.sh           # Debug build
./scripts/build.sh --release # Release build
cargo contract build         # Build a single contract (run from contract directory)
```

### Test
```bash
./scripts/test.sh                        # All tests
./scripts/test.sh --coverage             # With coverage report
./scripts/test.sh --e2e                  # E2E tests only
cargo test                               # All tests via cargo
cargo test <test_name>                   # Single test by name
cargo test <test_name> -- --nocapture    # With stdout
cargo test -p <package-name> <test_name> # Single test in a specific contract package
cargo test --lib                         # Unit tests only
```

### Lint & Format
```bash
cargo fmt --all            # Format all code
cargo clippy               # Lint
cargo check --all-features # Type check
```

### Local Dev Stack
```bash
docker-compose up          # Substrate node, IPFS, PostgreSQL, Redis, Indexer
```

## Architecture

### Stack
- **Language:** Rust (stable, see `rust-toolchain.toml`)
- **Framework:** ink! 5.0 smart contracts → WASM
- **Blockchain:** Substrate/Polkadot parachains
- **Off-chain:** IPFS (documents), PostgreSQL (indexing), Redis (caching)
- **SDKs:** TypeScript/React (`sdk/frontend/`), React Native & Flutter (`sdk/mobile/`)

### Contract Organization (`contracts/`)

All 30+ contracts live under `contracts/`, each as an independent Cargo workspace member. Key contracts:

| Contract | Role |
|---|---|
| `lib` | Core property registry — ownership, metadata, the main entry point |
| `traits` | Shared ink! trait definitions and types used across all contracts |
| `escrow` | Multi-sig fund locking and release for property transactions |
| `property-token` | ERC-721-style NFT representing property ownership |
| `compliance_registry` | Regulatory compliance checks |
| `identity` | KYC and reputation tracking |
| `oracle` | External price feed aggregation |
| `bridge` | Cross-chain interoperability |
| `fractional` | Fractional/partial ownership tokenization |
| `governance` | DAO voting and proposals |
| `lending` | Mortgage and lending protocol |
| `dex` | Decentralized property exchange |

### Data Flow
```
Frontend SDK → Indexer API → Smart Contracts → Substrate/Polkadot chain
                                    ↓
                             Events emitted
                                    ↓
                   Indexer (PostgreSQL) ← event stream
```

### Test Organization (`tests/`)
- `test_utils.rs` — shared fixtures used across all test files
- `integration_tests.rs` — cross-contract interaction tests
- `property_based_tests.rs` — proptest randomized tests
- `performance_tests.rs` — gas and benchmark tests
- Per-contract unit tests live in `contracts/{name}/src/tests.rs`

### Workspace Config
- `Cargo.toml` — workspace root; release profile uses LTO fat, opt-level z, panic=abort for minimal WASM size
- `clippy.toml` — project clippy rules
- `deny.toml` — dependency audit rules
- Pre-commit hooks enforce `cargo fmt`, `cargo clippy`, and `cargo check` before every commit
