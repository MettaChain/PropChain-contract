#![cfg_attr(not(feature = "std"), no_std, no_main)]

#[ink::contract]
mod token_vesting {
    use ink::storage::Mapping;

    #[derive(Debug, Clone, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout))]
    pub struct Schedule {
        pub total_amount: Balance,
        pub claimed: Balance,
        pub start_block: BlockNumber,
        pub cliff_duration: BlockNumber,
        pub vesting_duration: BlockNumber,
        pub interval: BlockNumber,
        pub revoked: bool,
    }

    #[derive(Debug, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub enum Error {
        Unauthorized, InvalidSchedule, NothingToClaim, AlreadyExists, NotFound, TransferFailed, Overflow,
    }

    #[ink(storage)]
    pub struct TokenVesting {
        admin: AccountId,
        vesting_token: AccountId,
        schedules: Mapping<AccountId, Schedule>,
    }

    #[ink(event)]
    pub struct BeneficiaryAdded { #[ink(topic)] beneficiary: AccountId, total_amount: Balance, cliff_duration: BlockNumber, vesting_duration: BlockNumber, interval: BlockNumber }
    #[ink(event)]
    pub struct TokensClaimed { #[ink(topic)] beneficiary: AccountId, amount: Balance, total_claimed: Balance }
    #[ink(event)]
    pub struct VestingRevoked { #[ink(topic)] beneficiary: AccountId, unvested_returned: Balance }

    impl TokenVesting {
        #[ink(constructor)]
        pub fn new(vesting_token: AccountId) -> Self {
            Self { admin: Self::env().caller(), vesting_token, schedules: Mapping::default() }
        }
        fn only_admin(&self) -> Result<(), Error> {
            if self.env().caller() != self.admin { return Err(Error::Unauthorized); }
            Ok(())
        }

        #[ink(message)]
        pub fn add_beneficiary(&mut self, beneficiary: AccountId, total_amount: Balance, cliff_duration: BlockNumber, vesting_duration: BlockNumber, interval: BlockNumber) -> Result<(), Error> {
            self.only_admin()?;
            if total_amount == 0 || vesting_duration == 0 || interval == 0 { return Err(Error::InvalidSchedule); }
            if self.schedules.get(beneficiary).is_some() { return Err(Error::AlreadyExists); }
            let start_block = self.env().block_number();
            self.schedules.insert(beneficiary, &Schedule { total_amount, claimed: 0, start_block, cliff_duration, vesting_duration, interval, revoked: false });
            self.env().emit_event(BeneficiaryAdded { beneficiary, total_amount, cliff_duration, vesting_duration, interval });
            Ok(())
        }

        fn vested_for(s: &Schedule, now: BlockNumber) -> Balance {
            let cliff_end = s.start_block.saturating_add(s.cliff_duration);
            if now < cliff_end { return 0; }
            let end = s.start_block.saturating_add(s.vesting_duration);
            let effective = core::cmp::min(now, end);
            let elapsed = effective.saturating_sub(s.start_block);
            let stepped = elapsed.saturating_sub(elapsed % s.interval.max(1));
            if stepped >= s.vesting_duration { return s.total_amount; }
            s.total_amount.saturating_mul(stepped as u128).checked_div(s.vesting_duration.max(1) as u128).unwrap_or(0)
        }

        #[ink(message)]
        pub fn vested_amount(&self, beneficiary: AccountId) -> Balance {
            match self.schedules.get(beneficiary) {
                Some(s) => Self::vested_for(&s, self.env().block_number()),
                None => 0,
            }
        }

        #[ink(message)]
        pub fn claim(&mut self) -> Result<Balance, Error> {
            let caller = self.env().caller();
            let mut schedule = self.schedules.get(caller).ok_or(Error::NotFound)?;
            let vested = Self::vested_for(&schedule, self.env().block_number());
            let claimable = vested.saturating_sub(schedule.claimed);
            if claimable == 0 { return Err(Error::NothingToClaim); }
            schedule.claimed = schedule.claimed.saturating_add(claimable);
            let total_claimed = schedule.claimed;
            self.schedules.insert(caller, &schedule);
            self.env().transfer(caller, claimable).map_err(|_| Error::TransferFailed)?;
            self.env().emit_event(TokensClaimed { beneficiary: caller, amount: claimable, total_claimed });
            Ok(claimable)
        }

        #[ink(message)]
        pub fn revoke(&mut self, beneficiary: AccountId) -> Result<Balance, Error> {
            self.only_admin()?;
            let mut schedule = self.schedules.get(beneficiary).ok_or(Error::NotFound)?;
            if schedule.revoked { return Err(Error::InvalidSchedule); }
            let vested = Self::vested_for(&schedule, self.env().block_number());
            let unvested = schedule.total_amount.saturating_sub(vested);
            schedule.revoked = true;
            schedule.total_amount = vested;
            self.schedules.insert(beneficiary, &schedule);
            if unvested > 0 {
                self.env().transfer(self.admin, unvested).map_err(|_| Error::TransferFailed)?;
            }
            self.env().emit_event(VestingRevoked { beneficiary, unvested_returned: unvested });
            Ok(unvested)
        }

        #[ink(message)] pub fn get_schedule(&self, beneficiary: AccountId) -> Option<Schedule> { self.schedules.get(beneficiary) }
        #[ink(message)] pub fn vesting_token(&self) -> AccountId { self.vesting_token }
        #[ink(message)] pub fn admin(&self) -> AccountId { self.admin }
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        fn accounts() -> ink::env::test::DefaultAccounts<ink::env::DefaultEnvironment> { ink::env::test::default_accounts::<ink::env::DefaultEnvironment>() }
        fn set_caller(c: AccountId) { ink::env::test::set_caller::<ink::env::DefaultEnvironment>(c); }
        fn set_block(b: BlockNumber) { ink::env::test::set_block_number::<ink::env::DefaultEnvironment>(b); }

        #[ink::test]
        fn cliff_blocks_then_vests() {
            let a = accounts(); set_caller(a.alice); set_block(1);
            let mut v = TokenVesting::new(a.django);
            v.add_beneficiary(a.bob, 1000, 10, 100, 1).unwrap();
            set_block(5); assert_eq!(v.vested_amount(a.bob), 0);
            set_block(11); assert!(v.vested_amount(a.bob) > 0);
        }

        #[ink::test]
        fn revoke_caps_to_vested() {
            let a = accounts(); set_caller(a.alice); set_block(0);
            let mut v = TokenVesting::new(a.django);
            v.add_beneficiary(a.bob, 1000, 0, 100, 1).unwrap();
            set_block(40);
            let unvested = v.revoke(a.bob).unwrap();
            assert_eq!(unvested, 600);
        }
    }
}
