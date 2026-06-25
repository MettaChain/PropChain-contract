use soroban_sdk::{Env, Address};

pub fn emit_rental_income_distributed(
    env: &Env,
    property_id: u64,
    amount: i128,
) {
    env.events().publish(("RentalIncomeDistributed",), (property_id, amount));
}

pub fn emit_rental_income_claimed(
    env: &Env,
    owner: Address,
    property_id: u64,
    amount: i128,
) {
    env.events().publish(
        ("RentalIncomeClaimed",),
        (owner, property_id, amount),
    );
}