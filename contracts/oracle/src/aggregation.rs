#![allow(
    clippy::module_name_repetitions,
    clippy::needless_borrows_for_generic_args,
    clippy::ptr_arg
)]

use ink::prelude::vec::Vec;

/// Simple (unweighted) median of a sample.
pub fn simple_median(values: &mut Vec<u128>) -> u128 {
    values.sort_unstable();
    let n = values.len();
    if n == 0 {
        return 0;
    }
    if n % 2 == 1 {
        values[n / 2]
    } else {
        (values[n / 2 - 1]).saturating_add(values[n / 2]) / 2
    }
}

/// Weighted median: the value at the cumulative-weight midpoint.
pub fn weighted_median(values: &[(u128, u32)]) -> u128 {
    if values.is_empty() {
        return 0;
    }
    let mut weighted_values = values.to_vec();
    weighted_values.sort_by_key(|(v, _)| *v);
    let total_weight: u32 = weighted_values.iter().map(|(_, w)| w).sum();
    if total_weight == 0 {
        return weighted_values.first().map_or(0, |(v, _)| *v);
    }
    let mut cumulative_weight: u32 = 0;
    for (value, weight) in &weighted_values {
        cumulative_weight = cumulative_weight.saturating_add(*weight);
        if cumulative_weight >= total_weight / 2 {
            return *value;
        }
    }
    weighted_values.last().map_or(0, |(v, _)| *v)
}

/// Trimmed mean: drop up to `trim_count` values from each end of the sorted
/// sample (capped at one third of the sample, matching the oracle's
/// `AggregationMethod::TrimmedMean` arm), then average the rest.
pub fn trimmed_mean(values: &mut Vec<u128>, trim_count: usize) -> u128 {
    values.sort_unstable();
    let n = values.len();
    if n == 0 {
        return 0;
    }
    let trim = trim_count.min(n / 3);
    let trimmed_values = &values[trim..n - trim];
    if trimmed_values.is_empty() {
        return 0;
    }
    let sum: u128 = trimmed_values.iter().sum();
    sum / (trimmed_values.len() as u128)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn simple_median_of_odd_length_is_middle_value() {
        let mut values = vec![3u128, 1, 2];
        assert_eq!(simple_median(&mut values), 2);
    }

    #[test]
    fn simple_median_of_even_length_averages_middle_two() {
        let mut values = vec![10u128, 20, 30, 40];
        assert_eq!(simple_median(&mut values), 25);
    }

    #[test]
    fn simple_median_of_empty_sample_is_zero() {
        let mut values: Vec<u128> = Vec::new();
        assert_eq!(simple_median(&mut values), 0);
    }

    #[test]
    fn weighted_median_picks_value_at_weight_midpoint() {
        // (price, weight): cumulative weight reaches half (75) at 100.
        let values = [(100u128, 50u32), (98, 50), (105, 50)];
        assert_eq!(weighted_median(&values), 100);
    }

    #[test]
    fn weighted_median_of_empty_sample_is_zero() {
        assert_eq!(weighted_median(&[]), 0);
    }

    #[test]
    fn trimmed_mean_drops_extremes() {
        let mut values = vec![1u128, 2, 3, 4, 100];
        // trim_count 1 drops 1 and 100, average of [2,3,4] = 3.
        assert_eq!(trimmed_mean(&mut values, 1), 3);
    }

    #[test]
    fn trimmed_mean_caps_trim_at_one_third() {
        let mut values = vec![0u128, 0, 0, 10, 10, 10, 10, 10, 100];
        // Requested trim 10 is capped at n/3 = 3: drops 0,0,0 from the front
        // and 10,10,100 from the back. Average of the middle [10,10,10] = 10.
        assert_eq!(trimmed_mean(&mut values, 10), 10);
    }

    #[test]
    fn trimmed_mean_of_empty_sample_is_zero() {
        let mut values: Vec<u128> = Vec::new();
        assert_eq!(trimmed_mean(&mut values, 2), 0);
    }
}
