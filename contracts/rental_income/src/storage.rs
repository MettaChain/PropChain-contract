use soroban_sdk::{Env, Address, Map};

const UNCLAIMED: &str = "unclaimed";

pub fn set_unclaimed(env: &Env, owner: &Address, property_id: u64, amount: i128) {
    let mut map: Map<(Address, u64), i128> = env.storage().instance().get(&UNCLAIMED).unwrap_or(Map::new(env));
    map.set((owner.clone(), property_id), amount);
    env.storage().instance().set(&UNCLAIMED, &map);
}

pub fn get_unclaimed(env: &Env, owner: &Address, property_id: u64) -> i128 {
    let map: Map<(Address, u64), i128> = env.storage().instance().get(&UNCLAIMED).unwrap_or(Map::new(env));
    map.get((owner.clone(), property_id)).unwrap_or(0)
}

pub fn clear_unclaimed(env: &Env, owner: &Address, property_id: u64) {
    let mut map: Map<(Address, u64), i128> = env.storage().instance().get(&UNCLAIMED).unwrap_or(Map::new(env));
    map.set((owner.clone(), property_id), 0);
    env.storage().instance().set(&UNCLAIMED, &map);
}