//! Lean integration-style demo examples for:
//! - #1507 Crowdfunding Campaign Lifecycle Demo
//! - #1506 Insurance Policy Purchase and Claim Demo
//!
//! Minimal, readable flows with printed state transitions for review/extension.

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CampaignStatus {
    Active,
    Successful,
    Failed,
}

#[derive(Debug, Clone)]
pub struct Campaign {
    pub goal: u128,
    pub raised: u128,
    pub deadline_block: u32,
    pub status: CampaignStatus,
    pub contributions: Vec<(String, u128)>,
}

pub fn run_crowdfunding_demo() {
    println!("== Crowdfunding Campaign Lifecycle (#1507) ==");
    let mut campaign = Campaign {
        goal: 1_000,
        raised: 0,
        deadline_block: 100,
        status: CampaignStatus::Active,
        contributions: vec![],
    };
    println!("created campaign goal={} deadline_block={}", campaign.goal, campaign.deadline_block);

    for (name, amount) in [("alice", 400u128), ("bob", 700u128)] {
        campaign.contributions.push((name.into(), amount));
        campaign.raised += amount;
        println!("contribution from {} amount={} total_raised={}", name, amount, campaign.raised);
    }

    if campaign.raised >= campaign.goal {
        campaign.status = CampaignStatus::Successful;
        println!("campaign successful: status={:?}", campaign.status);
        println!("creator withdraws funds amount={}", campaign.raised);
    }

    // failed campaign scenario
    let mut failed = Campaign {
        goal: 2_000,
        raised: 300,
        deadline_block: 50,
        status: CampaignStatus::Active,
        contributions: vec![("carol".into(), 300)],
    };
    let current_block = 60u32;
    if current_block >= failed.deadline_block && failed.raised < failed.goal {
        failed.status = CampaignStatus::Failed;
        println!("campaign failed at block {} status={:?}", current_block, failed.status);
        for (name, amount) in &failed.contributions {
            println!("refund claimed by {} amount={}", name, amount);
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PolicyStatus {
    Active,
    ClaimApproved,
    ClaimRejected,
    Expired,
}

#[derive(Debug, Clone)]
pub struct InsurancePolicy {
    pub holder: String,
    pub coverage: u128,
    pub premium: u128,
    pub expiry_block: u32,
    pub status: PolicyStatus,
}

pub fn run_insurance_demo() {
    println!("== Insurance Policy Purchase and Claim (#1506) ==");
    let mut policy = InsurancePolicy {
        holder: "dave".into(),
        coverage: 10_000,
        premium: 200,
        expiry_block: 200,
        status: PolicyStatus::Active,
    };
    println!("policy created holder={} coverage={} premium={} expiry={}", policy.holder, policy.coverage, policy.premium, policy.expiry_block);
    println!("premium paid -> policy activated status={:?}", policy.status);

    // claim with sufficient evidence
    let evidence_ok = true;
    if evidence_ok {
        policy.status = PolicyStatus::ClaimApproved;
        println!("claim filed (evidence_ok={}) -> status={:?} payout={}", evidence_ok, policy.status, policy.coverage);
    }

    // rejection scenario
    let mut rejected = InsurancePolicy { status: PolicyStatus::Active, ..policy.clone() };
    let evidence_ok = false;
    if !evidence_ok {
        rejected.status = PolicyStatus::ClaimRejected;
        println!("claim filed (evidence_ok={}) -> status={:?}", evidence_ok, rejected.status);
    }

    // expiry/renewal
    let mut expiring = InsurancePolicy { status: PolicyStatus::Active, ..policy.clone() };
    let current_block = 201u32;
    if current_block > expiring.expiry_block {
        expiring.status = PolicyStatus::Expired;
        println!("policy expired at block {} status={:?}", current_block, expiring.status);
        expiring.expiry_block = 400;
        expiring.status = PolicyStatus::Active;
        println!("policy renewed new_expiry={} status={:?}", expiring.expiry_block, expiring.status);
    }
}

fn main() {
    run_crowdfunding_demo();
    println!();
    run_insurance_demo();
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn demos_run_without_panic() {
        run_crowdfunding_demo();
        run_insurance_demo();
    }
}
