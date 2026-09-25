//! #1107 / #799: governance-configurable rate-limit parameters, replacing
//! the hardcoded `rate_limit_enabled` / `max_requests_per_day = 10` /
//! `max_value_per_day = 1e18` counter previously embedded in `BridgeConfig`.
//!
//! `RateLimitConfig` is stored on the contract itself
//! ([`crate::PropertyBridge::rate_limit`]) and is the single source of truth
//! every rate-limit decision routes through (see
//! [`crate::PropertyBridge::check_and_update_rate_limits`]). Admins update it
//! via the `set_rate_limit_config` message; per-route volume caps remain in
//! `ChainBridgeInfo::chain_daily_limit` and are floored by this config's
//! `max_value_per_day`.

/// Governance-configurable rate-limit parameters.
#[derive(Debug, Clone, Copy, PartialEq, Eq, scale::Encode, scale::Decode)]
#[cfg_attr(
    feature = "std",
    derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
)]
pub struct RateLimitConfig {
    /// Length of a single rate-limit window, in seconds.
    pub window_seconds: u64,
    /// Maximum bridge requests a single account may submit per window.
    pub max_requests_per_day: u64,
    /// Maximum value a single route may carry per window.
    pub max_value_per_day: u128,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            window_seconds: 86_400,
            max_requests_per_day: 10,
            max_value_per_day: 1_000_000_000_000_000_000,
        }
    }
}

impl RateLimitConfig {
    /// Validates that a proposed update keeps every limit positive.
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.window_seconds == 0 || self.max_requests_per_day == 0 || self.max_value_per_day == 0
        {
            return Err("rate limit values must be positive");
        }
        Ok(())
    }

    /// Returns the rate-limit window id for a millisecond timestamp.
    ///
    /// Window ids increase monotonically; the stored per-account/per-chain
    /// counters reset when the observed id advances past the stored one.
    pub fn window_id(&self, timestamp_ms: u64) -> u64 {
        timestamp_ms / (self.window_seconds * 1_000)
    }

    /// Whether an account that already made `current` requests in this
    /// window may submit one more.
    pub fn requests_allowed(&self, current: u64) -> bool {
        current < self.max_requests_per_day
    }

    /// Whether a route that already carried `current` value in this window
    /// may carry `incoming` more, given the route's own cap `route_cap`
    /// (the tighter of the global value cap and the per-route limit).
    pub fn volume_allowed(&self, current: u128, incoming: u128, route_cap: u128) -> bool {
        current.saturating_add(incoming) <= route_cap
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_match_current_hardcoded_values() {
        let cfg = RateLimitConfig::default();
        assert_eq!(cfg.window_seconds, 86_400);
        assert_eq!(cfg.max_requests_per_day, 10);
        assert_eq!(cfg.max_value_per_day, 1_000_000_000_000_000_000);
        assert!(cfg.validate().is_ok());
    }

    #[test]
    fn rejects_zeroed_limits() {
        let cfg = RateLimitConfig {
            max_requests_per_day: 0,
            ..RateLimitConfig::default()
        };
        assert!(cfg.validate().is_err());

        let cfg = RateLimitConfig {
            window_seconds: 0,
            ..RateLimitConfig::default()
        };
        assert!(cfg.validate().is_err());

        let cfg = RateLimitConfig {
            max_value_per_day: 0,
            ..RateLimitConfig::default()
        };
        assert!(cfg.validate().is_err());
    }

    #[test]
    fn window_id_advances_with_timestamp() {
        let cfg = RateLimitConfig::default();
        assert_eq!(cfg.window_id(0), 0);
        assert_eq!(cfg.window_id(86_399_999), 0);
        assert_eq!(cfg.window_id(86_400_000), 1);
        assert_eq!(cfg.window_id(172_800_000), 2);
    }

    #[test]
    fn requests_allowed_enforces_per_window_count() {
        let cfg = RateLimitConfig {
            max_requests_per_day: 2,
            ..RateLimitConfig::default()
        };
        assert!(cfg.requests_allowed(0));
        assert!(cfg.requests_allowed(1));
        assert!(!cfg.requests_allowed(2));
        assert!(!cfg.requests_allowed(3));
    }

    #[test]
    fn volume_allowed_enforces_cap() {
        let cfg = RateLimitConfig::default();
        assert!(cfg.volume_allowed(0, 1_000, 1_000));
        assert!(!cfg.volume_allowed(1, 1_000, 1_000));
        assert!(cfg.volume_allowed(500, 500, 1_000));
        // Saturating add never wraps past u128::MAX.
        assert!(cfg.volume_allowed(u128::MAX, 1, u128::MAX));
    }
}
