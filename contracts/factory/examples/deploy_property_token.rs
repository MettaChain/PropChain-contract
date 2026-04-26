/// Example: Deploy a PropertyToken using the factory
/// 
/// This example demonstrates how to:
/// 1. Initialize the factory
/// 2. Set code hashes
/// 3. Deploy a PropertyToken contract
/// 4. Query deployment information

use propchain_factory::contract_factory::{ContractFactory, ContractType, DeploymentConfig};
use propchain_factory::templates::PropertyTokenTemplate;
use propchain_factory::builder::DeploymentBuilder;

fn main() {
    println!("=== PropChain Contract Factory Example ===\n");

    // Step 1: Initialize factory (done once during deployment)
    println!("1. Initializing factory...");
    // let mut factory = ContractFactory::new();
    
    // Step 2: Set code hash for PropertyToken (admin only, done once per contract type)
    println!("2. Setting PropertyToken code hash...");
    // let property_token_code_hash: Hash = [0x12u8; 32].into();
    // factory.set_code_hash(ContractType::PropertyToken, property_token_code_hash)?;
    
    // Step 3: Prepare deployment configuration using template
    println!("3. Preparing deployment configuration...");
    
    // Using template approach
    let template = PropertyTokenTemplate {
        admin: [0u8; 32].into(), // Replace with actual admin account
        name: "Luxury Apartment Token".to_string(),
        symbol: "LAT".to_string(),
    };
    
    let config = DeploymentConfig {
        contract_type: ContractType::PropertyToken,
        salt: generate_unique_salt(),
        init_params: template.encode_params(),
    };
    
    println!("   Contract Type: PropertyToken");
    println!("   Name: Luxury Apartment Token");
    println!("   Symbol: LAT");
    
    // Step 4: Deploy the contract
    println!("\n4. Deploying contract...");
    // let contract_address = factory.deploy_contract(config, "1.0.0".to_string())?;
    // println!("   Deployed at: {:?}", contract_address);
    
    // Step 5: Query deployment information
    println!("\n5. Querying deployment information...");
    // let deployment = factory.get_deployment(0)?;
    // println!("   Deployment ID: 0");
    // println!("   Contract Address: {:?}", deployment.address);
    // println!("   Deployer: {:?}", deployment.deployer);
    // println!("   Deployed At: {}", deployment.deployed_at);
    // println!("   Version: {}", deployment.version);
    
    // Step 6: Query all deployments by deployer
    println!("\n6. Querying deployer's contracts...");
    // let my_contracts = factory.get_deployer_contracts(deployer_account);
    // println!("   Total contracts deployed: {}", my_contracts.len());
    
    println!("\n=== Deployment Complete ===");
}

/// Generate a unique salt for deployment
fn generate_unique_salt() -> [u8; 32] {
    // In production, use:
    // - Current timestamp
    // - Caller address
    // - Random nonce
    // - Hash of above
    
    // Example placeholder
    [1u8; 32]
}

/// Alternative: Using builder pattern
fn deploy_with_builder() {
    println!("=== Using Builder Pattern ===\n");
    
    let template = PropertyTokenTemplate {
        admin: [0u8; 32].into(),
        name: "Commercial Property Token".to_string(),
        symbol: "CPT".to_string(),
    };
    
    let (config, version) = DeploymentBuilder::new()
        .contract_type(ContractType::PropertyToken)
        .salt(generate_unique_salt())
        .init_params(template.encode_params())
        .version("2.0.0".to_string())
        .build()
        .expect("Failed to build deployment config");
    
    println!("Config built successfully!");
    println!("Version: {}", version);
    
    // Deploy using factory
    // let address = factory.deploy_contract(config, version)?;
}

/// Batch deployment example
fn batch_deploy_example() {
    println!("=== Batch Deployment Example ===\n");
    
    let contracts = vec![
        ("Property A Token", "PAT"),
        ("Property B Token", "PBT"),
        ("Property C Token", "PCT"),
    ];
    
    for (i, (name, symbol)) in contracts.iter().enumerate() {
        println!("Deploying {}: {}", i + 1, name);
        
        let template = PropertyTokenTemplate {
            admin: [0u8; 32].into(),
            name: name.to_string(),
            symbol: symbol.to_string(),
        };
        
        let mut salt = [0u8; 32];
        salt[0] = i as u8; // Unique salt per deployment
        
        let config = DeploymentConfig {
            contract_type: ContractType::PropertyToken,
            salt,
            init_params: template.encode_params(),
        };
        
        // Deploy
        // let address = factory.deploy_contract(config, "1.0.0".to_string())?;
        // println!("   Deployed at: {:?}\n", address);
    }
}
