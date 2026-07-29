//! Closes #810: bounded bulk-delete iteration for `Mapping`-backed storage.
//! Starter helper; full macro + gas-bound test harness is a follow-up.

/// Removes up to `max_items` entries from `keys`, calling `remove_fn` for
/// each, and returns how many were removed. Bounding the count per call
/// prevents a single transaction from exhausting the block gas limit when
/// clearing large collections.
pub fn bulk_remove<K: Clone>(keys: &[K], max_items: u32, mut remove_fn: impl FnMut(&K)) -> u32 {
    let limit = max_items as usize;
    let mut removed = 0u32;
    for key in keys.iter().take(limit) {
        remove_fn(key);
        removed += 1;
    }
    removed
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn respects_the_max_items_bound() {
        let keys = vec![1, 2, 3, 4, 5];
        let mut visited = Vec::new();
        let removed = bulk_remove(&keys, 3, |k| visited.push(*k));
        assert_eq!(removed, 3);
        assert_eq!(visited, vec![1, 2, 3]);
    }

    #[test]
    fn handles_fewer_keys_than_the_bound() {
        let keys = vec![1, 2];
        let mut count = 0;
        let removed = bulk_remove(&keys, 10, |_| count += 1);
        assert_eq!(removed, 2);
        assert_eq!(count, 2);
    }
}
