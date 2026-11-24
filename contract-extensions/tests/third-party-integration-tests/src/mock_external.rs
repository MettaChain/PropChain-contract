
//! Test double for an external contract interface.
//!
//! The adapter under test only depends on this small trait, mirroring how a
//! real cross-contract call is wrapped by an ink! wrapper contract. The mock
//! remains deterministic: no randomness, clock, or global state is involved.

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
    /// The requested API version is not supported.
    UnsupportedVersion,
}

/// Minimal ABI shared by every external contract the adapter can wrap.
pub trait ExternalContractInterface {
    /// Returns the semantic API version advertised by the contract.
    fn api_version(&self) -> String;

    /// Returns the account authorized to perform mutating calls.
    fn owner(&self) -> AccountId;

    /// Reads a value associated with the supplied key.
    fn view_value(&self, key: u64) -> Result<u128, ExternalContractError>;

    /// Writes a value associated with the supplied key.
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
    /// Creates a new mock external contract.
    pub fn new(owner: AccountId, api_version: &str) -> Self {
        Self {
            api_version: String::from(api_version),
            owner,
            entries: Vec::new(),
            fail_view: false,
            fail_write: false,
        }
    }

    /// Configures all subsequent view calls to return a contract error.
    pub fn fail_next_view(&mut self) {
        self.fail_view = true;
    }

    /// Configures all subsequent write calls to return a contract error.
    pub fn fail_next_write(&mut self) {
        self.fail_write = true;
    }

    /// Finds an entry by key and returns its position.
    fn find_entry(&self, key: u64) -> Option<usize> {
        self.entries
            .iter()
            .position(|(stored_key, _)| *stored_key == key)
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
            .find_entry(key)
            .map(|index| self.entries[index].1)
            .unwrap_or_default())
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

        match self.find_entry(key) {
            Some(index) => self.entries[index].1 = value,
            None => self.entries.push((key, value)),
        }

        Ok(())
    }
}
