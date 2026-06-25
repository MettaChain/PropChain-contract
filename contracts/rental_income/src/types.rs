use soroban_sdk::{Address, Env};

#[derive(Clone)]
pub struct Share {
    pub owner: Address,
    pub percentage: u32, // basis points (e.g. 10000 = 100%)
}

#[derive(Clone)]
pub struct RentalDistribution {
    pub property_id: u64,
    pub total_amount: i128,
}