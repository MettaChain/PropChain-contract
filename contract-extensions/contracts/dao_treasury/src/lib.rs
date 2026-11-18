#![cfg_attr(not(feature = "std"), no_std, no_main)]

#[ink::contract]
mod dao_treasury {
    use ink::storage::Mapping;

    #[derive(Debug, Clone, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout))]
    pub struct SpendingProposal {
        pub recipient: AccountId,
        pub amount: Balance,
        pub description_hash: Hash,
        pub approved: bool,
        pub executed: bool,
        pub rejected: bool,
    }

    #[derive(Debug, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub enum Error {
        Unauthorized, NotFound, NotApproved, AlreadyHandled, InsufficientBalance, ReentrantCall, TransferFailed,
    }

    #[ink(storage)]
    pub struct DaoTreasury {
        admin: AccountId,
        governance: AccountId,
        auto_approve_threshold: Balance,
        next_proposal_id: u64,
        proposals: Mapping<u64, SpendingProposal>,
        entered: bool,
    }

    #[ink(event)]
    pub struct SpendingProposed { #[ink(topic)] proposal_id: u64, #[ink(topic)] recipient: AccountId, amount: Balance }
    #[ink(event)]
    pub struct ProposalApproved { #[ink(topic)] proposal_id: u64 }
    #[ink(event)]
    pub struct ProposalExecuted { #[ink(topic)] proposal_id: u64, #[ink(topic)] recipient: AccountId, amount: Balance }
    #[ink(event)]
    pub struct ProposalRejected { #[ink(topic)] proposal_id: u64 }
    #[ink(event)]
    pub struct TreasuryDeposited { #[ink(topic)] from: AccountId, amount: Balance }

    impl DaoTreasury {
        #[ink(constructor, payable)]
        pub fn new(governance: AccountId) -> Self {
            Self { admin: Self::env().caller(), governance, auto_approve_threshold: 0, next_proposal_id: 1, proposals: Mapping::default(), entered: false }
        }
        fn guard(&mut self) -> Result<(), Error> { if self.entered { return Err(Error::ReentrantCall); } self.entered = true; Ok(()) }
        fn unguard(&mut self) { self.entered = false; }

        #[ink(message, payable)]
        pub fn deposit(&mut self) -> Result<(), Error> {
            let amount = self.env().transferred_value();
            if amount > 0 { self.env().emit_event(TreasuryDeposited { from: self.env().caller(), amount }); }
            Ok(())
        }

        #[ink(message)]
        pub fn propose_spending(&mut self, recipient: AccountId, amount: Balance, description_hash: Hash) -> Result<u64, Error> {
            let id = self.next_proposal_id;
            self.next_proposal_id = id.saturating_add(1);
            let approved = self.auto_approve_threshold > 0 && amount <= self.auto_approve_threshold;
            self.proposals.insert(id, &SpendingProposal { recipient, amount, description_hash, approved, executed: false, rejected: false });
            self.env().emit_event(SpendingProposed { proposal_id: id, recipient, amount });
            if approved { self.env().emit_event(ProposalApproved { proposal_id: id }); }
            Ok(id)
        }

        #[ink(message)]
        pub fn approve_proposal(&mut self, proposal_id: u64) -> Result<(), Error> {
            let caller = self.env().caller();
            if caller != self.governance && caller != self.admin { return Err(Error::Unauthorized); }
            let mut p = self.proposals.get(proposal_id).ok_or(Error::NotFound)?;
            if p.executed || p.rejected { return Err(Error::AlreadyHandled); }
            p.approved = true;
            self.proposals.insert(proposal_id, &p);
            self.env().emit_event(ProposalApproved { proposal_id });
            Ok(())
        }

        #[ink(message)]
        pub fn execute_proposal(&mut self, proposal_id: u64) -> Result<(), Error> {
            self.guard()?; let r = self.execute_inner(proposal_id); self.unguard(); r
        }
        fn execute_inner(&mut self, proposal_id: u64) -> Result<(), Error> {
            let mut p = self.proposals.get(proposal_id).ok_or(Error::NotFound)?;
            if p.executed || p.rejected { return Err(Error::AlreadyHandled); }
            if !p.approved { return Err(Error::NotApproved); }
            if self.env().balance() < p.amount { return Err(Error::InsufficientBalance); }
            p.executed = true;
            self.proposals.insert(proposal_id, &p);
            self.env().transfer(p.recipient, p.amount).map_err(|_| Error::TransferFailed)?;
            self.env().emit_event(ProposalExecuted { proposal_id, recipient: p.recipient, amount: p.amount });
            Ok(())
        }

        #[ink(message)]
        pub fn reject_proposal(&mut self, proposal_id: u64) -> Result<(), Error> {
            let caller = self.env().caller();
            if caller != self.admin && caller != self.governance { return Err(Error::Unauthorized); }
            let mut p = self.proposals.get(proposal_id).ok_or(Error::NotFound)?;
            if p.executed || p.rejected { return Err(Error::AlreadyHandled); }
            p.rejected = true;
            self.proposals.insert(proposal_id, &p);
            self.env().emit_event(ProposalRejected { proposal_id });
            Ok(())
        }

        #[ink(message)] pub fn get_balance(&self) -> Balance { self.env().balance() }
        #[ink(message)] pub fn set_auto_approve_threshold(&mut self, threshold: Balance) -> Result<(), Error> {
            if self.env().caller() != self.admin { return Err(Error::Unauthorized); }
            self.auto_approve_threshold = threshold; Ok(())
        }
        #[ink(message)] pub fn get_proposal(&self, id: u64) -> Option<SpendingProposal> { self.proposals.get(id) }
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        fn accounts() -> ink::env::test::DefaultAccounts<ink::env::DefaultEnvironment> { ink::env::test::default_accounts::<ink::env::DefaultEnvironment>() }
        fn set_caller(c: AccountId) { ink::env::test::set_caller::<ink::env::DefaultEnvironment>(c); }
        fn hash() -> Hash { Hash::from([1u8; 32]) }

        #[ink::test]
        fn propose_approve_flow() {
            let a = accounts(); set_caller(a.alice);
            let mut t = DaoTreasury::new(a.django);
            let id = t.propose_spending(a.bob, 100, hash()).unwrap();
            t.approve_proposal(id).unwrap();
            assert!(t.get_proposal(id).unwrap().approved);
        }
    }
}
