#[derive(Clone, Debug, PartialEq)]
pub struct ClaimEvidence {
    pub claim_id: u64,
    pub submitter: [u8; 32],
    pub ipfs_cid: [u8; 64],
    pub submitted_at: u64,
}

impl ClaimEvidence {
    pub fn new(claim_id: u64, submitter: [u8; 32], ipfs_cid: [u8; 64], submitted_at: u64) -> Self {
        Self { claim_id, submitter, ipfs_cid, submitted_at }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_evidence(claim_id: u64) -> ClaimEvidence {
        ClaimEvidence::new(claim_id, [1u8; 32], [0u8; 64], 1000)
    }

    #[test]
    fn evidence_fields_correct() {
        let ev = make_evidence(5);
        assert_eq!(ev.claim_id, 5);
        assert_eq!(ev.submitted_at, 1000);
    }

    #[test]
    fn evidence_equality() {
        assert_eq!(make_evidence(1), make_evidence(1));
        assert_ne!(make_evidence(1), make_evidence(2));
    }

    #[test]
    fn multiple_evidence_per_claim() {
        let items: alloc::vec::Vec<ClaimEvidence> = (0..3).map(|i| {
            ClaimEvidence::new(7, [i as u8; 32], [0u8; 64], i as u64 * 100)
        }).collect();
        assert!(items.iter().all(|e| e.claim_id == 7));
    }
}