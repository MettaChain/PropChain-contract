#![cfg_attr(not(feature = "std"), no_std, no_main)]

#[ink::contract]
mod property_lease {
    use ink::storage::Mapping;

    const BPS: u128 = 10_000;

    #[derive(Debug, Clone, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout))]
    pub struct Lease {
        pub property_id: u64,
        pub landlord: AccountId,
        pub tenant: AccountId,
        pub monthly_rent: Balance,
        pub deposit: Balance,
        pub duration_blocks: u64,
        pub late_fee_bps: u32,
        pub created_block: u64,
        pub active: bool,
        pub deposit_paid: bool,
        pub periods_paid: u64,
    }

    #[derive(Debug, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub enum Error {
        Unauthorized, NotFound, LeaseEnded, DepositAlreadyPaid, RentTooLow, ReentrantCall, TransferFailed, Overflow,
    }

    #[ink(storage)]
    pub struct PropertyLease {
        next_lease_id: u64,
        leases: Mapping<u64, Lease>,
        entered: bool,
    }

    #[ink(event)]
    pub struct LeaseCreated { #[ink(topic)] lease_id: u64, #[ink(topic)] tenant: AccountId, property_id: u64, monthly_rent: Balance, deposit: Balance }
    #[ink(event)]
    pub struct RentPaid { #[ink(topic)] lease_id: u64, #[ink(topic)] tenant: AccountId, amount: Balance, period: u64 }
    #[ink(event)]
    pub struct LateFeeApplied { #[ink(topic)] lease_id: u64, fee: Balance }
    #[ink(event)]
    pub struct LeaseTerminated { #[ink(topic)] lease_id: u64, reason: AccountId }
    #[ink(event)]
    pub struct DepositReturned { #[ink(topic)] lease_id: u64, to: AccountId, amount: Balance }

    impl PropertyLease {
        #[ink(constructor)]
        pub fn new() -> Self { Self { next_lease_id: 1, leases: Mapping::default(), entered: false } }
        fn guard(&mut self) -> Result<(), Error> { if self.entered { return Err(Error::ReentrantCall); } self.entered = true; Ok(()) }
        fn unguard(&mut self) { self.entered = false; }

        #[ink(message)]
        pub fn create_lease(&mut self, property_id: u64, tenant: AccountId, monthly_rent: Balance, deposit: Balance, duration_blocks: u64, late_fee_bps: u32) -> Result<u64, Error> {
            let caller = self.env().caller();
            let id = self.next_lease_id;
            self.next_lease_id = id.saturating_add(1);
            self.leases.insert(id, &Lease { property_id, landlord: caller, tenant, monthly_rent, deposit, duration_blocks, late_fee_bps, created_block: self.env().block_number().into(), active: true, deposit_paid: false, periods_paid: 0 });
            self.env().emit_event(LeaseCreated { lease_id: id, tenant, property_id, monthly_rent, deposit });
            Ok(id)
        }

        #[ink(message, payable)]
        pub fn pay_deposit(&mut self, lease_id: u64) -> Result<(), Error> {
            let mut lease = self.leases.get(lease_id).ok_or(Error::NotFound)?;
            if self.env().caller() != lease.tenant { return Err(Error::Unauthorized); }
            if lease.deposit_paid { return Err(Error::DepositAlreadyPaid); }
            if self.env().transferred_value() < lease.deposit { return Err(Error::RentTooLow); }
            self.env().transfer(lease.landlord, self.env().transferred_value()).map_err(|_| Error::TransferFailed)?;
            lease.deposit_paid = true;
            self.leases.insert(lease_id, &lease);
            Ok(())
        }

        #[ink(message, payable)]
        pub fn pay_rent(&mut self, lease_id: u64) -> Result<(), Error> {
            self.guard()?;
            let res = (|| {
                let mut lease = self.leases.get(lease_id).ok_or(Error::NotFound)?;
                if self.env().caller() != lease.tenant { return Err(Error::Unauthorized); }
                if !lease.active { return Err(Error::LeaseEnded); }
                if self.env().transferred_value() < lease.monthly_rent { return Err(Error::RentTooLow); }
                lease.periods_paid = lease.periods_paid.saturating_add(1);
                self.leases.insert(lease_id, &lease);
                self.env().transfer(lease.landlord, self.env().transferred_value()).map_err(|_| Error::TransferFailed)?;
                self.env().emit_event(RentPaid { lease_id, tenant: lease.tenant, amount: self.env().transferred_value(), period: lease.periods_paid });
                Ok(())
            })();
            self.unguard();
            res
        }

        #[ink(message, payable)]
        pub fn claim_unpaid_rent(&mut self, lease_id: u64) -> Result<Balance, Error> {
            let lease = self.leases.get(lease_id).ok_or(Error::NotFound)?;
            if self.env().caller() != lease.landlord { return Err(Error::Unauthorized); }
            let now: u64 = self.env().block_number().into();
            let periods_elapsed = now.saturating_sub(lease.created_block) / 100u64 + 1;
            let unpaid = periods_elapsed.saturating_sub(lease.periods_paid);
            let claimable = lease.monthly_rent.checked_mul(unpaid as u128).ok_or(Error::Overflow)?;
            Ok(claimable)
        }

        #[ink(message)]
        pub fn terminate_lease(&mut self, lease_id: u64, reason: AccountId) -> Result<(), Error> {
            self.guard()?;
            let res = (|| {
                let mut lease = self.leases.get(lease_id).ok_or(Error::NotFound)?;
                let caller = self.env().caller();
                if caller != lease.landlord && caller != lease.tenant { return Err(Error::Unauthorized); }
                lease.active = false;
                self.leases.insert(lease_id, &lease);
                if lease.deposit_paid && lease.deposit > 0 {
                    self.env().transfer(lease.tenant, lease.deposit.min(self.env().balance())).map_err(|_| Error::TransferFailed)?;
                    self.env().emit_event(DepositReturned { lease_id, to: lease.tenant, amount: lease.deposit });
                }
                self.env().emit_event(LeaseTerminated { lease_id, reason });
                Ok(())
            })();
            self.unguard();
            res
        }

        #[ink(message)] pub fn get_lease(&self, lease_id: u64) -> Option<Lease> { self.leases.get(lease_id) }
        #[ink(message)] pub fn is_expired(&self, lease_id: u64) -> bool {
            match self.leases.get(lease_id) {
                Some(l) => { let now: u64 = self.env().block_number().into(); now.saturating_sub(l.created_block) > l.duration_blocks }
                None => false,
            }
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        fn accounts() -> ink::env::test::DefaultAccounts<ink::env::DefaultEnvironment> { ink::env::test::default_accounts::<ink::env::DefaultEnvironment>() }
        fn set_caller(c: AccountId) { ink::env::test::set_caller::<ink::env::DefaultEnvironment>(c); }
        fn set_value(v: Balance) { ink::env::test::set_value_transferred::<ink::env::DefaultEnvironment>(v); }
        fn set_block(b: BlockNumber) { ink::env::test::set_block_number::<ink::env::DefaultEnvironment>(b); }

        #[ink::test]
        fn lease_lifecycle() {
            let a = accounts(); set_caller(a.alice); set_block(1);
            let mut c = PropertyLease::new();
            let id = c.create_lease(1, a.bob, 1000, 500, 1000, 500).unwrap();
            set_caller(a.bob); set_value(500); c.pay_deposit(id).unwrap();
            set_value(1000); c.pay_rent(id).unwrap();
            assert_eq!(c.get_lease(id).unwrap().periods_paid, 1);
        }
    }
}
