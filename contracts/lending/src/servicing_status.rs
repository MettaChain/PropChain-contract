//! Closes #808: replace the `String` `servicing_status` field (see
//! `LoanApplication.servicing_status` at `lending/src/lib.rs:151`) with a
//! single-byte discriminant. Starter enum + `String` migration helper;
//! swapping the struct field itself is a follow-up.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ServicingStatus {
    Pending = 0,
    Boarded = 1,
    InDefault = 2,
    PaidOff = 3,
}

impl ServicingStatus {
    /// Migrates the legacy free-text status strings to the new enum,
    /// defaulting to `Pending` for anything unrecognized.
    pub fn from_legacy_str(value: &str) -> Self {
        match value {
            "Boarded" => ServicingStatus::Boarded,
            "InDefault" => ServicingStatus::InDefault,
            "PaidOff" => ServicingStatus::PaidOff,
            _ => ServicingStatus::Pending,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migrates_known_legacy_strings() {
        assert_eq!(ServicingStatus::from_legacy_str("Boarded"), ServicingStatus::Boarded);
        assert_eq!(ServicingStatus::from_legacy_str("InDefault"), ServicingStatus::InDefault);
    }

    #[test]
    fn defaults_unknown_strings_to_pending() {
        assert_eq!(ServicingStatus::from_legacy_str("Whatever"), ServicingStatus::Pending);
    }
}
