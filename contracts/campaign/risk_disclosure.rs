// Soroban Smart Contract Feature: Require Risk Disclosure for All Campaigns
// This module enforces that every campaign must include a risk disclosure
// before it can be considered valid or active.

use soroban_sdk::{contract, contractimpl, contracttype, Env, String, Address};

#[derive(Clone)]
#[contracttype]
pub enum DataKey {
    CampaignCreator(u64),
    RiskDisclosure(u64),
}

#[contract]
pub struct CampaignContract;

#[contractimpl]
impl CampaignContract {

    // 🔹 Set Risk Disclosure (Required before campaign activation)
    pub fn set_risk_disclosure(
        env: Env,
        campaign_id: u64,
        creator: Address,
        disclosure: String,
    ) {
        // Ensure creator signs transaction
        creator.require_auth();

        // Verify campaign exists and creator owns it
        let stored_creator: Address = env
            .storage()
            .instance()
            .get(&DataKey::CampaignCreator(campaign_id))
            .expect("Campaign not found");

        if stored_creator != creator {
            panic!("Unauthorized: Not campaign owner");
        }

        // Basic validation
        if disclosure.len() < 20 {
            panic!("Risk disclosure too short");
        }

        // Store disclosure
        env.storage()
            .instance()
            .set(&DataKey::RiskDisclosure(campaign_id), &disclosure);

        // Emit event
        env.events().publish(
            ("risk_disclosure_set", campaign_id),
            disclosure.clone(),
        );
    }

    // 🔹 Get Risk Disclosure
    pub fn get_risk_disclosure(env: Env, campaign_id: u64) -> String {
        env.storage()
            .instance()
            .get(&DataKey::RiskDisclosure(campaign_id))
            .expect("Risk disclosure not set")
    }

    // 🔹 Validate Campaign Before Actions (e.g., funding)
    pub fn validate_campaign(env: Env, campaign_id: u64) {
        let exists: Option<String> = env
            .storage()
            .instance()
            .get(&DataKey::RiskDisclosure(campaign_id));

        if exists.is_none() {
            panic!("Campaign missing required risk disclosure");
        }
    }

    // 🔹 Example: Funding function with enforcement
    pub fn fund_campaign(env: Env, campaign_id: u64, backer: Address) {
        backer.require_auth();

        // Enforce disclosure before funding
        Self::validate_campaign(env.clone(), campaign_id);

        // Continue funding logic (not implemented here)
        env.events().publish(
            ("campaign_funded", campaign_id),
            backer,
        );
    }
}
