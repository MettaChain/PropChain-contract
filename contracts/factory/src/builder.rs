use crate::contract_factory::{ContractType, DeploymentConfig};
use ink::prelude::string::String;

/// Builder pattern for contract deployment
pub struct DeploymentBuilder {
    contract_type: Option<ContractType>,
    salt: [u8; 32],
    init_params: Vec<u8>,
    version: String,
}

impl DeploymentBuilder {
    pub fn new() -> Self {
        Self {
            contract_type: None,
            salt: [0u8; 32],
            init_params: Vec::new(),
            version: String::from("1.0.0"),
        }
    }

    pub fn contract_type(mut self, contract_type: ContractType) -> Self {
        self.contract_type = Some(contract_type);
        self
    }

    pub fn salt(mut self, salt: [u8; 32]) -> Self {
        self.salt = salt;
        self
    }

    pub fn init_params(mut self, params: Vec<u8>) -> Self {
        self.init_params = params;
        self
    }

    pub fn version(mut self, version: String) -> Self {
        self.version = version;
        self
    }

    pub fn build(self) -> Result<(DeploymentConfig, String), &'static str> {
        let contract_type = self.contract_type.ok_or("Contract type not set")?;

        Ok((
            DeploymentConfig {
                contract_type,
                salt: self.salt,
                init_params: self.init_params,
            },
            self.version,
        ))
    }
}

impl Default for DeploymentBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builder_pattern() {
        let builder = DeploymentBuilder::new()
            .contract_type(ContractType::PropertyToken)
            .salt([1u8; 32])
            .version(String::from("2.0.0"));

        let result = builder.build();
        assert!(result.is_ok());

        let (config, version) = result.unwrap();
        assert_eq!(config.contract_type, ContractType::PropertyToken);
        assert_eq!(config.salt, [1u8; 32]);
        assert_eq!(version, "2.0.0");
    }

    #[test]
    fn test_builder_missing_contract_type() {
        let builder = DeploymentBuilder::new().salt([1u8; 32]);

        let result = builder.build();
        assert!(result.is_err());
    }
}
