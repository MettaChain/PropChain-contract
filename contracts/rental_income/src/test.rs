#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::{testutils::Address as _, Env, Address};

    #[test]
    fn test_proportional_distribution() {
        let env = Env::default();
        let contract = RentalContract;

        let owner1 = Address::generate(&env);
        let owner2 = Address::generate(&env);
        let owner3 = Address::generate(&env);

        let shares = vec![
            Share { owner: owner1.clone(), percentage: 5000 },
            Share { owner: owner2.clone(), percentage: 3000 },
            Share { owner: owner3.clone(), percentage: 2000 },
        ];

        contract.distribute_rental_income(env.clone(), 1, 1000, shares);

        assert_eq!(contract.get_unclaimed_rental_income(env.clone(), owner1.clone(), 1), 500);
        assert_eq!(contract.get_unclaimed_rental_income(env.clone(), owner2.clone(), 1), 300);
        assert_eq!(contract.get_unclaimed_rental_income(env.clone(), owner3.clone(), 1), 200);
    }

    #[test]
    fn test_claim_income() {
        let env = Env::default();
        let contract = RentalContract;

        let owner = Address::generate(&env);

        let shares = vec![Share { owner: owner.clone(), percentage: 10000 }];

        contract.distribute_rental_income(env.clone(), 1, 1000, shares);

        let claimed = contract.claim_rental_income(env.clone(), owner.clone(), 1);
        assert_eq!(claimed, 1000);

        let after = contract.get_unclaimed_rental_income(env.clone(), owner.clone(), 1);
        assert_eq!(after, 0);
    }

    #[test]
    #[should_panic]
    fn test_zero_amount_rejected() {
        let env = Env::default();
        let contract = RentalContract;

        let owner = Address::generate(&env);

        let shares = vec![Share { owner: owner.clone(), percentage: 10000 }];

        contract.distribute_rental_income(env.clone(), 1, 0, shares);
    }
}