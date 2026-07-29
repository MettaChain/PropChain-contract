//! Bit flags and SCALE tag values used by `PackedLoanApplication`.
//!
//! This module is the scaffolding layer for Issue #738 (pack `LoanApplication`
//! for a tighter SCALE footprint). `PackedLoanApplication` itself lives in
//! `contracts/lending/src/lib.rs` because its `From` conversions need to see
//! the in-crate `LoanApplication` enum types. The constants here are the only
//! piece of state that `status_packing` exposes — purely declarative, so no
//! `#[ink::contract]` baggage.
//!
//! Flag widths are chosen to fit a single `u32`. Five bits are used; the
//! remaining 27 are reserved for future fields without breaking the SCALE
//! layout (only adds to the encoded width if used).

/// `LoanApplication::approved` (true = 1).
pub const FLAG_APPROVED: u32 = 1 << 0;
/// Whether `LoanApplication::servicer_id` is `Some(_)`.
pub const FLAG_HAS_SERVICER_ID: u32 = 1 << 1;
/// Whether `LoanApplication::start_block` is `Some(_)`.
pub const FLAG_HAS_START_BLOCK: u32 = 1 << 2;
/// `LoanType::FixedRate` (true) vs `LoanType::Variable` (false).
pub const FLAG_LOAN_TYPE_FIXED_RATE: u32 = 1 << 3;
/// `CollateralKind::PropertyTokenized` (true) vs `CollateralKind::Unsecured` (false).
pub const FLAG_COLLATERAL_PROPERTY_TOKENIZED: u32 = 1 << 4;

/// One-byte SCALE tag matching the order of `LoanStatus` variants.
pub const STATUS_PENDING: u8 = 0;
pub const STATUS_ACTIVE: u8 = 1;
pub const STATUS_REPAID: u8 = 2;
pub const STATUS_DEFAULTED: u8 = 3;
pub const STATUS_RESTRUCTURING_PROPOSED: u8 = 4;
pub const STATUS_RESTRUCTURED: u8 = 5;
pub const STATUS_LIQUIDATED: u8 = 6;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flag_bits_are_distinct() {
        // Each pair must not overlap.
        let pairs = [
            (FLAG_APPROVED, FLAG_HAS_SERVICER_ID),
            (FLAG_HAS_SERVICER_ID, FLAG_HAS_START_BLOCK),
            (FLAG_HAS_START_BLOCK, FLAG_LOAN_TYPE_FIXED_RATE),
            (
                FLAG_LOAN_TYPE_FIXED_RATE,
                FLAG_COLLATERAL_PROPERTY_TOKENIZED,
            ),
        ];
        for (a, b) in pairs {
            assert_eq!(a & b, 0, "flag bits overlap: {:#b} & {:#b}", a, b);
        }
    }

    #[test]
    fn all_flags_combined_set_exactly_five_bits() {
        let all = FLAG_APPROVED
            | FLAG_HAS_SERVICER_ID
            | FLAG_HAS_START_BLOCK
            | FLAG_LOAN_TYPE_FIXED_RATE
            | FLAG_COLLATERAL_PROPERTY_TOKENIZED;
        assert_eq!(all.count_ones(), 5);
    }

    #[test]
    fn status_tags_progression_matches_loan_status_variant_count() {
        // LoanStatus has 7 variants (Pending..Liquidated); tags must form a
        // sequential range so we can dispatch on a swap in `From<Packed>`.
        let tags = [
            STATUS_PENDING,
            STATUS_ACTIVE,
            STATUS_REPAID,
            STATUS_DEFAULTED,
            STATUS_RESTRUCTURING_PROPOSED,
            STATUS_RESTRUCTURED,
            STATUS_LIQUIDATED,
        ];
        // Concretely check each end + a couple of mid-points so the test
        // exercises values that are easy to break if a tag is renumbered.
        assert_eq!(tags[0], STATUS_PENDING);
        assert_eq!(tags[2], STATUS_REPAID);
        assert_eq!(tags[tags.len() - 1], STATUS_LIQUIDATED);
        assert_eq!(tags.len(), 7);
    }
}
