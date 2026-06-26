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
    NoVotingPower,
    ProposalNotFound,
    ProposalClosed,
    AlreadyVoted,
    VotingActive,
    VotingEnded,
    QuorumNotReached,
    TooManyProposals,
    EarlyWithdrawalPenaltyApplied,
    // ----- Validator / Delegation -----
    InsufficientValidatorStake,
    InvalidCommissionRate,
    AlreadyValidator,
    ValidatorNotFound,
    ValidatorNotActive,
    AlreadyDelegated,
    DelegationNotFound,
    AlreadyUnbonding,
    UnbondingPeriodActive,
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
            Error::NoVotingPower => write!(f, "Caller has no voting power"),
            Error::ProposalNotFound => write!(f, "Proposal not found"),
            Error::ProposalClosed => write!(f, "Proposal is no longer active"),
            Error::AlreadyVoted => write!(f, "Caller already voted on this proposal"),
            Error::VotingActive => write!(f, "Voting period is still active"),
            Error::VotingEnded => write!(f, "Voting period has ended"),
            Error::QuorumNotReached => write!(f, "Quorum not reached"),
            Error::TooManyProposals => write!(f, "Too many active proposals"),
            Error::EarlyWithdrawalPenaltyApplied => write!(f, "Early withdrawal penalty applied"),
            Error::InsufficientValidatorStake => write!(f, "Insufficient validator self-stake"),
            Error::InvalidCommissionRate => write!(f, "Commission rate exceeds maximum"),
            Error::AlreadyValidator => write!(f, "Already a registered validator"),
            Error::ValidatorNotFound => write!(f, "Validator not found"),
            Error::ValidatorNotActive => write!(f, "Validator is not active"),
            Error::AlreadyDelegated => write!(f, "Already delegated to this validator"),
            Error::DelegationNotFound => write!(f, "Delegation not found"),
            Error::AlreadyUnbonding => write!(f, "Already unbonding from this validator"),
            Error::UnbondingPeriodActive => write!(f, "Unbonding period still active"),
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
            Error::NoVotingPower => staking_codes::STAKING_NO_VOTING_POWER,
            Error::ProposalNotFound => staking_codes::STAKING_PROPOSAL_NOT_FOUND,
            Error::ProposalClosed => staking_codes::STAKING_PROPOSAL_CLOSED,
            Error::AlreadyVoted => staking_codes::STAKING_ALREADY_VOTED,
            Error::VotingActive => staking_codes::STAKING_VOTING_ACTIVE,
            Error::VotingEnded => staking_codes::STAKING_VOTING_ENDED,
            Error::QuorumNotReached => staking_codes::STAKING_QUORUM_NOT_REACHED,
            Error::TooManyProposals => staking_codes::STAKING_TOO_MANY_PROPOSALS,
            Error::EarlyWithdrawalPenaltyApplied => staking_codes::STAKING_LOCK_ACTIVE + 1,
            Error::InsufficientValidatorStake => staking_codes::STAKING_LOCK_ACTIVE + 2,
            Error::InvalidCommissionRate => staking_codes::STAKING_LOCK_ACTIVE + 3,
            Error::AlreadyValidator => staking_codes::STAKING_LOCK_ACTIVE + 4,
            Error::ValidatorNotFound => staking_codes::STAKING_LOCK_ACTIVE + 5,
            Error::ValidatorNotActive => staking_codes::STAKING_LOCK_ACTIVE + 6,
            Error::AlreadyDelegated => staking_codes::STAKING_LOCK_ACTIVE + 7,
            Error::DelegationNotFound => staking_codes::STAKING_LOCK_ACTIVE + 8,
            Error::AlreadyUnbonding => staking_codes::STAKING_LOCK_ACTIVE + 9,
            Error::UnbondingPeriodActive => staking_codes::STAKING_LOCK_ACTIVE + 10,
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
            Error::NoVotingPower => "Caller has zero governance power and cannot vote or propose",
            Error::ProposalNotFound => "No parameter proposal exists with this id",
            Error::ProposalClosed => "The proposal has already been finalised",
            Error::AlreadyVoted => "This account already voted on the proposal",
            Error::VotingActive => "Cannot execute while the voting window is still open",
            Error::VotingEnded => "Cannot vote after the voting window has closed",
            Error::QuorumNotReached => "Total turnout did not meet the quorum threshold",
            Error::TooManyProposals => "Active proposal limit reached",
            Error::EarlyWithdrawalPenaltyApplied => "Stake was withdrawn early; a penalty was deducted",
            Error::InsufficientValidatorStake => "Validator self-stake is below the minimum",
            Error::InvalidCommissionRate => "Commission rate exceeds the maximum allowed",
            Error::AlreadyValidator => "This account is already a registered validator",
            Error::ValidatorNotFound => "No validator found for this account",
            Error::ValidatorNotActive => "The validator is not currently active",
            Error::AlreadyDelegated => "Account already has an active delegation to this validator",
            Error::DelegationNotFound => "No delegation found for this pair",
            Error::AlreadyUnbonding => "Already unbonding from this validator",
            Error::UnbondingPeriodActive => "Unbonding period is still active for this delegation",
        }
    }

    fn error_category(&self) -> ErrorCategory {
        ErrorCategory::Staking
    }
}
