//! Unit tests for the sanctions screening contract (#1458).
//!
//! The contract lists *entities* (by id, name, type and jurisdiction) and
//! *properties*, and screens a property (optionally with an entity) in a
//! jurisdiction. It has no address blocklist, no bulk-update message and no
//! "blocked transfer" hook, so:
//! - "address on / off the list" is expressed as entity listed / removed,
//! - "bulk update" is repeated `add_sanctioned_entity` calls,
//! - "export for auditing" is `get_property_screenings` / `get_screening_result`,
//! - "compliance check integration" is `screen_property` returning `passed`.

#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(test)]
mod tests {
    use ink::env::{test, DefaultEnvironment};
    use propchain_sanctions::sanctions_screening::{
        EntityType, Error, SanctionLevel, SanctionsScreening,
    };

    type Accounts = test::DefaultAccounts<DefaultEnvironment>;

    const JURISDICTION_A: u32 = 840;
    const JURISDICTION_B: u32 = 826;
    const PROPERTY: u64 = 42;

    fn setup() -> (SanctionsScreening, Accounts) {
        let accounts = test::default_accounts::<DefaultEnvironment>();
        test::set_caller::<DefaultEnvironment>(accounts.alice);
        test::set_block_timestamp::<DefaultEnvironment>(1_000);
        (SanctionsScreening::new(), accounts)
    }

    fn list_entity(s: &mut SanctionsScreening, name: &str, level: SanctionLevel) -> u64 {
        s.add_sanctioned_entity(name.as_bytes().to_vec(), EntityType::Corporation, JURISDICTION_A, level)
            .expect("entity listed")
    }

    // ── Entity list management ──────────────────────────────────────────

    #[ink::test]
    fn adds_an_entity_and_reads_it_back() {
        let (mut s, _) = setup();
        let id = list_entity(&mut s, "ACME Holdings", SanctionLevel::Prohibited);
        assert_eq!(id, 1);

        let e = s.get_sanctioned_entity(id).unwrap();
        assert_eq!(e.entity_name, b"ACME Holdings".to_vec());
        assert_eq!(e.entity_type, EntityType::Corporation);
        assert_eq!(e.jurisdiction_code, JURISDICTION_A);
        assert_eq!(e.sanction_level, SanctionLevel::Prohibited);
        assert_eq!(e.listed_at, 1_000);
        assert!(e.active);
        assert_eq!(e.resolved_at, None);
        assert_eq!(s.active_entity_count(), 1);
        assert!(s.get_sanctioned_entity(99).is_none());
    }

    #[ink::test]
    fn removing_an_entity_deactivates_it_and_is_idempotent() {
        let (mut s, _) = setup();
        let id = list_entity(&mut s, "ACME", SanctionLevel::Prohibited);
        test::set_block_timestamp::<DefaultEnvironment>(2_000);
        s.remove_sanctioned_entity(id).unwrap();

        let e = s.get_sanctioned_entity(id).unwrap();
        assert!(!e.active);
        assert_eq!(e.resolved_at, Some(2_000));
        assert_eq!(s.active_entity_count(), 0);

        // Removing again changes nothing.
        s.remove_sanctioned_entity(id).unwrap();
        assert_eq!(s.active_entity_count(), 0);
        assert_eq!(s.get_sanctioned_entity(id).unwrap().resolved_at, Some(2_000));
    }

    #[ink::test]
    fn removing_an_unknown_entity_fails() {
        let (mut s, _) = setup();
        assert_eq!(s.remove_sanctioned_entity(7), Err(Error::EntityNotFound));
    }

    #[ink::test]
    fn a_bulk_update_assigns_sequential_ids_and_counts_every_entity() {
        let (mut s, _) = setup();
        let ids: Vec<u64> = (0..10)
            .map(|n| list_entity(&mut s, &format!("entity-{n}"), SanctionLevel::Restricted))
            .collect();
        assert_eq!(ids, (1..=10).collect::<Vec<u64>>());
        assert_eq!(s.active_entity_count(), 10);
    }

    #[ink::test]
    fn the_list_has_a_capacity_that_frees_up_on_removal() {
        let (mut s, _) = setup();
        s.set_max_sanctioned_entities(2).unwrap();
        let first = list_entity(&mut s, "a", SanctionLevel::Monitored);
        list_entity(&mut s, "b", SanctionLevel::Monitored);
        assert_eq!(
            s.add_sanctioned_entity(b"c".to_vec(), EntityType::Trust, JURISDICTION_A, SanctionLevel::Monitored),
            Err(Error::SanctionListFull)
        );
        s.remove_sanctioned_entity(first).unwrap();
        assert!(s.add_sanctioned_entity(b"c".to_vec(), EntityType::Trust, JURISDICTION_A, SanctionLevel::Monitored).is_ok());
    }

    // ── Screening: blocked vs clean ─────────────────────────────────────

    #[ink::test]
    fn a_listed_entity_in_the_same_jurisdiction_fails_screening() {
        let (mut s, _) = setup();
        let id = list_entity(&mut s, "ACME", SanctionLevel::Prohibited);
        let result = s.screen_property(PROPERTY, JURISDICTION_A, Some(id)).unwrap();
        assert!(!result.passed);
        assert_eq!(result.sanction_level, SanctionLevel::Prohibited);
        assert_eq!(result.entity_id, Some(id));
        assert_eq!(result.property_id, PROPERTY);
    }

    #[ink::test]
    fn an_unlisted_or_removed_entity_passes_screening() {
        let (mut s, _) = setup();
        // Never listed.
        assert!(s.screen_property(PROPERTY, JURISDICTION_A, Some(55)).unwrap().passed);
        // Listed, then removed.
        let id = list_entity(&mut s, "ACME", SanctionLevel::Prohibited);
        s.remove_sanctioned_entity(id).unwrap();
        assert!(s.screen_property(PROPERTY, JURISDICTION_A, Some(id)).unwrap().passed);
        // No entity at all.
        assert!(s.screen_property(PROPERTY, JURISDICTION_A, None).unwrap().passed);
    }

    #[ink::test]
    fn entity_matching_is_jurisdiction_specific() {
        let (mut s, _) = setup();
        let id = list_entity(&mut s, "ACME", SanctionLevel::Prohibited);
        assert!(s.screen_property(PROPERTY, JURISDICTION_B, Some(id)).unwrap().passed);
        assert!(!s.screen_property(PROPERTY, JURISDICTION_A, Some(id)).unwrap().passed);
    }

    #[ink::test]
    fn a_sanctioned_property_fails_until_it_is_cleared() {
        let (mut s, _) = setup();
        s.add_sanctioned_property(PROPERTY, JURISDICTION_A, SanctionLevel::Restricted, b"court order".to_vec()).unwrap();
        let blocked = s.screen_property(PROPERTY, JURISDICTION_A, None).unwrap();
        assert!(!blocked.passed);
        assert_eq!(blocked.sanction_level, SanctionLevel::Restricted);

        s.clear_sanctioned_property(PROPERTY).unwrap();
        assert!(s.screen_property(PROPERTY, JURISDICTION_A, None).unwrap().passed);
        assert!(!s.get_sanctioned_property(PROPERTY).unwrap().active);
    }

    #[ink::test]
    fn clearing_an_unknown_property_fails() {
        let (mut s, _) = setup();
        assert_eq!(s.clear_sanctioned_property(9), Err(Error::PropertyNotFound));
    }

    // ── Audit trail ─────────────────────────────────────────────────────

    #[ink::test]
    fn every_screening_is_recorded_and_exportable_in_order() {
        let (mut s, _) = setup();
        assert!(!s.is_property_screened(PROPERTY));
        let id = list_entity(&mut s, "ACME", SanctionLevel::Prohibited);

        let first = s.screen_property(PROPERTY, JURISDICTION_B, Some(id)).unwrap();
        let second = s.screen_property(PROPERTY, JURISDICTION_A, Some(id)).unwrap();
        assert!(s.is_property_screened(PROPERTY));

        let trail = s.get_property_screenings(PROPERTY);
        assert_eq!(trail.len(), 2);
        assert_eq!(trail[0].screening_id, first.screening_id);
        assert_eq!(trail[1].screening_id, second.screening_id);
        assert!(trail[0].passed && !trail[1].passed);
        assert_eq!(s.get_screening_result(second.screening_id).unwrap(), second);
        assert!(s.get_screening_result(999).is_none());
        assert!(s.get_property_screenings(7).is_empty());
    }

    #[ink::test]
    fn the_per_property_screening_cap_is_enforced() {
        let (mut s, _) = setup();
        s.set_max_screenings_per_property(2).unwrap();
        s.screen_property(PROPERTY, JURISDICTION_A, None).unwrap();
        s.screen_property(PROPERTY, JURISDICTION_A, None).unwrap();
        assert_eq!(
            s.screen_property(PROPERTY, JURISDICTION_A, None),
            Err(Error::ThresholdExceeded)
        );
        // Other properties are unaffected.
        assert!(s.screen_property(PROPERTY + 1, JURISDICTION_A, None).is_ok());
        assert_eq!(s.get_property_screenings(PROPERTY).len(), 2);
    }

    // ── Admin-only access ───────────────────────────────────────────────

    #[ink::test]
    fn sanctions_management_is_restricted_to_the_admin() {
        let (mut s, accounts) = setup();
        let id = list_entity(&mut s, "ACME", SanctionLevel::Prohibited);
        s.add_sanctioned_property(PROPERTY, JURISDICTION_A, SanctionLevel::Restricted, vec![]).unwrap();

        test::set_caller::<DefaultEnvironment>(accounts.bob);
        assert_eq!(
            s.add_sanctioned_entity(b"x".to_vec(), EntityType::Individual, JURISDICTION_A, SanctionLevel::Monitored),
            Err(Error::NotAuthorized)
        );
        assert_eq!(s.remove_sanctioned_entity(id), Err(Error::NotAuthorized));
        assert_eq!(
            s.add_sanctioned_property(1, JURISDICTION_A, SanctionLevel::Monitored, vec![]),
            Err(Error::NotAuthorized)
        );
        assert_eq!(s.clear_sanctioned_property(PROPERTY), Err(Error::NotAuthorized));
        assert_eq!(s.set_screening_threshold(5), Err(Error::NotAuthorized));
        assert_eq!(s.set_max_sanctioned_entities(1), Err(Error::NotAuthorized));
        assert_eq!(s.set_max_screenings_per_property(1), Err(Error::NotAuthorized));

        // Nothing changed.
        assert!(s.get_sanctioned_entity(id).unwrap().active);
        assert!(s.get_sanctioned_property(PROPERTY).unwrap().active);
        assert_eq!(s.screening_threshold(), 90);
    }

    #[ink::test]
    fn the_admin_can_tune_thresholds() {
        let (mut s, _) = setup();
        assert_eq!(s.screening_threshold(), 90);
        s.set_screening_threshold(30).unwrap();
        assert_eq!(s.screening_threshold(), 30);
        assert_eq!(s.max_sanctioned_entities(), 10_000);
        assert_eq!(s.max_screenings_per_property(), 1_000);
    }

    #[ink::test]
    fn anyone_can_screen() {
        let (mut s, accounts) = setup();
        test::set_caller::<DefaultEnvironment>(accounts.eve);
        assert!(s.screen_property(PROPERTY, JURISDICTION_A, None).unwrap().passed);
    }
}
