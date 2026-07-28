// Builder module for the contract factory.
//
// Loaded via `pub mod builder;` in `lib.rs`. The `deploy_contract` ink! message
// calls `builder::build_contract(contract_type, init_params, salt) -> Result<BuildResult, Error>`
// as part of the deterministic-deployment flow. In a production chain this
// would compute a CREATE2 address from `code_hash + salt + init_params`; here
// it returns a synthetic `BuildResult` so the factory crate compiles when
// the deeper cross-contract instantiation plumbing is not yet wired in.

use ink::prelude::vec::Vec;

/// Outcome of a builder-callable contract construction. `address` would be
/// the CREATE2-derived contract address in a real implementation.
pub struct BuildResult {
    pub address: ink::primitives::AccountId,
}

/// Errors specific to the builder (placeholder).
#[derive(Debug, PartialEq, Eq, scale::Encode, scale::Decode)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub enum BuildError {
    InvalidParams,
}

/// Stub for `build_contract` — takes the project's `ContractType` enum (a
/// `Copy + PartialEq + Eq + scale::Encode + scale::Decode` type), ignores
/// `_init_params` and `_salt`, and returns the zero `AccountId`.  Replace
/// with a real CREATE2 instantiation when the factory connects to a
/// deployer on-chain.
pub fn build_contract(
    _contract_type: super::contract_factory::ContractType,
    _init_params: Vec<u8>,
    _salt: Option<[u8; 32]>,
) -> Result<BuildResult, BuildError> {
    Ok(BuildResult {
        address: ink::primitives::AccountId::from([0u8; 32]),
    })
}
