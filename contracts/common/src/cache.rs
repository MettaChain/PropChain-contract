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
        self.entries.insert(key, CacheEntry { value, dirty: true });
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

    // ── Issue #1011: extended coverage ──────────────────────────────────

    #[test]
    fn insert_retrieve_roundtrip_via_get_and_set() {
        let mut cache: TransactionCache<u64, u128> = TransactionCache::new();

        // Miss before insert.
        assert!(cache.get(&1).is_none());

        // Insert then retrieve.
        cache.set(1, 1_000_000u128);
        assert_eq!(cache.get(&1), Some(&1_000_000u128));

        // Overwrite replaces the cached value.
        cache.set(1, 2_000_000u128);
        assert_eq!(cache.get(&1), Some(&2_000_000u128));

        // Distinct keys stay independent.
        cache.set(2, 42u128);
        assert_eq!(cache.get(&1), Some(&2_000_000u128));
        assert_eq!(cache.get(&2), Some(&42u128));
    }

    #[test]
    fn get_or_insert_with_recomputes_fresh_value_after_invalidate() {
        let mut cache: TransactionCache<u32, u32> = TransactionCache::new();

        let v = *cache.get_or_insert_with(7, || 100);
        assert_eq!(v, 100);

        cache.invalidate(&7);
        // The closure runs again and the new value wins.
        let v = *cache.get_or_insert_with(7, || 200);
        assert_eq!(v, 200);
        assert_eq!(cache.len(), 1);
    }

    #[test]
    fn computed_entries_are_clean_until_set() {
        let mut cache: TransactionCache<u32, u32> = TransactionCache::new();
        cache.get_or_insert_with(1, || 11);
        cache.get_or_insert_with(2, || 22);
        // Reads never dirty an entry — only `set` does.
        assert!(cache.dirty_keys().is_empty());

        cache.set(1, 111);
        assert_eq!(cache.dirty_keys(), vec![1]);
    }

    #[test]
    fn invalidate_removes_only_the_target_key() {
        let mut cache: TransactionCache<u32, u32> = TransactionCache::new();
        cache.set(1, 10);
        cache.set(2, 20);
        cache.set(3, 30);

        cache.invalidate(&2);
        assert!(cache.get(&2).is_none());
        // Siblings survive.
        assert_eq!(cache.get(&1), Some(&10));
        assert_eq!(cache.get(&3), Some(&30));
        assert_eq!(cache.len(), 2);
        assert_eq!(cache.dirty_keys(), vec![1, 3]);
    }

    #[test]
    fn invalidate_all_evicts_every_entry() {
        let mut cache: TransactionCache<u32, u32> = TransactionCache::new();
        cache.set(1, 10);
        cache.get_or_insert_with(2, || 20);
        cache.set(3, 30);
        assert_eq!(cache.len(), 3);

        cache.invalidate_all();
        assert!(cache.is_empty());
        assert_eq!(cache.len(), 0);
        assert!(cache.dirty_keys().is_empty());
        assert!(cache.get(&1).is_none());
        assert!(cache.get(&3).is_none());
    }

    #[test]
    fn dirty_keys_are_ordered_across_multiple_keys() {
        let mut cache: TransactionCache<u32, u32> = TransactionCache::new();

        // Insert out of order; BTreeMap backing yields ascending key order.
        cache.set(30, 300);
        cache.set(10, 100);
        cache.set(20, 200);
        assert_eq!(cache.dirty_keys(), vec![10, 20, 30]);

        // A later overwrite keeps the key in its sorted position once.
        cache.set(20, 250);
        assert_eq!(cache.dirty_keys(), vec![10, 20, 30]);
    }

    #[test]
    fn len_and_is_empty_track_entries() {
        let mut cache: TransactionCache<&'static str, u8> = TransactionCache::new();
        assert!(cache.is_empty());
        assert_eq!(cache.len(), 0);

        cache.set("a", 1);
        cache.set("b", 2);
        assert!(!cache.is_empty());
        assert_eq!(cache.len(), 2);

        cache.invalidate(&"a");
        assert_eq!(cache.len(), 1);

        cache.invalidate_all();
        assert!(cache.is_empty());
    }

    #[test]
    fn scale_roundtrip_is_byte_identical_for_cached_value_types() {
        use scale::{Decode, Encode};

        // Representative cached value types: token amounts and id hashes.
        let amount: u128 = 340_282_366_920_938_463_463u128;
        let property_id: u64 = 123_456_789;
        let code_hash: [u8; 32] = [0xA5; 32];

        for value in [amount, property_id as u128, u128::MAX] {
            let encoded = value.encode();
            let decoded = u128::decode(&mut &encoded[..]).unwrap();
            assert_eq!(decoded, value);
            // Re-encoding is byte-identical (format stability pin).
            assert_eq!(decoded.encode(), encoded);
        }

        let hash_encoded = code_hash.encode();
        let hash_decoded = <[u8; 32]>::decode(&mut &hash_encoded[..]).unwrap();
        assert_eq!(hash_decoded, code_hash);
        assert_eq!(hash_decoded.encode(), hash_encoded);

        // Composite of both shapes round-trips too.
        let entry = (property_id, amount, code_hash);
        let entry_encoded = entry.encode();
        let entry_decoded = <(u64, u128, [u8; 32])>::decode(&mut &entry_encoded[..]).unwrap();
        assert_eq!(entry_decoded, entry);
        assert_eq!(entry_decoded.encode(), entry_encoded);
    }
}
