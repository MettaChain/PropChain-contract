use ink::prelude::vec::Vec;
use propchain_traits::constants;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProposalParticipation {
    pub proposal_id: u64,
    pub participation_bps: u32,
}

/// Tracks governance participation drops across consecutive proposals over a
/// **bounded rolling window**.
///
/// # Why the window is bounded
///
/// This guard used to append every proposal it ever saw to an unbounded
/// `Vec` and derive its answers from that vector. Two consequences followed,
/// and both got worse over the life of a deployment:
///
/// * storage grew without limit, one entry per proposal, forever;
/// * every participation read scanned the entire lifetime history, so the
///   check that exists to make governance participation cheap became
///   O(total proposals since deployment) — the opposite of its purpose.
///
/// The window is now capped at
/// [`MONITORING_MAX_QUORUM_HISTORY`](propchain_traits::constants::MONITORING_MAX_QUORUM_HISTORY).
/// Once full, recording a new proposal evicts the oldest. Reads are therefore
/// bounded by a compile-time constant no matter how many proposals have been
/// recorded.
///
/// # What eviction costs
///
/// `participation_bps` and `average_participation_bps` answer only for
/// proposals still inside the window; a query for an evicted proposal returns
/// `None` rather than a fabricated value. `total_recorded` is a separate
/// monotonically increasing counter that survives eviction, so the total
/// number of proposals observed is never lost — only the per-proposal detail
/// of old ones is.
pub struct QuorumGuard {
    /// Most recent records, oldest first, at most `capacity()` long.
    history: Vec<ProposalParticipation>,
    /// Lifetime count of records submitted, including evicted ones.
    total_recorded: u64,
    warning_threshold_bps: u32,
}

impl QuorumGuard {
    pub fn new(warning_threshold_bps: u32) -> Self {
        Self {
            history: Vec::new(),
            total_recorded: 0,
            warning_threshold_bps,
        }
    }

    /// Maximum number of records retained in the rolling window.
    pub fn capacity() -> usize {
        constants::MONITORING_MAX_QUORUM_HISTORY
    }

    /// Records participation for `proposal_id` and reports whether this
    /// proposal dropped below the warning threshold that the previous one met.
    ///
    /// Returns `false` for the first proposal in the window, since there is no
    /// prior observation to compare against.
    pub fn record(&mut self, proposal_id: u64, participation_bps: u32) -> bool {
        let warned = self
            .history
            .last()
            .map(|prev| {
                prev.participation_bps >= self.warning_threshold_bps
                    && participation_bps < self.warning_threshold_bps
            })
            .unwrap_or(false);
        self.history.push(ProposalParticipation {
            proposal_id,
            participation_bps,
        });
        self.total_recorded = self.total_recorded.saturating_add(1);
        self.evict_overflow();
        warned
    }

    /// Drops the oldest records until the window is within capacity.
    ///
    /// Eviction happens only once the window is full, so this is a no-op for
    /// every record up to the cap and shifts one element per record after it.
    /// The shift is bounded by `capacity()`, a compile-time constant, so the
    /// amortised cost per record stays constant.
    fn evict_overflow(&mut self) {
        while self.history.len() > Self::capacity() {
            self.history.remove(0);
        }
    }

    /// Number of records currently inside the rolling window.
    ///
    /// Bounded by [`QuorumGuard::capacity`]; this is the value that used to
    /// grow without limit.
    pub fn history_len(&self) -> usize {
        self.history.len()
    }

    /// Whether the window is saturated, i.e. the next `record` call evicts.
    pub fn is_saturated(&self) -> bool {
        self.history.len() >= Self::capacity()
    }

    /// Lifetime number of records submitted, including those already evicted.
    pub fn total_recorded(&self) -> u64 {
        self.total_recorded
    }

    /// Number of records dropped from the window to stay within capacity.
    pub fn evicted_count(&self) -> u64 {
        self.total_recorded
            .saturating_sub(self.history.len() as u64)
    }

    /// Participation recorded for `proposal_id`, or `None` if that proposal is
    /// outside the rolling window (never seen, or already evicted).
    ///
    /// Scans at most [`QuorumGuard::capacity`] entries.
    pub fn participation_bps(&self, proposal_id: u64) -> Option<u32> {
        self.history
            .iter()
            .find(|entry| entry.proposal_id == proposal_id)
            .map(|entry| entry.participation_bps)
    }

    /// Mean participation across the current window, in bips.
    ///
    /// `None` for an empty window, since the mean of nothing is undefined.
    /// Bounded by [`QuorumGuard::capacity`] entries.
    pub fn average_participation_bps(&self) -> Option<u32> {
        if self.history.is_empty() {
            return None;
        }
        let sum: u128 = self
            .history
            .iter()
            .map(|entry| entry.participation_bps as u128)
            .sum();
        Some((sum / self.history.len() as u128) as u32)
    }

    /// A page of window records, oldest first, starting at `offset`.
    ///
    /// Lets a caller walk the retained window without materialising all of it.
    /// `limit` is clamped to what remains, so a caller cannot read past the
    /// window by asking for a large limit.
    pub fn history_page(&self, offset: usize, limit: usize) -> Vec<ProposalParticipation> {
        self.history
            .iter()
            .skip(offset)
            .take(limit)
            .cloned()
            .collect()
    }

    /// Copies of the current window, oldest first.
    pub fn window(&self) -> Vec<ProposalParticipation> {
        self.history.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_warning_on_first_proposal() {
        let mut g = QuorumGuard::new(500);
        assert!(!g.record(1, 300));
    }

    #[test]
    fn warns_when_participation_drops_below_threshold() {
        let mut g = QuorumGuard::new(500);
        g.record(1, 800);
        assert!(g.record(2, 300));
    }

    #[test]
    fn no_warning_when_staying_above_threshold() {
        let mut g = QuorumGuard::new(500);
        g.record(1, 800);
        assert!(!g.record(2, 600));
    }

    #[test]
    fn history_accumulates() {
        let mut g = QuorumGuard::new(500);
        g.record(1, 800);
        g.record(2, 300);
        assert_eq!(g.history_len(), 2);
    }

    #[test]
    fn average_is_none_until_something_is_recorded() {
        let g = QuorumGuard::new(500);
        assert_eq!(g.average_participation_bps(), None);
    }

    #[test]
    fn average_participation_is_over_the_window() {
        let mut g = QuorumGuard::new(500);
        g.record(1, 800);
        g.record(2, 300);
        assert_eq!(g.average_participation_bps(), Some(550));
    }

    #[test]
    fn participation_lookup_finds_retained_proposal() {
        let mut g = QuorumGuard::new(500);
        g.record(7, 640);
        assert_eq!(g.participation_bps(7), Some(640));
    }

    #[test]
    fn participation_lookup_is_none_for_unknown_proposal() {
        let mut g = QuorumGuard::new(500);
        g.record(7, 640);
        assert_eq!(g.participation_bps(8), None);
    }

    // --- Bounded-window behaviour (issue #1196) ---

    #[test]
    fn history_is_capped_at_capacity() {
        let mut g = QuorumGuard::new(500);
        for id in 0..(QuorumGuard::capacity() as u64 * 4) {
            g.record(id, 600);
        }
        assert_eq!(g.history_len(), QuorumGuard::capacity());
        assert!(g.is_saturated());
    }

    #[test]
    fn load_at_and_beyond_the_cap_keeps_window_bounded() {
        // Records far more proposals than the window can hold, which is the
        // scenario the unbounded Vec could not survive.
        let mut g = QuorumGuard::new(500);
        let total = 50_000u64;
        for id in 0..total {
            g.record(id, 600);
        }

        assert_eq!(g.history_len(), QuorumGuard::capacity());
        assert_eq!(g.total_recorded(), total);
        assert_eq!(g.evicted_count(), total - QuorumGuard::capacity() as u64);
    }

    #[test]
    fn eviction_keeps_the_newest_and_drops_the_oldest() {
        let mut g = QuorumGuard::new(500);
        let total = QuorumGuard::capacity() as u64 + 50;
        for id in 0..total {
            g.record(id, 600);
        }

        // The 50 oldest are gone, and report absence rather than a guess.
        for id in 0..50 {
            assert_eq!(g.participation_bps(id), None, "proposal {id} evicted");
        }
        // The newest are retained.
        for id in 50..total {
            assert_eq!(
                g.participation_bps(id),
                Some(600),
                "proposal {id} should be retained"
            );
        }
    }

    #[test]
    fn window_holds_exactly_the_last_capacity_records_in_order() {
        let mut g = QuorumGuard::new(500);
        let total = QuorumGuard::capacity() as u64 + 10;
        for id in 0..total {
            g.record(id, 600);
        }

        let window = g.window();
        assert_eq!(window.len(), QuorumGuard::capacity());
        assert_eq!(window.first().unwrap().proposal_id, 10);
        assert_eq!(window.last().unwrap().proposal_id, total - 1);
    }

    #[test]
    fn participation_query_cost_is_bounded_after_eviction() {
        // A miss for a long-evicted proposal still only touches the window.
        let mut g = QuorumGuard::new(500);
        for id in 0..10_000 {
            g.record(id, 600);
        }
        assert_eq!(g.participation_bps(0), None);
        // The average spans the window, not the lifetime history.
        assert_eq!(g.average_participation_bps(), Some(600));
    }

    #[test]
    fn drop_warning_still_fires_after_eviction() {
        // The consecutive-proposal comparison must keep working once the
        // window has rolled over many times.
        let mut g = QuorumGuard::new(500);
        for id in 0..10_000u64 {
            g.record(id, 800);
        }
        assert!(g.record(10_000, 100), "drop below threshold should warn");
    }

    #[test]
    fn history_page_is_clamped_to_the_window() {
        let mut g = QuorumGuard::new(500);
        for id in 0..10u64 {
            g.record(id, 600);
        }
        assert_eq!(g.history_page(0, 4).len(), 4);
        assert_eq!(g.history_page(8, 100).len(), 2);
        assert!(g.history_page(50, 10).is_empty());
    }
}
