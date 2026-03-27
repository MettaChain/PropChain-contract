#![cfg_attr(not(feature = "std"), no_std, no_main)]

#[ink::contract]
mod governance {
    use ink::prelude::vec::Vec;
    use ink::storage::Mapping;
    use propchain_traits::constants;
    use propchain_traits::errors::*;

    // =========================================================================
    // Error
    // =========================================================================

    #[derive(Debug, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub enum Error {
        Unauthorized,
        ProposalNotFound,
        AlreadyVoted,
        ProposalClosed,
        ThresholdNotMet,
        TimelockActive,
        InvalidThreshold,
        SignerExists,
        SignerNotFound,
        MinSigners,
        MaxProposals,
        NotASigner,
        ProposalExpired,
    }

    impl core::fmt::Display for Error {
        fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
            match self {
                Error::Unauthorized => write!(f, "Caller is not authorized"),
                Error::ProposalNotFound => write!(f, "Proposal not found"),
                Error::AlreadyVoted => write!(f, "Already voted on this proposal"),
                Error::ProposalClosed => write!(f, "Proposal is closed"),
                Error::ThresholdNotMet => write!(f, "Approval threshold not met"),
                Error::TimelockActive => write!(f, "Timelock period has not elapsed"),
                Error::InvalidThreshold => write!(f, "Invalid threshold value"),
                Error::SignerExists => write!(f, "Signer already exists"),
                Error::SignerNotFound => write!(f, "Signer not found"),
                Error::MinSigners => write!(f, "Cannot go below minimum signers"),
                Error::MaxProposals => write!(f, "Maximum active proposals reached"),
                Error::NotASigner => write!(f, "Caller is not a signer"),
                Error::ProposalExpired => write!(f, "Proposal has expired"),
            }
        }
    }

    impl ContractError for Error {
        fn error_code(&self) -> u32 {
            match self {
                Error::Unauthorized => governance_codes::GOVERNANCE_UNAUTHORIZED,
                Error::ProposalNotFound => governance_codes::GOVERNANCE_PROPOSAL_NOT_FOUND,
                Error::AlreadyVoted => governance_codes::GOVERNANCE_ALREADY_VOTED,
                Error::ProposalClosed => governance_codes::GOVERNANCE_PROPOSAL_CLOSED,
                Error::ThresholdNotMet => governance_codes::GOVERNANCE_THRESHOLD_NOT_MET,
                Error::TimelockActive => governance_codes::GOVERNANCE_TIMELOCK_ACTIVE,
                Error::InvalidThreshold => governance_codes::GOVERNANCE_INVALID_THRESHOLD,
                Error::SignerExists => governance_codes::GOVERNANCE_SIGNER_EXISTS,
                Error::SignerNotFound => governance_codes::GOVERNANCE_SIGNER_NOT_FOUND,
                Error::MinSigners => governance_codes::GOVERNANCE_MIN_SIGNERS,
                Error::MaxProposals => governance_codes::GOVERNANCE_MAX_PROPOSALS,
                Error::NotASigner => governance_codes::GOVERNANCE_NOT_A_SIGNER,
                Error::ProposalExpired => governance_codes::GOVERNANCE_PROPOSAL_EXPIRED,
            }
        }

        fn error_description(&self) -> &'static str {
            match self {
                Error::Unauthorized => "Caller does not have governance permissions",
                Error::ProposalNotFound => "The governance proposal does not exist",
                Error::AlreadyVoted => "Caller has already voted on this proposal",
                Error::ProposalClosed => "The proposal is no longer accepting votes",
                Error::ThresholdNotMet => "Not enough votes to meet the approval threshold",
                Error::TimelockActive => "The timelock period has not elapsed yet",
                Error::InvalidThreshold => "Threshold must be between 1 and signer count",
                Error::SignerExists => "This account is already a signer",
                Error::SignerNotFound => "This account is not a registered signer",
                Error::MinSigners => "Cannot remove signer: minimum signer count reached",
                Error::MaxProposals => "Cannot create proposal: active limit reached",
                Error::NotASigner => "Only signers can perform this action",
                Error::ProposalExpired => "The proposal voting period has expired",
            }
        }

        fn error_category(&self) -> ErrorCategory {
            ErrorCategory::Governance
        }
    }

    // =========================================================================
    // Types
    // =========================================================================

    /// Governance action types that require multisig approval.
    #[derive(
        Debug,
        Clone,
        PartialEq,
        Eq,
        scale::Encode,
        scale::Decode,
        ink::storage::traits::StorageLayout,
    )]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub enum GovernanceAction {
        ModifyProperty,
        SaleApproval,
        ChangeThreshold,
        AddSigner,
        RemoveSigner,
        EmergencyOverride,
    }

    /// Status of a governance proposal.
    #[derive(
        Debug,
        Clone,
        PartialEq,
        Eq,
        scale::Encode,
        scale::Decode,
        ink::storage::traits::StorageLayout,
    )]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub enum ProposalStatus {
        Active,
        Approved,
        Executed,
        Rejected,
        Cancelled,
        Expired,
    }

    /// A governance proposal.
    #[derive(
        Debug,
        Clone,
        PartialEq,
        Eq,
        scale::Encode,
        scale::Decode,
        ink::storage::traits::StorageLayout,
    )]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub struct GovernanceProposal {
        pub id: u64,
        pub proposer: AccountId,
        pub description_hash: Hash,
        pub action_type: GovernanceAction,
        pub target: Option<AccountId>,
        pub threshold: u32,
        pub votes_for: u32,
        pub votes_against: u32,
        pub status: ProposalStatus,
        pub created_at: u64,
        pub executed_at: u64,
        pub timelock_until: u64,
    }

    // =========================================================================
    // Events
    // =========================================================================

    #[ink(event)]
    pub struct ProposalCreated {
        #[ink(topic)]
        pub proposal_id: u64,
        #[ink(topic)]
        pub proposer: AccountId,
        pub action_type: GovernanceAction,
        pub threshold: u32,
    }

    #[ink(event)]
    pub struct VoteCast {
        #[ink(topic)]
        pub proposal_id: u64,
        #[ink(topic)]
        pub voter: AccountId,
        pub support: bool,
    }

    #[ink(event)]
    pub struct ProposalExecuted {
        #[ink(topic)]
        pub proposal_id: u64,
        pub executed_at: u64,
    }

    #[ink(event)]
    pub struct ProposalRejected {
        #[ink(topic)]
        pub proposal_id: u64,
    }

    #[ink(event)]
    pub struct SignerAdded {
        #[ink(topic)]
        pub signer: AccountId,
        #[ink(topic)]
        pub added_by: AccountId,
    }

    #[ink(event)]
    pub struct SignerRemoved {
        #[ink(topic)]
        pub signer: AccountId,
        #[ink(topic)]
        pub removed_by: AccountId,
    }

    #[ink(event)]
    pub struct ThresholdUpdated {
        pub old_threshold: u32,
        pub new_threshold: u32,
    }

    #[ink(event)]
    pub struct EmergencyOverrideUsed {
        #[ink(topic)]
        pub proposal_id: u64,
        #[ink(topic)]
        pub admin: AccountId,
    }

    // =========================================================================
    // Storage
    // =========================================================================

    #[ink(storage)]
    pub struct Governance {
        admin: AccountId,
        signers: Vec<AccountId>,
        threshold: u32,
        proposal_counter: u64,
        active_proposal_count: u32,
        proposals: Mapping<u64, GovernanceProposal>,
        votes: Mapping<(u64, AccountId), bool>,
        timelock_blocks: u64,
    }

    // =========================================================================
    // Implementation
    // =========================================================================

    impl Governance {
        /// Creates a new Governance contract.
        ///
        /// # Arguments
        /// * `signers` - Initial list of signer accounts
        /// * `threshold` - Number of approvals required (must be <= signers.len())
        /// * `timelock_blocks` - Blocks to wait after approval before execution
        #[ink(constructor)]
        pub fn new(signers: Vec<AccountId>, threshold: u32, timelock_blocks: u64) -> Self {
            let caller = Self::env().caller();
            let mut unique_signers = signers;
            unique_signers.dedup();
            let signer_count = unique_signers.len() as u32;
            let safe_threshold = if threshold == 0 || threshold > signer_count {
                signer_count
            } else {
                threshold
            };

            Self {
                admin: caller,
                signers: unique_signers,
                threshold: safe_threshold,
                proposal_counter: 0,
                active_proposal_count: 0,
                proposals: Mapping::default(),
                votes: Mapping::default(),
                timelock_blocks,
            }
        }

        // ----- Queries -----

        /// Returns a proposal by ID.
        #[ink(message)]
        pub fn get_proposal(&self, proposal_id: u64) -> Option<GovernanceProposal> {
            self.proposals.get(proposal_id)
        }

        /// Returns the current list of signers.
        #[ink(message)]
        pub fn get_signers(&self) -> Vec<AccountId> {
            self.signers.clone()
        }

        /// Returns the current approval threshold.
        #[ink(message)]
        pub fn get_threshold(&self) -> u32 {
            self.threshold
        }

        /// Returns the admin address.
        #[ink(message)]
        pub fn get_admin(&self) -> AccountId {
            self.admin
        }

        /// Returns the number of active proposals.
        #[ink(message)]
        pub fn get_active_proposal_count(&self) -> u32 {
            self.active_proposal_count
        }

        // ----- Mutations -----

        /// Creates a new governance proposal. Only signers may propose.
        #[ink(message)]
        pub fn create_proposal(
            &mut self,
            description_hash: Hash,
            action_type: GovernanceAction,
            target: Option<AccountId>,
        ) -> Result<u64, Error> {
            let caller = self.env().caller();
            self.ensure_signer(caller)?;

            if self.active_proposal_count >= constants::GOVERNANCE_MAX_ACTIVE_PROPOSALS {
                return Err(Error::MaxProposals);
            }

            let proposal_id = self.proposal_counter;
            self.proposal_counter = self.proposal_counter.saturating_add(1);
            let now = self.env().block_number() as u64;

            let proposal = GovernanceProposal {
                id: proposal_id,
                proposer: caller,
                description_hash,
                action_type: action_type.clone(),
                target,
                threshold: self.threshold,
                votes_for: 0,
                votes_against: 0,
                status: ProposalStatus::Active,
                created_at: now,
                executed_at: 0,
                timelock_until: 0,
            };

            self.proposals.insert(proposal_id, &proposal);
            self.active_proposal_count = self.active_proposal_count.saturating_add(1);

            self.env().emit_event(ProposalCreated {
                proposal_id,
                proposer: caller,
                action_type,
                threshold: self.threshold,
            });

            Ok(proposal_id)
        }

        /// Casts a vote on an active proposal. Only signers may vote.
        #[ink(message)]
        pub fn vote(&mut self, proposal_id: u64, support: bool) -> Result<(), Error> {
            let caller = self.env().caller();
            self.ensure_signer(caller)?;

            let mut proposal = self
                .proposals
                .get(proposal_id)
                .ok_or(Error::ProposalNotFound)?;

            if proposal.status != ProposalStatus::Active {
                return Err(Error::ProposalClosed);
            }

            if self.votes.contains((proposal_id, caller)) {
                return Err(Error::AlreadyVoted);
            }

            self.votes.insert((proposal_id, caller), &support);
            if support {
                proposal.votes_for = proposal.votes_for.saturating_add(1);
            } else {
                proposal.votes_against = proposal.votes_against.saturating_add(1);
            }

            // Check if threshold reached → move to Approved with timelock
            if proposal.votes_for >= proposal.threshold {
                let now = self.env().block_number() as u64;
                proposal.status = ProposalStatus::Approved;
                proposal.timelock_until = now.saturating_add(self.timelock_blocks);
                self.active_proposal_count = self.active_proposal_count.saturating_sub(1);
            }

            // Check if rejection is certain (remaining votes can't reach threshold)
            let total_signers = self.signers.len() as u32;
            let total_votes = proposal.votes_for.saturating_add(proposal.votes_against);
            let remaining = total_signers.saturating_sub(total_votes);
            if proposal.votes_for.saturating_add(remaining) < proposal.threshold {
                proposal.status = ProposalStatus::Rejected;
                self.active_proposal_count = self.active_proposal_count.saturating_sub(1);
                self.env().emit_event(ProposalRejected { proposal_id });
            }

            self.proposals.insert(proposal_id, &proposal);

            self.env().emit_event(VoteCast {
                proposal_id,
                voter: caller,
                support,
            });

            Ok(())
        }

        /// Executes an approved proposal after the timelock has elapsed.
        #[ink(message)]
        pub fn execute_proposal(&mut self, proposal_id: u64) -> Result<(), Error> {
            let caller = self.env().caller();
            self.ensure_signer(caller)?;

            let mut proposal = self
                .proposals
                .get(proposal_id)
                .ok_or(Error::ProposalNotFound)?;

            if proposal.status != ProposalStatus::Approved {
                return Err(Error::ProposalClosed);
            }

            let now = self.env().block_number() as u64;
            if now < proposal.timelock_until {
                return Err(Error::TimelockActive);
            }

            proposal.status = ProposalStatus::Executed;
            proposal.executed_at = now;
            self.proposals.insert(proposal_id, &proposal);

            self.env().emit_event(ProposalExecuted {
                proposal_id,
                executed_at: now,
            });

            Ok(())
        }

        /// Cancels an active proposal. Only the proposer or admin may cancel.
        #[ink(message)]
        pub fn cancel_proposal(&mut self, proposal_id: u64) -> Result<(), Error> {
            let caller = self.env().caller();
            let mut proposal = self
                .proposals
                .get(proposal_id)
                .ok_or(Error::ProposalNotFound)?;

            if proposal.status != ProposalStatus::Active
                && proposal.status != ProposalStatus::Approved
            {
                return Err(Error::ProposalClosed);
            }

            if caller != proposal.proposer && caller != self.admin {
                return Err(Error::Unauthorized);
            }

            if proposal.status == ProposalStatus::Active {
                self.active_proposal_count = self.active_proposal_count.saturating_sub(1);
            }
            proposal.status = ProposalStatus::Cancelled;
            self.proposals.insert(proposal_id, &proposal);

            Ok(())
        }

        /// Adds a new signer. Only admin may call.
        #[ink(message)]
        pub fn add_signer(&mut self, new_signer: AccountId) -> Result<(), Error> {
            self.ensure_admin()?;

            if self.signers.contains(&new_signer) {
                return Err(Error::SignerExists);
            }

            if self.signers.len() as u32 >= constants::GOVERNANCE_MAX_SIGNERS {
                return Err(Error::MaxProposals);
            }

            self.signers.push(new_signer);

            self.env().emit_event(SignerAdded {
                signer: new_signer,
                added_by: self.env().caller(),
            });

            Ok(())
        }

        /// Removes a signer. Only admin may call.
        #[ink(message)]
        pub fn remove_signer(&mut self, signer: AccountId) -> Result<(), Error> {
            self.ensure_admin()?;

            if self.signers.len() as u32 <= constants::GOVERNANCE_MIN_SIGNERS {
                return Err(Error::MinSigners);
            }

            let pos = self
                .signers
                .iter()
                .position(|s| *s == signer)
                .ok_or(Error::SignerNotFound)?;

            self.signers.swap_remove(pos);

            // Adjust threshold if it's now greater than signer count
            let new_count = self.signers.len() as u32;
            if self.threshold > new_count {
                let old = self.threshold;
                self.threshold = new_count;
                self.env().emit_event(ThresholdUpdated {
                    old_threshold: old,
                    new_threshold: new_count,
                });
            }

            self.env().emit_event(SignerRemoved {
                signer,
                removed_by: self.env().caller(),
            });

            Ok(())
        }

        /// Updates the approval threshold. Only admin may call.
        #[ink(message)]
        pub fn update_threshold(&mut self, new_threshold: u32) -> Result<(), Error> {
            self.ensure_admin()?;

            if new_threshold == 0 || new_threshold > self.signers.len() as u32 {
                return Err(Error::InvalidThreshold);
            }

            let old = self.threshold;
            self.threshold = new_threshold;

            self.env().emit_event(ThresholdUpdated {
                old_threshold: old,
                new_threshold,
            });

            Ok(())
        }

        /// Emergency override: admin can force-execute or reject a proposal.
        #[ink(message)]
        pub fn emergency_override(&mut self, proposal_id: u64, execute: bool) -> Result<(), Error> {
            self.ensure_admin()?;

            let mut proposal = self
                .proposals
                .get(proposal_id)
                .ok_or(Error::ProposalNotFound)?;

            if proposal.status == ProposalStatus::Executed
                || proposal.status == ProposalStatus::Cancelled
            {
                return Err(Error::ProposalClosed);
            }

            if proposal.status == ProposalStatus::Active {
                self.active_proposal_count = self.active_proposal_count.saturating_sub(1);
            }

            let now = self.env().block_number() as u64;
            if execute {
                proposal.status = ProposalStatus::Executed;
                proposal.executed_at = now;
            } else {
                proposal.status = ProposalStatus::Rejected;
            }

            self.proposals.insert(proposal_id, &proposal);

            self.env().emit_event(EmergencyOverrideUsed {
                proposal_id,
                admin: self.env().caller(),
            });

            Ok(())
        }

        // ----- Internal helpers -----

        fn ensure_admin(&self) -> Result<(), Error> {
            if self.env().caller() != self.admin {
                return Err(Error::Unauthorized);
            }
            Ok(())
        }

        fn ensure_signer(&self, account: AccountId) -> Result<(), Error> {
            if !self.signers.contains(&account) {
                return Err(Error::NotASigner);
            }
            Ok(())
        }
    }

    // =========================================================================
    // Tests
    // =========================================================================

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
            Governance::new(signers, 2, 10) // threshold=2, timelock=10 blocks
        }

        fn dummy_hash() -> Hash {
            Hash::from([0x01; 32])
        }

        // ----- Constructor tests -----

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
            assert_eq!(gov.get_threshold(), 2); // clamped to signer count
        }

        // ----- Proposal tests -----

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

        // ----- Voting tests -----

        #[ink::test]
        fn voting_and_threshold_approval() {
            let mut gov = create_governance();
            let accounts = default_accounts();

            // Alice proposes
            set_caller(accounts.alice);
            gov.create_proposal(dummy_hash(), GovernanceAction::ModifyProperty, None)
                .unwrap();

            // Alice votes yes
            gov.vote(0, true).unwrap();
            let proposal = gov.get_proposal(0).unwrap();
            assert_eq!(proposal.votes_for, 1);
            assert_eq!(proposal.status, ProposalStatus::Active);

            // Bob votes yes → threshold met
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
            // 2 signers, threshold 2 — one "no" vote makes it impossible
            let signers = vec![accounts.alice, accounts.bob];
            let mut gov = Governance::new(signers, 2, 10);
            gov.create_proposal(dummy_hash(), GovernanceAction::SaleApproval, None)
                .unwrap();

            // Alice votes no
            gov.vote(0, false).unwrap();
            let proposal = gov.get_proposal(0).unwrap();
            assert_eq!(proposal.status, ProposalStatus::Rejected);
        }

        // ----- Execution tests -----

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

            // Too early
            let result = gov.execute_proposal(0);
            assert_eq!(result, Err(Error::TimelockActive));

            // Advance past timelock
            advance_block(11);
            let result = gov.execute_proposal(0);
            assert!(result.is_ok());
            let proposal = gov.get_proposal(0).unwrap();
            assert_eq!(proposal.status, ProposalStatus::Executed);
        }

        // ----- Signer management tests -----

        #[ink::test]
        fn add_and_remove_signer() {
            let mut gov = create_governance();
            let accounts = default_accounts();
            set_caller(accounts.alice);

            // Add django
            gov.add_signer(accounts.django).unwrap();
            assert_eq!(gov.get_signers().len(), 4);

            // Remove charlie
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

        // ----- Threshold tests -----

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

        // ----- Emergency override tests -----

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

        // ----- Cancel proposal tests -----

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
}
