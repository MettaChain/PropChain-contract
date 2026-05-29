#![cfg_attr(not(feature = "std"), no_std)]

use ink::prelude::string::String;

#[derive(Debug, PartialEq, Eq, scale::Encode, scale::Decode)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub enum ReentrancyError {
    ReentrantCall,
}

/// Simple mutex-based reentrancy guard (OpenZeppelin-style)
#[derive(Default)]
pub struct ReentrancyGuard {
    locked: bool,
}

impl ReentrancyGuard {
    pub fn new() -> Self {
        Self { locked: false }
    }

    /// Enter protected section
    pub fn enter(&mut self) -> Result<(), ReentrancyError> {
        if self.locked {
            return Err(ReentrancyError::ReentrantCall);
        }
        self.locked = true;
        Ok(())
    }

    /// Exit protected section
    pub fn exit(&mut self) {
        self.locked = false;
    }
}

/// Helper macro to simplify usage
#[macro_export]
macro_rules! non_reentrant {
    ($self:ident, $body:block) => {{
        $self.reentrancy_guard.enter().map_err(|_| ())?;
        let result = (|| $body)();
        $self.reentrancy_guard.exit();
        result
    }};
}

/// Optional: Gas limit wrapper for external calls
pub fn safe_external_call<F, T>(call: F, gas_limit: u64) -> Result<T, String>
where
    F: FnOnce() -> Result<T, String>,
{
    // In real ink!, gas control is limited, but we simulate safety check
    if gas_limit == 0 {
        return Err("Gas limit too low".into());
    }

    call()
}
