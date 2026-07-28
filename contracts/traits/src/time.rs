//! Closes #801: unified time primitive so Oracle, Bridge, and Lending don't
//! each pick their own of block height vs. Unix seconds. Starter type +
//! conversions; adopting it across those three contracts is a follow-up.

/// A timeline point carrying both representations, so callers can compare
/// against whichever primitive their gate uses without a lossy conversion.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Timepoint {
    pub block_height: u32,
    pub timestamp: u64,
}

impl Timepoint {
    pub fn new(block_height: u32, timestamp: u64) -> Self {
        Self { block_height, timestamp }
    }

    /// Estimates a future timepoint `blocks` ahead, given an average block
    /// time in seconds.
    pub fn advance_blocks(&self, blocks: u32, avg_block_time_secs: u64) -> Self {
        Self {
            block_height: self.block_height + blocks,
            timestamp: self.timestamp + blocks as u64 * avg_block_time_secs,
        }
    }

    pub fn is_after(&self, other: &Timepoint) -> bool {
        self.timestamp > other.timestamp
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn advances_both_block_height_and_timestamp() {
        let t = Timepoint::new(100, 1_000);
        let future = t.advance_blocks(10, 6);
        assert_eq!(future.block_height, 110);
        assert_eq!(future.timestamp, 1_060);
    }

    #[test]
    fn compares_by_timestamp() {
        let earlier = Timepoint::new(1, 100);
        let later = Timepoint::new(2, 200);
        assert!(later.is_after(&earlier));
        assert!(!earlier.is_after(&later));
    }
}
