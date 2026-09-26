// Data types for the database contract (Issue #101 - extracted from lib.rs)

pub type SyncId = u64;
pub type ExportBatchId = u64;

#[derive(
    Debug, Clone, PartialEq, scale::Encode, scale::Decode, ink::storage::traits::StorageLayout,
)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub struct SyncRecord {
    pub sync_id: SyncId,
    pub data_type: DataType,
    pub block_number: u32,
    pub timestamp: u64,
    pub data_checksum: Hash,
    pub record_count: u64,
    pub status: SyncStatus,
    pub initiated_by: AccountId,
}

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
    Properties,
    Transfers,
    Escrows,
    Compliance,
    Valuations,
    Tokens,
    Analytics,
    FullState,
}

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
    Initiated,
    Confirmed,
    Failed,
    /// The supplied checksum matched the checksum recorded at `emit_sync_event`.
    ///
    /// This is a self-consistency check between two on-chain values, not a
    /// proof that the data matches the source chain. See `verify_sync`.
    Verified,
}

#[derive(
    Debug, Clone, PartialEq, scale::Encode, scale::Decode, ink::storage::traits::StorageLayout,
)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub struct AnalyticsSnapshot {
    pub snapshot_id: u64,
    pub block_number: u32,
    pub timestamp: u64,
    pub total_properties: u64,
    pub total_transfers: u64,
    pub total_escrows: u64,
    pub total_valuation: u128,
    pub avg_valuation: u128,
    pub active_accounts: u64,
    /// Publisher-supplied checksum over the snapshot's figures.
    ///
    /// Nothing on-chain recomputes it. See `verify_sync`.
    pub integrity_checksum: Hash,
    pub created_by: AccountId,
}

#[derive(
    Debug, Clone, PartialEq, scale::Encode, scale::Decode, ink::storage::traits::StorageLayout,
)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub struct ExportRequest {
    pub batch_id: ExportBatchId,
    pub data_type: DataType,
    pub from_id: u64,
    pub to_id: u64,
    pub from_block: u32,
    pub to_block: u32,
    pub requested_by: AccountId,
    pub requested_at: u64,
    pub completed: bool,
    pub export_checksum: Option<Hash>,
}

#[derive(
    Debug, Clone, PartialEq, scale::Encode, scale::Decode, ink::storage::traits::StorageLayout,
)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub struct IndexerInfo {
    pub account: AccountId,
    pub name: String,
    pub last_synced_block: u32,
    pub is_active: bool,
    pub registered_at: u64,
    /// Block timestamp of the indexer's most recent liveness signal.
    ///
    /// `last_synced_block` alone cannot establish liveness: an indexer that
    /// stalls at block 100 forever still reports 100, and a fresh registration
    /// reports 0, which is indistinguishable from a dead node. This is
    /// refreshed on every heartbeat and compared against
    /// `INDEXER_STALE_AFTER` to decide whether the indexer is still live.
    pub last_heartbeat: u64,
}
