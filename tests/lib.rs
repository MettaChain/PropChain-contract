//! PropChain Test Suite
//!
//! This module provides the test library for PropChain contracts,
//! including shared utilities, fixtures, and test helpers.

#![cfg_attr(not(feature = "std"), no_std)]

// Core test modules
pub mod bridge_load_tests;
pub mod load_tests;
pub mod tax_compliance;
pub mod test_utils; // Load testing framework

// Re-export commonly used items
pub use load_tests::{LoadTestConfig, LoadTestMetrics};
pub use test_utils::*;

// ── Security Test Modules ─────────────────────────────────────────────────
pub mod security_access_control_tests;
pub mod security_audit_runner;
pub mod security_bridge_tests;
pub mod security_compliance_tests;
pub mod security_fuzzing_tests;
pub mod security_overflow_tests;
