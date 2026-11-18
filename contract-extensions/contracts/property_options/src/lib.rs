#![cfg_attr(not(feature = "std"), no_std, no_main)]

#[ink::contract]
mod property_options {
    use ink::storage::Mapping;

    #[derive(Debug, Clone, Copy, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub enum OptionType { Call, Put }

    #[derive(Debug, Clone, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout))]
    pub struct OptionContract {
        pub writer: AccountId,
        pub buyer: Option<AccountId>,
        pub option_type: OptionType,
        pub strike_price: Balance,
        pub amount: Balance,
        pub expiry_block: BlockNumber,
        pub premium: Balance,
        pub collateral: Balance,
        pub exercised: bool,
        pub expired_withdrawn: bool,
    }

    #[derive(Debug, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub enum Error {
        Unauthorized, NotFound, WrongPrice, NotExpired, Expired, AlreadyHandled, ReentrantCall, TransferFailed, Overflow,
    }

    #[ink(storage)]
    pub struct PropertyOptions {
        next_id: u64,
        options: Mapping<u64, OptionContract>,
        entered: bool,
    }

    #[ink(event)]
    pub struct OptionWritten { #[ink(topic)] option_id: u64, #[ink(topic)] writer: AccountId, option_type: OptionType, strike_price: Balance, amount: Balance, expiry_block: BlockNumber, premium: Balance }
    #[ink(event)]
    pub struct OptionBought { #[ink(topic)] option_id: u64, #[ink(topic)] buyer: AccountId, premium: Balance }
    #[ink(event)]
    pub struct OptionExercised { #[ink(topic)] option_id: u64, #[ink(topic)] buyer: AccountId, payout: Balance }
    #[ink(event)]
    pub struct OptionExpired { #[ink(topic)] option_id: u64, #[ink(topic)] writer: AccountId, collateral_returned: Balance }

    impl PropertyOptions {
        #[ink(constructor)]
        pub fn new() -> Self { Self { next_id: 1, options: Mapping::default(), entered: false } }

        fn guard(&mut self) -> Result<(), Error> {
            if self.entered { return Err(Error::ReentrantCall); }
            self.entered = true;
            Ok(())
        }
        fn unguard(&mut self) { self.entered = false; }

        #[ink(message, payable)]
        pub fn write_option(&mut self, option_type: OptionType, strike_price: Balance, amount: Balance, expiry_block: BlockNumber, premium: Balance) -> Result<u64, Error> {
            let collateral = self.env().transferred_value();
            let required = match option_type {
                OptionType::Call => amount,
                OptionType::Put => strike_price.checked_mul(amount).ok_or(Error::Overflow)?,
            };
            if collateral < required { return Err(Error::WrongPrice); }
            let writer = self.env().caller();
            let id = self.next_id;
            self.next_id = id.saturating_add(1);
            self.options.insert(id, &OptionContract {
                writer, buyer: None, option_type, strike_price, amount, expiry_block, premium, collateral, exercised: false, expired_withdrawn: false,
            });
            self.env().emit_event(OptionWritten { option_id: id, writer, option_type, strike_price, amount, expiry_block, premium });
            Ok(id)
        }

        #[ink(message, payable)]
        pub fn buy_option(&mut self, option_id: u64) -> Result<(), Error> {
            self.guard()?;
            let res = (|| {
                let mut opt = self.options.get(option_id).ok_or(Error::NotFound)?;
                if opt.buyer.is_some() || opt.exercised || opt.expired_withdrawn { return Err(Error::AlreadyHandled); }
                if self.env().block_number() >= opt.expiry_block { return Err(Error::Expired); }
                let paid = self.env().transferred_value();
                if paid < opt.premium { return Err(Error::WrongPrice); }
                let buyer = self.env().caller();
                opt.buyer = Some(buyer);
                self.options.insert(option_id, &opt);
                self.env().transfer(opt.writer, paid).map_err(|_| Error::TransferFailed)?;
                self.env().emit_event(OptionBought { option_id, buyer, premium: paid });
                Ok(())
            })();
            self.unguard();
            res
        }

        #[ink(message, payable)]
        pub fn exercise(&mut self, option_id: u64) -> Result<Balance, Error> {
            self.guard()?;
            let res = (|| {
                let mut opt = self.options.get(option_id).ok_or(Error::NotFound)?;
                let buyer = opt.buyer.ok_or(Error::Unauthorized)?;
                if self.env().caller() != buyer { return Err(Error::Unauthorized); }
                if opt.exercised || opt.expired_withdrawn { return Err(Error::AlreadyHandled); }
                let now = self.env().block_number();
                if now != opt.expiry_block { return if now < opt.expiry_block { Err(Error::NotExpired) } else { Err(Error::Expired) }; }
                opt.exercised = true;
                self.options.insert(option_id, &opt);
                let payout = opt.collateral;
                match opt.option_type {
                    OptionType::Call => {
                        let cost = opt.strike_price.checked_mul(opt.amount).ok_or(Error::Overflow)?;
                        let paid = self.env().transferred_value();
                        if paid < cost { return Err(Error::WrongPrice); }
                        self.env().transfer(opt.writer, cost).map_err(|_| Error::TransferFailed)?;
                        self.env().transfer(buyer, payout).map_err(|_| Error::TransferFailed)?;
                    }
                    OptionType::Put => {
                        self.env().transfer(buyer, payout).map_err(|_| Error::TransferFailed)?;
                    }
                }
                self.env().emit_event(OptionExercised { option_id, buyer, payout });
                Ok(payout)
            })();
            self.unguard();
            res
        }

        #[ink(message)]
        pub fn expire_unexercised(&mut self, option_id: u64) -> Result<(), Error> {
            self.guard()?;
            let res = (|| {
                let mut opt = self.options.get(option_id).ok_or(Error::NotFound)?;
                if self.env().caller() != opt.writer { return Err(Error::Unauthorized); }
                if opt.exercised || opt.expired_withdrawn { return Err(Error::AlreadyHandled); }
                if self.env().block_number() <= opt.expiry_block { return Err(Error::NotExpired); }
                opt.expired_withdrawn = true;
                self.options.insert(option_id, &opt);
                let collateral = opt.collateral;
                self.env().transfer(opt.writer, collateral).map_err(|_| Error::TransferFailed)?;
                self.env().emit_event(OptionExpired { option_id, writer: opt.writer, collateral_returned: collateral });
                Ok(())
            })();
            self.unguard();
            res
        }

        #[ink(message)]
        pub fn get_option(&self, option_id: u64) -> Option<OptionContract> { self.options.get(option_id) }
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        fn accounts() -> ink::env::test::DefaultAccounts<ink::env::DefaultEnvironment> { ink::env::test::default_accounts::<ink::env::DefaultEnvironment>() }
        fn set_caller(c: AccountId) { ink::env::test::set_caller::<ink::env::DefaultEnvironment>(c); }
        fn set_value(v: Balance) { ink::env::test::set_value_transferred::<ink::env::DefaultEnvironment>(v); }
        fn set_block(b: BlockNumber) { ink::env::test::set_block_number::<ink::env::DefaultEnvironment>(b); }

        #[ink::test]
        fn write_buy_exercise_exercised() {
            let a = accounts(); set_caller(a.alice); set_block(1);
            let mut c = PropertyOptions::new();
            set_value(1000);
            let id = c.write_option(OptionType::Call, 10, 1, 5, 50).unwrap();
            set_caller(a.bob); set_value(50); c.buy_option(id).unwrap();
            set_block(5); set_value(10); c.exercise(id).unwrap();
        }

        #[ink::test]
        fn expire_unexercised_returns_collateral() {
            let a = accounts(); set_caller(a.alice); set_block(1);
            let mut c = PropertyOptions::new();
            set_value(500);
            let id = c.write_option(OptionType::Call, 10, 1, 5, 50).unwrap();
            set_block(10);
            c.expire_unexercised(id).unwrap();
        }
    }
}
