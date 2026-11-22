//! Adapter around the compiled third-party service registry.
//!
//! The adapter is the compatibility boundary: it validates an external
//! contract's advertised API version, registers it through the real
//! `ThirdPartyIntegration` contract, and forwards view/write calls while
//! translating external failures into adapter errors.

use ink::prelude::string::String;
use ink::prelude::vec::Vec;
use ink::primitives::AccountId;
use propchain_third_party::propchain_third_party::{
    Error as RegistryError, ServiceConfig, ServiceType, ThirdPartyIntegration,
};

use crate::mock_external::{ExternalContractError, ExternalContractInterface};

/// Only the `v1` API family is understood by this adapter.
pub const SUPPORTED_API_PREFIX: &str = "v1";

/// Reasons an integration can be refused by the adapter.
#[derive(Debug, PartialEq, Eq)]
pub enum AdapterError {
    /// `caller` is not the integrator that configured this adapter.
    UnauthorizedIntegrator,
    /// The external contract advertises an unsupported API version.
    IncompatibleContract,
    /// A view call failed on the external contract.
    ExternalCallFailed,
    /// A write call failed on the external contract (including auth failures).
    ExternalWriteRejected,
    /// The service was never registered through this adapter.
    NotRegistered,
    /// The compiled registry contract rejected the registration.
    Registry(RegistryError),
}

/// A successfully registered external contract.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExternalContractRegistration {
    pub service_id: u32,
    pub account: AccountId,
    pub api_version: String,
    pub compatible: bool,
}

/// Check whether an external contract version can be wrapped safely.
pub fn is_compatible(api_version: &str) -> bool {
    api_version.starts_with(SUPPORTED_API_PREFIX) && !api_version.contains("legacy")
}

/// Compatibility gate and call-forwarding layer for third-party contracts.
pub struct ThirdPartyAdapter {
    registry: ThirdPartyIntegration,
    integrator: AccountId,
    registrations: Vec<ExternalContractRegistration>,
}

impl ThirdPartyAdapter {
    /// Create an adapter. The ink! caller must already be set to `integrator`.
    pub fn new(integrator: AccountId) -> Self {
        Self {
            registry: ThirdPartyIntegration::new(),
            integrator,
            registrations: Vec::new(),
        }
    }

    /// Register an external contract after checking authorization and
    /// compatibility. The registration itself is performed by the compiled
    /// `ThirdPartyIntegration` contract, so it emits the same event an
    /// indexer sees on chain.
    pub fn add_external_contract(
        &mut self,
        caller: AccountId,
        service_type: ServiceType,
        name: String,
        external: &dyn ExternalContractInterface,
        fee_percentage: u16,
    ) -> Result<u32, AdapterError> {
        if caller != self.integrator {
            return Err(AdapterError::UnauthorizedIntegrator);
        }

        let api_version = external.api_version();
        if !is_compatible(&api_version) {
            return Err(AdapterError::IncompatibleContract);
        }

        let service_id = self
            .registry
            .register_service(
                service_type,
                name,
                external.owner(),
                api_version.clone(),
                api_version.clone(),
                fee_percentage,
            )
            .map_err(AdapterError::Registry)?;

        self.registrations.push(ExternalContractRegistration {
            service_id,
            account: external.owner(),
            api_version,
            compatible: true,
        });
        Ok(service_id)
    }

    /// Forward a read-only call to a registered external contract.
    pub fn view(
        &self,
        service_id: u32,
        external: &dyn ExternalContractInterface,
        key: u64,
    ) -> Result<u128, AdapterError> {
        self.ensure_registered(service_id)?;
        match external.view_value(key) {
            Ok(value) => Ok(value),
            Err(ExternalContractError::UnsupportedVersion) => Err(AdapterError::IncompatibleContract),
            Err(_) => Err(AdapterError::ExternalCallFailed),
        }
    }

    /// Forward a mutating call to a registered external contract.
    pub fn write(
        &self,
        service_id: u32,
        external: &mut dyn ExternalContractInterface,
        caller: AccountId,
        key: u64,
        value: u128,
    ) -> Result<(), AdapterError> {
        self.ensure_registered(service_id)?;
        external
            .write_value(key, value, caller)
            .map_err(|_| AdapterError::ExternalWriteRejected)
    }

    /// Whether `service_id` was registered through this adapter.
    pub fn is_registered(&self, service_id: u32) -> bool {
        self.registrations
            .iter()
            .any(|registration| registration.service_id == service_id)
    }

    pub fn registrations(&self) -> &[ExternalContractRegistration] {
        &self.registrations
    }

    /// Read the service config directly from the compiled registry contract.
    pub fn service_config(&self, service_id: u32) -> Option<ServiceConfig> {
        self.registry.get_service_config(service_id)
    }

    /// Access the compiled registry contract (used to assert emitted events).
    pub fn registry(&self) -> &ThirdPartyIntegration {
        &self.registry
    }

    fn ensure_registered(&self, service_id: u32) -> Result<(), AdapterError> {
        if self.is_registered(service_id) {
            Ok(())
        } else {
            Err(AdapterError::NotRegistered)
        }
    }
}
