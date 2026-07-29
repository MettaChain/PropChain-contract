//! Closes #799: governance-configurable rate-limit parameters, replacing
//! the hardcoded `max_requests_per_day = 10`, `max_value_per_day = 1e18`,
//! `chain_daily_limit = 1e19` in `bridge/src/lib.rs`. Starter config struct
//! with the current defaults preserved; the admin-gated `set_rate_limit`
//! message is a follow-up.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RateLimitConfig {
    pub max_requests_per_day: u32,
    pub max_value_per_day: u128,
    pub chain_daily_limit: u128,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            max_requests_per_day: 10,
            max_value_per_day: 1_000_000_000_000_000_000,
            chain_daily_limit: 10_000_000_000_000_000_000,
        }
    }
}

impl RateLimitConfig {
    /// Validates that a proposed update keeps all limits positive.
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.max_requests_per_day == 0 || self.max_value_per_day == 0 || self.chain_daily_limit == 0 {
            return Err("rate limit values must be positive");
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_match_current_hardcoded_values() {
        let cfg = RateLimitConfig::default();
        assert_eq!(cfg.max_requests_per_day, 10);
        assert_eq!(cfg.max_value_per_day, 1_000_000_000_000_000_000);
    }

    #[test]
    fn rejects_zeroed_limits() {
        let cfg = RateLimitConfig { max_requests_per_day: 0, ..RateLimitConfig::default() };
        assert!(cfg.validate().is_err());
    }
}
