#![cfg_attr(not(feature = "std"), no_std)]
#![allow(unexpected_cfgs)]
#![allow(clippy::new_without_default)]

//! # PropChain Database Integration Contract
//!
//! On-chain coordination layer for off-chain database integration providing:
//! - Database synchronization event emission for off-chain indexers
//! - Data export capabilities via structured events
//! - Analytics data aggregation and snapshots
//! - Sync state tracking and verification
//! - Data integrity checksums for off-chain validation
//!
//! ## Architecture
//!
//! This contract acts as the on-chain coordination point:
//! 1. **Sync Events**: Emits structured events that off-chain indexers consume
//!    to keep databases synchronized with on-chain state.
//! 2. **Data Export**: Provides batch query endpoints for initial DB population
//!    and periodic reconciliation.
//! 3. **Analytics Snapshots**: Records periodic analytics snapshots on-chain
//!    that can be verified against off-chain analytics databases.
//! 4. **Integrity Verification**: Stores Merkle roots / checksums of data sets
//!    to allow off-chain databases to prove data integrity.
//!
//! Resolves: https://github.com/MettaChain/PropChain-contract/issues/112

use ink::prelude::string::String;
use ink::prelude::vec::Vec;
use ink::storage::Mapping;

#[ink::contract]
mod propchain_database {
    use super::*;

    // ========================================================================
    // TYPES
    // ========================================================================

    /// Unique identifier for sync operations
    pub type SyncId = u64;

    /// Data export batch identifier
    pub type ExportBatchId = u64;

    // ========================================================================
    // DATA STRUCTURES
    // ========================================================================

    /// Database sync record tracking synchronization state
    #[derive(
        Debug, Clone, PartialEq, scale::Encode, scale::Decode, ink::storage::traits::StorageLayout,
    )]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub struct SyncRecord {
        /// Unique sync operation ID
        pub sync_id: SyncId,
        /// Type of data being synced
        pub data_type: DataType,
        /// Block number at which sync was recorded
        pub block_number: u32,
        /// Timestamp of sync
        pub timestamp: u64,
        /// Hash/checksum of the synced data
        pub data_checksum: Hash,
        /// Number of records in this sync batch
        pub record_count: u64,
        /// Status of the sync operation
        pub status: SyncStatus,
        /// Account that initiated the sync
        pub initiated_by: AccountId,
    }

    /// Types of data that can be synced to off-chain database
    #[derive(
        Debug,
        Clone,
        PartialEq,
        Eq,
        scale::Encode,
        scale::Decode,
        ink::storage::traits::StorageLayout,
    )]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub enum DataType {
        /// Property registration data
        Properties,
        /// Ownership transfer records
        Transfers,
        /// Escrow operations
        Escrows,
        /// Compliance/KYC data
        Compliance,
        /// Valuation/price data
        Valuations,
        /// Token operations
        Tokens,
        /// Analytics snapshots
        Analytics,
        /// Full state export
        FullState,
    }

    /// Sync operation status
    #[derive(
        Debug,
        Clone,
        PartialEq,
        Eq,
        scale::Encode,
        scale::Decode,
        ink::storage::traits::StorageLayout,
    )]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub enum SyncStatus {
        /// Sync initiated, events emitted
        Initiated,
        /// Sync confirmed by off-chain indexer
        Confirmed,
        /// Sync failed and needs retry
        Failed,
        /// Sync data verified against off-chain DB
        Verified,
    }

    /// Analytics snapshot stored on-chain for verification
    #[derive(
        Debug, Clone, PartialEq, scale::Encode, scale::Decode, ink::storage::traits::StorageLayout,
    )]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub struct AnalyticsSnapshot {
        /// Snapshot identifier
        pub snapshot_id: u64,
        /// Block number when snapshot was taken
        pub block_number: u32,
        /// Timestamp
        pub timestamp: u64,
        /// Total properties in the system
        pub total_properties: u64,
        /// Total transfers recorded
        pub total_transfers: u64,
        /// Total escrows created
        pub total_escrows: u64,
        /// Total valuation across all properties (in smallest unit)
        pub total_valuation: u128,
        /// Average property valuation
        pub avg_valuation: u128,
        /// Total active users (unique accounts)
        pub active_accounts: u64,
        /// Data integrity checksum (Merkle root of all data)
        pub integrity_checksum: Hash,
        /// Created by
        pub created_by: AccountId,
    }

    /// Data export request for batch operations
    #[derive(
        Debug, Clone, PartialEq, scale::Encode, scale::Decode, ink::storage::traits::StorageLayout,
    )]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub struct ExportRequest {
        /// Export batch ID
        pub batch_id: ExportBatchId,
        /// Type of data requested
        pub data_type: DataType,
        /// Start index / from ID
        pub from_id: u64,
        /// End index / to ID
        pub to_id: u64,
        /// Block range start
        pub from_block: u32,
        /// Block range end
        pub to_block: u32,
        /// Requested by
        pub requested_by: AccountId,
        /// Request timestamp
        pub requested_at: u64,
        /// Whether export is complete
        pub completed: bool,
        /// Checksum of exported data
        pub export_checksum: Option<Hash>,
    }

    /// Indexer registration for sync coordination
    #[derive(
        Debug, Clone, PartialEq, scale::Encode, scale::Decode, ink::storage::traits::StorageLayout,
    )]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub struct IndexerInfo {
        /// Indexer account
        pub account: AccountId,
        /// Indexer name/identifier
        pub name: String,
        /// Last synced block
        pub last_synced_block: u32,
        /// Whether indexer is active
        pub is_active: bool,
        /// Registration timestamp
        pub registered_at: u64,
    }

    // ========================================================================
    // ERRORS
    // ========================================================================

    #[derive(Debug, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub enum Error {
        Unauthorized,
        SyncNotFound,
        ExportNotFound,
        InvalidDataRange,
        IndexerNotFound,
        IndexerAlreadyRegistered,
        InvalidChecksum,
        SnapshotNotFound,
    }

    // ========================================================================
    // EVENTS
    // ========================================================================

    /// Emitted for every data change that off-chain databases should sync
    #[ink(event)]
    pub struct DataSyncEvent {
        #[ink(topic)]
        sync_id: SyncId,
        #[ink(topic)]
        data_type: DataType,
        #[ink(topic)]
        block_number: u32,
        data_checksum: Hash,
        record_count: u64,
        timestamp: u64,
    }

    /// Emitted when a sync is confirmed by an indexer
    #[ink(event)]
    pub struct SyncConfirmed {
        #[ink(topic)]
        sync_id: SyncId,
        #[ink(topic)]
        indexer: AccountId,
        block_number: u32,
        timestamp: u64,
    }

    /// Emitted when an analytics snapshot is recorded
    #[ink(event)]
    pub struct AnalyticsSnapshotRecorded {
        #[ink(topic)]
        snapshot_id: u64,
        #[ink(topic)]
        block_number: u32,
        total_properties: u64,
        total_valuation: u128,
        integrity_checksum: Hash,
        timestamp: u64,
    }

    /// Emitted when a data export is requested
    #[ink(event)]
    pub struct DataExportRequested {
        #[ink(topic)]
        batch_id: ExportBatchId,
        #[ink(topic)]
        data_type: DataType,
        from_id: u64,
        to_id: u64,
        requested_by: AccountId,
        timestamp: u64,
    }

    /// Emitted when a data export is completed
    #[ink(event)]
    pub struct DataExportCompleted {
        #[ink(topic)]
        batch_id: ExportBatchId,
        export_checksum: Hash,
        timestamp: u64,
    }

    /// Emitted when an indexer is registered
    #[ink(event)]
    pub struct IndexerRegistered {
        #[ink(topic)]
        indexer: AccountId,
        name: String,
        timestamp: u64,
    }

    // ========================================================================
    // CONTRACT STORAGE
    // ========================================================================

    #[ink(storage)]
    pub struct DatabaseIntegration {
        /// Contract admin
        admin: AccountId,
        /// Sync records
        sync_records: Mapping<SyncId, SyncRecord>,
        /// Sync counter
        sync_counter: SyncId,
        /// Analytics snapshots
        analytics_snapshots: Mapping<u64, AnalyticsSnapshot>,
        /// Snapshot counter
        snapshot_counter: u64,
        /// Export requests
        export_requests: Mapping<ExportBatchId, ExportRequest>,
        /// Export counter
        export_counter: ExportBatchId,
        /// Registered indexers
        indexers: Mapping<AccountId, IndexerInfo>,
        /// List of registered indexer accounts
        indexer_list: Vec<AccountId>,
        /// Last sync block per data type (stored as u8 key)
        last_sync_block: Mapping<u8, u32>,
        /// Authorized data publishers (contracts that can emit sync events)
        authorized_publishers: Mapping<AccountId, bool>,
    }

    // ========================================================================
    // IMPLEMENTATION
    // ========================================================================

    impl DatabaseIntegration {
        #[ink(constructor)]
        pub fn new() -> Self {
            let caller = Self::env().caller();
            Self {
                admin: caller,
                sync_records: Mapping::default(),
                sync_counter: 0,
                analytics_snapshots: Mapping::default(),
                snapshot_counter: 0,
                export_requests: Mapping::default(),
                export_counter: 0,
                indexers: Mapping::default(),
                indexer_list: Vec::new(),
                last_sync_block: Mapping::default(),
                authorized_publishers: Mapping::default(),
            }
        }

        // ====================================================================
        // DATA SYNCHRONIZATION
        // ====================================================================

        /// Emits a sync event for off-chain database synchronization.
        /// Called by authorized contracts when data changes occur.
        #[ink(message)]
        pub fn emit_sync_event(
            &mut self,
            data_type: DataType,
            data_checksum: Hash,
            record_count: u64,
        ) -> Result<SyncId, Error> {
            let caller = self.env().caller();
            if caller != self.admin && !self.authorized_publishers.get(caller).unwrap_or(false) {
                return Err(Error::Unauthorized);
            }

            self.sync_counter += 1;
            let sync_id = self.sync_counter;
            let block_number = self.env().block_number();
            let timestamp = self.env().block_timestamp();

            let record = SyncRecord {
                sync_id,
                data_type: data_type.clone(),
                block_number,
                timestamp,
                data_checksum,
                record_count,
                status: SyncStatus::Initiated,
                initiated_by: caller,
            };

            self.sync_records.insert(sync_id, &record);

            // Update last sync block for this data type
            let dt_key = self.data_type_to_key(&data_type);
            self.last_sync_block.insert(dt_key, &block_number);

            self.env().emit_event(DataSyncEvent {
                sync_id,
                data_type,
                block_number,
                data_checksum,
                record_count,
                timestamp,
            });

            Ok(sync_id)
        }

        /// Confirms a sync operation (called by registered indexer)
        #[ink(message)]
        pub fn confirm_sync(&mut self, sync_id: SyncId) -> Result<(), Error> {
            let caller = self.env().caller();

            // Must be a registered indexer
            if !self.indexers.contains(caller) {
                return Err(Error::IndexerNotFound);
            }

            let mut record = self
                .sync_records
                .get(sync_id)
                .ok_or(Error::SyncNotFound)?;

            record.status = SyncStatus::Confirmed;
            self.sync_records.insert(sync_id, &record);

            // Update indexer's last synced block
            if let Some(mut indexer) = self.indexers.get(caller) {
                indexer.last_synced_block = record.block_number;
                self.indexers.insert(caller, &indexer);
            }

            self.env().emit_event(SyncConfirmed {
                sync_id,
                indexer: caller,
                block_number: record.block_number,
                timestamp: self.env().block_timestamp(),
            });

            Ok(())
        }

        /// Verifies sync data integrity by comparing checksums
        #[ink(message)]
        pub fn verify_sync(
            &mut self,
            sync_id: SyncId,
            verification_checksum: Hash,
        ) -> Result<bool, Error> {
            let mut record = self
                .sync_records
                .get(sync_id)
                .ok_or(Error::SyncNotFound)?;

            let is_valid = record.data_checksum == verification_checksum;

            if is_valid {
                record.status = SyncStatus::Verified;
            } else {
                record.status = SyncStatus::Failed;
            }

            self.sync_records.insert(sync_id, &record);
            Ok(is_valid)
        }

        // ====================================================================
        // ANALYTICS SNAPSHOTS
        // ====================================================================

        /// Records an analytics snapshot on-chain for later verification
        #[ink(message)]
        pub fn record_analytics_snapshot(
            &mut self,
            total_properties: u64,
            total_transfers: u64,
            total_escrows: u64,
            total_valuation: u128,
            avg_valuation: u128,
            active_accounts: u64,
            integrity_checksum: Hash,
        ) -> Result<u64, Error> {
            let caller = self.env().caller();
            if caller != self.admin {
                return Err(Error::Unauthorized);
            }

            self.snapshot_counter += 1;
            let snapshot_id = self.snapshot_counter;
            let block_number = self.env().block_number();
            let timestamp = self.env().block_timestamp();

            let snapshot = AnalyticsSnapshot {
                snapshot_id,
                block_number,
                timestamp,
                total_properties,
                total_transfers,
                total_escrows,
                total_valuation,
                avg_valuation,
                active_accounts,
                integrity_checksum,
                created_by: caller,
            };

            self.analytics_snapshots.insert(snapshot_id, &snapshot);

            self.env().emit_event(AnalyticsSnapshotRecorded {
                snapshot_id,
                block_number,
                total_properties,
                total_valuation,
                integrity_checksum,
                timestamp,
            });

            Ok(snapshot_id)
        }

        /// Retrieves an analytics snapshot
        #[ink(message)]
        pub fn get_analytics_snapshot(&self, snapshot_id: u64) -> Option<AnalyticsSnapshot> {
            self.analytics_snapshots.get(snapshot_id)
        }

        /// Gets the latest snapshot ID
        #[ink(message)]
        pub fn latest_snapshot_id(&self) -> u64 {
            self.snapshot_counter
        }

        // ====================================================================
        // DATA EXPORT
        // ====================================================================

        /// Requests a data export for a specific range
        #[ink(message)]
        pub fn request_data_export(
            &mut self,
            data_type: DataType,
            from_id: u64,
            to_id: u64,
            from_block: u32,
            to_block: u32,
        ) -> Result<ExportBatchId, Error> {
            let caller = self.env().caller();
            if caller != self.admin {
                return Err(Error::Unauthorized);
            }

            if from_id > to_id || from_block > to_block {
                return Err(Error::InvalidDataRange);
            }

            self.export_counter += 1;
            let batch_id = self.export_counter;
            let timestamp = self.env().block_timestamp();

            let request = ExportRequest {
                batch_id,
                data_type: data_type.clone(),
                from_id,
                to_id,
                from_block,
                to_block,
                requested_by: caller,
                requested_at: timestamp,
                completed: false,
                export_checksum: None,
            };

            self.export_requests.insert(batch_id, &request);

            self.env().emit_event(DataExportRequested {
                batch_id,
                data_type,
                from_id,
                to_id,
                requested_by: caller,
                timestamp,
            });

            Ok(batch_id)
        }

        /// Marks a data export as completed with verification checksum
        #[ink(message)]
        pub fn complete_data_export(
            &mut self,
            batch_id: ExportBatchId,
            export_checksum: Hash,
        ) -> Result<(), Error> {
            let caller = self.env().caller();
            if caller != self.admin {
                return Err(Error::Unauthorized);
            }

            let mut request = self
                .export_requests
                .get(batch_id)
                .ok_or(Error::ExportNotFound)?;

            request.completed = true;
            request.export_checksum = Some(export_checksum);

            self.export_requests.insert(batch_id, &request);

            self.env().emit_event(DataExportCompleted {
                batch_id,
                export_checksum,
                timestamp: self.env().block_timestamp(),
            });

            Ok(())
        }

        /// Gets export request details
        #[ink(message)]
        pub fn get_export_request(&self, batch_id: ExportBatchId) -> Option<ExportRequest> {
            self.export_requests.get(batch_id)
        }

        // ====================================================================
        // INDEXER MANAGEMENT
        // ====================================================================

        /// Registers an off-chain indexer
        #[ink(message)]
        pub fn register_indexer(&mut self, indexer: AccountId, name: String) -> Result<(), Error> {
            let caller = self.env().caller();
            if caller != self.admin {
                return Err(Error::Unauthorized);
            }

            if self.indexers.contains(indexer) {
                return Err(Error::IndexerAlreadyRegistered);
            }

            let info = IndexerInfo {
                account: indexer,
                name: name.clone(),
                last_synced_block: 0,
                is_active: true,
                registered_at: self.env().block_timestamp(),
            };

            self.indexers.insert(indexer, &info);
            self.indexer_list.push(indexer);

            self.env().emit_event(IndexerRegistered {
                indexer,
                name,
                timestamp: self.env().block_timestamp(),
            });

            Ok(())
        }

        /// Deactivates an indexer
        #[ink(message)]
        pub fn deactivate_indexer(&mut self, indexer: AccountId) -> Result<(), Error> {
            let caller = self.env().caller();
            if caller != self.admin {
                return Err(Error::Unauthorized);
            }

            let mut info = self
                .indexers
                .get(indexer)
                .ok_or(Error::IndexerNotFound)?;

            info.is_active = false;
            self.indexers.insert(indexer, &info);

            Ok(())
        }

        /// Gets indexer information
        #[ink(message)]
        pub fn get_indexer(&self, indexer: AccountId) -> Option<IndexerInfo> {
            self.indexers.get(indexer)
        }

        /// Gets all registered indexer accounts
        #[ink(message)]
        pub fn get_indexer_list(&self) -> Vec<AccountId> {
            self.indexer_list.clone()
        }

        // ====================================================================
        // PUBLISHER MANAGEMENT
        // ====================================================================

        /// Authorizes a contract to publish sync events
        #[ink(message)]
        pub fn authorize_publisher(&mut self, publisher: AccountId) -> Result<(), Error> {
            let caller = self.env().caller();
            if caller != self.admin {
                return Err(Error::Unauthorized);
            }
            self.authorized_publishers.insert(publisher, &true);
            Ok(())
        }

        /// Revokes a publisher's authorization
        #[ink(message)]
        pub fn revoke_publisher(&mut self, publisher: AccountId) -> Result<(), Error> {
            let caller = self.env().caller();
            if caller != self.admin {
                return Err(Error::Unauthorized);
            }
            self.authorized_publishers.remove(publisher);
            Ok(())
        }

        // ====================================================================
        // QUERY FUNCTIONS
        // ====================================================================

        /// Gets a sync record
        #[ink(message)]
        pub fn get_sync_record(&self, sync_id: SyncId) -> Option<SyncRecord> {
            self.sync_records.get(sync_id)
        }

        /// Gets total sync operations count
        #[ink(message)]
        pub fn total_syncs(&self) -> SyncId {
            self.sync_counter
        }

        /// Gets the last synced block for a data type
        #[ink(message)]
        pub fn last_synced_block(&self, data_type: DataType) -> u32 {
            let key = self.data_type_to_key(&data_type);
            self.last_sync_block.get(key).unwrap_or(0)
        }

        /// Gets admin
        #[ink(message)]
        pub fn admin(&self) -> AccountId {
            self.admin
        }

        // ====================================================================
        // INTERNAL
        // ====================================================================

        fn data_type_to_key(&self, dt: &DataType) -> u8 {
            match dt {
                DataType::Properties => 0,
                DataType::Transfers => 1,
                DataType::Escrows => 2,
                DataType::Compliance => 3,
                DataType::Valuations => 4,
                DataType::Tokens => 5,
                DataType::Analytics => 6,
                DataType::FullState => 7,
            }
        }
    }

    impl Default for DatabaseIntegration {
        fn default() -> Self {
            Self::new()
        }
    }

    // ========================================================================
    // UNIT TESTS
    // ========================================================================

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
            let result = contract.emit_sync_event(
                DataType::Properties,
                Hash::from([0x01; 32]),
                10,
            );
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
                100, 50, 20, 10_000_000, 100_000, 30, Hash::from([0x02; 32]),
            );
            assert!(result.is_ok());

            let snapshot = contract.get_analytics_snapshot(1).unwrap();
            assert_eq!(snapshot.total_properties, 100);
            assert_eq!(snapshot.total_valuation, 10_000_000);
        }

        #[ink::test]
        fn data_export_works() {
            let mut contract = DatabaseIntegration::new();
            let result =
                contract.request_data_export(DataType::Properties, 1, 100, 0, 1000);
            assert!(result.is_ok());

            let batch_id = result.unwrap();
            let request = contract.get_export_request(batch_id).unwrap();
            assert!(!request.completed);

            let complete_result =
                contract.complete_data_export(batch_id, Hash::from([0x03; 32]));
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

            // Correct checksum
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
}
