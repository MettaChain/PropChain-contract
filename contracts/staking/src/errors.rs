// Error types for the staking contract (Issue #101 - extracted from lib.rs)

#[derive(Debug, PartialEq, Eq, scale::Encode, scale::Decode)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub enum Error {
    Unauthorized,
    InsufficientAmount,
    StakeNotFound,
    LockActive,
    NoRewards,
    InsufficientPool,
    InvalidConfig,
    AlreadyStaked,
    InvalidDelegate,
    ZeroAmount,
    ReentrantCall,
}

impl core::fmt::Display for Error {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Error::Unauthorized => write!(f, "Caller is not authorized"),
            Error::InsufficientAmount => write!(f, "Amount below minimum stake"),
            Error::StakeNotFound => write!(f, "No active stake found"),
            Error::LockActive => write!(f, "Lock period has not expired"),
            Error::NoRewards => write!(f, "No rewards available"),
            Error::InsufficientPool => write!(f, "Reward pool insufficient"),
            Error::InvalidConfig => write!(f, "Invalid configuration"),
            Error::AlreadyStaked => write!(f, "Account already has an active stake"),
            Error::InvalidDelegate => write!(f, "Invalid delegation target"),
            Error::ZeroAmount => write!(f, "Amount must be greater than zero"),
            Error::ReentrantCall => write!(f, "Reentrant call detected"),
        }
    }
}

impl ContractError for Error {
    fn error_code(&self) -> u32 {
        match self {
            Error::Unauthorized => staking_codes::STAKING_UNAUTHORIZED,
            Error::InsufficientAmount => staking_codes::STAKING_INSUFFICIENT_AMOUNT,
            Error::StakeNotFound => staking_codes::STAKING_NOT_FOUND,
            Error::LockActive => staking_codes::STAKING_LOCK_ACTIVE,
            Error::NoRewards => staking_codes::STAKING_NO_REWARDS,
            Error::InsufficientPool => staking_codes::STAKING_INSUFFICIENT_POOL,
            Error::InvalidConfig => staking_codes::STAKING_INVALID_CONFIG,
            Error::AlreadyStaked => staking_codes::STAKING_ALREADY_STAKED,
            Error::InvalidDelegate => staking_codes::STAKING_INVALID_DELEGATE,
            Error::ZeroAmount => staking_codes::STAKING_ZERO_AMOUNT,
            Error::ReentrantCall => 9999,
        }
    }

    fn error_description(&self) -> &'static str {
        match self {
            Error::Unauthorized => "Caller does not have staking permissions",
            Error::InsufficientAmount => "Stake amount is below the minimum threshold",
            Error::StakeNotFound => "No active stake found for this account",
            Error::LockActive => "Cannot unstake while the lock period is active",
            Error::NoRewards => "No pending rewards to claim",
            Error::InsufficientPool => "Reward pool has insufficient funds",
            Error::InvalidConfig => "The provided configuration parameters are invalid",
            Error::AlreadyStaked => "This account already has an active stake",
            Error::InvalidDelegate => "Cannot delegate governance to this address",
            Error::ZeroAmount => "The amount must be greater than zero",
            Error::ReentrantCall => "Reentrant call detected",
        }
    }

    fn error_category(&self) -> ErrorCategory {
        ErrorCategory::Staking
    }
}
