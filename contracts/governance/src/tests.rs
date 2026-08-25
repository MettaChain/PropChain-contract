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

    #[ink::test]
    fn emergency_proposal_succeeds_without_timelock() {
        let mut gov = create_governance();
        let accounts = default_accounts();

        // Create emergency proposal
        set_caller(accounts.alice);
        let id = gov.create_emergency_proposal(dummy_hash(), GovernanceAction::ModifyProperty, None)
            .unwrap();

        let proposal = gov.get_proposal(id).unwrap();
        assert!(proposal.is_emergency);
        assert_eq!(proposal.threshold, 3); // Unanimous: all 3 signers

        // Vote on proposal
        gov.vote(id, true).unwrap();
        
        set_caller(accounts.bob);
        gov.vote(id, true).unwrap();

        set_caller(accounts.charlie);
        gov.vote(id, true).unwrap();

        // Once approved, emergency proposals bypass timelock and can be executed immediately!
        let proposal = gov.get_proposal(id).unwrap();
        assert_eq!(proposal.status, ProposalStatus::Approved);
        assert_eq!(
            proposal.timelock_until,
            ink::env::block_number::<ink::env::DefaultEnvironment>() as u64
        );

        // Execute immediately
        gov.execute_proposal(id).unwrap();
        let proposal = gov.get_proposal(id).unwrap();
        assert_eq!(proposal.status, ProposalStatus::Executed);
    }

    #[ink::test]
    fn governance_analytics_and_participation_rates() {
        let mut gov = create_governance();
        let accounts = default_accounts();

        // 1. Check initial empty analytics
        let stats = gov.get_analytics();
        assert_eq!(stats.total_proposals, 0);
        assert_eq!(stats.executed_proposals, 0);
        assert_eq!(stats.avg_participation_bps, 0);

        // 2. Create and execute proposal
        set_caller(accounts.alice);
        gov.create_proposal(dummy_hash(), GovernanceAction::ModifyProperty, None)
            .unwrap();

        // Bob and Charlie vote (2 out of 3 signers vote) -> 66% (6666 bps)
        set_caller(accounts.bob);
        gov.vote(0, true).unwrap();
        set_caller(accounts.charlie);
        gov.vote(0, true).unwrap();

        // Timelock and execute
        advance_block(11);
        set_caller(accounts.alice);
        gov.execute_proposal(0).unwrap();

        // 3. Create another proposal that gets rejected
        let id2 = gov.create_proposal(dummy_hash(), GovernanceAction::SaleApproval, None).unwrap();
        // Alice votes against, Bob votes against -> 2 out of 3 vote (66.6%)
        set_caller(accounts.alice);
        gov.vote(id2, false).unwrap();
        set_caller(accounts.bob);
        gov.vote(id2, false).unwrap();

        let stats = gov.get_analytics();
        assert_eq!(stats.total_proposals, 2);
        assert_eq!(stats.executed_proposals, 1);
        assert_eq!(stats.rejected_proposals, 1);
        // Average participation rate: (6666 + 6666) / 2 = 6666 bps
        assert_eq!(stats.avg_participation_bps, 6666);

        // Proposal participation rate query
        assert_eq!(gov.get_proposal_participation(0).unwrap(), 6666);
    }

    // =========================================================================
    // Voting Privacy: commit / reveal / signed voting (Issue #998)
    // =========================================================================

    /// Commitment must reproduce the exact encoding used by `reveal_vote`:
    /// hash_encoded((proposal_id, caller, support, salt)).
    fn commitment_for(proposal_id: u64, caller: AccountId, support: bool, salt: [u8; 32]) -> Hash {
        propchain_traits::crypto::hash_encoded(&(proposal_id, caller, support, salt))
    }

    #[ink::test]
    fn commit_and_reveal_records_private_vote() {
        let mut gov = create_governance();
        let accounts = default_accounts();

        set_caller(accounts.alice);
        gov.create_proposal(dummy_hash(), GovernanceAction::ModifyProperty, None)
            .unwrap();

        let salt = [0x2A; 32];
        set_caller(accounts.bob);
        gov.commit_vote(0, commitment_for(0, accounts.bob, true, salt))
            .unwrap();

        // Reveal before the reveal phase started is rejected.
        assert_eq!(
            gov.reveal_vote(0, true, salt),
            Err(Error::ProposalClosed)
        );

        // Any signer may start the reveal phase; starting it twice is rejected.
        set_caller(accounts.alice);
        gov.start_reveal_phase(0).unwrap();
        assert!(gov.is_reveal_phase_started(0));
        assert_eq!(
            gov.start_reveal_phase(0),
            Err(Error::AlreadyVoted)
        );

        // A wrong salt does not match the commitment.
        set_caller(accounts.bob);
        let wrong_salt = [0x00; 32];
        assert_eq!(
            gov.reveal_vote(0, true, wrong_salt),
            Err(Error::Unauthorized)
        );

        // Revealing with a support flag different from the committed one also
        // mismatches (the vote is part of the hashed message).
        assert_eq!(
            gov.reveal_vote(0, false, salt),
            Err(Error::Unauthorized)
        );

        // The correct reveal records the vote.
        gov.reveal_vote(0, true, salt).unwrap();
        let proposal = gov.get_proposal(0).unwrap();
        assert_eq!(proposal.votes_for, 1);
        assert_eq!(proposal.status, ProposalStatus::Active);

        // The commitment was cleared to prevent double reveals.
        assert_eq!(
            gov.reveal_vote(0, true, salt),
            Err(Error::AlreadyVoted)
        );
    }

    #[ink::test]
    fn commit_vote_rejects_non_signers_and_bad_state() {
        let mut gov = create_governance();
        let accounts = default_accounts();

        let salt = [0x11; 32];

        // Only signers may commit.
        set_caller(accounts.django);
        assert_eq!(
            gov.commit_vote(0, commitment_for(0, accounts.django, true, salt)),
            Err(Error::NotASigner)
        );

        // Unknown proposal id.
        set_caller(accounts.bob);
        assert_eq!(
            gov.commit_vote(42, commitment_for(42, accounts.bob, true, salt)),
            Err(Error::ProposalNotFound)
        );

        set_caller(accounts.alice);
        gov.create_proposal(dummy_hash(), GovernanceAction::ModifyProperty, None)
            .unwrap();

        set_caller(accounts.bob);
        gov.commit_vote(0, commitment_for(0, accounts.bob, false, salt))
            .unwrap();

        // A second commitment from the same voter is rejected.
        assert_eq!(
            gov.commit_vote(0, commitment_for(0, accounts.bob, false, salt)),
            Err(Error::AlreadyVoted)
        );

        // Non-signers cannot start the reveal phase either.
        set_caller(accounts.django);
        assert_eq!(
            gov.start_reveal_phase(0),
            Err(Error::NotASigner)
        );

        // Reveal from a voter without any commitment fails once the phase is
        // open (the missing commitment maps to AlreadyVoted).
        set_caller(accounts.alice);
        gov.start_reveal_phase(0).unwrap();
        set_caller(accounts.charlie);
        assert_eq!(
            gov.reveal_vote(0, true, salt),
            Err(Error::AlreadyVoted)
        );
    }

    #[ink::test]
    fn vote_with_signature_without_signature_is_plain_vote() {
        let mut gov = create_governance();
        let accounts = default_accounts();

        set_caller(accounts.alice);
        gov.create_proposal(dummy_hash(), GovernanceAction::ModifyProperty, None)
            .unwrap();

        set_caller(accounts.bob);
        gov.vote_with_signature(0, true, None).unwrap();
        let proposal = gov.get_proposal(0).unwrap();
        assert_eq!(proposal.votes_for, 1);
    }

    #[ink::test]
    fn vote_with_signature_rejects_missing_key_and_invalid_signature() {
        let mut gov = create_governance();
        let accounts = default_accounts();

        set_caller(accounts.alice);
        gov.create_proposal(dummy_hash(), GovernanceAction::ModifyProperty, None)
            .unwrap();

        // An approval supplied by a caller with no registered public key is
        // rejected before any signature math runs. The signature bytes are
        // canonical (r/s < curve order, recovery id 0/1) so the engine would
        // recover cleanly rather than panic if it got that far.
        let mut sig = [0x7Fu8; 65];
        sig[64] = 0x01; // recovery id
        let approval = propchain_traits::SignedApproval {
            signature: sig,
            message_hash: [0x01; 32],
        };
        set_caller(accounts.bob);
        assert_eq!(
            gov.vote_with_signature(0, true, Some(approval.clone())),
            Err(Error::Unauthorized)
        );
        assert_eq!(gov.get_proposal(0).unwrap().votes_for, 0);

        // With a key registered, an unrecoverable signature still fails and
        // no vote is recorded.
        set_caller(accounts.alice);
        gov.register_public_key([0x02; 33]).unwrap();
        assert_eq!(
            gov.vote_with_signature(0, true, Some(approval)),
            Err(Error::Unauthorized)
        );
        assert_eq!(gov.get_proposal(0).unwrap().votes_for, 0);
    }

    #[ink::test]
    fn register_public_key_requires_signer() {
        let mut gov = create_governance();
        let accounts = default_accounts();

        set_caller(accounts.alice);
        assert!(gov.register_public_key([0x03; 33]).is_ok());

        set_caller(accounts.django);
        assert_eq!(
            gov.register_public_key([0x04; 33]),
            Err(Error::NotASigner)
        );
    }

    /// Brute-force recount helper mirroring the pre-#972 scan semantics:
    /// tallies each proposal by its *final* status and computes the
    /// participation average over Executed/Rejected proposals.
    fn brute_force_recount(gov: &Governance) -> GovernanceAnalytics {
        let signer_count = gov.get_signers().len() as u64;
        let mut executed = 0u64;
        let mut rejected = 0u64;
        let mut cancelled = 0u64;
        let mut active = 0u64;
        let mut participation_sum = 0u64;
        let mut closed = 0u64;

        for id in 0..gov.proposal_counter {
            if let Some(p) = gov.proposals.get(id) {
                match p.status {
                    ProposalStatus::Active | ProposalStatus::Approved => active += 1,
                    ProposalStatus::Executed => {
                        executed += 1;
                        closed += 1;
                        if signer_count > 0 {
                            participation_sum +=
                                (p.votes_for.saturating_add(p.votes_against) as u64)
                                    .saturating_mul(10_000)
                                    / signer_count;
                        }
                    }
                    ProposalStatus::Rejected => {
                        rejected += 1;
                        closed += 1;
                        if signer_count > 0 {
                            participation_sum +=
                                (p.votes_for.saturating_add(p.votes_against) as u64)
                                    .saturating_mul(10_000)
                                    / signer_count;
                        }
                    }
                    ProposalStatus::Cancelled => cancelled += 1,
                    ProposalStatus::Expired => {}
                }
            }
        }

        GovernanceAnalytics {
            total_proposals: gov.proposal_counter,
            executed_proposals: executed,
            rejected_proposals: rejected,
            cancelled_proposals: cancelled,
            active_proposals: active,
            avg_participation_bps: if closed > 0 {
                (participation_sum / closed) as u32
            } else {
                0
            },
        }
    }

    fn assert_analytics_match_recount(gov: &Governance) {
        let fast = gov.get_analytics();
        let brute = brute_force_recount(gov);
        assert_eq!(fast.total_proposals, brute.total_proposals);
        assert_eq!(fast.executed_proposals, brute.executed_proposals);
        assert_eq!(fast.rejected_proposals, brute.rejected_proposals);
        assert_eq!(fast.cancelled_proposals, brute.cancelled_proposals);
        assert_eq!(fast.active_proposals, brute.active_proposals);
        assert_eq!(fast.avg_participation_bps, brute.avg_participation_bps);
    }

    /// Incremental counters must match a brute-force recount across every
    /// status transition path (vote, execute, cancel, emergency override).
    #[ink::test]
    fn analytics_counters_match_brute_force_recount_across_transitions() {
        let accounts = default_accounts();
        let mut gov = create_governance();

        // p0: approved by 2/3 votes, then executed after timelock.
        let p0 = gov.create_proposal(dummy_hash(), GovernanceAction::ModifyProperty, None).unwrap();
        set_caller(accounts.alice);
        gov.vote(p0, true).unwrap();
        set_caller(accounts.bob);
        gov.vote(p0, true).unwrap(); // reaches Approved
        assert_analytics_match_recount(&gov);
        advance_block(11);
        set_caller(accounts.alice); // signer executes after timelock
        gov.execute_proposal(p0).unwrap();

        // p1: voted down (certain rejection).
        let p1 = gov.create_proposal(dummy_hash(), GovernanceAction::SaleApproval, None).unwrap();
        set_caller(accounts.alice);
        gov.vote(p1, false).unwrap();
        set_caller(accounts.bob);
        gov.vote(p1, false).unwrap();
        assert_analytics_match_recount(&gov);

        // p2: cancelled by proposer while still active.
        let p2 = gov.create_proposal(dummy_hash(), GovernanceAction::SaleApproval, None).unwrap();
        gov.cancel_proposal(p2).unwrap();
        assert_analytics_match_recount(&gov);

        // p3: emergency-rejected straight from Active.
        let p3 = gov.create_proposal(dummy_hash(), GovernanceAction::SaleApproval, None).unwrap();
        set_caller(accounts.alice); // admin-only
        gov.emergency_override(p3, false).unwrap();
        assert_analytics_match_recount(&gov);

        // p4: emergency-executed from Active.
        let p4 = gov.create_proposal(dummy_hash(), GovernanceAction::SaleApproval, None).unwrap();
        gov.emergency_override(p4, true).unwrap();
        assert_analytics_match_recount(&gov);

        // p5: approved but never executed (stays in Approved/timelock state).
        let p5 = gov.create_proposal(dummy_hash(), GovernanceAction::SaleApproval, None).unwrap();
        set_caller(accounts.alice);
        gov.vote(p5, true).unwrap();
        set_caller(accounts.bob);
        gov.vote(p5, true).unwrap();
        assert_analytics_match_recount(&gov);

        // Tricky case: emergency-execute an already-rejected proposal.
        // The recount reads final statuses, so p1 moves rejected -> executed.
        set_caller(accounts.alice); // admin-only
        gov.emergency_override(p1, true).unwrap();
        assert_analytics_match_recount(&gov);

        // Final sanity snapshot.
        let stats = gov.get_analytics();
        assert_eq!(stats.total_proposals, 6);
        assert_eq!(stats.executed_proposals, 3); // p0, p1 (override), p4
        assert_eq!(stats.rejected_proposals, 1); // p3
        assert_eq!(stats.cancelled_proposals, 1); // p2
        assert_eq!(stats.active_proposals, 1); // p5 still in Approved

        // p1 contributed its participation exactly once (when it was
        // rejected), so re-closing it via override must not skew the average.
        // Closures: p0 (6666 bps), p1 (6666 bps), p3 (0), p4 (0) -> avg 3333.
        let closed_bps = (2 * 10_000 / 3) as u32;
        let zero_turnouts = [0u32, 0];
        let expected_avg = (closed_bps + closed_bps + zero_turnouts[0] + zero_turnouts[1]) / 4;
        assert_eq!(stats.avg_participation_bps, expected_avg);
    }

    /// A proposal-heavy history (>1000 proposals) must not degrade or corrupt
    /// analytics: counters are maintained incrementally per transition.
    #[ink::test]
    fn analytics_stays_consistent_over_thousand_proposal_history() {
        let _ = default_accounts();
        let mut gov = create_governance();

        // GOVERNANCE_MAX_ACTIVE_PROPOSALS caps concurrent active proposals,
        // so create batches of 100 and cancel them to free room again.
        let batch = 100usize;
        let rounds = 11;
        for round in 0..rounds {
            for _ in 0..batch {
                let id = gov.create_proposal(dummy_hash(), GovernanceAction::ModifyProperty, None).unwrap();
                assert_eq!(id as usize, round * batch + (id as usize % batch));
            }
            for id in (round * batch) as u64..((round + 1) * batch) as u64 {
                gov.cancel_proposal(id).unwrap();
            }
        }

        let stats = gov.get_analytics();
        assert_eq!(stats.total_proposals, (batch * rounds) as u64);
        assert_eq!(stats.cancelled_proposals, (batch * rounds) as u64);
        assert_eq!(stats.active_proposals, 0);
        assert_eq!(stats.executed_proposals, 0);
        assert_eq!(stats.rejected_proposals, 0);
        assert_eq!(stats.avg_participation_bps, 0);

        assert_analytics_match_recount(&gov);
    }
}

