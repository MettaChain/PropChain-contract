//! Closes #805: pack `BridgeTransaction` status into a `u8` bit field
//! instead of a wider enum + adjacent fields. Starter pack/unpack; the
//! struct migration, microbench, and Kani proof are follow-ups.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum BridgeStatus {
    Pending = 0,
    Confirmed = 1,
    Failed = 2,
    Refunded = 3,
}

/// Packs the status (2 bits) and a `finalized` flag (1 bit) into one byte,
/// replacing separate `status` + `finalized: bool` fields.
pub fn pack_status(status: BridgeStatus, finalized: bool) -> u8 {
    (status as u8) | ((finalized as u8) << 2)
}

pub fn unpack_status(packed: u8) -> BridgeStatus {
    match packed & 0b11 {
        1 => BridgeStatus::Confirmed,
        2 => BridgeStatus::Failed,
        3 => BridgeStatus::Refunded,
        _ => BridgeStatus::Pending,
    }
}

pub fn unpack_finalized(packed: u8) -> bool {
    (packed >> 2) & 1 == 1
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_status_and_finalized_flag() {
        let packed = pack_status(BridgeStatus::Confirmed, true);
        assert_eq!(unpack_status(packed), BridgeStatus::Confirmed);
        assert!(unpack_finalized(packed));
    }

    #[test]
    fn round_trips_pending_unfinalized() {
        let packed = pack_status(BridgeStatus::Pending, false);
        assert_eq!(unpack_status(packed), BridgeStatus::Pending);
        assert!(!unpack_finalized(packed));
    }
}
