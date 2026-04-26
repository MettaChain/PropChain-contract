#[cfg(test)]
mod tests {
    use super::contract_factory::*;
    use ink::env::test;

    #[ink::test]
    fn test_factory_initialization() {
        let factory = ContractFactory::new();
        let accounts = test::default_accounts::<ink::env::DefaultEnvironment>();
        
        assert_eq!(factory.admin(), accounts.alice);
        assert_eq!(factory.get_deployment_count(), 0);
    }

    #[ink::test]
    fn test_set_code_hash() {
        let mut factory = ContractFactory::new();
        let code_hash: Hash = [1u8; 32].into();
        
        let result = factory.set_code_hash(ContractType::PropertyToken, code_hash);
        assert!(result.is_ok());
        
        let retrieved = factory.get_code_hash(ContractType::PropertyToken);
        assert_eq!(retrieved, Some(code_hash));
    }

    #[ink::test]
    fn test_unauthorized_set_code_hash() {
        let mut factory = ContractFactory::new();
        let accounts = test::default_accounts::<ink::env::DefaultEnvironment>();
        
        // Change caller to non-admin
        test::set_caller::<ink::env::DefaultEnvironment>(accounts.bob);
        
        let code_hash: Hash = [1u8; 32].into();
        let result = factory.set_code_hash(ContractType::PropertyToken, code_hash);
        
        assert_eq!(result, Err(Error::Unauthorized));
    }

    #[ink::test]
    fn test_change_admin() {
        let mut factory = ContractFactory::new();
        let accounts = test::default_accounts::<ink::env::DefaultEnvironment>();
        
        let result = factory.change_admin(accounts.bob);
        assert!(result.is_ok());
        assert_eq!(factory.admin(), accounts.bob);
    }

    #[ink::test]
    fn test_get_deployer_contracts_empty() {
        let factory = ContractFactory::new();
        let accounts = test::default_accounts::<ink::env::DefaultEnvironment>();
        
        let contracts = factory.get_deployer_contracts(accounts.alice);
        assert_eq!(contracts.len(), 0);
    }
}
