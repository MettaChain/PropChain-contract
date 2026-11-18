#![cfg_attr(not(feature = "std"), no_std, no_main)]
#![allow(clippy::new_without_default)]

//! # Milestone-based crowdfunding (#1441)
//!
//! Backers' money is held in this contract (escrow) and released to the project
//! owner in **milestone tranches** once backers approve the evidence.
//!
//! ```text
//! Funding --goal reached--> Active --last milestone approved--> Completed
//!    |                         |  \--milestone rejected--> Disputed (refunds open)
//!    \--deadline, goal missed--> Failed (refunds open)
//!                              \--submission window missed--> refunds open
//! ```
//!
//! ## Rules
//!
//! * Each milestone has a description hash and a **release share** in basis
//!   points; shares sum to 10 000. Approving milestone `i` releases
//!   `raised * share_i / 10 000` (the last milestone releases whatever escrow is
//!   left, so rounding dust is never stranded).
//! * Milestones are worked **in order**. The owner submits evidence for the
//!   current milestone within `submission_window_blocks` of the stage start (the
//!   moment funding completed or the previous tranche was released).
//! * Backers vote with weight equal to their contribution. A milestone is
//!   **approved** once approving weight reaches `quorum_bps` of the amount raised,
//!   and **rejected** once approval can no longer reach quorum (rejecting weight
//!   exceeds `10 000 - quorum_bps` of the raised amount); rejection puts the
//!   campaign into `Disputed`.
//! * **Refunds** open when the campaign `Failed`, is `Disputed`, or the owner
//!   missed the submission window of the current milestone. A backer's refund is
//!   their share of the escrow that has *not* been released:
//!   `contribution * (raised - released) / raised`, paid once.
//!
//! The dispute is only a state change (refunds open); this contract does not
//! arbitrate it.

#[ink::contract]
pub mod milestone_crowdfunding {
    use ink::prelude::vec::Vec;
    use ink::storage::Mapping;

    pub const MAX_MILESTONES: usize = 10;

    #[derive(Debug, Clone, Copy, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout))]
    pub struct MilestoneSpec {
        pub description_hash: Hash,
        /// Share of the raised amount released on approval, in basis points.
        pub release_bps: u32,
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout))]
    pub enum CampaignStatus {
        Funding,
        Active,
        Completed,
        Failed,
        Disputed,
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout))]
    pub enum MilestoneStatus {
        Pending,
        Submitted,
        Approved,
        Rejected,
    }

    #[derive(Debug, Clone, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout))]
    pub struct Campaign {
        pub owner: AccountId,
        pub funding_goal: u128,
        pub deadline_block: u32,
        pub submission_window_blocks: u32,
        pub quorum_bps: u32,
        pub raised: u128,
        pub released: u128,
        pub status: CampaignStatus,
        pub current_milestone: u32,
        pub stage_start: u32,
        pub milestones: Vec<MilestoneSpec>,
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout))]
    pub struct MilestoneState {
        pub status: MilestoneStatus,
        pub evidence_hash: Option<Hash>,
        pub approve_weight: u128,
        pub reject_weight: u128,
    }

    impl MilestoneState {
        fn pending() -> Self {
            Self { status: MilestoneStatus::Pending, evidence_hash: None, approve_weight: 0, reject_weight: 0 }
        }
    }

    #[derive(Debug, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub enum Error {
        Unauthorized,
        CampaignNotFound,
        InvalidParameters,
        InvalidStatus,
        DeadlinePassed,
        DeadlineNotPassed,
        ZeroAmount,
        /// The index is not the milestone currently being worked on.
        WrongMilestone,
        /// The owner missed the submission window.
        SubmissionWindowClosed,
        NotABacker,
        AlreadyVoted,
        NotRefundable,
        NothingToRefund,
        TransferFailed,
    }

    #[ink(event)]
    pub struct CampaignCreated {
        #[ink(topic)]
        pub campaign_id: u64,
        pub owner: AccountId,
        pub funding_goal: u128,
    }

    #[ink(event)]
    pub struct Invested {
        #[ink(topic)]
        pub campaign_id: u64,
        pub backer: AccountId,
        pub amount: u128,
    }

    #[ink(event)]
    pub struct MilestoneSubmitted {
        #[ink(topic)]
        pub campaign_id: u64,
        pub milestone_index: u32,
        pub evidence_hash: Hash,
    }

    #[ink(event)]
    pub struct MilestoneApproved {
        #[ink(topic)]
        pub campaign_id: u64,
        pub milestone_index: u32,
    }

    #[ink(event)]
    pub struct MilestoneRejected {
        #[ink(topic)]
        pub campaign_id: u64,
        pub milestone_index: u32,
        pub reason_hash: Hash,
    }

    #[ink(event)]
    pub struct FundsReleased {
        #[ink(topic)]
        pub campaign_id: u64,
        pub milestone_index: u32,
        pub amount: u128,
    }

    #[ink(event)]
    pub struct Refunded {
        #[ink(topic)]
        pub campaign_id: u64,
        pub backer: AccountId,
        pub amount: u128,
    }

    #[ink(storage)]
    pub struct MilestoneCrowdfunding {
        next_id: u64,
        campaigns: Mapping<u64, Campaign>,
        milestone_states: Mapping<(u64, u32), MilestoneState>,
        contributions: Mapping<(u64, AccountId), u128>,
        refunded: Mapping<(u64, AccountId), bool>,
        voted: Mapping<(u64, u32, AccountId), bool>,
    }

    impl MilestoneCrowdfunding {
        #[ink(constructor)]
        pub fn new() -> Self {
            Self {
                next_id: 1,
                campaigns: Mapping::default(),
                milestone_states: Mapping::default(),
                contributions: Mapping::default(),
                refunded: Mapping::default(),
                voted: Mapping::default(),
            }
        }

        // ── Campaign set-up ────────────────────────────────────────────

        #[ink(message)]
        pub fn create_campaign_with_milestones(
            &mut self,
            milestones: Vec<MilestoneSpec>,
            funding_goal: u128,
            deadline_block: u32,
            submission_window_blocks: u32,
            quorum_bps: u32,
        ) -> Result<u64, Error> {
            let now = self.env().block_number();
            let n = milestones.len();
            let sum: u64 = milestones.iter().map(|m| m.release_bps as u64).sum();
            if n == 0
                || n > MAX_MILESTONES
                || sum != 10_000
                || milestones.iter().any(|m| m.release_bps == 0)
                || funding_goal == 0
                || deadline_block <= now
                || submission_window_blocks == 0
                || quorum_bps == 0
                || quorum_bps > 10_000
            {
                return Err(Error::InvalidParameters);
            }
            let owner = self.env().caller();
            let id = self.next_id;
            self.next_id += 1;
            for i in 0..n as u32 {
                self.milestone_states.insert((id, i), &MilestoneState::pending());
            }
            self.campaigns.insert(
                id,
                &Campaign {
                    owner,
                    funding_goal,
                    deadline_block,
                    submission_window_blocks,
                    quorum_bps,
                    raised: 0,
                    released: 0,
                    status: CampaignStatus::Funding,
                    current_milestone: 0,
                    stage_start: now,
                    milestones,
                },
            );
            self.env().emit_event(CampaignCreated { campaign_id: id, owner, funding_goal });
            Ok(id)
        }

        // ── Funding ────────────────────────────────────────────────────

        /// Back a campaign with the value sent. Reaching the goal activates it.
        #[ink(message, payable)]
        pub fn invest(&mut self, campaign_id: u64) -> Result<(), Error> {
            let amount = self.env().transferred_value();
            if amount == 0 {
                return Err(Error::ZeroAmount);
            }
            let mut c = self.load(campaign_id)?;
            if c.status != CampaignStatus::Funding {
                return Err(Error::InvalidStatus);
            }
            let now = self.env().block_number();
            if now > c.deadline_block {
                return Err(Error::DeadlinePassed);
            }
            let backer = self.env().caller();
            let held = self.contributions.get((campaign_id, backer)).unwrap_or(0);
            self.contributions.insert((campaign_id, backer), &held.saturating_add(amount));
            c.raised = c.raised.saturating_add(amount);
            if c.raised >= c.funding_goal {
                c.status = CampaignStatus::Active;
                c.stage_start = now;
            }
            self.campaigns.insert(campaign_id, &c);
            self.env().emit_event(Invested { campaign_id, backer, amount });
            Ok(())
        }

        /// After the funding deadline, mark a campaign that missed its goal `Failed`.
        #[ink(message)]
        pub fn finalize_funding(&mut self, campaign_id: u64) -> Result<(), Error> {
            let mut c = self.load(campaign_id)?;
            if c.status != CampaignStatus::Funding {
                return Err(Error::InvalidStatus);
            }
            if self.env().block_number() <= c.deadline_block {
                return Err(Error::DeadlineNotPassed);
            }
            c.status = CampaignStatus::Failed;
            self.campaigns.insert(campaign_id, &c);
            Ok(())
        }

        // ── Milestones ─────────────────────────────────────────────────

        /// The owner submits evidence for the milestone currently being worked on.
        #[ink(message)]
        pub fn submit_milestone(&mut self, campaign_id: u64, milestone_index: u32, evidence_hash: Hash) -> Result<(), Error> {
            let c = self.load(campaign_id)?;
            if self.env().caller() != c.owner {
                return Err(Error::Unauthorized);
            }
            if c.status != CampaignStatus::Active {
                return Err(Error::InvalidStatus);
            }
            if milestone_index != c.current_milestone {
                return Err(Error::WrongMilestone);
            }
            if self.env().block_number() > c.stage_start.saturating_add(c.submission_window_blocks) {
                return Err(Error::SubmissionWindowClosed);
            }
            let mut m = self.milestone(campaign_id, milestone_index);
            if m.status != MilestoneStatus::Pending {
                return Err(Error::InvalidStatus);
            }
            m.status = MilestoneStatus::Submitted;
            m.evidence_hash = Some(evidence_hash);
            self.milestone_states.insert((campaign_id, milestone_index), &m);
            self.env().emit_event(MilestoneSubmitted { campaign_id, milestone_index, evidence_hash });
            Ok(())
        }

        /// A backer approves the submitted milestone. Reaching quorum releases its tranche.
        #[ink(message)]
        pub fn approve_milestone(&mut self, campaign_id: u64, milestone_index: u32) -> Result<(), Error> {
            let mut c = self.load(campaign_id)?;
            let weight = self.begin_vote(&c, campaign_id, milestone_index)?;
            let mut m = self.milestone(campaign_id, milestone_index);
            m.approve_weight = m.approve_weight.saturating_add(weight);
            let quorum_met = m.approve_weight.saturating_mul(10_000) >= c.raised.saturating_mul(c.quorum_bps as u128);
            if quorum_met {
                m.status = MilestoneStatus::Approved;
            }
            self.milestone_states.insert((campaign_id, milestone_index), &m);
            if quorum_met {
                self.env().emit_event(MilestoneApproved { campaign_id, milestone_index });
                self.release(&mut c, campaign_id, milestone_index)?;
            }
            Ok(())
        }

        /// A backer rejects the submitted milestone. Once approval can no longer reach
        /// quorum the milestone is rejected and the campaign is disputed.
        #[ink(message)]
        pub fn reject_milestone(&mut self, campaign_id: u64, milestone_index: u32, reason_hash: Hash) -> Result<(), Error> {
            let mut c = self.load(campaign_id)?;
            let weight = self.begin_vote(&c, campaign_id, milestone_index)?;
            let mut m = self.milestone(campaign_id, milestone_index);
            m.reject_weight = m.reject_weight.saturating_add(weight);
            // Approval can no longer reach quorum once rejecters hold more than (100% - quorum).
            let blocked = m.reject_weight.saturating_mul(10_000) > c.raised.saturating_mul(10_000 - c.quorum_bps as u128);
            if blocked {
                m.status = MilestoneStatus::Rejected;
                c.status = CampaignStatus::Disputed;
                self.campaigns.insert(campaign_id, &c);
                self.env().emit_event(MilestoneRejected { campaign_id, milestone_index, reason_hash });
            }
            self.milestone_states.insert((campaign_id, milestone_index), &m);
            Ok(())
        }

        // ── Refunds ────────────────────────────────────────────────────

        /// A backer's refund (once): their share of the escrow not yet released.
        #[ink(message)]
        pub fn claim_refund(&mut self, campaign_id: u64) -> Result<u128, Error> {
            let c = self.load(campaign_id)?;
            if !self.refundable(&c, campaign_id) {
                return Err(Error::NotRefundable);
            }
            let backer = self.env().caller();
            if self.refunded.get((campaign_id, backer)).unwrap_or(false) {
                return Err(Error::NothingToRefund);
            }
            let contributed = self.contributions.get((campaign_id, backer)).unwrap_or(0);
            if contributed == 0 || c.raised == 0 {
                return Err(Error::NothingToRefund);
            }
            let remaining = c.raised - c.released;
            let amount = contributed.saturating_mul(remaining) / c.raised;
            if amount == 0 {
                return Err(Error::NothingToRefund);
            }
            self.refunded.insert((campaign_id, backer), &true);
            self.env().transfer(backer, amount).map_err(|_| Error::TransferFailed)?;
            self.env().emit_event(Refunded { campaign_id, backer, amount });
            Ok(amount)
        }

        // ── Queries ────────────────────────────────────────────────────

        #[ink(message)]
        pub fn get_campaign(&self, campaign_id: u64) -> Option<Campaign> {
            self.campaigns.get(campaign_id)
        }

        #[ink(message)]
        pub fn get_milestone(&self, campaign_id: u64, milestone_index: u32) -> Option<MilestoneState> {
            self.milestone_states.get((campaign_id, milestone_index))
        }

        #[ink(message)]
        pub fn get_contribution(&self, campaign_id: u64, backer: AccountId) -> u128 {
            self.contributions.get((campaign_id, backer)).unwrap_or(0)
        }

        #[ink(message)]
        pub fn is_refundable(&self, campaign_id: u64) -> bool {
            self.campaigns.get(campaign_id).map(|c| self.refundable(&c, campaign_id)).unwrap_or(false)
        }

        // ── Internals ──────────────────────────────────────────────────

        fn load(&self, id: u64) -> Result<Campaign, Error> {
            self.campaigns.get(id).ok_or(Error::CampaignNotFound)
        }

        fn milestone(&self, campaign_id: u64, index: u32) -> MilestoneState {
            self.milestone_states.get((campaign_id, index)).unwrap_or_else(MilestoneState::pending)
        }

        /// Shared checks for a backer's vote; returns their weight and records the vote.
        fn begin_vote(&mut self, c: &Campaign, campaign_id: u64, index: u32) -> Result<u128, Error> {
            if c.status != CampaignStatus::Active {
                return Err(Error::InvalidStatus);
            }
            if index != c.current_milestone {
                return Err(Error::WrongMilestone);
            }
            let m = self.milestone(campaign_id, index);
            if m.status != MilestoneStatus::Submitted {
                return Err(Error::InvalidStatus);
            }
            let voter = self.env().caller();
            let weight = self.contributions.get((campaign_id, voter)).unwrap_or(0);
            if weight == 0 {
                return Err(Error::NotABacker);
            }
            if self.voted.get((campaign_id, index, voter)).unwrap_or(false) {
                return Err(Error::AlreadyVoted);
            }
            self.voted.insert((campaign_id, index, voter), &true);
            Ok(weight)
        }

        fn release(&mut self, c: &mut Campaign, campaign_id: u64, index: u32) -> Result<(), Error> {
            let last = index as usize + 1 == c.milestones.len();
            let amount = if last {
                c.raised - c.released
            } else {
                c.raised.saturating_mul(c.milestones[index as usize].release_bps as u128) / 10_000
            };
            self.env().transfer(c.owner, amount).map_err(|_| Error::TransferFailed)?;
            c.released = c.released.saturating_add(amount);
            c.current_milestone = index + 1;
            c.stage_start = self.env().block_number();
            if last {
                c.status = CampaignStatus::Completed;
            }
            self.campaigns.insert(campaign_id, &*c);
            self.env().emit_event(FundsReleased { campaign_id, milestone_index: index, amount });
            Ok(())
        }

        fn refundable(&self, c: &Campaign, campaign_id: u64) -> bool {
            match c.status {
                CampaignStatus::Failed | CampaignStatus::Disputed => true,
                CampaignStatus::Active => {
                    let m = self.milestone(campaign_id, c.current_milestone);
                    m.status == MilestoneStatus::Pending
                        && self.env().block_number() > c.stage_start.saturating_add(c.submission_window_blocks)
                }
                _ => false,
            }
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use ink::env::test;

        type Accounts = test::DefaultAccounts<ink::env::DefaultEnvironment>;

        fn h(b: u8) -> Hash {
            Hash::from([b; 32])
        }

        fn block(n: u32) {
            test::set_block_number::<ink::env::DefaultEnvironment>(n);
        }

        fn as_user(who: AccountId) {
            test::set_caller::<ink::env::DefaultEnvironment>(who);
        }

        /// Alice owns campaign 1: goal 1_000, three milestones (30% / 30% / 40%),
        /// funding deadline block 500, 100-block submission window, 60% quorum.
        fn setup() -> (MilestoneCrowdfunding, Accounts, u64) {
            let a = test::default_accounts::<ink::env::DefaultEnvironment>();
            for who in [a.alice, a.bob, a.charlie, a.django, a.eve] {
                test::set_account_balance::<ink::env::DefaultEnvironment>(who, 1_000_000);
            }
            as_user(a.alice);
            block(100);
            let mut c = MilestoneCrowdfunding::new();
            let spec = |bps| MilestoneSpec { description_hash: h(bps as u8), release_bps: bps };
            let id = c
                .create_campaign_with_milestones(vec![spec(3_000), spec(3_000), spec(4_000)], 1_000, 500, 100, 6_000)
                .unwrap();
            (c, a, id)
        }

        fn back(c: &mut MilestoneCrowdfunding, who: AccountId, id: u64, amount: u128) {
            as_user(who);
            test::transfer_in::<ink::env::DefaultEnvironment>(amount);
            c.invest(id).unwrap();
        }

        /// Fully funded: bob 600 and charlie 400 (weights 60% / 40%).
        fn funded() -> (MilestoneCrowdfunding, Accounts, u64) {
            let (mut c, a, id) = setup();
            back(&mut c, a.bob, id, 600);
            back(&mut c, a.charlie, id, 400);
            (c, a, id)
        }

        fn balance(who: AccountId) -> u128 {
            test::get_account_balance::<ink::env::DefaultEnvironment>(who).unwrap()
        }

        // ── Creation ────────────────────────────────────────────────────

        #[ink::test]
        fn a_campaign_is_created_with_its_milestones_in_funding_state() {
            let (c, a, id) = setup();
            let camp = c.get_campaign(id).unwrap();
            assert_eq!((camp.owner, camp.status, camp.raised, camp.funding_goal), (a.alice, CampaignStatus::Funding, 0, 1_000));
            assert_eq!(camp.milestones.len(), 3);
            assert_eq!(c.get_milestone(id, 2).unwrap().status, MilestoneStatus::Pending);
            assert!(c.get_milestone(id, 3).is_none());
        }

        #[ink::test]
        fn invalid_campaign_parameters_are_rejected() {
            let (mut c, _, _) = setup();
            let spec = |bps| MilestoneSpec { description_hash: h(1), release_bps: bps };
            let ok = vec![spec(5_000), spec(5_000)];
            assert_eq!(c.create_campaign_with_milestones(vec![], 1, 500, 10, 5_000), Err(Error::InvalidParameters));
            assert_eq!(c.create_campaign_with_milestones(vec![spec(5_000), spec(4_999)], 1, 500, 10, 5_000), Err(Error::InvalidParameters));
            assert_eq!(c.create_campaign_with_milestones(vec![spec(10_000), spec(0)], 1, 500, 10, 5_000), Err(Error::InvalidParameters));
            assert_eq!(c.create_campaign_with_milestones(ok.clone(), 0, 500, 10, 5_000), Err(Error::InvalidParameters));
            assert_eq!(c.create_campaign_with_milestones(ok.clone(), 1, 100, 10, 5_000), Err(Error::InvalidParameters)); // deadline = now
            assert_eq!(c.create_campaign_with_milestones(ok.clone(), 1, 500, 0, 5_000), Err(Error::InvalidParameters));
            assert_eq!(c.create_campaign_with_milestones(ok.clone(), 1, 500, 10, 0), Err(Error::InvalidParameters));
            assert_eq!(c.create_campaign_with_milestones(ok, 1, 500, 10, 10_001), Err(Error::InvalidParameters));
        }

        // ── Funding ─────────────────────────────────────────────────────

        #[ink::test]
        fn reaching_the_goal_activates_the_campaign() {
            let (mut c, a, id) = setup();
            back(&mut c, a.bob, id, 600);
            assert_eq!(c.get_campaign(id).unwrap().status, CampaignStatus::Funding);
            block(150);
            back(&mut c, a.charlie, id, 400);
            let camp = c.get_campaign(id).unwrap();
            assert_eq!((camp.status, camp.raised, camp.stage_start), (CampaignStatus::Active, 1_000, 150));
            assert_eq!(c.get_contribution(id, a.bob), 600);
        }

        #[ink::test]
        fn investing_is_refused_after_the_deadline_with_zero_value_or_once_active() {
            let (mut c, a, id) = setup();
            as_user(a.bob);
            assert_eq!(c.invest(id), Err(Error::ZeroAmount));
            block(501);
            test::transfer_in::<ink::env::DefaultEnvironment>(10);
            assert_eq!(c.invest(id), Err(Error::DeadlinePassed));

            let (mut c2, a2, id2) = funded();
            as_user(a2.eve);
            test::transfer_in::<ink::env::DefaultEnvironment>(10);
            assert_eq!(c2.invest(id2), Err(Error::InvalidStatus));
            assert_eq!(c2.invest(99), Err(Error::CampaignNotFound));
        }

        #[ink::test]
        fn a_campaign_that_misses_its_goal_fails_after_the_deadline_and_refunds_in_full() {
            let (mut c, a, id) = setup();
            back(&mut c, a.bob, id, 300);
            assert_eq!(c.finalize_funding(id), Err(Error::DeadlineNotPassed));
            block(501);
            c.finalize_funding(id).unwrap();
            assert_eq!(c.get_campaign(id).unwrap().status, CampaignStatus::Failed);
            assert!(c.is_refundable(id));
            as_user(a.bob);
            let before = balance(a.bob);
            assert_eq!(c.claim_refund(id), Ok(300));
            assert_eq!(balance(a.bob) - before, 300);
        }

        // ── Submitting evidence ─────────────────────────────────────────

        #[ink::test]
        fn only_the_owner_submits_the_current_milestone_within_the_window() {
            let (mut c, a, id) = funded();
            as_user(a.bob);
            assert_eq!(c.submit_milestone(id, 0, h(9)), Err(Error::Unauthorized));
            as_user(a.alice);
            assert_eq!(c.submit_milestone(id, 1, h(9)), Err(Error::WrongMilestone));
            c.submit_milestone(id, 0, h(9)).unwrap();
            let m = c.get_milestone(id, 0).unwrap();
            assert_eq!((m.status, m.evidence_hash), (MilestoneStatus::Submitted, Some(h(9))));
            assert_eq!(c.submit_milestone(id, 0, h(9)), Err(Error::InvalidStatus));
        }

        #[ink::test]
        fn the_submission_window_boundary_is_inclusive() {
            let (mut c, a, id) = funded();
            // Funding completed at block 100; the window is 100 blocks.
            block(200);
            as_user(a.alice);
            assert!(c.submit_milestone(id, 0, h(9)).is_ok());
            let (mut late, a2, id2) = funded();
            block(201);
            as_user(a2.alice);
            assert_eq!(late.submit_milestone(id2, 0, h(9)), Err(Error::SubmissionWindowClosed));
        }

        // ── Approval and proportional release ───────────────────────────

        #[ink::test]
        fn approval_at_quorum_releases_the_milestones_share_of_the_escrow() {
            let (mut c, a, id) = funded();
            as_user(a.alice);
            c.submit_milestone(id, 0, h(9)).unwrap();
            let owner_before = balance(a.alice);

            // Charlie (40%) is below the 60% quorum; nothing is released yet.
            as_user(a.charlie);
            c.approve_milestone(id, 0).unwrap();
            assert_eq!(c.get_milestone(id, 0).unwrap().status, MilestoneStatus::Submitted);
            assert_eq!(balance(a.alice), owner_before);

            // Bob (60%) pushes approval to 100%: 30% of 1_000 is released.
            as_user(a.bob);
            c.approve_milestone(id, 0).unwrap();
            assert_eq!(c.get_milestone(id, 0).unwrap().status, MilestoneStatus::Approved);
            assert_eq!(balance(a.alice) - owner_before, 300);
            let camp = c.get_campaign(id).unwrap();
            assert_eq!((camp.released, camp.current_milestone, camp.status), (300, 1, CampaignStatus::Active));
        }

        #[ink::test]
        fn a_single_backer_with_enough_weight_reaches_quorum_alone() {
            let (mut c, a, id) = funded();
            as_user(a.alice);
            c.submit_milestone(id, 0, h(9)).unwrap();
            as_user(a.bob); // exactly 60% = quorum
            c.approve_milestone(id, 0).unwrap();
            assert_eq!(c.get_campaign(id).unwrap().released, 300);
        }

        #[ink::test]
        fn all_milestones_release_the_whole_escrow_and_complete_the_campaign() {
            let (mut c, a, id) = funded();
            let owner_before = balance(a.alice);
            for i in 0..3u32 {
                as_user(a.alice);
                c.submit_milestone(id, i, h(9)).unwrap();
                as_user(a.bob);
                c.approve_milestone(id, i).unwrap();
            }
            let camp = c.get_campaign(id).unwrap();
            assert_eq!((camp.released, camp.status), (1_000, CampaignStatus::Completed));
            assert_eq!(balance(a.alice) - owner_before, 1_000);
            // 30% + 30% + the remaining 40%.
            as_user(a.alice);
            assert_eq!(c.submit_milestone(id, 2, h(9)), Err(Error::InvalidStatus));
        }

        #[ink::test]
        fn the_last_tranche_takes_any_rounding_remainder() {
            let (mut c, a, _) = setup();
            let spec = |bps| MilestoneSpec { description_hash: h(1), release_bps: bps };
            as_user(a.alice);
            let id = c.create_campaign_with_milestones(vec![spec(3_333), spec(3_333), spec(3_334)], 100, 500, 100, 5_000).unwrap();
            back(&mut c, a.bob, id, 100);
            for i in 0..3u32 {
                as_user(a.alice);
                c.submit_milestone(id, i, h(9)).unwrap();
                as_user(a.bob);
                c.approve_milestone(id, i).unwrap();
            }
            assert_eq!(c.get_campaign(id).unwrap().released, 100);
        }

        #[ink::test]
        fn voting_rules_are_enforced() {
            let (mut c, a, id) = funded();
            as_user(a.alice);
            c.submit_milestone(id, 0, h(9)).unwrap();
            as_user(a.eve); // not a backer
            assert_eq!(c.approve_milestone(id, 0), Err(Error::NotABacker));
            assert_eq!(c.reject_milestone(id, 0, h(1)), Err(Error::NotABacker));
            as_user(a.charlie);
            c.approve_milestone(id, 0).unwrap();
            assert_eq!(c.approve_milestone(id, 0), Err(Error::AlreadyVoted));
            assert_eq!(c.reject_milestone(id, 0, h(1)), Err(Error::AlreadyVoted));
            assert_eq!(c.approve_milestone(id, 1), Err(Error::WrongMilestone));
        }

        #[ink::test]
        fn votes_need_a_submitted_milestone() {
            let (mut c, a, id) = funded();
            as_user(a.bob);
            assert_eq!(c.approve_milestone(id, 0), Err(Error::InvalidStatus));
        }

        // ── Rejection and dispute ───────────────────────────────────────

        #[ink::test]
        fn rejection_beyond_the_blocking_threshold_disputes_the_campaign() {
            let (mut c, a, id) = funded();
            as_user(a.alice);
            c.submit_milestone(id, 0, h(9)).unwrap();
            // Quorum is 60%, so rejecters need strictly more than 40% to block it.
            as_user(a.charlie); // exactly 40%: not enough
            c.reject_milestone(id, 0, h(7)).unwrap();
            assert_eq!(c.get_campaign(id).unwrap().status, CampaignStatus::Active);
            as_user(a.bob); // pushes the rejecting weight to 100%
            c.reject_milestone(id, 0, h(7)).unwrap();
            assert_eq!(c.get_milestone(id, 0).unwrap().status, MilestoneStatus::Rejected);
            assert_eq!(c.get_campaign(id).unwrap().status, CampaignStatus::Disputed);
            assert!(c.is_refundable(id));
        }

        #[ink::test]
        fn a_disputed_campaign_refunds_only_the_unreleased_escrow() {
            let (mut c, a, id) = funded();
            // Milestone 0 is approved (300 released), milestone 1 is rejected.
            as_user(a.alice);
            c.submit_milestone(id, 0, h(9)).unwrap();
            as_user(a.bob);
            c.approve_milestone(id, 0).unwrap();
            as_user(a.alice);
            c.submit_milestone(id, 1, h(9)).unwrap();
            as_user(a.bob);
            c.reject_milestone(id, 1, h(7)).unwrap();
            as_user(a.charlie);
            c.reject_milestone(id, 1, h(7)).unwrap();
            assert_eq!(c.get_campaign(id).unwrap().status, CampaignStatus::Disputed);

            // 700 remains: bob (60%) gets 420, charlie (40%) gets 280.
            as_user(a.bob);
            assert_eq!(c.claim_refund(id), Ok(420));
            as_user(a.charlie);
            assert_eq!(c.claim_refund(id), Ok(280));
        }

        // ── Timeout refunds ─────────────────────────────────────────────

        #[ink::test]
        fn missing_the_submission_window_opens_refunds() {
            let (mut c, a, id) = funded();
            assert!(!c.is_refundable(id));
            as_user(a.bob);
            assert_eq!(c.claim_refund(id), Err(Error::NotRefundable));
            block(201); // stage started at 100 with a 100-block window
            assert!(c.is_refundable(id));
            assert_eq!(c.claim_refund(id), Ok(600));
        }

        #[ink::test]
        fn a_submitted_milestone_keeps_refunds_closed_even_after_the_window() {
            let (mut c, a, id) = funded();
            as_user(a.alice);
            c.submit_milestone(id, 0, h(9)).unwrap();
            block(10_000);
            assert!(!c.is_refundable(id));
        }

        #[ink::test]
        fn the_timeout_applies_per_stage_after_a_release() {
            let (mut c, a, id) = funded();
            as_user(a.alice);
            c.submit_milestone(id, 0, h(9)).unwrap();
            as_user(a.bob);
            c.approve_milestone(id, 0).unwrap(); // released at block 100, next stage starts there
            block(200);
            assert!(!c.is_refundable(id));
            block(201);
            assert!(c.is_refundable(id));
            // Refund is the unreleased 70%.
            as_user(a.charlie);
            assert_eq!(c.claim_refund(id), Ok(280));
        }

        #[ink::test]
        fn refunds_are_paid_once_and_only_to_backers() {
            let (mut c, a, id) = funded();
            block(201);
            as_user(a.eve);
            assert_eq!(c.claim_refund(id), Err(Error::NothingToRefund));
            as_user(a.bob);
            assert!(c.claim_refund(id).is_ok());
            assert_eq!(c.claim_refund(id), Err(Error::NothingToRefund));
            assert_eq!(c.claim_refund(99), Err(Error::CampaignNotFound));
        }

        #[ink::test]
        fn a_completed_campaign_is_not_refundable() {
            let (mut c, a, id) = funded();
            for i in 0..3u32 {
                as_user(a.alice);
                c.submit_milestone(id, i, h(9)).unwrap();
                as_user(a.bob);
                c.approve_milestone(id, i).unwrap();
            }
            block(10_000);
            assert!(!c.is_refundable(id));
        }

        // ── Events ──────────────────────────────────────────────────────

        #[ink::test]
        fn submission_approval_and_release_emit_events() {
            let (mut c, a, id) = funded();
            let before = test::recorded_events().count();
            as_user(a.alice);
            c.submit_milestone(id, 0, h(9)).unwrap(); // MilestoneSubmitted
            as_user(a.bob);
            c.approve_milestone(id, 0).unwrap(); // MilestoneApproved + FundsReleased
            assert_eq!(test::recorded_events().count() - before, 3);
        }
    }
}
