#![cfg_attr(target_family = "wasm", no_std)]
use soroban_sdk::{contract, contractimpl, contracttype, symbol_short, Env, Symbol};

const STATS: Symbol = symbol_short!("STATS");

#[contracttype]
#[derive(Clone, Default, Debug)]
pub struct LoanStats {
    pub total_loaned: i128,
    pub active_loans: u32,
    pub defaults: u32,
}

#[contract]
pub struct LoanAnalyticsContract;

#[contractimpl]
impl LoanAnalyticsContract {
    pub fn record_loan(env: Env, amount: i128) {
        let mut stats: LoanStats = env.storage().instance().get(&STATS).unwrap_or_default();

        stats.total_loaned += amount;
        stats.active_loans += 1;

        env.storage().instance().set(&STATS, &stats);
    }

    pub fn record_default(env: Env) {
        let mut stats: LoanStats = env.storage().instance().get(&STATS).unwrap_or_default();

        if stats.active_loans > 0 {
            stats.active_loans -= 1;
        }
        stats.defaults += 1;

        env.storage().instance().set(&STATS, &stats);
    }

    pub fn get_stats(env: Env) -> LoanStats {
        env.storage().instance().get(&STATS).unwrap_or_default()
    }
}

mod test;
