#![cfg_attr(not(feature = "std"), no_std, no_main)]

#[ink::contract]
mod nft_marketplace {
    use ink::storage::Mapping;

    #[derive(Debug, Clone, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout))]
    pub struct Listing { pub seller: AccountId, pub token_id: u64, pub price: Balance, pub expires_at: BlockNumber, pub active: bool }
    #[derive(Debug, Clone, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout))]
    pub struct Offer { pub buyer: AccountId, pub token_id: u64, pub price: Balance, pub expires_at: BlockNumber, pub active: bool }

    #[derive(Debug, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub enum Error {
        Unauthorized, NotFound, Inactive, Expired, WrongPrice, ReentrantCall, TransferFailed, Overflow,
    }

    #[ink(storage)]
    pub struct NftMarketplace {
        admin: AccountId,
        fee_bps: u32,
        next_listing_id: u64,
        next_offer_id: u64,
        listings: Mapping<u64, Listing>,
        offers: Mapping<u64, Offer>,
        owners: Mapping<u64, AccountId>,
        entered: bool,
    }

    #[ink(event)] pub struct Listed { #[ink(topic)] listing_id: u64, #[ink(topic)] seller: AccountId, token_id: u64, price: Balance }
    #[ink(event)] pub struct Sold { #[ink(topic)] listing_id: u64, #[ink(topic)] buyer: AccountId, token_id: u64, price: Balance }
    #[ink(event)] pub struct OfferPlaced { #[ink(topic)] offer_id: u64, #[ink(topic)] buyer: AccountId, token_id: u64, price: Balance }
    #[ink(event)] pub struct OfferAccepted { #[ink(topic)] offer_id: u64, #[ink(topic)] seller: AccountId, token_id: u64, price: Balance }
    #[ink(event)] pub struct ListingCancelled { #[ink(topic)] listing_id: u64 }
    #[ink(event)] pub struct OfferCancelled { #[ink(topic)] offer_id: u64 }
    #[ink(event)] pub struct FeeUpdated { #[ink(topic)] by: AccountId, fee_bps: u32 }

    impl NftMarketplace {
        #[ink(constructor)]
        pub fn new(fee_bps: u32) -> Self {
            Self { admin: Self::env().caller(), fee_bps: fee_bps.min(10_000), next_listing_id: 1, next_offer_id: 1, listings: Mapping::default(), offers: Mapping::default(), owners: Mapping::default(), entered: false }
        }
        fn guard(&mut self) -> Result<(), Error> { if self.entered { return Err(Error::ReentrantCall); } self.entered = true; Ok(()) }
        fn unguard(&mut self) { self.entered = false; }

        #[ink(message)]
        pub fn register_owner(&mut self, token_id: u64, owner: AccountId) -> Result<(), Error> {
            if self.env().caller() != self.admin { return Err(Error::Unauthorized); }
            self.owners.insert(token_id, &owner); Ok(())
        }
        fn owner_of(&self, token_id: u64) -> Option<AccountId> { self.owners.get(token_id) }

        #[ink(message)]
        pub fn list_nft(&mut self, token_id: u64, price: Balance, expires_at: BlockNumber) -> Result<u64, Error> {
            let caller = self.env().caller();
            if self.owner_of(token_id) != Some(caller) { return Err(Error::Unauthorized); }
            let id = self.next_listing_id;
            self.next_listing_id = id.saturating_add(1);
            self.listings.insert(id, &Listing { seller: caller, token_id, price, expires_at, active: true });
            self.env().emit_event(Listed { listing_id: id, seller: caller, token_id, price });
            Ok(id)
        }

        #[ink(message, payable)]
        pub fn buy_nft(&mut self, listing_id: u64) -> Result<(), Error> {
            self.guard()?; let res = self.buy_inner(listing_id); self.unguard(); res
        }
        fn buy_inner(&mut self, listing_id: u64) -> Result<(), Error> {
            let mut listing = self.listings.get(listing_id).ok_or(Error::NotFound)?;
            if !listing.active { return Err(Error::Inactive); }
            if self.env().block_number() > listing.expires_at { return Err(Error::Expired); }
            let paid = self.env().transferred_value();
            if paid < listing.price { return Err(Error::WrongPrice); }
            let buyer = self.env().caller();
            listing.active = false;
            self.listings.insert(listing_id, &listing);
            self.owners.insert(listing.token_id, &buyer);
            let fee = paid.saturating_mul(self.fee_bps as u128) / 10_000u128;
            let seller_amount = paid.saturating_sub(fee);
            self.env().transfer(listing.seller, seller_amount).map_err(|_| Error::TransferFailed)?;
            if fee > 0 { self.env().transfer(self.admin, fee).map_err(|_| Error::TransferFailed)?; }
            self.env().emit_event(Sold { listing_id, buyer, token_id: listing.token_id, price: paid });
            Ok(())
        }

        #[ink(message, payable)]
        pub fn place_offer(&mut self, token_id: u64, _price: Balance, expires_at: BlockNumber) -> Result<u64, Error> {
            let buyer = self.env().caller();
            let price = self.env().transferred_value();
            if price == 0 { return Err(Error::WrongPrice); }
            let id = self.next_offer_id;
            self.next_offer_id = id.saturating_add(1);
            self.offers.insert(id, &Offer { buyer, token_id, price, expires_at, active: true });
            self.env().emit_event(OfferPlaced { offer_id: id, buyer, token_id, price });
            Ok(id)
        }

        #[ink(message)]
        pub fn accept_offer(&mut self, offer_id: u64) -> Result<(), Error> {
            self.guard()?; let res = self.accept_offer_inner(offer_id); self.unguard(); res
        }
        fn accept_offer_inner(&mut self, offer_id: u64) -> Result<(), Error> {
            let mut offer = self.offers.get(offer_id).ok_or(Error::NotFound)?;
            if !offer.active { return Err(Error::Inactive); }
            if self.env().block_number() > offer.expires_at { return Err(Error::Expired); }
            let seller = self.env().caller();
            if self.owner_of(offer.token_id) != Some(seller) { return Err(Error::Unauthorized); }
            offer.active = false;
            self.offers.insert(offer_id, &offer);
            self.owners.insert(offer.token_id, &offer.buyer);
            let fee = offer.price.saturating_mul(self.fee_bps as u128) / 10_000u128;
            let seller_amount = offer.price.saturating_sub(fee);
            self.env().transfer(seller, seller_amount).map_err(|_| Error::TransferFailed)?;
            if fee > 0 { self.env().transfer(self.admin, fee).map_err(|_| Error::TransferFailed)?; }
            self.env().emit_event(OfferAccepted { offer_id, seller, token_id: offer.token_id, price: offer.price });
            Ok(())
        }

        #[ink(message)]
        pub fn cancel_listing(&mut self, listing_id: u64) -> Result<(), Error> {
            let caller = self.env().caller();
            let mut listing = self.listings.get(listing_id).ok_or(Error::NotFound)?;
            if listing.seller != caller { return Err(Error::Unauthorized); }
            listing.active = false;
            self.listings.insert(listing_id, &listing);
            self.env().emit_event(ListingCancelled { listing_id });
            Ok(())
        }

        #[ink(message)]
        pub fn cancel_offer(&mut self, offer_id: u64) -> Result<(), Error> {
            self.guard()?; let res = self.cancel_offer_inner(offer_id); self.unguard(); res
        }
        fn cancel_offer_inner(&mut self, offer_id: u64) -> Result<(), Error> {
            let caller = self.env().caller();
            let mut offer = self.offers.get(offer_id).ok_or(Error::NotFound)?;
            if offer.buyer != caller { return Err(Error::Unauthorized); }
            if !offer.active { return Err(Error::Inactive); }
            offer.active = false;
            self.offers.insert(offer_id, &offer);
            self.env().transfer(caller, offer.price).map_err(|_| Error::TransferFailed)?;
            self.env().emit_event(OfferCancelled { offer_id });
            Ok(())
        }

        #[ink(message)]
        pub fn set_fee_bps(&mut self, fee_bps: u32) -> Result<(), Error> {
            if self.env().caller() != self.admin { return Err(Error::Unauthorized); }
            self.fee_bps = fee_bps.min(10_000);
            self.env().emit_event(FeeUpdated { by: self.env().caller(), fee_bps: self.fee_bps });
            Ok(())
        }

        #[ink(message)] pub fn get_listing(&self, id: u64) -> Option<Listing> { self.listings.get(id) }
        #[ink(message)] pub fn get_offer(&self, id: u64) -> Option<Offer> { self.offers.get(id) }
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        fn accounts() -> ink::env::test::DefaultAccounts<ink::env::DefaultEnvironment> { ink::env::test::default_accounts::<ink::env::DefaultEnvironment>() }
        fn set_caller(c: AccountId) { ink::env::test::set_caller::<ink::env::DefaultEnvironment>(c); }
        fn set_block(b: BlockNumber) { ink::env::test::set_block_number::<ink::env::DefaultEnvironment>(b); }
        fn set_value(v: Balance) { ink::env::test::set_value_transferred::<ink::env::DefaultEnvironment>(v); }

        #[ink::test]
        fn list_and_buy() {
            let a = accounts(); set_caller(a.alice);
            let mut m = NftMarketplace::new(100);
            m.register_owner(1, a.bob).unwrap();
            set_caller(a.bob); set_block(1);
            let id = m.list_nft(1, 1000, 10).unwrap();
            set_caller(a.charlie); set_value(1000);
            assert!(m.buy_nft(id).is_ok());
        }

        #[ink::test]
        fn offer_and_accept() {
            let a = accounts(); set_caller(a.alice);
            let mut m = NftMarketplace::new(0);
            m.register_owner(2, a.bob).unwrap();
            set_caller(a.charlie); set_value(500);
            let oid = m.place_offer(2, 500, 10).unwrap();
            set_caller(a.bob);
            assert!(m.accept_offer(oid).is_ok());
        }
    }
}
