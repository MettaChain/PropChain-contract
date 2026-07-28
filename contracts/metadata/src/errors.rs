// Error types for the metadata contract (Issue #101 - extracted from lib.rs)

#[derive(Debug, PartialEq, Eq, scale::Encode, scale::Decode)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub enum Error {
    PropertyNotFound,
    Unauthorized,
    InvalidMetadata,
    MetadataAlreadyFinalized,
    InvalidIpfsCid,
    DocumentNotFound,
    DocumentAlreadyExists,
    VersionConflict,
    RequiredFieldMissing,
    SizeLimitExceeded,
    InvalidContentHash,
    SearchQueryTooLong,
}
