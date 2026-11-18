#![cfg_attr(not(feature = "std"), no_std, no_main)]
#![allow(clippy::new_without_default)]

//! # Tenant onboarding (#1439)
//!
//! Tracks a tenant through the pipeline
//!
//! ```text
//! Submitted --approve--> Approved --sign_lease--> LeaseSigned --confirm_move_in--> MovedIn
//!     \--reject--> Rejected
//! ```
//!
//! Each step is performed by the right party:
//!
//! | Step | Who |
//! |---|---|
//! | `submit_application` | the applicant |
//! | `approve_application` / `reject_application` | the property's landlord (or the admin) |
//! | `sign_lease` | the applicant, once approved |
//! | `confirm_move_in` | the landlord (or admin), once the lease is signed |
//!
//! Only 32-byte **hashes** of off-chain documents are stored (application data,
//! lease terms, rejection reason, signature). The contract is self-contained: it
//! does not call the property-management contract; landlords are registered per
//! property by the admin, and creating the on-chain lease at move-in is left to
//! the integration layer (the `MoveInConfirmed` event carries what it needs).

#[ink::contract]
pub mod tenant_onboarding {
    use ink::prelude::vec::Vec;
    use ink::storage::Mapping;

    #[derive(Debug, Clone, Copy, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout))]
    pub enum ApplicationStatus {
        Submitted,
        Approved,
        Rejected,
        LeaseSigned,
        MovedIn,
    }

    #[derive(Debug, Clone, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout))]
    pub struct Application {
        pub id: u64,
        pub property_id: u64,
        pub applicant: AccountId,
        pub landlord: AccountId,
        pub status: ApplicationStatus,
        pub application_data_hash: Hash,
        pub lease_terms_hash: Option<Hash>,
        pub rejection_reason_hash: Option<Hash>,
        pub signature_hash: Option<Hash>,
        pub submitted_at: u32,
        pub updated_at: u32,
    }

    #[derive(Debug, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub enum Error {
        Unauthorized,
        PropertyNotRegistered,
        ApplicationNotFound,
        /// The application is not in the status this action requires.
        InvalidStatus,
        /// The applicant already has an open application for this property.
        DuplicateApplication,
    }

    #[ink(event)]
    pub struct ApplicationSubmitted {
        #[ink(topic)]
        pub application_id: u64,
        #[ink(topic)]
        pub property_id: u64,
        pub applicant: AccountId,
    }

    #[ink(event)]
    pub struct ApplicationApproved {
        #[ink(topic)]
        pub application_id: u64,
        pub lease_terms_hash: Hash,
    }

    #[ink(event)]
    pub struct ApplicationRejected {
        #[ink(topic)]
        pub application_id: u64,
        pub reason_hash: Hash,
    }

    #[ink(event)]
    pub struct LeaseSigned {
        #[ink(topic)]
        pub application_id: u64,
        pub signature_hash: Hash,
    }

    #[ink(event)]
    pub struct MoveInConfirmed {
        #[ink(topic)]
        pub application_id: u64,
        #[ink(topic)]
        pub property_id: u64,
        pub tenant: AccountId,
        pub landlord: AccountId,
        pub lease_terms_hash: Hash,
    }

    #[ink(storage)]
    pub struct TenantOnboarding {
        admin: AccountId,
        landlords: Mapping<u64, AccountId>,
        applications: Mapping<u64, Application>,
        by_applicant: Mapping<AccountId, Vec<u64>>,
        by_property: Mapping<u64, Vec<u64>>,
        next_id: u64,
    }

    impl TenantOnboarding {
        #[ink(constructor)]
        pub fn new() -> Self {
            Self {
                admin: Self::env().caller(),
                landlords: Mapping::default(),
                applications: Mapping::default(),
                by_applicant: Mapping::default(),
                by_property: Mapping::default(),
                next_id: 1,
            }
        }

        /// Record who the landlord of `property_id` is. Admin only.
        #[ink(message)]
        pub fn register_property(&mut self, property_id: u64, landlord: AccountId) -> Result<(), Error> {
            if self.env().caller() != self.admin {
                return Err(Error::Unauthorized);
            }
            self.landlords.insert(property_id, &landlord);
            Ok(())
        }

        #[ink(message)]
        pub fn get_landlord(&self, property_id: u64) -> Option<AccountId> {
            self.landlords.get(property_id)
        }

        // ── Pipeline ───────────────────────────────────────────────────

        /// Apply for `property_id`. The caller must be the `applicant`.
        #[ink(message)]
        pub fn submit_application(
            &mut self,
            property_id: u64,
            applicant: AccountId,
            application_data_hash: Hash,
        ) -> Result<u64, Error> {
            if self.env().caller() != applicant {
                return Err(Error::Unauthorized);
            }
            let landlord = self.landlords.get(property_id).ok_or(Error::PropertyNotRegistered)?;
            // One open application per applicant and property.
            for id in self.by_applicant.get(applicant).unwrap_or_default() {
                if let Some(a) = self.applications.get(id) {
                    let open = matches!(
                        a.status,
                        ApplicationStatus::Submitted | ApplicationStatus::Approved | ApplicationStatus::LeaseSigned
                    );
                    if a.property_id == property_id && open {
                        return Err(Error::DuplicateApplication);
                    }
                }
            }
            let now = self.env().block_number();
            let id = self.next_id;
            self.next_id += 1;
            self.applications.insert(
                id,
                &Application {
                    id,
                    property_id,
                    applicant,
                    landlord,
                    status: ApplicationStatus::Submitted,
                    application_data_hash,
                    lease_terms_hash: None,
                    rejection_reason_hash: None,
                    signature_hash: None,
                    submitted_at: now,
                    updated_at: now,
                },
            );
            let mut mine = self.by_applicant.get(applicant).unwrap_or_default();
            mine.push(id);
            self.by_applicant.insert(applicant, &mine);
            let mut on_property = self.by_property.get(property_id).unwrap_or_default();
            on_property.push(id);
            self.by_property.insert(property_id, &on_property);
            self.env().emit_event(ApplicationSubmitted { application_id: id, property_id, applicant });
            Ok(id)
        }

        #[ink(message)]
        pub fn approve_application(&mut self, application_id: u64, lease_terms_hash: Hash) -> Result<(), Error> {
            let mut a = self.load(application_id)?;
            self.ensure_landlord(&a)?;
            Self::expect(&a, ApplicationStatus::Submitted)?;
            a.status = ApplicationStatus::Approved;
            a.lease_terms_hash = Some(lease_terms_hash);
            self.save(a);
            self.env().emit_event(ApplicationApproved { application_id, lease_terms_hash });
            Ok(())
        }

        #[ink(message)]
        pub fn reject_application(&mut self, application_id: u64, reason_hash: Hash) -> Result<(), Error> {
            let mut a = self.load(application_id)?;
            self.ensure_landlord(&a)?;
            Self::expect(&a, ApplicationStatus::Submitted)?;
            a.status = ApplicationStatus::Rejected;
            a.rejection_reason_hash = Some(reason_hash);
            self.save(a);
            self.env().emit_event(ApplicationRejected { application_id, reason_hash });
            Ok(())
        }

        /// The applicant signs the approved lease.
        #[ink(message)]
        pub fn sign_lease(&mut self, application_id: u64, signature_hash: Hash) -> Result<(), Error> {
            let mut a = self.load(application_id)?;
            if self.env().caller() != a.applicant {
                return Err(Error::Unauthorized);
            }
            Self::expect(&a, ApplicationStatus::Approved)?;
            a.status = ApplicationStatus::LeaseSigned;
            a.signature_hash = Some(signature_hash);
            self.save(a);
            self.env().emit_event(LeaseSigned { application_id, signature_hash });
            Ok(())
        }

        /// The landlord records that the tenant moved in; the lease becomes active.
        #[ink(message)]
        pub fn confirm_move_in(&mut self, application_id: u64) -> Result<(), Error> {
            let mut a = self.load(application_id)?;
            self.ensure_landlord(&a)?;
            Self::expect(&a, ApplicationStatus::LeaseSigned)?;
            a.status = ApplicationStatus::MovedIn;
            let (property_id, tenant, landlord, terms) =
                (a.property_id, a.applicant, a.landlord, a.lease_terms_hash.unwrap_or_default());
            self.save(a);
            self.env().emit_event(MoveInConfirmed {
                application_id,
                property_id,
                tenant,
                landlord,
                lease_terms_hash: terms,
            });
            Ok(())
        }

        // ── Queries ────────────────────────────────────────────────────

        #[ink(message)]
        pub fn get_application_status(&self, application_id: u64) -> Option<ApplicationStatus> {
            self.applications.get(application_id).map(|a| a.status)
        }

        #[ink(message)]
        pub fn get_application(&self, application_id: u64) -> Option<Application> {
            self.applications.get(application_id)
        }

        #[ink(message)]
        pub fn get_applications_by_applicant(&self, applicant: AccountId) -> Vec<u64> {
            self.by_applicant.get(applicant).unwrap_or_default()
        }

        #[ink(message)]
        pub fn get_applications_for_property(&self, property_id: u64) -> Vec<u64> {
            self.by_property.get(property_id).unwrap_or_default()
        }

        // ── Internals ──────────────────────────────────────────────────

        fn load(&self, id: u64) -> Result<Application, Error> {
            self.applications.get(id).ok_or(Error::ApplicationNotFound)
        }

        fn save(&mut self, mut a: Application) {
            a.updated_at = self.env().block_number();
            self.applications.insert(a.id, &a);
        }

        fn ensure_landlord(&self, a: &Application) -> Result<(), Error> {
            let caller = self.env().caller();
            if caller != a.landlord && caller != self.admin {
                return Err(Error::Unauthorized);
            }
            Ok(())
        }

        fn expect(a: &Application, status: ApplicationStatus) -> Result<(), Error> {
            if a.status != status {
                return Err(Error::InvalidStatus);
            }
            Ok(())
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use ink::env::test;

        type Accounts = test::DefaultAccounts<ink::env::DefaultEnvironment>;
        const PROPERTY: u64 = 7;

        fn h(b: u8) -> Hash {
            Hash::from([b; 32])
        }

        /// Admin = alice; bob is the landlord of property 7; charlie the applicant.
        fn setup() -> (TenantOnboarding, Accounts) {
            let a = test::default_accounts::<ink::env::DefaultEnvironment>();
            test::set_caller::<ink::env::DefaultEnvironment>(a.alice);
            test::set_block_number::<ink::env::DefaultEnvironment>(10);
            let mut t = TenantOnboarding::new();
            t.register_property(PROPERTY, a.bob).unwrap();
            (t, a)
        }

        fn apply(t: &mut TenantOnboarding, who: AccountId) -> u64 {
            test::set_caller::<ink::env::DefaultEnvironment>(who);
            t.submit_application(PROPERTY, who, h(1)).unwrap()
        }

        fn as_user(who: AccountId) {
            test::set_caller::<ink::env::DefaultEnvironment>(who);
        }

        // ── Full lifecycle ──────────────────────────────────────────────

        #[ink::test]
        fn an_application_moves_through_the_whole_pipeline() {
            let (mut t, a) = setup();
            let id = apply(&mut t, a.charlie);
            assert_eq!(t.get_application_status(id), Some(ApplicationStatus::Submitted));

            as_user(a.bob);
            t.approve_application(id, h(2)).unwrap();
            assert_eq!(t.get_application_status(id), Some(ApplicationStatus::Approved));

            as_user(a.charlie);
            t.sign_lease(id, h(3)).unwrap();
            assert_eq!(t.get_application_status(id), Some(ApplicationStatus::LeaseSigned));

            as_user(a.bob);
            test::set_block_number::<ink::env::DefaultEnvironment>(99);
            t.confirm_move_in(id).unwrap();
            let app = t.get_application(id).unwrap();
            assert_eq!(app.status, ApplicationStatus::MovedIn);
            assert_eq!(
                (app.lease_terms_hash, app.signature_hash, app.submitted_at, app.updated_at),
                (Some(h(2)), Some(h(3)), 10, 99)
            );
        }

        #[ink::test]
        fn a_rejected_application_records_the_reason_and_ends_there() {
            let (mut t, a) = setup();
            let id = apply(&mut t, a.charlie);
            as_user(a.bob);
            t.reject_application(id, h(9)).unwrap();
            let app = t.get_application(id).unwrap();
            assert_eq!((app.status, app.rejection_reason_hash), (ApplicationStatus::Rejected, Some(h(9))));

            // Nothing further can happen to it.
            assert_eq!(t.approve_application(id, h(2)), Err(Error::InvalidStatus));
            assert_eq!(t.reject_application(id, h(2)), Err(Error::InvalidStatus));
            as_user(a.charlie);
            assert_eq!(t.sign_lease(id, h(3)), Err(Error::InvalidStatus));
            as_user(a.bob);
            assert_eq!(t.confirm_move_in(id), Err(Error::InvalidStatus));
        }

        // ── Ordering of steps ───────────────────────────────────────────

        #[ink::test]
        fn steps_cannot_be_skipped_or_repeated() {
            let (mut t, a) = setup();
            let id = apply(&mut t, a.charlie);
            // Cannot sign or move in before approval.
            assert_eq!(t.sign_lease(id, h(3)), Err(Error::InvalidStatus));
            as_user(a.bob);
            assert_eq!(t.confirm_move_in(id), Err(Error::InvalidStatus));
            t.approve_application(id, h(2)).unwrap();
            assert_eq!(t.approve_application(id, h(2)), Err(Error::InvalidStatus));
            // Cannot move in before the lease is signed.
            assert_eq!(t.confirm_move_in(id), Err(Error::InvalidStatus));
            as_user(a.charlie);
            t.sign_lease(id, h(3)).unwrap();
            assert_eq!(t.sign_lease(id, h(3)), Err(Error::InvalidStatus));
            as_user(a.bob);
            t.confirm_move_in(id).unwrap();
            assert_eq!(t.confirm_move_in(id), Err(Error::InvalidStatus));
        }

        // ── Authorization ───────────────────────────────────────────────

        #[ink::test]
        fn only_the_applicant_can_apply_for_themselves() {
            let (mut t, a) = setup();
            as_user(a.eve);
            assert_eq!(t.submit_application(PROPERTY, a.charlie, h(1)), Err(Error::Unauthorized));
            assert!(t.get_applications_by_applicant(a.charlie).is_empty());
        }

        #[ink::test]
        fn only_the_landlord_or_admin_can_approve_reject_or_confirm_move_in() {
            let (mut t, a) = setup();
            let id = apply(&mut t, a.charlie);
            for who in [a.charlie, a.eve] {
                as_user(who);
                assert_eq!(t.approve_application(id, h(2)), Err(Error::Unauthorized));
                assert_eq!(t.reject_application(id, h(2)), Err(Error::Unauthorized));
            }
            // The admin may act for the landlord.
            as_user(a.alice);
            t.approve_application(id, h(2)).unwrap();
            as_user(a.charlie);
            t.sign_lease(id, h(3)).unwrap();
            as_user(a.eve);
            assert_eq!(t.confirm_move_in(id), Err(Error::Unauthorized));
            as_user(a.alice);
            assert!(t.confirm_move_in(id).is_ok());
        }

        #[ink::test]
        fn only_the_applicant_signs_the_lease() {
            let (mut t, a) = setup();
            let id = apply(&mut t, a.charlie);
            as_user(a.bob);
            t.approve_application(id, h(2)).unwrap();
            for who in [a.bob, a.alice, a.eve] {
                as_user(who);
                assert_eq!(t.sign_lease(id, h(3)), Err(Error::Unauthorized));
            }
            assert_eq!(t.get_application_status(id), Some(ApplicationStatus::Approved));
        }

        #[ink::test]
        fn property_registration_is_admin_only() {
            let (mut t, a) = setup();
            as_user(a.bob);
            assert_eq!(t.register_property(8, a.bob), Err(Error::Unauthorized));
            assert_eq!(t.get_landlord(8), None);
            assert_eq!(t.get_landlord(PROPERTY), Some(a.bob));
        }

        // ── Validation and bookkeeping ──────────────────────────────────

        #[ink::test]
        fn unknown_properties_and_applications_are_reported() {
            let (mut t, a) = setup();
            as_user(a.charlie);
            assert_eq!(t.submit_application(99, a.charlie, h(1)), Err(Error::PropertyNotRegistered));
            assert_eq!(t.get_application_status(5), None);
            as_user(a.bob);
            assert_eq!(t.approve_application(5, h(1)), Err(Error::ApplicationNotFound));
            assert_eq!(t.confirm_move_in(5), Err(Error::ApplicationNotFound));
        }

        #[ink::test]
        fn an_applicant_has_one_open_application_per_property_but_may_reapply_after_rejection() {
            let (mut t, a) = setup();
            let first = apply(&mut t, a.charlie);
            as_user(a.charlie);
            assert_eq!(t.submit_application(PROPERTY, a.charlie, h(1)), Err(Error::DuplicateApplication));

            as_user(a.bob);
            t.reject_application(first, h(9)).unwrap();
            let second = apply(&mut t, a.charlie);
            assert_ne!(first, second);
            assert_eq!(t.get_applications_by_applicant(a.charlie), vec![first, second]);
            assert_eq!(t.get_applications_for_property(PROPERTY), vec![first, second]);
        }

        #[ink::test]
        fn different_applicants_and_properties_are_independent() {
            let (mut t, a) = setup();
            t.register_property(8, a.django).unwrap();
            let one = apply(&mut t, a.charlie);
            let two = apply(&mut t, a.eve);
            as_user(a.charlie);
            let three = t.submit_application(8, a.charlie, h(1)).unwrap();
            assert_eq!(t.get_application(three).unwrap().landlord, a.django);
            assert_eq!(t.get_applications_for_property(PROPERTY), vec![one, two]);
        }

        #[ink::test]
        fn each_step_emits_exactly_one_event() {
            let (mut t, a) = setup();
            let before = test::recorded_events().count();
            let id = apply(&mut t, a.charlie);
            as_user(a.bob);
            t.approve_application(id, h(2)).unwrap();
            as_user(a.charlie);
            t.sign_lease(id, h(3)).unwrap();
            as_user(a.bob);
            t.confirm_move_in(id).unwrap();
            assert_eq!(test::recorded_events().count() - before, 4);
        }
    }
}
