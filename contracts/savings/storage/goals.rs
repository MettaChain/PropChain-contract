use soroban_sdk::{Env, Map};
use crate::types::SavingsGoal;

pub fn goals_key(user: &Address) -> Symbol {
    Symbol::new("user_goals")
}