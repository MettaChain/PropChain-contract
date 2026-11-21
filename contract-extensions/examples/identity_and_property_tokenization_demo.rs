//! Lean integration-style demo examples for:
//! - #1509 Identity Verification Flow Demo
//! - #1505 Property Tokenization End-to-End Demo
//!
//! These demos are intentionally minimal and focus on readable flow/state output.
//! They can be adapted into runnable `cargo run --example` binaries with full
//! contract wiring in the extension workspace.

#[derive(Debug, Clone)]
pub struct IdentityState {
    pub owner: String,
    pub attributes: Vec<(String, String, bool)>, // (key, value/hash, verified)
    pub recovery_guardians: Vec<String>,
    pub recovery_initiated: bool,
}

pub fn run_identity_verification_demo() {
    println!("== Identity Verification Flow (#1509) ==");
    let mut identity = IdentityState {
        owner: "alice".into(),
        attributes: vec![],
        recovery_guardians: vec!["guardian1".into(), "guardian2".into()],
        recovery_initiated: false,
    };
    println!("created identity owner={}", identity.owner);

    identity.attributes.push(("email".into(), "alice@example.com".into(), false));
    identity.attributes.push(("kyc_hash".into(), "0xabc123".into(), false));
    println!("added attributes: {:?}", identity.attributes);

    // verifier confirms attributes
    for attr in &mut identity.attributes {
        attr.2 = true;
    }
    println!("verified attributes by authorized verifier: {:?}", identity.attributes);

    // social recovery initiated by guardians
    identity.recovery_initiated = true;
    println!("social recovery initiated by guardians: {:?}", identity.recovery_guardians);
    println!("recovery status: initiated={}", identity.recovery_initiated);

    // transfer identity to new owner after recovery
    identity.owner = "alice_new_key".into();
    identity.recovery_initiated = false;
    println!("identity transferred to new owner={}, recovery_initiated={}", identity.owner, identity.recovery_initiated);
}

#[derive(Debug, Clone)]
pub struct PropertyState {
    pub id: u64,
    pub valuation: u128,
    pub total_tokens: u128,
    pub balances: Vec<(String, u128)>,
    pub yield_pool: u128,
}

pub fn run_property_tokenization_demo() {
    println!("== Property Tokenization End-to-End (#1505) ==");
    let mut property = PropertyState {
        id: 101,
        valuation: 500_000,
        total_tokens: 0,
        balances: vec![],
        yield_pool: 0,
    };
    println!("registered property id={} valuation={}", property.id, property.valuation);

    property.total_tokens = 100_000;
    println!("minted fractional tokens total={}", property.total_tokens);

    property.balances = vec![("investor_a".into(), 60_000), ("investor_b".into(), 40_000)];
    println!("distributed tokens: {:?}", property.balances);

    // oracle valuation update
    property.valuation = 525_000;
    println!("oracle updated property valuation={}", property.valuation);

    // rental yield deposit and proportional distribution
    property.yield_pool = 2_000;
    println!("deposited rental yield pool={}", property.yield_pool);
    for (holder, balance) in &property.balances {
        let share = if property.total_tokens == 0 { 0 } else { property.yield_pool * *balance / property.total_tokens };
        println!("yield distribution holder={} tokens={} share={}", holder, balance, share);
    }
}

fn main() {
    run_identity_verification_demo();
    println!();
    run_property_tokenization_demo();
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn demos_run_without_panic() {
        run_identity_verification_demo();
        run_property_tokenization_demo();
    }
}
