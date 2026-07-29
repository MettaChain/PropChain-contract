//! Intra-transaction caching abstraction for ink! contracts.
//!
//! `TransactionCache` avoids repeated full state reads for the same key
//! within a single message call. It is NOT persisted across transactions —
//! it lives only for the lifetime of the call and must be re-populated
//! (or dropped) on the next invocation.

use ink::prelude::collections::BTreeMap;
use ink::prelude::vec::Vec;

/// A local, transaction-scoped cache backed by a `BTreeMap`.
///
/// Intended usage: construct once at the top of a message handler,
/// read/write through it instead of hitting `Mapping` storage directly
/// for repeated lookups of the same key, then let it drop at the end
/// of the call.
#[derive(Debug, Default)]
pub struct TransactionCache<K, V>
where
    K: Ord,
{
    entries: BTreeMap<K, CacheEntry<V>>,
}

#[derive(Debug, Clone)]
struct CacheEntry<V> {
    value: V,
    dirty: bool,
}

impl<K, V> TransactionCache<K, V>
where
    K: Ord + Clone,
    V: Clone,
{
    /// Create a new, empty cache scoped to this transaction.
    pub fn new() -> Self {
        Self {
            entries: BTreeMap::new(),
        }
    }

    /// Get a cached value, if present.
    pub fn get(&self, key: &K) -> Option<&V> {
        self.entries.get(key).map(|e| &e.value)
    }

    /// Get a cached value, or compute and cache it via `on_miss` if absent.
    ///
    /// `on_miss` is typically a closure that performs the real storage read
    /// (e.g. `self.env().storage().get(key)` or a `Mapping::get`).
    pub fn get_or_insert_with<F>(&mut self, key: K, on_miss: F) -> &V
    where
        F: FnOnce() -> V,
    {
        self.entries
            .entry(key)
            .or_insert_with(|| CacheEntry {
                value: on_miss(),
                dirty: false,
            })
            .value_ref()
    }

    /// Insert or overwrite a value, marking it dirty (needs flush to storage).
    pub fn set(&mut self, key: K, value: V) {
        self.entries.insert(
            key,
            CacheEntry {
                value,
                dirty: true,
            },
        );
    }

    /// Invalidate a single key, forcing the next `get_or_insert_with` to miss.
    pub fn invalidate(&mut self, key: &K) {
        self.entries.remove(key);
    }

    /// Invalidate all entries. Call this on state-change events that
    /// affect an unknown or unbounded set of keys.
    pub fn invalidate_all(&mut self) {
        self.entries.clear();
    }

    /// Returns all keys currently marked dirty, for flushing back to
    /// persistent storage at the end of the transaction.
    pub fn dirty_keys(&self) -> Vec<K> {
        self.entries
            .iter()
            .filter(|(_, e)| e.dirty)
            .map(|(k, _)| k.clone())
            .collect()
    }

    /// Number of entries currently cached.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

// small helper so `.value_ref()` reads cleanly above
impl<V> CacheEntry<V> {
    fn value_ref(&self) -> &V {
        &self.value
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hit_returns_cached_value_without_recomputing() {
        let mut cache: TransactionCache<u32, u32> = TransactionCache::new();
        let mut calls = 0;

        {
            let v = cache.get_or_insert_with(1, || {
                calls += 1;
                42
            });
            assert_eq!(*v, 42);
        }
        {
            let v = cache.get_or_insert_with(1, || {
                calls += 1;
                99
            });
            assert_eq!(*v, 42); // still 42, closure not called again
        }
        assert_eq!(calls, 1);
    }

    #[test]
    fn invalidate_forces_recompute() {
        let mut cache: TransactionCache<u32, u32> = TransactionCache::new();
        cache.get_or_insert_with(1, || 42);
        cache.invalidate(&1);
        assert!(cache.get(&1).is_none());
    }

    #[test]
    fn invalidate_all_clears_everything() {
        let mut cache: TransactionCache<u32, u32> = TransactionCache::new();
        cache.get_or_insert_with(1, || 1);
        cache.get_or_insert_with(2, || 2);
        cache.invalidate_all();
        assert!(cache.is_empty());
    }

    #[test]
    fn set_marks_entry_dirty() {
        let mut cache: TransactionCache<u32, u32> = TransactionCache::new();
        cache.set(1, 100);
        assert_eq!(cache.dirty_keys(), vec![1]);
    }
}