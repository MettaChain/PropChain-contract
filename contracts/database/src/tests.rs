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
}
