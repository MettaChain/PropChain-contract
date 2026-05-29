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
}
