
use ink::prelude::vec::Vec;

pub fn simple_median(values: &mut Vec<u128>) -> u128 {
    pub fn weighted_median(values: &[(u128, u32)]) -> u128 {
    let mut weighted_values = values.to_vec();
    weighted_values.sort_by_key(|(v, _)| *v);
    let total_weight: u32 = weighted_values.iter().map(|(_, w)| w).sum();
    let mut cumulative_weight = 0;
    for (value, weight) in &weighted_values {
        cumulative_weight += *weight;
        if cumulative_weight >= total_weight / 2 {
            return *value;
        }
    }
    weighted_values.last().map_or(0, |(v, _)| *v)
}

pub fn trimmed_mean(values: &mut Vec<u128>, trim_percent: u32) -> u128 {
    values.sort_unstable();
    let trim_count = (values.len() as u32 * trim_percent / 100) as usize;
    let trimmed_values = &values[trim_count..values.len() - trim_count];
    if trimmed_values.is_empty() {
        return 0;
    }
    let sum: u128 = trimmed_values.iter().sum();
    sum / trimmed_values.len() as u128
}
}