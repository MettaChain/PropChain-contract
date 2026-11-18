//! Unit tests for property management registration and lease transfer of
//! occupancy (#1455).
//!
//! This contract has no property *ownership transfer*. The nearest flows it
//! provides, and which these tests cover, are: gated registration
//! (`register_property`, which needs a configured identity registry),
//! manager / jurisdiction administration, and the lease lifecycle (create,
//! pay rent, end) that hands occupancy of a tokenized property to a tenant.

#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(test)]
mod tests {
    use ink::env::{test, DefaultEnvironment};
    use ink::primitives::{AccountId, Hash};
    use property_management::property_management::{
        Error, JurisdictionCompliance, LeaseStatus, PropertyManagement,
    };

    type Accounts = test::DefaultAccounts<DefaultEnvironment>;

    const TOKEN: u64 = 7;
    const RENT: u128 = 1_000;
    const PERIOD: u64 = 30 * 86_400;
    const FIRST_DUE: u64 = 5_000;

    /// Admin = alice. Alice (landlord), bob (tenant), charlie (fee beneficiary).
    fn setup() -> (PropertyManagement, Accounts) {
        let accounts = test::default_accounts::<DefaultEnvironment>();
        for a in [accounts.alice, accounts.bob, accounts.charlie, accounts.django] {
            test::set_account_balance::<DefaultEnvironment>(a, 1_000_000);
        }
        test::set_caller::<DefaultEnvironment>(accounts.alice);
        test::set_block_timestamp::<DefaultEnvironment>(1_000);
        (PropertyManagement::new(), accounts)
    }

    fn lease(
        pm: &mut PropertyManagement,
        landlord: AccountId,
        tenant: AccountId,
        fee_bps: u16,
        deposit: u128,
    ) -> Result<u64, Error> {
        pm.create_lease(TOKEN, tenant, landlord, RENT, PERIOD, fee_bps, deposit, FIRST_DUE)
    }

    fn jurisdiction(max_deposit_bps: u16) -> JurisdictionCompliance {
        JurisdictionCompliance {
            jurisdiction_code: "US-NY".into(),
            max_security_deposit_bps: max_deposit_bps,
            min_notice_period_days: 30,
            late_fee_cap_bps: 500,
            last_audit_ts: 0,
            compliant: true,
        }
    }

    // ── Registration ────────────────────────────────────────────────────

    #[ink::test]
    fn registration_requires_a_configured_identity_registry() {
        let (mut pm, accounts) = setup();
        let hash = Hash::from([1u8; 32]);
        assert_eq!(pm.register_property(TOKEN, hash), Err(Error::ComplianceViolation));
        assert_eq!(
            pm.verify_onboarding_cross_chain(accounts.bob),
            Err(Error::ComplianceViolation)
        );
    }

    #[ink::test]
    fn the_registries_can_only_be_configured_by_the_admin() {
        let (mut pm, accounts) = setup();
        test::set_caller::<DefaultEnvironment>(accounts.bob);
        assert_eq!(pm.set_identity_registry(Some(accounts.eve)), Err(Error::Unauthorized));
        assert_eq!(pm.set_compliance_registry(Some(accounts.eve)), Err(Error::Unauthorized));
        assert_eq!(pm.set_fee_beneficiary(accounts.bob), Err(Error::Unauthorized));
    }

    // ── Managers and jurisdiction rules ─────────────────────────────────

    #[ink::test]
    fn the_admin_appoints_and_revokes_managers() {
        let (mut pm, accounts) = setup();
        assert_eq!(pm.admin(), accounts.alice);
        assert!(!pm.is_manager(accounts.bob));
        pm.add_manager(accounts.bob).unwrap();
        assert!(pm.is_manager(accounts.bob));
        pm.remove_manager(accounts.bob).unwrap();
        assert!(!pm.is_manager(accounts.bob));

        test::set_caller::<DefaultEnvironment>(accounts.charlie);
        assert_eq!(pm.add_manager(accounts.charlie), Err(Error::Unauthorized));
        assert_eq!(pm.remove_manager(accounts.bob), Err(Error::Unauthorized));
    }

    #[ink::test]
    fn jurisdiction_rules_round_trip_and_are_restricted() {
        let (mut pm, accounts) = setup();
        assert_eq!(pm.get_jurisdiction_compliance(TOKEN), None);
        pm.set_jurisdiction_compliance(TOKEN, jurisdiction(200)).unwrap();
        assert_eq!(pm.get_jurisdiction_compliance(TOKEN), Some(jurisdiction(200)));

        test::set_caller::<DefaultEnvironment>(accounts.eve);
        assert_eq!(
            pm.set_jurisdiction_compliance(TOKEN, jurisdiction(10_000)),
            Err(Error::Unauthorized)
        );
    }

    // ── Creating a lease ────────────────────────────────────────────────

    #[ink::test]
    fn a_landlord_creates_an_active_lease() {
        let (mut pm, accounts) = setup();
        let id = lease(&mut pm, accounts.alice, accounts.bob, 500, 100).unwrap();
        assert_eq!(id, 1);

        let l = pm.get_lease(id).unwrap();
        assert_eq!(l.token_id, TOKEN);
        assert_eq!(l.tenant, accounts.bob);
        assert_eq!(l.landlord, accounts.alice);
        assert_eq!(l.rent_per_period, RENT);
        assert_eq!(l.period_secs, PERIOD);
        assert_eq!(l.next_due, FIRST_DUE);
        assert_eq!(l.management_fee_bps, 500);
        assert_eq!(l.status, LeaseStatus::Active);
        assert_eq!(l.created_at, 1_000);
        assert!(pm.get_lease(2).is_none());
    }

    #[ink::test]
    fn lease_ids_increase_and_leases_are_independent() {
        let (mut pm, accounts) = setup();
        let a = lease(&mut pm, accounts.alice, accounts.bob, 0, 0).unwrap();
        let b = lease(&mut pm, accounts.alice, accounts.charlie, 0, 0).unwrap();
        assert_eq!((a, b), (1, 2));
        assert_eq!(pm.get_lease(a).unwrap().tenant, accounts.bob);
        assert_eq!(pm.get_lease(b).unwrap().tenant, accounts.charlie);
    }

    #[ink::test]
    fn only_the_landlord_a_manager_or_the_admin_can_create_a_lease() {
        let (mut pm, accounts) = setup();
        // Bob (a would-be tenant) cannot create a lease for someone else's property.
        test::set_caller::<DefaultEnvironment>(accounts.bob);
        assert_eq!(
            lease(&mut pm, accounts.charlie, accounts.bob, 0, 0),
            Err(Error::NotLandlordOrManager)
        );
        // The named landlord can.
        test::set_caller::<DefaultEnvironment>(accounts.charlie);
        assert!(lease(&mut pm, accounts.charlie, accounts.bob, 0, 0).is_ok());
        // So can a manager on the landlord's behalf.
        test::set_caller::<DefaultEnvironment>(accounts.alice);
        pm.add_manager(accounts.django).unwrap();
        test::set_caller::<DefaultEnvironment>(accounts.django);
        assert!(lease(&mut pm, accounts.charlie, accounts.bob, 0, 0).is_ok());
    }

    #[ink::test]
    fn invalid_terms_are_rejected() {
        let (mut pm, accounts) = setup();
        assert_eq!(
            pm.create_lease(TOKEN, accounts.bob, accounts.alice, 0, PERIOD, 0, 0, FIRST_DUE),
            Err(Error::InvalidAmount)
        );
        assert_eq!(
            pm.create_lease(TOKEN, accounts.bob, accounts.alice, RENT, 0, 0, 0, FIRST_DUE),
            Err(Error::InvalidAmount)
        );
        assert_eq!(lease(&mut pm, accounts.alice, accounts.bob, 10_001, 0), Err(Error::InvalidFee));
        // The boundary fee of 100% is accepted.
        assert!(lease(&mut pm, accounts.alice, accounts.bob, 10_000, 0).is_ok());
    }

    #[ink::test]
    fn the_security_deposit_cap_follows_the_jurisdiction_rule() {
        let (mut pm, accounts) = setup();
        // 12 monthly periods per year -> annual rent 12_000; 200 bps cap = 240.
        pm.set_jurisdiction_compliance(TOKEN, jurisdiction(200)).unwrap();
        assert!(lease(&mut pm, accounts.alice, accounts.bob, 0, 240).is_ok());
        assert_eq!(
            lease(&mut pm, accounts.alice, accounts.bob, 0, 241),
            Err(Error::ComplianceViolation)
        );
    }

    // ── Paying rent ─────────────────────────────────────────────────────

    #[ink::test]
    fn rent_is_split_between_landlord_and_fee_beneficiary() {
        let (mut pm, accounts) = setup();
        pm.set_fee_beneficiary(accounts.charlie).unwrap();
        let id = lease(&mut pm, accounts.alice, accounts.bob, 500, 0).unwrap();

        let landlord_before = test::get_account_balance::<DefaultEnvironment>(accounts.alice).unwrap();
        let fee_before = test::get_account_balance::<DefaultEnvironment>(accounts.charlie).unwrap();

        test::set_caller::<DefaultEnvironment>(accounts.bob);
        test::transfer_in::<DefaultEnvironment>(RENT);
        pm.pay_rent(id).unwrap();

        // 5% of 1_000 = 50 to the beneficiary, 950 to the landlord.
        let landlord_after = test::get_account_balance::<DefaultEnvironment>(accounts.alice).unwrap();
        let fee_after = test::get_account_balance::<DefaultEnvironment>(accounts.charlie).unwrap();
        assert_eq!(landlord_after - landlord_before, 950);
        assert_eq!(fee_after - fee_before, 50);
        assert_eq!(pm.get_lease(id).unwrap().next_due, FIRST_DUE + PERIOD);
        assert_eq!(pm.get_property_analytics(TOKEN).rent_collected, RENT);
    }

    #[ink::test]
    fn only_the_tenant_can_pay_and_only_the_exact_rent() {
        let (mut pm, accounts) = setup();
        let id = lease(&mut pm, accounts.alice, accounts.bob, 0, 0).unwrap();

        test::set_caller::<DefaultEnvironment>(accounts.charlie);
        test::set_value_transferred::<DefaultEnvironment>(RENT);
        assert_eq!(pm.pay_rent(id), Err(Error::NotTenant));

        test::set_caller::<DefaultEnvironment>(accounts.bob);
        test::set_value_transferred::<DefaultEnvironment>(RENT - 1);
        assert_eq!(pm.pay_rent(id), Err(Error::InvalidAmount));
        test::set_value_transferred::<DefaultEnvironment>(RENT + 1);
        assert_eq!(pm.pay_rent(id), Err(Error::InvalidAmount));
        assert_eq!(pm.get_lease(id).unwrap().next_due, FIRST_DUE);
    }

    #[ink::test]
    fn paying_rent_on_an_unknown_lease_fails() {
        let (mut pm, accounts) = setup();
        test::set_caller::<DefaultEnvironment>(accounts.bob);
        assert_eq!(pm.pay_rent(99), Err(Error::NotFound));
    }

    // ── Ending a lease ──────────────────────────────────────────────────

    #[ink::test]
    fn ending_a_lease_closes_it_and_blocks_further_rent() {
        let (mut pm, accounts) = setup();
        let id = lease(&mut pm, accounts.alice, accounts.bob, 0, 0).unwrap();
        pm.end_lease(id).unwrap();
        assert_eq!(pm.get_lease(id).unwrap().status, LeaseStatus::Ended);

        test::set_caller::<DefaultEnvironment>(accounts.bob);
        test::set_value_transferred::<DefaultEnvironment>(RENT);
        assert_eq!(pm.pay_rent(id), Err(Error::LeaseNotActive));

        test::set_caller::<DefaultEnvironment>(accounts.alice);
        assert_eq!(pm.end_lease(id), Err(Error::InvalidStatus));
    }

    #[ink::test]
    fn only_the_landlord_a_manager_or_the_admin_can_end_a_lease() {
        let (mut pm, accounts) = setup();
        test::set_caller::<DefaultEnvironment>(accounts.charlie);
        let id = lease(&mut pm, accounts.charlie, accounts.bob, 0, 0).unwrap();

        test::set_caller::<DefaultEnvironment>(accounts.bob); // the tenant
        assert_eq!(pm.end_lease(id), Err(Error::NotLandlordOrManager));
        test::set_caller::<DefaultEnvironment>(accounts.eve);
        assert_eq!(pm.end_lease(id), Err(Error::NotLandlordOrManager));

        test::set_caller::<DefaultEnvironment>(accounts.alice); // admin
        assert!(pm.end_lease(id).is_ok());
        assert_eq!(pm.end_lease(404), Err(Error::NotFound));
    }

    #[ink::test]
    fn the_dashboard_counts_active_leases() {
        let (mut pm, accounts) = setup();
        let a = lease(&mut pm, accounts.alice, accounts.bob, 0, 0).unwrap();
        lease(&mut pm, accounts.alice, accounts.charlie, 0, 0).unwrap();
        assert_eq!(pm.get_management_dashboard().active_leases, 2);
        pm.end_lease(a).unwrap();
        assert_eq!(pm.get_management_dashboard().active_leases, 1);
    }
}
