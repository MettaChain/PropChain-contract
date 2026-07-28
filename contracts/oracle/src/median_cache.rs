//! Closes #812: median price cache helper for the oracle's aggregation path
//! (see the `// ── Median Price Cache (Issue #XXX) ──` TODO at
//! `contracts/oracle/src/lib.rs:218`). Starter pure-function version;
//! wiring into `Mapping<(SourceId, BlockNumber), u128>` storage plus a Kani
//! harness are follow-ups.

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
