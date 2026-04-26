use ink::prelude::vec::Vec;
use scale::Encode;

/// Deployment template for PropertyToken
pub struct PropertyTokenTemplate {
    pub admin: ink::primitives::AccountId,
    pub name: ink::prelude::string::String,
    pub symbol: ink::prelude::string::String,
}

impl PropertyTokenTemplate {
    pub fn encode_params(&self) -> Vec<u8> {
        (self.admin, self.name.clone(), self.symbol.clone()).encode()
    }
}

/// Deployment template for Escrow
pub struct EscrowTemplate {
    pub admin: ink::primitives::AccountId,
    pub fee_percentage: u128,
}

impl EscrowTemplate {
    pub fn encode_params(&self) -> Vec<u8> {
        (self.admin, self.fee_percentage).encode()
    }
}

/// Deployment template for Oracle
pub struct OracleTemplate {
    pub admin: ink::primitives::AccountId,
    pub update_interval: u64,
}

impl OracleTemplate {
    pub fn encode_params(&self) -> Vec<u8> {
        (self.admin, self.update_interval).encode()
    }
}

/// Deployment template for Bridge
pub struct BridgeTemplate {
    pub admin: ink::primitives::AccountId,
    pub validators: Vec<ink::primitives::AccountId>,
    pub threshold: u32,
}

impl BridgeTemplate {
    pub fn encode_params(&self) -> Vec<u8> {
        (self.admin, self.validators.clone(), self.threshold).encode()
    }
}

/// Deployment template for Insurance
pub struct InsuranceTemplate {
    pub admin: ink::primitives::AccountId,
    pub premium_rate: u128,
    pub coverage_limit: u128,
}

impl InsuranceTemplate {
    pub fn encode_params(&self) -> Vec<u8> {
        (self.admin, self.premium_rate, self.coverage_limit).encode()
    }
}

/// Deployment template for Governance
pub struct GovernanceTemplate {
    pub admin: ink::primitives::AccountId,
    pub voting_period: u64,
    pub quorum_percentage: u32,
}

impl GovernanceTemplate {
    pub fn encode_params(&self) -> Vec<u8> {
        (self.admin, self.voting_period, self.quorum_percentage).encode()
    }
}

/// Deployment template for DEX
pub struct DexTemplate {
    pub admin: ink::primitives::AccountId,
    pub fee_rate: u128,
}

impl DexTemplate {
    pub fn encode_params(&self) -> Vec<u8> {
        (self.admin, self.fee_rate).encode()
    }
}

/// Deployment template for Lending
pub struct LendingTemplate {
    pub admin: ink::primitives::AccountId,
    pub interest_rate: u128,
    pub collateral_ratio: u128,
}

impl LendingTemplate {
    pub fn encode_params(&self) -> Vec<u8> {
        (self.admin, self.interest_rate, self.collateral_ratio).encode()
    }
}

/// Deployment template for Crowdfunding
pub struct CrowdfundingTemplate {
    pub admin: ink::primitives::AccountId,
    pub min_contribution: u128,
    pub platform_fee: u128,
}

impl CrowdfundingTemplate {
    pub fn encode_params(&self) -> Vec<u8> {
        (self.admin, self.min_contribution, self.platform_fee).encode()
    }
}

/// Deployment template for Fractional
pub struct FractionalTemplate {
    pub admin: ink::primitives::AccountId,
    pub property_id: u64,
    pub total_shares: u128,
}

impl FractionalTemplate {
    pub fn encode_params(&self) -> Vec<u8> {
        (self.admin, self.property_id, self.total_shares).encode()
    }
}
