#![cfg_attr(not(feature = "std"), no_std, no_main)]
#![allow(clippy::new_without_default)]

//! # Verifiable credentials (#1438)
//!
//! Issuers attach credentials to identity holders; anyone can check a
//! credential's validity on-chain. A credential stores only a **hash** of its
//! off-chain data (the W3C-style document), the issuer, the subject, a type and
//! an expiry block.
//!
//! ## Status
//!
//! | status | meaning |
//! |---|---|
//! | `Valid` | issued, not revoked, and (if it has an expiry) `now <= expiration_block` |
//! | `Expired` | `now > expiration_block` |
//! | `Revoked` | revoked by its issuer (revocation wins over expiry) |
//! | `NotFound` | no such credential |
//!
//! `expiration_block == 0` means the credential never expires.
//!
//! ## Roles
//!
//! The admin appoints **issuers**. Only the issuer of a credential may revoke it
//! (not even the admin), as the issue requires. The contract does not call the
//! `identity` contract on-chain: the subject is any account, and checking that
//! it holds an identity is left to the issuer's off-chain process or the
//! integration layer.
//!
//! `verify_credential` is a read-only view. Because a view cannot emit events,
//! `verify_and_log` is provided for callers that want a `CredentialVerified`
//! event on-chain.

#[ink::contract]
pub mod verifiable_credentials {
    use ink::prelude::vec::Vec;
    use ink::storage::Mapping;

    /// Most credentials one subject can hold.
    pub const MAX_CREDENTIALS_PER_SUBJECT: usize = 64;

    #[derive(Debug, Clone, Copy, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout))]
    pub enum CredentialType {
        Kyc,
        PropertyOwnership,
        AccreditedInvestor,
        InsuranceActive,
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub enum CredentialStatus {
        Valid,
        Expired,
        Revoked,
        NotFound,
    }

    #[derive(Debug, Clone, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout))]
    pub struct Credential {
        pub id: u64,
        pub issuer: AccountId,
        pub subject: AccountId,
        pub credential_type: CredentialType,
        pub data_hash: Hash,
        pub issued_at: u32,
        /// `0` = never expires.
        pub expiration_block: u32,
        pub revoked: bool,
    }

    /// Result of verifying a credential.
    #[derive(Debug, Clone, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub struct Verification {
        pub status: CredentialStatus,
        pub issuer: Option<AccountId>,
        pub credential_type: Option<CredentialType>,
        pub subject: Option<AccountId>,
    }

    #[derive(Debug, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub enum Error {
        Unauthorized,
        NotAnIssuer,
        CredentialNotFound,
        AlreadyRevoked,
        /// `expiration_block` is non-zero but not after the current block.
        InvalidExpiration,
        TooManyCredentials,
    }

    #[ink(event)]
    pub struct CredentialIssued {
        #[ink(topic)]
        pub credential_id: u64,
        #[ink(topic)]
        pub issuer: AccountId,
        #[ink(topic)]
        pub subject: AccountId,
        pub credential_type: CredentialType,
    }

    #[ink(event)]
    pub struct CredentialVerified {
        #[ink(topic)]
        pub credential_id: u64,
        pub verifier: AccountId,
        pub valid: bool,
    }

    #[ink(event)]
    pub struct CredentialRevoked {
        #[ink(topic)]
        pub credential_id: u64,
        #[ink(topic)]
        pub issuer: AccountId,
    }

    #[ink(storage)]
    pub struct VerifiableCredentials {
        admin: AccountId,
        issuers: Mapping<AccountId, bool>,
        credentials: Mapping<u64, Credential>,
        by_subject: Mapping<AccountId, Vec<u64>>,
        next_id: u64,
    }

    impl VerifiableCredentials {
        #[ink(constructor)]
        pub fn new() -> Self {
            Self {
                admin: Self::env().caller(),
                issuers: Mapping::default(),
                credentials: Mapping::default(),
                by_subject: Mapping::default(),
                next_id: 1,
            }
        }

        // ── Administration ─────────────────────────────────────────────

        #[ink(message)]
        pub fn set_issuer(&mut self, issuer: AccountId, allowed: bool) -> Result<(), Error> {
            if self.env().caller() != self.admin {
                return Err(Error::Unauthorized);
            }
            if allowed {
                self.issuers.insert(issuer, &true);
            } else {
                self.issuers.remove(issuer);
            }
            Ok(())
        }

        #[ink(message)]
        pub fn is_issuer(&self, account: AccountId) -> bool {
            self.issuers.get(account).unwrap_or(false)
        }

        // ── Issuing and revoking ───────────────────────────────────────

        /// Issue a credential to `subject`. Issuers only.
        #[ink(message)]
        pub fn issue_credential(
            &mut self,
            subject: AccountId,
            credential_type: CredentialType,
            data_hash: Hash,
            expiration_block: u32,
        ) -> Result<u64, Error> {
            let issuer = self.env().caller();
            if !self.is_issuer(issuer) {
                return Err(Error::NotAnIssuer);
            }
            let now = self.env().block_number();
            if expiration_block != 0 && expiration_block <= now {
                return Err(Error::InvalidExpiration);
            }
            let mut ids = self.by_subject.get(subject).unwrap_or_default();
            if ids.len() >= MAX_CREDENTIALS_PER_SUBJECT {
                return Err(Error::TooManyCredentials);
            }
            let id = self.next_id;
            self.next_id += 1;
            self.credentials.insert(
                id,
                &Credential {
                    id,
                    issuer,
                    subject,
                    credential_type,
                    data_hash,
                    issued_at: now,
                    expiration_block,
                    revoked: false,
                },
            );
            ids.push(id);
            self.by_subject.insert(subject, &ids);
            self.env().emit_event(CredentialIssued { credential_id: id, issuer, subject, credential_type });
            Ok(id)
        }

        /// Revoke a credential. Only its issuer may.
        #[ink(message)]
        pub fn revoke_credential(&mut self, credential_id: u64) -> Result<(), Error> {
            let mut c = self.credentials.get(credential_id).ok_or(Error::CredentialNotFound)?;
            let caller = self.env().caller();
            if c.issuer != caller {
                return Err(Error::Unauthorized);
            }
            if c.revoked {
                return Err(Error::AlreadyRevoked);
            }
            c.revoked = true;
            self.credentials.insert(credential_id, &c);
            self.env().emit_event(CredentialRevoked { credential_id, issuer: caller });
            Ok(())
        }

        // ── Verification ───────────────────────────────────────────────

        /// Validity, issuer, type and subject of a credential. Read-only.
        #[ink(message)]
        pub fn verify_credential(&self, credential_id: u64) -> Verification {
            match self.credentials.get(credential_id) {
                None => Verification { status: CredentialStatus::NotFound, issuer: None, credential_type: None, subject: None },
                Some(c) => Verification {
                    status: self.status_of(&c),
                    issuer: Some(c.issuer),
                    credential_type: Some(c.credential_type),
                    subject: Some(c.subject),
                },
            }
        }

        /// Like `verify_credential`, and records a `CredentialVerified` event.
        #[ink(message)]
        pub fn verify_and_log(&mut self, credential_id: u64) -> Verification {
            let result = self.verify_credential(credential_id);
            if result.status != CredentialStatus::NotFound {
                self.env().emit_event(CredentialVerified {
                    credential_id,
                    verifier: self.env().caller(),
                    valid: result.status == CredentialStatus::Valid,
                });
            }
            result
        }

        /// Whether `subject` holds a currently valid credential of `credential_type`.
        #[ink(message)]
        pub fn has_valid_credential(&self, subject: AccountId, credential_type: CredentialType) -> bool {
            self.by_subject
                .get(subject)
                .unwrap_or_default()
                .iter()
                .filter_map(|id| self.credentials.get(id))
                .any(|c| c.credential_type == credential_type && self.status_of(&c) == CredentialStatus::Valid)
        }

        #[ink(message)]
        pub fn get_credential(&self, credential_id: u64) -> Option<Credential> {
            self.credentials.get(credential_id)
        }

        /// Every credential ever issued to `subject` (including revoked and expired ones).
        #[ink(message)]
        pub fn get_credentials(&self, subject: AccountId) -> Vec<Credential> {
            self.by_subject
                .get(subject)
                .unwrap_or_default()
                .iter()
                .filter_map(|id| self.credentials.get(id))
                .collect()
        }

        fn status_of(&self, c: &Credential) -> CredentialStatus {
            if c.revoked {
                CredentialStatus::Revoked
            } else if c.expiration_block != 0 && self.env().block_number() > c.expiration_block {
                CredentialStatus::Expired
            } else {
                CredentialStatus::Valid
            }
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use ink::env::test;

        type Accounts = test::DefaultAccounts<ink::env::DefaultEnvironment>;

        fn hash(b: u8) -> Hash {
            Hash::from([b; 32])
        }

        fn block(n: u32) {
            test::set_block_number::<ink::env::DefaultEnvironment>(n);
        }

        /// Admin = alice; bob is an issuer; charlie holds credentials.
        fn setup() -> (VerifiableCredentials, Accounts) {
            let a = test::default_accounts::<ink::env::DefaultEnvironment>();
            test::set_caller::<ink::env::DefaultEnvironment>(a.alice);
            block(10);
            let mut v = VerifiableCredentials::new();
            v.set_issuer(a.bob, true).unwrap();
            (v, a)
        }

        fn issue(v: &mut VerifiableCredentials, a: &Accounts, kind: CredentialType, expires: u32) -> u64 {
            test::set_caller::<ink::env::DefaultEnvironment>(a.bob);
            v.issue_credential(a.charlie, kind, hash(1), expires).unwrap()
        }

        // ── Issuing ─────────────────────────────────────────────────────

        #[ink::test]
        fn an_issuer_issues_a_credential_with_the_given_fields() {
            let (mut v, a) = setup();
            let id = issue(&mut v, &a, CredentialType::Kyc, 100);
            let c = v.get_credential(id).unwrap();
            assert_eq!(
                (c.issuer, c.subject, c.credential_type, c.data_hash, c.issued_at, c.expiration_block, c.revoked),
                (a.bob, a.charlie, CredentialType::Kyc, hash(1), 10, 100, false)
            );
            assert_eq!(id, 1);
        }

        #[ink::test]
        fn only_appointed_issuers_can_issue() {
            let (mut v, a) = setup();
            test::set_caller::<ink::env::DefaultEnvironment>(a.eve);
            assert_eq!(
                v.issue_credential(a.charlie, CredentialType::Kyc, hash(1), 0),
                Err(Error::NotAnIssuer)
            );
            // Even the admin is not an issuer unless appointed.
            test::set_caller::<ink::env::DefaultEnvironment>(a.alice);
            assert_eq!(
                v.issue_credential(a.charlie, CredentialType::Kyc, hash(1), 0),
                Err(Error::NotAnIssuer)
            );
            // Only the admin appoints issuers.
            test::set_caller::<ink::env::DefaultEnvironment>(a.bob);
            assert_eq!(v.set_issuer(a.eve, true), Err(Error::Unauthorized));
        }

        #[ink::test]
        fn expiry_must_be_in_the_future_or_zero() {
            let (mut v, a) = setup();
            test::set_caller::<ink::env::DefaultEnvironment>(a.bob);
            assert_eq!(
                v.issue_credential(a.charlie, CredentialType::Kyc, hash(1), 10),
                Err(Error::InvalidExpiration)
            );
            assert_eq!(
                v.issue_credential(a.charlie, CredentialType::Kyc, hash(1), 5),
                Err(Error::InvalidExpiration)
            );
            assert!(v.issue_credential(a.charlie, CredentialType::Kyc, hash(1), 11).is_ok());
            assert!(v.issue_credential(a.charlie, CredentialType::Kyc, hash(1), 0).is_ok());
        }

        #[ink::test]
        fn a_subject_can_hold_a_bounded_number_of_credentials() {
            let (mut v, a) = setup();
            test::set_caller::<ink::env::DefaultEnvironment>(a.bob);
            for _ in 0..MAX_CREDENTIALS_PER_SUBJECT {
                v.issue_credential(a.charlie, CredentialType::Kyc, hash(1), 0).unwrap();
            }
            assert_eq!(
                v.issue_credential(a.charlie, CredentialType::Kyc, hash(1), 0),
                Err(Error::TooManyCredentials)
            );
            // Other subjects are unaffected.
            assert!(v.issue_credential(a.django, CredentialType::Kyc, hash(1), 0).is_ok());
        }

        // ── Verification and expiry ─────────────────────────────────────

        #[ink::test]
        fn a_fresh_credential_verifies_and_reports_issuer_type_and_subject() {
            let (mut v, a) = setup();
            let id = issue(&mut v, &a, CredentialType::PropertyOwnership, 100);
            let r = v.verify_credential(id);
            assert_eq!(r.status, CredentialStatus::Valid);
            assert_eq!(r.issuer, Some(a.bob));
            assert_eq!(r.credential_type, Some(CredentialType::PropertyOwnership));
            assert_eq!(r.subject, Some(a.charlie));
        }

        #[ink::test]
        fn an_unknown_credential_is_not_found() {
            let (v, _) = setup();
            let r = v.verify_credential(99);
            assert_eq!(r.status, CredentialStatus::NotFound);
            assert_eq!((r.issuer, r.credential_type, r.subject), (None, None, None));
        }

        #[ink::test]
        fn a_credential_expires_after_its_expiration_block() {
            let (mut v, a) = setup();
            let id = issue(&mut v, &a, CredentialType::AccreditedInvestor, 50);
            block(50);
            assert_eq!(v.verify_credential(id).status, CredentialStatus::Valid, "valid through the expiry block");
            block(51);
            assert_eq!(v.verify_credential(id).status, CredentialStatus::Expired);
            assert!(!v.has_valid_credential(a.charlie, CredentialType::AccreditedInvestor));
        }

        #[ink::test]
        fn a_zero_expiration_never_expires() {
            let (mut v, a) = setup();
            let id = issue(&mut v, &a, CredentialType::Kyc, 0);
            block(u32::MAX);
            assert_eq!(v.verify_credential(id).status, CredentialStatus::Valid);
        }

        // ── Revocation ──────────────────────────────────────────────────

        #[ink::test]
        fn only_the_issuer_can_revoke_and_revocation_sticks() {
            let (mut v, a) = setup();
            let id = issue(&mut v, &a, CredentialType::Kyc, 0);
            for who in [a.alice, a.charlie, a.eve] {
                test::set_caller::<ink::env::DefaultEnvironment>(who);
                assert_eq!(v.revoke_credential(id), Err(Error::Unauthorized), "caller {who:?}");
            }
            test::set_caller::<ink::env::DefaultEnvironment>(a.bob);
            v.revoke_credential(id).unwrap();
            assert_eq!(v.verify_credential(id).status, CredentialStatus::Revoked);
            assert_eq!(v.revoke_credential(id), Err(Error::AlreadyRevoked));
            assert_eq!(v.revoke_credential(99), Err(Error::CredentialNotFound));
        }

        #[ink::test]
        fn revocation_takes_precedence_over_expiry() {
            let (mut v, a) = setup();
            let id = issue(&mut v, &a, CredentialType::Kyc, 20);
            test::set_caller::<ink::env::DefaultEnvironment>(a.bob);
            v.revoke_credential(id).unwrap();
            block(100);
            assert_eq!(v.verify_credential(id).status, CredentialStatus::Revoked);
        }

        #[ink::test]
        fn a_revoked_issuer_keeps_credentials_issued_earlier_but_cannot_issue_more() {
            let (mut v, a) = setup();
            let id = issue(&mut v, &a, CredentialType::Kyc, 0);
            test::set_caller::<ink::env::DefaultEnvironment>(a.alice);
            v.set_issuer(a.bob, false).unwrap();
            assert!(!v.is_issuer(a.bob));
            assert_eq!(v.verify_credential(id).status, CredentialStatus::Valid);
            test::set_caller::<ink::env::DefaultEnvironment>(a.bob);
            assert_eq!(
                v.issue_credential(a.charlie, CredentialType::Kyc, hash(1), 0),
                Err(Error::NotAnIssuer)
            );
            // The former issuer can still revoke what they issued.
            assert!(v.revoke_credential(id).is_ok());
        }

        // ── Queries ─────────────────────────────────────────────────────

        #[ink::test]
        fn a_subjects_credentials_are_listed_in_issue_order_with_every_status() {
            let (mut v, a) = setup();
            let kyc = issue(&mut v, &a, CredentialType::Kyc, 0);
            let owner = issue(&mut v, &a, CredentialType::PropertyOwnership, 0);
            test::set_caller::<ink::env::DefaultEnvironment>(a.bob);
            v.revoke_credential(kyc).unwrap();
            let list = v.get_credentials(a.charlie);
            assert_eq!(list.iter().map(|c| c.id).collect::<Vec<_>>(), vec![kyc, owner]);
            assert!(list[0].revoked && !list[1].revoked);
            assert!(v.get_credentials(a.eve).is_empty());
        }

        #[ink::test]
        fn has_valid_credential_checks_type_and_validity() {
            let (mut v, a) = setup();
            issue(&mut v, &a, CredentialType::Kyc, 0);
            issue(&mut v, &a, CredentialType::InsuranceActive, 30);
            assert!(v.has_valid_credential(a.charlie, CredentialType::Kyc));
            assert!(v.has_valid_credential(a.charlie, CredentialType::InsuranceActive));
            assert!(!v.has_valid_credential(a.charlie, CredentialType::AccreditedInvestor));
            block(31);
            assert!(!v.has_valid_credential(a.charlie, CredentialType::InsuranceActive));
            assert!(v.has_valid_credential(a.charlie, CredentialType::Kyc));
        }

        // ── Events ──────────────────────────────────────────────────────

        #[ink::test]
        fn issue_verify_and_revoke_emit_events() {
            let (mut v, a) = setup();
            let base = test::recorded_events().count();
            let id = issue(&mut v, &a, CredentialType::Kyc, 0);
            assert_eq!(test::recorded_events().count() - base, 1);
            test::set_caller::<ink::env::DefaultEnvironment>(a.eve);
            v.verify_and_log(id);
            assert_eq!(test::recorded_events().count() - base, 2);
            // Plain verification and unknown ids emit nothing.
            v.verify_credential(id);
            v.verify_and_log(99);
            assert_eq!(test::recorded_events().count() - base, 2);
            test::set_caller::<ink::env::DefaultEnvironment>(a.bob);
            v.revoke_credential(id).unwrap();
            assert_eq!(test::recorded_events().count() - base, 3);
        }
    }
}
