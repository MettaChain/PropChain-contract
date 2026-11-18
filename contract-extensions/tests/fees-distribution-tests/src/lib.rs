//! Unit tests for fee calculation and distribution (#1460).
//!
//! Distribution facts these tests pin: `distribute_fees` pays the *validator
//! share* of the treasury, split equally with the division remainder kept, and
//! leaves everything not paid out (treasury share + remainder) in the treasury.

#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(test)]
mod tests {
    use ink::env::{test, DefaultEnvironment};
    use propchain_fees::propchain_fees::{
        FeeCalculationMethod, FeeConfig, FeeError, FeeManager,
    };
    use propchain_traits::{BasisPoints, FeeOperation};

    type Accounts = test::DefaultAccounts<DefaultEnvironment>;

    fn setup() -> (FeeManager, Accounts) {
        let accounts = test::default_accounts::<DefaultEnvironment>();
        test::set_caller::<DefaultEnvironment>(accounts.alice);
        (FeeManager::new(1_000, 100, 5_000), accounts)
    }

    fn fixed(base: u128, min: u128, max: u128) -> FeeConfig {
        FeeConfig {
            base_fee: base,
            min_fee: min,
            max_fee: max,
            congestion_sensitivity: 80,
            demand_factor_bp: BasisPoints::new(500),
            calculation_method: FeeCalculationMethod::Fixed,
            last_updated: 0,
        }
    }

    const ALL_OPERATIONS: [FeeOperation; 8] = [
        FeeOperation::RegisterProperty,
        FeeOperation::TransferProperty,
        FeeOperation::UpdateMetadata,
        FeeOperation::CreateEscrow,
        FeeOperation::ReleaseEscrow,
        FeeOperation::PremiumListingBid,
        FeeOperation::IssueBadge,
        FeeOperation::OracleUpdate,
    ];

    /// Collect `amount` into the treasury (admin only).
    fn collect(f: &mut FeeManager, accounts: &Accounts, amount: u128) {
        test::set_caller::<DefaultEnvironment>(accounts.alice);
        f.record_fee_collected(FeeOperation::TransferProperty, amount, accounts.bob).unwrap();
    }

    // ── Fee calculation ─────────────────────────────────────────────────

    #[ink::test]
    fn default_fees_stay_within_the_configured_bounds_for_every_operation() {
        let (f, _) = setup();
        for op in ALL_OPERATIONS {
            let fee = f.calculate_fee(op);
            assert!((100..=5_000).contains(&fee), "{op:?} priced at {fee}");
        }
    }

    #[ink::test]
    fn a_fixed_override_pins_the_fee_for_that_operation_only() {
        let (mut f, _) = setup();
        let before_other = f.calculate_fee(FeeOperation::TransferProperty);
        f.set_operation_config(FeeOperation::OracleUpdate, fixed(777, 500, 900)).unwrap();
        assert_eq!(f.calculate_fee(FeeOperation::OracleUpdate), 777);
        assert_eq!(f.calculate_fee(FeeOperation::TransferProperty), before_other);
    }

    #[ink::test]
    fn invalid_operation_configs_are_rejected_without_changing_pricing() {
        let (mut f, _) = setup();
        let before = f.calculate_fee(FeeOperation::UpdateMetadata);
        assert_eq!(
            f.set_operation_config(FeeOperation::UpdateMetadata, fixed(500, 900, 1_000)),
            Err(FeeError::InvalidConfig)
        ); // min > max
        assert_eq!(
            f.set_operation_config(FeeOperation::UpdateMetadata, fixed(400, 500, 1_000)),
            Err(FeeError::InvalidConfig)
        ); // base < min
        assert_eq!(f.calculate_fee(FeeOperation::UpdateMetadata), before);
    }

    #[ink::test]
    fn only_the_admin_can_change_pricing() {
        let (mut f, accounts) = setup();
        test::set_caller::<DefaultEnvironment>(accounts.bob);
        assert_eq!(
            f.set_operation_config(FeeOperation::OracleUpdate, fixed(1, 1, 2)),
            Err(FeeError::Unauthorized)
        );
    }

    #[ink::test]
    fn the_recommended_fee_matches_the_calculated_fee() {
        let (f, _) = setup();
        for op in ALL_OPERATIONS {
            assert_eq!(f.get_recommended_fee(op), f.calculate_fee(op));
        }
    }

    // ── Fee collection ──────────────────────────────────────────────────

    #[ink::test]
    fn collected_fees_accumulate_in_the_treasury() {
        let (mut f, accounts) = setup();
        assert_eq!(f.fee_treasury(), 0);
        collect(&mut f, &accounts, 700);
        collect(&mut f, &accounts, 300);
        assert_eq!(f.fee_treasury(), 1_000);
    }

    #[ink::test]
    fn only_the_admin_can_record_collected_fees() {
        let (mut f, accounts) = setup();
        test::set_caller::<DefaultEnvironment>(accounts.bob);
        assert_eq!(
            f.record_fee_collected(FeeOperation::TransferProperty, 100, accounts.bob),
            Err(FeeError::Unauthorized)
        );
        assert_eq!(f.fee_treasury(), 0);
    }

    // ── Distribution ────────────────────────────────────────────────────

    #[ink::test]
    fn the_validator_share_is_split_equally_and_the_rest_stays_in_the_treasury() {
        let (mut f, accounts) = setup();
        f.add_validator(accounts.bob).unwrap();
        f.add_validator(accounts.charlie).unwrap();
        collect(&mut f, &accounts, 10_000);

        f.distribute_fees().unwrap();
        // Default split is 50% validators: 5_000 -> 2_500 each.
        assert_eq!(f.pending_reward(accounts.bob), 2_500);
        assert_eq!(f.pending_reward(accounts.charlie), 2_500);
        assert_eq!(f.fee_treasury(), 5_000);
    }

    #[ink::test]
    fn a_division_remainder_stays_in_the_treasury_instead_of_being_lost() {
        let (mut f, accounts) = setup();
        for v in [accounts.bob, accounts.charlie, accounts.django] {
            f.add_validator(v).unwrap();
        }
        collect(&mut f, &accounts, 10);

        f.distribute_fees().unwrap();
        // 50% of 10 = 5; three validators get 1 each (3 paid out), 7 remain.
        for v in [accounts.bob, accounts.charlie, accounts.django] {
            assert_eq!(f.pending_reward(v), 1);
        }
        assert_eq!(f.fee_treasury(), 7);
    }

    #[ink::test]
    fn distribution_without_validators_or_fees_changes_nothing() {
        let (mut f, accounts) = setup();
        // No fees at all.
        f.distribute_fees().unwrap();
        assert_eq!(f.fee_treasury(), 0);
        // Fees but no validators.
        collect(&mut f, &accounts, 1_000);
        f.distribute_fees().unwrap();
        assert_eq!(f.fee_treasury(), 1_000);
        assert_eq!(f.pending_reward(accounts.bob), 0);
    }

    #[ink::test]
    fn a_removed_validator_receives_nothing() {
        let (mut f, accounts) = setup();
        f.add_validator(accounts.bob).unwrap();
        f.add_validator(accounts.charlie).unwrap();
        f.remove_validator(accounts.bob).unwrap();
        collect(&mut f, &accounts, 10_000);
        f.distribute_fees().unwrap();
        assert_eq!(f.pending_reward(accounts.bob), 0);
        assert_eq!(f.pending_reward(accounts.charlie), 5_000);
    }

    #[ink::test]
    fn adding_a_validator_twice_does_not_double_their_share() {
        let (mut f, accounts) = setup();
        f.add_validator(accounts.bob).unwrap();
        f.add_validator(accounts.bob).unwrap();
        collect(&mut f, &accounts, 10_000);
        f.distribute_fees().unwrap();
        assert_eq!(f.pending_reward(accounts.bob), 5_000);
    }

    #[ink::test]
    fn distribution_rates_cannot_exceed_one_hundred_percent() {
        let (mut f, accounts) = setup();
        assert_eq!(
            f.set_distribution_rates(BasisPoints::new(6_000), BasisPoints::new(4_001)),
            Err(FeeError::InvalidConfig)
        );
        f.set_distribution_rates(BasisPoints::new(6_000), BasisPoints::new(4_000)).unwrap();

        f.add_validator(accounts.bob).unwrap();
        collect(&mut f, &accounts, 10_000);
        f.distribute_fees().unwrap();
        assert_eq!(f.pending_reward(accounts.bob), 6_000);
        assert_eq!(f.fee_treasury(), 4_000);
    }

    #[ink::test]
    fn a_full_validator_share_pays_the_whole_treasury_out() {
        let (mut f, accounts) = setup();
        f.set_distribution_rates(BasisPoints::new(10_000), BasisPoints::new(0)).unwrap();
        f.add_validator(accounts.bob).unwrap();
        f.add_validator(accounts.charlie).unwrap();
        collect(&mut f, &accounts, 10_000);
        f.distribute_fees().unwrap();
        assert_eq!(f.pending_reward(accounts.bob) + f.pending_reward(accounts.charlie), 10_000);
        assert_eq!(f.fee_treasury(), 0);
    }

    #[ink::test]
    fn only_the_admin_can_manage_validators_rates_and_distribution() {
        let (mut f, accounts) = setup();
        test::set_caller::<DefaultEnvironment>(accounts.bob);
        assert_eq!(f.add_validator(accounts.bob), Err(FeeError::Unauthorized));
        assert_eq!(f.remove_validator(accounts.bob), Err(FeeError::Unauthorized));
        assert_eq!(
            f.set_distribution_rates(BasisPoints::new(1), BasisPoints::new(1)),
            Err(FeeError::Unauthorized)
        );
        assert_eq!(f.distribute_fees(), Err(FeeError::Unauthorized));
    }

    // ── Claiming rewards ────────────────────────────────────────────────

    #[ink::test]
    fn a_validator_claims_pending_rewards_once() {
        let (mut f, accounts) = setup();
        f.add_validator(accounts.bob).unwrap();
        collect(&mut f, &accounts, 10_000);
        f.distribute_fees().unwrap();

        test::set_caller::<DefaultEnvironment>(accounts.bob);
        assert_eq!(f.claim_rewards(), Ok(5_000));
        assert_eq!(f.pending_reward(accounts.bob), 0);
        assert_eq!(f.claim_rewards(), Ok(0));
    }

    #[ink::test]
    fn accounts_without_rewards_claim_zero() {
        let (mut f, accounts) = setup();
        test::set_caller::<DefaultEnvironment>(accounts.eve);
        assert_eq!(f.claim_rewards(), Ok(0));
    }
}
