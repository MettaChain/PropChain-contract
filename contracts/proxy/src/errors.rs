// Error types for the proxy contract (Issue #101 - extracted from lib.rs)

#[derive(Debug, PartialEq, Eq, scale::Encode, scale::Decode)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub enum Error {
    Unauthorized,
    UpgradeFailed,
    ProposalNotFound,
    ProposalAlreadyExists,
    TimelockNotExpired,
    InsufficientApprovals,
    AlreadyApproved,
    NoPreviousVersion,
    IncompatibleVersion,
    MigrationInProgress,
    NotGovernor,
    ProposalCancelled,
    EmergencyPauseActive,
    InvalidTimelockPeriod,
}
