#![cfg_attr(not(feature = "std"), no_std, no_main)]

#[ink::contract]
mod flash_loan {
    use ink::env::call::{build_call, ExecutionInput, Selector};
    use ink::env::DefaultEnvironment;
    use ink::prelude::vec::Vec;
    use ink::storage::Mapping;

    #[derive(Debug, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub enum Error {
        Unauthorized,
        TokenNotSupported,
        InsufficientLiquidity,
        CallbackFailed,
        RepaymentFailed,
        ReentrantCall,
        Overflow,
        TransferFailed,
    }

    #[ink(storage)]
    pub struct FlashLoan {
        admin: AccountId,
        fee_bps: u32, // 9 bps default = 0.09%
        supported_tokens: Mapping<AccountId, bool>,
        accumulated_fees: Mapping<AccountId, Balance>,
        /// per-token reentrancy guard
        active: Mapping<AccountId, bool>,
    }

    #[ink(event)]
    pub struct FlashLoanExecuted {
        #[ink(topic)]
        borrower: AccountId,
        #[ink(topic)]
        token: AccountId,
        amount: Balance,
        fee: Balance,
    }

    #[ink(event)]
    pub struct FlashLoanRepaid {
        #[ink(topic)]
        borrower: AccountId,
        #[ink(topic)]
        token: AccountId,
        amount: Balance,
        fee: Balance,
    }

    impl FlashLoan {
        #[ink(constructor)]
        pub fn new() -> Self {
            Self {
                admin: Self::env().caller(),
                fee_bps: 9,
                supported_tokens: Mapping::default(),
                accumulated_fees: Mapping::default(),
                active: Mapping::default(),
            }
        }

        fn only_admin(&self) -> Result<(), Error> {
            if self.env().caller() != self.admin { return Err(Error::Unauthorized); }
            Ok(())
        }

        #[ink(message)]
        pub fn set_fee_bps(&mut self, fee_bps: u32) -> Result<(), Error> {
            self.only_admin()?;
            self.fee_bps = fee_bps.min(10_000);
            Ok(())
        }

        #[ink(message)]
        pub fn register_token(&mut self, token: AccountId, supported: bool) -> Result<(), Error> {
            self.only_admin()?;
            self.supported_tokens.insert(token, &supported);
            Ok(())
        }

        #[ink(message)]
        pub fn withdraw_fees(&mut self, token: AccountId) -> Result<Balance, Error> {
            self.only_admin()?;
            let fees = self.accumulated_fees.get(token).unwrap_or(0);
            if fees == 0 { return Ok(0); }
            self.accumulated_fees.insert(token, &0);
            // Native fee withdrawal model: transfer balance out
            self.env().transfer(self.admin, fees).map_err(|_| Error::TransferFailed)?;
            Ok(fees)
        }

        /// Initiates flash loan. Transfers native tokens to callback address and invokes
        /// `on_flash_loan(token, amount, fee, data)` on it. Expects repayment (principal+fee)
        /// to be returned to this contract as value in a follow-up or be present; for this
        /// standalone extension we verify contract balance delta >= amount+fee.
        #[ink(message)]
        pub fn flash_loan(
            &mut self,
            token: AccountId,
            amount: Balance,
            callback_addr: AccountId,
            data: Vec<u8>,
        ) -> Result<(), Error> {
            if !self.supported_tokens.get(token).unwrap_or(false) {
                return Err(Error::TokenNotSupported);
            }
            if self.active.get(token).unwrap_or(false) {
                return Err(Error::ReentrantCall);
            }
            if amount == 0 { return Err(Error::InsufficientLiquidity); }
            self.active.insert(token, &true);

            let fee = amount.saturating_mul(self.fee_bps as u128) / 10_000u128;
            let borrower = self.env().caller();
            let balance_before = self.env().balance();

            if balance_before < amount {
                self.active.insert(token, &false);
                return Err(Error::InsufficientLiquidity);
            }

            // send funds to callback
            if self.env().transfer(callback_addr, amount).is_err() {
                self.active.insert(token, &false);
                return Err(Error::TransferFailed);
            }

            self.env().emit_event(FlashLoanExecuted {
                borrower,
                token,
                amount,
                fee,
            });

            // callback: on_flash_loan(token, amount, fee, data) -> bool/Result expected but we treat call success as ack
            let cb_result = build_call::<DefaultEnvironment>()
                .call(callback_addr)
                .transferred_value(0)
                .exec_input(
                    ExecutionInput::new(Selector::new(ink::selector_bytes!("on_flash_loan")))
                        .push_arg(token)
                        .push_arg(amount)
                        .push_arg(fee)
                        .push_arg(data),
                )
                .returns::<()>()
                .invoke();

            if cb_result.is_err() {
                self.active.insert(token, &false);
                return Err(Error::CallbackFailed);
            }

            // verify repayment: balance after should be >= before + fee (loan returned plus fee)
            let balance_after = self.env().balance();
            let required = balance_before.saturating_add(fee);
            if balance_after < required {
                self.active.insert(token, &false);
                return Err(Error::RepaymentFailed);
            }

            let accrued = self.accumulated_fees.get(token).unwrap_or(0).saturating_add(fee);
            self.accumulated_fees.insert(token, &accrued);
            self.active.insert(token, &false);

            self.env().emit_event(FlashLoanRepaid { borrower, token, amount, fee });
            Ok(())
        }

        #[ink(message)]
        pub fn fee_bps(&self) -> u32 { self.fee_bps }
        #[ink(message)]
        pub fn is_supported(&self, token: AccountId) -> bool { self.supported_tokens.get(token).unwrap_or(false) }
        #[ink(message)]
        pub fn accumulated_fee(&self, token: AccountId) -> Balance { self.accumulated_fees.get(token).unwrap_or(0) }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        fn accounts() -> ink::env::test::DefaultAccounts<ink::env::DefaultEnvironment> {
            ink::env::test::default_accounts::<ink::env::DefaultEnvironment>()
        }
        fn set_caller(c: AccountId) { ink::env::test::set_caller::<ink::env::DefaultEnvironment>(c); }

        #[ink::test]
        fn defaults_and_admin_controls() {
            let a = accounts();
            set_caller(a.alice);
            let mut fl = FlashLoan::new();
            assert_eq!(fl.fee_bps(), 9);
            fl.set_fee_bps(20).unwrap();
            assert_eq!(fl.fee_bps(), 20);
            fl.register_token(a.bob, true).unwrap();
            assert!(fl.is_supported(a.bob));
            set_caller(a.charlie);
            assert_eq!(fl.set_fee_bps(1), Err(Error::Unauthorized));
        }

        #[ink::test]
        fn unsupported_token_rejected() {
            let a = accounts();
            set_caller(a.alice);
            let mut fl = FlashLoan::new();
            assert_eq!(fl.flash_loan(a.bob, 100, a.charlie, vec![]), Err(Error::TokenNotSupported));
        }
    }
}
