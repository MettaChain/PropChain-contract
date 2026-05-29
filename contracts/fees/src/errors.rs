// Error types for the fees contract (Issue #101 - extracted from lib.rs)

#[derive(Debug, PartialEq, Eq, scale::Encode, scale::Decode)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub enum FeeError {
    Unauthorized,
    AuctionNotFound,
    AuctionEnded,
    AuctionNotEnded,
    BidTooLow,
    AlreadySettled,
    InvalidConfig,
    InvalidProperty,
}

impl core::fmt::Display for FeeError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            FeeError::Unauthorized => write!(f, "Caller is not authorized"),
            FeeError::AuctionNotFound => write!(f, "Auction does not exist"),
            FeeError::AuctionEnded => write!(f, "Auction has ended"),
            FeeError::AuctionNotEnded => write!(f, "Auction has not ended yet"),
            FeeError::BidTooLow => write!(f, "Bid amount is too low"),
            FeeError::AlreadySettled => write!(f, "Auction already settled"),
            FeeError::InvalidConfig => write!(f, "Invalid configuration"),
            FeeError::InvalidProperty => write!(f, "Invalid property ID"),
        }
    }
}

impl ContractError for FeeError {
    fn error_code(&self) -> u32 {
        match self {
            FeeError::Unauthorized => propchain_traits::errors::fee_codes::FEE_UNAUTHORIZED,
            FeeError::AuctionNotFound => {
                propchain_traits::errors::fee_codes::FEE_AUCTION_NOT_FOUND
            }
            FeeError::AuctionEnded => propchain_traits::errors::fee_codes::FEE_AUCTION_ENDED,
            FeeError::AuctionNotEnded => {
                propchain_traits::errors::fee_codes::FEE_AUCTION_NOT_ENDED
            }
            FeeError::BidTooLow => propchain_traits::errors::fee_codes::FEE_BID_TOO_LOW,
            FeeError::AlreadySettled => {
                propchain_traits::errors::fee_codes::FEE_ALREADY_SETTLED
            }
            FeeError::InvalidConfig => propchain_traits::errors::fee_codes::FEE_INVALID_CONFIG,
            FeeError::InvalidProperty => {
                propchain_traits::errors::fee_codes::FEE_INVALID_PROPERTY
            }
        }
    }

    fn error_description(&self) -> &'static str {
        match self {
            FeeError::Unauthorized => {
                "Caller does not have permission to perform this operation"
            }
            FeeError::AuctionNotFound => "The specified auction does not exist",
            FeeError::AuctionEnded => "This auction has already ended",
            FeeError::AuctionNotEnded => "The auction is still active and has not ended",
            FeeError::BidTooLow => "The bid amount is below the minimum required",
            FeeError::AlreadySettled => "This auction has already been settled",
            FeeError::InvalidConfig => "The fee configuration is invalid",
            FeeError::InvalidProperty => "The property ID is invalid or does not exist",
        }
    }

    fn error_category(&self) -> ErrorCategory {
        ErrorCategory::Fees
    }
}
