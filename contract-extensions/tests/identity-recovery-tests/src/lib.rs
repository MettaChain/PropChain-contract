//! Unit tests for the identity contract's creation, verification and social
//! recovery flows (#1454).
//!
//! The contract has no email / phone / wallet *attribute* API (identity data is
//! a DID document plus privacy settings), so the "attribute" items are covered
//! through the verification level an authorized verifier attaches to an identity
//! and through revocation, which is the contract's way of removing that standing.

#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(test)]
mod tests {
    use ink::env::{test, DefaultEnvironment};
    use propchain_identity::propchain_identity::{
        IdentityError, IdentityRegistry, PrivacySettings, VerificationLevel,
    };

    type Accounts = test::DefaultAccounts<DefaultEnvironment>;

    fn privacy() -> PrivacySettings {
        PrivacySettings {
            public_reputation: true,
            public_verification: true,
            data_sharing_consent: false,
            zero_knowledge_proof: false,
            selective_disclosure: Vec::new(),
        }
    }

    /// A 64-byte value passes the contract's (length-only) signature check.
    fn sig64() -> Vec<u8> {
        vec![0xAB; 64]
    }

    /// Admin (alice) deploys; `who` registers an identity.
    fn setup() -> (IdentityRegistry, Accounts) {
        let accounts = test::default_accounts::<DefaultEnvironment>();
        test::set_caller::<DefaultEnvironment>(accounts.alice);
        (IdentityRegistry::new(), accounts)
    }

    fn create(registry: &mut IdentityRegistry, caller: ink::primitives::AccountId, did: &str) {
        test::set_caller::<DefaultEnvironment>(caller);
        registry
            .create_identity(did.into(), vec![1; 32], "Ed25519".into(), None, privacy())
            .expect("identity created");
    }

    // ── Creation ────────────────────────────────────────────────────────

    #[ink::test]
    fn creates_an_identity_with_valid_parameters() {
        let (mut r, accounts) = setup();
        create(&mut r, accounts.bob, "did:prop:bob");

        let id = r.get_identity(accounts.bob).expect("stored");
        assert_eq!(id.account_id, accounts.bob);
        assert_eq!(id.did_document.did, "did:prop:bob");
        assert_eq!(id.did_document.version, 1);
        assert_eq!(id.reputation_score, 500);
        assert_eq!(id.verification_level, VerificationLevel::None);
        assert!(!id.is_verified);
        assert!(r.get_identity(accounts.charlie).is_none());
    }

    #[ink::test]
    fn rejects_a_duplicate_identity_for_the_same_account() {
        let (mut r, accounts) = setup();
        create(&mut r, accounts.bob, "did:prop:bob");
        test::set_caller::<DefaultEnvironment>(accounts.bob);
        assert_eq!(
            r.create_identity("did:prop:bob2".into(), vec![2; 32], "Ed25519".into(), None, privacy()),
            Err(IdentityError::IdentityAlreadyExists)
        );
    }

    #[ink::test]
    fn rejects_malformed_dids() {
        let (mut r, accounts) = setup();
        test::set_caller::<DefaultEnvironment>(accounts.bob);
        for bad in ["", "bob", "did:", "did:prop", "DID:prop:bob"] {
            assert_eq!(
                r.create_identity(bad.into(), vec![1; 32], "Ed25519".into(), None, privacy()),
                Err(IdentityError::InvalidDid),
                "{bad:?}"
            );
        }
        assert!(r.get_identity(accounts.bob).is_none());
    }

    // ── Verification (the contract's equivalent of attribute verification) ──

    #[ink::test]
    fn an_authorized_verifier_can_verify_an_identity() {
        let (mut r, accounts) = setup();
        create(&mut r, accounts.bob, "did:prop:bob");

        test::set_caller::<DefaultEnvironment>(accounts.alice);
        r.add_authorized_verifier(accounts.charlie).unwrap();

        test::set_caller::<DefaultEnvironment>(accounts.charlie);
        test::set_block_timestamp::<DefaultEnvironment>(10_000);
        r.verify_identity(accounts.bob, VerificationLevel::Standard, Some(30)).unwrap();

        let id = r.get_identity(accounts.bob).unwrap();
        assert!(id.is_verified);
        assert_eq!(id.verification_level, VerificationLevel::Standard);
        assert_eq!(id.verified_at, Some(10_000));
        assert_eq!(id.verification_expires, Some(10_000 + 30 * 86_400));
        assert_eq!(id.trust_score, 75);
    }

    #[ink::test]
    fn an_unauthorized_account_cannot_verify() {
        let (mut r, accounts) = setup();
        create(&mut r, accounts.bob, "did:prop:bob");
        test::set_caller::<DefaultEnvironment>(accounts.charlie);
        assert_eq!(
            r.verify_identity(accounts.bob, VerificationLevel::Basic, None),
            Err(IdentityError::Unauthorized)
        );
        assert!(!r.get_identity(accounts.bob).unwrap().is_verified);
    }

    #[ink::test]
    fn verifying_an_unknown_identity_fails_and_only_the_admin_manages_verifiers() {
        let (mut r, accounts) = setup();
        assert_eq!(
            r.verify_identity(accounts.eve, VerificationLevel::Basic, None),
            Err(IdentityError::IdentityNotFound)
        );
        test::set_caller::<DefaultEnvironment>(accounts.bob);
        assert_eq!(r.add_authorized_verifier(accounts.charlie), Err(IdentityError::Unauthorized));
    }

    #[ink::test]
    fn removing_a_verifier_takes_their_authority_away() {
        let (mut r, accounts) = setup();
        create(&mut r, accounts.bob, "did:prop:bob");
        test::set_caller::<DefaultEnvironment>(accounts.alice);
        r.add_authorized_verifier(accounts.charlie).unwrap();
        r.remove_authorized_verifier(accounts.charlie).unwrap();
        test::set_caller::<DefaultEnvironment>(accounts.charlie);
        assert_eq!(
            r.verify_identity(accounts.bob, VerificationLevel::Basic, None),
            Err(IdentityError::Unauthorized)
        );
    }

    // ── Social recovery ─────────────────────────────────────────────────

    /// Bob has an identity; alice (admin) appoints three guardians with the given threshold.
    fn with_guardians(threshold: u8) -> (IdentityRegistry, Accounts) {
        let (mut r, accounts) = setup();
        create(&mut r, accounts.bob, "did:prop:bob");
        test::set_caller::<DefaultEnvironment>(accounts.alice);
        r.set_recovery_guardians(
            accounts.bob,
            vec![accounts.charlie, accounts.django, accounts.eve],
            threshold,
        )
        .unwrap();
        (r, accounts)
    }

    #[ink::test]
    fn only_the_admin_can_set_guardians() {
        let (mut r, accounts) = setup();
        create(&mut r, accounts.bob, "did:prop:bob");
        test::set_caller::<DefaultEnvironment>(accounts.bob);
        assert_eq!(
            r.set_recovery_guardians(accounts.bob, vec![accounts.charlie], 1),
            Err(IdentityError::Unauthorized)
        );
    }

    #[ink::test]
    fn recovery_completes_once_the_guardian_threshold_is_met() {
        let (mut r, accounts) = with_guardians(2);
        test::set_caller::<DefaultEnvironment>(accounts.bob);
        r.initiate_recovery(accounts.frank, sig64()).unwrap();

        test::set_caller::<DefaultEnvironment>(accounts.charlie);
        r.approve_recovery(accounts.bob, accounts.frank).unwrap();
        // One approval of two: the identity has not moved yet.
        assert!(r.get_identity(accounts.bob).is_some());
        assert!(r.get_identity(accounts.frank).is_none());

        test::set_caller::<DefaultEnvironment>(accounts.django);
        r.approve_recovery(accounts.bob, accounts.frank).unwrap();
        assert!(r.get_identity(accounts.bob).is_none());
        let moved = r.get_identity(accounts.frank).expect("recovered to the new account");
        assert_eq!(moved.account_id, accounts.frank);
        assert!(!moved.social_recovery.is_recovery_active);
        assert_eq!(moved.did_document.did, "did:prop:bob");
    }

    #[ink::test]
    fn recovery_with_too_few_guardians_does_not_complete() {
        let (mut r, accounts) = with_guardians(3);
        test::set_caller::<DefaultEnvironment>(accounts.bob);
        r.initiate_recovery(accounts.frank, sig64()).unwrap();
        for guardian in [accounts.charlie, accounts.django] {
            test::set_caller::<DefaultEnvironment>(guardian);
            r.approve_recovery(accounts.bob, accounts.frank).unwrap();
        }
        let still = r.get_identity(accounts.bob).expect("not moved");
        assert!(still.social_recovery.is_recovery_active);
        assert_eq!(still.social_recovery.recovery_approvals.len(), 2);
        assert!(r.get_identity(accounts.frank).is_none());
    }

    #[ink::test]
    fn the_same_guardian_approving_twice_counts_once() {
        let (mut r, accounts) = with_guardians(2);
        test::set_caller::<DefaultEnvironment>(accounts.bob);
        r.initiate_recovery(accounts.frank, sig64()).unwrap();
        test::set_caller::<DefaultEnvironment>(accounts.charlie);
        r.approve_recovery(accounts.bob, accounts.frank).unwrap();
        r.approve_recovery(accounts.bob, accounts.frank).unwrap();
        assert!(r.get_identity(accounts.bob).is_some());
        assert_eq!(
            r.get_identity(accounts.bob).unwrap().social_recovery.recovery_approvals.len(),
            1
        );
    }

    #[ink::test]
    fn non_guardians_cannot_approve_recovery() {
        let (mut r, accounts) = with_guardians(1);
        test::set_caller::<DefaultEnvironment>(accounts.bob);
        r.initiate_recovery(accounts.frank, sig64()).unwrap();

        test::set_caller::<DefaultEnvironment>(accounts.alice); // admin, but not a guardian
        assert_eq!(r.approve_recovery(accounts.bob, accounts.frank), Err(IdentityError::Unauthorized));
        test::set_caller::<DefaultEnvironment>(accounts.bob); // the owner is not a guardian either
        assert_eq!(r.approve_recovery(accounts.bob, accounts.frank), Err(IdentityError::Unauthorized));
        assert!(r.get_identity(accounts.frank).is_none());
    }

    #[ink::test]
    fn guardians_cannot_approve_before_recovery_is_initiated() {
        let (mut r, accounts) = with_guardians(1);
        test::set_caller::<DefaultEnvironment>(accounts.charlie);
        assert_eq!(
            r.approve_recovery(accounts.bob, accounts.frank),
            Err(IdentityError::RecoveryNotActive)
        );
    }

    #[ink::test]
    fn recovery_needs_a_well_formed_signature_and_cannot_be_started_twice() {
        let (mut r, accounts) = with_guardians(2);
        test::set_caller::<DefaultEnvironment>(accounts.bob);
        assert_eq!(
            r.initiate_recovery(accounts.frank, vec![1, 2, 3]),
            Err(IdentityError::InvalidSignature)
        );
        r.initiate_recovery(accounts.frank, sig64()).unwrap();
        assert_eq!(
            r.initiate_recovery(accounts.frank, sig64()),
            Err(IdentityError::RecoveryInProgress)
        );
    }

    #[ink::test]
    fn an_account_without_an_identity_cannot_start_recovery() {
        let (mut r, accounts) = setup();
        test::set_caller::<DefaultEnvironment>(accounts.eve);
        assert_eq!(
            r.initiate_recovery(accounts.frank, sig64()),
            Err(IdentityError::IdentityNotFound)
        );
    }

    // ── Transfer to a new owner ─────────────────────────────────────────

    #[ink::test]
    fn an_identity_can_be_ported_to_a_new_account() {
        let (mut r, accounts) = setup();
        create(&mut r, accounts.bob, "did:prop:bob");
        test::set_caller::<DefaultEnvironment>(accounts.bob);
        r.port_identity(accounts.charlie).unwrap();

        assert!(r.get_identity(accounts.bob).is_none());
        let moved = r.get_identity(accounts.charlie).unwrap();
        assert_eq!(moved.account_id, accounts.charlie);
        assert_eq!(moved.did_document.did, "did:prop:bob");
        assert_eq!(moved.did_document.version, 2);
    }

    #[ink::test]
    fn porting_to_an_occupied_or_identical_account_is_rejected() {
        let (mut r, accounts) = setup();
        create(&mut r, accounts.bob, "did:prop:bob");
        create(&mut r, accounts.charlie, "did:prop:charlie");
        test::set_caller::<DefaultEnvironment>(accounts.bob);
        assert_eq!(r.port_identity(accounts.charlie), Err(IdentityError::IdentityAlreadyExists));
        assert_eq!(r.port_identity(accounts.bob), Err(IdentityError::IdentityAlreadyExists));
        assert!(r.get_identity(accounts.bob).is_some());
    }

    #[ink::test]
    fn porting_without_an_identity_fails() {
        let (mut r, accounts) = setup();
        test::set_caller::<DefaultEnvironment>(accounts.eve);
        assert_eq!(r.port_identity(accounts.frank), Err(IdentityError::IdentityNotFound));
    }
}
