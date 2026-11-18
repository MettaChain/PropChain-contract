//! Unit tests for the crowdfunding campaign state machine (#1453).
//!
//! States: `Draft -> Active -> Funded` on reaching the target, `Active ->
//! Cancelled` via `fail_campaign`, with refunds available only once a campaign
//! is `Cancelled`. Only transitions the contract actually enforces are asserted.

#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(test)]
mod tests {
    use ink::env::{test, DefaultEnvironment};
    use ink::primitives::AccountId;
    use propchain_crowdfunding::propchain_crowdfunding::{
        CampaignStatus, CrowdfundingError, RealEstateCrowdfunding,
    };

    type Accounts = test::DefaultAccounts<DefaultEnvironment>;

    const TARGET: u128 = 100_000;

    /// Admin = alice. Returns the contract plus a freshly created *Draft* campaign.
    fn setup() -> (RealEstateCrowdfunding, Accounts, u64) {
        let accounts = test::default_accounts::<DefaultEnvironment>();
        test::set_caller::<DefaultEnvironment>(accounts.alice);
        let mut cf = RealEstateCrowdfunding::new(accounts.alice);
        let id = cf.create_campaign("Harbor Lofts".into(), TARGET).unwrap();
        (cf, accounts, id)
    }

    fn activate(cf: &mut RealEstateCrowdfunding, accounts: &Accounts, id: u64) {
        test::set_caller::<DefaultEnvironment>(accounts.alice);
        cf.activate_campaign(id).unwrap();
    }

    /// Onboard `who` as an accredited investor (admin verifies).
    fn accredit(cf: &mut RealEstateCrowdfunding, accounts: &Accounts, who: AccountId) {
        test::set_caller::<DefaultEnvironment>(who);
        cf.onboard_investor("US".into(), false).unwrap();
        test::set_caller::<DefaultEnvironment>(accounts.alice);
        cf.verify_accreditation(who).unwrap();
    }

    fn invest(
        cf: &mut RealEstateCrowdfunding,
        who: AccountId,
        id: u64,
        amount: u128,
    ) -> Result<(), CrowdfundingError> {
        test::set_caller::<DefaultEnvironment>(who);
        test::set_value_transferred::<DefaultEnvironment>(amount);
        cf.invest(id, amount)
    }

    fn status(cf: &RealEstateCrowdfunding, id: u64) -> CampaignStatus {
        cf.get_campaign(id).unwrap().status
    }

    // ── Draft ───────────────────────────────────────────────────────────

    #[ink::test]
    fn a_new_campaign_starts_in_draft_with_nothing_raised() {
        let (cf, accounts, id) = setup();
        let c = cf.get_campaign(id).unwrap();
        assert_eq!(c.status, CampaignStatus::Draft);
        assert_eq!((c.raised_amount, c.investor_count), (0, 0));
        assert_eq!(c.target_amount, TARGET);
        assert_eq!(c.creator, accounts.alice);
        assert!(cf.get_campaign(id + 1).is_none());
    }

    #[ink::test]
    fn a_draft_campaign_does_not_accept_investments() {
        let (mut cf, accounts, id) = setup();
        accredit(&mut cf, &accounts, accounts.bob);
        assert_eq!(
            invest(&mut cf, accounts.bob, id, 10_000),
            Err(CrowdfundingError::CampaignNotActive)
        );
        assert_eq!(cf.get_campaign(id).unwrap().raised_amount, 0);
    }

    // ── Draft -> Active ─────────────────────────────────────────────────

    #[ink::test]
    fn only_the_creator_or_admin_can_activate() {
        let (mut cf, accounts, id) = setup();
        test::set_caller::<DefaultEnvironment>(accounts.bob);
        assert_eq!(cf.activate_campaign(id), Err(CrowdfundingError::Unauthorized));
        assert_eq!(status(&cf, id), CampaignStatus::Draft);

        activate(&mut cf, &accounts, id);
        assert_eq!(status(&cf, id), CampaignStatus::Active);
    }

    #[ink::test]
    fn activating_an_unknown_campaign_fails() {
        let (mut cf, _, _) = setup();
        assert_eq!(cf.activate_campaign(99), Err(CrowdfundingError::CampaignNotFound));
    }

    // ── Active: investing ───────────────────────────────────────────────

    #[ink::test]
    fn investments_accumulate_while_active() {
        let (mut cf, accounts, id) = setup();
        activate(&mut cf, &accounts, id);
        accredit(&mut cf, &accounts, accounts.bob);
        accredit(&mut cf, &accounts, accounts.charlie);

        invest(&mut cf, accounts.bob, id, 30_000).unwrap();
        invest(&mut cf, accounts.bob, id, 10_000).unwrap();
        invest(&mut cf, accounts.charlie, id, 20_000).unwrap();

        let c = cf.get_campaign(id).unwrap();
        assert_eq!(c.status, CampaignStatus::Active);
        assert_eq!(c.raised_amount, 60_000);
        // A repeat investor is counted once.
        assert_eq!(c.investor_count, 2);
        assert_eq!(cf.get_investment(id, accounts.bob), 40_000);
    }

    #[ink::test]
    fn the_transferred_value_must_match_the_amount() {
        let (mut cf, accounts, id) = setup();
        activate(&mut cf, &accounts, id);
        accredit(&mut cf, &accounts, accounts.bob);
        test::set_caller::<DefaultEnvironment>(accounts.bob);
        test::set_value_transferred::<DefaultEnvironment>(5_000);
        assert_eq!(cf.invest(id, 10_000), Err(CrowdfundingError::ValueMismatch));
        assert_eq!(cf.get_campaign(id).unwrap().raised_amount, 0);
    }

    // ── Active -> Funded ────────────────────────────────────────────────

    #[ink::test]
    fn reaching_the_target_exactly_marks_the_campaign_funded() {
        let (mut cf, accounts, id) = setup();
        activate(&mut cf, &accounts, id);
        accredit(&mut cf, &accounts, accounts.bob);

        invest(&mut cf, accounts.bob, id, TARGET - 1).unwrap();
        assert_eq!(status(&cf, id), CampaignStatus::Active);
        invest(&mut cf, accounts.bob, id, 1).unwrap();
        assert_eq!(status(&cf, id), CampaignStatus::Funded);
        assert_eq!(cf.get_campaign(id).unwrap().raised_amount, TARGET);
    }

    #[ink::test]
    fn a_funded_campaign_rejects_further_investment() {
        let (mut cf, accounts, id) = setup();
        activate(&mut cf, &accounts, id);
        accredit(&mut cf, &accounts, accounts.bob);
        invest(&mut cf, accounts.bob, id, TARGET).unwrap();
        assert_eq!(status(&cf, id), CampaignStatus::Funded);
        assert_eq!(
            invest(&mut cf, accounts.bob, id, 1_000),
            Err(CrowdfundingError::CampaignNotActive)
        );
        assert_eq!(cf.get_campaign(id).unwrap().raised_amount, TARGET);
    }

    #[ink::test]
    fn overfunding_in_one_investment_still_ends_funded() {
        let (mut cf, accounts, id) = setup();
        activate(&mut cf, &accounts, id);
        accredit(&mut cf, &accounts, accounts.bob);
        invest(&mut cf, accounts.bob, id, TARGET * 2).unwrap();
        assert_eq!(status(&cf, id), CampaignStatus::Funded);
    }

    // ── Active -> Cancelled and refunds ─────────────────────────────────

    #[ink::test]
    fn only_the_admin_can_fail_a_campaign() {
        let (mut cf, accounts, id) = setup();
        activate(&mut cf, &accounts, id);
        test::set_caller::<DefaultEnvironment>(accounts.bob);
        assert_eq!(cf.fail_campaign(id), Err(CrowdfundingError::Unauthorized));
        assert_eq!(status(&cf, id), CampaignStatus::Active);

        test::set_caller::<DefaultEnvironment>(accounts.alice);
        cf.fail_campaign(id).unwrap();
        assert_eq!(status(&cf, id), CampaignStatus::Cancelled);
    }

    #[ink::test]
    fn a_cancelled_campaign_rejects_investment() {
        let (mut cf, accounts, id) = setup();
        activate(&mut cf, &accounts, id);
        accredit(&mut cf, &accounts, accounts.bob);
        test::set_caller::<DefaultEnvironment>(accounts.alice);
        cf.fail_campaign(id).unwrap();
        assert_eq!(
            invest(&mut cf, accounts.bob, id, 1_000),
            Err(CrowdfundingError::CampaignNotActive)
        );
    }

    #[ink::test]
    fn investors_are_refunded_once_after_cancellation() {
        let (mut cf, accounts, id) = setup();
        activate(&mut cf, &accounts, id);
        accredit(&mut cf, &accounts, accounts.bob);
        invest(&mut cf, accounts.bob, id, 25_000).unwrap();

        test::set_caller::<DefaultEnvironment>(accounts.alice);
        cf.fail_campaign(id).unwrap();

        test::set_caller::<DefaultEnvironment>(accounts.bob);
        assert!(!cf.is_refunded(id, accounts.bob));
        assert_eq!(cf.claim_refund(id), Ok(25_000));
        assert!(cf.is_refunded(id, accounts.bob));
        assert_eq!(cf.claim_refund(id), Err(CrowdfundingError::AlreadyRefunded));
    }

    #[ink::test]
    fn refunds_are_not_available_unless_the_campaign_was_cancelled() {
        let (mut cf, accounts, id) = setup();
        activate(&mut cf, &accounts, id);
        accredit(&mut cf, &accounts, accounts.bob);
        invest(&mut cf, accounts.bob, id, 10_000).unwrap();
        test::set_caller::<DefaultEnvironment>(accounts.bob);
        assert_eq!(cf.claim_refund(id), Err(CrowdfundingError::CampaignNotFailed));

        // Funded campaigns are not refundable either.
        let id2 = {
            test::set_caller::<DefaultEnvironment>(accounts.alice);
            let id2 = cf.create_campaign("Second".into(), 5_000).unwrap();
            cf.activate_campaign(id2).unwrap();
            id2
        };
        invest(&mut cf, accounts.bob, id2, 5_000).unwrap();
        assert_eq!(status(&cf, id2), CampaignStatus::Funded);
        test::set_caller::<DefaultEnvironment>(accounts.bob);
        assert_eq!(cf.claim_refund(id2), Err(CrowdfundingError::CampaignNotFailed));
    }

    #[ink::test]
    fn a_refund_needs_an_actual_investment() {
        let (mut cf, accounts, id) = setup();
        activate(&mut cf, &accounts, id);
        test::set_caller::<DefaultEnvironment>(accounts.alice);
        cf.fail_campaign(id).unwrap();
        test::set_caller::<DefaultEnvironment>(accounts.eve);
        assert_eq!(cf.claim_refund(id), Err(CrowdfundingError::NoInvestmentFound));
        assert_eq!(cf.claim_refund(77), Err(CrowdfundingError::CampaignNotFound));
    }

    // ── Compliance gating on investing ──────────────────────────────────

    #[ink::test]
    fn investors_must_be_onboarded_and_accredited() {
        let (mut cf, accounts, id) = setup();
        activate(&mut cf, &accounts, id);
        assert_eq!(
            invest(&mut cf, accounts.bob, id, 1_000),
            Err(CrowdfundingError::InvestorNotCompliant)
        );
        test::set_caller::<DefaultEnvironment>(accounts.bob);
        cf.onboard_investor("US".into(), false).unwrap();
        assert_eq!(
            invest(&mut cf, accounts.bob, id, 1_000),
            Err(CrowdfundingError::AccreditationNotVerified)
        );
    }

    #[ink::test]
    fn campaigns_progress_independently() {
        let (mut cf, accounts, a) = setup();
        test::set_caller::<DefaultEnvironment>(accounts.alice);
        let b = cf.create_campaign("Other".into(), 1_000).unwrap();
        activate(&mut cf, &accounts, a);
        assert_eq!(status(&cf, a), CampaignStatus::Active);
        assert_eq!(status(&cf, b), CampaignStatus::Draft);
        assert_eq!(cf.get_campaign_count(), 2);
    }
}
