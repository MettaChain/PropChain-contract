#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PathKey { pub from_token: u64, pub to_token: u64 }

pub struct PathCache {
    entries: alloc::vec::Vec<(PathKey, alloc::vec::Vec<u64>)>,
    hits: u64,
    misses: u64,
}

impl PathCache {
    pub fn new() -> Self { Self { entries: alloc::vec::Vec::new(), hits: 0, misses: 0 } }

    pub fn get(&mut self, key: &PathKey) -> Option<alloc::vec::Vec<u64>> {
        match self.entries.iter().find(|(k, _)| k == key).map(|(_, p)| p.clone()) {
            Some(p) => { self.hits += 1; Some(p) }
            None => { self.misses += 1; None }
        }
    }

    pub fn insert(&mut self, key: PathKey, path: alloc::vec::Vec<u64>) {
        self.entries.retain(|(k, _)| k != &key);
        self.entries.push((key, path));
    }

    pub fn hits(&self) -> u64 { self.hits }
    pub fn misses(&self) -> u64 { self.misses }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn miss_then_hit() {
        let mut c = PathCache::new();
        let k = PathKey { from_token: 1, to_token: 5 };
        assert!(c.get(&k).is_none());
        c.insert(k.clone(), alloc::vec![1, 2, 3, 4, 5]);
        assert_eq!(c.get(&k), Some(alloc::vec![1, 2, 3, 4, 5]));
        assert_eq!(c.hits(), 1); assert_eq!(c.misses(), 1);
    }

    #[test]
    fn insert_overwrites_existing_key() {
        let mut c = PathCache::new();
        let k = PathKey { from_token: 1, to_token: 3 };
        c.insert(k.clone(), alloc::vec![1, 2, 3]);
        c.insert(k.clone(), alloc::vec![1, 3]);
        assert_eq!(c.get(&k), Some(alloc::vec![1, 3]));
    }
}