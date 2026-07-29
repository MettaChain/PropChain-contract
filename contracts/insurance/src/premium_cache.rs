//! Closes #813: extends the lazy reinsurance cache pattern (see
//! `lazy_reinsurance.rs`) to the premium computation pipeline, which
//! currently re-resolves the coster on every call. Starter cache keyed by
//! pool version; wiring into the premium calculation path is a follow-up.

#[derive(Clone, Default)]
pub struct ResolvedCosterCache {
    pool_version: u64,
    coster_rate_bps: u32,
    loaded: bool,
}

impl ResolvedCosterCache {
    pub fn new() -> Self {
        Self::default()
    }
    /// Returns the cached coster rate if `pool_version` still matches the
    /// version it was resolved against; invalidates automatically otherwise.
    pub fn get(&self, pool_version: u64) -> Option<u32> {
        if self.loaded && self.pool_version == pool_version {
            Some(self.coster_rate_bps)
        } else {
            None
        }
    }

    pub fn set(&mut self, pool_version: u64, coster_rate_bps: u32) {
        self.pool_version = pool_version;
        self.coster_rate_bps = coster_rate_bps;
        self.loaded = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hits_cache_for_matching_pool_version() {
        let mut cache = ResolvedCosterCache::new();
        cache.set(1, 250);
        assert_eq!(cache.get(1), Some(250));
    }

    #[test]
    fn invalidates_on_pool_version_change() {
        let mut cache = ResolvedCosterCache::new();
        cache.set(1, 250);
        assert_eq!(cache.get(2), None);
    }
}
