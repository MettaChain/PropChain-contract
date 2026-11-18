#![cfg_attr(not(feature = "std"), no_std, no_main)]

#[ink::contract]
mod flash_liquidation_auction {
    use ink::storage::Mapping;

    #[derive(Debug, Clone, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout))]
    pub struct Auction {
        pub position_id: u64,
        pub highest_bid: Balance,
        pub highest_bidder: Option<AccountId>,
        pub end_block: BlockNumber,
        pub settled: bool,
        pub fallback: bool,
        pub deposit: Mapping<AccountId, Balance>,
    }

    #[derive(Debug, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub enum Error {
        NotFound, AuctionActive, AuctionEnded, InvalidBid, ReentrantCall, TransferFailed, Overflow, Unauthorized,
    }

    #[ink(storage)]
    pub struct FlashLiquidationAuction {
        admin: AccountId,
        auction_blocks: BlockNumber,
        auctions: Mapping<u64, Auction>,
        entered: bool,
    }

    #[ink(event)] pub struct LiquidationAuctionStarted { #[ink(topic)] position_id: u64, end_block: BlockNumber }
    #[ink(event)] pub struct LiquidationBidPlaced { #[ink(topic)] position_id: u64, #[ink(topic)] bidder: AccountId, bid_amount: Balance }
    #[ink(event)] pub struct LiquidationAuctionSettled { #[ink(topic)] position_id: u64, winner: Option<AccountId>, amount: Balance }
    #[ink(event)] pub struct LiquidationFallback { #[ink(topic)] position_id: u64 }

    impl FlashLiquidationAuction {
        #[ink(constructor)]
        pub fn new() -> Self {
            Self { admin: Self::env().caller(), auction_blocks: 10, auctions: Mapping::default(), entered: false }
        }

        #[ink(message)]
        pub fn set_auction_blocks(&mut self, blocks: BlockNumber) -> Result<(), Error> {
            if self.env().caller() != self.admin { return Err(Error::Unauthorized); }
            self.auction_blocks = blocks.max(1);
            Ok(())
        }

        fn guard(&mut self) -> Result<(), Error> { if self.entered { return Err(Error::ReentrantCall); } self.entered = true; Ok(()) }
        fn unguard(&mut self) { self.entered = false; }

        #[ink(message)]
        pub fn trigger_liquidation_auction(&mut self, position_id: u64) -> Result<(), Error> {
            let end_block = self.env().block_number().saturating_add(self.auction_blocks);
            self.auctions.insert(position_id, &Auction {
                position_id, highest_bid: 0, highest_bidder: None, end_block, settled: false, fallback: false, deposit: Mapping::default(),
            });
            self.env().emit_event(LiquidationAuctionStarted { position_id, end_block });
            Ok(())
        }

        #[ink(message, payable)]
        pub fn bid_on_liquidation(&mut self, position_id: u64, bid_amount: Balance) -> Result<(), Error> {
            self.guard()?;
            let res = self.bid_inner(position_id, bid_amount);
            self.unguard();
            res
        }
        fn bid_inner(&mut self, position_id: u64, bid_amount: Balance) -> Result<(), Error> {
            let mut auction = self.auctions.get(position_id).ok_or(Error::NotFound)?;
            if self.env().block_number() >= auction.end_block { return Err(Error::AuctionEnded); }
            if auction.settled { return Err(Error::AuctionEnded); }
            if bid_amount <= auction.highest_bid { return Err(Error::InvalidBid); }
            let bidder = self.env().caller();
            let paid = self.env().transferred_value();
            if paid < bid_amount { return Err(Error::InvalidBid); }
            if let Some(prev) = auction.highest_bidder {
                let refund = auction.highest_bid;
                if refund > 0 { self.env().transfer(prev, refund).map_err(|_| Error::TransferFailed)?; }
            }
            auction.highest_bid = bid_amount;
            auction.highest_bidder = Some(bidder);
            auction.deposit.insert(bidder, &paid);
            self.auctions.insert(position_id, &auction);
            self.env().emit_event(LiquidationBidPlaced { position_id, bidder, bid_amount });
            Ok(())
        }

        #[ink(message)]
        pub fn settle_auction(&mut self, position_id: u64) -> Result<(), Error> {
            self.guard()?;
            let res = self.settle_inner(position_id);
            self.unguard();
            res
        }
        fn settle_inner(&mut self, position_id: u64) -> Result<(), Error> {
            let mut auction = self.auctions.get(position_id).ok_or(Error::NotFound)?;
            if auction.settled { return Err(Error::AuctionEnded); }
            if self.env().block_number() < auction.end_block { return Err(Error::AuctionActive); }
            auction.settled = true;
            if let Some(winner) = auction.highest_bidder {
                let amount = auction.highest_bid;
                self.auctions.insert(position_id, &auction);
                self.env().emit_event(LiquidationAuctionSettled { position_id, winner: Some(winner), amount });
            } else {
                auction.fallback = true;
                self.auctions.insert(position_id, &auction);
                self.env().emit_event(LiquidationFallback { position_id });
            }
            Ok(())
        }

        #[ink(message)]
        pub fn fallback_liquidation(&mut self, position_id: u64) -> Result<(), Error> {
            if self.env().caller() != self.admin { return Err(Error::Unauthorized); }
            let mut auction = self.auctions.get(position_id).ok_or(Error::NotFound)?;
            if auction.highest_bidder.is_some() { return Err(Error::AuctionActive); }
            auction.settled = true;
            auction.fallback = true;
            self.auctions.insert(position_id, &auction);
            self.env().emit_event(LiquidationFallback { position_id });
            Ok(())
        }

        #[ink(message)] pub fn get_auction(&self, position_id: u64) -> Option<(Balance, Option<AccountId>, BlockNumber, bool)> {
            self.auctions.get(position_id).map(|a| (a.highest_bid, a.highest_bidder, a.end_block, a.settled))
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        fn accounts() -> ink::env::test::DefaultAccounts<ink::env::DefaultEnvironment> { ink::env::test::default_accounts::<ink::env::DefaultEnvironment>() }
        fn set_caller(c: AccountId) { ink::env::test::set_caller::<ink::env::DefaultEnvironment>(c); }
        fn set_value(v: Balance) { ink::env::test::set_value_transferred::<ink::env::DefaultEnvironment>(v); }
        fn set_block(b: BlockNumber) { ink::env::test::set_block_number::<ink::env::DefaultEnvironment>(b); }

        #[ink::test]
        fn auction_lifecycle() {
            let a = accounts(); set_caller(a.alice); set_block(1);
            let mut la = FlashLiquidationAuction::new();
            la.trigger_liquidation_auction(1).unwrap();
            set_caller(a.bob); set_value(100);
            la.bid_on_liquidation(1, 100).unwrap();
            set_block(20);
            la.settle_auction(1).unwrap();
            let (bid, _, _, settled) = la.get_auction(1).unwrap();
            assert_eq!(bid, 100);
            assert!(settled);
        }

        #[ink::test]
        fn no_bid_fallback() {
            let a = accounts(); set_caller(a.alice); set_block(1);
            let mut la = FlashLiquidationAuction::new();
            la.trigger_liquidation_auction(1).unwrap();
            set_block(20);
            la.settle_auction(1).unwrap();
        }
    }
}
