//! Closes #806: compact encoding helper for `LendingPool` (see the 12+ field
//! struct at `lending/src/lib.rs:64`). Starter pack/unpack for the rate +
//! flag fields; the full struct migration and Kani proof are follow-ups.

/// Packs a `base_rate` (0-999_999 bps) and up to 8 boolean flags into one
/// u32, instead of storing rate (u32) and flags (multiple bools) separately.
pub fn pack_rate_and_flags(base_rate_bps: u32, flags: u8) -> u32 {
    debug_assert!(base_rate_bps < (1 << 24));
    (base_rate_bps << 8) | flags as u32
}

pub fn unpack_rate(packed: u32) -> u32 {
    packed >> 8
}

pub fn unpack_flags(packed: u32) -> u8 {
    (packed & 0xFF) as u8
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_rate_and_flags() {
        let packed = pack_rate_and_flags(1250, 0b0000_0011);
        assert_eq!(unpack_rate(packed), 1250);
        assert_eq!(unpack_flags(packed), 0b0000_0011);
    }

    #[test]
    fn zero_flags_round_trip() {
        let packed = pack_rate_and_flags(500, 0);
        assert_eq!(unpack_rate(packed), 500);
        assert_eq!(unpack_flags(packed), 0);
    }
}
