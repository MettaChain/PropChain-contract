//! Re-entrancy protection exported from `propchain_traits` so that
//! `propchain_contracts::ReentrancyError` and `propchain_traits::ReentrancyError`
//! are the **same type**, enabling the `map_reentrancy!` macro to work uniformly.
//!
//! Previously this file defined an independent copy of both `ReentrancyError`
//! and `ReentrancyGuard`.  By re-exporting we eliminate the duplicate, ensuring
//! that every contract uses the canonical `From<propchain_traits::ReentrancyError>`
//! impl produced by the macro.

#![cfg_attr(not(feature = "std"), no_std)]

pub use propchain_traits::reentrancy_guard::*;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_guard_creation() {
        let guard = ReentrancyGuard::new();
        assert!(!guard.is_locked());
    }

    #[test]
    fn test_enter_success() {
        let mut guard = ReentrancyGuard::new();
        assert!(guard.enter().is_ok());
        assert!(guard.is_locked());
    }

    #[test]
    fn test_reentrant_detection() {
        let mut guard = ReentrancyGuard::new();
        assert!(guard.enter().is_ok());
        // Second enter should fail
        assert_eq!(guard.enter(), Err(ReentrancyError::ReentrantCall));
    }

    #[test]
    fn test_exit_unlocks() {
        let mut guard = ReentrancyGuard::new();
        let _ = guard.enter();
        assert!(guard.is_locked());
        guard.exit();
        assert!(!guard.is_locked());
    }
}
