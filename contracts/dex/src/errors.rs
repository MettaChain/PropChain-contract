// Error types for the DEX contract (Issue #101 - extracted from lib.rs)

#[derive(Debug, PartialEq, Eq, scale::Encode, scale::Decode)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub enum Error {
    Unauthorized,
    InvalidPair,
    PoolNotFound,
    InsufficientLiquidity,
    SlippageExceeded,
    OrderNotFound,
    InvalidOrder,
    InvalidRequest,
    OrderNotExecutable,
    RewardUnavailable,
    ProposalNotFound,
    ProposalClosed,
    AlreadyVoted,
    InvalidBridgeRoute,
    CrossChainTradeNotFound,
    InsufficientGovernanceBalance,
    ReentrantCall,
    TimelockRequired,
    TimelockActive,
    AdminActionNotFound,
    AdminActionAlreadyFinalized,
}

impl core::fmt::Display for Error {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Error::Unauthorized => write!(f, "Caller is not authorized"),
            Error::InvalidPair => write!(f, "Invalid trading pair"),
            Error::PoolNotFound => write!(f, "Liquidity pool not found"),
            Error::InsufficientLiquidity => write!(f, "Insufficient liquidity"),
            Error::SlippageExceeded => write!(f, "Slippage tolerance exceeded"),
            Error::OrderNotFound => write!(f, "Order not found"),
            Error::InvalidOrder => write!(f, "Order parameters are invalid"),
            Error::OrderNotExecutable => write!(f, "Order is not executable"),
            Error::InvalidRequest => write!(f, "Invalid request"),
            Error::RewardUnavailable => write!(f, "Reward unavailable"),
            Error::ProposalNotFound => write!(f, "Governance proposal not found"),
            Error::ProposalClosed => write!(f, "Governance proposal is closed"),
            Error::AlreadyVoted => write!(f, "Vote already recorded"),
            Error::InvalidBridgeRoute => write!(f, "Invalid cross-chain bridge route"),
            Error::CrossChainTradeNotFound => write!(f, "Cross-chain trade not found"),
            Error::InsufficientGovernanceBalance => {
                write!(f, "Insufficient governance balance")
            }
            Error::ReentrantCall => write!(f, "Reentrant call"),
            Error::TimelockRequired => {
                write!(f, "Sensitive admin change must be scheduled through the timelock")
            }
            Error::TimelockActive => {
                write!(f, "Scheduled admin action has not reached its execution block")
            }
            Error::AdminActionNotFound => write!(f, "Scheduled admin action not found"),
            Error::AdminActionAlreadyFinalized => {
                write!(f, "Scheduled admin action was already executed or cancelled")
            }
        }
    }
}

impl ContractError for Error {
    fn error_code(&self) -> u32 {
        match self {
            Error::Unauthorized => dex_codes::DEX_UNAUTHORIZED,
            Error::InvalidPair => dex_codes::DEX_INVALID_PAIR,
            Error::PoolNotFound => dex_codes::DEX_POOL_NOT_FOUND,
            Error::InsufficientLiquidity => dex_codes::DEX_INSUFFICIENT_LIQUIDITY,
            Error::SlippageExceeded => dex_codes::DEX_SLIPPAGE_EXCEEDED,
            Error::OrderNotFound => dex_codes::DEX_ORDER_NOT_FOUND,
            Error::InvalidOrder => dex_codes::DEX_INVALID_ORDER,
            Error::InvalidRequest => dex_codes::DEX_INVALID_REQUEST,
            Error::OrderNotExecutable => dex_codes::DEX_ORDER_NOT_EXECUTABLE,
            Error::RewardUnavailable => dex_codes::DEX_REWARD_UNAVAILABLE,
            Error::ProposalNotFound => dex_codes::DEX_PROPOSAL_NOT_FOUND,
            Error::ProposalClosed => dex_codes::DEX_PROPOSAL_CLOSED,
            Error::AlreadyVoted => dex_codes::DEX_ALREADY_VOTED,
            Error::InvalidBridgeRoute => dex_codes::DEX_INVALID_BRIDGE_ROUTE,
            Error::CrossChainTradeNotFound => dex_codes::DEX_CROSS_CHAIN_TRADE_NOT_FOUND,
            Error::InsufficientGovernanceBalance => {
                dex_codes::DEX_INSUFFICIENT_GOVERNANCE_BALANCE
            }
            Error::ReentrantCall => dex_codes::REENTRANT_CALL,
            Error::TimelockRequired => dex_codes::DEX_TIMELOCK_REQUIRED,
            Error::TimelockActive => dex_codes::DEX_TIMELOCK_ACTIVE,
            Error::AdminActionNotFound => dex_codes::DEX_ADMIN_ACTION_NOT_FOUND,
            Error::AdminActionAlreadyFinalized => dex_codes::DEX_ADMIN_ACTION_ALREADY_FINALIZED,
        }
    }

    fn error_description(&self) -> &'static str {
        match self {
            Error::Unauthorized => "Caller does not have permission to perform this operation",
            Error::InvalidPair => "The requested trading pair is invalid or inactive",
            Error::PoolNotFound => "The referenced liquidity pool does not exist",
            Error::InsufficientLiquidity => "Not enough liquidity is available",
            Error::SlippageExceeded => "Trade output is below the allowed threshold",
            Error::OrderNotFound => "The order does not exist",
            Error::InvalidOrder => "Order parameters are invalid",
            Error::InvalidRequest => "The request is invalid",
            Error::OrderNotExecutable => "Order conditions are not satisfied",
            Error::RewardUnavailable => "There are no rewards available to claim",
            Error::ProposalNotFound => "The governance proposal does not exist",
            Error::ProposalClosed => "The governance proposal can no longer be modified",
            Error::AlreadyVoted => "The account has already voted on this proposal",
            Error::InvalidBridgeRoute => "The selected bridge route is not supported",
            Error::CrossChainTradeNotFound => "The cross-chain trade does not exist",
            Error::InsufficientGovernanceBalance => {
                "The account does not hold enough governance tokens"
            }
            Error::ReentrantCall => "Reentrancy guard detected a reentrant call",
            Error::TimelockRequired => {
                "Direct admin call blocked: action must be scheduled while a timelock is active"
            }
            Error::TimelockActive => {
                "Scheduled admin action cannot execute until the timelock delay has elapsed"
            }
            Error::AdminActionNotFound => "The scheduled admin action does not exist",
            Error::AdminActionAlreadyFinalized => {
                "The scheduled admin action has already been executed or cancelled"
            }
        }
    }

    fn error_category(&self) -> ErrorCategory {
        ErrorCategory::Dex
    }
}
