//! Top-level verification module.
//!
//! Exposes the Kani proof harnesses declared under
//! [`invariants`] so that the formal-verification CI workflow
//! (`.github/workflows/formal-verification.yml`) can resolve
//! harnesses like `balance_proofs::prove_balance_conservation`.
//!
//! Gated to `cfg(kani)` to mirror the `#[cfg(kani)] mod verification;`
//! declaration in `lib.rs`, so this file is only compiled under Kani.
#![cfg(kani)]

pub mod invariants;
