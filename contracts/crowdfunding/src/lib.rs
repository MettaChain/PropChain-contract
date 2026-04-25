#![cfg_attr(not(feature = "std"), no_std, no_main)]
#![allow(
    clippy::arithmetic_side_effects,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::needless_borrows_for_generic_args
)]

use ink::storage::Mapping;

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
        InvalidParameters,
        AlreadyVoted,
        ReentrantCall,
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

    #[ink(storage)]
    pub struct RealEstateCrowdfunding {
        admin: AccountId,
        campaigns: Mapping<u64, Campaign>,
        campaign_count: u64,
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
        blocked_jurisdictions: Vec<String>,
        reentrancy_guard: propchain_traits::ReentrancyGuard,
    }

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

    impl RealEstateCrowdfunding {
        #[ink(constructor)]
        pub fn new(admin: AccountId) -> Self {
            Self {
                admin,
                campaigns: Mapping::default(),
                campaign_count: 0,
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
                blocked_jurisdictions: Vec::new(),
                reentrancy_guard: propchain_traits::ReentrancyGuard::new(),
            }
        }

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
            };
            self.milestones.insert(self.milestone_count, &milestone);
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
                milestone.status = MilestoneStatus::Released;
                self.milestones.insert(milestone_id, &milestone);
                Ok(())
            })
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
        pub fn get_shares(&self, campaign_id: u64, investor: AccountId) -> u64 {
            self.share_holdings
                .get((campaign_id, investor))
                .unwrap_or(0)
        }

        #[ink(message)]
        pub fn get_admin(&self) -> AccountId {
            self.admin
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
    use propchain_crowdfunding::{CampaignStatus, CrowdfundingError, RealEstateCrowdfunding};

    fn setup() -> RealEstateCrowdfunding {
        let accounts = test::default_accounts::<DefaultEnvironment>();
        test::set_caller::<DefaultEnvironment>(accounts.alice);
        RealEstateCrowdfunding::new(accounts.alice)
    }

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
        test::set_caller::<DefaultEnvironment>(accounts.bob);
        contract.onboard_investor("US".into(), true).unwrap();
        assert!(contract.invest(campaign_id, 100_000).is_ok());
        let campaign = contract.get_campaign(campaign_id).unwrap();
        assert_eq!(campaign.status, CampaignStatus::Funded);
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
        assert!(contract.approve_milestone(milestone_id).is_ok());
        assert!(contract.release_milestone(milestone_id).is_ok());
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
}
