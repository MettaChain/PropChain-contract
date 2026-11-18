//! Unit tests for insurance policy claims and payouts (#1452).
//!
//! Setup mirrors the contract's own tests: an admin seeds a Fire risk pool with
//! liquidity and a risk assessment, then a policyholder (bob) buys a policy.

#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(test)]
mod tests {
    use ink::env::{test, DefaultEnvironment};
    use propchain_insurance::propchain_insurance::{
        ClaimStatus, CoverageType, InsuranceError, PropertyInsurance,
    };

    type Accounts = test::DefaultAccounts<DefaultEnvironment>;

    const COVERAGE_AMOUNT: u128 = 100_000_000;
    const POOL_LIQUIDITY: u128 = 1_000_000_000_000;
    const CLAIM_AMOUNT: u128 = 40_000_000;
    const DURATION_SECONDS: u64 = 86_400 * 365;
    const START: u64 = 3_000_000;

    fn s(text: &str) -> String {
        text.to_string()
    }

    /// Returns `(contract, accounts, pool_id, policy_id)`; the policy belongs to bob.
    fn setup() -> (PropertyInsurance, Accounts, u64, u64) {
        let accounts = test::default_accounts::<DefaultEnvironment>();
        test::set_caller::<DefaultEnvironment>(accounts.alice);
        // Past the 30-day default claim cooldown measured from timestamp 0.
        test::set_block_timestamp::<DefaultEnvironment>(START);
        let mut c = PropertyInsurance::new(accounts.alice);

        let pool_id = c
            .create_risk_pool(s("Fire Pool"), CoverageType::Fire, 8000, 500_000_000_000u128)
            .expect("pool created");
        c.update_risk_assessment(1, 75, 80, 85, 90, DURATION_SECONDS)
            .expect("assessment seeded");
        test::set_value_transferred::<DefaultEnvironment>(POOL_LIQUIDITY);
        c.provide_pool_liquidity(pool_id).expect("liquidity provided");

        let calc = c
            .calculate_premium(1, COVERAGE_AMOUNT, CoverageType::Fire)
            .expect("premium calculated");
        test::set_caller::<DefaultEnvironment>(accounts.bob);
        test::set_value_transferred::<DefaultEnvironment>(calc.annual_premium * 2);
        let policy_id = c
            .create_policy(1, CoverageType::Fire, COVERAGE_AMOUNT, 1, DURATION_SECONDS, s("ipfs://policy"))
            .expect("policy created");
        (c, accounts, pool_id, policy_id)
    }

    fn submit(c: &mut PropertyInsurance, amount: u128, policy_id: u64) -> Result<u64, InsuranceError> {
        let accounts = test::default_accounts::<DefaultEnvironment>();
        test::set_caller::<DefaultEnvironment>(accounts.bob);
        c.submit_claim(policy_id, amount, s("Fire damage"), s("ipfs://evidence"))
    }

    fn as_admin(accounts: &Accounts) {
        test::set_caller::<DefaultEnvironment>(accounts.alice);
    }

    // ── Submitting claims ───────────────────────────────────────────────

    #[ink::test]
    fn a_policyholder_can_submit_a_claim() {
        let (mut c, accounts, _, policy_id) = setup();
        let claim_id = submit(&mut c, CLAIM_AMOUNT, policy_id).unwrap();

        let claim = c.get_claim(claim_id).unwrap();
        assert_eq!(claim.status, ClaimStatus::Pending);
        assert_eq!(claim.claimant, accounts.bob);
        assert_eq!(claim.claim_amount, CLAIM_AMOUNT);
        assert_eq!(claim.payout_amount, 0);
        assert_eq!(claim.assessor, None);
        assert_eq!(c.get_policy_claims(policy_id), vec![claim_id]);
        assert_eq!(c.get_policy(policy_id).unwrap().claims_count, 1);
    }

    #[ink::test]
    fn only_the_policyholder_can_claim() {
        let (mut c, accounts, _, policy_id) = setup();
        test::set_caller::<DefaultEnvironment>(accounts.charlie);
        assert_eq!(
            c.submit_claim(policy_id, CLAIM_AMOUNT, s("x"), s("y")),
            Err(InsuranceError::Unauthorized)
        );
        assert!(c.get_policy_claims(policy_id).is_empty());
    }

    #[ink::test]
    fn claims_beyond_the_coverage_amount_are_rejected() {
        let (mut c, _, _, policy_id) = setup();
        assert_eq!(
            submit(&mut c, COVERAGE_AMOUNT + 1, policy_id),
            Err(InsuranceError::ClaimExceedsCoverage)
        );
        // Exactly the coverage amount is allowed.
        assert!(submit(&mut c, COVERAGE_AMOUNT, policy_id).is_ok());
    }

    #[ink::test]
    fn unknown_policies_and_expired_policies_cannot_be_claimed_against() {
        let (mut c, _, _, policy_id) = setup();
        assert_eq!(submit(&mut c, 1_000, 999), Err(InsuranceError::PolicyNotFound));

        test::set_block_timestamp::<DefaultEnvironment>(START + DURATION_SECONDS + 1_000);
        assert_eq!(submit(&mut c, 1_000, policy_id), Err(InsuranceError::PolicyExpired));
    }

    // ── Processing: approval and payout ─────────────────────────────────

    #[ink::test]
    fn an_approved_claim_pays_out_the_amount_minus_the_deductible() {
        let (mut c, accounts, pool_id, policy_id) = setup();
        let claim_id = submit(&mut c, CLAIM_AMOUNT, policy_id).unwrap();
        let deductible = c.get_policy(policy_id).unwrap().deductible;
        let capital_before = c.get_pool(pool_id).unwrap().available_capital;

        as_admin(&accounts);
        c.process_claim(claim_id, true, s("ipfs://report"), s("")).unwrap();

        let claim = c.get_claim(claim_id).unwrap();
        assert_eq!(claim.status, ClaimStatus::Paid);
        assert_eq!(claim.payout_amount, CLAIM_AMOUNT - deductible);
        assert_eq!(claim.assessor, Some(accounts.alice));
        assert!(claim.processed_at.is_some());
        // The pool was debited by the payout.
        let capital_after = c.get_pool(pool_id).unwrap().available_capital;
        assert_eq!(capital_before - capital_after, CLAIM_AMOUNT - deductible);
    }

    #[ink::test]
    fn a_claim_at_or_below_the_deductible_pays_nothing() {
        let (mut c, accounts, pool_id, policy_id) = setup();
        let deductible = c.get_policy(policy_id).unwrap().deductible;
        let claim_id = submit(&mut c, deductible, policy_id).unwrap();
        let before = c.get_pool(pool_id).unwrap().available_capital;
        as_admin(&accounts);
        c.process_claim(claim_id, true, s("report"), s("")).unwrap();
        assert_eq!(c.get_claim(claim_id).unwrap().payout_amount, 0);
        assert_eq!(c.get_pool(pool_id).unwrap().available_capital, before);
    }

    #[ink::test]
    fn a_rejected_claim_records_the_reason_and_pays_nothing() {
        let (mut c, accounts, pool_id, policy_id) = setup();
        let claim_id = submit(&mut c, CLAIM_AMOUNT, policy_id).unwrap();
        let before = c.get_pool(pool_id).unwrap().available_capital;

        as_admin(&accounts);
        c.process_claim(claim_id, false, s("report"), s("Damage not covered")).unwrap();

        let claim = c.get_claim(claim_id).unwrap();
        assert_eq!(claim.status, ClaimStatus::Rejected);
        assert_eq!(claim.rejection_reason, "Damage not covered");
        assert_eq!(claim.payout_amount, 0);
        assert_eq!(c.get_pool(pool_id).unwrap().available_capital, before);
    }

    // ── Authorization and lifecycle ─────────────────────────────────────

    #[ink::test]
    fn only_the_admin_or_an_authorized_assessor_can_process_claims() {
        let (mut c, accounts, _, policy_id) = setup();
        let claim_id = submit(&mut c, CLAIM_AMOUNT, policy_id).unwrap();

        test::set_caller::<DefaultEnvironment>(accounts.charlie);
        assert_eq!(
            c.process_claim(claim_id, true, s("r"), s("")),
            Err(InsuranceError::Unauthorized)
        );
        // The policyholder cannot approve their own claim.
        test::set_caller::<DefaultEnvironment>(accounts.bob);
        assert_eq!(
            c.process_claim(claim_id, true, s("r"), s("")),
            Err(InsuranceError::Unauthorized)
        );

        as_admin(&accounts);
        c.authorize_assessor(accounts.charlie).unwrap();
        test::set_caller::<DefaultEnvironment>(accounts.charlie);
        c.process_claim(claim_id, true, s("r"), s("")).unwrap();
        assert_eq!(c.get_claim(claim_id).unwrap().assessor, Some(accounts.charlie));
    }

    #[ink::test]
    fn a_processed_claim_cannot_be_processed_again() {
        let (mut c, accounts, _, policy_id) = setup();
        let claim_id = submit(&mut c, CLAIM_AMOUNT, policy_id).unwrap();
        as_admin(&accounts);
        c.process_claim(claim_id, false, s("r"), s("no")).unwrap();
        assert_eq!(
            c.process_claim(claim_id, true, s("r"), s("")),
            Err(InsuranceError::ClaimAlreadyProcessed)
        );
        assert_eq!(c.get_claim(claim_id).unwrap().status, ClaimStatus::Rejected);
    }

    #[ink::test]
    fn processing_an_unknown_claim_fails() {
        let (mut c, accounts, _, _) = setup();
        as_admin(&accounts);
        assert_eq!(
            c.process_claim(42, true, s("r"), s("")),
            Err(InsuranceError::ClaimNotFound)
        );
    }

    // ── Cooldown ────────────────────────────────────────────────────────

    #[ink::test]
    fn a_second_claim_inside_the_cooldown_is_rejected_until_it_elapses() {
        let (mut c, accounts, _, policy_id) = setup();
        let first = submit(&mut c, CLAIM_AMOUNT, policy_id).unwrap();
        as_admin(&accounts);
        c.process_claim(first, true, s("r"), s("")).unwrap();

        assert_eq!(
            submit(&mut c, 1_000_000, policy_id),
            Err(InsuranceError::CooldownPeriodActive)
        );

        // 30 days later the cooldown has elapsed.
        test::set_block_timestamp::<DefaultEnvironment>(START + 86_400 * 30 + 1);
        assert!(submit(&mut c, 1_000_000, policy_id).is_ok());
    }

    #[ink::test]
    fn the_admin_can_shorten_the_cooldown_and_others_cannot() {
        let (mut c, accounts, _, policy_id) = setup();
        test::set_caller::<DefaultEnvironment>(accounts.bob);
        assert_eq!(c.set_claim_cooldown(0), Err(InsuranceError::Unauthorized));

        as_admin(&accounts);
        c.set_claim_cooldown(0).unwrap();
        let first = submit(&mut c, CLAIM_AMOUNT, policy_id).unwrap();
        as_admin(&accounts);
        c.process_claim(first, true, s("r"), s("")).unwrap();
        // With no cooldown a second claim is accepted immediately.
        assert!(submit(&mut c, 1_000_000, policy_id).is_ok());
    }
}
