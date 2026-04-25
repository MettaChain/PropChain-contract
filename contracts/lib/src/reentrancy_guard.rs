#![cfg_attr(not(feature = "std"), no_std)]

/// Error type for reentrancy protection
#[derive(Debug, PartialEq, Eq, scale::Encode, scale::Decode)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub enum ReentrancyError {
    /// Attempt to call a protected function while already in a protected call
    ReentrantCall,
}

/// Simple mutex-based reentrancy guard (OpenZeppelin-style)
///
/// This guard prevents reentrancy attacks by tracking whether we're currently
/// in the middle of a protected operation. If a reentrancy attempt is detected,
/// the guard returns an error.
///
/// # Example
/// ```ignore
/// non_reentrant!(self, {
///     // This code cannot be reentered
///     self.env().transfer(recipient, amount)?;
///     state_update();
/// })
/// ```
#[derive(Default, Debug, Clone, PartialEq, Eq, scale::Encode, scale::Decode)]
#[cfg_attr(
    feature = "std",
    derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
)]
pub struct ReentrancyGuard {
    locked: bool,
}

impl ReentrancyGuard {
    /// Create a new reentrancy guard
    pub fn new() -> Self {
        Self { locked: false }
    }

    /// Enter a protected section
    ///
    /// Returns Ok(()) if we're not currently locked, or Err(ReentrancyError::ReentrantCall)
    /// if a reentrancy attempt is detected.
    pub fn enter(&mut self) -> Result<(), ReentrancyError> {
        if self.locked {
            return Err(ReentrancyError::ReentrantCall);
        }
        self.locked = true;
        Ok(())
    }

    /// Exit a protected section
    ///
    /// This must always be called after enter(), typically via the non_reentrant! macro.
    pub fn exit(&mut self) {
        self.locked = false;
    }

    /// Check if currently locked without modifying state
    pub fn is_locked(&self) -> bool {
        self.locked
    }
}

/// Macro to simplify reentrancy protection usage
///
/// # Example
/// ```ignore
/// #[ink(message)]
/// pub fn transfer_and_update(&mut self, to: AccountId, amount: u128) -> Result<(), Error> {
///     non_reentrant!(self, {
///         // Check conditions first
///         if self.balance < amount {
///             return Err(Error::InsufficientBalance);
///         }
///         
///         // Transfer (external call)
///         self.env().transfer(to, amount)?;
///         
///         // Update state after transfer
///         self.balance -= amount;
///         self.emit_event();
///         
///         Ok(())
///     })
/// }
/// ```
#[macro_export]
macro_rules! non_reentrant {
    ($self:ident, $body:block) => {{
        $self.reentrancy_guard.enter()?;
        let result = (|| $body)();
        $self.reentrancy_guard.exit();
        result
    }};
}

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
