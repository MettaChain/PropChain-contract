#![cfg_attr(not(feature = "std"), no_std, no_main)]

#[ink::contract]
mod hoa_voting {
    use ink::storage::Mapping;

    #[derive(Debug, Clone, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout))]
    pub struct Proposal {
        pub description_hash: Hash,
        pub execution_data: Vec<u8>,
        pub voting_end: BlockNumber,
        pub quorum_bps: u32,
        pub supermajority: bool,
        pub votes_for: u128,
        pub votes_against: u128,
        pub total_votes: u128,
        pub executed: bool,
        pub rejected: bool,
    }

    #[derive(Debug, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub enum Error {
        Unauthorized, NotFound, AlreadyVoted, VotingActive, VotingEnded, InvalidState, ReentrantCall, Overflow,
    }

    #[ink(storage)]
    pub struct HoaVoting {
        fractional_contract: AccountId,
        total_shares: Balance,
        quorum_bps: u32,
        next_proposal_id: u64,
        proposals: Mapping<u64, Proposal>,
        shares: Mapping<AccountId, Balance>,
        voted: Mapping<(u64, AccountId), bool>,
        entered: bool,
    }

    #[ink(event)] pub struct ProposalCreated { #[ink(topic)] proposal_id: u64, description_hash: Hash, voting_end: BlockNumber }
    #[ink(event)] pub struct VoteCast { #[ink(topic)] proposal_id: u64, #[ink(topic)] voter: AccountId, support: bool, weight: u128 }
    #[ink(event)] pub struct ProposalExecuted { #[ink(topic)] proposal_id: u64 }
    #[ink(event)] pub struct ProposalRejected { #[ink(topic)] proposal_id: u64 }

    impl HoaVoting {
        #[ink(constructor)]
        pub fn new(fractional_contract: AccountId, total_shares: Balance, quorum_bps: u32) -> Self {
            Self { fractional_contract, total_shares, quorum_bps: quorum_bps.min(10_000), next_proposal_id: 1, proposals: Mapping::default(), shares: Mapping::default(), voted: Mapping::default(), entered: false }
        }

        #[ink(message)]
        pub fn set_member_shares(&mut self, member: AccountId, shares: Balance) -> Result<(), Error> {
            self.shares.insert(member, &shares); Ok(())
        }

        fn voting_power(&self, member: AccountId) -> Balance {
            use ink::env::call::{build_call, ExecutionInput, Selector};
            use ink::env::DefaultEnvironment;
            // Query fractional contract for share balance
            let queried: Balance = build_call::<DefaultEnvironment>()
                .call(self.fractional_contract)
                .exec_input(ExecutionInput::new(Selector::new(ink::selector_bytes!("balance_of"))).push_arg(member))
                .returns::<Balance>()
                .invoke()
                .unwrap_or(0);
            if queried > 0 { queried } else { self.shares.get(member).unwrap_or(0) }
        }

        #[ink(message)]
        pub fn get_voting_power(&self, member: AccountId) -> Balance {
            self.voting_power(member)
        }

        #[ink(message)]
        pub fn create_proposal(&mut self, description_hash: Hash, execution_data: Vec<u8>, voting_period: BlockNumber) -> Result<u64, Error> {
            let caller = self.env().caller();
            if self.voting_power(caller) == 0 { return Err(Error::Unauthorized); }
            let id = self.next_proposal_id;
            self.next_proposal_id = id.saturating_add(1);
            let voting_end = self.env().block_number().saturating_add(voting_period);
            self.proposals.insert(id, &Proposal {
                description_hash, execution_data, voting_end, quorum_bps: self.quorum_bps, supermajority: false,
                votes_for: 0, votes_against: 0, total_votes: 0, executed: false, rejected: false,
            });
            self.env().emit_event(ProposalCreated { proposal_id: id, description_hash, voting_end });
            Ok(id)
        }

        #[ink(message)]
        pub fn cast_vote(&mut self, proposal_id: u64, support: bool) -> Result<(), Error> {
            let caller = self.env().caller();
            if self.voted.get((proposal_id, caller)).unwrap_or(false) { return Err(Error::AlreadyVoted); }
            let mut proposal = self.proposals.get(proposal_id).ok_or(Error::NotFound)?;
            if self.env().block_number() >= proposal.voting_end { return Err(Error::VotingEnded); }
            let weight = self.voting_power(caller);
            if support { proposal.votes_for = proposal.votes_for.saturating_add(weight); } else { proposal.votes_against = proposal.votes_against.saturating_add(weight); }
            proposal.total_votes = proposal.total_votes.saturating_add(weight);
            self.voted.insert((proposal_id, caller), &true);
            self.proposals.insert(proposal_id, &proposal);
            self.env().emit_event(VoteCast { proposal_id, voter: caller, support, weight });
            Ok(())
        }

        #[ink(message)]
        pub fn execute_proposal(&mut self, proposal_id: u64) -> Result<(), Error> {
            let mut proposal = self.proposals.get(proposal_id).ok_or(Error::NotFound)?;
            if proposal.executed || proposal.rejected { return Err(Error::InvalidState); }
            if self.env().block_number() < proposal.voting_end { return Err(Error::VotingActive); }
            // quorum: percentage of total shares required
            let quorum_required = self.total_shares.saturating_mul(proposal.quorum_bps as u128) / 10_000u128;
            if proposal.total_votes < quorum_required { proposal.rejected = true; self.proposals.insert(proposal_id, &proposal); self.env().emit_event(ProposalRejected { proposal_id }); return Ok(()); }
            let majority_needed = if proposal.supermajority { (proposal.total_votes * 2 + 2) / 3 } else { proposal.total_votes / 2 + 1 };
            if proposal.votes_for < majority_needed { proposal.rejected = true; self.proposals.insert(proposal_id, &proposal); self.env().emit_event(ProposalRejected { proposal_id }); return Ok(()); }
            proposal.executed = true;
            self.proposals.insert(proposal_id, &proposal);
            self.env().emit_event(ProposalExecuted { proposal_id });
            Ok(())
        }

        #[ink(message)] pub fn get_proposal(&self, proposal_id: u64) -> Option<Proposal> { self.proposals.get(proposal_id) }
        #[ink(message)] pub fn has_voted(&self, proposal_id: u64, voter: AccountId) -> bool { self.voted.get((proposal_id, voter)).unwrap_or(false) }
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        fn accounts() -> ink::env::test::DefaultAccounts<ink::env::DefaultEnvironment> { ink::env::test::default_accounts::<ink::env::DefaultEnvironment>() }
        fn set_caller(c: AccountId) { ink::env::test::set_caller::<ink::env::DefaultEnvironment>(c); }
        fn set_block(b: BlockNumber) { ink::env::test::set_block_number::<ink::env::DefaultEnvironment>(b); }

        #[ink::test]
        fn vote_and_execute() {
            let a = accounts(); set_caller(a.alice);
            let mut v = HoaVoting::new(a.django, 10_000, 2000);
            v.set_member_shares(a.alice, 1000).unwrap();
            v.set_member_shares(a.bob, 500).unwrap();
            set_caller(a.alice);
            let id = v.create_proposal(Hash::from([1u8;32]), vec![], 100).unwrap();
            v.cast_vote(id, true).unwrap();
            set_block(200);
            assert!(v.execute_proposal(id).is_ok());
            assert!(v.get_proposal(id).unwrap().executed);
        }
    }
}
