//! Median price cache helpers for the oracle's aggregation path.
//!
//! Wired into live code paths in `contracts/oracle/src/lib.rs`:
//! - `compute_median` is used by `update_valuation_from_sources` to store the
//!   median of the collected source prices in the `cached_median_prices`
//!   storage mapping under `(property_id, "default")`;
//! - `is_cache_fresh` is used by `get_property_valuation` to decide whether a
//!   cached entry is still within its TTL (configured via `set_cache_ttl`).

/// Computes the median of `prices`, sorting a local copy (input is not mutated).
pub fn compute_median(prices: &[u128]) -> Option<u128> {
    if prices.is_empty() {
        return None;
    }
    let mut sorted = prices.to_vec();
    sorted.sort_unstable();
    let mid = sorted.len() / 2;
    if sorted.len() % 2 == 0 {
        Some((sorted[mid - 1] + sorted[mid]) / 2)
    } else {
        Some(sorted[mid])
    }
}

/// Returns true if `current_block - cached_at < ttl`, i.e. the cached
/// median at `cached_at` is still valid at `current_block`.
pub fn is_cache_fresh(cached_at: u32, current_block: u32, ttl: u32) -> bool {
    current_block.saturating_sub(cached_at) < ttl
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn computes_median_of_odd_length() {
        assert_eq!(compute_median(&[3, 1, 2]), Some(2));
    }

    #[test]
    fn computes_median_of_even_length() {
        assert_eq!(compute_median(&[10, 20, 30, 40]), Some(25));
    }

    #[test]
    fn empty_input_has_no_median() {
        assert_eq!(compute_median(&[]), None);
    }
}
