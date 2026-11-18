#![cfg_attr(not(feature = "std"), no_std, no_main)]

#[ink::contract]
mod yield_vault {
    use ink::storage::Mapping;

    #[derive(Debug, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub enum Error {
        Unauthorized, ZeroAmount, InsufficientShares, ReentrantCall, Overflow, Underflow, NoShares, TransferFailed,
    }

    #[ink(storage)]
    pub struct YieldVault {
        manager: AccountId,
        total_assets: Balance,
        total_shares: Balance,
        balances: Mapping<AccountId, Balance>,
        active_strategy: Option<AccountId>,
        entered: bool,
    }

    #[ink(event)] pub struct Deposit { #[ink(topic)] user: AccountId, amount: Balance, shares_minted: Balance }
    #[ink(event)] pub struct Withdraw { #[ink(topic)] user: AccountId, shares_burned: Balance, amount_returned: Balance }
    #[ink(event)] pub struct StrategyUpdated { #[ink(topic)] by: AccountId, new_strategy: Option<AccountId> }
    #[ink(event)] pub struct YieldHarvested { #[ink(topic)] by: AccountId, amount: Balance, total_assets: Balance }

    impl YieldVault {
        #[ink(constructor)]
        pub fn new(manager: AccountId) -> Self {
            Self { manager, total_assets: 0, total_shares: 0, balances: Mapping::default(), active_strategy: None, entered: false }
        }
        fn ensure_not_entered(&mut self) -> Result<(), Error> {
            if self.entered { return Err(Error::ReentrantCall); }
            self.entered = true; Ok(())
        }
        fn exit(&mut self) { self.entered = false; }
        fn only_manager(&self) -> Result<(), Error> {
            if self.env().caller() != self.manager { return Err(Error::Unauthorized); }
            Ok(())
        }

        #[ink(message, payable)]
        pub fn deposit(&mut self) -> Result<Balance, Error> {
            self.ensure_not_entered()?;
            let res = self.deposit_inner();
            self.exit();
            res
        }
        fn deposit_inner(&mut self) -> Result<Balance, Error> {
            let amount = self.env().transferred_value();
            if amount == 0 { return Err(Error::ZeroAmount); }
            let caller = self.env().caller();
            let shares_to_mint = if self.total_shares == 0 || self.total_assets == 0 { amount } else {
                amount.checked_mul(self.total_shares).ok_or(Error::Overflow)?.checked_div(self.total_assets).ok_or(Error::Overflow)?
            };
            if shares_to_mint == 0 { return Err(Error::Overflow); }
            self.total_assets = self.total_assets.checked_add(amount).ok_or(Error::Overflow)?;
            self.total_shares = self.total_shares.checked_add(shares_to_mint).ok_or(Error::Overflow)?;
            let prev = self.balances.get(caller).unwrap_or(0);
            self.balances.insert(caller, &(prev.checked_add(shares_to_mint).ok_or(Error::Overflow)?));
            self.env().emit_event(Deposit { user: caller, amount, shares_minted: shares_to_mint });
            Ok(shares_to_mint)
        }

        #[ink(message)]
        pub fn withdraw(&mut self, shares: Balance) -> Result<Balance, Error> {
            self.ensure_not_entered()?;
            let res = self.withdraw_inner(shares);
            self.exit();
            res
        }
        fn withdraw_inner(&mut self, shares: Balance) -> Result<Balance, Error> {
            if shares == 0 { return Err(Error::ZeroAmount); }
            let caller = self.env().caller();
            let user_shares = self.balances.get(caller).unwrap_or(0);
            if user_shares < shares { return Err(Error::InsufficientShares); }
            if self.total_shares == 0 { return Err(Error::NoShares); }
            let amount = shares.checked_mul(self.total_assets).ok_or(Error::Overflow)?.checked_div(self.total_shares).ok_or(Error::Overflow)?;
            self.balances.insert(caller, &(user_shares - shares));
            self.total_shares = self.total_shares.checked_sub(shares).ok_or(Error::Underflow)?;
            self.total_assets = self.total_assets.checked_sub(amount).ok_or(Error::Underflow)?;
            self.env().transfer(caller, amount).map_err(|_| Error::TransferFailed)?;
            self.env().emit_event(Withdraw { user: caller, shares_burned: shares, amount_returned: amount });
            Ok(amount)
        }

        #[ink(message)]
        pub fn set_strategy(&mut self, strategy_address: Option<AccountId>) -> Result<(), Error> {
            self.only_manager()?;
            self.active_strategy = strategy_address;
            self.env().emit_event(StrategyUpdated { by: self.env().caller(), new_strategy: strategy_address });
            Ok(())
        }

        #[ink(message, payable)]
        pub fn harvest_yield(&mut self) -> Result<(), Error> {
            self.only_manager()?;
            let amount = self.env().transferred_value();
            if amount == 0 { return Err(Error::ZeroAmount); }
            self.total_assets = self.total_assets.checked_add(amount).ok_or(Error::Overflow)?;
            self.env().emit_event(YieldHarvested { by: self.env().caller(), amount, total_assets: self.total_assets });
            Ok(())
        }

        #[ink(message)] pub fn balance_of(&self, account: AccountId) -> Balance { self.balances.get(account).unwrap_or(0) }
        #[ink(message)] pub fn total_assets(&self) -> Balance { self.total_assets }
        #[ink(message)] pub fn total_shares(&self) -> Balance { self.total_shares }
        #[ink(message)] pub fn active_strategy(&self) -> Option<AccountId> { self.active_strategy }
        #[ink(message)] pub fn manager(&self) -> AccountId { self.manager }
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        fn accounts() -> ink::env::test::DefaultAccounts<ink::env::DefaultEnvironment> { ink::env::test::default_accounts::<ink::env::DefaultEnvironment>() }
        fn set_caller(c: AccountId) { ink::env::test::set_caller::<ink::env::DefaultEnvironment>(c); }
        fn set_value(v: Balance) { ink::env::test::set_value_transferred::<ink::env::DefaultEnvironment>(v); }

        #[ink::test]
        fn deposit_mints_initial_shares_one_to_one() {
            let a = accounts(); let mut vault = YieldVault::new(a.alice);
            set_caller(a.bob); set_value(1000);
            let shares = vault.deposit().unwrap(); assert_eq!(shares, 1000);
        }

        #[ink::test]
        fn share_price_increases_with_harvest() {
            let a = accounts(); let mut vault = YieldVault::new(a.alice);
            set_caller(a.bob); set_value(1000); vault.deposit().unwrap();
            set_caller(a.alice); set_value(500); vault.harvest_yield().unwrap();
            set_caller(a.charlie); set_value(1500);
            assert_eq!(vault.deposit().unwrap(), 1000);
        }

        #[ink::test]
        fn withdraw_proportional() {
            let a = accounts(); let mut vault = YieldVault::new(a.alice);
            set_caller(a.bob); set_value(1000); vault.deposit().unwrap();
            assert_eq!(vault.withdraw(500).unwrap(), 500);
        }
    }
}
