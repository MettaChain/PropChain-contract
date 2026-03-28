//! PropChain Test Suite
//!
//! This module provides the test library for PropChain contracts,
//! including shared utilities, fixtures, and test helpers.

#![cfg_attr(not(feature = "std"), no_std)]

// Core test modules
pub mod test_utils;
pub mod load_tests;                    // Load testing framework
pub mod load_test_property_registration;  // Registration load tests
pub mod load_test_property_transfer;      // Transfer load tests
pub mod load_test_endurance_spike;        // Endurance and spike tests
pub mod load_test_scalability;            // Scalability tests

// Re-export commonly used items
pub use test_utils::*;
pub use load_tests::{LoadTestConfig, LoadTestMetrics};

// ── Security Test Modules ─────────────────────────────────────────────────
pub mod security_access_control_tests;
pub mod security_bridge_tests;
pub mod security_compliance_tests;
pub mod security_overflow_tests;
pub mod security_fuzzing_tests;
pub mod security_audit_runner;
