#![cfg_attr(not(feature = "std"), no_std, no_main)]

#[ink::contract]
mod governance {
    use ink::prelude::vec::Vec;
    use ink::storage::Mapping;
    use propchain_traits::constants;
    use propchain_traits::errors::*;

    include!("errors.rs");
    include!("types.rs");

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
    pub struct QuadraticVoteCast {
        #[ink(topic)]
        pub proposal_id: u64,
        #[ink(topic)]
        pub voter: AccountId,
        pub support: bool,
        pub credits_spent: u32,
        pub voting_weight: u32,
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

    // ── Discussion Forum Event (Issue #233) ─────────────────────────────────

    #[ink(event)]
    pub struct CommentAdded {
        #[ink(topic)]
        pub proposal_id: u64,
        #[ink(topic)]
        pub author: AccountId,
        pub discussion_id: u64,
        pub parent_id: Option<u64>,
    }

    #[ink(event)]
    pub struct EmergencyOverrideUsed {
        #[ink(topic)]
        pub proposal_id: u64,
        #[ink(topic)]
        pub admin: AccountId,
    }

    /// Emitted when auto-execute is toggled for a proposal.
    #[ink(event)]
    pub struct AutoExecuteToggled {
        #[ink(topic)]
        pub proposal_id: u64,
        pub auto_execute: bool,
        pub toggled_by: AccountId,
    }

    /// Emitted when a proposal is automatically executed.
    #[ink(event)]
    pub struct AutoExecuted {
        #[ink(topic)]
        pub proposal_id: u64,
        pub executed_at: u64,
    }

    /// Emitted when a proposal template is created.
    #[ink(event)]
    pub struct TemplateCreated {
        #[ink(topic)]
        pub template_id: u64,
        pub name: String,
        pub action_type: GovernanceAction,
        pub created_by: AccountId,
    }

    /// Emitted when a proposal is created from a template.
    #[ink(event)]
    pub struct ProposalFromTemplate {
        #[ink(topic)]
        pub proposal_id: u64,
        #[ink(topic)]
        pub template_id: u64,
        pub proposer: AccountId,
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
        /// Registered ECDSA public keys for optional cryptographic signature verification
        signer_public_keys: Mapping<AccountId, [u8; 33]>,
        /// Pending admin key rotation request
        pending_admin_rotation: Option<propchain_traits::KeyRotationRequest>,
        // ── Voting Privacy (Issue #234) ───────────────────────────────────────
        /// Commitments: (proposal_id, voter) -> hashed vote commitment
        vote_commitments: Mapping<(u64, AccountId), Hash>,
        /// Reveal phase active: proposal_id -> bool
        reveal_phase_started: Mapping<u64, bool>,
        /// Reveal phase duration in blocks
        reveal_phase_duration: u64,
        // ── Discussion Forum (Issue #233) ───────────────────────────────────────
        /// Comments/discussion for each proposal
        proposal_comments: Mapping<u64, Vec<DiscussionComment>>,
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
                signer_public_keys: Mapping::default(),
                pending_admin_rotation: None,
                vote_commitments: Mapping::default(),
                reveal_phase_started: Mapping::default(),
                reveal_phase_duration: 10_800, // ~18 hours at 6s blocks
                proposal_comments: Mapping::default(),
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

        /// Returns whether the reveal phase has started for a proposal.
        #[ink(message)]
        pub fn is_reveal_phase_started(&self, proposal_id: u64) -> bool {
            self.reveal_phase_started.get(proposal_id).unwrap_or(false)
        }

        /// Returns whether a signer has committed a vote.
        #[ink(message)]
        pub fn has_committed_vote(&self, proposal_id: u64, signer: AccountId) -> bool {
            self.vote_commitments.contains((proposal_id, signer))
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
                is_emergency: false,
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

        /// Creates a new emergency proposal. Only signers may propose.
        /// Emergency proposals require unanimous signer approval but bypass the timelock.
        #[ink(message)]
        pub fn create_emergency_proposal(
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

            // Unanimous approval required for emergency
            let emergency_threshold = self.signers.len() as u32;

            let proposal = GovernanceProposal {
                id: proposal_id,
                proposer: caller,
                description_hash,
                action_type: action_type.clone(),
                target,
                threshold: emergency_threshold,
                votes_for: 0,
                votes_against: 0,
                status: ProposalStatus::Active,
                created_at: now,
                executed_at: 0,
                timelock_until: 0,
                is_emergency: true,
            };

            self.proposals.insert(proposal_id, &proposal);
            self.active_proposal_count = self.active_proposal_count.saturating_add(1);

            self.env().emit_event(ProposalCreated {
                proposal_id,
                proposer: caller,
                action_type,
                threshold: emergency_threshold,
            });

            Ok(proposal_id)
        }

        /// Returns the governance analytics.
        #[ink(message)]
        pub fn get_analytics(&self) -> GovernanceAnalytics {
            let total = self.proposal_counter;
            let mut executed = 0;
            let mut rejected = 0;
            let mut cancelled = 0;
            let mut active = 0;

            let mut total_participation_bps: u64 = 0;
            let mut closed_count = 0;

            let signer_count = self.signers.len() as u64;

            for id in 0..total {
                if let Some(proposal) = self.proposals.get(id) {
                    match proposal.status {
                        ProposalStatus::Active => active += 1,
                        ProposalStatus::Approved => active += 1,
                        ProposalStatus::Executed => {
                            executed += 1;
                            closed_count += 1;
                            if signer_count > 0 {
                                let total_votes =
                                    (proposal.votes_for.saturating_add(proposal.votes_against))
                                        as u64;
                                let bps = total_votes.saturating_mul(10_000) / signer_count;
                                total_participation_bps =
                                    total_participation_bps.saturating_add(bps);
                            }
                        }
                        ProposalStatus::Rejected => {
                            rejected += 1;
                            closed_count += 1;
                            if signer_count > 0 {
                                let total_votes =
                                    (proposal.votes_for.saturating_add(proposal.votes_against))
                                        as u64;
                                let bps = total_votes.saturating_mul(10_000) / signer_count;
                                total_participation_bps =
                                    total_participation_bps.saturating_add(bps);
                            }
                        }
                        ProposalStatus::Cancelled => {
                            cancelled += 1;
                        }
                        ProposalStatus::Expired => {
                            closed_count += 1;
                            if signer_count > 0 {
                                let total_votes =
                                    (proposal.votes_for.saturating_add(proposal.votes_against))
                                        as u64;
                                let bps = total_votes.saturating_mul(10_000) / signer_count;
                                total_participation_bps =
                                    total_participation_bps.saturating_add(bps);
                            }
                        }
                    }
                }
            }

            let avg_participation_bps = if closed_count > 0 {
                (total_participation_bps / closed_count) as u32
            } else {
                0
            };

            GovernanceAnalytics {
                total_proposals: total,
                executed_proposals: executed,
                rejected_proposals: rejected,
                cancelled_proposals: cancelled,
                active_proposals: active,
                avg_participation_bps,
            }
        }

        /// Returns all comments for a proposal.
        #[ink(message)]
        pub fn get_proposal_comments(&self, proposal_id: u64) -> Vec<DiscussionComment> {
            self.proposal_comments.get(&proposal_id).unwrap_or_default()
        }

        /// Returns the participation rate for a specific proposal in basis points.
        #[ink(message)]
        pub fn get_proposal_participation(&self, proposal_id: u64) -> Result<u32, Error> {
            let proposal = self
                .proposals
                .get(proposal_id)
                .ok_or(Error::ProposalNotFound)?;
            let signer_count = self.signers.len() as u32;
            if signer_count == 0 {
                return Ok(0);
            }
            let total_votes = proposal.votes_for.saturating_add(proposal.votes_against);
            let bps = (total_votes as u64).saturating_mul(10_000) / (signer_count as u64);
            Ok(bps as u32)
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
                if proposal.is_emergency {
                    proposal.timelock_until = now; // Bypass timelock
                } else {
                    proposal.timelock_until = now.saturating_add(self.timelock_blocks);
                }
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

        /// Register an ECDSA public key for cryptographic signature verification.
        #[ink(message)]
        pub fn register_public_key(&mut self, public_key: [u8; 33]) -> Result<(), Error> {
            let caller = self.env().caller();
            self.ensure_signer(caller)?;
            self.signer_public_keys.insert(caller, &public_key);
            Ok(())
        }

        /// Vote with optional ECDSA cryptographic signature verification.
        #[ink(message)]
        pub fn vote_with_signature(
            &mut self,
            proposal_id: u64,
            support: bool,
            signed_approval: Option<propchain_traits::SignedApproval>,
        ) -> Result<(), Error> {
            let caller = self.env().caller();

            if let Some(ref approval) = signed_approval {
                let expected_key = self
                    .signer_public_keys
                    .get(caller)
                    .ok_or(Error::Unauthorized)?;
                propchain_traits::crypto::verify_signed_approval(approval, &expected_key)
                    .map_err(|_| Error::Unauthorized)?;

                let expected_hash = propchain_traits::crypto::hash_encoded(&(
                    proposal_id,
                    support,
                    caller,
                    self.env().block_number(),
                ));
                if approval.message_hash != <[u8; 32]>::from(expected_hash) {
                    return Err(Error::Unauthorized);
                }
            }

            self.vote(proposal_id, support)
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

        // ── Voting Privacy (Issue #234) ───────────────────────────────────────

        /// Submit a hashed commitment for a private vote.
        /// The commitment should be hash(proposal_id || voter || support || salt).
        #[ink(message)]
        pub fn commit_vote(&mut self, proposal_id: u64, commitment: Hash) -> Result<(), Error> {
            let caller = self.env().caller();
            self.ensure_signer(caller)?;

            let proposal = self
                .proposals
                .get(proposal_id)
                .ok_or(Error::ProposalNotFound)?;

            if proposal.status != ProposalStatus::Active {
                return Err(Error::ProposalClosed);
            }

            if self.vote_commitments.contains((proposal_id, caller)) {
                return Err(Error::AlreadyVoted);
            }

            self.vote_commitments
                .insert((proposal_id, caller), &commitment);

            Ok(())
        }

        /// Start the reveal phase for a proposal (any signer may call).
        #[ink(message)]
        pub fn start_reveal_phase(&mut self, proposal_id: u64) -> Result<(), Error> {
            let caller = self.env().caller();
            self.ensure_signer(caller)?;

            let proposal = self
                .proposals
                .get(proposal_id)
                .ok_or(Error::ProposalNotFound)?;

            if proposal.status != ProposalStatus::Active {
                return Err(Error::ProposalClosed);
            }

            if self.reveal_phase_started.get(proposal_id).unwrap_or(false) {
                return Err(Error::AlreadyVoted);
            }

            self.reveal_phase_started.insert(proposal_id, &true);
            Ok(())
        }

        /// Reveal a private vote after the commitment phase.
        /// Verifies that the revealed vote matches the earlier commitment.
        #[ink(message)]
        pub fn reveal_vote(
            &mut self,
            proposal_id: u64,
            support: bool,
            salt: [u8; 32],
        ) -> Result<(), Error> {
            let caller = self.env().caller();
            self.ensure_signer(caller)?;

            if !self.reveal_phase_started.get(proposal_id).unwrap_or(false) {
                return Err(Error::ProposalClosed);
            }

            let commitment = self
                .vote_commitments
                .get((proposal_id, caller))
                .ok_or(Error::AlreadyVoted)?;

            // Verify the commitment matches
            let encoded = (proposal_id, caller, support, salt);
            let expected = propchain_traits::crypto::hash_encoded(&encoded);
            if commitment != expected {
                return Err(Error::Unauthorized);
            }

            // Clear commitment to prevent double-reveal
            self.vote_commitments.remove((proposal_id, caller));

            // Record the vote via internal logic
            self.record_vote(proposal_id, caller, support)?;

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

        /// Request a two-step admin rotation with cooldown.
        #[ink(message)]
        pub fn request_admin_rotation(&mut self, new_admin: AccountId) -> Result<(), Error> {
            self.ensure_admin()?;
            let caller = self.env().caller();
            let block = self.env().block_number();
            let effective_at =
                block.saturating_add(propchain_traits::constants::KEY_ROTATION_COOLDOWN_BLOCKS);

            self.pending_admin_rotation = Some(propchain_traits::KeyRotationRequest {
                old_account: caller,
                new_account: new_admin,
                requested_at: block,
                effective_at,
                confirmed: false,
            });

            Ok(())
        }

        /// Confirm a pending admin rotation after cooldown.
        #[ink(message)]
        pub fn confirm_admin_rotation(&mut self) -> Result<(), Error> {
            let caller = self.env().caller();
            let block = self.env().block_number();

            let request = self
                .pending_admin_rotation
                .as_ref()
                .ok_or(Error::ProposalNotFound)?;

            if request.new_account != caller {
                return Err(Error::Unauthorized);
            }
            if block < request.effective_at {
                return Err(Error::TimelockActive);
            }
            let expiry = request
                .effective_at
                .saturating_add(propchain_traits::constants::KEY_ROTATION_EXPIRY_BLOCKS);
            if block > expiry {
                self.pending_admin_rotation = None;
                return Err(Error::ProposalExpired);
            }

            self.admin = caller;
            self.pending_admin_rotation = None;
            Ok(())
        }

        /// Cancel a pending admin rotation.
        #[ink(message)]
        pub fn cancel_admin_rotation(&mut self) -> Result<(), Error> {
            let caller = self.env().caller();
            let request = self
                .pending_admin_rotation
                .as_ref()
                .ok_or(Error::ProposalNotFound)?;

            if caller != request.old_account && caller != request.new_account {
                return Err(Error::Unauthorized);
            }

            self.pending_admin_rotation = None;
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

        /// Internal vote recording logic shared by `vote` and `reveal_vote`.
        fn record_vote(
            &mut self,
            proposal_id: u64,
            caller: AccountId,
            support: bool,
        ) -> Result<(), Error> {
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
                if proposal.is_emergency {
                    proposal.timelock_until = now;
                } else {
                    proposal.timelock_until = now.saturating_add(self.timelock_blocks);
                }
                self.active_proposal_count = self.active_proposal_count.saturating_sub(1);
            }

            // Check if rejection is certain
            let total_signers = self.signers.len() as u32;
            let total_votes = proposal.votes_for.saturating_add(proposal.votes_against);
            let remaining = total_signers.saturating_sub(total_votes);
            if proposal.votes_for.saturating_add(remaining) < proposal.threshold {
                proposal.status = ProposalStatus::Rejected;
                self.active_proposal_count = self.active_proposal_count.saturating_sub(1);
                self.env().emit_event(ProposalRejected { proposal_id });
            }

            self.proposals.insert(proposal_id, &proposal);
            Ok(())
        }
    }

    // =========================================================================
    // Tests
    // =========================================================================
    include!("tests.rs");
}
