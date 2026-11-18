#![cfg_attr(not(feature = "std"), no_std, no_main)]

#[ink::contract]
mod liquid_staking_derivative {
    use ink::storage::Mapping;

    #[derive(Debug, Clone, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout))]
    pub struct WithdrawalRequest { pub st_amount: Balance, pub unlock_block: BlockNumber, pub claimed: bool }

    #[derive(Debug, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub enum Error {
        Unauthorized, ZeroAmount, InsufficientShares, NothingToWithdraw, TooEarly,
        ReentrantCall, Overflow, Underflow, TransferFailed,
    }

    #[ink(storage)]
    pub struct LiquidStakingDerivative {
        admin: AccountId,
        total_staked: Balance,
        total_st_shares: Balance,
        unbonding_blocks: BlockNumber,
        balances: Mapping<AccountId, Balance>,
        withdrawals: Mapping<(AccountId, u64), WithdrawalRequest>,
        next_withdrawal_id: Mapping<AccountId, u64>,
        entered: bool,
    }

    #[ink(event)] pub struct Staked { #[ink(topic)] user: AccountId, amount: Balance, st_minted: Balance }
    #[ink(event)] pub struct Unstaked { #[ink(topic)] user: AccountId, st_burned: Balance, unlock_block: BlockNumber, req_id: u64 }
    #[ink(event)] pub struct Withdrawn { #[ink(topic)] user: AccountId, amount: Balance, req_id: u64 }
    #[ink(event)] pub struct ExchangeRateUpdated { #[ink(topic)] by: AccountId, total_staked: Balance, total_shares: Balance }

    impl LiquidStakingDerivative {
        #[ink(constructor)]
        pub fn new(unbonding_blocks: BlockNumber) -> Self {
            Self { admin: Self::env().caller(), total_staked: 0, total_st_shares: 0, unbonding_blocks, balances: Mapping::default(), withdrawals: Mapping::default(), next_withdrawal_id: Mapping::default(), entered: false }
        }
        fn guard(&mut self) -> Result<(), Error> { if self.entered { return Err(Error::ReentrantCall); } self.entered = true; Ok(()) }
        fn unguard(&mut self) { self.entered = false; }
        fn only_admin(&self) -> Result<(), Error> { if self.env().caller() != self.admin { return Err(Error::Unauthorized); } Ok(()) }

        #[ink(message)] pub fn get_exchange_rate(&self) -> (Balance, Balance) { if self.total_st_shares == 0 { return (1, 1); } (self.total_staked, self.total_st_shares) }

        #[ink(message, payable)]
        pub fn stake(&mut self) -> Result<Balance, Error> { self.guard()?; let r = self.stake_inner(); self.unguard(); r }
        fn stake_inner(&mut self) -> Result<Balance, Error> {
            let amount = self.env().transferred_value();
            if amount == 0 { return Err(Error::ZeroAmount); }
            let caller = self.env().caller();
            let st_minted = if self.total_st_shares == 0 || self.total_staked == 0 { amount } else {
                amount.checked_mul(self.total_st_shares).ok_or(Error::Overflow)?.checked_div(self.total_staked).ok_or(Error::Overflow)?
            };
            if st_minted == 0 { return Err(Error::Overflow); }
            self.total_staked = self.total_staked.checked_add(amount).ok_or(Error::Overflow)?;
            self.total_st_shares = self.total_st_shares.checked_add(st_minted).ok_or(Error::Overflow)?;
            let prev = self.balances.get(caller).unwrap_or(0);
            self.balances.insert(caller, &(prev.checked_add(st_minted).ok_or(Error::Overflow)?));
            self.env().emit_event(Staked { user: caller, amount, st_minted });
            Ok(st_minted)
        }

        #[ink(message)]
        pub fn unstake(&mut self, st_amount: Balance) -> Result<u64, Error> { self.guard()?; let r = self.unstake_inner(st_amount); self.unguard(); r }
        fn unstake_inner(&mut self, st_amount: Balance) -> Result<u64, Error> {
            if st_amount == 0 { return Err(Error::ZeroAmount); }
            let caller = self.env().caller();
            let bal = self.balances.get(caller).unwrap_or(0);
            if bal < st_amount { return Err(Error::InsufficientShares); }
            self.balances.insert(caller, &(bal - st_amount));
            let id = self.next_withdrawal_id.get(caller).unwrap_or(0);
            self.next_withdrawal_id.insert(caller, &(id.saturating_add(1)));
            let unlock = self.env().block_number().saturating_add(self.unbonding_blocks);
            self.withdrawals.insert((caller, id), &WithdrawalRequest { st_amount, unlock_block: unlock, claimed: false });
            self.env().emit_event(Unstaked { user: caller, st_burned: st_amount, unlock_block: unlock, req_id: id });
            Ok(id)
        }

        #[ink(message)]
        pub fn withdraw(&mut self, req_id: u64) -> Result<Balance, Error> { self.guard()?; let r = self.withdraw_inner(req_id); self.unguard(); r }
        fn withdraw_inner(&mut self, req_id: u64) -> Result<Balance, Error> {
            let caller = self.env().caller();
            let mut req = self.withdrawals.get((caller, req_id)).ok_or(Error::NothingToWithdraw)?;
            if req.claimed { return Err(Error::NothingToWithdraw); }
            if self.env().block_number() < req.unlock_block { return Err(Error::TooEarly); }
            if self.total_st_shares == 0 { return Err(Error::Underflow); }
            let amount = req.st_amount.checked_mul(self.total_staked).ok_or(Error::Overflow)?.checked_div(self.total_st_shares).ok_or(Error::Overflow)?;
            req.claimed = true;
            self.withdrawals.insert((caller, req_id), &req);
            self.total_st_shares = self.total_st_shares.checked_sub(req.st_amount).ok_or(Error::Underflow)?;
            self.total_staked = self.total_staked.checked_sub(amount).ok_or(Error::Underflow)?;
            self.env().transfer(caller, amount).map_err(|_| Error::TransferFailed)?;
            self.env().emit_event(Withdrawn { user: caller, amount, req_id });
            Ok(amount)
        }

        #[ink(message)]
        pub fn update_exchange_rate(&mut self) -> Result<(), Error> {
            self.only_admin()?;
            self.env().emit_event(ExchangeRateUpdated { by: self.env().caller(), total_staked: self.total_staked, total_shares: self.total_st_shares });
            Ok(())
        }

        #[ink(message, payable)]
        pub fn add_rewards(&mut self) -> Result<(), Error> {
            self.only_admin()?;
            let amount = self.env().transferred_value();
            if amount == 0 { return Err(Error::ZeroAmount); }
            self.total_staked = self.total_staked.checked_add(amount).ok_or(Error::Overflow)?;
            self.env().emit_event(ExchangeRateUpdated { by: self.env().caller(), total_staked: self.total_staked, total_shares: self.total_st_shares });
            Ok(())
        }

        #[ink(message)]
        pub fn slash(&mut self, amount: Balance) -> Result<(), Error> {
            self.only_admin()?;
            if amount == 0 { return Err(Error::ZeroAmount); }
            self.total_staked = self.total_staked.saturating_sub(amount);
            self.env().emit_event(ExchangeRateUpdated { by: self.env().caller(), total_staked: self.total_staked, total_shares: self.total_st_shares });
            Ok(())
        }

        #[ink(message)] pub fn balance_of(&self, a: AccountId) -> Balance { self.balances.get(a).unwrap_or(0) }
        #[ink(message)] pub fn total_staked(&self) -> Balance { self.total_staked }
        #[ink(message)] pub fn total_shares(&self) -> Balance { self.total_st_shares }
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        fn accounts() -> ink::env::test::DefaultAccounts<ink::env::DefaultEnvironment> { ink::env::test::default_accounts::<ink::env::DefaultEnvironment>() }
        fn set_caller(c: AccountId) { ink::env::test::set_caller::<ink::env::DefaultEnvironment>(c); }
        fn set_value(v: Balance) { ink::env::test::set_value_transferred::<ink::env::DefaultEnvironment>(v); }
        fn set_block(b: BlockNumber) { ink::env::test::set_block_number::<ink::env::DefaultEnvironment>(b); }

        #[ink::test]
        fn stake_unstake_withdraw_flow() {
            let a = accounts(); set_caller(a.bob);
            let mut lsd = LiquidStakingDerivative::new(10);
            set_value(1000); let st = lsd.stake().unwrap(); assert_eq!(st, 1000);
            let req = lsd.unstake(500).unwrap(); set_block(15); assert_eq!(lsd.withdraw(req).unwrap(), 500);
        }

        #[ink::test]
        fn rewards_raise_rate() {
            let a = accounts(); let mut lsd = LiquidStakingDerivative::new(1); set_caller(a.bob);
            set_value(1000); lsd.stake().unwrap();
            set_caller(a.alice); set_value(500); lsd.add_rewards().unwrap();
            assert_eq!(lsd.total_staked(), 1500);
        }
    }
}
