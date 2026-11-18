#![cfg_attr(not(feature = "std"), no_std, no_main)]

#[ink::contract]
mod property_title_registry {
    use ink::prelude::string::String;
    use ink::storage::Mapping;

    #[derive(Debug, Clone, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout))]
    pub struct TitleRecord {
        pub owner: AccountId,
        pub metadata_cid: String,
        pub valuation: u128,
        pub jurisdiction: String,
    }

    #[derive(Debug, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub enum Error {
        Unauthorized, NotFound, ComplianceFailed, ReentrantCall, Overflow,
    }

    #[ink(storage)]
    pub struct PropertyTitleRegistry {
        admin: AccountId,
        oracle: AccountId,
        compliance_registry: AccountId,
        next_token_id: u64,
        titles: Mapping<u64, TitleRecord>,
        entered: bool,
    }

    #[ink(event)]
    pub struct PropertyRegistered { #[ink(topic)] token_id: u64, #[ink(topic)] owner: AccountId, metadata_cid: String, valuation: u128 }
    #[ink(event)]
    pub struct TitleTransferred { #[ink(topic)] token_id: u64, #[ink(topic)] from: AccountId, to: AccountId }
    #[ink(event)]
    pub struct ValuationUpdated { #[ink(topic)] token_id: u64, valuation: u128, oracle_source: String }

    impl PropertyTitleRegistry {
        #[ink(constructor)]
        pub fn new(compliance_registry: AccountId, oracle: AccountId) -> Self {
            Self { admin: Self::env().caller(), oracle, compliance_registry, next_token_id: 1, titles: Mapping::default(), entered: false }
        }
        fn guard(&mut self) -> Result<(), Error> { if self.entered { return Err(Error::ReentrantCall); } self.entered = true; Ok(()) }
        fn unguard(&mut self) { self.entered = false; }

        #[ink(message)]
        pub fn register_property(&mut self, metadata_cid: String, valuation: u128, jurisdiction: String) -> Result<u64, Error> {
            let caller = self.env().caller();
            let id = self.next_token_id;
            self.next_token_id = id.saturating_add(1);
            self.titles.insert(id, &TitleRecord { owner: caller, metadata_cid: metadata_cid.clone(), valuation, jurisdiction });
            self.env().emit_event(PropertyRegistered { token_id: id, owner: caller, metadata_cid, valuation });
            Ok(id)
        }

        fn compliance_ok(&self, _account: &AccountId) -> bool {
            use ink::env::call::{build_call, ExecutionInput, Selector};
            use ink::env::DefaultEnvironment;
            build_call::<DefaultEnvironment>()
                .call(self.compliance_registry)
                .exec_input(ExecutionInput::new(Selector::new(ink::selector_bytes!("is_compliant"))).push_arg(*_account))
                .returns::<bool>()
                .invoke()
                .unwrap_or(true)
        }

        #[ink(message)]
        pub fn transfer_title(&mut self, token_id: u64, new_owner: AccountId) -> Result<(), Error> {
            self.guard()?;
            let res = (|| {
                let caller = self.env().caller();
                let mut title = self.titles.get(token_id).ok_or(Error::NotFound)?;
                if title.owner != caller { return Err(Error::Unauthorized); }
                if !self.compliance_ok(&new_owner) { return Err(Error::ComplianceFailed); }
                title.owner = new_owner;
                self.titles.insert(token_id, &title);
                self.env().emit_event(TitleTransferred { token_id, from: caller, to: new_owner });
                Ok(())
            })();
            self.unguard();
            res
        }

        #[ink(message)] pub fn verify_title(&self, token_id: u64) -> Option<TitleRecord> { self.titles.get(token_id) }

        #[ink(message)]
        pub fn update_valuation(&mut self, token_id: u64, new_valuation: u128, oracle_source: String) -> Result<(), Error> {
            let caller = self.env().caller();
            if caller != self.oracle && caller != self.admin { return Err(Error::Unauthorized); }
            let mut title = self.titles.get(token_id).ok_or(Error::NotFound)?;
            title.valuation = new_valuation;
            self.titles.insert(token_id, &title);
            self.env().emit_event(ValuationUpdated { token_id, valuation: new_valuation, oracle_source });
            Ok(())
        }

        #[ink(message)] pub fn set_compliance_registry(&mut self, registry: AccountId) -> Result<(), Error> {
            if self.env().caller() != self.admin { return Err(Error::Unauthorized); }
            self.compliance_registry = registry; Ok(())
        }
        #[ink(message)] pub fn set_oracle(&mut self, oracle: AccountId) -> Result<(), Error> {
            if self.env().caller() != self.admin { return Err(Error::Unauthorized); }
            self.oracle = oracle; Ok(())
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        fn accounts() -> ink::env::test::DefaultAccounts<ink::env::DefaultEnvironment> { ink::env::test::default_accounts::<ink::env::DefaultEnvironment>() }
        fn set_caller(c: AccountId) { ink::env::test::set_caller::<ink::env::DefaultEnvironment>(c); }

        #[ink::test]
        fn register_and_verify() {
            let a = accounts(); set_caller(a.alice);
            let mut r = PropertyTitleRegistry::new(a.django, a.charlie);
            let id = r.register_property("QmCID".into(), 1000, "NG".into()).unwrap();
            assert_eq!(r.verify_title(id).unwrap().valuation, 1000);
        }

        #[ink::test]
        fn update_valuation_oracle_only() {
            let a = accounts(); set_caller(a.alice);
            let mut r = PropertyTitleRegistry::new(a.django, a.charlie);
            let id = r.register_property("QmCID".into(), 1000, "NG".into()).unwrap();
            set_caller(a.bob);
            assert_eq!(r.update_valuation(id, 2000, "src".into()), Err(Error::Unauthorized));
            set_caller(a.charlie);
            r.update_valuation(id, 2000, "src".into()).unwrap();
        }
    }
}
