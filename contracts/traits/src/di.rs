//! Dependency Injection framework for PropChain contracts.
//!
//! # Overview
//!
//! This module provides a lightweight, `no_std`-compatible DI framework
//! designed for ink! smart contracts. Rather than a runtime container
//! (impossible on-chain), it offers:
//!
//! - [`ServiceKey`] — a typed enum identifying every injectable service.
//! - [`ServiceRegistry`] — an ink! trait that any contract can implement to
//!   expose its registered service addresses.
//! - [`ContainerConfig`] — a plain storage struct that holds all optional
//!   `AccountId` service addresses and is embedded directly in contract storage.
//! - [`Injectable`] — a marker trait for contracts that accept injected deps.
//! - [`DependencyError`] — unified error type for DI operations.
//!
//! # Design rationale
//!
//! On-chain DI cannot use heap-allocated vtables or dynamic dispatch the way
//! server-side frameworks do. Instead, each service is identified by a
//! `ServiceKey` variant and resolved to an `Option<AccountId>` at call time.
//! Cross-contract calls are then made via ink!'s `CallBuilder` using the
//! resolved address, keeping coupling at the *address* level rather than the
//! *type* level.
//!
//! # Usage
//!
//! ```rust,ignore
//! // 1. Embed ContainerConfig in your contract storage:
//! #[ink(storage)]
//! pub struct MyContract {
//!     deps: ContainerConfig,
//!     // ...
//! }
//!
//! // 2. Register services during construction or via admin setter:
//! deps.register(ServiceKey::Oracle, oracle_address);
//!
//! // 3. Resolve at call time:
//! let oracle_addr = deps.resolve(ServiceKey::Oracle)?;
//! ```

#![cfg_attr(not(feature = "std"), no_std)]

use ink::primitives::AccountId;

// =========================================================================
// Error Type
// =========================================================================

/// Errors that can occur during dependency injection operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, scale::Encode, scale::Decode)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub enum DependencyError {
    /// The requested service has not been registered.
    ServiceNotRegistered,
    /// Caller is not authorised to modify the service registry.
    Unauthorized,
    /// The provided address is the zero address.
    InvalidAddress,
}

impl core::fmt::Display for DependencyError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            DependencyError::ServiceNotRegistered => {
                write!(f, "Service not registered in the dependency container")
            }
            DependencyError::Unauthorized => {
                write!(f, "Caller is not authorised to modify the service registry")
            }
            DependencyError::InvalidAddress => {
                write!(f, "Provided address is the zero address")
            }
        }
    }
}

// =========================================================================
// Service Key
// =========================================================================

/// Identifies every injectable service in the PropChain ecosystem.
///
/// Add a new variant here whenever a new cross-contract dependency is
/// introduced. The variant name doubles as documentation for what the
/// service does.
#[derive(Debug, Clone, Copy, PartialEq, Eq, scale::Encode, scale::Decode)]
#[cfg_attr(
    feature = "std",
    derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
)]
pub enum ServiceKey {
    /// Property valuation oracle (implements [`Oracle`] trait).
    Oracle,
    /// Regulatory compliance checker (implements [`ComplianceChecker`] trait).
    ComplianceRegistry,
    /// Dynamic fee provider (implements [`DynamicFeeProvider`] trait).
    FeeManager,
    /// Identity / KYC registry.
    IdentityRegistry,
    /// Property management workflow contract.
    PropertyManagement,
    /// Cross-chain bridge contract.
    Bridge,
    /// Insurance pool contract.
    Insurance,
    /// Governance / multi-sig contract.
    Governance,
}

// =========================================================================
// ContainerConfig — embeddable storage struct
// =========================================================================

/// Holds all optional service `AccountId`s for a contract.
///
/// Embed this struct directly in your `#[ink(storage)]` struct. It is
/// intentionally flat (no `Mapping`) so that the full config can be read
/// in a single storage access.
///
/// All fields are `Option<AccountId>` — `None` means "not yet registered".
/// Use [`ContainerConfig::register`] / [`ContainerConfig::unregister`] to
/// mutate, and [`ContainerConfig::resolve`] to look up at call time.
#[ink::storage_item]
#[derive(Debug, Default)]
pub struct ContainerConfig {
    /// Valuation oracle address.
    pub oracle: Option<AccountId>,
    /// Compliance registry address.
    pub compliance_registry: Option<AccountId>,
    /// Fee manager address.
    pub fee_manager: Option<AccountId>,
    /// Identity registry address.
    pub identity_registry: Option<AccountId>,
    /// Property management contract address.
    pub property_management: Option<AccountId>,
    /// Bridge contract address.
    pub bridge: Option<AccountId>,
    /// Insurance contract address.
    pub insurance: Option<AccountId>,
    /// Governance contract address.
    pub governance: Option<AccountId>,
}

impl ContainerConfig {
    /// Create a new, empty container (all services unregistered).
    pub fn new() -> Self {
        Self::default()
    }

    /// Register (or replace) a service address.
    ///
    /// Returns `Err(DependencyError::InvalidAddress)` if `address` is the
    /// all-zeros account (a common mistake when passing uninitialized values).
    pub fn register(&mut self, key: ServiceKey, address: AccountId) -> Result<(), DependencyError> {
        if address == AccountId::from([0u8; 32]) {
            return Err(DependencyError::InvalidAddress);
        }
        match key {
            ServiceKey::Oracle => self.oracle = Some(address),
            ServiceKey::ComplianceRegistry => self.compliance_registry = Some(address),
            ServiceKey::FeeManager => self.fee_manager = Some(address),
            ServiceKey::IdentityRegistry => self.identity_registry = Some(address),
            ServiceKey::PropertyManagement => self.property_management = Some(address),
            ServiceKey::Bridge => self.bridge = Some(address),
            ServiceKey::Insurance => self.insurance = Some(address),
            ServiceKey::Governance => self.governance = Some(address),
        }
        Ok(())
    }

    /// Unregister a service (sets it back to `None`).
    pub fn unregister(&mut self, key: ServiceKey) {
        match key {
            ServiceKey::Oracle => self.oracle = None,
            ServiceKey::ComplianceRegistry => self.compliance_registry = None,
            ServiceKey::FeeManager => self.fee_manager = None,
            ServiceKey::IdentityRegistry => self.identity_registry = None,
            ServiceKey::PropertyManagement => self.property_management = None,
            ServiceKey::Bridge => self.bridge = None,
            ServiceKey::Insurance => self.insurance = None,
            ServiceKey::Governance => self.governance = None,
        }
    }

    /// Resolve a service address.
    ///
    /// Returns `Ok(AccountId)` if registered, or
    /// `Err(DependencyError::ServiceNotRegistered)` otherwise.
    pub fn resolve(&self, key: ServiceKey) -> Result<AccountId, DependencyError> {
        let opt = match key {
            ServiceKey::Oracle => self.oracle,
            ServiceKey::ComplianceRegistry => self.compliance_registry,
            ServiceKey::FeeManager => self.fee_manager,
            ServiceKey::IdentityRegistry => self.identity_registry,
            ServiceKey::PropertyManagement => self.property_management,
            ServiceKey::Bridge => self.bridge,
            ServiceKey::Insurance => self.insurance,
            ServiceKey::Governance => self.governance,
        };
        opt.ok_or(DependencyError::ServiceNotRegistered)
    }

    /// Returns `true` if the given service is currently registered.
    pub fn is_registered(&self, key: ServiceKey) -> bool {
        self.resolve(key).is_ok()
    }

    /// Returns a snapshot of all registered services as `(ServiceKey, AccountId)` pairs.
    ///
    /// Useful for admin dashboards and off-chain indexers.
    pub fn list_registered(&self) -> ink::prelude::vec::Vec<(ServiceKey, AccountId)> {
        let mut out = ink::prelude::vec::Vec::new();
        let keys = [
            ServiceKey::Oracle,
            ServiceKey::ComplianceRegistry,
            ServiceKey::FeeManager,
            ServiceKey::IdentityRegistry,
            ServiceKey::PropertyManagement,
            ServiceKey::Bridge,
            ServiceKey::Insurance,
            ServiceKey::Governance,
        ];
        for key in keys {
            if let Ok(addr) = self.resolve(key) {
                out.push((key, addr));
            }
        }
        out
    }
}

// =========================================================================
// ServiceRegistry ink! trait — implement on any contract that exposes DI
// =========================================================================

/// ink! trait for contracts that expose a service registry.
///
/// Implement this on your contract to provide a standard interface for
/// registering, unregistering, and resolving service dependencies.
/// Admin-gating of `register_service` / `unregister_service` is the
/// responsibility of the implementing contract.
#[ink::trait_definition]
pub trait ServiceRegistry {
    /// Register a service address for the given key.
    ///
    /// Only callable by the contract admin.
    #[ink(message)]
    fn register_service(
        &mut self,
        key: ServiceKey,
        address: AccountId,
    ) -> Result<(), DependencyError>;

    /// Unregister a service.
    ///
    /// Only callable by the contract admin.
    #[ink(message)]
    fn unregister_service(&mut self, key: ServiceKey) -> Result<(), DependencyError>;

    /// Resolve a service address by key.
    ///
    /// Returns `Err(DependencyError::ServiceNotRegistered)` if not set.
    #[ink(message)]
    fn resolve_service(&self, key: ServiceKey) -> Result<AccountId, DependencyError>;

    /// Returns `true` if the service is currently registered.
    #[ink(message)]
    fn is_service_registered(&self, key: ServiceKey) -> bool;
}

// =========================================================================
// Injectable marker trait
// =========================================================================

/// Marker trait for contracts that accept injected dependencies.
///
/// Implementing this trait signals that the contract uses `ContainerConfig`
/// internally and honours the `ServiceRegistry` interface. No methods are
/// required — it exists purely for documentation and potential blanket impls.
pub trait Injectable: ServiceRegistry {}
