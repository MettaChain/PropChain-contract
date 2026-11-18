#![cfg_attr(not(feature = "std"), no_std, no_main)]

#[ink::contract]
mod english_auction {
    use ink::storage::Mapping;

    #[derive(Debug, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub enum Error {
        Unauthorized, AuctionEnded, AuctionNotEnded, BidTooLow, ReserveNotMet,
        AlreadyEnded, AlreadyCancelled, ReentrantCall, TransferFailed, Overflow,
    }

    #[ink(storage)]
    pub struct EnglishAuction {
        seller: AccountId,
        minimum_bid: Balance,
        bid_increment: Balance,
        reserve_price: Balance,
        start_block: BlockNumber,
        end_block: BlockNumber,
        highest_bidder: Option<AccountId>,
        highest_bid: Balance,
        bid_history: Mapping<AccountId, Balance>,
        ended: bool,
        cancelled: bool,
        entered: bool,
    }

    #[ink(event)]
    pub struct BidPlaced { #[ink(topic)] bidder: AccountId, amount: Balance, #[ink(topic)] previous_bidder: Option<AccountId> }
    #[ink(event)]
    pub struct AuctionEnded { #[ink(topic)] winner: Option<AccountId>, amount: Balance, settled: bool }
    #[ink(event)]
    pub struct AuctionCancelled { #[ink(topic)] by: AccountId, refunded: Option<AccountId>, amount: Balance }

    impl EnglishAuction {
        #[ink(constructor)]
        pub fn new(seller: AccountId, minimum_bid: Balance, bid_increment: Balance, auction_duration: BlockNumber) -> Self {
            let start_block = Self::env().block_number();
            Self { seller, minimum_bid, bid_increment, reserve_price: minimum_bid, start_block, end_block: start_block.saturating_add(auction_duration), highest_bidder: None, highest_bid: 0, bid_history: Mapping::default(), ended: false, cancelled: false, entered: false }
        }
        fn ensure_not_entered(&mut self) -> Result<(), Error> {
            if self.entered { return Err(Error::ReentrantCall); }
            self.entered = true; Ok(())
        }
        fn exit(&mut self) { self.entered = false; }

        #[ink(message, payable)]
        pub fn bid(&mut self) -> Result<(), Error> {
            if self.cancelled { return Err(Error::AlreadyCancelled); }
            if self.ended { return Err(Error::AuctionEnded); }
            if self.env().block_number() > self.end_block { return Err(Error::AuctionEnded); }
            let amount = self.env().transferred_value();
            let required = if self.highest_bidder.is_some() {
                self.highest_bid.checked_add(self.bid_increment).ok_or(Error::Overflow)?
            } else { self.minimum_bid };
            if amount < required { return Err(Error::BidTooLow); }
            let previous_bidder = self.highest_bidder;
            let previous_amount = self.highest_bid;
            let caller = self.env().caller();
            self.bid_history.insert(caller, &amount);
            self.highest_bidder = Some(caller);
            self.highest_bid = amount;
            if let Some(prev) = previous_bidder {
                if previous_amount > 0 {
                    self.env().transfer(prev, previous_amount).map_err(|_| Error::TransferFailed)?;
                }
            }
            self.env().emit_event(BidPlaced { bidder: caller, amount, previous_bidder });
            Ok(())
        }

        #[ink(message)]
        pub fn end_auction(&mut self) -> Result<(), Error> {
            self.ensure_not_entered()?;
            let result = self.end_auction_inner();
            self.exit();
            result
        }
        fn end_auction_inner(&mut self) -> Result<(), Error> {
            if self.cancelled { return Err(Error::AlreadyCancelled); }
            if self.ended { return Err(Error::AlreadyEnded); }
            if self.env().block_number() <= self.end_block { return Err(Error::AuctionNotEnded); }
            if self.highest_bid < self.reserve_price || self.highest_bidder.is_none() { return Err(Error::ReserveNotMet); }
            self.ended = true;
            let winner = self.highest_bidder;
            let amount = self.highest_bid;
            self.env().transfer(self.seller, amount).map_err(|_| Error::TransferFailed)?;
            self.env().emit_event(AuctionEnded { winner, amount, settled: true });
            Ok(())
        }

        #[ink(message)]
        pub fn cancel_auction(&mut self) -> Result<(), Error> {
            self.ensure_not_entered()?;
            let result = self.cancel_inner();
            self.exit();
            result
        }
        fn cancel_inner(&mut self) -> Result<(), Error> {
            let caller = self.env().caller();
            if caller != self.seller { return Err(Error::Unauthorized); }
            if self.ended { return Err(Error::AlreadyEnded); }
            if self.cancelled { return Err(Error::AlreadyCancelled); }
            self.cancelled = true;
            let refunded = self.highest_bidder;
            let amount = self.highest_bid;
            if let Some(bidder) = refunded {
                if amount > 0 { self.env().transfer(bidder, amount).map_err(|_| Error::TransferFailed)?; }
            }
            self.env().emit_event(AuctionCancelled { by: caller, refunded, amount });
            Ok(())
        }

        #[ink(message)] pub fn get_highest_bid(&self) -> (Option<AccountId>, Balance) { (self.highest_bidder, self.highest_bid) }
        #[ink(message)] pub fn get_bid(&self, bidder: AccountId) -> Option<Balance> { self.bid_history.get(bidder) }
        #[ink(message)] pub fn is_active(&self) -> bool { !self.ended && !self.cancelled && self.env().block_number() <= self.end_block }
        #[ink(message)] pub fn seller(&self) -> AccountId { self.seller }
        #[ink(message)] pub fn minimum_bid(&self) -> Balance { self.minimum_bid }
        #[ink(message)] pub fn reserve_price(&self) -> Balance { self.reserve_price }
        #[ink(message)] pub fn end_block(&self) -> BlockNumber { self.end_block }
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        fn accounts() -> ink::env::test::DefaultAccounts<ink::env::DefaultEnvironment> { ink::env::test::default_accounts::<ink::env::DefaultEnvironment>() }
        fn set_block(b: BlockNumber) { ink::env::test::set_block_number::<ink::env::DefaultEnvironment>(b); }
        fn set_caller(c: AccountId) { ink::env::test::set_caller::<ink::env::DefaultEnvironment>(c); }
        fn set_value(v: Balance) { ink::env::test::set_value_transferred::<ink::env::DefaultEnvironment>(v); }

        #[ink::test]
        fn bid_and_settle() {
            let a = accounts(); set_block(1);
            let mut auction = EnglishAuction::new(a.alice, 100, 10, 50);
            set_caller(a.bob); set_value(120); auction.bid().unwrap();
            set_block(60); set_caller(a.alice);
            assert!(auction.end_auction().is_ok());
        }

        #[ink::test]
        fn bid_requires_increment() {
            let a = accounts(); set_block(1);
            let mut auction = EnglishAuction::new(a.alice, 100, 10, 50);
            set_caller(a.bob); set_value(120); auction.bid().unwrap();
            set_caller(a.charlie); set_value(125);
            assert_eq!(auction.bid(), Err(Error::BidTooLow));
        }

        #[ink::test]
        fn reserve_blocks_settlement() {
            let a = accounts(); set_block(1);
            let mut auction = EnglishAuction::new(a.alice, 100, 10, 5);
            set_block(10);
            assert_eq!(auction.end_auction(), Err(Error::ReserveNotMet));
        }
    }
}
