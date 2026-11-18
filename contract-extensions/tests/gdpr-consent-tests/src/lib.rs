//! Unit tests for the GDPR consent contract (#1459).
//!
//! The contract manages consent, retention policies and data-access requests. It
//! has **no erasure ("right to be forgotten") message**, so erasure is not tested
//! here; the closest implemented rights - consent withdrawal and data access -
//! are, along with consent expiry and retention policy administration.

#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(test)]
mod tests {
    use ink::env::{test, DefaultEnvironment};
    use propchain_gdpr::gdpr_consent::{ConsentStatus, Error, GdprConsent, ProcessingPurpose};

    type Accounts = test::DefaultAccounts<DefaultEnvironment>;

    const DAY_MS: u64 = 86_400_000;
    const START: u64 = 1_000;

    fn setup() -> (GdprConsent, Accounts) {
        let accounts = test::default_accounts::<DefaultEnvironment>();
        test::set_caller::<DefaultEnvironment>(accounts.alice);
        test::set_block_timestamp::<DefaultEnvironment>(START);
        (GdprConsent::new(), accounts)
    }

    fn as_user(who: ink::primitives::AccountId) {
        test::set_caller::<DefaultEnvironment>(who);
    }

    // ── Granting consent ────────────────────────────────────────────────

    #[ink::test]
    fn a_subject_can_grant_consent_for_a_purpose() {
        let (mut g, accounts) = setup();
        as_user(accounts.bob);
        let id = g.grant_consent(accounts.bob, ProcessingPurpose::KYC, DAY_MS).unwrap();

        let record = g.get_consent(id).unwrap();
        assert_eq!(record.data_subject, accounts.bob);
        assert_eq!(record.purpose, ProcessingPurpose::KYC);
        assert_eq!(record.status, ConsentStatus::Granted);
        assert_eq!(record.granted_at, START);
        assert_eq!(record.expires_at, START + DAY_MS);
        assert_eq!(record.withdrawn_at, None);
        assert!(g.check_consent(accounts.bob, ProcessingPurpose::KYC));
    }

    #[ink::test]
    fn consent_is_purpose_specific() {
        let (mut g, accounts) = setup();
        as_user(accounts.bob);
        g.grant_consent(accounts.bob, ProcessingPurpose::KYC, DAY_MS).unwrap();
        assert!(!g.check_consent(accounts.bob, ProcessingPurpose::Marketing));
        assert!(!g.check_consent(accounts.charlie, ProcessingPurpose::KYC));
        // `Other` purposes are distinguished by their payload.
        g.grant_consent(accounts.bob, ProcessingPurpose::Other(vec![1]), DAY_MS).unwrap();
        assert!(g.check_consent(accounts.bob, ProcessingPurpose::Other(vec![1])));
        assert!(!g.check_consent(accounts.bob, ProcessingPurpose::Other(vec![2])));
    }

    #[ink::test]
    fn only_the_subject_or_admin_can_grant_on_a_subjects_behalf() {
        let (mut g, accounts) = setup();
        as_user(accounts.charlie);
        assert_eq!(
            g.grant_consent(accounts.bob, ProcessingPurpose::KYC, DAY_MS),
            Err(Error::NotAuthorized)
        );
        assert!(!g.check_consent(accounts.bob, ProcessingPurpose::KYC));

        as_user(accounts.alice); // admin
        assert!(g.grant_consent(accounts.bob, ProcessingPurpose::KYC, DAY_MS).is_ok());
    }

    #[ink::test]
    fn a_zero_duration_is_rejected() {
        let (mut g, accounts) = setup();
        as_user(accounts.bob);
        assert_eq!(
            g.grant_consent(accounts.bob, ProcessingPurpose::KYC, 0),
            Err(Error::InvalidDuration)
        );
        assert!(g.get_subject_consents(accounts.bob).is_empty());
    }

    #[ink::test]
    fn an_overflowing_expiry_is_rejected() {
        let (mut g, accounts) = setup();
        as_user(accounts.bob);
        assert_eq!(
            g.grant_consent(accounts.bob, ProcessingPurpose::KYC, u64::MAX),
            Err(Error::InvalidDuration)
        );
    }

    #[ink::test]
    fn consents_are_listed_per_subject_in_order() {
        let (mut g, accounts) = setup();
        as_user(accounts.bob);
        let a = g.grant_consent(accounts.bob, ProcessingPurpose::KYC, DAY_MS).unwrap();
        let b = g.grant_consent(accounts.bob, ProcessingPurpose::Marketing, DAY_MS).unwrap();
        as_user(accounts.charlie);
        g.grant_consent(accounts.charlie, ProcessingPurpose::KYC, DAY_MS).unwrap();

        let ids: Vec<u64> = g.get_subject_consents(accounts.bob).iter().map(|c| c.consent_id).collect();
        assert_eq!(ids, vec![a, b]);
        assert!(g.get_subject_consents(accounts.eve).is_empty());
        assert!(g.get_consent(999).is_none());
    }

    // ── Withdrawal ──────────────────────────────────────────────────────

    #[ink::test]
    fn the_subject_can_withdraw_consent_and_processing_must_stop() {
        let (mut g, accounts) = setup();
        as_user(accounts.bob);
        let id = g.grant_consent(accounts.bob, ProcessingPurpose::Marketing, DAY_MS).unwrap();
        test::set_block_timestamp::<DefaultEnvironment>(START + 10);
        g.withdraw_consent(id).unwrap();

        let record = g.get_consent(id).unwrap();
        assert_eq!(record.status, ConsentStatus::Withdrawn);
        assert_eq!(record.withdrawn_at, Some(START + 10));
        assert!(!g.check_consent(accounts.bob, ProcessingPurpose::Marketing));
    }

    #[ink::test]
    fn withdrawal_is_limited_to_the_subject_or_admin_and_cannot_repeat() {
        let (mut g, accounts) = setup();
        as_user(accounts.bob);
        let id = g.grant_consent(accounts.bob, ProcessingPurpose::KYC, DAY_MS).unwrap();

        as_user(accounts.charlie);
        assert_eq!(g.withdraw_consent(id), Err(Error::NotAuthorized));

        as_user(accounts.alice);
        g.withdraw_consent(id).unwrap();
        assert_eq!(g.withdraw_consent(id), Err(Error::ConsentNotFound));
        assert_eq!(g.withdraw_consent(404), Err(Error::ConsentNotFound));
    }

    #[ink::test]
    fn withdrawing_one_consent_leaves_others_for_the_same_subject_intact() {
        let (mut g, accounts) = setup();
        as_user(accounts.bob);
        let kyc = g.grant_consent(accounts.bob, ProcessingPurpose::KYC, DAY_MS).unwrap();
        g.grant_consent(accounts.bob, ProcessingPurpose::TaxReporting, DAY_MS).unwrap();
        g.withdraw_consent(kyc).unwrap();
        assert!(!g.check_consent(accounts.bob, ProcessingPurpose::KYC));
        assert!(g.check_consent(accounts.bob, ProcessingPurpose::TaxReporting));
    }

    // ── Expiry ──────────────────────────────────────────────────────────

    #[ink::test]
    fn consent_stops_counting_once_it_expires_even_before_it_is_marked() {
        let (mut g, accounts) = setup();
        as_user(accounts.bob);
        let id = g.grant_consent(accounts.bob, ProcessingPurpose::KYC, 100).unwrap();

        test::set_block_timestamp::<DefaultEnvironment>(START + 99);
        assert!(g.check_consent(accounts.bob, ProcessingPurpose::KYC));
        // At exactly `expires_at` the consent no longer holds.
        test::set_block_timestamp::<DefaultEnvironment>(START + 100);
        assert!(!g.check_consent(accounts.bob, ProcessingPurpose::KYC));
        // The record still reads Granted until the admin marks it expired.
        assert_eq!(g.get_consent(id).unwrap().status, ConsentStatus::Granted);
    }

    #[ink::test]
    fn the_admin_can_mark_a_consent_expired_only_after_its_expiry() {
        let (mut g, accounts) = setup();
        as_user(accounts.bob);
        let id = g.grant_consent(accounts.bob, ProcessingPurpose::KYC, 100).unwrap();

        as_user(accounts.alice);
        // Not expired yet.
        assert_eq!(g.expire_consent(id), Err(Error::ConsentNotFound));
        test::set_block_timestamp::<DefaultEnvironment>(START + 100);
        g.expire_consent(id).unwrap();
        assert_eq!(g.get_consent(id).unwrap().status, ConsentStatus::Expired);
        // Already expired.
        assert_eq!(g.expire_consent(id), Err(Error::ConsentNotFound));
    }

    #[ink::test]
    fn only_the_admin_can_expire_consents() {
        let (mut g, accounts) = setup();
        as_user(accounts.bob);
        let id = g.grant_consent(accounts.bob, ProcessingPurpose::KYC, 100).unwrap();
        test::set_block_timestamp::<DefaultEnvironment>(START + 200);
        assert_eq!(g.expire_consent(id), Err(Error::NotAuthorized));
    }

    // ── Retention policies ──────────────────────────────────────────────

    #[ink::test]
    fn the_admin_sets_and_reads_a_retention_policy_per_purpose() {
        let (mut g, _) = setup();
        assert!(g.get_retention_policy(ProcessingPurpose::KYC).is_none());
        g.set_retention_policy(ProcessingPurpose::KYC, 365, true).unwrap();
        g.set_retention_policy(ProcessingPurpose::Marketing, 30, false).unwrap();

        let kyc = g.get_retention_policy(ProcessingPurpose::KYC).unwrap();
        assert_eq!((kyc.retention_days, kyc.auto_delete), (365, true));
        let marketing = g.get_retention_policy(ProcessingPurpose::Marketing).unwrap();
        assert_eq!((marketing.retention_days, marketing.auto_delete), (30, false));

        // Updating replaces the previous policy for that purpose.
        g.set_retention_policy(ProcessingPurpose::KYC, 730, false).unwrap();
        assert_eq!(g.get_retention_policy(ProcessingPurpose::KYC).unwrap().retention_days, 730);
    }

    #[ink::test]
    fn retention_policies_are_admin_only() {
        let (mut g, accounts) = setup();
        as_user(accounts.bob);
        assert_eq!(
            g.set_retention_policy(ProcessingPurpose::KYC, 1, true),
            Err(Error::NotAuthorized)
        );
        assert!(g.get_retention_policy(ProcessingPurpose::KYC).is_none());
    }

    // ── Data access requests ────────────────────────────────────────────

    #[ink::test]
    fn a_subject_can_request_access_and_the_admin_fulfils_it() {
        let (mut g, accounts) = setup();
        as_user(accounts.bob);
        let id = g.request_data_access().unwrap();
        let req = g.get_data_access_request(id).unwrap();
        assert_eq!(req.data_subject, accounts.bob);
        assert!(!req.fulfilled);
        assert_eq!(req.requested_at, START);

        as_user(accounts.alice);
        test::set_block_timestamp::<DefaultEnvironment>(START + 50);
        g.fulfill_data_access(id).unwrap();
        let done = g.get_data_access_request(id).unwrap();
        assert!(done.fulfilled);
        assert_eq!(done.fulfilled_at, Some(START + 50));
    }

    #[ink::test]
    fn only_the_admin_can_fulfil_and_unknown_requests_are_reported() {
        let (mut g, accounts) = setup();
        as_user(accounts.bob);
        let id = g.request_data_access().unwrap();
        assert_eq!(g.fulfill_data_access(id), Err(Error::NotAuthorized));
        as_user(accounts.alice);
        assert_eq!(g.fulfill_data_access(99), Err(Error::DataRequestNotFound));
        assert!(g.get_data_access_request(99).is_none());
    }

    #[ink::test]
    fn requests_are_listed_per_subject() {
        let (mut g, accounts) = setup();
        as_user(accounts.bob);
        g.request_data_access().unwrap();
        g.request_data_access().unwrap();
        as_user(accounts.charlie);
        g.request_data_access().unwrap();
        assert_eq!(g.get_subject_requests(accounts.bob).len(), 2);
        assert_eq!(g.get_subject_requests(accounts.charlie).len(), 1);
        assert!(g.get_subject_requests(accounts.eve).is_empty());
    }
}
