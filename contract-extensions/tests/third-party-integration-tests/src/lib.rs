//! Integration tests for third-party adapters and external contract
//! interfaces (issue #1535).
//!
//! The compiled `propchain-third-party` contract is used as a path dependency,
//! so compatibility checks, registration and event emission are exercised
//! against real contract code. All files live under `contract-extensions/`.

#![cfg_attr(not(feature = "std"), no_std)]

pub mod adapter;
pub mod mock_external;

#[cfg(test)]
mod tests;
