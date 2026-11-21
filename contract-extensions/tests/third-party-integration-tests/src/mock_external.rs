//! Test double for an external contract interface.
//!
//! The adapter under test only depends on this small trait, mirroring how a
//! real cross-contract call is wrapped by an ink! wrapper contract. The mock is
//! deliberately deterministic: no randomness, no clock and no global state.

use ink::prelude::string::String;
use ink::prelude::vec::Vec;
use ink::primitives::AccountId;

/// Errors an external contract may return through its public interface.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExternalContractError {
    /// The external contract rejected the call at application level.
    ContractReturnedError,
    /// The caller is not the owner of the external contract.
    Unauthorized,
    /// The requested API version is not served by this contract.
    UnsupportedVersion,
}

/// Minimal ABI shared by every external contract the adapter can wrap.
pub trait ExternalContractInterface {
    /// Semantic version advertised by the external contract (for example `v1.2`).
    fn api_version(&self) -> String;
    /// Account allowed to perform mutating calls.
    fn owner(&self) -> AccountId;
    /// Read-only call against the external contract.
    fn view_value(&self, key: u64) -> Result<u128, ExternalContractError>;
    /// Mutating call against the external contract.
    fn write_value(
        &mut self,
        key: u64,
        value: u128,
        caller: AccountId,
    ) -> Result<(), ExternalContractError>;
}

/// Deterministic implementation of [`ExternalContractInterface`].
#[derive(Clone)]
pub struct MockExternalContract {
    api_version: String,
    owner: AccountId,
    entries: Vec<(u64, u128)>,
    fail_view: bool,
    fail_write: bool,
}

impl MockExternalContract {
    pub fn new(owner: AccountId, api_version: &str) -> Self {
        Self {
            api_version: String::from(api_version),
            owner,
            entries: Vec::new(),
            fail_view: false,
            fail_write: false,
        }
    }

    /// Make every view call fail with [`ExternalContractError::ContractReturnedError`].
    pub fn fail_next_view(&mut self) {
        self.fail_view = true;
    }

    /// Make every write call fail with [`ExternalContractError::ContractReturnedError`].
    pub fn fail_next_write(&mut self) {
        self.fail_write = true;
    }
}

impl ExternalContractInterface for MockExternalContract {
    fn api_version(&self) -> String {
        self.api_version.clone()
    }

    fn owner(&self) -> AccountId {
        self.owner
    }

    fn view_value(&self, key: u64) -> Result<u128, ExternalContractError> {
        if self.fail_view {
            return Err(ExternalContractError::ContractReturnedError);
        }
        Ok(self
            .entries
            .iter()
            .find(|(stored_key, _)| *stored_key == key)
            .map(|(_, value)| *value)
            .unwrap_or(0))
    }

    fn write_value(
        &mut self,
        key: u64,
        value: u128,
        caller: AccountId,
    ) -> Result<(), ExternalContractError> {
        if self.fail_write {
            return Err(ExternalContractError::ContractReturnedError);
        }
        if caller != self.owner {
            return Err(ExternalContractError::Unauthorized);
        }
        if let Some(entry) = self.entries.iter_mut().find(|(stored_key, _)| *stored_key == key) {
            entry.1 = value;
        } else {
            self.entries.push((key, value));
        }
        Ok(())
    }
}
