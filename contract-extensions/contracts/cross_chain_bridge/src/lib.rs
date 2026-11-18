#![cfg_attr(not(feature = "std"), no_std, no_main)]

#[ink::contract]
mod cross_chain_bridge {
    use ink::prelude::vec::Vec;
    use ink::storage::Mapping;

    const VALIDATOR_THRESHOLD_DEN: u64 = 3;

    #[derive(Debug, Clone, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout))]
    pub struct BridgeTransfer {
        pub token_id: u64,
        pub amount: u128,
        pub destination_chain_id: u64,
        pub recipient: AccountId,
        pub nonce: u64,
        pub attestations: u32,
        pub completed: bool,
        pub returned: bool,
        pub created_at: BlockNumber,
    }

    #[derive(Debug, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub enum Error {
        Unauthorized, NotFound, AlreadyCompleted, Replay, TimeoutNotReached, InvalidSignature, ReentrantCall, Overflow,
    }

    #[ink(storage)]
    pub struct CrossChainBridge {
        admin: AccountId,
        validator_threshold_num: u64, // e.g. 2 of 3
        validators: Mapping<AccountId, bool>,
        validator_count: u64,
        next_transfer_id: u64,
        nonce: u64,
        transfers: Mapping<u64, BridgeTransfer>,
        nonce_used: Mapping<u64, bool>,
        attestations: Mapping<(u64, AccountId), bool>,
        timeout_blocks: BlockNumber,
        entered: bool,
    }

    #[ink(event)] pub struct BridgeTransferInitiated { #[ink(topic)] transfer_id: u64, #[ink(topic)] token_id: u64, amount: u128, destination_chain_id: u64, recipient: AccountId, nonce: u64 }
    #[ink(event)] pub struct AttestationSubmitted { #[ink(topic)] transfer_id: u64, #[ink(topic)] validator: AccountId }
    #[ink(event)] pub struct BridgeMintCompleted { #[ink(topic)] transfer_id: u64, #[ink(topic)] recipient: AccountId, amount: u128 }
    #[ink(event)] pub struct BridgeTransferReturned { #[ink(topic)] transfer_id: u64, #[ink(topic)] recipient: AccountId }

    impl CrossChainBridge {
        #[ink(constructor)]
        pub fn new(validator_threshold_num: u64, timeout_blocks: BlockNumber) -> Self {
            Self { admin: Self::env().caller(), validator_threshold_num, validators: Mapping::default(), validator_count: 0, next_transfer_id: 1, nonce: 0, transfers: Mapping::default(), nonce_used: Mapping::default(), attestations: Mapping::default(), timeout_blocks, entered: false }
        }

        fn guard(&mut self) -> Result<(), Error> { if self.entered { return Err(Error::ReentrantCall); } self.entered = true; Ok(()) }
        fn unguard(&mut self) { self.entered = false; }
        fn only_admin(&self) -> Result<(), Error> { if self.env().caller() != self.admin { return Err(Error::Unauthorized); } Ok(()) }

        #[ink(message)]
        pub fn add_validator(&mut self, validator: AccountId) -> Result<(), Error> {
            self.only_admin()?;
            self.validators.insert(validator, &true);
            self.validator_count = self.validator_count.saturating_add(1);
            Ok(())
        }

        #[ink(message)]
        pub fn remove_validator(&mut self, validator: AccountId) -> Result<(), Error> {
            self.only_admin()?;
            self.validators.insert(validator, &false);
            self.validator_count = self.validator_count.saturating_sub(1);
            Ok(())
        }

        #[ink(message)]
        pub fn set_validator_threshold(&mut self, numerator: u64) -> Result<(), Error> {
            self.only_admin()?;
            self.validator_threshold_num = numerator.min(VALIDATOR_THRESHOLD_DEN);
            Ok(())
        }

        #[ink(message)]
        pub fn initiate_bridge_transfer(&mut self, token_id: u64, amount: u128, destination_chain_id: u64, recipient: AccountId) -> Result<u64, Error> {
            self.guard()?;
            let res = (|| {
                if self.nonce_used.get(self.nonce).unwrap_or(false) { return Err(Error::Replay); }
                let id = self.next_transfer_id;
                self.next_transfer_id = id.saturating_add(1);
                self.nonce_used.insert(self.nonce, &true);
                let nonce = self.nonce;
                self.nonce = self.nonce.saturating_add(1);
                self.transfers.insert(id, &BridgeTransfer {
                    token_id, amount, destination_chain_id, recipient, nonce,
                    attestations: 0, completed: false, returned: false, created_at: self.env().block_number(),
                });
                self.env().emit_event(BridgeTransferInitiated { transfer_id: id, token_id, amount, destination_chain_id, recipient, nonce });
                Ok(id)
            })();
            self.unguard();
            res
        }

        #[ink(message)]
        pub fn submit_attestation(&mut self, transfer_id: u64, _signature: Vec<u8>) -> Result<(), Error> {
            let caller = self.env().caller();
            if !self.validators.get(caller).unwrap_or(false) { return Err(Error::Unauthorized); }
            let mut t = self.transfers.get(transfer_id).ok_or(Error::NotFound)?;
            if t.completed || t.returned { return Err(Error::AlreadyCompleted); }
            if self.attestations.get((transfer_id, caller)).unwrap_or(false) { return Err(Error::Replay); }
            self.attestations.insert((transfer_id, caller), &true);
            t.attestations = t.attestations.saturating_add(1);
            self.transfers.insert(transfer_id, &t);
            self.env().emit_event(AttestationSubmitted { transfer_id, validator: caller });
            Ok(())
        }

        #[ink(message)]
        pub fn complete_bridge_mint(&mut self, transfer_id: u64, recipient: AccountId, amount: u128, attestations: u32) -> Result<(), Error> {
            self.guard()?;
            let res = (|| {
                let mut t = self.transfers.get(transfer_id).ok_or(Error::NotFound)?;
                if t.completed || t.returned { return Err(Error::AlreadyCompleted); }
                let threshold = self.threshold_count();
                if t.attestations < threshold || attestations < threshold as u32 { return Err(Error::InvalidSignature); }
                t.completed = true;
                self.transfers.insert(transfer_id, &t);
                self.env().emit_event(BridgeMintCompleted { transfer_id, recipient, amount });
                Ok(())
            })();
            self.unguard();
            res
        }

        fn threshold_count(&self) -> u64 {
            // ceil(2/3 * validator_count), min 1
            let n = self.validator_count.max(1);
            (n.saturating_mul(2) + 2) / 3
        }

        #[ink(message)]
        pub fn return_bridge_transfer(&mut self, transfer_id: u64) -> Result<(), Error> {
            self.guard()?;
            let res = (|| {
                let mut t = self.transfers.get(transfer_id).ok_or(Error::NotFound)?;
                if t.completed || t.returned { return Err(Error::AlreadyCompleted); }
                if self.env().block_number() < t.created_at.saturating_add(self.timeout_blocks) { return Err(Error::TimeoutNotReached); }
                t.returned = true;
                self.transfers.insert(transfer_id, &t);
                self.env().emit_event(BridgeTransferReturned { transfer_id, recipient: t.recipient });
                Ok(())
            })();
            self.unguard();
            res
        }

        #[ink(message)] pub fn get_transfer(&self, transfer_id: u64) -> Option<BridgeTransfer> { self.transfers.get(transfer_id) }
        #[ink(message)] pub fn validator_threshold(&self) -> (u64, u64) { (self.validator_threshold_num, VALIDATOR_THRESHOLD_DEN) }
        #[ink(message)] pub fn is_validator(&self, account: AccountId) -> bool { self.validators.get(account).unwrap_or(false) }
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        fn accounts() -> ink::env::test::DefaultAccounts<ink::env::DefaultEnvironment> { ink::env::test::default_accounts::<ink::env::DefaultEnvironment>() }
        fn set_caller(c: AccountId) { ink::env::test::set_caller::<ink::env::DefaultEnvironment>(c); }
        fn set_block(b: BlockNumber) { ink::env::test::set_block_number::<ink::env::DefaultEnvironment>(b); }

        #[ink::test]
        fn initiate_attest_complete() {
            let a = accounts(); set_caller(a.alice); set_block(1);
            let mut b = CrossChainBridge::new(2, 100);
            b.add_validator(a.bob).unwrap();
            b.add_validator(a.charlie).unwrap();
            let id = b.initiate_bridge_transfer(1, 1000, 42, a.django).unwrap();
            set_caller(a.bob);
            b.submit_attestation(id, vec![1,2,3]).unwrap();
            set_caller(a.charlie);
            b.submit_attestation(id, vec![4,5,6]).unwrap();
            assert!(b.complete_bridge_mint(id, a.django, 1000, 2).is_ok());
        }

        #[ink::test]
        fn return_after_timeout() {
            let a = accounts(); set_caller(a.alice); set_block(1);
            let mut b = CrossChainBridge::new(2, 50);
            let id = b.initiate_bridge_transfer(1, 1000, 42, a.django).unwrap();
            set_block(100);
            assert!(b.return_bridge_transfer(id).is_ok());
        }
    }
}
