#[derive(Clone, Debug, PartialEq)]
pub struct Delegation { pub delegator: [u8; 32], pub delegate: [u8; 32] }

pub struct DelegationRegistry {
    pairs: alloc::vec::Vec<([u8; 32], [u8; 32])>,
}

impl DelegationRegistry {
    pub fn new() -> Self { Self { pairs: alloc::vec::Vec::new() } }

    pub fn delegate_to(&mut self, delegator: [u8; 32], delegate: [u8; 32]) {
        self.undelegate(delegator);
        self.pairs.push((delegator, delegate));
    }

    pub fn undelegate(&mut self, delegator: [u8; 32]) {
        self.pairs.retain(|(d, _)| *d != delegator);
    }

    pub fn get_delegate(&self, delegator: &[u8; 32]) -> Option<[u8; 32]> {
        self.pairs.iter().find(|(d, _)| d == delegator).map(|(_, t)| *t)
    }

    pub fn delegated_count(&self, delegate: &[u8; 32]) -> usize {
        self.pairs.iter().filter(|(_, t)| t == delegate).count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    const A: [u8; 32] = [1u8; 32];
    const B: [u8; 32] = [2u8; 32];
    const C: [u8; 32] = [3u8; 32];

    #[test]
    fn delegate_and_retrieve() {
        let mut r = DelegationRegistry::new();
        r.delegate_to(A, B);
        assert_eq!(r.get_delegate(&A), Some(B));
    }

    #[test]
    fn undelegate_clears() {
        let mut r = DelegationRegistry::new();
        r.delegate_to(A, B);
        r.undelegate(A);
        assert_eq!(r.get_delegate(&A), None);
    }

    #[test]
    fn multiple_delegations_counted() {
        let mut r = DelegationRegistry::new();
        r.delegate_to(A, C);
        r.delegate_to(B, C);
        assert_eq!(r.delegated_count(&C), 2);
    }
}