use ink::prelude::string::String;
use ink::prelude::vec::Vec;
use scale::{Decode, Encode};

/// Errors that can occur when validating a deployment template.
#[derive(Debug, Encode, Decode, PartialEq, Eq, Clone)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub enum TemplateError {
    EmptyName,
    EmptySymbol,
    NameTooLong,
    SymbolTooLong,
    PercentageOutOfRange,
    ZeroValidators,
    ThresholdExceedsValidators,
    ZeroThreshold,
    DuplicateValidators,
    ZeroInterval,
}

/// Common behavior every deployment template must provide.
///
/// Keeping this as a trait (rather than just `encode_params` per struct)
/// means callers can validate-then-encode through one interface instead of
/// hand-rolling the same two calls for every contract type.
pub trait DeploymentTemplate {
    /// SCALE-encode the constructor parameters, in the exact order the
    /// target contract's constructor expects them.
    fn encode_params(&self) -> Vec<u8>;

    /// Sanity-check the parameters before they're ever encoded/deployed.
    /// Default impl is a no-op so templates with no extra constraints
    /// don't need to implement this.
    fn validate(&self) -> Result<(), TemplateError> {
        Ok(())
    }

    /// Convenience: validate, then encode. This is what most callers
    /// should actually use.
    fn try_encode_params(&self) -> Result<Vec<u8>, TemplateError> {
        self.validate()?;
        Ok(self.encode_params())
    }
}

const MAX_NAME_LEN: usize = 64;
const MAX_SYMBOL_LEN: usize = 12;

fn validate_percentage(value: u32, max: u32) -> Result<(), TemplateError> {
    if value > max {
        Err(TemplateError::PercentageOutOfRange)
    } else {
        Ok(())
    }
}

fn validate_name_symbol(name: &str, symbol: &str) -> Result<(), TemplateError> {
    if name.is_empty() {
        return Err(TemplateError::EmptyName);
    }
    if name.len() > MAX_NAME_LEN {
        return Err(TemplateError::NameTooLong);
    }
    if symbol.is_empty() {
        return Err(TemplateError::EmptySymbol);
    }
    if symbol.len() > MAX_SYMBOL_LEN {
        return Err(TemplateError::SymbolTooLong);
    }
    Ok(())
}

/// Deployment template for PropertyToken
#[derive(Debug, Encode, Decode, Clone)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub struct PropertyTokenTemplate {
    pub admin: ink::primitives::AccountId,
    pub name: String,
    pub symbol: String,
}

impl DeploymentTemplate for PropertyTokenTemplate {
    fn encode_params(&self) -> Vec<u8> {
        (self.admin, self.name.clone(), self.symbol.clone()).encode()
    }

    fn validate(&self) -> Result<(), TemplateError> {
        validate_name_symbol(&self.name, &self.symbol)
    }
}

/// Deployment template for Escrow
#[derive(Debug, Encode, Decode, Clone)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub struct EscrowTemplate {
    pub admin: ink::primitives::AccountId,
    /// Basis points (0 - 10_000) rather than a raw percentage, to allow
    /// fractional percentages (e.g. 250 = 2.50%).
    pub fee_bps: u32,
}

impl DeploymentTemplate for EscrowTemplate {
    fn encode_params(&self) -> Vec<u8> {
        (self.admin, self.fee_bps).encode()
    }

    fn validate(&self) -> Result<(), TemplateError> {
        validate_percentage(self.fee_bps, 10_000)
    }
}

/// Deployment template for Oracle
#[derive(Debug, Encode, Decode, Clone)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub struct OracleTemplate {
    pub admin: ink::primitives::AccountId,
    pub update_interval: u64,
}

impl DeploymentTemplate for OracleTemplate {
    fn encode_params(&self) -> Vec<u8> {
        (self.admin, self.update_interval).encode()
    }

    fn validate(&self) -> Result<(), TemplateError> {
        if self.update_interval == 0 {
            Err(TemplateError::ZeroInterval)
        } else {
            Ok(())
        }
    }
}

/// Deployment template for Bridge
#[derive(Debug, Encode, Decode, Clone)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub struct BridgeTemplate {
    pub admin: ink::primitives::AccountId,
    pub validators: Vec<ink::primitives::AccountId>,
    pub threshold: u32,
}

impl DeploymentTemplate for BridgeTemplate {
    fn encode_params(&self) -> Vec<u8> {
        (self.admin, self.validators.clone(), self.threshold).encode()
    }

    fn validate(&self) -> Result<(), TemplateError> {
        if self.validators.is_empty() {
            return Err(TemplateError::ZeroValidators);
        }
        if self.threshold == 0 {
            return Err(TemplateError::ZeroThreshold);
        }
        if (self.threshold as usize) > self.validators.len() {
            return Err(TemplateError::ThresholdExceedsValidators);
        }
        let mut sorted = self.validators.clone();
        sorted.sort();
        if sorted.windows(2).any(|w| w[0] == w[1]) {
            return Err(TemplateError::DuplicateValidators);
        }
        Ok(())
    }
}

/// Deployment template for Insurance
#[derive(Debug, Encode, Decode, Clone)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub struct InsuranceTemplate {
    pub admin: ink::primitives::AccountId,
    pub premium_rate: u128,
    pub coverage_limit: u128,
}

impl DeploymentTemplate for InsuranceTemplate {
    fn encode_params(&self) -> Vec<u8> {
        (self.admin, self.premium_rate, self.coverage_limit).encode()
    }

    fn validate(&self) -> Result<(), TemplateError> {
        if self.coverage_limit == 0 {
            return Err(TemplateError::ZeroInterval); // reuse not ideal; see note below
        }
        Ok(())
    }
}

/// Deployment template for Governance
#[derive(Debug, Encode, Decode, Clone)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub struct GovernanceTemplate {
    pub admin: ink::primitives::AccountId,
    pub voting_period: u64,
    pub quorum_percentage: u32,
}

impl DeploymentTemplate for GovernanceTemplate {
    fn encode_params(&self) -> Vec<u8> {
        (self.admin, self.voting_period, self.quorum_percentage).encode()
    }

    fn validate(&self) -> Result<(), TemplateError> {
        if self.voting_period == 0 {
            return Err(TemplateError::ZeroInterval);
        }
        validate_percentage(self.quorum_percentage, 100)
    }
}

/// Deployment template for DEX
#[derive(Debug, Encode, Decode, Clone)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub struct DexTemplate {
    pub admin: ink::primitives::AccountId,
    pub fee_bps: u32,
}

impl DeploymentTemplate for DexTemplate {
    fn encode_params(&self) -> Vec<u8> {
        (self.admin, self.fee_bps).encode()
    }

    fn validate(&self) -> Result<(), TemplateError> {
        validate_percentage(self.fee_bps, 10_000)
    }
}

/// Deployment template for Lending
#[derive(Debug, Encode, Decode, Clone)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub struct LendingTemplate {
    pub admin: ink::primitives::AccountId,
    pub interest_rate: u128,
    pub collateral_ratio: u128,
}

impl DeploymentTemplate for LendingTemplate {
    fn encode_params(&self) -> Vec<u8> {
        (self.admin, self.interest_rate, self.collateral_ratio).encode()
    }

    fn validate(&self) -> Result<(), TemplateError> {
        if self.collateral_ratio == 0 {
            return Err(TemplateError::ZeroInterval);
        }
        Ok(())
    }
}

/// Deployment template for Crowdfunding
#[derive(Debug, Encode, Decode, Clone)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub struct CrowdfundingTemplate {
    pub admin: ink::primitives::AccountId,
    pub min_contribution: u128,
    pub platform_fee_bps: u32,
}

impl DeploymentTemplate for CrowdfundingTemplate {
    fn encode_params(&self) -> Vec<u8> {
        (self.admin, self.min_contribution, self.platform_fee_bps).encode()
    }

    fn validate(&self) -> Result<(), TemplateError> {
        validate_percentage(self.platform_fee_bps, 10_000)
    }
}

/// Deployment template for Fractional
#[derive(Debug, Encode, Decode, Clone)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub struct FractionalTemplate {
    pub admin: ink::primitives::AccountId,
    pub property_id: u64,
    pub total_shares: u128,
}

impl DeploymentTemplate for FractionalTemplate {
    fn encode_params(&self) -> Vec<u8> {
        (self.admin, self.property_id, self.total_shares).encode()
    }

    fn validate(&self) -> Result<(), TemplateError> {
        if self.total_shares == 0 {
            return Err(TemplateError::ZeroInterval);
        }
        Ok(())
    }
}

/// A type-erased wrapper over every template kind, for cases where you need
/// to store/dispatch templates generically (e.g. a deployment registry
/// keyed by contract type) without paying for `dyn Trait` + heap allocation
/// in an ink! contract.
#[derive(Debug, Encode, Decode, Clone)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub enum ContractTemplate {
    PropertyToken(PropertyTokenTemplate),
    Escrow(EscrowTemplate),
    Oracle(OracleTemplate),
    Bridge(BridgeTemplate),
    Insurance(InsuranceTemplate),
    Governance(GovernanceTemplate),
    Dex(DexTemplate),
    Lending(LendingTemplate),
    Crowdfunding(CrowdfundingTemplate),
    Fractional(FractionalTemplate),
}

impl ContractTemplate {
    pub fn admin(&self) -> ink::primitives::AccountId {
        match self {
            ContractTemplate::PropertyToken(t) => t.admin,
            ContractTemplate::Escrow(t) => t.admin,
            ContractTemplate::Oracle(t) => t.admin,
            ContractTemplate::Bridge(t) => t.admin,
            ContractTemplate::Insurance(t) => t.admin,
            ContractTemplate::Governance(t) => t.admin,
            ContractTemplate::Dex(t) => t.admin,
            ContractTemplate::Lending(t) => t.admin,
            ContractTemplate::Crowdfunding(t) => t.admin,
            ContractTemplate::Fractional(t) => t.admin,
        }
    }

    pub fn validate(&self) -> Result<(), TemplateError> {
        match self {
            ContractTemplate::PropertyToken(t) => t.validate(),
            ContractTemplate::Escrow(t) => t.validate(),
            ContractTemplate::Oracle(t) => t.validate(),
            ContractTemplate::Bridge(t) => t.validate(),
            ContractTemplate::Insurance(t) => t.validate(),
            ContractTemplate::Governance(t) => t.validate(),
            ContractTemplate::Dex(t) => t.validate(),
            ContractTemplate::Lending(t) => t.validate(),
            ContractTemplate::Crowdfunding(t) => t.validate(),
            ContractTemplate::Fractional(t) => t.validate(),
        }
    }

    pub fn encode_params(&self) -> Vec<u8> {
        match self {
            ContractTemplate::PropertyToken(t) => t.encode_params(),
            ContractTemplate::Escrow(t) => t.encode_params(),
            ContractTemplate::Oracle(t) => t.encode_params(),
            ContractTemplate::Bridge(t) => t.encode_params(),
            ContractTemplate::Insurance(t) => t.encode_params(),
            ContractTemplate::Governance(t) => t.encode_params(),
            ContractTemplate::Dex(t) => t.encode_params(),
            ContractTemplate::Lending(t) => t.encode_params(),
            ContractTemplate::Crowdfunding(t) => t.encode_params(),
            ContractTemplate::Fractional(t) => t.encode_params(),
        }
    }

    pub fn try_encode_params(&self) -> Result<Vec<u8>, TemplateError> {
        self.validate()?;
        Ok(self.encode_params())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn acc(byte: u8) -> ink::primitives::AccountId {
        ink::primitives::AccountId::from([byte; 32])
    }

    #[test]
    fn property_token_rejects_empty_symbol() {
        let t = PropertyTokenTemplate {
            admin: acc(1),
            name: "Lekki Tower".into(),
            symbol: "".into(),
        };
        assert_eq!(t.validate(), Err(TemplateError::EmptySymbol));
    }

    #[test]
    fn bridge_rejects_threshold_above_validator_count() {
        let t = BridgeTemplate {
            admin: acc(1),
            validators: vec![acc(2), acc(3)],
            threshold: 3,
        };
        assert_eq!(t.validate(), Err(TemplateError::ThresholdExceedsValidators));
    }

    #[test]
    fn bridge_rejects_duplicate_validators() {
        let t = BridgeTemplate {
            admin: acc(1),
            validators: vec![acc(2), acc(2)],
            threshold: 1,
        };
        assert_eq!(t.validate(), Err(TemplateError::DuplicateValidators));
    }

    #[test]
    fn dex_rejects_fee_over_100_percent() {
        let t = DexTemplate {
            admin: acc(1),
            fee_bps: 10_001,
        };
        assert_eq!(t.validate(), Err(TemplateError::PercentageOutOfRange));
    }

    #[test]
    fn encode_params_matches_tuple_order() {
        let t = OracleTemplate {
            admin: acc(1),
            update_interval: 600,
        };
        assert_eq!(t.encode_params(), (t.admin, t.update_interval).encode());
    }
}
