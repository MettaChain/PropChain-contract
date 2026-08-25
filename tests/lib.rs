#![allow(clippy::clone_on_copy)] // fires inside ink! generated storage code
//! PropChain Test Suite
//!
//! This module provides the test library for PropChain contracts,
//! including shared utilities, fixtures, and test helpers.

#![cfg_attr(not(feature = "std"), no_std)]

// Core test modules
pub mod bridge_load_tests;
pub mod test_utils; // Load testing framework

// Re-export commonly used items
pub use test_utils::*;

// ─── Security Test Modules ───────────────────────────────────────────
pub mod security_audit_runner;

// ─── Regression Test Suite ───────────────────────────────────────────
/// Issue #487: Regression test suite for all previously fixed bugs
pub mod regression;

// ─── Integration Test Modules ─────────────────────────────────────────
/// Issue #1014: Compliance registry integration coverage
pub mod integration_compliance;
/// Issues #1006 / #1007: Contract factory and IPFS metadata registry tests
pub mod integration_factory_ipfs;
/// Issue #1008: Fractional share trading integration tests
pub mod integration_fractional;
/// Issue #1005: GDPR consent management integration tests
pub mod integration_gdpr;
/// Issue #1002: Governance integration coverage (signers → proposal →
/// votes → timelock → execution)
pub mod integration_governance;
/// Issue #1001: Insurance integration coverage (policy lifecycle, claims,
/// admin/oracle authorization paths)
pub mod integration_insurance;
/// Issue #1010: Mock oracle integration tests
pub mod integration_mock_oracle;
/// Issues #1003 / #1004: Monitoring and sanctions screening integration
/// coverage (admin surface, pause gating; sanctioned entity/property flows)
pub mod integration_monitoring_sanctions;
/// Issue #1013: Third-party registry integration coverage
pub mod integration_third_party;
