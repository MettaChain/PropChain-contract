//! Unit tests for the version registry (#1463).
//!
//! The registry records deployments per contract name and version; it has **no**
//! deprecation flag and no compatibility API. The tests therefore cover what the
//! registry does provide and map the issue's "compatibility" item onto the
//! comparisons it does support: ordering of versions, code-hash differences
//! between versions, and gap handling in the history. Deprecation and
//! "deprecation prevents new deployments" are not testable against this
//! contract and are intentionally not faked here.

#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(test)]
mod tests {
    use ink::env::{test, DefaultEnvironment};
    use version_registry::version_registry::{Error, VersionRegistry};

    type Accounts = test::DefaultAccounts<DefaultEnvironment>;

    const V1_HASH: [u8; 32] = [0x11; 32];
    const V2_HASH: [u8; 32] = [0x22; 32];
    const V3_HASH: [u8; 32] = [0x33; 32];

    fn setup() -> (VersionRegistry, Accounts) {
        let accounts = test::default_accounts::<DefaultEnvironment>();
        test::set_caller::<DefaultEnvironment>(accounts.alice);
        test::set_block_timestamp::<DefaultEnvironment>(5_000);
        (VersionRegistry::new(), accounts)
    }

    fn name(s: &str) -> String {
        s.to_string()
    }

    // ── Registration ────────────────────────────────────────────────────

    #[ink::test]
    fn registers_a_version_with_valid_metadata() {
        let (mut r, accounts) = setup();
        assert_eq!(r.register_deployment(name("token"), V1_HASH), Ok(1));

        let rec = r.get_deployment(name("token"), 1).expect("recorded");
        assert_eq!(rec.contract_name, "token");
        assert_eq!(rec.version, 1);
        assert_eq!(rec.code_hash, V1_HASH);
        assert_eq!(rec.deployed_at, 5_000);
        assert_eq!(rec.deployer, accounts.alice);
    }

    #[ink::test]
    fn versions_auto_increment_per_name() {
        let (mut r, _) = setup();
        assert_eq!(r.register_deployment(name("token"), V1_HASH), Ok(1));
        assert_eq!(r.register_deployment(name("token"), V2_HASH), Ok(2));
        // A different name starts again at 1.
        assert_eq!(r.register_deployment(name("escrow"), V1_HASH), Ok(1));
        assert_eq!(r.get_latest_version(name("token")), Some(2));
        assert_eq!(r.get_latest_version(name("escrow")), Some(1));
    }

    #[ink::test]
    fn registering_an_existing_explicit_version_is_rejected() {
        let (mut r, _) = setup();
        r.register_deployment_with_version(name("token"), 3, V3_HASH).unwrap();
        assert_eq!(
            r.register_deployment_with_version(name("token"), 3, V1_HASH),
            Err(Error::VersionAlreadyExists)
        );
        // The original record is untouched.
        assert_eq!(r.get_deployment(name("token"), 3).unwrap().code_hash, V3_HASH);
    }

    #[ink::test]
    fn version_zero_is_invalid() {
        let (mut r, _) = setup();
        assert_eq!(
            r.register_deployment_with_version(name("token"), 0, V1_HASH),
            Err(Error::InvalidVersion)
        );
        assert!(r.get_latest_version(name("token")).is_none());
    }

    #[ink::test]
    fn only_the_admin_can_register() {
        let (mut r, accounts) = setup();
        test::set_caller::<DefaultEnvironment>(accounts.bob);
        assert_eq!(r.register_deployment(name("token"), V1_HASH), Err(Error::Unauthorized));
        assert_eq!(
            r.register_deployment_with_version(name("token"), 1, V1_HASH),
            Err(Error::Unauthorized)
        );
        assert!(r.get_all_names().is_empty());
    }

    // ── Version comparison (the closest thing to "compatibility") ───────

    #[ink::test]
    fn two_versions_can_be_compared_by_code_hash_and_order() {
        let (mut r, _) = setup();
        r.register_deployment(name("token"), V1_HASH).unwrap();
        r.register_deployment(name("token"), V2_HASH).unwrap();

        let v1 = r.get_deployment(name("token"), 1).unwrap();
        let v2 = r.get_deployment(name("token"), 2).unwrap();
        assert!(v2.version > v1.version);
        assert_ne!(v1.code_hash, v2.code_hash);
        assert!(v2.deployed_at >= v1.deployed_at);
    }

    #[ink::test]
    fn redeploying_the_same_code_hash_is_a_distinct_version() {
        let (mut r, _) = setup();
        r.register_deployment(name("token"), V1_HASH).unwrap();
        r.register_deployment(name("token"), V1_HASH).unwrap();
        assert_eq!(
            r.get_deployment(name("token"), 1).unwrap().code_hash,
            r.get_deployment(name("token"), 2).unwrap().code_hash
        );
        assert_eq!(r.get_latest_version(name("token")), Some(2));
    }

    // ── Queries ─────────────────────────────────────────────────────────

    #[ink::test]
    fn a_version_query_returns_the_matching_metadata_only() {
        let (mut r, _) = setup();
        r.register_deployment(name("token"), V1_HASH).unwrap();
        r.register_deployment(name("token"), V2_HASH).unwrap();
        assert_eq!(r.get_deployment(name("token"), 1).unwrap().code_hash, V1_HASH);
        assert_eq!(r.get_deployment(name("token"), 2).unwrap().code_hash, V2_HASH);
        assert!(r.get_deployment(name("token"), 3).is_none());
        assert!(r.get_deployment(name("other"), 1).is_none());
    }

    #[ink::test]
    fn latest_version_tracks_the_highest_registered() {
        let (mut r, _) = setup();
        assert_eq!(r.get_latest_version(name("token")), None);
        r.register_deployment(name("token"), V1_HASH).unwrap();
        assert_eq!(r.get_latest_version(name("token")), Some(1));
        r.register_deployment_with_version(name("token"), 5, V2_HASH).unwrap();
        assert_eq!(r.get_latest_version(name("token")), Some(5));
        // Registering a lower explicit version does not lower the latest.
        r.register_deployment_with_version(name("token"), 3, V3_HASH).unwrap();
        assert_eq!(r.get_latest_version(name("token")), Some(5));
    }

    #[ink::test]
    fn an_explicit_version_advances_the_auto_increment_counter() {
        let (mut r, _) = setup();
        r.register_deployment_with_version(name("token"), 4, V1_HASH).unwrap();
        assert_eq!(r.register_deployment(name("token"), V2_HASH), Ok(5));
    }

    // ── History ─────────────────────────────────────────────────────────

    #[ink::test]
    fn history_is_returned_in_ascending_version_order() {
        let (mut r, _) = setup();
        r.register_deployment(name("token"), V1_HASH).unwrap();
        r.register_deployment(name("token"), V2_HASH).unwrap();
        r.register_deployment(name("token"), V3_HASH).unwrap();
        let versions: Vec<u32> = r.get_deployment_history(name("token")).iter().map(|d| d.version).collect();
        assert_eq!(versions, vec![1, 2, 3]);
        let hashes: Vec<[u8; 32]> = r.get_deployment_history(name("token")).iter().map(|d| d.code_hash).collect();
        assert_eq!(hashes, vec![V1_HASH, V2_HASH, V3_HASH]);
    }

    #[ink::test]
    fn history_skips_gaps_left_by_explicit_versions() {
        let (mut r, _) = setup();
        r.register_deployment_with_version(name("token"), 1, V1_HASH).unwrap();
        r.register_deployment_with_version(name("token"), 4, V2_HASH).unwrap();
        let versions: Vec<u32> = r.get_deployment_history(name("token")).iter().map(|d| d.version).collect();
        assert_eq!(versions, vec![1, 4]);
    }

    #[ink::test]
    fn unknown_names_have_empty_history() {
        let (r, _) = setup();
        assert!(r.get_deployment_history(name("ghost")).is_empty());
    }

    #[ink::test]
    fn names_are_listed_once_in_first_registration_order() {
        let (mut r, _) = setup();
        r.register_deployment(name("token"), V1_HASH).unwrap();
        r.register_deployment(name("escrow"), V1_HASH).unwrap();
        r.register_deployment(name("token"), V2_HASH).unwrap();
        assert_eq!(r.get_all_names(), vec![name("token"), name("escrow")]);
    }

    #[ink::test]
    fn registration_emits_one_event_per_deployment() {
        let (mut r, _) = setup();
        r.register_deployment(name("token"), V1_HASH).unwrap();
        assert_eq!(test::recorded_events().count(), 1);
        r.register_deployment_with_version(name("token"), 7, V2_HASH).unwrap();
        assert_eq!(test::recorded_events().count(), 2);
    }
}
