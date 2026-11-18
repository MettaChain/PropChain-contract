# Contract Extensions

This folder contains self-contained extensions to the PropChain smart contract suite.

## Purpose

The main codebase (`contracts/`, `tests/`, `scripts/`, etc.) is undergoing active cleanup and refactoring by the core team. To keep new work isolated, all external additions live here.

## Subfolders

| Subfolder | Contents |
|-----------|---------|
| `contracts/` | New ink! contract modules |
| `tests/` | Unit, integration, and fuzz tests |
| `tools/` | Deployment, analysis, and CLI tools |
| `docs/` | Contract documentation and architecture notes |
| `examples/` | Usage examples and integration demos |
| `scripts/` | Build, test, and automation scripts |

## Rules

1. **Stay in this folder.** All new files must be under `contract-extensions/`.
2. **Do not modify the main codebase** (`contracts/`, `tests/`, `scripts/`, etc.) unless an issue explicitly requires it.
3. **Follow existing conventions.** Match the coding style and module structure in the main codebase.
4. **Write tests.** Every contract module should include comprehensive tests.
5. **Use ink! framework patterns.** Follow existing contract patterns (storage, events, errors, modifiers).
6. **Run clippy.** Ensure `cargo clippy` passes with no warnings.
7. **Run tests.** Ensure `cargo test` passes before submitting a PR.
