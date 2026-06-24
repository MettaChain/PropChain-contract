/// # Regression: ReinsuranceStats Encode / Decode / TypeInfo derives
///
/// Bug reference: ReinsuranceStats was missing `scale::Encode`, `scale::Decode`,
/// and `scale_info::TypeInfo` derives, causing compile errors whenever the type
/// was used in a storage mapping or returned from an ink! message.
///
/// This module verifies those derives are present and functional.
#[cfg(test)]
mod reinsurance_stats_derives {
    use insurance::insurance::{Insurance, ReinsuranceTreatyType};
    use scale::{Decode, Encode};

    // We import the concrete type so the test fails to compile if the derives
    // are removed again.
    use insurance::insurance::ReinsuranceStats;

    /// Bug: ReinsuranceStats was missing Encode / Decode derives.
    /// Regression: encoding and then decoding must produce an identical value.
    #[test]
    fn reinsurance_stats_encode_decode_roundtrip() {
        let original = ReinsuranceStats {
            agreement_id: 42,
            treaty_type: ReinsuranceTreatyType::QuotaShare,
            total_ceded_premiums: 1_000_000,
            total_recoveries: 500_000,
            cession_count: 10,
            recovery_count: 5,
            net_recovery: -500_000,
        };

        // Encode
        let encoded = original.encode();
        assert!(!encoded.is_empty(), "encoded bytes must be non-empty");

        // Decode
        let decoded =
            ReinsuranceStats::decode(&mut encoded.as_slice()).expect("decode must succeed");

        assert_eq!(
            decoded.agreement_id, original.agreement_id,
            "agreement_id must survive encode/decode"
        );
        assert_eq!(
            decoded.treaty_type, original.treaty_type,
            "treaty_type must survive encode/decode"
        );
        assert_eq!(
            decoded.total_ceded_premiums, original.total_ceded_premiums,
            "total_ceded_premiums must survive encode/decode"
        );
        assert_eq!(
            decoded.total_recoveries, original.total_recoveries,
            "total_recoveries must survive encode/decode"
        );
        assert_eq!(
            decoded.cession_count, original.cession_count,
            "cession_count must survive encode/decode"
        );
        assert_eq!(
            decoded.recovery_count, original.recovery_count,
            "recovery_count must survive encode/decode"
        );
        assert_eq!(
            decoded.net_recovery, original.net_recovery,
            "net_recovery must survive encode/decode"
        );
    }

    /// Bug: TypeInfo derive was missing, preventing use in ink! messages.
    /// Regression: the TypeInfo implementation must be accessible at compile
    /// time — this is validated implicitly by the trait bound check below.
    #[test]
    fn reinsurance_stats_type_info_implemented() {
        // If TypeInfo is not derived, scale_info::TypeInfo::<any> won't be
        // satisfied and this function body won't compile.
        fn assert_type_info<T: scale_info::TypeInfo + 'static>() {}
        assert_type_info::<ReinsuranceStats>();
    }

    /// Verify zero-value default fields encode correctly (boundary condition).
    #[test]
    fn reinsurance_stats_zero_values_encode() {
        let zero = ReinsuranceStats {
            agreement_id: 0,
            treaty_type: ReinsuranceTreatyType::ExcessOfLoss,
            total_ceded_premiums: 0,
            total_recoveries: 0,
            cession_count: 0,
            recovery_count: 0,
            net_recovery: 0,
        };
        let encoded = zero.encode();
        let decoded = ReinsuranceStats::decode(&mut encoded.as_slice()).expect("zero-value decode");
        assert_eq!(decoded.agreement_id, 0);
        assert_eq!(decoded.net_recovery, 0);
    }
}
