# Hello-World Starter Crate: Removal Record

`contracts/hello-world` has been removed from this repository. This document is the
record of that removal and of what was removed with it.

## Deprecation notice

- `contracts/hello-world` was used as an initial template for ink! contract development.
- Production contracts (`bridge`, `lending`, `oracle`, `property-management`) take
  precedence, and every one of them is ink!.
- The crate was marked for removal from the workspace `Cargo.toml` members. **That
  removal is now complete** — see below.

## What was removed

| Removed                  | Detail                                                                                                                                                                                                                         |
| ------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `contracts/hello-world/` | The crate itself: `Cargo.toml`, `src/lib.rs`, `src/test.rs`, `Makefile` and `test_snapshots/`.                                                                                                                                 |
| `Cargo.toml` member      | `"contracts/hello-world"` (previously carrying a `# Added this` comment).                                                                                                                                                      |
| `Cargo.toml` dependency  | `soroban-sdk = "28.0.0"` from `[workspace.dependencies]`. It was declared as a workspace dependency but the crate used a direct `soroban-sdk = "28.0.0"` path instead, so cargo reported it as an unused workspace dependency. |
| `Cargo.lock`             | The `hello-world` package entry and the resolved `soroban-*` / `stellar-xdr` graph it was the sole consumer of.                                                                                                                |

## Why the crate was removed rather than kept

`contracts/hello-world/src/lib.rs` was a **Soroban** contract
(`soroban_sdk::{contract, contractimpl, ...}`) — the Stellar SDK — inside a
Substrate/ink! workspace. Concretely:

- it pulled `soroban-sdk` and its dependency tree into the workspace resolution for a
  crate that no production contract, test suite or script referenced;
- clippy and `cargo check` over the workspace had to build it with a toolchain and SDK
  unrelated to the rest of the repository, which is exactly what the audit tooling is
  pointed at; and
- `docs/hello_world_deprecation.md` already declared it marked for removal, so its
  continued presence contradicted the repository's own documentation.

## Verification

- `cargo metadata` resolves 803 packages and **no package named `hello-world`**; no
  package name matches `soroban`.
- `grep -c 'soroban\|stellar-xdr\|hello-world' Cargo.lock` returns `0`.
- `cargo check --workspace` no longer emits the
  `unused workspace dependency 'soroban-sdk'` warning it used to, and no longer warns
  that `contracts/hello-world/Cargo.toml` declares profiles for a non-root package.##
  Note on the removed comment token

An earlier revision of this file contained a stray HTML comment token — an editor
artifact with no relation to the deprecation content, and one that rendered as literal
text in viewers which do not strip HTML comments. It has been removed. The file now
contains no HTML comment markers at all.
