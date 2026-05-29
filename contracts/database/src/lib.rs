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

    // Data types extracted to types.rs (Issue #101)
    include!("types.rs");

    // Error types extracted to errors.rs (Issue #101)
    include!("errors.rs");

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

            let mut record = self.sync_records.get(sync_id).ok_or(Error::SyncNotFound)?;

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
            let mut record = self.sync_records.get(sync_id).ok_or(Error::SyncNotFound)?;

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
        #[allow(clippy::too_many_arguments)]
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

            let mut info = self.indexers.get(indexer).ok_or(Error::IndexerNotFound)?;

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

    // Unit tests extracted to tests.rs (Issue #101)
}
