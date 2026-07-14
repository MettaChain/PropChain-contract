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

/// Trimmed mean: drop `trim_percent%` from each end, average the rest.
pub fn trimmed_mean(values: &mut Vec<u128>, trim_percent: u32) -> u128 {
    values.sort_unstable();
    let n = values.len();
    if n == 0 {
        return 0;
    }
    let trim_count = ((n as u32) * trim_percent / 100) as usize;
    if trim_count * 2 >= n {
        return values[n / 2];
    }
    let trimmed_values = &values[trim_count..n - trim_count];
    if trimmed_values.is_empty() {
        return 0;
    }
    let sum: u128 = trimmed_values.iter().sum();
    sum / (trimmed_values.len() as u128)
}
