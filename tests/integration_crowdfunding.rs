/// # Integration Tests: Crowdfunding Campaign Lifecycle (Issue #920)
///
/// Campaigns move Draft -> Active -> Funded (or Cancelled), investors must
/// clear KYC and accreditation, milestones require admin approval plus
/// oracle verification before release, and cancelled campaigns refund.
///
/// Acceptance criteria tested:
///   check Campaign creation and creator-only activation
///   check Investment requires onboarding, KYC, and verified accreditation
///   check Reaching the target flips the campaign to Funded
///   check Milestone flow: add -> approve -> oracle verify -> release
///   check Failed campaigns refund each investor exactly once
#[cfg(test)]
mod integration_crowdfunding {
    use ink::env::{test, DefaultEnvironment};
    use propchain_crowdfunding::propchain_crowdfunding::{
        CampaignStatus, CrowdfundingError, MilestoneStatus, RealEstateCrowdfunding,
    };

    #[ink::test]
    fn campaign_activation_and_investor_gating() {
        let accounts = test::default_accounts::<DefaultEnvironment>();
        test::set_caller::<DefaultEnvironment>(accounts.alice);
        let mut cf = RealEstateCrowdfunding::new(accounts.alice);

        let campaign = cf
            .create_campaign("Harbor Lofts".into(), 100_000)
            .expect("campaign created");
        assert_eq!(
            cf.get_campaign(campaign).unwrap().status,
            CampaignStatus::Draft
        );

        // Only the creator (or admin) can activate.
        test::set_caller::<DefaultEnvironment>(accounts.bob);
        assert_eq!(
            cf.activate_campaign(campaign),
            Err(CrowdfundingError::Unauthorized)
        );

        // Un-onboarded investors cannot invest even on an active campaign.
        test::set_caller::<DefaultEnvironment>(accounts.alice);
        cf.activate_campaign(campaign).expect("creator activates");
        test::set_caller::<DefaultEnvironment>(accounts.bob);
        assert_eq!(
            cf.invest(campaign, 10_000),
            Err(CrowdfundingError::InvestorNotCompliant)
        );

        // Onboarding alone is insufficient: accreditation must be verified.
        cf.onboard_investor("US".into(), false)
            .expect("investor onboarded");
        assert_eq!(
            cf.invest(campaign, 10_000),
            Err(CrowdfundingError::AccreditationNotVerified)
        );
        assert!(!cf.is_accredited(accounts.bob));

        // Accreditation verification is admin-only and requires a profile...
        test::set_caller::<DefaultEnvironment>(accounts.charlie);
        assert_eq!(
            cf.verify_accreditation(accounts.bob),
            Err(CrowdfundingError::Unauthorized)
        );
        test::set_caller::<DefaultEnvironment>(accounts.alice);
        assert_eq!(
            cf.verify_accreditation(accounts.django),
            Err(CrowdfundingError::InvestorNotCompliant)
        );

        // ...after which investing works and aggregates per investor.
        cf.verify_accreditation(accounts.bob)
            .expect("admin verifies bob");
        assert!(cf.is_accredited(accounts.bob));
        test::set_caller::<DefaultEnvironment>(accounts.bob);
        ink::env::test::set_value_transferred::<DefaultEnvironment>(40_000);
        cf.invest(campaign, 40_000).expect("first investment");
        ink::env::test::set_value_transferred::<DefaultEnvironment>(20_000);
        cf.invest(campaign, 20_000).expect("top-up investment");
        assert_eq!(cf.get_investment(campaign, accounts.bob), 60_000);

        let stored = cf.get_campaign(campaign).unwrap();
        assert_eq!(stored.raised_amount, 60_000);
        assert_eq!(stored.investor_count, 1, "top-ups do not inflate the count");
        assert_eq!(stored.status, CampaignStatus::Active);

        // Investing into an unknown campaign fails cleanly.
        assert_eq!(
            cf.invest(999, 1_000),
            Err(CrowdfundingError::CampaignNotFound)
        );
    }

    #[ink::test]
    fn reaching_target_marks_campaign_funded() {
        let accounts = test::default_accounts::<DefaultEnvironment>();
        test::set_caller::<DefaultEnvironment>(accounts.alice);
        let mut cf = RealEstateCrowdfunding::new(accounts.alice);
        let campaign = cf
            .create_campaign("Midrise".into(), 50_000)
            .expect("created");
        cf.activate_campaign(campaign).expect("activated");

        for who in [accounts.bob, accounts.charlie] {
            test::set_caller::<DefaultEnvironment>(who);
            cf.onboard_investor("DE".into(), false)
                .expect("investor onboarded");
            test::set_caller::<DefaultEnvironment>(accounts.alice);
            cf.verify_accreditation(who).expect("accredited");
        }

        test::set_caller::<DefaultEnvironment>(accounts.bob);
        ink::env::test::set_value_transferred::<DefaultEnvironment>(30_000);
        cf.invest(campaign, 30_000).expect("bob invests");
        test::set_caller::<DefaultEnvironment>(accounts.charlie);
        ink::env::test::set_value_transferred::<DefaultEnvironment>(25_000);
        cf.invest(campaign, 25_000)
            .expect("charlie overshoots target");

        let stored = cf.get_campaign(campaign).unwrap();
        assert_eq!(stored.raised_amount, 55_000);
        assert_eq!(
            stored.status,
            CampaignStatus::Funded,
            "crossing the target funds the campaign"
        );
        assert_eq!(stored.investor_count, 2);
    }

    #[ink::test]
    fn milestone_release_requires_approval_and_oracle() {
        let accounts = test::default_accounts::<DefaultEnvironment>();
        test::set_caller::<DefaultEnvironment>(accounts.alice);
        let mut cf = RealEstateCrowdfunding::new(accounts.alice);
        let campaign = cf.create_campaign("Tower".into(), 200_000).expect("ok");
        cf.activate_campaign(campaign).expect("ok");

        // Only creator/admin can add milestones.
        test::set_caller::<DefaultEnvironment>(accounts.bob);
        assert_eq!(
            cf.add_milestone(campaign, "Foundations".into(), 50_000),
            Err(CrowdfundingError::Unauthorized)
        );

        test::set_caller::<DefaultEnvironment>(accounts.alice);
        let milestone = cf
            .add_milestone(campaign, "Foundations".into(), 50_000)
            .expect("milestone added");
        assert_eq!(
            cf.get_milestone(milestone).unwrap().status,
            MilestoneStatus::Pending
        );

        // Release before approval is rejected outright.
        assert_eq!(
            cf.release_milestone(milestone),
            Err(CrowdfundingError::MilestoneNotApproved)
        );

        // Approval is admin-only; oracle verification is oracle/admin-only.
        test::set_caller::<DefaultEnvironment>(accounts.bob);
        assert_eq!(
            cf.approve_milestone(milestone),
            Err(CrowdfundingError::Unauthorized)
        );
        assert_eq!(
            cf.oracle_verify_milestone(milestone, [7u8; 32]),
            Err(CrowdfundingError::Unauthorized)
        );

        test::set_caller::<DefaultEnvironment>(accounts.alice);
        cf.add_oracle(accounts.charlie).expect("oracle added");
        test::set_caller::<DefaultEnvironment>(accounts.charlie);
        cf.oracle_verify_milestone(milestone, [7u8; 32])
            .expect("authorized oracle verifies");
        assert!(cf.get_milestone(milestone).unwrap().oracle_verified);

        // Approved + verified unlocks release and capital accounting.
        test::set_caller::<DefaultEnvironment>(accounts.alice);
        cf.approve_milestone(milestone).expect("admin approves");
        cf.release_milestone(milestone).expect("released");
        let released = cf.get_milestone(milestone).unwrap();
        assert_eq!(released.status, MilestoneStatus::Released);

        // Double release is blocked by the status guard.
        assert_eq!(
            cf.release_milestone(milestone),
            Err(CrowdfundingError::MilestoneNotApproved)
        );
    }

    #[ink::test]
    fn failed_campaigns_refund_exactly_once() {
        let accounts = test::default_accounts::<DefaultEnvironment>();
        test::set_caller::<DefaultEnvironment>(accounts.alice);
        let mut cf = RealEstateCrowdfunding::new(accounts.alice);
        let campaign = cf.create_campaign("Doomed".into(), 80_000).expect("ok");
        cf.activate_campaign(campaign).expect("ok");

        test::set_caller::<DefaultEnvironment>(accounts.bob);
        cf.onboard_investor("FR".into(), false)
            .expect("investor onboarded");
        test::set_caller::<DefaultEnvironment>(accounts.alice);
        cf.verify_accreditation(accounts.bob).expect("ok");
        test::set_caller::<DefaultEnvironment>(accounts.bob);
        ink::env::test::set_value_transferred::<DefaultEnvironment>(25_000);
        cf.invest(campaign, 25_000).expect("invested");

        // Refunds only exist after the admin cancels the campaign.
        assert_eq!(
            cf.claim_refund(campaign),
            Err(CrowdfundingError::CampaignNotFailed)
        );

        // Cancellation itself is admin-gated.
        test::set_caller::<DefaultEnvironment>(accounts.charlie);
        assert_eq!(
            cf.fail_campaign(campaign),
            Err(CrowdfundingError::Unauthorized)
        );

        test::set_caller::<DefaultEnvironment>(accounts.alice);
        cf.fail_campaign(campaign).expect("cancelled");

        test::set_caller::<DefaultEnvironment>(accounts.bob);
        assert_eq!(cf.claim_refund(campaign), Ok(25_000));
        assert!(cf.is_refunded(campaign, accounts.bob));

        // The refund flag blocks double-dipping.
        assert_eq!(
            cf.claim_refund(campaign),
            Err(CrowdfundingError::AlreadyRefunded)
        );

        // Investors without positions have nothing to claim.
        test::set_caller::<DefaultEnvironment>(accounts.charlie);
        assert_eq!(
            cf.claim_refund(campaign),
            Err(CrowdfundingError::NoInvestmentFound)
        );
    }
}
