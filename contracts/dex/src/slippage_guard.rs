pub fn check_slippage(expected_out: u128, actual_out: u128, max_bps: u32) -> Result<(), SlippageError> {
    if expected_out == 0 { return Err(SlippageError::ZeroExpected); }
    let loss = expected_out.saturating_mul(max_bps as u128).saturating_div(10_000);
    if actual_out < expected_out.saturating_sub(loss) {
        return Err(SlippageError::Exceeded);
    }
    Ok(())
}

#[derive(Debug, PartialEq)]
pub enum SlippageError { Exceeded, ZeroExpected }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn within_tolerance_is_ok() { assert!(check_slippage(1000, 995, 50).is_ok()); }

    #[test]
    fn exceeding_tolerance_errors() {
        assert_eq!(check_slippage(1000, 900, 50), Err(SlippageError::Exceeded));
    }

    #[test]
    fn exact_boundary_accepted() {
        // 1% of 1000 = 10; min acceptable = 990
        assert!(check_slippage(1000, 990, 100).is_ok());
        assert_eq!(check_slippage(1000, 989, 100), Err(SlippageError::Exceeded));
    }

    #[test]
    fn zero_expected_errors() {
        assert_eq!(check_slippage(0, 0, 100), Err(SlippageError::ZeroExpected));
    }

    #[test]
    fn zero_slippage_requires_exact_output() {
        assert!(check_slippage(1000, 1000, 0).is_ok());
        assert_eq!(check_slippage(1000, 999, 0), Err(SlippageError::Exceeded));
    }
}