#![cfg_attr(not(feature = "std"), no_std, no_main)]

#[ink::contract]
mod merkle_rewards_distributor {
    use ink::env::hash::{Blake2x256, HashOutput};
    use ink::prelude::vec::Vec;
    use ink::storage::Mapping;

    #[derive(Debug, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub enum Error {
        Unauthorized,
        InvalidProof,
        AlreadyClaimed,
        TransferFailed,
    }

    #[ink(storage)]
    pub struct MerkleRewardsDistributor {
        owner: AccountId,
        merkle_root: Hash,
        reward_token: AccountId,
        claimed: Mapping<AccountId, bool>,
        epoch: u64,
    }

    #[ink(event)]
    pub struct Claimed {
        #[ink(topic)]
        claimer: AccountId,
        amount: Balance,
        epoch: u64,
    }

    #[ink(event)]
    pub struct MerkleRootUpdated {
        #[ink(topic)]
        by: AccountId,
        old_root: Hash,
        new_root: Hash,
        epoch: u64,
    }

    impl MerkleRewardsDistributor {
        #[ink(constructor)]
        pub fn new(merkle_root: Hash, reward_token: AccountId) -> Self {
            Self {
                owner: Self::env().caller(),
                merkle_root,
                reward_token,
                claimed: Mapping::default(),
                epoch: 0,
            }
        }

        fn leaf_hash(account: &AccountId, amount: Balance) -> Hash {
            let mut encoded = Vec::new();
            encoded.extend_from_slice(b"MRD");
            encoded.extend_from_slice(&scale::Encode::encode(account));
            encoded.extend_from_slice(&scale::Encode::encode(&amount));
            let mut out = <Blake2x256 as HashOutput>::Type::default();
            ink::env::hash_bytes::<Blake2x256>(&encoded, &mut out);
            out.into()
        }

        fn hash_pair(a: Hash, b: Hash) -> Hash {
            let (left, right) = if a.as_ref() <= b.as_ref() { (a, b) } else { (b, a) };
            let mut data = Vec::with_capacity(64);
            data.extend_from_slice(left.as_ref());
            data.extend_from_slice(right.as_ref());
            let mut out = <Blake2x256 as HashOutput>::Type::default();
            ink::env::hash_bytes::<Blake2x256>(&data, &mut out);
            out.into()
        }

        fn verify(&self, account: AccountId, amount: Balance, proof: Vec<Hash>) -> bool {
            let mut computed = Self::leaf_hash(&account, amount);
            for p in proof {
                computed = Self::hash_pair(computed, p);
            }
            computed == self.merkle_root
        }

        #[ink(message)]
        pub fn claim(&mut self, amount: Balance, merkle_proof: Vec<Hash>) -> Result<(), Error> {
            let caller = self.env().caller();
            if self.claimed.get(caller).unwrap_or(false) {
                return Err(Error::AlreadyClaimed);
            }
            if !self.verify(caller, amount, merkle_proof) {
                return Err(Error::InvalidProof);
            }
            self.claimed.insert(caller, &true);
            self.env()
                .transfer(caller, amount)
                .map_err(|_| Error::TransferFailed)?;
            self.env().emit_event(Claimed {
                claimer: caller,
                amount,
                epoch: self.epoch,
            });
            Ok(())
        }

        #[ink(message)]
        pub fn set_merkle_root(&mut self, new_root: Hash) -> Result<(), Error> {
            let caller = self.env().caller();
            if caller != self.owner {
                return Err(Error::Unauthorized);
            }
            let old_root = self.merkle_root;
            self.merkle_root = new_root;
            self.epoch = self.epoch.saturating_add(1);
            self.env().emit_event(MerkleRootUpdated {
                by: caller,
                old_root,
                new_root,
                epoch: self.epoch,
            });
            Ok(())
        }

        #[ink(message)]
        pub fn has_claimed(&self, account: AccountId) -> bool {
            self.claimed.get(account).unwrap_or(false)
        }

        #[ink(message)]
        pub fn merkle_root(&self) -> Hash {
            self.merkle_root
        }

        #[ink(message)]
        pub fn reward_token(&self) -> AccountId {
            self.reward_token
        }

        #[ink(message)]
        pub fn epoch(&self) -> u64 {
            self.epoch
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        fn accounts() -> ink::env::test::DefaultAccounts<ink::env::DefaultEnvironment> {
            ink::env::test::default_accounts::<ink::env::DefaultEnvironment>()
        }
        fn set_caller(c: AccountId) {
            ink::env::test::set_caller::<ink::env::DefaultEnvironment>(c);
        }
        fn leaf(account: AccountId, amount: Balance) -> Hash {
            MerkleRewardsDistributor::leaf_hash(&account, amount)
        }
        fn pair(a: Hash, b: Hash) -> Hash {
            MerkleRewardsDistributor::hash_pair(a, b)
        }

        #[ink::test]
        fn invalid_proof_rejected() {
            let a = accounts();
            let l1 = leaf(a.bob, 100);
            let l2 = leaf(a.charlie, 200);
            let root = pair(l1, l2);
            set_caller(a.alice);
            let mut dist = MerkleRewardsDistributor::new(root, a.django);
            set_caller(a.bob);
            assert_eq!(dist.claim(150, vec![l2]), Err(Error::InvalidProof));
            assert_eq!(dist.claim(100, vec![l1]), Err(Error::InvalidProof));
        }

        #[ink::test]
        fn root_update_epoch_and_access_control() {
            let a = accounts();
            let l1 = leaf(a.bob, 100);
            let l2 = leaf(a.charlie, 200);
            let root = pair(l1, l2);
            set_caller(a.alice);
            let mut dist = MerkleRewardsDistributor::new(root, a.django);
            assert_eq!(dist.epoch(), 0);
            let new_root = pair(leaf(a.bob, 50), l2);
            assert!(dist.set_merkle_root(new_root).is_ok());
            assert_eq!(dist.merkle_root(), new_root);
            assert_eq!(dist.epoch(), 1);
            set_caller(a.bob);
            assert_eq!(dist.set_merkle_root(root), Err(Error::Unauthorized));
        }

        #[ink::test]
        fn claimed_state_default_false() {
            let a = accounts();
            let root = pair(leaf(a.bob, 1), leaf(a.charlie, 2));
            set_caller(a.alice);
            let dist = MerkleRewardsDistributor::new(root, a.django);
            assert!(!dist.has_claimed(a.bob));
        }
    }
}
