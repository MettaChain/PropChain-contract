#![cfg_attr(not(feature = "std"), no_std, no_main)]

#[ink::contract]
mod revenue_splitter {
    use ink::prelude::vec::Vec;
    use ink::storage::Mapping;

    const BPS: u128 = 10_000;
    const MAX_PAYEES: usize = 50;

    #[derive(Debug, Clone, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout))]
    pub struct Payee { pub account: AccountId, pub share_bps: u32 }

    #[derive(Debug, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub enum Error {
        Unauthorized, InvalidShareTotal, PayeeExists, PayeeNotFound, TooManyPayees, ReentrantCall, TransferFailed, Overflow,
    }

    #[ink(storage)]
    pub struct RevenueSplitter {
        admin: AccountId,
        payees: Vec<Payee>,
        total_shares_bps: u32,
        entered: bool,
    }

    #[ink(event)] pub struct RevenueDistributed { #[ink(topic)] by: AccountId, total_amount: Balance, payee_count: u32 }
    #[ink(event)] pub struct PayeeAdded { #[ink(topic)] account: AccountId, share_bps: u32 }
    #[ink(event)] pub struct PayeeRemoved { #[ink(topic)] account: AccountId }
    #[ink(event)] pub struct ShareUpdated { #[ink(topic)] account: AccountId, new_share_bps: u32 }

    impl RevenueSplitter {
        #[ink(constructor)]
        pub fn new(payees: Vec<(AccountId, u32)>) -> Self {
            let mut list = Vec::with_capacity(payees.len());
            let mut total: u32 = 0;
            for (account, share_bps) in payees {
                list.push(Payee { account, share_bps });
                total = total.saturating_add(share_bps);
            }
            Self { admin: Self::env().caller(), payees: list, total_shares_bps: total, entered: false }
        }

        fn only_admin(&self) -> Result<(), Error> {
            if self.env().caller() != self.admin { return Err(Error::Unauthorized); }
            Ok(())
        }
        fn total(&self) -> u32 { self.total_shares_bps }
        fn validate_total(&self) -> Result<(), Error> {
            if self.total() != 10_000 { return Err(Error::InvalidShareTotal); }
            Ok(())
        }

        #[ink(message, payable)]
        pub fn receive(&mut self) {
            // incoming funds are held until distribute() is called
        }

        #[ink(message)]
        pub fn distribute(&mut self) -> Result<Balance, Error> {
            if self.entered { return Err(Error::ReentrantCall); }
            self.entered = true;
            let res = self.distribute_inner();
            self.entered = false;
            res
        }
        fn distribute_inner(&mut self) -> Result<Balance, Error> {
            self.validate_total()?;
            let total = self.env().balance();
            if total == 0 { return Ok(0); }
            let mut distributed: Balance = 0;
            for payee in self.payees.iter() {
                let amount = total.checked_mul(payee.share_bps as u128).ok_or(Error::Overflow)? / BPS;
                if amount > 0 {
                    self.env().transfer(payee.account, amount).map_err(|_| Error::TransferFailed)?;
                    distributed = distributed.saturating_add(amount);
                }
            }
            self.env().emit_event(RevenueDistributed { by: self.env().caller(), total_amount: distributed, payee_count: self.payees.len() as u32 });
            Ok(distributed)
        }

        #[ink(message)]
        pub fn add_payee(&mut self, account: AccountId, share_bps: u32) -> Result<(), Error> {
            self.only_admin()?;
            if self.payees.len() >= MAX_PAYEES { return Err(Error::TooManyPayees); }
            if self.payees.iter().any(|p| p.account == account) { return Err(Error::PayeeExists); }
            self.payees.push(Payee { account, share_bps });
            self.total_shares_bps = self.total_shares_bps.saturating_add(share_bps);
            self.env().emit_event(PayeeAdded { account, share_bps });
            Ok(())
        }

        #[ink(message)]
        pub fn remove_payee(&mut self, account: AccountId) -> Result<(), Error> {
            self.only_admin()?;
            if let Some(idx) = self.payees.iter().position(|p| p.account == account) {
                let removed = self.payees.remove(idx);
                self.total_shares_bps = self.total_shares_bps.saturating_sub(removed.share_bps);
                self.env().emit_event(PayeeRemoved { account });
                Ok(())
            } else { Err(Error::PayeeNotFound) }
        }

        #[ink(message)]
        pub fn update_share(&mut self, account: AccountId, new_share_bps: u32) -> Result<(), Error> {
            self.only_admin()?;
            if let Some(p) = self.payees.iter_mut().find(|p| p.account == account) {
                self.total_shares_bps = self.total_shares_bps.saturating_sub(p.share_bps).saturating_add(new_share_bps);
                p.share_bps = new_share_bps;
                self.env().emit_event(ShareUpdated { account, new_share_bps });
                Ok(())
            } else { Err(Error::PayeeNotFound) }
        }

        #[ink(message)] pub fn get_payees(&self) -> Vec<Payee> { self.payees.clone() }
        #[ink(message)] pub fn total_shares_bps(&self) -> u32 { self.total_shares_bps }
        #[ink(message)] pub fn is_valid_distribution(&self) -> bool { self.total() == 10_000 }
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        fn accounts() -> ink::env::test::DefaultAccounts<ink::env::DefaultEnvironment> { ink::env::test::default_accounts::<ink::env::DefaultEnvironment>() }
        fn set_caller(c: AccountId) { ink::env::test::set_caller::<ink::env::DefaultEnvironment>(c); }

        #[ink::test]
        fn split_distribution() {
            let a = accounts(); set_caller(a.alice);
            let mut s = RevenueSplitter::new(vec![(a.bob, 6000), (a.charlie, 4000)]);
            assert_eq!(s.total_shares_bps(), 10_000);
            assert!(s.is_valid_distribution());
        }

        #[ink::test]
        fn add_remove_payee() {
            let a = accounts(); set_caller(a.alice);
            let mut s = RevenueSplitter::new(vec![(a.bob, 10_000)]);
            s.add_payee(a.charlie, 1000).unwrap();
            s.remove_payee(a.charlie).unwrap();
            assert_eq!(s.get_payees().len(), 1);
        }
    }
}
