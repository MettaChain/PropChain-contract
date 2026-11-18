#![cfg_attr(not(feature = "std"), no_std, no_main)]

#[ink::contract]
mod property_maintenance {
    use ink::storage::Mapping;

    #[derive(Debug, Clone, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout))]
    pub struct MaintenanceRequest {
        pub owner: AccountId,
        pub property_id: u64,
        pub description_hash: Hash,
        pub budget: Balance,
        pub provider: Option<AccountId>,
        pub completion_hash: Option<Hash>,
        pub status: u8,
    }

    #[derive(Debug, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub enum Error {
        Unauthorized, NotFound, InvalidState, ReentrantCall, TransferFailed, Overflow,
    }

    #[ink(storage)]
    pub struct PropertyMaintenance {
        arbitrator: AccountId,
        next_id: u64,
        requests: Mapping<u64, MaintenanceRequest>,
        escrow: Mapping<u64, Balance>,
        entered: bool,
    }

    #[ink(event)] pub struct RequestCreated { #[ink(topic)] request_id: u64, #[ink(topic)] owner: AccountId, property_id: u64, budget: Balance }
    #[ink(event)] pub struct ProviderAssigned { #[ink(topic)] request_id: u64, #[ink(topic)] provider: AccountId }
    #[ink(event)] pub struct WorkSubmitted { #[ink(topic)] request_id: u64, #[ink(topic)] completion_hash: Hash }
    #[ink(event)] pub struct WorkApproved { #[ink(topic)] request_id: u64, amount: Balance }
    #[ink(event)] pub struct DisputeRaised { #[ink(topic)] request_id: u64, by: AccountId, reason: u64 }
    #[ink(event)] pub struct DisputeResolved { #[ink(topic)] request_id: u64, provider_share_bps: u32 }

    impl PropertyMaintenance {
        #[ink(constructor)]
        pub fn new(arbitrator: AccountId) -> Self {
            Self { arbitrator, next_id: 1, requests: Mapping::default(), escrow: Mapping::default(), entered: false }
        }
        fn guard(&mut self) -> Result<(), Error> { if self.entered { return Err(Error::ReentrantCall); } self.entered = true; Ok(()) }
        fn unguard(&mut self) { self.entered = false; }

        #[ink(message, payable)]
        pub fn create_request(&mut self, property_id: u64, description_hash: Hash, budget: Balance) -> Result<u64, Error> {
            let owner = self.env().caller();
            let id = self.next_id;
            self.next_id = id.saturating_add(1);
            self.requests.insert(id, &MaintenanceRequest { owner, property_id, description_hash, budget, provider: None, completion_hash: None, status: 0 });
            self.escrow.insert(id, &budget);
            self.env().emit_event(RequestCreated { request_id: id, owner, property_id, budget });
            Ok(id)
        }

        #[ink(message)]
        pub fn assign_provider(&mut self, request_id: u64, provider: AccountId) -> Result<(), Error> {
            let caller = self.env().caller();
            let mut req = self.requests.get(request_id).ok_or(Error::NotFound)?;
            if req.owner != caller { return Err(Error::Unauthorized); }
            req.provider = Some(provider);
            req.status = 1;
            self.requests.insert(request_id, &req);
            self.env().emit_event(ProviderAssigned { request_id, provider });
            Ok(())
        }

        #[ink(message)]
        pub fn provider_claim(&mut self, request_id: u64) -> Result<(), Error> {
            let caller = self.env().caller();
            let mut req = self.requests.get(request_id).ok_or(Error::NotFound)?;
            if req.provider != Some(caller) { return Err(Error::Unauthorized); }
            req.status = 2;
            self.requests.insert(request_id, &req);
            Ok(())
        }

        #[ink(message)]
        pub fn submit_work(&mut self, request_id: u64, completion_hash: Hash) -> Result<(), Error> {
            let caller = self.env().caller();
            let mut req = self.requests.get(request_id).ok_or(Error::NotFound)?;
            if req.provider != Some(caller) { return Err(Error::Unauthorized); }
            req.completion_hash = Some(completion_hash);
            req.status = 3;
            self.requests.insert(request_id, &req);
            self.env().emit_event(WorkSubmitted { request_id, completion_hash });
            Ok(())
        }

        #[ink(message)]
        pub fn approve_work(&mut self, request_id: u64) -> Result<(), Error> {
            self.guard()?;
            let res = self.approve_inner(request_id);
            self.unguard();
            res
        }
        fn approve_inner(&mut self, request_id: u64) -> Result<(), Error> {
            let caller = self.env().caller();
            let mut req = self.requests.get(request_id).ok_or(Error::NotFound)?;
            if req.owner != caller { return Err(Error::Unauthorized); }
            if req.status != 3 { return Err(Error::InvalidState); }
            let amount = self.escrow.get(request_id).unwrap_or(0);
            req.status = 4;
            self.requests.insert(request_id, &req);
            self.escrow.insert(request_id, &0);
            if let Some(p) = req.provider {
                self.env().transfer(p, amount).map_err(|_| Error::TransferFailed)?;
            }
            self.env().emit_event(WorkApproved { request_id, amount });
            Ok(())
        }

        #[ink(message)]
        pub fn dispute_work(&mut self, request_id: u64, reason: u64) -> Result<(), Error> {
            let caller = self.env().caller();
            let mut req = self.requests.get(request_id).ok_or(Error::NotFound)?;
            if Some(caller) != req.provider && caller != req.owner { return Err(Error::Unauthorized); }
            req.status = 5;
            self.requests.insert(request_id, &req);
            self.env().emit_event(DisputeRaised { request_id, by: caller, reason });
            Ok(())
        }

        #[ink(message)]
        pub fn resolve_dispute(&mut self, request_id: u64, provider_share_bps: u32) -> Result<(), Error> {
            self.guard()?;
            let res = self.resolve_inner(request_id, provider_share_bps);
            self.unguard();
            res
        }
        fn resolve_inner(&mut self, request_id: u64, provider_share_bps: u32) -> Result<(), Error> {
            if self.env().caller() != self.arbitrator { return Err(Error::Unauthorized); }
            let mut req = self.requests.get(request_id).ok_or(Error::NotFound)?;
            if req.status != 5 { return Err(Error::InvalidState); }
            let amount = self.escrow.get(request_id).unwrap_or(0);
            let provider_amount = amount.saturating_mul(provider_share_bps.min(10_000) as u128) / 10_000u128;
            let owner_amount = amount.saturating_sub(provider_amount);
            if let Some(p) = req.provider {
                if provider_amount > 0 { self.env().transfer(p, provider_amount).map_err(|_| Error::TransferFailed)?; }
            }
            if owner_amount > 0 { self.env().transfer(req.owner, owner_amount).map_err(|_| Error::TransferFailed)?; }
            self.escrow.insert(request_id, &0);
            req.status = 6;
            self.requests.insert(request_id, &req);
            self.env().emit_event(DisputeResolved { request_id, provider_share_bps });
            Ok(())
        }

        #[ink(message)] pub fn get_request(&self, request_id: u64) -> Option<MaintenanceRequest> { self.requests.get(request_id) }
        #[ink(message)] pub fn escrow_balance(&self, request_id: u64) -> Balance { self.escrow.get(request_id).unwrap_or(0) }
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        fn accounts() -> ink::env::test::DefaultAccounts<ink::env::DefaultEnvironment> { ink::env::test::default_accounts::<ink::env::DefaultEnvironment>() }
        fn set_caller(c: AccountId) { ink::env::test::set_caller::<ink::env::DefaultEnvironment>(c); }
        fn set_value(v: Balance) { ink::env::test::set_value_transferred::<ink::env::DefaultEnvironment>(v); }

        #[ink::test]
        fn full_lifecycle() {
            let a = accounts(); set_caller(a.alice);
            let mut m = PropertyMaintenance::new(a.django);
            set_value(1000);
            let id = m.create_request(1, Hash::from([1u8;32]), 1000).unwrap();
            m.assign_provider(id, a.bob).unwrap();
            set_caller(a.bob); m.provider_claim(id).unwrap();
            m.submit_work(id, Hash::from([2u8;32])).unwrap();
            set_caller(a.alice); m.approve_work(id).unwrap();
            assert_eq!(m.escrow_balance(id), 0);
        }
    }
}
