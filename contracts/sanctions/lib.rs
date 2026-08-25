#![allow(clippy::clone_on_copy)] // fires inside ink! generated storage code
#![cfg_attr(not(feature = "std"), no_std, no_main)]
#![allow(
    clippy::needless_borrows_for_generic_args,
    clippy::too_many_arguments,
    clippy::upper_case_acronyms
)]

#[ink::contract]
pub mod sanctions_screening {
    use ink::prelude::vec::Vec;
    use ink::storage::Mapping;

    // ── Errors ──────────────────────────────────────────────────────────────

    #[derive(Debug, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub enum Error {
        NotAuthorized,
        EntityNotFound,
        PropertyNotFound,
        AlreadyScreened,
        ScreeningNotFound,
        SanctionListFull,
        InvalidJurisdiction,
        ThresholdExceeded,
    }

    pub type Result<T> = core::result::Result<T, Error>;

    // ── Types ───────────────────────────────────────────────────────────────

    #[derive(Debug, Clone, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(
        feature = "std",
        derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
    )]
    pub enum EntityType {
        Individual,
        Corporation,
        Trust,
        Partnership,
        Other,
    }

    #[derive(Debug, Clone, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(
        feature = "std",
        derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
    )]
    pub enum SanctionLevel {
        None,
        Monitored,
        Restricted,
        Prohibited,
    }

    #[derive(Debug, Clone, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(
        feature = "std",
        derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
    )]
    pub struct SanctionedEntity {
        pub entity_id: u64,
        pub entity_name: Vec<u8>,
        pub entity_type: EntityType,
        pub jurisdiction_code: u32,
        pub sanction_level: SanctionLevel,
        pub listed_at: u64,
        pub resolved_at: Option<u64>,
        pub active: bool,
    }

    #[derive(Debug, Clone, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(
        feature = "std",
        derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
    )]
    pub struct SanctionedProperty {
        pub property_id: u64,
        pub jurisdiction_code: u32,
        pub sanction_level: SanctionLevel,
        pub listed_at: u64,
        pub notes: Vec<u8>,
        pub active: bool,
    }

    #[derive(Debug, Clone, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(
        feature = "std",
        derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
    )]
    pub struct ScreeningResult {
        pub screening_id: u64,
        pub property_id: u64,
        pub entity_id: Option<u64>,
        pub jurisdiction_code: u32,
        pub sanction_level: SanctionLevel,
        pub screened_at: u64,
        pub passed: bool,
    }

    // ── Events ──────────────────────────────────────────────────────────────

    #[ink(event)]
    pub struct EntitySanctioned {
        #[ink(topic)]
        entity_id: u64,
        sanction_level: SanctionLevel,
        timestamp: u64,
    }

    #[ink(event)]
    pub struct EntityRemovedFromSanctions {
        #[ink(topic)]
        entity_id: u64,
        timestamp: u64,
    }

    #[ink(event)]
    pub struct PropertySanctioned {
        #[ink(topic)]
        property_id: u64,
        sanction_level: SanctionLevel,
        timestamp: u64,
    }

    #[ink(event)]
    pub struct PropertyCleared {
        #[ink(topic)]
        property_id: u64,
        timestamp: u64,
    }

    #[ink(event)]
    pub struct ScreeningsPerformed {
        #[ink(topic)]
        property_id: u64,
        result_count: u32,
        passed: bool,
        timestamp: u64,
    }

    #[ink(event)]
    pub struct SanctionThresholdUpdated {
        threshold: u32,
        timestamp: u64,
    }

    // ── Storage ─────────────────────────────────────────────────────────────

    #[ink(storage)]
    pub struct SanctionsScreening {
        admin: AccountId,
        sanctioned_entities: Mapping<u64, SanctionedEntity>,
        sanctioned_properties: Mapping<u64, SanctionedProperty>,
        screening_results: Mapping<u64, ScreeningResult>,
        property_screenings: Mapping<u64, Vec<u64>>,
        next_entity_id: u64,
        next_screening_id: u64,
        max_sanctioned_entities: u32,
        /// Number of currently active (non-soft-deleted) sanctioned entities.
        active_entity_count: u32,
        screening_threshold_days: u32,
    }

    impl SanctionsScreening {
        #[ink(constructor)]
        pub fn new() -> Self {
            Self {
                admin: Self::env().caller(),
                sanctioned_entities: Mapping::default(),
                sanctioned_properties: Mapping::default(),
                screening_results: Mapping::default(),
                property_screenings: Mapping::default(),
                next_entity_id: 1,
                next_screening_id: 1,
                max_sanctioned_entities: 10_000,
                active_entity_count: 0,
                screening_threshold_days: 90,
            }
        }

        fn ensure_admin(&self) -> Result<()> {
            if self.env().caller() != self.admin {
                return Err(Error::NotAuthorized);
            }
            Ok(())
        }

        // ── Admin: Manage sanctioned entities ───────────────────────────────

        /// Lists a new sanctioned entity.
        ///
        /// Admin only. Fails with [`Error::SanctionListFull`] when the number
        /// of active entities has reached the configured cap (see
        /// [`max_sanctioned_entities`](Self::max_sanctioned_entities));
        /// soft-removing an entity frees its slot again.
        #[ink(message)]
        pub fn add_sanctioned_entity(
            &mut self,
            entity_name: Vec<u8>,
            entity_type: EntityType,
            jurisdiction_code: u32,
            sanction_level: SanctionLevel,
        ) -> Result<u64> {
            self.ensure_admin()?;
            if self.active_entity_count >= self.max_sanctioned_entities {
                return Err(Error::SanctionListFull);
            }
            let entity_id = self.next_entity_id;
            self.next_entity_id = entity_id.checked_add(1).ok_or(Error::SanctionListFull)?;

            let now = self.env().block_timestamp();
            let entity = SanctionedEntity {
                entity_id,
                entity_name,
                entity_type,
                jurisdiction_code,
                sanction_level: sanction_level.clone(),
                listed_at: now,
                resolved_at: None,
                active: true,
            };
            self.sanctioned_entities.insert(entity_id, &entity);
            self.active_entity_count = self
                .active_entity_count
                .checked_add(1)
                .ok_or(Error::SanctionListFull)?;
            self.env().emit_event(EntitySanctioned {
                entity_id,
                sanction_level,
                timestamp: now,
            });
            Ok(entity_id)
        }

        /// Soft-removes (resolves) a sanctioned entity, freeing a list slot.
        ///
        /// Admin only. The historical record is kept for audit purposes but
        /// marked inactive; the freed slot allows adding new entities even
        /// when the list was previously full.
        #[ink(message)]
        pub fn remove_sanctioned_entity(&mut self, entity_id: u64) -> Result<()> {
            self.ensure_admin()?;
            let mut entity = self
                .sanctioned_entities
                .get(entity_id)
                .ok_or(Error::EntityNotFound)?;
            if entity.active {
                entity.active = false;
                entity.resolved_at = Some(self.env().block_timestamp());
                self.sanctioned_entities.insert(entity_id, &entity);
                self.active_entity_count = self.active_entity_count.saturating_sub(1);
                self.env().emit_event(EntityRemovedFromSanctions {
                    entity_id,
                    timestamp: self.env().block_timestamp(),
                });
            }
            Ok(())
        }

        #[ink(message)]
        pub fn get_sanctioned_entity(&self, entity_id: u64) -> Option<SanctionedEntity> {
            self.sanctioned_entities.get(entity_id)
        }

        // ── Admin: Manage sanctioned properties ─────────────────────────────

        #[ink(message)]
        pub fn add_sanctioned_property(
            &mut self,
            property_id: u64,
            jurisdiction_code: u32,
            sanction_level: SanctionLevel,
            notes: Vec<u8>,
        ) -> Result<()> {
            self.ensure_admin()?;
            let now = self.env().block_timestamp();
            let entry = SanctionedProperty {
                property_id,
                jurisdiction_code,
                sanction_level: sanction_level.clone(),
                listed_at: now,
                notes,
                active: true,
            };
            self.sanctioned_properties.insert(property_id, &entry);
            self.env().emit_event(PropertySanctioned {
                property_id,
                sanction_level,
                timestamp: now,
            });
            Ok(())
        }

        #[ink(message)]
        pub fn clear_sanctioned_property(&mut self, property_id: u64) -> Result<()> {
            self.ensure_admin()?;
            let mut entry = self
                .sanctioned_properties
                .get(property_id)
                .ok_or(Error::PropertyNotFound)?;
            entry.active = false;
            self.sanctioned_properties.insert(property_id, &entry);
            self.env().emit_event(PropertyCleared {
                property_id,
                timestamp: self.env().block_timestamp(),
            });
            Ok(())
        }

        #[ink(message)]
        pub fn get_sanctioned_property(&self, property_id: u64) -> Option<SanctionedProperty> {
            self.sanctioned_properties.get(property_id)
        }

        // ── Screening ───────────────────────────────────────────────────────

        #[ink(message)]
        pub fn screen_property(
            &mut self,
            property_id: u64,
            jurisdiction_code: u32,
            entity_id: Option<u64>,
        ) -> Result<ScreeningResult> {
            self.ensure_admin()?;

            // Check if property itself is sanctioned
            if let Some(prop) = self.sanctioned_properties.get(property_id) {
                if prop.active {
                    let screening_id = self.next_screening_id;
                    self.next_screening_id = screening_id
                        .checked_add(1)
                        .ok_or(Error::ThresholdExceeded)?;
                    let now = self.env().block_timestamp();
                    let result = ScreeningResult {
                        screening_id,
                        property_id,
                        entity_id,
                        jurisdiction_code,
                        sanction_level: prop.sanction_level.clone(),
                        screened_at: now,
                        passed: false,
                    };
                    self.screening_results.insert(screening_id, &result);
                    self.record_screening(property_id, screening_id);
                    self.env().emit_event(ScreeningsPerformed {
                        property_id,
                        result_count: 1,
                        passed: false,
                        timestamp: now,
                    });
                    return Ok(result);
                }
            }

            // Check entity if provided
            if let Some(eid) = entity_id {
                if let Some(entity) = self.sanctioned_entities.get(eid) {
                    if entity.active && entity.jurisdiction_code == jurisdiction_code {
                        let screening_id = self.next_screening_id;
                        self.next_screening_id = screening_id
                            .checked_add(1)
                            .ok_or(Error::ThresholdExceeded)?;
                        let now = self.env().block_timestamp();
                        let result = ScreeningResult {
                            screening_id,
                            property_id,
                            entity_id: Some(eid),
                            jurisdiction_code,
                            sanction_level: entity.sanction_level.clone(),
                            screened_at: now,
                            passed: entity.sanction_level == SanctionLevel::None,
                        };
                        self.screening_results.insert(screening_id, &result);
                        self.record_screening(property_id, screening_id);
                        self.env().emit_event(ScreeningsPerformed {
                            property_id,
                            result_count: 1,
                            passed: result.passed,
                            timestamp: now,
                        });
                        return Ok(result);
                    }
                }
            }

            // No match found — property passes screening
            let screening_id = self.next_screening_id;
            self.next_screening_id = screening_id
                .checked_add(1)
                .ok_or(Error::ThresholdExceeded)?;
            let now = self.env().block_timestamp();
            let result = ScreeningResult {
                screening_id,
                property_id,
                entity_id,
                jurisdiction_code,
                sanction_level: SanctionLevel::None,
                screened_at: now,
                passed: true,
            };
            self.screening_results.insert(screening_id, &result);
            self.record_screening(property_id, screening_id);
            self.env().emit_event(ScreeningsPerformed {
                property_id,
                result_count: 1,
                passed: true,
                timestamp: now,
            });
            Ok(result)
        }

        fn record_screening(&mut self, property_id: u64, screening_id: u64) {
            let mut existing = self
                .property_screenings
                .get(property_id)
                .unwrap_or_default();
            existing.push(screening_id);
            self.property_screenings.insert(property_id, &existing);
        }

        #[ink(message)]
        pub fn get_screening_result(&self, screening_id: u64) -> Option<ScreeningResult> {
            self.screening_results.get(screening_id)
        }

        #[ink(message)]
        pub fn get_property_screenings(&self, property_id: u64) -> Vec<ScreeningResult> {
            match self.property_screenings.get(property_id) {
                Some(ids) => {
                    let mut results = Vec::new();
                    for id in ids {
                        if let Some(r) = self.screening_results.get(id) {
                            results.push(r);
                        }
                    }
                    results
                }
                None => Vec::new(),
            }
        }

        #[ink(message)]
        pub fn is_property_screened(&self, property_id: u64) -> bool {
            self.property_screenings.get(property_id).is_some()
        }

        #[ink(message)]
        pub fn admin(&self) -> AccountId {
            self.admin
        }

        #[ink(message)]
        pub fn set_screening_threshold(&mut self, days: u32) -> Result<()> {
            self.ensure_admin()?;
            self.screening_threshold_days = days;
            self.env().emit_event(SanctionThresholdUpdated {
                threshold: days,
                timestamp: self.env().block_timestamp(),
            });
            Ok(())
        }

        #[ink(message)]
        pub fn screening_threshold(&self) -> u32 {
            self.screening_threshold_days
        }

        /// Updates the maximum number of concurrently active sanctioned
        /// entities (admin only).
        ///
        /// The cap starts at `10_000` from the constructor. Lowering it below
        /// the current active count is allowed but blocks further additions
        /// until entities are removed or the cap is raised again.
        #[ink(message)]
        pub fn set_max_sanctioned_entities(&mut self, max: u32) -> Result<()> {
            self.ensure_admin()?;
            self.max_sanctioned_entities = max;
            Ok(())
        }

        /// Returns the maximum number of concurrently active sanctioned
        /// entities.
        #[ink(message)]
        pub fn max_sanctioned_entities(&self) -> u32 {
            self.max_sanctioned_entities
        }

        /// Returns the number of currently active sanctioned entities.
        #[ink(message)]
        pub fn active_entity_count(&self) -> u32 {
            self.active_entity_count
        }
    }

    impl Default for SanctionsScreening {
        fn default() -> Self {
            Self::new()
        }
    }

    // ── Tests ──────────────────────────────────────────────────────────────

    #[cfg(test)]
    mod tests {
        use super::*;

        fn default_contract() -> SanctionsScreening {
            SanctionsScreening::new()
        }

        #[ink::test]
        fn test_admin_is_caller() {
            let contract = default_contract();
            assert_eq!(contract.admin(), AccountId::from([0x01; 32]));
        }

        #[ink::test]
        fn test_add_sanctioned_entity() {
            let mut contract = default_contract();
            let id = contract
                .add_sanctioned_entity(
                    b"Bad Actor Corp".to_vec(),
                    EntityType::Corporation,
                    1001,
                    SanctionLevel::Prohibited,
                )
                .expect("should add entity");
            assert_eq!(id, 1);
            let entity = contract.get_sanctioned_entity(id).expect("should exist");
            assert_eq!(entity.entity_type, EntityType::Corporation);
            assert!(entity.active);
        }

        #[ink::test]
        fn test_remove_sanctioned_entity() {
            let mut contract = default_contract();
            let id = contract
                .add_sanctioned_entity(
                    b"Bad Entity".to_vec(),
                    EntityType::Individual,
                    1001,
                    SanctionLevel::Restricted,
                )
                .expect("add");
            contract.remove_sanctioned_entity(id).expect("remove");
            let entity = contract.get_sanctioned_entity(id).expect("should exist");
            assert!(!entity.active);
        }

        #[ink::test]
        fn test_screen_property_clean() {
            let mut contract = default_contract();
            let result = contract.screen_property(42, 1001, None).expect("screen");
            assert!(result.passed);
            assert_eq!(result.sanction_level, SanctionLevel::None);
        }

        #[ink::test]
        fn test_screen_property_sanctioned() {
            let mut contract = default_contract();
            contract
                .add_sanctioned_property(42, 1001, SanctionLevel::Prohibited, b"OFAC list".to_vec())
                .expect("sanction");
            let result = contract.screen_property(42, 1001, None).expect("screen");
            assert!(!result.passed);
            assert_eq!(result.sanction_level, SanctionLevel::Prohibited);
        }

        #[ink::test]
        fn test_screen_property_with_entity() {
            let mut contract = default_contract();
            let eid = contract
                .add_sanctioned_entity(
                    b"Restricted Entity".to_vec(),
                    EntityType::Corporation,
                    1001,
                    SanctionLevel::Restricted,
                )
                .expect("add entity");
            let result = contract
                .screen_property(42, 1001, Some(eid))
                .expect("screen");
            assert!(!result.passed);
            assert_eq!(result.sanction_level, SanctionLevel::Restricted);
        }

        #[ink::test]
        fn test_get_property_screenings() {
            let mut contract = default_contract();
            contract.screen_property(1, 1001, None).expect("screen");
            contract.screen_property(1, 1002, None).expect("screen");
            let results = contract.get_property_screenings(1);
            assert_eq!(results.len(), 2);
        }

        #[ink::test]
        fn test_is_property_screened() {
            let mut contract = default_contract();
            assert!(!contract.is_property_screened(99));
            contract.screen_property(99, 1001, None).expect("screen");
            assert!(contract.is_property_screened(99));
        }

        #[ink::test]
        fn test_screen_property_unknown_jurisdiction_still_passes() {
            let mut contract = default_contract();
            let result = contract.screen_property(50, 9999, None).expect("screen");
            assert!(result.passed);
            assert_eq!(result.sanction_level, SanctionLevel::None);
        }

        // ── Screening-threshold tests (Issue #1020) ────────────────────────────

        #[ink::test]
        fn test_non_admin_cannot_set_screening_threshold() {
            let mut contract = default_contract();
            let accounts = ink::env::test::default_accounts::<ink::env::DefaultEnvironment>();

            ink::env::test::set_caller::<ink::env::DefaultEnvironment>(accounts.bob);
            let result = contract.set_screening_threshold(30);
            assert_eq!(result, Err(Error::NotAuthorized));

            // Threshold must be unchanged after the rejected update
            assert_eq!(contract.screening_threshold(), 90);
        }

        #[ink::test]
        fn test_admin_sets_screening_threshold() {
            let mut contract = default_contract();

            // Initial value is 90 days
            assert_eq!(contract.screening_threshold(), 90);

            contract.set_screening_threshold(30).expect("admin set");
            assert_eq!(contract.screening_threshold(), 30);
        }

        #[ink::test]
        fn test_screening_threshold_round_trips() {
            let mut contract = default_contract();

            contract.set_screening_threshold(30).expect("set 30");
            assert_eq!(contract.screening_threshold(), 30);

            // A second update overwrites the stored value
            contract.set_screening_threshold(45).expect("set 45");
            assert_eq!(contract.screening_threshold(), 45);

            // Zero is accepted as a plain value (no special-casing)
            contract.set_screening_threshold(0).expect("set 0");
            assert_eq!(contract.screening_threshold(), 0);
        }

        #[ink::test]
        fn test_threshold_update_emits_sanction_threshold_updated_event() {
            use scale::Decode as _;

            let mut contract = default_contract();
            contract.set_screening_threshold(30).expect("admin set");

            let events = ink::env::test::recorded_events().collect::<Vec<_>>();
            assert_eq!(events.len(), 1);

            let decoded =
                SanctionThresholdUpdated::decode(&mut &events[0].data[..]).expect("decode event");
            assert_eq!(decoded.threshold, 30);
        }

        #[ink::test]
        fn test_add_entity_at_exact_cap_succeeds_next_fails() {
            let mut contract = default_contract();
            contract.set_max_sanctioned_entities(2).expect("set cap");

            let _first = contract
                .add_sanctioned_entity(
                    b"One".to_vec(),
                    EntityType::Individual,
                    1,
                    SanctionLevel::Monitored,
                )
                .expect("first at boundary");
            assert_eq!(contract.active_entity_count(), 1);

            let _second = contract
                .add_sanctioned_entity(
                    b"Two".to_vec(),
                    EntityType::Individual,
                    2,
                    SanctionLevel::Monitored,
                )
                .expect("second fills cap exactly");
            assert_eq!(contract.active_entity_count(), 2);

            let overflow = contract.add_sanctioned_entity(
                b"Three".to_vec(),
                EntityType::Individual,
                3,
                SanctionLevel::Monitored,
            );
            assert_eq!(overflow, Err(Error::SanctionListFull));
        }

        #[ink::test]
        fn test_remove_frees_cap_slot() {
            let mut contract = default_contract();
            contract.set_max_sanctioned_entities(1).expect("set cap");

            let id = contract
                .add_sanctioned_entity(
                    b"Full".to_vec(),
                    EntityType::Corporation,
                    7,
                    SanctionLevel::Prohibited,
                )
                .expect("fills cap");

            // Cap is full.
            assert_eq!(
                contract.add_sanctioned_entity(
                    b"No Room".to_vec(),
                    EntityType::Trust,
                    8,
                    SanctionLevel::Restricted
                ),
                Err(Error::SanctionListFull)
            );

            // Soft removal frees the slot for a subsequent add.
            contract.remove_sanctioned_entity(id).expect("remove");
            let replacement = contract
                .add_sanctioned_entity(
                    b"Frees Slot".to_vec(),
                    EntityType::Trust,
                    8,
                    SanctionLevel::Restricted,
                )
                .expect("slot freed");
            assert_ne!(replacement, id);
        }

        #[ink::test]
        fn test_double_remove_counts_once_and_keeps_count_consistent() {
            let mut contract = default_contract();
            contract.set_max_sanctioned_entities(1).expect("set cap");
            let id = contract
                .add_sanctioned_entity(
                    b"Once".to_vec(),
                    EntityType::Individual,
                    9,
                    SanctionLevel::Monitored,
                )
                .expect("add");

            contract.remove_sanctioned_entity(id).expect("first remove");
            contract
                .remove_sanctioned_entity(id)
                .expect("idempotent remove");

            assert_eq!(contract.active_entity_count(), 0);

            // Removing an unknown entity is still an error.
            assert_eq!(
                contract.remove_sanctioned_entity(999),
                Err(Error::EntityNotFound)
            );
        }

        #[ink::test]
        fn test_default_cap_is_ten_thousand() {
            let contract = default_contract();
            assert_eq!(contract.max_sanctioned_entities(), 10_000);
        }
    }
}
