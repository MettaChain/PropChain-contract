#[derive(Clone, Debug)]
pub struct ProposalParticipation { pub proposal_id: u64, pub participation_bps: u32 }

pub struct QuorumGuard {
    history: alloc::vec::Vec<ProposalParticipation>,
    warning_threshold_bps: u32,
}

impl QuorumGuard {
    pub fn new(warning_threshold_bps: u32) -> Self {
        Self { history: alloc::vec::Vec::new(), warning_threshold_bps }
    }

    pub fn record(&mut self, proposal_id: u64, participation_bps: u32) -> bool {
        let warned = self.history.last()
            .map(|prev| prev.participation_bps >= self.warning_threshold_bps && participation_bps < self.warning_threshold_bps)
            .unwrap_or(false);
        self.history.push(ProposalParticipation { proposal_id, participation_bps });
        warned
    }

    pub fn history_len(&self) -> usize { self.history.len() }
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
        g.record(1, 800); g.record(2, 300);
        assert_eq!(g.history_len(), 2);
    }
}