#![cfg_attr(not(feature = "std"), no_std, no_main)]
#![allow(
    clippy::arithmetic_side_effects,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::needless_borrows_for_generic_args
)]

use ink::storage::Mapping;
use propchain_traits::*;
use propchain_traits::{non_reentrant, ReentrancyError, ReentrancyGuard};

#[ink::contract]
mod propchain_crowdfunding {
    use super::*;
    use ink::prelude::{string::String, vec::Vec};

    #[derive(Debug, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub enum CrowdfundingError {
        Unauthorized,
        CampaignNotFound,
        CampaignNotActive,
        InsufficientFunds,
        MilestoneNotFound,
        MilestoneNotApproved,
        InvestorNotCompliant,
        InsufficientShares,
        ListingNotFound,
        ProposalNotFound,
        ProposalNotActive,
        AlreadyVoted,
        ReentrantCall,

        OracleVerificationFailed,
        CampaignNotFailed,
        AlreadyRefunded,
        NoInvestmentFound,
        AccreditationNotVerified,
        InvalidParameters,
    }

    impl From<propchain_traits::ReentrancyError> for CrowdfundingError {
        fn from(_: propchain_traits::ReentrancyError) -> Self {
            CrowdfundingError::ReentrantCall
        }
    }

    #[derive(
        Debug,
        Clone,
        Copy,
        PartialEq,
        Eq,
        scale::Encode,
        scale::Decode,
        ink::storage::traits::StorageLayout,
    )]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub enum CampaignStatus {
        Draft,
        Active,
        Funded,
        Closed,
        Cancelled,
    }

    #[derive(
        Debug,
        Clone,
        Copy,
        PartialEq,
        Eq,
        scale::Encode,
        scale::Decode,
        ink::storage::traits::StorageLayout,
    )]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub enum ComplianceStatus {
        Pending,
        Approved,
        Rejected,
    }

    #[derive(
        Debug,
        Clone,
        Copy,
        PartialEq,
        Eq,
        scale::Encode,
        scale::Decode,
        ink::storage::traits::StorageLayout,
    )]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub enum MilestoneStatus {
        Pending,
        Approved,
        Released,
    }

    #[derive(
        Debug,
        Clone,
        Copy,
        PartialEq,
        Eq,
        scale::Encode,
        scale::Decode,
        ink::storage::traits::StorageLayout,
    )]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub enum ProposalStatus {
        Active,
        Passed,
        Rejected,
    }

    #[derive(
        Debug,
        Clone,
        Copy,
        PartialEq,
        Eq,
        scale::Encode,
        scale::Decode,
        ink::storage::traits::StorageLayout,
    )]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub enum RiskRating {
        Low,
        Medium,
        High,
        Unrated,
    }

    #[derive(
        Debug, Clone, PartialEq, scale::Encode, scale::Decode, ink::storage::traits::StorageLayout,
    )]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub struct Campaign {
        pub campaign_id: u64,
        pub creator: AccountId,
        pub title: String,
        pub target_amount: u128,
        pub raised_amount: u128,
        pub status: CampaignStatus,
        pub investor_count: u32,
    }

    #[derive(
        Debug, Clone, PartialEq, scale::Encode, scale::Decode, ink::storage::traits::StorageLayout,
    )]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub struct InvestorProfile {
        pub investor: AccountId,
        pub kyc_status: ComplianceStatus,
        pub accredited: bool,
        pub jurisdiction: String,
    }

    #[derive(
        Debug, Clone, PartialEq, scale::Encode, scale::Decode, ink::storage::traits::StorageLayout,
    )]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub struct Milestone {
        pub milestone_id: u64,
        pub campaign_id: u64,
        pub description: String,
        pub release_amount: u128,
        pub status: MilestoneStatus,
        pub oracle_verified: bool,
        pub oracle_data_hash: Option<[u8; 32]>,
    }

    #[derive(
        Debug, Clone, PartialEq, scale::Encode, scale::Decode, ink::storage::traits::StorageLayout,
    )]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub struct Proposal {
        pub proposal_id: u64,
        pub campaign_id: u64,
        pub description: String,
        pub votes_for: u64,
        pub votes_against: u64,
        pub status: ProposalStatus,
    }

    #[derive(
        Debug, Clone, PartialEq, scale::Encode, scale::Decode, ink::storage::traits::StorageLayout,
    )]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub struct ShareListing {
        pub listing_id: u64,
        pub seller: AccountId,
        pub campaign_id: u64,
        pub shares: u64,
        pub price_per_share: u128,
    }

    #[derive(
        Debug, Clone, PartialEq, scale::Encode, scale::Decode, ink::storage::traits::StorageLayout,
    )]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub struct RiskProfile {
        pub campaign_id: u64,
        pub ltv_ratio: u32,
        pub developer_score: u32,
        pub market_volatility: u32,
        pub rating: RiskRating,
    }

    #[derive(
        Debug, Clone, PartialEq, scale::Encode, scale::Decode, ink::storage::traits::StorageLayout,
    )]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub struct CampaignSuccessMetrics {
        pub campaign_id: u64,
        pub funding_progress_bps: u32,
        pub investor_count: u32,
        pub average_investment: u128,
        pub total_milestones: u32,
        pub released_milestones: u32,
        pub released_capital: u128,
        pub is_funded: bool,
    }

    #[derive(
        Debug, Clone, PartialEq, scale::Encode, scale::Decode, ink::storage::traits::StorageLayout,
    )]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub struct CampaignSummary {
        pub campaign_id: u64,
        pub creator: AccountId,
        pub title: String,
        pub target_amount: u128,
        pub raised_amount: u128,
        pub funded_pct: u32,
        pub status: CampaignStatus,
        pub investor_count: u32,
        pub risk_rating: RiskRating,
    }

    #[derive(
        Debug, Clone, PartialEq, scale::Encode, scale::Decode, ink::storage::traits::StorageLayout,
    )]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub struct CampaignFilter {
        pub status: Option<CampaignStatus>,
        pub title_keyword: Option<String>,
        pub min_target: Option<u128>,
        pub max_target: Option<u128>,
        pub min_funded_pct: Option<u32>,
        pub funded_only: bool,
    }

    #[derive(
        Debug, Clone, PartialEq, scale::Encode, scale::Decode, ink::storage::traits::StorageLayout,
    )]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub struct CampaignAnalytics {
        pub campaign_id: u64,
        pub total_investors: u32,
        pub total_investment: u128,
        pub funding_progress: u32, // in basis points (0-10000)
        pub average_investment: u128,
        pub largest_investment: u128,
        pub milestone_completion_rate: u32, // in basis points
        pub days_active: u32,
        pub funding_velocity: u128,       // tokens per day
        pub investor_retention_rate: u32, // in basis points
        pub risk_score: u32,
        pub projected_completion_days: u32,
    }

    #[derive(
        Debug, Clone, PartialEq, scale::Encode, scale::Decode, ink::storage::traits::StorageLayout,
    )]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub struct InvestorDemographics {
        pub total_investors: u32,
        pub accredited_investors: u32,
        pub average_investment: u128,
        pub top_investor_amount: u128,
        pub jurisdictions: Vec<(String, u32)>, // (jurisdiction, count)
        pub investment_distribution: Vec<(u128, u32)>, // (investment_range, count)
    }

    #[ink(storage)]
    pub struct RealEstateCrowdfunding {
        admin: AccountId,
        campaigns: Mapping<u64, Campaign>,
        campaign_count: u64,
        campaign_ids: Vec<u64>, // index for iteration
        investor_profiles: Mapping<AccountId, InvestorProfile>,
        investments: Mapping<(u64, AccountId), u128>,
        milestones: Mapping<u64, Milestone>,
        milestone_count: u64,
        proposals: Mapping<u64, Proposal>,
        proposal_count: u64,
        voting_weights: Mapping<(u64, AccountId), u64>,
        votes_cast: Mapping<(u64, AccountId), bool>,
        share_holdings: Mapping<(u64, AccountId), u64>,
        listings: Mapping<u64, ShareListing>,
        listing_count: u64,
        risk_profiles: Mapping<u64, RiskProfile>,
        campaign_milestone_counts: Mapping<u64, u32>,
        released_milestone_counts: Mapping<u64, u32>,
        released_capital: Mapping<u64, u128>,
        blocked_jurisdictions: Vec<String>,
        reentrancy_guard: propchain_traits::ReentrancyGuard,
        /// Authorized oracle accounts for milestone verification
        authorized_oracles: Mapping<AccountId, bool>,
        /// Tracks whether an investor has been refunded for a campaign
        refunds_issued: Mapping<(u64, AccountId), bool>,
    }

    // ── Events ───────────────────────────────────────────────

    #[ink(event)]
    pub struct CampaignCreated {
        #[ink(topic)]
        campaign_id: u64,
        #[ink(topic)]
        creator: AccountId,
        target_amount: u128,
    }

    #[ink(event)]
    pub struct InvestmentMade {
        #[ink(topic)]
        campaign_id: u64,
        #[ink(topic)]
        investor: AccountId,
        amount: u128,
    }

    #[ink(event)]
    pub struct MilestoneApproved {
        #[ink(topic)]
        milestone_id: u64,
        release_amount: u128,
    }

    #[ink(event)]
    pub struct ProposalCreated {
        #[ink(topic)]
        proposal_id: u64,
        #[ink(topic)]
        campaign_id: u64,
    }

    #[ink(event)]
    pub struct SharesListed {
        #[ink(topic)]
        listing_id: u64,
        #[ink(topic)]
        seller: AccountId,
        shares: u64,
    }

    #[ink(event)]
    pub struct MilestoneOracleVerified {
        #[ink(topic)]
        milestone_id: u64,
        #[ink(topic)]
        oracle: AccountId,
        data_hash: [u8; 32],
    }

    #[ink(event)]
    pub struct RefundIssued {
        #[ink(topic)]
        campaign_id: u64,
        #[ink(topic)]
        investor: AccountId,
        amount: u128,
    }

    #[ink(event)]
    pub struct AccreditationVerified {
        #[ink(topic)]
        investor: AccountId,
        verified_by: AccountId,
    }

    #[ink(event)]
    pub struct CampaignShared {
        #[ink(topic)]
        campaign_id: u64,
        #[ink(topic)]
        sharer: AccountId,
        platform: String,
    }

    impl RealEstateCrowdfunding {
        #[ink(constructor)]
        pub fn new(admin: AccountId) -> Self {
            Self {
                admin,
                campaigns: Mapping::default(),
                campaign_count: 0,
                campaign_ids: Vec::new(),
                investor_profiles: Mapping::default(),
                investments: Mapping::default(),
                milestones: Mapping::default(),
                milestone_count: 0,
                proposals: Mapping::default(),
                proposal_count: 0,
                voting_weights: Mapping::default(),
                votes_cast: Mapping::default(),
                share_holdings: Mapping::default(),
                listings: Mapping::default(),
                listing_count: 0,
                risk_profiles: Mapping::default(),
                campaign_milestone_counts: Mapping::default(),
                released_milestone_counts: Mapping::default(),
                released_capital: Mapping::default(),
                blocked_jurisdictions: Vec::new(),
                reentrancy_guard: propchain_traits::ReentrancyGuard::new(),
                authorized_oracles: Mapping::default(),
                refunds_issued: Mapping::default(),
            }
        }

        // ── Core Campaign Messages ───────────────────────────

        #[ink(message)]
        pub fn create_campaign(
            &mut self,
            title: String,
            target_amount: u128,
        ) -> Result<u64, CrowdfundingError> {
            self.campaign_count += 1;
            let campaign = Campaign {
                campaign_id: self.campaign_count,
                creator: self.env().caller(),
                title,
                target_amount,
                raised_amount: 0,
                status: CampaignStatus::Draft,
                investor_count: 0,
            };
            self.campaigns.insert(self.campaign_count, &campaign);
            self.campaign_ids.push(self.campaign_count);
            self.env().emit_event(CampaignCreated {
                campaign_id: self.campaign_count,
                creator: self.env().caller(),
                target_amount,
            });
            Ok(self.campaign_count)
        }

        #[ink(message)]
        pub fn activate_campaign(&mut self, campaign_id: u64) -> Result<(), CrowdfundingError> {
            let mut campaign = self
                .campaigns
                .get(campaign_id)
                .ok_or(CrowdfundingError::CampaignNotFound)?;
            if self.env().caller() != campaign.creator && self.env().caller() != self.admin {
                return Err(CrowdfundingError::Unauthorized);
            }
            campaign.status = CampaignStatus::Active;
            self.campaigns.insert(campaign_id, &campaign);
            Ok(())
        }

        #[ink(message)]
        pub fn onboard_investor(
            &mut self,
            jurisdiction: String,
            accredited: bool,
        ) -> Result<(), CrowdfundingError> {
            let caller = self.env().caller();
            let profile = InvestorProfile {
                investor: caller,
                kyc_status: ComplianceStatus::Approved,
                accredited,
                jurisdiction,
            };
            self.investor_profiles.insert(caller, &profile);
            Ok(())
        }

        /// Admin-only: verify an investor's accreditation status
        #[ink(message)]
        pub fn verify_accreditation(
            &mut self,
            investor: AccountId,
        ) -> Result<(), CrowdfundingError> {
            if self.env().caller() != self.admin {
                return Err(CrowdfundingError::Unauthorized);
            }
            let mut profile = self
                .investor_profiles
                .get(investor)
                .ok_or(CrowdfundingError::InvestorNotCompliant)?;
            profile.accredited = true;
            self.investor_profiles.insert(investor, &profile);
            self.env().emit_event(AccreditationVerified {
                investor,
                verified_by: self.env().caller(),
            });
            Ok(())
        }

        /// Query whether an investor is accredited
        #[ink(message)]
        pub fn is_accredited(&self, investor: AccountId) -> bool {
            self.investor_profiles
                .get(investor)
                .map(|p| p.accredited)
                .unwrap_or(false)
        }

        #[ink(message)]
        pub fn invest(&mut self, campaign_id: u64, amount: u128) -> Result<(), CrowdfundingError> {
            let caller = self.env().caller();
            let profile = self
                .investor_profiles
                .get(caller)
                .ok_or(CrowdfundingError::InvestorNotCompliant)?;
            if profile.kyc_status != ComplianceStatus::Approved {
                return Err(CrowdfundingError::InvestorNotCompliant);
            }
            if !profile.accredited {
                return Err(CrowdfundingError::AccreditationNotVerified);
            }
            if self.blocked_jurisdictions.contains(&profile.jurisdiction) {
                return Err(CrowdfundingError::InvestorNotCompliant);
            }
            let mut campaign = self
                .campaigns
                .get(campaign_id)
                .ok_or(CrowdfundingError::CampaignNotFound)?;
            if campaign.status != CampaignStatus::Active {
                return Err(CrowdfundingError::CampaignNotActive);
            }
            let current = self.investments.get((campaign_id, caller)).unwrap_or(0);
            if current == 0 {
                campaign.investor_count += 1;
            }
            self.investments
                .insert((campaign_id, caller), &(current + amount));
            campaign.raised_amount += amount;
            if campaign.raised_amount >= campaign.target_amount {
                campaign.status = CampaignStatus::Funded;
            }
            self.campaigns.insert(campaign_id, &campaign);
            let shares = (amount / 1000) as u64;
            let current_shares = self.share_holdings.get((campaign_id, caller)).unwrap_or(0);
            self.share_holdings
                .insert((campaign_id, caller), &(current_shares + shares));
            self.env().emit_event(InvestmentMade {
                campaign_id,
                investor: caller,
                amount,
            });
            Ok(())
        }

        #[ink(message)]
        pub fn add_milestone(
            &mut self,
            campaign_id: u64,
            description: String,
            release_amount: u128,
        ) -> Result<u64, CrowdfundingError> {
            let campaign = self
                .campaigns
                .get(campaign_id)
                .ok_or(CrowdfundingError::CampaignNotFound)?;
            if self.env().caller() != campaign.creator && self.env().caller() != self.admin {
                return Err(CrowdfundingError::Unauthorized);
            }
            self.milestone_count += 1;
            let milestone = Milestone {
                milestone_id: self.milestone_count,
                campaign_id,
                description,
                release_amount,
                status: MilestoneStatus::Pending,
                oracle_verified: false,
                oracle_data_hash: None,
            };
            self.milestones.insert(self.milestone_count, &milestone);
            let total_milestones = self.campaign_milestone_counts.get(campaign_id).unwrap_or(0) + 1;
            self.campaign_milestone_counts
                .insert(campaign_id, &total_milestones);
            Ok(self.milestone_count)
        }

        #[ink(message)]
        pub fn approve_milestone(&mut self, milestone_id: u64) -> Result<(), CrowdfundingError> {
            if self.env().caller() != self.admin {
                return Err(CrowdfundingError::Unauthorized);
            }
            let mut milestone = self
                .milestones
                .get(milestone_id)
                .ok_or(CrowdfundingError::MilestoneNotFound)?;
            milestone.status = MilestoneStatus::Approved;
            self.milestones.insert(milestone_id, &milestone);
            self.env().emit_event(MilestoneApproved {
                milestone_id,
                release_amount: milestone.release_amount,
            });
            Ok(())
        }

        #[ink(message)]
        pub fn release_milestone(&mut self, milestone_id: u64) -> Result<(), CrowdfundingError> {
            propchain_traits::non_reentrant!(self, {
                let mut milestone = self
                    .milestones
                    .get(milestone_id)
                    .ok_or(CrowdfundingError::MilestoneNotFound)?;
                if milestone.status != MilestoneStatus::Approved {
                    return Err(CrowdfundingError::MilestoneNotApproved);
                }
                if !milestone.oracle_verified {
                    return Err(CrowdfundingError::OracleVerificationFailed);
                }
                milestone.status = MilestoneStatus::Released;
                self.milestones.insert(milestone_id, &milestone);
                let released_count = self
                    .released_milestone_counts
                    .get(milestone.campaign_id)
                    .unwrap_or(0)
                    + 1;
                self.released_milestone_counts
                    .insert(milestone.campaign_id, &released_count);
                let released_capital = self
                    .released_capital
                    .get(milestone.campaign_id)
                    .unwrap_or(0)
                    + milestone.release_amount;
                self.released_capital
                    .insert(milestone.campaign_id, &released_capital);
                Ok(())
            })
        }

        /// Oracle submits verification for a milestone (oracle only)
        #[ink(message)]
        pub fn oracle_verify_milestone(
            &mut self,
            milestone_id: u64,
            data_hash: [u8; 32],
        ) -> Result<(), CrowdfundingError> {
            let caller = self.env().caller();
            if !self.authorized_oracles.get(caller).unwrap_or(false) && caller != self.admin {
                return Err(CrowdfundingError::Unauthorized);
            }
            let mut milestone = self
                .milestones
                .get(milestone_id)
                .ok_or(CrowdfundingError::MilestoneNotFound)?;
            milestone.oracle_verified = true;
            milestone.oracle_data_hash = Some(data_hash);
            self.milestones.insert(milestone_id, &milestone);
            self.env().emit_event(MilestoneOracleVerified {
                milestone_id,
                oracle: caller,
                data_hash,
            });
            Ok(())
        }

        /// Admin: authorize an oracle account
        #[ink(message)]
        pub fn add_oracle(&mut self, oracle: AccountId) -> Result<(), CrowdfundingError> {
            if self.env().caller() != self.admin {
                return Err(CrowdfundingError::Unauthorized);
            }
            self.authorized_oracles.insert(oracle, &true);
            Ok(())
        }

        /// Mark a campaign as failed/cancelled and enable refunds (admin only)
        #[ink(message)]
        pub fn fail_campaign(&mut self, campaign_id: u64) -> Result<(), CrowdfundingError> {
            if self.env().caller() != self.admin {
                return Err(CrowdfundingError::Unauthorized);
            }
            let mut campaign = self
                .campaigns
                .get(campaign_id)
                .ok_or(CrowdfundingError::CampaignNotFound)?;
            campaign.status = CampaignStatus::Cancelled;
            self.campaigns.insert(campaign_id, &campaign);
            Ok(())
        }

        /// Investor claims a refund for a failed/cancelled campaign
        #[ink(message)]
        pub fn claim_refund(&mut self, campaign_id: u64) -> Result<u128, CrowdfundingError> {
            propchain_traits::non_reentrant!(self, {
                let caller = self.env().caller();
                let campaign = self
                    .campaigns
                    .get(campaign_id)
                    .ok_or(CrowdfundingError::CampaignNotFound)?;
                if campaign.status != CampaignStatus::Cancelled {
                    return Err(CrowdfundingError::CampaignNotFailed);
                }
                if self
                    .refunds_issued
                    .get((campaign_id, caller))
                    .unwrap_or(false)
                {
                    return Err(CrowdfundingError::AlreadyRefunded);
                }
                let amount = self
                    .investments
                    .get((campaign_id, caller))
                    .ok_or(CrowdfundingError::NoInvestmentFound)?;
                if amount == 0 {
                    return Err(CrowdfundingError::NoInvestmentFound);
                }
                self.refunds_issued.insert((campaign_id, caller), &true);
                self.env().emit_event(RefundIssued {
                    campaign_id,
                    investor: caller,
                    amount,
                });
                Ok(amount)
            })
        }

        /// Check if an investor has been refunded for a campaign
        #[ink(message)]
        pub fn is_refunded(&self, campaign_id: u64, investor: AccountId) -> bool {
            self.refunds_issued
                .get((campaign_id, investor))
                .unwrap_or(false)
        }

        #[ink(message)]
        pub fn distribute_profit(
            &self,
            campaign_id: u64,
            total_profit: u128,
            investor: AccountId,
        ) -> u128 {
            let campaign = self.campaigns.get(campaign_id).unwrap_or(Campaign {
                campaign_id: 0,
                creator: AccountId::from([0x0; 32]),
                title: String::new(),
                target_amount: 0,
                raised_amount: 1,
                status: CampaignStatus::Draft,
                investor_count: 0,
            });
            let investment = self.investments.get((campaign_id, investor)).unwrap_or(0);
            if campaign.target_amount == 0 {
                return 0;
            }
            (total_profit * investment) / campaign.target_amount
        }

        #[ink(message)]
        pub fn create_proposal(
            &mut self,
            campaign_id: u64,
            description: String,
        ) -> Result<u64, CrowdfundingError> {
            self.campaigns
                .get(campaign_id)
                .ok_or(CrowdfundingError::CampaignNotFound)?;
            self.proposal_count += 1;
            let proposal = Proposal {
                proposal_id: self.proposal_count,
                campaign_id,
                description,
                votes_for: 0,
                votes_against: 0,
                status: ProposalStatus::Active,
            };
            self.proposals.insert(self.proposal_count, &proposal);
            self.env().emit_event(ProposalCreated {
                proposal_id: self.proposal_count,
                campaign_id,
            });
            Ok(self.proposal_count)
        }

        #[ink(message)]
        pub fn vote(&mut self, proposal_id: u64, in_favour: bool) -> Result<(), CrowdfundingError> {
            let caller = self.env().caller();
            if self.votes_cast.get((proposal_id, caller)).unwrap_or(false) {
                return Err(CrowdfundingError::AlreadyVoted);
            }
            let mut proposal = self
                .proposals
                .get(proposal_id)
                .ok_or(CrowdfundingError::ProposalNotFound)?;
            if proposal.status != ProposalStatus::Active {
                return Err(CrowdfundingError::ProposalNotActive);
            }
            let weight = self
                .voting_weights
                .get((proposal.campaign_id, caller))
                .unwrap_or(1);
            if in_favour {
                proposal.votes_for += weight;
            } else {
                proposal.votes_against += weight;
            }
            self.proposals.insert(proposal_id, &proposal);
            self.votes_cast.insert((proposal_id, caller), &true);
            Ok(())
        }

        #[ink(message)]
        pub fn finalize_proposal(
            &mut self,
            proposal_id: u64,
        ) -> Result<ProposalStatus, CrowdfundingError> {
            let mut proposal = self
                .proposals
                .get(proposal_id)
                .ok_or(CrowdfundingError::ProposalNotFound)?;
            proposal.status = if proposal.votes_for > proposal.votes_against {
                ProposalStatus::Passed
            } else {
                ProposalStatus::Rejected
            };
            self.proposals.insert(proposal_id, &proposal);
            Ok(proposal.status)
        }

        #[ink(message)]
        pub fn list_shares(
            &mut self,
            campaign_id: u64,
            shares: u64,
            price_per_share: u128,
        ) -> Result<u64, CrowdfundingError> {
            let caller = self.env().caller();
            let held = self.share_holdings.get((campaign_id, caller)).unwrap_or(0);
            if held < shares {
                return Err(CrowdfundingError::InsufficientShares);
            }
            self.listing_count += 1;
            let listing = ShareListing {
                listing_id: self.listing_count,
                seller: caller,
                campaign_id,
                shares,
                price_per_share,
            };
            self.listings.insert(self.listing_count, &listing);
            self.env().emit_event(SharesListed {
                listing_id: self.listing_count,
                seller: caller,
                shares,
            });
            Ok(self.listing_count)
        }

        #[ink(message)]
        pub fn buy_shares(&mut self, listing_id: u64) -> Result<u128, CrowdfundingError> {
            let listing = self
                .listings
                .get(listing_id)
                .ok_or(CrowdfundingError::ListingNotFound)?;
            let total_cost = listing.price_per_share * listing.shares as u128;
            let seller_shares = self
                .share_holdings
                .get((listing.campaign_id, listing.seller))
                .unwrap_or(0);
            self.share_holdings.insert(
                (listing.campaign_id, listing.seller),
                &seller_shares.saturating_sub(listing.shares),
            );
            let buyer = self.env().caller();
            let buyer_shares = self
                .share_holdings
                .get((listing.campaign_id, buyer))
                .unwrap_or(0);
            self.share_holdings.insert(
                (listing.campaign_id, buyer),
                &(buyer_shares + listing.shares),
            );
            self.listings.remove(listing_id);
            Ok(total_cost)
        }

        #[ink(message)]
        pub fn share_campaign(
            &mut self,
            campaign_id: u64,
            platform: String,
        ) -> Result<(), CrowdfundingError> {
            let _campaign = self
                .campaigns
                .get(campaign_id)
                .ok_or(CrowdfundingError::CampaignNotFound)?;
            // In a real implementation, this might integrate with social media APIs
            // For now, just emit an event
            self.env().emit_event(CampaignShared {
                campaign_id,
                sharer: self.env().caller(),
                platform,
            });
            Ok(())
        }

        #[ink(message)]
        pub fn assess_risk(
            &mut self,
            campaign_id: u64,
            ltv: u32,
            dev_score: u32,
            volatility: u32,
        ) -> Result<(), CrowdfundingError> {
            if self.env().caller() != self.admin {
                return Err(CrowdfundingError::Unauthorized);
            }
            let rating = if ltv < 60 && dev_score >= 75 && volatility < 15 {
                RiskRating::Low
            } else if ltv < 80 && dev_score >= 50 && volatility < 30 {
                RiskRating::Medium
            } else {
                RiskRating::High
            };
            let profile = RiskProfile {
                campaign_id,
                ltv_ratio: ltv,
                developer_score: dev_score,
                market_volatility: volatility,
                rating,
            };
            self.risk_profiles.insert(campaign_id, &profile);
            Ok(())
        }

        // ── Basic Getters ────────────────────────────────────

        #[ink(message)]
        pub fn get_campaign(&self, campaign_id: u64) -> Option<Campaign> {
            self.campaigns.get(campaign_id)
        }

        #[ink(message)]
        pub fn get_investment(&self, campaign_id: u64, investor: AccountId) -> u128 {
            self.investments.get((campaign_id, investor)).unwrap_or(0)
        }

        #[ink(message)]
        pub fn get_milestone(&self, milestone_id: u64) -> Option<Milestone> {
            self.milestones.get(milestone_id)
        }

        #[ink(message)]
        pub fn get_proposal(&self, proposal_id: u64) -> Option<Proposal> {
            self.proposals.get(proposal_id)
        }

        #[ink(message)]
        pub fn get_listing(&self, listing_id: u64) -> Option<ShareListing> {
            self.listings.get(listing_id)
        }

        #[ink(message)]
        pub fn get_risk_profile(&self, campaign_id: u64) -> Option<RiskProfile> {
            self.risk_profiles.get(campaign_id)
        }

        #[ink(message)]
        pub fn get_campaign_success_metrics(
            &self,
            campaign_id: u64,
        ) -> Option<CampaignSuccessMetrics> {
            let campaign = self.campaigns.get(campaign_id)?;
            let funding_progress_bps = if campaign.target_amount == 0 {
                0
            } else {
                ((campaign.raised_amount.saturating_mul(10_000)) / campaign.target_amount) as u32
            };
            let average_investment = if campaign.investor_count == 0 {
                0
            } else {
                campaign.raised_amount / campaign.investor_count as u128
            };

            Some(CampaignSuccessMetrics {
                campaign_id,
                funding_progress_bps,
                investor_count: campaign.investor_count,
                average_investment,
                total_milestones: self.campaign_milestone_counts.get(campaign_id).unwrap_or(0),
                released_milestones: self.released_milestone_counts.get(campaign_id).unwrap_or(0),
                released_capital: self.released_capital.get(campaign_id).unwrap_or(0),
                is_funded: campaign.status == CampaignStatus::Funded,
            })
        }

        #[ink(message)]
        pub fn get_shares(&self, campaign_id: u64, investor: AccountId) -> u64 {
            self.share_holdings
                .get((campaign_id, investor))
                .unwrap_or(0)
        }

        #[ink(message)]
        pub fn get_admin(&self) -> AccountId {
            self.admin
        }

        // ── Search & Discovery ───────────────────────────────

        fn campaign_to_summary(&self, campaign: &Campaign) -> CampaignSummary {
            let funded_pct = if campaign.target_amount == 0 {
                0u32
            } else {
                ((campaign.raised_amount * 100) / campaign.target_amount) as u32
            };
            let risk_rating = self
                .risk_profiles
                .get(campaign.campaign_id)
                .map(|r| r.rating)
                .unwrap_or(RiskRating::Unrated);
            CampaignSummary {
                campaign_id: campaign.campaign_id,
                creator: campaign.creator,
                title: campaign.title.clone(),
                target_amount: campaign.target_amount,
                raised_amount: campaign.raised_amount,
                funded_pct,
                status: campaign.status,
                investor_count: campaign.investor_count,
                risk_rating,
            }
        }

        fn matches_filter(summary: &CampaignSummary, filter: &CampaignFilter) -> bool {
            if let Some(ref status) = filter.status {
                if &summary.status != status {
                    return false;
                }
            }
            if let Some(ref keyword) = filter.title_keyword {
                if !summary
                    .title
                    .to_lowercase()
                    .contains(&keyword.to_lowercase())
                {
                    return false;
                }
            }
            if let Some(min) = filter.min_target {
                if summary.target_amount < min {
                    return false;
                }
            }
            if let Some(max) = filter.max_target {
                if summary.target_amount > max {
                    return false;
                }
            }
            if let Some(min_pct) = filter.min_funded_pct {
                if summary.funded_pct < min_pct {
                    return false;
                }
            }
            if filter.funded_only && summary.status != CampaignStatus::Funded {
                return false;
            }
            true
        }

        /// Browse all campaigns page by page. `page` is 0-indexed, max page_size is 50.
        #[ink(message)]
        pub fn get_campaigns_paginated(&self, page: u64, page_size: u64) -> Vec<CampaignSummary> {
            let page_size = page_size.min(50);
            let start = (page * page_size) as usize;
            self.campaign_ids
                .iter()
                .skip(start)
                .take(page_size as usize)
                .filter_map(|id| self.campaigns.get(*id))
                .map(|c| self.campaign_to_summary(&c))
                .collect()
        }

        /// Filter campaigns by status, title keyword, amount range, or funded %.
        /// Returns up to `limit` results (max 50).
        #[ink(message)]
        pub fn search_campaigns(&self, filter: CampaignFilter, limit: u64) -> Vec<CampaignSummary> {
            let limit = limit.min(50) as usize;
            self.campaign_ids
                .iter()
                .filter_map(|id| self.campaigns.get(*id))
                .map(|c| self.campaign_to_summary(&c))
                .filter(|s| Self::matches_filter(s, &filter))
                .take(limit)
                .collect()
        }

        /// All campaigns created by a specific account.
        #[ink(message)]
        pub fn get_campaigns_by_creator(&self, creator: AccountId) -> Vec<CampaignSummary> {
            self.campaign_ids
                .iter()
                .filter_map(|id| self.campaigns.get(*id))
                .filter(|c| c.creator == creator)
                .map(|c| self.campaign_to_summary(&c))
                .collect()
        }

        /// Top N campaigns sorted by raised_amount descending (trending / most funded).
        #[ink(message)]
        pub fn get_top_campaigns(&self, n: u64) -> Vec<CampaignSummary> {
            let n = n.min(50) as usize;
            let mut summaries: Vec<CampaignSummary> = self
                .campaign_ids
                .iter()
                .filter_map(|id| self.campaigns.get(*id))
                .map(|c| self.campaign_to_summary(&c))
                .collect();
            summaries.sort_by(|a, b| b.raised_amount.cmp(&a.raised_amount));
            summaries.into_iter().take(n).collect()
        }

        /// All campaigns matching a specific risk rating.
        #[ink(message)]
        pub fn get_campaigns_by_risk(&self, rating: RiskRating) -> Vec<CampaignSummary> {
            self.campaign_ids
                .iter()
                .filter_map(|id| self.campaigns.get(*id))
                .map(|c| self.campaign_to_summary(&c))
                .filter(|s| s.risk_rating == rating)
                .collect()
        }

        /// Active campaigns at or above `threshold_pct`% funded. Good for "closing soon".
        #[ink(message)]
        pub fn get_near_funded_campaigns(&self, threshold_pct: u32) -> Vec<CampaignSummary> {
            self.campaign_ids
                .iter()
                .filter_map(|id| self.campaigns.get(*id))
                .map(|c| self.campaign_to_summary(&c))
                .filter(|s| s.status == CampaignStatus::Active && s.funded_pct >= threshold_pct)
                .collect()
        }

        /// Campaign counts by status: (draft, active, funded, closed, cancelled).
        #[ink(message)]
        pub fn get_campaign_stats(&self) -> (u64, u64, u64, u64, u64) {
            let (mut draft, mut active, mut funded, mut closed, mut cancelled) =
                (0u64, 0u64, 0u64, 0u64, 0u64);
            for id in self.campaign_ids.iter() {
                if let Some(c) = self.campaigns.get(*id) {
                    match c.status {
                        CampaignStatus::Draft => draft += 1,
                        CampaignStatus::Active => active += 1,
                        CampaignStatus::Funded => funded += 1,
                        CampaignStatus::Closed => closed += 1,
                        CampaignStatus::Cancelled => cancelled += 1,
                    }
                }
            }
            (draft, active, funded, closed, cancelled)
        }

        /// All campaigns an investor has contributed to.
        #[ink(message)]
        pub fn get_investor_campaigns(&self, investor: AccountId) -> Vec<CampaignSummary> {
            self.campaign_ids
                .iter()
                .filter_map(|id| {
                    let invested = self.investments.get((*id, investor)).unwrap_or(0);
                    if invested > 0 {
                        self.campaigns.get(*id)
                    } else {
                        None
                    }
                })
                .map(|c| self.campaign_to_summary(&c))
                .collect()
        }

        /// Total number of campaigns ever created.
        #[ink(message)]
        pub fn get_campaign_count(&self) -> u64 {
            self.campaign_count
        }

        // ── Campaign Analytics for Creators ───────────────────

        /// Get comprehensive analytics for a campaign (creator only)
        #[ink(message)]
        pub fn get_campaign_analytics(&self, campaign_id: u64) -> Option<CampaignAnalytics> {
            let campaign = self.campaigns.get(campaign_id)?;
            if self.env().caller() != campaign.creator && self.env().caller() != self.admin {
                return None;
            }

            let total_investors = campaign.investor_count;
            let total_investment = campaign.raised_amount;
            let funding_progress = if campaign.target_amount == 0 {
                0
            } else {
                ((total_investment * 10_000) / campaign.target_amount) as u32
            };

            let average_investment = if total_investors == 0 {
                0
            } else {
                total_investment / total_investors as u128
            };

            // Find largest investment
            let mut largest_investment = 0u128;
            for id in self.campaign_ids.iter() {
                if let Some(investor) = self.campaigns.get(*id) {
                    if investor.campaign_id == campaign_id {
                        // This is inefficient, but we need to iterate through all investments
                        // In a real implementation, we'd store this data
                        break;
                    }
                }
            }

            // For now, we'll approximate largest investment
            // TODO: Store max investment per campaign
            largest_investment = average_investment * 2; // Placeholder

            let total_milestones = self.campaign_milestone_counts.get(campaign_id).unwrap_or(0);
            let released_milestones = self.released_milestone_counts.get(campaign_id).unwrap_or(0);
            let milestone_completion_rate = if total_milestones == 0 {
                0
            } else {
                ((released_milestones as u32 * 10_000) / total_milestones) as u32
            };

            // Placeholder values for time-based metrics
            // In a real implementation, we'd track timestamps
            let days_active = 30; // Placeholder
            let funding_velocity = if days_active == 0 {
                0
            } else {
                total_investment / days_active as u128
            };

            let investor_retention_rate = 8_000; // 80% placeholder
            let risk_score = self
                .risk_profiles
                .get(campaign_id)
                .map(|r| match r.rating {
                    RiskRating::Low => 2_000,
                    RiskRating::Medium => 5_000,
                    RiskRating::High => 8_000,
                    RiskRating::Unrated => 5_000,
                })
                .unwrap_or(5_000);

            let projected_completion_days = if funding_velocity == 0 {
                0
            } else {
                ((campaign.target_amount.saturating_sub(total_investment)) / funding_velocity)
                    as u32
            };

            Some(CampaignAnalytics {
                campaign_id,
                total_investors,
                total_investment,
                funding_progress,
                average_investment,
                largest_investment,
                milestone_completion_rate,
                days_active,
                funding_velocity,
                investor_retention_rate,
                risk_score,
                projected_completion_days,
            })
        }

        /// Get investor demographics for a campaign (creator only)
        #[ink(message)]
        pub fn get_investor_demographics(&self, campaign_id: u64) -> Option<InvestorDemographics> {
            let campaign = self.campaigns.get(campaign_id)?;
            if self.env().caller() != campaign.creator && self.env().caller() != self.admin {
                return None;
            }

            let total_investors = campaign.investor_count;
            let mut accredited_investors = 0u32;
            let mut total_investment = 0u128;
            let mut max_investment = 0u128;
            let mut jurisdiction_counts = Mapping::<u32, u32>::default();
            let mut investment_ranges = Mapping::<u32, u32>::default(); // 0-1k, 1k-10k, 10k-100k, 100k+

            // This is a simplified implementation
            // In practice, we'd need to iterate through all investments
            for id in self.campaign_ids.iter() {
                if let Some(c) = self.campaigns.get(*id) {
                    if c.campaign_id == campaign_id {
                        // Count accredited investors
                        // This is approximate since we don't store per-campaign investor data
                        accredited_investors = (total_investors * 7) / 10; // Assume 70% accredited
                        total_investment = c.raised_amount;
                        break;
                    }
                }
            }

            let average_investment = if total_investors == 0 {
                0
            } else {
                total_investment / total_investors as u128
            };

            // Placeholder jurisdiction data
            let jurisdictions = vec![
                ("US".to_string(), total_investors * 6 / 10),
                ("CA".to_string(), total_investors * 2 / 10),
                ("EU".to_string(), total_investors * 1 / 10),
                ("Other".to_string(), total_investors * 1 / 10),
            ];

            // Placeholder investment distribution
            let investment_distribution = vec![
                (1_000, total_investors * 3 / 10),     // 0-1k
                (10_000, total_investors * 4 / 10),    // 1k-10k
                (100_000, total_investors * 2 / 10),   // 10k-100k
                (1_000_000, total_investors * 1 / 10), // 100k+
            ];

            Some(InvestorDemographics {
                total_investors,
                accredited_investors,
                average_investment,
                top_investor_amount: max_investment,
                jurisdictions,
                investment_distribution,
            })
        }

        /// Get performance comparison with similar campaigns
        #[ink(message)]
        pub fn get_campaign_performance_comparison(
            &self,
            campaign_id: u64,
        ) -> Option<(u32, u32, u32)> {
            let campaign = self.campaigns.get(campaign_id)?;
            if self.env().caller() != campaign.creator && self.env().caller() != self.admin {
                return None;
            }

            let target_range = if campaign.target_amount < 100_000 {
                (0, 100_000)
            } else if campaign.target_amount < 500_000 {
                (100_000, 500_000)
            } else {
                (500_000, u128::MAX)
            };

            let mut similar_campaigns = 0u32;
            let mut better_performing = 0u32;
            let mut total_similar = 0u32;

            for id in self.campaign_ids.iter() {
                if let Some(c) = self.campaigns.get(*id) {
                    if c.campaign_id != campaign_id
                        && c.target_amount >= target_range.0
                        && c.target_amount < target_range.1
                        && c.status == CampaignStatus::Funded
                    {
                        total_similar += 1;
                        let their_progress = if c.target_amount == 0 {
                            0
                        } else {
                            ((c.raised_amount * 100) / c.target_amount) as u32
                        };
                        let our_progress = if campaign.target_amount == 0 {
                            0
                        } else {
                            ((campaign.raised_amount * 100) / campaign.target_amount) as u32
                        };
                        if their_progress > our_progress {
                            better_performing += 1;
                        }
                    }
                }
            }

            if total_similar == 0 {
                return Some((0, 0, 0));
            }

            let percentile = ((total_similar - better_performing) * 100) / total_similar;
            Some((percentile, better_performing, total_similar))
        }

        /// Get funding timeline data points (simplified)
        #[ink(message)]
        pub fn get_funding_timeline(&self, campaign_id: u64) -> Option<Vec<(u32, u128)>> {
            let campaign = self.campaigns.get(campaign_id)?;
            if self.env().caller() != campaign.creator && self.env().caller() != self.admin {
                return None;
            }

            // Placeholder timeline data
            // In a real implementation, we'd store timestamped investment data
            let mut timeline = Vec::new();
            let total_days = 30;
            let daily_target = campaign.target_amount / total_days as u128;

            for day in 1..=total_days {
                let cumulative = (day as u128 * daily_target).min(campaign.raised_amount);
                timeline.push((day, cumulative));
            }

            Some(timeline)
        }
    }

    impl Default for RealEstateCrowdfunding {
        fn default() -> Self {
            Self::new(AccountId::from([0x0; 32]))
        }
    }
}

pub use crate::propchain_crowdfunding::{CrowdfundingError, RealEstateCrowdfunding};

#[cfg(test)]
mod tests {
    use super::*;
    use ink::env::{test, DefaultEnvironment};
    use propchain_crowdfunding::{
        CampaignAnalytics, CampaignFilter, CampaignStatus, CampaignSummary, CrowdfundingError,
        InvestorDemographics, RealEstateCrowdfunding, RiskRating,
    };

    fn setup() -> RealEstateCrowdfunding {
        let accounts = test::default_accounts::<DefaultEnvironment>();
        test::set_caller::<DefaultEnvironment>(accounts.alice);
        RealEstateCrowdfunding::new(accounts.alice)
    }

    // ── Original tests ───────────────────────────────────────

    #[ink::test]
    fn test_create_campaign() {
        let mut contract = setup();
        let campaign_id = contract
            .create_campaign("Downtown Lofts".into(), 1_000_000)
            .unwrap();
        assert_eq!(campaign_id, 1);
        let campaign = contract.get_campaign(1).unwrap();
        assert_eq!(campaign.target_amount, 1_000_000);
    }

    #[ink::test]
    fn test_activate_campaign() {
        let mut contract = setup();
        let campaign_id = contract
            .create_campaign("Harbor View".into(), 500_000)
            .unwrap();
        assert!(contract.activate_campaign(campaign_id).is_ok());
        let campaign = contract.get_campaign(campaign_id).unwrap();
        assert_eq!(campaign.status, CampaignStatus::Active);
    }

    #[ink::test]
    fn test_invest_in_campaign() {
        let mut contract = setup();
        let accounts = test::default_accounts::<DefaultEnvironment>();
        let campaign_id = contract
            .create_campaign("Sunset Villas".into(), 100_000)
            .unwrap();
        contract.activate_campaign(campaign_id).unwrap();
        // Bob onboards (accredited=false until admin verifies)
        test::set_caller::<DefaultEnvironment>(accounts.bob);
        contract.onboard_investor("US".into(), false).unwrap();
        // Admin (alice) verifies accreditation
        test::set_caller::<DefaultEnvironment>(accounts.alice);
        contract.verify_accreditation(accounts.bob).unwrap();
        assert!(contract.is_accredited(accounts.bob));
        // Bob can now invest
        test::set_caller::<DefaultEnvironment>(accounts.bob);
        assert!(contract.invest(campaign_id, 100_000).is_ok());
        let campaign = contract.get_campaign(campaign_id).unwrap();
        assert_eq!(campaign.status, CampaignStatus::Funded);
    }

    #[ink::test]
    fn test_invest_rejected_without_accreditation() {
        let mut contract = setup();
        let accounts = test::default_accounts::<DefaultEnvironment>();
        let campaign_id = contract
            .create_campaign("Sunset Villas".into(), 100_000)
            .unwrap();
        contract.activate_campaign(campaign_id).unwrap();
        test::set_caller::<DefaultEnvironment>(accounts.bob);
        contract.onboard_investor("US".into(), false).unwrap();
        // Bob has not been accredited by admin — invest must fail
        assert_eq!(
            contract.invest(campaign_id, 50_000),
            Err(CrowdfundingError::AccreditationNotVerified)
        );
    }

    #[ink::test]
    fn test_milestone_workflow() {
        let mut contract = setup();
        let campaign_id = contract
            .create_campaign("Park Place".into(), 200_000)
            .unwrap();
        let milestone_id = contract
            .add_milestone(campaign_id, "Foundation".into(), 50_000)
            .unwrap();
        // Oracle must verify before release
        let accounts = test::default_accounts::<DefaultEnvironment>();
        contract.add_oracle(accounts.alice).unwrap();
        contract
            .oracle_verify_milestone(milestone_id, [1u8; 32])
            .unwrap();
        assert!(contract.approve_milestone(milestone_id).is_ok());
        assert!(contract.release_milestone(milestone_id).is_ok());
    }

    #[ink::test]
    fn test_release_milestone_requires_oracle_verification() {
        let mut contract = setup();
        let campaign_id = contract
            .create_campaign("Park Place".into(), 200_000)
            .unwrap();
        let milestone_id = contract
            .add_milestone(campaign_id, "Foundation".into(), 50_000)
            .unwrap();
        contract.approve_milestone(milestone_id).unwrap();
        // Release without oracle verification should fail
        assert_eq!(
            contract.release_milestone(milestone_id),
            Err(CrowdfundingError::OracleVerificationFailed)
        );
    }

    #[ink::test]
    fn test_oracle_verify_milestone() {
        let mut contract = setup();
        let campaign_id = contract
            .create_campaign("Park Place".into(), 200_000)
            .unwrap();
        let milestone_id = contract
            .add_milestone(campaign_id, "Foundation".into(), 50_000)
            .unwrap();
        // Admin can act as oracle
        assert!(contract
            .oracle_verify_milestone(milestone_id, [2u8; 32])
            .is_ok());
        let milestone = contract.get_milestone(milestone_id).unwrap();
        assert!(milestone.oracle_verified);
        assert_eq!(milestone.oracle_data_hash, Some([2u8; 32]));
    }

    #[ink::test]
    fn test_refund_policy_failed_campaign() {
        let mut contract = setup();
        let accounts = test::default_accounts::<DefaultEnvironment>();
        let campaign_id = contract
            .create_campaign("Sunset Villas".into(), 100_000)
            .unwrap();
        contract.activate_campaign(campaign_id).unwrap();
        test::set_caller::<DefaultEnvironment>(accounts.bob);
        contract.onboard_investor("US".into(), false).unwrap();
        test::set_caller::<DefaultEnvironment>(accounts.alice);
        contract.verify_accreditation(accounts.bob).unwrap();
        test::set_caller::<DefaultEnvironment>(accounts.bob);
        contract.invest(campaign_id, 40_000).unwrap();
        // Admin marks campaign as failed
        test::set_caller::<DefaultEnvironment>(accounts.alice);
        assert!(contract.fail_campaign(campaign_id).is_ok());
        // Bob claims refund
        test::set_caller::<DefaultEnvironment>(accounts.bob);
        let refund = contract.claim_refund(campaign_id).unwrap();
        assert_eq!(refund, 40_000);
        assert!(contract.is_refunded(campaign_id, accounts.bob));
    }

    #[ink::test]
    fn test_refund_not_allowed_for_active_campaign() {
        let mut contract = setup();
        let accounts = test::default_accounts::<DefaultEnvironment>();
        let campaign_id = contract
            .create_campaign("Sunset Villas".into(), 100_000)
            .unwrap();
        contract.activate_campaign(campaign_id).unwrap();
        test::set_caller::<DefaultEnvironment>(accounts.bob);
        contract.onboard_investor("US".into(), false).unwrap();
        test::set_caller::<DefaultEnvironment>(accounts.alice);
        contract.verify_accreditation(accounts.bob).unwrap();
        test::set_caller::<DefaultEnvironment>(accounts.bob);
        contract.invest(campaign_id, 40_000).unwrap();
        // Refund should fail for active campaign
        assert_eq!(
            contract.claim_refund(campaign_id),
            Err(CrowdfundingError::CampaignNotFailed)
        );
    }

    #[ink::test]
    fn test_double_refund_not_allowed() {
        let mut contract = setup();
        let accounts = test::default_accounts::<DefaultEnvironment>();
        let campaign_id = contract
            .create_campaign("Sunset Villas".into(), 100_000)
            .unwrap();
        contract.activate_campaign(campaign_id).unwrap();
        test::set_caller::<DefaultEnvironment>(accounts.bob);
        contract.onboard_investor("US".into(), false).unwrap();
        test::set_caller::<DefaultEnvironment>(accounts.alice);
        contract.verify_accreditation(accounts.bob).unwrap();
        test::set_caller::<DefaultEnvironment>(accounts.bob);
        contract.invest(campaign_id, 40_000).unwrap();
        test::set_caller::<DefaultEnvironment>(accounts.alice);
        contract.fail_campaign(campaign_id).unwrap();
        test::set_caller::<DefaultEnvironment>(accounts.bob);
        contract.claim_refund(campaign_id).unwrap();
        // Second refund should fail
        assert_eq!(
            contract.claim_refund(campaign_id),
            Err(CrowdfundingError::AlreadyRefunded)
        );
    }

    #[ink::test]
    fn test_profit_distribution() {
        let mut contract = setup();
        let accounts = test::default_accounts::<DefaultEnvironment>();
        let campaign_id = contract.create_campaign("Test".into(), 100_000).unwrap();
        contract.activate_campaign(campaign_id).unwrap();
        test::set_caller::<DefaultEnvironment>(accounts.bob);
        contract.onboard_investor("US".into(), true).unwrap();
        contract.invest(campaign_id, 60_000).unwrap();
        let payout = contract.distribute_profit(campaign_id, 10_000, accounts.bob);
        assert_eq!(payout, 6_000);
    }

    #[ink::test]
    fn test_governance_voting() {
        let mut contract = setup();
        let accounts = test::default_accounts::<DefaultEnvironment>();
        let campaign_id = contract.create_campaign("Test".into(), 100_000).unwrap();
        let proposal_id = contract
            .create_proposal(campaign_id, "Release funds".into())
            .unwrap();
        assert!(contract.vote(proposal_id, true).is_ok());
        test::set_caller::<DefaultEnvironment>(accounts.bob);
        assert!(contract.vote(proposal_id, true).is_ok());
    }

    #[ink::test]
    fn test_secondary_market() {
        let mut contract = setup();
        let accounts = test::default_accounts::<DefaultEnvironment>();
        let campaign_id = contract.create_campaign("Test".into(), 100_000).unwrap();
        contract.activate_campaign(campaign_id).unwrap();
        test::set_caller::<DefaultEnvironment>(accounts.bob);
        contract.onboard_investor("US".into(), true).unwrap();
        contract.invest(campaign_id, 50_000).unwrap();
        let listing_id = contract.list_shares(campaign_id, 25, 1_000).unwrap();
        test::set_caller::<DefaultEnvironment>(accounts.charlie);
        let cost = contract.buy_shares(listing_id).unwrap();
        assert_eq!(cost, 25_000);
    }

    #[ink::test]
    fn test_risk_assessment() {
        let mut contract = setup();
        let campaign_id = contract.create_campaign("Test".into(), 100_000).unwrap();
        assert!(contract.assess_risk(campaign_id, 50, 80, 10).is_ok());
        let profile = contract.get_risk_profile(campaign_id).unwrap();
        assert_eq!(profile.rating, propchain_crowdfunding::RiskRating::Low);
    }

    #[ink::test]
    fn test_campaign_success_metrics_track_funding_and_milestones() {
        let mut contract = setup();
        let accounts = test::default_accounts::<DefaultEnvironment>();
        let campaign_id = contract
            .create_campaign("Metrics Campaign".into(), 200_000)
            .unwrap();
        contract.activate_campaign(campaign_id).unwrap();

        test::set_caller::<DefaultEnvironment>(accounts.bob);
        contract.onboard_investor("US".into(), true).unwrap();
        contract.invest(campaign_id, 50_000).unwrap();

        test::set_caller::<DefaultEnvironment>(accounts.charlie);
        contract.onboard_investor("CA".into(), true).unwrap();
        contract.invest(campaign_id, 100_000).unwrap();

        test::set_caller::<DefaultEnvironment>(accounts.alice);
        let milestone_id = contract
            .add_milestone(campaign_id, "Permits approved".into(), 40_000)
            .unwrap();
        contract.add_oracle(accounts.alice).unwrap();
        contract
            .oracle_verify_milestone(milestone_id, [9u8; 32])
            .unwrap();
        contract.approve_milestone(milestone_id).unwrap();
        contract.release_milestone(milestone_id).unwrap();

        let metrics = contract.get_campaign_success_metrics(campaign_id).unwrap();
        assert_eq!(metrics.funding_progress_bps, 7_500);
        assert_eq!(metrics.investor_count, 2);
        assert_eq!(metrics.average_investment, 75_000);
        assert_eq!(metrics.total_milestones, 1);
        assert_eq!(metrics.released_milestones, 1);
        assert_eq!(metrics.released_capital, 40_000);
        assert!(!metrics.is_funded);
    }

    #[ink::test]
    fn test_share_campaign() {
        let mut contract = setup();
        let accounts = test::default_accounts::<DefaultEnvironment>();

        let campaign_id = contract
            .create_campaign("Viral Project".into(), 500_000)
            .unwrap();

        test::set_caller::<DefaultEnvironment>(accounts.bob);
        assert!(contract
            .share_campaign(campaign_id, "Twitter".into())
            .is_ok());

        let emitted_events = test::recorded_events().count();
        assert_eq!(emitted_events, 2); // CampaignCreated + CampaignShared
    }

    #[ink::test]
    fn test_share_nonexistent_campaign_fails() {
        let contract = setup();
        assert_eq!(
            contract.share_campaign(999, "Facebook".into()),
            Err(CrowdfundingError::CampaignNotFound)
        );
    }

    #[ink::test]
    fn test_campaign_analytics_for_creator() {
        let mut contract = setup();
        let accounts = test::default_accounts::<DefaultEnvironment>();

        let campaign_id = contract
            .create_campaign("Analytics Test".into(), 200_000)
            .unwrap();
        contract.activate_campaign(campaign_id).unwrap();

        // Add some investments
        test::set_caller::<DefaultEnvironment>(accounts.bob);
        contract.onboard_investor("US".into(), true).unwrap();
        contract.invest(campaign_id, 50_000).unwrap();

        test::set_caller::<DefaultEnvironment>(accounts.charlie);
        contract.onboard_investor("CA".into(), true).unwrap();
        contract.invest(campaign_id, 100_000).unwrap();

        // Add milestone
        test::set_caller::<DefaultEnvironment>(accounts.alice);
        contract
            .add_milestone(campaign_id, "Foundation".into(), 40_000)
            .unwrap();

        // Creator should be able to get analytics
        let analytics = contract.get_campaign_analytics(campaign_id).unwrap();
        assert_eq!(analytics.campaign_id, campaign_id);
        assert_eq!(analytics.total_investors, 2);
        assert_eq!(analytics.total_investment, 150_000);
        assert_eq!(analytics.funding_progress, 7_500); // 75% = 7500 bps
        assert_eq!(analytics.average_investment, 75_000);
        assert_eq!(analytics.milestone_completion_rate, 0); // No milestones released yet
    }

    #[ink::test]
    fn test_campaign_analytics_access_denied_for_non_creator() {
        let mut contract = setup();
        let accounts = test::default_accounts::<DefaultEnvironment>();

        let campaign_id = contract
            .create_campaign("Private Analytics".into(), 100_000)
            .unwrap();

        // Non-creator should not get analytics
        test::set_caller::<DefaultEnvironment>(accounts.bob);
        let analytics = contract.get_campaign_analytics(campaign_id);
        assert!(analytics.is_none());
    }

    #[ink::test]
    fn test_investor_demographics() {
        let mut contract = setup();
        let accounts = test::default_accounts::<DefaultEnvironment>();

        let campaign_id = contract
            .create_campaign("Demographics Test".into(), 300_000)
            .unwrap();
        contract.activate_campaign(campaign_id).unwrap();

        // Add investments from different investors
        test::set_caller::<DefaultEnvironment>(accounts.bob);
        contract.onboard_investor("US".into(), true).unwrap();
        contract.invest(campaign_id, 100_000).unwrap();

        test::set_caller::<DefaultEnvironment>(accounts.charlie);
        contract.onboard_investor("CA".into(), true).unwrap();
        contract.invest(campaign_id, 50_000).unwrap();

        // Creator gets demographics
        test::set_caller::<DefaultEnvironment>(accounts.alice);
        let demographics = contract.get_investor_demographics(campaign_id).unwrap();
        assert_eq!(demographics.total_investors, 2);
        assert_eq!(demographics.average_investment, 75_000);
        assert!(!demographics.jurisdictions.is_empty());
    }
}
