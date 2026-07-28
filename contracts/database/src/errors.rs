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
    InvalidChecksum,
    SnapshotNotFound,
}
