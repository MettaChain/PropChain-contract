pub fn round_fee_up(amount: u128, fee_bps: u128, denominator: u128) -> u128 {
    if denominator == 0 { return 0; }
    let n = amount.saturating_mul(fee_bps);
    n.saturating_add(denominator.saturating_sub(1)).saturating_div(denominator)
}

pub fn round_fee_down(amount: u128, fee_bps: u128, denominator: u128) -> u128 {
    if denominator == 0 { return 0; }
    amount.saturating_mul(fee_bps).saturating_div(denominator)
}

pub fn fee_dust(amount: u128, fee_bps: u128, denominator: u128) -> u128 {
    round_fee_up(amount, fee_bps, denominator)
        .saturating_sub(round_fee_down(amount, fee_bps, denominator))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ceil_eliminates_dust_on_tick_fees() {
        // 0.3% of 1001 = 3.003; ceil=4, floor=3, dust=1
        assert_eq!(round_fee_up(1001, 30, 10_000), 4);
        assert_eq!(round_fee_down(1001, 30, 10_000), 3);
        assert_eq!(fee_dust(1001, 30, 10_000), 1);
    }

    #[test]
    fn exact_amounts_produce_no_dust() {
        assert_eq!(round_fee_up(10_000, 30, 10_000), round_fee_down(10_000, 30, 10_000));
        assert_eq!(fee_dust(10_000, 30, 10_000), 0);
    }

    #[test]
    fn zero_denominator_is_safe() {
        assert_eq!(round_fee_up(100, 30, 0), 0);
        assert_eq!(round_fee_down(100, 30, 0), 0);
    }

    #[test]
    fn zero_amount_yields_no_fee() {
        assert_eq!(round_fee_up(0, 30, 10_000), 0);
    }
}