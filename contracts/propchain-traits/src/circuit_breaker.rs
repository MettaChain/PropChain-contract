use core::fmt;

/// Reason a contract has been paused, surfaced uniformly across all
/// contracts implementing `CircuitBreaker`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PauseReason {
    OracleDrift,
    RateLimitBreached,
    ManualIntervention,
    ZeroLiquidity,
}

impl fmt::Display for PauseReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PauseReason::OracleDrift => write!(f, "oracle_drift"),
            PauseReason::RateLimitBreached => write!(f, "rate_limit_breached"),
            PauseReason::ManualIntervention => write!(f, "manual_intervention"),
            PauseReason::ZeroLiquidity => write!(f, "zero_liquidity"),
        }
    }
}

/// Uniform circuit-breaker behavior all pausable contracts implement,
/// replacing bespoke PauseFlags / set_pause_state / is_paused patterns.
pub trait CircuitBreaker {
    fn is_paused(&self) -> bool;
    fn pause(&mut self, reason: PauseReason);
    fn resume(&mut self);
    fn pause_reason(&self) -> Option<PauseReason>;
}

/// Guarded division helper used by DEX view functions to avoid
/// division-by-zero on empty/fresh pools.
pub fn guarded_div(numerator: u128, denominator: u128) -> Result<u128, PauseReason> {
    if denominator == 0 {
        return Err(PauseReason::ZeroLiquidity);
    }
    Ok(numerator / denominator)
}