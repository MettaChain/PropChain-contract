#![cfg(test)]

use ink::env::test::{default_accounts, DefaultAccounts};
use ink::primitives::AccountId;
use propchain_identity::propchain_identity::{
    IdentityError, IdentityRegistry, PrivacySettings, VerificationLevel,
};
use propchain_traits::ChainId;

#[ink::test]
fn test_create_identity() {
    let accounts: DefaultAccounts<ink::env::DefaultEnvironment> = default_accounts();
    let mut identity_registry = IdentityRegistry::new();

    let did = "did:example:123456789abcdefghi".to_string();
    let public_key = vec![1u8; 32]; // Mock public key
    let verification_method = "Ed25519VerificationKey2018".to_string();
    let service_endpoint = Some("https://example.com/identity".to_string());

    let privacy_settings = PrivacySettings {
        public_reputation: true,
        public_verification: true,
        data_sharing_consent: true,
        zero_knowledge_proof: false,
        selective_disclosure: vec![],
    };

    // Create identity should succeed
    assert_eq!(
        identity_registry.create_identity(
            did.clone(),
            public_key.clone(),
            verification_method.clone(),
            service_endpoint.clone(),
            privacy_settings.clone()
        ),
        Ok(())
    );

    // Verify identity was created
    let identity = identity_registry.get_identity(accounts.alice).unwrap();
    assert_eq!(identity.did_document.did, did);
    assert_eq!(identity.did_document.public_key, public_key);
    assert_eq!(
        identity.did_document.verification_method,
        verification_method
    );
    assert_eq!(identity.did_document.service_endpoint, service_endpoint);
    assert_eq!(identity.reputation_score, 500); // Default starting reputation
    assert_eq!(identity.verification_level, VerificationLevel::None);
    assert!(!identity.is_verified);
}

#[ink::test]
fn test_create_identity_already_exists() {
    let accounts: DefaultAccounts<ink::env::DefaultEnvironment> = default_accounts();
    let mut identity_registry = IdentityRegistry::new();

    let did = "did:example:123456789abcdefghi".to_string();
    let public_key = vec![1u8; 32];
    let verification_method = "Ed25519VerificationKey2018".to_string();
    let privacy_settings = PrivacySettings {
        public_reputation: true,
        public_verification: true,
        data_sharing_consent: true,
        zero_knowledge_proof: false,
        selective_disclosure: vec![],
    };

    // Create identity first time
    assert_eq!(
        identity_registry.create_identity(
            did.clone(),
            public_key.clone(),
            verification_method.clone(),
            None,
            privacy_settings.clone()
        ),
        Ok(())
    );

    // Creating identity again should fail
    assert_eq!(
        identity_registry.create_identity(
            did.clone(),
            public_key.clone(),
            verification_method.clone(),
            None,
            privacy_settings.clone()
        ),
        Err(IdentityError::IdentityAlreadyExists)
    );
}

#[ink::test]
fn test_invalid_did_format() {
    let mut identity_registry = IdentityRegistry::new();

    let invalid_did = "invalid-did-format".to_string();
    let public_key = vec![1u8; 32];
    let verification_method = "Ed25519VerificationKey2018".to_string();
    let privacy_settings = PrivacySettings {
        public_reputation: true,
        public_verification: true,
        data_sharing_consent: true,
        zero_knowledge_proof: false,
        selective_disclosure: vec![],
    };

    // Creating identity with invalid DID should fail
    assert_eq!(
        identity_registry.create_identity(
            invalid_did,
            public_key,
            verification_method,
            None,
            privacy_settings
        ),
        Err(IdentityError::InvalidDid)
    );
}

#[ink::test]
fn test_verify_identity() {
    let accounts: DefaultAccounts<ink::env::DefaultEnvironment> = default_accounts();
    let mut identity_registry = IdentityRegistry::new();

    // Set caller to bob before creating identity
    ink::env::test::set_caller::<ink::env::DefaultEnvironment>(accounts.bob);

    // First create an identity
    let did = "did:example:123456789abcdefghi".to_string();
    let public_key = vec![1u8; 32];
    let verification_method = "Ed25519VerificationKey2018".to_string();
    let privacy_settings = PrivacySettings {
        public_reputation: true,
        public_verification: true,
        data_sharing_consent: true,
        zero_knowledge_proof: false,
        selective_disclosure: vec![],
    };

    assert_eq!(
        identity_registry.create_identity(
            did,
            public_key,
            verification_method,
            None,
            privacy_settings
        ),
        Ok(())
    );

    // Add alice as authorized verifier (alice is admin)
    ink::env::test::set_caller::<ink::env::DefaultEnvironment>(accounts.alice);
    assert_eq!(
        identity_registry.add_authorized_verifier(accounts.alice),
        Ok(())
    );

    // Set caller as alice for verification
    ink::env::test::set_caller::<ink::env::DefaultEnvironment>(accounts.alice);

    // Verify identity with standard level
    assert_eq!(
        identity_registry.verify_identity(
            accounts.bob,
            VerificationLevel::Standard,
            Some(365) // 1 year expiry
        ),
        Ok(())
    );

    // Check verification was applied
    let identity = identity_registry.get_identity(accounts.bob).unwrap();
    assert_eq!(identity.verification_level, VerificationLevel::Standard);
    assert!(identity.is_verified);
    assert!(identity.verified_at.is_some());
    assert!(identity.verification_expires.is_some());
    assert_eq!(identity.trust_score, 75); // Standard verification gives 75 trust score
}

#[ink::test]
fn test_unauthorized_verification() {
    let accounts: DefaultAccounts<ink::env::DefaultEnvironment> = default_accounts();
    let mut identity_registry = IdentityRegistry::new();

    // Create identity
    let did = "did:example:123456789abcdefghi".to_string();
    let public_key = vec![1u8; 32];
    let verification_method = "Ed25519VerificationKey2018".to_string();
    let privacy_settings = PrivacySettings {
        public_reputation: true,
        public_verification: true,
        data_sharing_consent: true,
        zero_knowledge_proof: false,
        selective_disclosure: vec![],
    };

    assert_eq!(
        identity_registry.create_identity(
            did,
            public_key,
            verification_method,
            None,
            privacy_settings
        ),
        Ok(())
    );

    // Try to verify without authorization should fail
    // Set caller to charlie (non-admin, non-authorized)
    ink::env::test::set_caller::<ink::env::DefaultEnvironment>(accounts.charlie);
    assert_eq!(
        identity_registry.verify_identity(accounts.bob, VerificationLevel::Standard, Some(365)),
        Err(IdentityError::Unauthorized)
    );
}

#[ink::test]
fn test_update_reputation() {
    let accounts: DefaultAccounts<ink::env::DefaultEnvironment> = default_accounts();
    let mut identity_registry = IdentityRegistry::new();

    // Set caller to bob before creating identity
    ink::env::test::set_caller::<ink::env::DefaultEnvironment>(accounts.bob);

    // Create identity
    let did = "did:example:123456789abcdefghi".to_string();
    let public_key = vec![1u8; 32];
    let verification_method = "Ed25519VerificationKey2018".to_string();
    let privacy_settings = PrivacySettings {
        public_reputation: true,
        public_verification: true,
        data_sharing_consent: true,
        zero_knowledge_proof: false,
        selective_disclosure: vec![],
    };

    assert_eq!(
        identity_registry.create_identity(
            did,
            public_key,
            verification_method,
            None,
            privacy_settings
        ),
        Ok(())
    );

    // Add alice as authorized verifier (alice is admin)
    ink::env::test::set_caller::<ink::env::DefaultEnvironment>(accounts.alice);
    assert_eq!(
        identity_registry.add_authorized_verifier(accounts.alice),
        Ok(())
    );

    // Set caller as alice for reputation update
    ink::env::test::set_caller::<ink::env::DefaultEnvironment>(accounts.alice);

    let initial_reputation = identity_registry
        .get_identity(accounts.bob)
        .unwrap()
        .reputation_score;

    // Update reputation for successful transaction
    assert_eq!(
        identity_registry.update_reputation(accounts.bob, true, 1000000),
        Ok(())
    );

    let updated_reputation = identity_registry
        .get_identity(accounts.bob)
        .unwrap()
        .reputation_score;
    assert_eq!(updated_reputation, initial_reputation + 5);

    // Update reputation for failed transaction
    assert_eq!(
        identity_registry.update_reputation(accounts.bob, false, 1000000),
        Ok(())
    );

    let final_reputation = identity_registry
        .get_identity(accounts.bob)
        .unwrap()
        .reputation_score;
    assert_eq!(final_reputation, updated_reputation - 10);
}

#[ink::test]
fn test_assess_trust() {
    let accounts: DefaultAccounts<ink::env::DefaultEnvironment> = default_accounts();
    let mut identity_registry = IdentityRegistry::new();

    // Set caller to bob before creating identity
    ink::env::test::set_caller::<ink::env::DefaultEnvironment>(accounts.bob);

    // Create identity for bob
    let did = "did:example:123456789abcdefghi".to_string();
    let public_key = vec![1u8; 32];
    let verification_method = "Ed25519VerificationKey2018".to_string();
    let privacy_settings = PrivacySettings {
        public_reputation: true,
        public_verification: true,
        data_sharing_consent: true,
        zero_knowledge_proof: false,
        selective_disclosure: vec![],
    };

    assert_eq!(
        identity_registry.create_identity(
            did,
            public_key,
            verification_method,
            None,
            privacy_settings
        ),
        Ok(())
    );

    // Assess trust from alice's perspective
    let trust_assessment = identity_registry.assess_trust(accounts.bob).unwrap();

    assert_eq!(trust_assessment.target_account, accounts.bob);
    assert!(trust_assessment.trust_score >= 0 && trust_assessment.trust_score <= 100);
    assert_eq!(trust_assessment.verification_level, VerificationLevel::None);
    assert_eq!(trust_assessment.reputation_score, 500); // Default reputation
}

#[ink::test]
fn test_cross_chain_verification() {
    let accounts: DefaultAccounts<ink::env::DefaultEnvironment> = default_accounts();
    let mut identity_registry = IdentityRegistry::new();

    // Set caller to bob before creating identity
    ink::env::test::set_caller::<ink::env::DefaultEnvironment>(accounts.bob);

    // Create identity
    let did = "did:example:123456789abcdefghi".to_string();
    let public_key = vec![1u8; 32];
    let verification_method = "Ed25519VerificationKey2018".to_string();
    let privacy_settings = PrivacySettings {
        public_reputation: true,
        public_verification: true,
        data_sharing_consent: true,
        zero_knowledge_proof: false,
        selective_disclosure: vec![],
    };

    assert_eq!(
        identity_registry.create_identity(
            did,
            public_key,
            verification_method,
            None,
            privacy_settings
        ),
        Ok(())
    );

    let chain_id = 1; // Ethereum
    let verification_hash = [1u8; 32].into();
    let chain_reputation_score = 750;

    // Add cross-chain verification
    assert_eq!(
        identity_registry.add_cross_chain_verification(
            chain_id,
            verification_hash,
            chain_reputation_score
        ),
        Ok(())
    );

    // Check cross-chain verification was added
    let cross_chain_verification = identity_registry
        .get_cross_chain_verification(accounts.bob, chain_id)
        .unwrap();
    assert_eq!(cross_chain_verification.chain_id, chain_id);
    assert_eq!(
        cross_chain_verification.verification_hash,
        verification_hash
    );
    assert_eq!(
        cross_chain_verification.reputation_score,
        chain_reputation_score
    );
    assert!(cross_chain_verification.is_active);

    // Check that reputation was updated (average of local and chain reputation)
    let identity = identity_registry.get_identity(accounts.bob).unwrap();
    assert_eq!(identity.reputation_score, (500 + 750) / 2);
}

#[ink::test]
fn test_unsupported_chain() {
    let accounts: DefaultAccounts<ink::env::DefaultEnvironment> = default_accounts();
    let mut identity_registry = IdentityRegistry::new();

    // Create identity
    let did = "did:example:123456789abcdefghi".to_string();
    let public_key = vec![1u8; 32];
    let verification_method = "Ed25519VerificationKey2018".to_string();
    let privacy_settings = PrivacySettings {
        public_reputation: true,
        public_verification: true,
        data_sharing_consent: true,
        zero_knowledge_proof: false,
        selective_disclosure: vec![],
    };

    assert_eq!(
        identity_registry.create_identity(
            did,
            public_key,
            verification_method,
            None,
            privacy_settings
        ),
        Ok(())
    );

    let unsupported_chain_id = 999;
    let verification_hash = [1u8; 32].into();

    // Adding verification for unsupported chain should fail
    assert_eq!(
        identity_registry.add_cross_chain_verification(
            unsupported_chain_id,
            verification_hash,
            750
        ),
        Err(IdentityError::UnsupportedChain)
    );
}

#[ink::test]
fn test_social_recovery_initiation() {
    let accounts: DefaultAccounts<ink::env::DefaultEnvironment> = default_accounts();
    let mut identity_registry = IdentityRegistry::new();

    // Set caller to bob before creating identity
    ink::env::test::set_caller::<ink::env::DefaultEnvironment>(accounts.bob);

    // Create identity
    let did = "did:example:123456789abcdefghi".to_string();
    let public_key = vec![1u8; 32];
    let verification_method = "Ed25519VerificationKey2018".to_string();
    let privacy_settings = PrivacySettings {
        public_reputation: true,
        public_verification: true,
        data_sharing_consent: true,
        zero_knowledge_proof: false,
        selective_disclosure: vec![],
    };

    assert_eq!(
        identity_registry.create_identity(
            did,
            public_key,
            verification_method,
            None,
            privacy_settings
        ),
        Ok(())
    );

    let new_account = AccountId::from([2u8; 32]);
    let recovery_signature = vec![1u8; 64]; // Mock signature

    // Initiate recovery
    assert_eq!(
        identity_registry.initiate_recovery(new_account, recovery_signature),
        Ok(())
    );

    // Check recovery was initiated
    let identity = identity_registry.get_identity(accounts.bob).unwrap();
    assert!(identity.social_recovery.is_recovery_active);
    assert!(identity.social_recovery.last_recovery_attempt.is_some());
}

#[ink::test]
fn test_privacy_preserving_verification() {
    let accounts: DefaultAccounts<ink::env::DefaultEnvironment> = default_accounts();
    let mut identity_registry = IdentityRegistry::new();

    // Create identity with privacy settings enabled
    let did = "did:example:123456789abcdefghi".to_string();
    let public_key = vec![1u8; 32];
    let verification_method = "Ed25519VerificationKey2018".to_string();
    let privacy_settings = PrivacySettings {
        public_reputation: true,
        public_verification: true,
        data_sharing_consent: true,
        zero_knowledge_proof: true, // Enable ZK proofs
        selective_disclosure: vec![],
    };

    assert_eq!(
        identity_registry.create_identity(
            did,
            public_key,
            verification_method,
            None,
            privacy_settings
        ),
        Ok(())
    );

    let proof = vec![1u8; 32];
    let public_inputs = vec![2u8; 16];
    let verification_type = "identity_proof".to_string();

    // Privacy-preserving verification should succeed
    assert_eq!(
        identity_registry.verify_privacy_preserving(proof, public_inputs, verification_type),
        Ok(true)
    );
}

#[ink::test]
fn test_privacy_verification_failed() {
    let accounts: DefaultAccounts<ink::env::DefaultEnvironment> = default_accounts();
    let mut identity_registry = IdentityRegistry::new();

    // Create identity with privacy settings disabled
    let did = "did:example:123456789abcdefghi".to_string();
    let public_key = vec![1u8; 32];
    let verification_method = "Ed25519VerificationKey2018".to_string();
    let privacy_settings = PrivacySettings {
        public_reputation: true,
        public_verification: true,
        data_sharing_consent: true,
        zero_knowledge_proof: false, // Disable ZK proofs
        selective_disclosure: vec![],
    };

    assert_eq!(
        identity_registry.create_identity(
            did,
            public_key,
            verification_method,
            None,
            privacy_settings
        ),
        Ok(())
    );

    let proof = vec![1u8; 32];
    let public_inputs = vec![2u8; 16];
    let verification_type = "identity_proof".to_string();

    // Privacy-preserving verification should fail
    assert_eq!(
        identity_registry.verify_privacy_preserving(proof, public_inputs, verification_type),
        Err(IdentityError::PrivacyVerificationFailed)
    );
}

#[ink::test]
fn test_reputation_threshold_check() {
    let accounts: DefaultAccounts<ink::env::DefaultEnvironment> = default_accounts();
    let mut identity_registry = IdentityRegistry::new();

    // Set caller to bob before creating identity
    ink::env::test::set_caller::<ink::env::DefaultEnvironment>(accounts.bob);

    // Create identity
    let did = "did:example:123456789abcdefghi".to_string();
    let public_key = vec![1u8; 32];
    let verification_method = "Ed25519VerificationKey2018".to_string();
    let privacy_settings = PrivacySettings {
        public_reputation: true,
        public_verification: true,
        data_sharing_consent: true,
        zero_knowledge_proof: false,
        selective_disclosure: vec![],
    };

    assert_eq!(
        identity_registry.create_identity(
            did,
            public_key,
            verification_method,
            None,
            privacy_settings
        ),
        Ok(())
    );

    // Check with threshold below current reputation (500)
    assert!(identity_registry.meets_reputation_threshold(accounts.bob, 400));

    // Check with threshold above current reputation
    assert!(!identity_registry.meets_reputation_threshold(accounts.bob, 600));
}

#[ink::test]
fn test_admin_functions() {
    let accounts: DefaultAccounts<ink::env::DefaultEnvironment> = default_accounts();

    // Set caller to non-admin (bob) before creating contract
    ink::env::test::set_caller::<ink::env::DefaultEnvironment>(accounts.bob);
    let mut identity_registry = IdentityRegistry::new();

    // Test with charlie as non-admin caller
    ink::env::test::set_caller::<ink::env::DefaultEnvironment>(accounts.charlie);

    // Only admin can add authorized verifiers
    assert_eq!(
        identity_registry.add_authorized_verifier(accounts.charlie),
        Err(IdentityError::Unauthorized)
    );

    // Set caller as admin (alice)
    ink::env::test::set_caller::<ink::env::DefaultEnvironment>(accounts.alice);
    let mut identity_registry = IdentityRegistry::new();

    // Now admin can add authorized verifiers
    assert_eq!(
        identity_registry.add_authorized_verifier(accounts.bob),
        Ok(())
    );

    // Admin can add supported chains
    assert_eq!(identity_registry.add_supported_chain(999), Ok(()));

    // Check supported chains
    let supported_chains = identity_registry.get_supported_chains();
    assert!(supported_chains.contains(&999));
}
