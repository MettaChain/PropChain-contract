#![cfg_attr(not(feature = "std"), no_std, no_main)]

#[ink::contract]
mod property_escrow {
    use ink::prelude::vec::Vec;
    use ink::storage::Mapping;

    #[derive(Debug, Clone, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout))]
    pub struct Condition {
        pub description_hash: Hash,
        pub confirmer: AccountId,
        pub confirmed: bool,
    }

    #[derive(Debug, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub enum Error {
        Unauthorized, InvalidCondition, AlreadyConfirmed, NotAllConfirmed, Disputed, NotDisputed,
        TimeoutNotReached, AlreadyResolved, AlreadyRefunded, ReentrantCall, TransferFailed, InvalidState,
    }

    #[ink(storage)]
    pub struct PropertyEscrow {
        buyer: AccountId,
        seller: AccountId,
        arbitrator: AccountId,
        escrow_amount: Balance,
        funded: bool,
        conditions: Mapping<u32, Condition>,
        condition_count: u32,
        disputed: bool,
        released: bool,
        refunded: bool,
        timeout_block: BlockNumber,
        entered: bool,
    }

    #[ink(event)] pub struct ConditionConfirmed { #[ink(topic)] condition_id: u32, #[ink(topic)] confirmer: AccountId }
    #[ink(event)] pub struct FundsReleased { #[ink(topic)] to: AccountId, amount: Balance }
    #[ink(event)] pub struct DisputeRaised { #[ink(topic)] by: AccountId }
    #[ink(event)] pub struct DisputeResolved { #[ink(topic)] by: AccountId, release_to_buyer: bool }
    #[ink(event)] pub struct RefundIssued { #[ink(topic)] to: AccountId, amount: Balance }

    impl PropertyEscrow {
        #[ink(constructor, payable)]
        pub fn new(buyer: AccountId, seller: AccountId, arbitrator: AccountId, escrow_amount: Balance, condition_confirmer_hashes: Vec<(AccountId, Hash)>, timeout_blocks: BlockNumber) -> Self {
            let now = Self::env().block_number();
            let mut conditions = Mapping::default();
            let mut count: u32 = 0;
            for (confirmer, description_hash) in condition_confirmer_hashes {
                conditions.insert(count, &Condition { description_hash, confirmer, confirmed: false });
                count = count.saturating_add(1);
            }
            let transferred = Self::env().transferred_value();
            Self { buyer, seller, arbitrator, escrow_amount, funded: transferred >= escrow_amount && escrow_amount > 0, conditions, condition_count: count, disputed: false, released: false, refunded: false, timeout_block: now.saturating_add(timeout_blocks), entered: false }
        }
        fn guard(&mut self) -> Result<(), Error> { if self.entered { return Err(Error::ReentrantCall); } self.entered = true; Ok(()) }
        fn unguard(&mut self) { self.entered = false; }
        fn all_conditions_met(&self) -> bool {
            if self.condition_count == 0 { return true; }
            let mut i = 0;
            while i < self.condition_count {
                match self.conditions.get(i) { Some(c) if c.confirmed => { i += 1; } _ => return false }
            }
            true
        }

        #[ink(message)]
        pub fn confirm_condition(&mut self, condition_id: u32) -> Result<(), Error> {
            if self.disputed { return Err(Error::Disputed); }
            let caller = self.env().caller();
            let mut cond = self.conditions.get(condition_id).ok_or(Error::InvalidCondition)?;
            if cond.confirmer != caller { return Err(Error::Unauthorized); }
            if cond.confirmed { return Err(Error::AlreadyConfirmed); }
            cond.confirmed = true;
            self.conditions.insert(condition_id, &cond);
            self.env().emit_event(ConditionConfirmed { condition_id, confirmer: caller });
            if self.all_conditions_met() && self.funded && !self.released && !self.refunded { let _ = self.try_release(); }
            Ok(())
        }

        fn try_release(&mut self) -> Result<(), Error> {
            self.guard()?;
            let res = (|| {
                if self.disputed || self.released || self.refunded { return Err(Error::InvalidState); }
                if !self.all_conditions_met() { return Err(Error::NotAllConfirmed); }
                self.released = true;
                let amount = self.env().balance().min(self.escrow_amount);
                self.env().transfer(self.seller, amount).map_err(|_| Error::TransferFailed)?;
                self.env().emit_event(FundsReleased { to: self.seller, amount });
                Ok(())
            })();
            self.unguard();
            res
        }

        #[ink(message)]
        pub fn raise_dispute(&mut self) -> Result<(), Error> {
            let caller = self.env().caller();
            if caller != self.buyer && caller != self.seller { return Err(Error::Unauthorized); }
            if self.released || self.refunded { return Err(Error::AlreadyResolved); }
            self.disputed = true;
            self.env().emit_event(DisputeRaised { by: caller });
            Ok(())
        }

        #[ink(message)]
        pub fn resolve_dispute(&mut self, release_to_buyer: bool) -> Result<(), Error> {
            self.guard()?;
            let res = self.resolve_dispute_inner(release_to_buyer);
            self.unguard();
            res
        }
        fn resolve_dispute_inner(&mut self, release_to_buyer: bool) -> Result<(), Error> {
            let caller = self.env().caller();
            if caller != self.arbitrator { return Err(Error::Unauthorized); }
            if !self.disputed { return Err(Error::NotDisputed); }
            if self.released || self.refunded { return Err(Error::AlreadyResolved); }
            let amount = self.env().balance().min(self.escrow_amount);
            let to = if release_to_buyer { self.buyer } else { self.seller };
            self.env().transfer(to, amount).map_err(|_| Error::TransferFailed)?;
            if release_to_buyer { self.refunded = true; self.env().emit_event(RefundIssued { to, amount }); }
            else { self.released = true; self.env().emit_event(FundsReleased { to, amount }); }
            self.env().emit_event(DisputeResolved { by: caller, release_to_buyer });
            Ok(())
        }

        #[ink(message)]
        pub fn refund(&mut self) -> Result<(), Error> {
            self.guard()?;
            let res = self.refund_inner();
            self.unguard();
            res
        }
        fn refund_inner(&mut self) -> Result<(), Error> {
            if self.released || self.refunded { return Err(Error::AlreadyRefunded); }
            if self.disputed { return Err(Error::Disputed); }
            if self.env().block_number() < self.timeout_block { return Err(Error::TimeoutNotReached); }
            if self.all_conditions_met() { return Err(Error::InvalidState); }
            self.refunded = true;
            let amount = self.env().balance().min(self.escrow_amount);
            self.env().transfer(self.buyer, amount).map_err(|_| Error::TransferFailed)?;
            self.env().emit_event(RefundIssued { to: self.buyer, amount });
            Ok(())
        }

        #[ink(message)] pub fn is_released(&self) -> bool { self.released }
        #[ink(message)] pub fn is_refunded(&self) -> bool { self.refunded }
        #[ink(message)] pub fn is_disputed(&self) -> bool { self.disputed }
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use ink::env::hash::{Blake2x256, HashOutput};
        fn accounts() -> ink::env::test::DefaultAccounts<ink::env::DefaultEnvironment> { ink::env::test::default_accounts::<ink::env::DefaultEnvironment>() }
        fn set_caller(c: AccountId) { ink::env::test::set_caller::<ink::env::DefaultEnvironment>(c); }
        fn set_block(b: BlockNumber) { ink::env::test::set_block_number::<ink::env::DefaultEnvironment>(b); }
        fn set_value(v: Balance) { ink::env::test::set_value_transferred::<ink::env::DefaultEnvironment>(v); }
        fn h(input: &[u8]) -> Hash { let mut out = <Blake2x256 as HashOutput>::Type::default(); ink::env::hash_bytes::<Blake2x256>(input, &mut out); out.into() }

        #[ink::test]
        fn full_release_when_conditions_met() {
            let a = accounts(); set_block(1); set_value(1000);
            let conds = vec![(a.bob, h(b"c1"))];
            let mut escrow = PropertyEscrow::new(a.alice, a.charlie, a.django, 1000, conds, 100);
            set_caller(a.bob);
            assert!(escrow.confirm_condition(0).is_ok());
            assert!(escrow.is_released());
        }

        #[ink::test]
        fn dispute_and_resolve_to_buyer() {
            let a = accounts(); set_block(1); set_value(1000);
            let conds = vec![(a.bob, h(b"c1"))];
            let mut escrow = PropertyEscrow::new(a.alice, a.charlie, a.django, 1000, conds, 100);
            set_caller(a.alice); escrow.raise_dispute().unwrap();
            set_caller(a.django); escrow.resolve_dispute(true).unwrap();
            assert!(escrow.is_refunded());
        }

        #[ink::test]
        fn refund_after_timeout_when_not_met() {
            let a = accounts(); set_block(1); set_value(1000);
            let conds = vec![(a.bob, h(b"c1"))];
            let mut escrow = PropertyEscrow::new(a.alice, a.charlie, a.django, 1000, conds, 10);
            set_block(20);
            assert!(escrow.refund().is_ok());
            assert!(escrow.is_refunded());
        }
    }
}
