// Unit tests for the database contract (Issue #101 - extracted from lib.rs)

#[cfg(test)]
mod tests {
    use super::*;

    #[ink::test]
    fn new_initializes_correctly() {
        let contract = DatabaseIntegration::new();
        assert_eq!(contract.total_syncs(), 0);
        assert_eq!(contract.latest_snapshot_id(), 0);
    }

    #[ink::test]
    fn emit_sync_event_works() {
        let mut contract = DatabaseIntegration::new();
        let result = contract.emit_sync_event(DataType::Properties, Hash::from([0x01; 32]), 10);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 1);
        assert_eq!(contract.total_syncs(), 1);

        let record = contract.get_sync_record(1).unwrap();
        assert_eq!(record.data_type, DataType::Properties);
        assert_eq!(record.record_count, 10);
        assert_eq!(record.status, SyncStatus::Initiated);
    }

    #[ink::test]
    fn analytics_snapshot_works() {
        let mut contract = DatabaseIntegration::new();
        let result = contract.record_analytics_snapshot(
            100,
            50,
            20,
            10_000_000,
            100_000,
            30,
            Hash::from([0x02; 32]),
        );
        assert!(result.is_ok());

        let snapshot = contract.get_analytics_snapshot(1).unwrap();
        assert_eq!(snapshot.total_properties, 100);
        assert_eq!(snapshot.total_valuation, 10_000_000);
    }

    #[ink::test]
    fn data_export_works() {
        let mut contract = DatabaseIntegration::new();
        let result = contract.request_data_export(DataType::Properties, 1, 100, 0, 1000);
        assert!(result.is_ok());

        let batch_id = result.unwrap();
        let request = contract.get_export_request(batch_id).unwrap();
        assert!(!request.completed);

        let complete_result = contract.complete_data_export(batch_id, Hash::from([0x03; 32]));
        assert!(complete_result.is_ok());

        let completed = contract.get_export_request(batch_id).unwrap();
        assert!(completed.completed);
    }

    #[ink::test]
    fn verify_sync_works() {
        let mut contract = DatabaseIntegration::new();
        let checksum = Hash::from([0x01; 32]);
        contract
            .emit_sync_event(DataType::Transfers, checksum, 5)
            .unwrap();

        let result = contract.verify_sync(1, checksum);
        assert_eq!(result, Ok(true));

        let record = contract.get_sync_record(1).unwrap();
        assert_eq!(record.status, SyncStatus::Verified);
    }

    #[ink::test]
    fn indexer_registration_works() {
        let mut contract = DatabaseIntegration::new();
        let indexer = AccountId::from([0x02; 32]);

        let result = contract.register_indexer(indexer, String::from("TestIndexer"));
        assert!(result.is_ok());

        let info = contract.get_indexer(indexer).unwrap();
        assert_eq!(info.name, "TestIndexer");
        assert!(info.is_active);

        let list = contract.get_indexer_list();
        assert_eq!(list.len(), 1);
    }

    // ========================================================================
    // Indexer registry lifecycle (Issue #1192)
    // ========================================================================

    /// Advances the chain clock so staleness can be exercised.
    ///
    /// `ink::env::test::set_block_timestamp` moves time forward for the
    /// contract under test, which is what the staleness window reads.
    fn advance_time(seconds: u64) {
        let now = ink::env::block_timestamp::<ink::env::DefaultEnvironment>();
        ink::env::test::set_block_timestamp::<ink::env::DefaultEnvironment>(now + seconds);
    }

    fn indexer_account(seed: u8) -> AccountId {
        AccountId::from([seed; 32])
    }

    /// A freshly registered indexer counts as active.
    #[ink::test]
    fn registered_indexer_is_active() {
        let contract = new_admin_contract();
        let indexer = indexer_account(0x11);

        assert_eq!(
            contract.register_indexer(indexer, String::from("Fresh")),
            Ok(())
        );

        assert_eq!(contract.get_active_indexers(), vec![indexer]);
    }

    /// A heartbeat inside the window keeps an indexer in the active set.
    #[ink::test]
    fn heartbeat_keeps_indexer_active() {
        let mut contract = new_admin_contract();
        let indexer = indexer_account(0x12);

        contract
            .register_indexer(indexer, String::from("Alive"))
            .unwrap();

        // Most of the way to the window, then a heartbeat.
        let window = contract.indexer_stale_window();
        advance_time(window - 1);

        let sync_id = contract
            .emit_sync_event(DataType::Properties, Hash::from([0x01; 32]), 1)
            .unwrap();

        ink::env::test::set_caller::<ink::env::DefaultEnvironment>(indexer);
        assert_eq!(contract.confirm_sync(sync_id), Ok(()));
        ink::env::test::set_caller::<ink::env::DefaultEnvironment>(
            ink::env::test::default_accounts::<ink::env::DefaultEnvironment>().alice,
        );

        let info = contract.get_indexer(indexer).unwrap();
        assert!(info.is_active);
        assert_eq!(contract.get_active_indexers(), vec![indexer]);
    }

    /// An indexer that goes quiet past the window drops out of the active set.
    ///
    /// This is the core of the issue: a burnt-out indexer must stop being
    /// counted as healthy.
    #[ink::test]
    fn stale_indexer_leaves_active_set() {
        let mut contract = new_admin_contract();
        let indexer = indexer_account(0x13);

        contract
            .register_indexer(indexer, String::from("Doomed"))
            .unwrap();
        assert_eq!(contract.get_active_indexers(), vec![indexer]);

        // One second past the window.
        advance_time(contract.indexer_stale_window() + 1);

        // Excluded on read, with no sweep required.
        assert!(contract.get_active_indexers().is_empty());
        // Still present in the raw registry listing, which is the distinction
        // the two getters exist to make.
        assert_eq!(contract.get_indexer_list(), vec![indexer]);
    }

    /// The boundary is inclusive: exactly at the window is still live.
    #[ink::test]
    fn indexer_at_exactly_the_window_is_still_active() {
        let mut contract = new_admin_contract();
        let indexer = indexer_account(0x14);

        contract
            .register_indexer(indexer, String::from("Edge"))
            .unwrap();

        advance_time(contract.indexer_stale_window());

        assert_eq!(contract.get_active_indexers(), vec![indexer]);
    }

    /// Sweeping persists the demotion and reclaims the slot.
    #[ink::test]
    fn prune_removes_stale_indexers() {
        let mut contract = new_admin_contract();
        let live = indexer_account(0x15);
        let dead = indexer_account(0x16);

        contract.register_indexer(live, String::from("Live")).unwrap();
        contract.register_indexer(dead, String::from("Dead")).unwrap();

        // Keep `live` fresh, then let both age out.
        advance_time(contract.indexer_stale_window() / 2);
        let sync_id = contract
            .emit_sync_event(DataType::Properties, Hash::from([0x02; 32]), 1)
            .unwrap();
        ink::env::test::set_caller::<ink::env::DefaultEnvironment>(live);
        contract.confirm_sync(sync_id).unwrap();
        ink::env::test::set_caller::<ink::env::DefaultEnvironment>(
            ink::env::test::default_accounts::<ink::env::DefaultEnvironment>().alice,
        );

        advance_time(contract.indexer_stale_window());

        // `dead` is now stale, `live` is not.
        assert_eq!(contract.prune_stale_indexers(), Ok(1));
        assert_eq!(contract.get_indexer_list(), vec![live]);
        assert_eq!(contract.get_active_indexers(), vec![live]);

        let dead_info = contract.get_indexer(dead).unwrap();
        assert!(!dead_info.is_active, "pruned indexer must be demoted");
    }

    /// A deactivated indexer is out of the active set and out of the list.
    #[ink::test]
    fn deactivation_removes_from_list_and_frees_slot() {
        let mut contract = new_admin_contract();
        let indexer = indexer_account(0x17);

        contract
            .register_indexer(indexer, String::from("Doomed"))
            .unwrap();

        assert_eq!(contract.deactivate_indexer(indexer), Ok(()));
        assert!(contract.get_active_indexers().is_empty());
        assert!(contract.get_indexer_list().is_empty());
        // The record survives for lookups.
        assert!(!contract.get_indexer(indexer).unwrap().is_active);

        // Deactivating twice is an error, not a silent success.
        assert_eq!(
            contract.deactivate_indexer(indexer),
            Err(Error::IndexerInactive)
        );
    }

    /// A deactivated indexer cannot confirm syncs or refresh its heartbeat.
    #[ink::test]
    fn deactivated_indexer_cannot_confirm_sync() {
        let mut contract = new_admin_contract();
        let indexer = indexer_account(0x18);

        contract
            .register_indexer(indexer, String::from("Doomed"))
            .unwrap();

        let sync_id = contract
            .emit_sync_event(DataType::Properties, Hash::from([0x03; 32]), 1)
            .unwrap();

        contract.deactivate_indexer(indexer).unwrap();

        ink::env::test::set_caller::<ink::env::DefaultEnvironment>(indexer);
        assert_eq!(contract.confirm_sync(sync_id), Err(Error::IndexerInactive));
    }

    /// Reactivation returns an indexer to the live set.
    #[ink::test]
    fn reactivation_restores_indexer() {
        let mut contract = new_admin_contract();
        let indexer = indexer_account(0x19);

        contract
            .register_indexer(indexer, String::from("Paused"))
            .unwrap();
        contract.deactivate_indexer(indexer).unwrap();
        assert!(contract.get_active_indexers().is_empty());

        assert_eq!(contract.reactivate_indexer(indexer), Ok(()));
        assert_eq!(contract.get_active_indexers(), vec![indexer]);
        assert_eq!(contract.get_indexer_list(), vec![indexer]);

        // Reactivating a live indexer is rejected.
        assert_eq!(
            contract.reactivate_indexer(indexer),
            Err(Error::IndexerAlreadyRegistered)
        );
    }

    /// The registry refuses to grow past `MAX_INDEXERS`.
    ///
    /// Also pins down the documented limit so the constant and the behaviour
    /// cannot drift apart silently.
    #[ink::test]
    fn registration_is_capped() {
        let mut contract = new_admin_contract();
        let cap = contract.max_indexers();

        for i in 0..cap {
            assert_eq!(
                contract.register_indexer(indexer_account(0x20 + i as u8), String::from("Filler")),
                Ok(()),
                "registration {i} should succeed"
            );
        }

        assert_eq!(contract.get_indexer_list().len() as u32, cap);

        // One past the cap.
        assert_eq!(
            contract.register_indexer(indexer_account(0x80), String::from("OneTooMany")),
            Err(Error::IndexerLimitReached)
        );
        assert_eq!(contract.get_indexer_list().len() as u32, cap);
    }

    /// Lifecycle management is admin-only.
    #[ink::test]
    fn indexer_lifecycle_is_admin_only() {
        let accounts = ink::env::test::default_accounts::<ink::env::DefaultEnvironment>();
        let mut contract = new_admin_contract();
        let indexer = indexer_account(0x1A);

        contract
            .register_indexer(indexer, String::from("Owned"))
            .unwrap();

        ink::env::test::set_caller::<ink::env::DefaultEnvironment>(accounts.bob);
        assert_eq!(
            contract.deactivate_indexer(indexer),
            Err(Error::Unauthorized)
        );
        assert_eq!(contract.reactivate_indexer(indexer), Err(Error::Unauthorized));
        assert_eq!(contract.prune_stale_indexers(), Err(Error::Unauthorized));
    }

    // ========================================================================
    // Export-request lifecycle (Issue #1016)
    // ========================================================================

    fn new_admin_contract() -> DatabaseIntegration {
        let accounts = ink::env::test::default_accounts::<ink::env::DefaultEnvironment>();
        ink::env::test::set_caller::<ink::env::DefaultEnvironment>(accounts.alice);
        DatabaseIntegration::new()
    }

    #[ink::test]
    fn request_data_export_is_admin_only() {
        let accounts = ink::env::test::default_accounts::<ink::env::DefaultEnvironment>();
        let mut contract = new_admin_contract();

        // Non-admin is rejected
        ink::env::test::set_caller::<ink::env::DefaultEnvironment>(accounts.bob);
        let forbidden = contract.request_data_export(DataType::Properties, 1, 100, 0, 1000);
        assert_eq!(forbidden, Err(Error::Unauthorized));

        // Admin succeeds
        ink::env::test::set_caller::<ink::env::DefaultEnvironment>(accounts.alice);
        let batch = contract
            .request_data_export(DataType::Properties, 1, 100, 0, 1000)
            .expect("Admin should request an export");
        assert_eq!(batch, 1);
    }

    #[ink::test]
    fn inverted_id_range_with_equal_blocks_is_rejected() {
        let mut contract = new_admin_contract();
        let result = contract.request_data_export(DataType::Transfers, 100, 1, 500, 500);
        assert_eq!(result, Err(Error::InvalidDataRange));
    }

    #[ink::test]
    fn inverted_block_range_with_equal_ids_is_rejected() {
        let mut contract = new_admin_contract();
        let result = contract.request_data_export(DataType::Escrows, 42, 42, 900, 100);
        assert_eq!(result, Err(Error::InvalidDataRange));
    }

    #[ink::test]
    fn fully_inverted_ranges_are_rejected() {
        let mut contract = new_admin_contract();
        let result = contract.request_data_export(DataType::Valuations, 100, 1, 900, 100);
        assert_eq!(result, Err(Error::InvalidDataRange));
    }

    #[ink::test]
    fn equal_boundaries_are_accepted_as_valid_range() {
        let mut contract = new_admin_contract();
        let result =
            contract.request_data_export(DataType::Compliance, 7, 7, 1234, 1234);
        assert!(
            result.is_ok(),
            "from == to on both axes must be a valid single-record range"
        );
    }

    #[ink::test]
    fn valid_request_stores_exact_fields_and_increments_batch_ids() {
        let accounts = ink::env::test::default_accounts::<ink::env::DefaultEnvironment>();
        let mut contract = new_admin_contract();

        let first = contract
            .request_data_export(DataType::Properties, 10, 20, 100, 200)
            .expect("first export");
        let second = contract
            .request_data_export(DataType::Tokens, 30, 40, 300, 400)
            .expect("second export");

        assert_eq!(first, 1, "batch ids increment sequentially");
        assert_eq!(second, 2);

        let stored = contract.get_export_request(first).unwrap();
        assert_eq!(stored.batch_id, first);
        assert_eq!(stored.data_type, DataType::Properties);
        assert_eq!(stored.from_id, 10);
        assert_eq!(stored.to_id, 20);
        assert_eq!(stored.from_block, 100);
        assert_eq!(stored.to_block, 200);
        assert_eq!(stored.requested_by, accounts.alice);
        assert!(!stored.completed);
        assert_eq!(stored.export_checksum, None);
    }

    #[ink::test]
    fn completion_sets_completed_flag_and_checksum() {
        let checksum = Hash::from([0xAB; 32]);
        let mut contract = new_admin_contract();

        let batch = contract
            .request_data_export(DataType::Analytics, 1, 9, 5, 50)
            .expect("export request");
        assert!(!contract.get_export_request(batch).unwrap().completed);

        contract
            .complete_data_export(batch, checksum)
            .expect("Admin completes the export");

        let completed = contract.get_export_request(batch).unwrap();
        assert!(completed.completed);
        assert_eq!(completed.export_checksum, Some(checksum));
    }

    #[ink::test]
    fn completing_unknown_batch_returns_export_not_found() {
        let mut contract = new_admin_contract();
        let result = contract.complete_data_export(999, Hash::from([0x01; 32]));
        assert_eq!(result, Err(Error::ExportNotFound));
    }

    #[ink::test]
    fn get_export_request_returns_none_for_unknown_batch() {
        let contract = new_admin_contract();
        assert!(contract.get_export_request(12345).is_none());
    }
}
