#![cfg_attr(not(feature = "std"), no_std, no_main)]

#[ink::contract]
mod governance_timelock {
    use ink::env::hash::{Keccak256, HashOutput};
    use ink::prelude::vec::Vec;
    use ink::storage::Mapping;

    #[derive(Debug, Clone, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout))]
    pub struct QueuedProposal {
        pub target: AccountId,
        pub value: Balance,
        pub data_hash: Hash,
        pub eta: BlockNumber,
        pub queued: bool,
        pub executed: bool,
        pub cancelled: bool,
    }

    #[derive(Debug, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub enum Error {
        Unauthorized, NotQueued, TooEarly, Expired, AlreadyDone, ReentrantCall, TransferFailed,
    }

    #[ink(storage)]
    pub struct GovernanceTimelock {
        admin: AccountId,
        delay: BlockNumber,
        grace_period: BlockNumber,
        queue: Mapping<Hash, QueuedProposal>,
        queue_count: u64,
        entered: bool,
    }

    #[ink(event)]
    pub struct ProposalQueued { #[ink(topic)] id: Hash, #[ink(topic)] target: AccountId, eta: BlockNumber }
    #[ink(event)]
    pub struct ProposalExecuted { #[ink(topic)] id: Hash, #[ink(topic)] target: AccountId }
    #[ink(event)]
    pub struct ProposalCancelled { #[ink(topic)] id: Hash, #[ink(topic)] by: AccountId }

    impl GovernanceTimelock {
        #[ink(constructor)]
        pub fn new(delay: BlockNumber, admin: AccountId) -> Self {
            Self { admin, delay, grace_period: delay.saturating_mul(2).max(1), queue: Mapping::default(), queue_count: 0, entered: false }
        }
        fn guard(&mut self) -> Result<(), Error> { if self.entered { return Err(Error::ReentrantCall); } self.entered = true; Ok(()) }
        fn unguard(&mut self) { self.entered = false; }

        fn proposal_hash(target: AccountId, value: Balance, data: &Vec<u8>, eta: BlockNumber) -> Hash {
            let mut encoded = Vec::new();
            encoded.extend_from_slice(&scale::Encode::encode(&target));
            encoded.extend_from_slice(&scale::Encode::encode(&value));
            encoded.extend_from_slice(&scale::Encode::encode(data));
            encoded.extend_from_slice(&scale::Encode::encode(&eta));
            let mut out = <Keccak256 as HashOutput>::Type::default();
            ink::env::hash_bytes::<Keccak256>(&encoded, &mut out);
            out.into()
        }

        #[ink(message)]
        pub fn queue(&mut self, target: AccountId, value: Balance, data: Vec<u8>, eta: BlockNumber) -> Result<Hash, Error> {
            if self.env().caller() != self.admin { return Err(Error::Unauthorized); }
            let min_eta = self.env().block_number().saturating_add(self.delay);
            let effective_eta = if eta < min_eta { min_eta } else { eta };
            let id = Self::proposal_hash(target, value, &data, effective_eta);
            let mut dh = <Keccak256 as HashOutput>::Type::default();
            ink::env::hash_bytes::<Keccak256>(&data, &mut dh);
            let data_hash: Hash = dh.into();
            self.queue.insert(id, &QueuedProposal { target, value, data_hash, eta: effective_eta, queued: true, executed: false, cancelled: false });
            self.queue_count = self.queue_count.saturating_add(1);
            self.env().emit_event(ProposalQueued { id, target, eta: effective_eta });
            Ok(id)
        }

        #[ink(message)]
        pub fn execute(&mut self, target: AccountId, value: Balance, data: Vec<u8>, eta: BlockNumber) -> Result<(), Error> {
            self.guard()?; let r = self.execute_inner(target, value, data, eta); self.unguard(); r
        }
        fn execute_inner(&mut self, target: AccountId, value: Balance, data: Vec<u8>, eta: BlockNumber) -> Result<(), Error> {
            let id = Self::proposal_hash(target, value, &data, eta);
            let mut p = self.queue.get(id).ok_or(Error::NotQueued)?;
            if p.executed || p.cancelled { return Err(Error::AlreadyDone); }
            let now = self.env().block_number();
            if now < p.eta { return Err(Error::TooEarly); }
            if now > p.eta.saturating_add(self.grace_period) { return Err(Error::Expired); }
            p.executed = true;
            self.queue.insert(id, &p);
            if value > 0 { self.env().transfer(target, value).map_err(|_| Error::TransferFailed)?; }
            let _ = data;
            self.env().emit_event(ProposalExecuted { id, target });
            Ok(())
        }

        #[ink(message)]
        pub fn cancel(&mut self, target: AccountId, value: Balance, data: Vec<u8>, eta: BlockNumber) -> Result<(), Error> {
            if self.env().caller() != self.admin { return Err(Error::Unauthorized); }
            let id = Self::proposal_hash(target, value, &data, eta);
            let mut p = self.queue.get(id).ok_or(Error::NotQueued)?;
            if p.executed || p.cancelled { return Err(Error::AlreadyDone); }
            p.cancelled = true;
            self.queue.insert(id, &p);
            self.env().emit_event(ProposalCancelled { id, by: self.env().caller() });
            Ok(())
        }

        #[ink(message)] pub fn get_queue_state(&self) -> (u64, BlockNumber, BlockNumber) { (self.queue_count, self.delay, self.grace_period) }
        #[ink(message)] pub fn is_queued(&self, target: AccountId, value: Balance, data: Vec<u8>, eta: BlockNumber) -> bool {
            let id = Self::proposal_hash(target, value, &data, eta);
            self.queue.get(id).map(|p| p.queued && !p.executed && !p.cancelled).unwrap_or(false)
        }
        #[ink(message)] pub fn timelock_delay(&self) -> BlockNumber { self.delay }
        #[ink(message)] pub fn admin(&self) -> AccountId { self.admin }
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        fn accounts() -> ink::env::test::DefaultAccounts<ink::env::DefaultEnvironment> { ink::env::test::default_accounts::<ink::env::DefaultEnvironment>() }
        fn set_caller(c: AccountId) { ink::env::test::set_caller::<ink::env::DefaultEnvironment>(c); }
        fn set_block(b: BlockNumber) { ink::env::test::set_block_number::<ink::env::DefaultEnvironment>(b); }

        #[ink::test]
        fn queue_execute_after_delay() {
            let a = accounts(); set_caller(a.alice); set_block(1);
            let mut tl = GovernanceTimelock::new(10, a.alice);
            tl.queue(a.bob, 0, vec![1,2,3], 20).unwrap();
            set_block(25);
            assert!(tl.execute(a.bob, 0, vec![1,2,3], 20).is_ok());
        }
    }
}
