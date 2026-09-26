// Error types for the database contract (Issue #101 - extracted from lib.rs)

#[derive(Debug, PartialEq, Eq, scale::Encode, scale::Decode)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub enum Error {
    Unauthorized,
    SyncNotFound,
    ExportNotFound,
    InvalidDataRange,
    IndexerNotFound,
    IndexerAlreadyRegistered,
    /// The registry already holds `MAX_INDEXERS` indexers.
    ///
    /// The cap exists so `indexer_list` cannot grow without bound; without it
    /// the list is a pure leak, since entries are only ever appended.
    IndexerLimitReached,
    /// The indexer exists but has been deactivated, so it may not act.
    IndexerInactive,
    InvalidChecksum,
    SnapshotNotFound,
}
