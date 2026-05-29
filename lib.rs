#![no_std]
use soroban_sdk::{contract, contractimpl, contracttype, symbol_short, Env, Symbol};


/// Storage keys for the contract.

#[contracttype]
#[derive(Clone)]
pub enum DataKey {
    Analytics,
}


/// The core analytics data structure to track loan health.

#[contracttype]
#[derive(Clone, Default, Debug)]
pub struct AnalyticsData {
    pub total_principal_lent: i128,
    pub active_loans_count: u32,
    pub completed_loans_count: u32,
    pub defaulted_loans_count: u32,
}

#[contract]
pub struct LendingAnalyticsContract;

#[contractimpl]
impl LendingAnalyticsContract {
    /// Updates the dashboard stats when a new loan is issued.
    pub fn update_stats_on_new_loan(env: Env, amount: i128) {
        let mut stats: AnalyticsData = env
            .storage()
            .instance()
            .get(&DataKey::Analytics)
            .unwrap_or_default();

        stats.total_principal_lent += amount;
        stats.active_loans_count += 1;

        env.storage().instance().set(&DataKey::Analytics, &stats);

        // Emit a Soroban Event for indexers (e.g., Mercury)
        // Topics: ["loan", "new"] | Data: amount
        env.events()
            .publish((symbol_short!("loan"), symbol_short!("new")), amount);
    }

    /// Updates the dashboard stats when a loan is repaid or defaults.
    pub fn update_stats_on_repayment(env: Env, is_default: bool) {
        let mut stats: AnalyticsData = env
            .storage()
            .instance()
            .get(&DataKey::Analytics)
            .unwrap_or_default();

        if stats.active_loans_count > 0 {
            stats.active_loans_count -= 1;
        }

        if is_default {
            stats.defaulted_loans_count += 1;
        } else {
            stats.completed_loans_count += 1;
        }

        env.storage().instance().set(&DataKey::Analytics, &stats);

        // Emit a Soroban Event for indexers
        // Topics: ["loan", "repay"] | Data: is_default
        env.events().publish(
            (symbol_short!("loan"), symbol_short!("repay")),
            is_default,
        );
    }

    /// Public view function to fetch current dashboard statistics.
    pub fn get_dashboard_stats(env: Env) -> AnalyticsData {
        env.storage()
            .instance()
            .get(&DataKey::Analytics)
            .unwrap_or_default()
    }

    /// Helper logic returning the default rate in basis points.
    /// Uses fixed-point math: 10000 basis points = 100.00%.
    pub fn get_default_rate_bps(env: Env) -> u32 {
        let stats = Self::get_dashboard_stats(env);
        let total_resolved = stats.completed_loans_count + stats.defaulted_loans_count;

        if total_resolved == 0 {
            return 0;
        }

        // Cast to u64 to prevent overflow during multiplication before dividing
        (((stats.defaulted_loans_count as u64) * 10_000) / (total_resolved as u64)) as u32
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use soroban_sdk::Env;

    #[test]
    fn test_analytics_flow() {
        let env = Env::default();
        let contract_id = env.register_contract(None, LendingAnalyticsContract);
        let client = LendingAnalyticsContractClient::new(&env, &contract_id);

        // 1. Check initial state
        let initial_stats = client.get_dashboard_stats();
        assert_eq!(initial_stats.total_principal_lent, 0);
        assert_eq!(initial_stats.active_loans_count, 0);

        // 2. Add a new loan
        client.update_stats_on_new_loan(&1000);
        let stats_after_loan = client.get_dashboard_stats();
        assert_eq!(stats_after_loan.total_principal_lent, 1000);
        assert_eq!(stats_after_loan.active_loans_count, 1);

        // 3. Repay loan (not default)
        client.update_stats_on_repayment(&false);
        let stats_after_repay = client.get_dashboard_stats();
        assert_eq!(stats_after_repay.active_loans_count, 0);
        assert_eq!(stats_after_repay.completed_loans_count, 1);
        assert_eq!(stats_after_repay.defaulted_loans_count, 0);

        // 4. Default rate check should be 0 bps
        assert_eq!(client.get_default_rate_bps(), 0);

        // 5. Add another loan and default it
        client.update_stats_on_new_loan(&2000);
        client.update_stats_on_repayment(&true);

        // Default rate should now be 50.00% -> 5000 bps
        assert_eq!(client.get_default_rate_bps(), 5000);
    }
}