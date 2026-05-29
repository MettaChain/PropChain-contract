// Unit tests for the governance contract (Issue #101 - extracted from lib.rs)

#[cfg(test)]
mod tests {
    use super::*;

    fn default_accounts() -> ink::env::test::DefaultAccounts<ink::env::DefaultEnvironment> {
        ink::env::test::default_accounts::<ink::env::DefaultEnvironment>()
    }

    fn set_caller(caller: AccountId) {
        ink::env::test::set_caller::<ink::env::DefaultEnvironment>(caller);
    }

    fn advance_block(n: u32) {
        ink::env::test::advance_block::<ink::env::DefaultEnvironment>();
        for _ in 1..n {
            ink::env::test::advance_block::<ink::env::DefaultEnvironment>();
        }
    }

    fn create_governance() -> Governance {
        let accounts = default_accounts();
        set_caller(accounts.alice);
        let signers = vec![accounts.alice, accounts.bob, accounts.charlie];
        Governance::new(signers, 2, 10)
    }

    fn dummy_hash() -> Hash {
        Hash::from([0x01; 32])
    }

    #[ink::test]
    fn constructor_sets_admin_and_signers() {
        let gov = create_governance();
        let accounts = default_accounts();
        assert_eq!(gov.get_admin(), accounts.alice);
        assert_eq!(gov.get_signers().len(), 3);
        assert_eq!(gov.get_threshold(), 2);
    }

    #[ink::test]
    fn constructor_clamps_threshold() {
        let accounts = default_accounts();
        set_caller(accounts.alice);
        let signers = vec![accounts.alice, accounts.bob];
        let gov = Governance::new(signers, 99, 10);
        assert_eq!(gov.get_threshold(), 2);
    }

    #[ink::test]
    fn create_proposal_succeeds() {
        let mut gov = create_governance();
        let result = gov.create_proposal(dummy_hash(), GovernanceAction::ModifyProperty, None);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 0);
        assert_eq!(gov.get_active_proposal_count(), 1);
    }

    #[ink::test]
    fn non_signer_cannot_propose() {
        let mut gov = create_governance();
        let accounts = default_accounts();
        set_caller(accounts.django);
        let result = gov.create_proposal(dummy_hash(), GovernanceAction::SaleApproval, None);
        assert_eq!(result, Err(Error::NotASigner));
    }

    #[ink::test]
    fn voting_and_threshold_approval() {
        let mut gov = create_governance();
        let accounts = default_accounts();

        set_caller(accounts.alice);
        gov.create_proposal(dummy_hash(), GovernanceAction::ModifyProperty, None)
            .unwrap();

        gov.vote(0, true).unwrap();
        let proposal = gov.get_proposal(0).unwrap();
        assert_eq!(proposal.votes_for, 1);
        assert_eq!(proposal.status, ProposalStatus::Active);

        set_caller(accounts.bob);
        gov.vote(0, true).unwrap();
        let proposal = gov.get_proposal(0).unwrap();
        assert_eq!(proposal.votes_for, 2);
        assert_eq!(proposal.status, ProposalStatus::Approved);
    }

    #[ink::test]
    fn double_vote_rejected() {
        let mut gov = create_governance();
        let accounts = default_accounts();
        set_caller(accounts.alice);
        gov.create_proposal(dummy_hash(), GovernanceAction::ModifyProperty, None)
            .unwrap();
        gov.vote(0, true).unwrap();
        assert_eq!(gov.vote(0, true), Err(Error::AlreadyVoted));
    }

    #[ink::test]
    fn rejection_when_impossible_to_reach_threshold() {
        let accounts = default_accounts();
        set_caller(accounts.alice);
        let signers = vec![accounts.alice, accounts.bob];
        let mut gov = Governance::new(signers, 2, 10);
        gov.create_proposal(dummy_hash(), GovernanceAction::SaleApproval, None)
            .unwrap();

        gov.vote(0, false).unwrap();
        let proposal = gov.get_proposal(0).unwrap();
        assert_eq!(proposal.status, ProposalStatus::Rejected);
    }

    #[ink::test]
    fn execute_after_timelock() {
        let mut gov = create_governance();
        let accounts = default_accounts();
        set_caller(accounts.alice);
        gov.create_proposal(dummy_hash(), GovernanceAction::ModifyProperty, None)
            .unwrap();
        gov.vote(0, true).unwrap();
        set_caller(accounts.bob);
        gov.vote(0, true).unwrap();

        let result = gov.execute_proposal(0);
        assert_eq!(result, Err(Error::TimelockActive));

        advance_block(11);
        let result = gov.execute_proposal(0);
        assert!(result.is_ok());
        let proposal = gov.get_proposal(0).unwrap();
        assert_eq!(proposal.status, ProposalStatus::Executed);
    }

    #[ink::test]
    fn add_and_remove_signer() {
        let mut gov = create_governance();
        let accounts = default_accounts();
        set_caller(accounts.alice);

        gov.add_signer(accounts.django).unwrap();
        assert_eq!(gov.get_signers().len(), 4);

        gov.remove_signer(accounts.charlie).unwrap();
        assert_eq!(gov.get_signers().len(), 3);
    }

    #[ink::test]
    fn cannot_remove_below_min_signers() {
        let accounts = default_accounts();
        set_caller(accounts.alice);
        let signers = vec![accounts.alice, accounts.bob];
        let mut gov = Governance::new(signers, 2, 10);
        assert_eq!(gov.remove_signer(accounts.bob), Err(Error::MinSigners));
    }

    #[ink::test]
    fn non_admin_cannot_add_signer() {
        let mut gov = create_governance();
        let accounts = default_accounts();
        set_caller(accounts.bob);
        assert_eq!(gov.add_signer(accounts.django), Err(Error::Unauthorized));
    }

    #[ink::test]
    fn update_threshold_succeeds() {
        let mut gov = create_governance();
        gov.update_threshold(3).unwrap();
        assert_eq!(gov.get_threshold(), 3);
    }

    #[ink::test]
    fn invalid_threshold_rejected() {
        let mut gov = create_governance();
        assert_eq!(gov.update_threshold(0), Err(Error::InvalidThreshold));
        assert_eq!(gov.update_threshold(99), Err(Error::InvalidThreshold));
    }

    #[ink::test]
    fn emergency_override_works() {
        let mut gov = create_governance();
        let accounts = default_accounts();
        set_caller(accounts.alice);
        gov.create_proposal(dummy_hash(), GovernanceAction::ModifyProperty, None)
            .unwrap();
        gov.emergency_override(0, true).unwrap();
        let proposal = gov.get_proposal(0).unwrap();
        assert_eq!(proposal.status, ProposalStatus::Executed);
    }

    #[ink::test]
    fn cancel_proposal_by_proposer() {
        let mut gov = create_governance();
        gov.create_proposal(dummy_hash(), GovernanceAction::ModifyProperty, None)
            .unwrap();
        gov.cancel_proposal(0).unwrap();
        let proposal = gov.get_proposal(0).unwrap();
        assert_eq!(proposal.status, ProposalStatus::Cancelled);
        assert_eq!(gov.get_active_proposal_count(), 0);
    }
}
