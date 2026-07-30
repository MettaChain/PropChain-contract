//! Closes #798: canonical selector mapping for Multicall's typed
//! return-decoding (see `MulticallContract::aggregate` returning raw
//! `Vec<u8>`). Starter selector-tagged result type; wiring an
//! `aggregate_typed(...)` message that populates this is a follow-up.

/// A single call's result, tagged with the 4-byte selector it corresponds
/// to so off-chain SDKs can decode without guessing call order.
pub struct TypedResult {
    pub selector: [u8; 4],
    pub success: bool,
    pub return_data: Vec<u8>,
}

/// Pairs raw per-call results with their originating selectors.
pub fn tag_results(selectors: &[[u8; 4]], raw_results: Vec<(bool, Vec<u8>)>) -> Vec<TypedResult> {
    selectors
        .iter()
        .zip(raw_results.into_iter())
        .map(|(selector, (success, return_data))| TypedResult {
            selector: *selector,
            success,
            return_data,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pairs_each_selector_with_its_result() {
        let selectors = [[1, 2, 3, 4], [5, 6, 7, 8]];
        let raw = vec![(true, vec![0xAA]), (false, vec![])];
        let tagged = tag_results(&selectors, raw);
        assert_eq!(tagged.len(), 2);
        assert_eq!(tagged[0].selector, [1, 2, 3, 4]);
        assert!(tagged[0].success);
        assert!(!tagged[1].success);
    }
}
