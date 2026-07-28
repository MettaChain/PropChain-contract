#[derive(Clone, Default)]
pub struct ReinsurancePoolCache {
    loaded: bool,
    premium_ceded: u128,
    loss_recovered: u128,
}

impl ReinsurancePoolCache {
    pub fn new() -> Self { Self::default() }

    pub fn load(&mut self, premium_ceded: u128, loss_recovered: u128) {
        if !self.loaded {
            self.premium_ceded = premium_ceded;
            self.loss_recovered = loss_recovered;
            self.loaded = true;
        }
    }

    pub fn is_loaded(&self) -> bool { self.loaded }
    pub fn premium_ceded(&self) -> u128 { self.premium_ceded }
    pub fn loss_recovered(&self) -> u128 { self.loss_recovered }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn not_loaded_by_default() {
        assert!(!ReinsurancePoolCache::new().is_loaded());
    }

    #[test]
    fn loads_only_once() {
        let mut c = ReinsurancePoolCache::new();
        c.load(1000, 500);
        c.load(9999, 9999);
        assert_eq!(c.premium_ceded(), 1000);
        assert_eq!(c.loss_recovered(), 500);
    }

    #[test]
    fn values_accessible_after_load() {
        let mut c = ReinsurancePoolCache::new();
        c.load(2000, 750);
        assert!(c.is_loaded());
        assert_eq!(c.loss_recovered(), 750);
    }
}