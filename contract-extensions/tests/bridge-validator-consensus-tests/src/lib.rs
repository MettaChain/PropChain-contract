//! Unit tests for bridge validator consensus and signature thresholds (#1451).
//!
//! Chain 1 is the contract's (hard-coded) source chain, 2 and 3 are
//! destinations. The bridge is deployed with `min = 2`, `max = 5` signatures.

#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(test)]
mod tests {
    use ink::env::{test, DefaultEnvironment};
    use propchain_bridge::bridge::{Error, PropertyBridge};
    use propchain_traits::PropertyMetadata;

    type Accounts = test::DefaultAccounts<DefaultEnvironment>;

    const DEST: u64 = 2;

    fn metadata() -> PropertyMetadata {
        PropertyMetadata {
            location: String::from("Test Property"),
            size: 1000,
            legal_description: String::from("Test"),
            valuation: 100_000,
            documents_url: String::from("ipfs://test"),
        }
    }

    /// Admin (alice) is the only operator. Validators: alice, bob, charlie.
    fn setup() -> (PropertyBridge, Accounts) {
        let accounts = test::default_accounts::<DefaultEnvironment>();
        test::set_caller::<DefaultEnvironment>(accounts.alice);
        let mut bridge = PropertyBridge::new(vec![1, 2, 3], 2, 5, 100, 500_000);
        for v in [accounts.alice, accounts.bob, accounts.charlie] {
            bridge.add_validator(v).unwrap();
        }
        (bridge, accounts)
    }

    fn request(bridge: &mut PropertyBridge, accounts: &Accounts, required: u8) -> u64 {
        test::set_caller::<DefaultEnvironment>(accounts.alice);
        bridge
            .initiate_bridge_multisig(1, DEST, accounts.django, required, Some(50), metadata())
            .expect("request created")
    }

    fn sign(
        bridge: &mut PropertyBridge,
        who: ink::primitives::AccountId,
        id: u64,
        approve: bool,
    ) -> Result<(), Error> {
        test::set_caller::<DefaultEnvironment>(who);
        bridge.sign_bridge_request(id, approve)
    }

    fn execute(bridge: &mut PropertyBridge, accounts: &Accounts, id: u64) -> Result<(), Error> {
        test::set_caller::<DefaultEnvironment>(accounts.alice);
        bridge.execute_bridge(id)
    }

    // ── Threshold configuration ─────────────────────────────────────────

    #[ink::test]
    fn the_required_signature_count_must_be_within_the_configured_bounds() {
        let (mut b, accounts) = setup();
        test::set_caller::<DefaultEnvironment>(accounts.alice);
        for bad in [0u8, 1, 6, 255] {
            assert_eq!(
                b.initiate_bridge_multisig(1, DEST, accounts.django, bad, None, metadata()),
                Err(Error::InsufficientSignatures),
                "required = {bad}"
            );
        }
        for good in [2u8, 5] {
            assert!(b.initiate_bridge_multisig(1, DEST, accounts.django, good, None, metadata()).is_ok());
        }
    }

    #[ink::test]
    fn requests_to_unsupported_chains_are_rejected() {
        let (mut b, accounts) = setup();
        test::set_caller::<DefaultEnvironment>(accounts.alice);
        assert_eq!(
            b.initiate_bridge_multisig(1, 99, accounts.django, 2, None, metadata()),
            Err(Error::InvalidChain)
        );
    }

    // ── Validator set management ────────────────────────────────────────

    #[ink::test]
    fn only_the_admin_manages_validators_and_adding_is_idempotent() {
        let (mut b, accounts) = setup();
        assert_eq!(b.get_validators().len(), 3);

        test::set_caller::<DefaultEnvironment>(accounts.bob);
        assert_eq!(b.add_validator(accounts.eve), Err(Error::Unauthorized));
        assert_eq!(b.remove_validator(accounts.alice), Err(Error::Unauthorized));

        test::set_caller::<DefaultEnvironment>(accounts.alice);
        b.add_validator(accounts.bob).unwrap();
        assert_eq!(b.get_validators().len(), 3);
        assert!(b.is_validator(accounts.bob));
        assert!(!b.is_validator(accounts.eve));

        b.remove_validator(accounts.charlie).unwrap();
        assert!(!b.is_validator(accounts.charlie));
        assert_eq!(b.get_validators().len(), 2);
    }

    #[ink::test]
    fn non_validators_cannot_sign() {
        let (mut b, accounts) = setup();
        let id = request(&mut b, &accounts, 2);
        assert_eq!(sign(&mut b, accounts.eve, id, true), Err(Error::Unauthorized));
        assert!(b.get_signer_list(id).unwrap().is_empty());
    }

    #[ink::test]
    fn a_removed_validator_can_no_longer_sign() {
        let (mut b, accounts) = setup();
        let id = request(&mut b, &accounts, 2);
        test::set_caller::<DefaultEnvironment>(accounts.alice);
        b.remove_validator(accounts.bob).unwrap();
        assert_eq!(sign(&mut b, accounts.bob, id, true), Err(Error::Unauthorized));
    }

    #[ink::test]
    fn signing_an_unknown_request_fails() {
        let (mut b, accounts) = setup();
        assert_eq!(sign(&mut b, accounts.alice, 99, true), Err(Error::InvalidRequest));
    }

    // ── Consensus threshold ─────────────────────────────────────────────

    #[ink::test]
    fn the_request_executes_only_after_the_threshold_is_reached() {
        let (mut b, accounts) = setup();
        let id = request(&mut b, &accounts, 2);

        sign(&mut b, accounts.alice, id, true).unwrap();
        assert_eq!(execute(&mut b, &accounts, id), Err(Error::InvalidRequest));

        sign(&mut b, accounts.bob, id, true).unwrap();
        assert!(execute(&mut b, &accounts, id).is_ok());
        assert_eq!(b.get_bridge_history(accounts.alice, 0, 10).len(), 1);
    }

    #[ink::test]
    fn the_signer_list_records_each_validator_once() {
        let (mut b, accounts) = setup();
        let id = request(&mut b, &accounts, 3);
        sign(&mut b, accounts.alice, id, true).unwrap();
        sign(&mut b, accounts.charlie, id, true).unwrap();
        let signers = b.get_signer_list(id).unwrap();
        assert_eq!(signers.len(), 2);
        assert!(signers.contains(&accounts.alice) && signers.contains(&accounts.charlie));
    }

    #[ink::test]
    fn a_larger_threshold_needs_every_required_signature() {
        let (mut b, accounts) = setup();
        let id = request(&mut b, &accounts, 3);
        sign(&mut b, accounts.alice, id, true).unwrap();
        sign(&mut b, accounts.bob, id, true).unwrap();
        // 2 of 3: still not executable.
        assert!(execute(&mut b, &accounts, id).is_err());
        sign(&mut b, accounts.charlie, id, true).unwrap();
        assert!(execute(&mut b, &accounts, id).is_ok());
    }

    #[ink::test]
    fn a_threshold_larger_than_the_validator_set_can_never_be_met() {
        let (mut b, accounts) = setup();
        let id = request(&mut b, &accounts, 4); // only three validators exist
        for v in [accounts.alice, accounts.bob, accounts.charlie] {
            sign(&mut b, v, id, true).unwrap();
        }
        assert!(execute(&mut b, &accounts, id).is_err());
    }

    #[ink::test]
    fn an_executed_request_cannot_be_executed_again() {
        let (mut b, accounts) = setup();
        let id = request(&mut b, &accounts, 2);
        sign(&mut b, accounts.alice, id, true).unwrap();
        sign(&mut b, accounts.bob, id, true).unwrap();
        execute(&mut b, &accounts, id).unwrap();
        assert!(execute(&mut b, &accounts, id).is_err());
        assert_eq!(b.get_bridge_history(accounts.alice, 0, 10).len(), 1);
    }

    // ── Duplicate, conflicting and rejecting votes ──────────────────────

    #[ink::test]
    fn a_duplicate_signature_is_rejected_and_not_counted() {
        let (mut b, accounts) = setup();
        let id = request(&mut b, &accounts, 2);
        sign(&mut b, accounts.alice, id, true).unwrap();
        assert_eq!(sign(&mut b, accounts.alice, id, true), Err(Error::AlreadySigned));
        assert_eq!(b.get_signer_list(id).unwrap().len(), 1);
        // The duplicate must not have satisfied the threshold on its own.
        assert!(execute(&mut b, &accounts, id).is_err());
    }

    #[ink::test]
    fn a_contradictory_second_vote_is_rejected() {
        let (mut b, accounts) = setup();
        let id = request(&mut b, &accounts, 2);
        sign(&mut b, accounts.alice, id, true).unwrap();
        assert_eq!(sign(&mut b, accounts.alice, id, false), Err(Error::VoteConflict));
        assert_eq!(b.get_signer_list(id).unwrap().len(), 1);
    }

    #[ink::test]
    fn a_rejecting_vote_prevents_execution() {
        let (mut b, accounts) = setup();
        let id = request(&mut b, &accounts, 2);
        sign(&mut b, accounts.alice, id, true).unwrap();
        sign(&mut b, accounts.bob, id, false).unwrap();
        assert!(execute(&mut b, &accounts, id).is_err());
        // Further approvals cannot revive the failed request.
        let _ = sign(&mut b, accounts.charlie, id, true);
        assert!(execute(&mut b, &accounts, id).is_err());
    }

    // ── Operator role and expiry ────────────────────────────────────────

    #[ink::test]
    fn only_a_bridge_operator_can_execute() {
        let (mut b, accounts) = setup();
        let id = request(&mut b, &accounts, 2);
        sign(&mut b, accounts.alice, id, true).unwrap();
        sign(&mut b, accounts.bob, id, true).unwrap();

        // Bob is a validator but not an operator.
        test::set_caller::<DefaultEnvironment>(accounts.bob);
        assert_eq!(b.execute_bridge(id), Err(Error::Unauthorized));

        test::set_caller::<DefaultEnvironment>(accounts.alice);
        b.add_bridge_operator(accounts.bob).unwrap();
        test::set_caller::<DefaultEnvironment>(accounts.bob);
        assert!(b.execute_bridge(id).is_ok());
    }

    #[ink::test]
    fn an_expired_request_cannot_be_signed() {
        let (mut b, accounts) = setup();
        test::set_caller::<DefaultEnvironment>(accounts.alice);
        let id = b
            .initiate_bridge_multisig(1, DEST, accounts.django, 2, Some(1), metadata())
            .unwrap();
        test::advance_block::<DefaultEnvironment>();
        test::advance_block::<DefaultEnvironment>();
        assert_eq!(sign(&mut b, accounts.alice, id, true), Err(Error::RequestExpired));
    }
}
