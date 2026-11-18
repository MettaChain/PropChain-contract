#![cfg_attr(not(feature = "std"), no_std, no_main)]

#[ink::contract]
mod dynamic_reward_rate {
    #[derive(Debug, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub enum Error {
        Unauthorized, CurveNotSet, Overflow,
    }

    #[ink(storage)]
    pub struct DynamicRewardRate {
        admin: AccountId,
        min_rate: u128,
        max_rate: u128,
        target_stake: u128,
        curve_set: bool,
        total_stake: u128,
        current_rate: u128,
        fixed_rate: u128,
    }

    #[ink(event)]
    pub struct RewardRateAdjusted { #[ink(topic)] by: AccountId, old_rate: u128, new_rate: u128 }

    impl DynamicRewardRate {
        #[ink(constructor)]
        pub fn new(fixed_rate: u128) -> Self {
            Self { admin: Self::env().caller(), min_rate: 0, max_rate: 0, target_stake: 0, curve_set: false, total_stake: 0, current_rate: fixed_rate, fixed_rate }
        }

        #[ink(message)]
        pub fn set_reward_curve(&mut self, min_rate: u128, max_rate: u128, target_stake: u128) -> Result<(), Error> {
            if self.env().caller() != self.admin { return Err(Error::Unauthorized); }
            if target_stake == 0 || min_rate > max_rate { return Err(Error::CurveNotSet); }
            self.min_rate = min_rate;
            self.max_rate = max_rate;
            self.target_stake = target_stake;
            self.curve_set = true;
            self.recalculate_and_emit();
            Ok(())
        }

        fn compute_rate(&self) -> u128 {
            if !self.curve_set { return self.fixed_rate; }
            if self.total_stake >= self.target_stake { return self.min_rate; }
            // max_rate - (max_rate - min_rate) * (total_stake / target_stake)
            let range = self.max_rate.saturating_sub(self.min_rate);
            let scaled = range.saturating_mul(self.total_stake) / self.target_stake.max(1);
            self.max_rate.saturating_sub(scaled).max(self.min_rate)
        }

        fn recalculate_and_emit(&mut self) {
            let new_rate = self.compute_rate();
            if new_rate != self.current_rate {
                let old = self.current_rate;
                self.current_rate = new_rate;
                self.env().emit_event(RewardRateAdjusted { by: self.env().caller(), old_rate: old, new_rate });
            }
        }

        #[ink(message)]
        pub fn get_current_reward_rate(&self) -> u128 { self.current_rate }

        #[ink(message)]
        pub fn notify_stake_changed(&mut self, new_total_stake: u128) -> Result<u128, Error> {
            if self.env().caller() != self.admin { return Err(Error::Unauthorized); }
            self.total_stake = new_total_stake;
            self.recalculate_and_emit();
            Ok(self.current_rate)
        }

        #[ink(message)]
        pub fn set_fixed_rate(&mut self, rate: u128) -> Result<(), Error> {
            if self.env().caller() != self.admin { return Err(Error::Unauthorized); }
            self.fixed_rate = rate;
            if !self.curve_set { self.current_rate = rate; }
            Ok(())
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        fn accounts() -> ink::env::test::DefaultAccounts<ink::env::DefaultEnvironment> { ink::env::test::default_accounts::<ink::env::DefaultEnvironment>() }
        fn set_caller(c: AccountId) { ink::env::test::set_caller::<ink::env::DefaultEnvironment>(c); }

        #[ink::test]
        fn fallback_fixed_rate() {
            let a = accounts(); set_caller(a.alice);
            let r = DynamicRewardRate::new(500);
            assert_eq!(r.get_current_reward_rate(), 500);
        }

        #[ink::test]
        fn dynamic_curve_low_stake_high_rate() {
            let a = accounts(); set_caller(a.alice);
            let mut r = DynamicRewardRate::new(100);
            r.set_reward_curve(100, 1000, 10_000).unwrap();
            r.notify_stake_changed(0).unwrap();
            assert_eq!(r.get_current_reward_rate(), 1000);
            r.notify_stake_changed(10_000).unwrap();
            assert_eq!(r.get_current_reward_rate(), 100);
        }

        #[ink::test]
        fn dynamic_curve_midpoint() {
            let a = accounts(); set_caller(a.alice);
            let mut r = DynamicRewardRate::new(100);
            r.set_reward_curve(100, 1000, 10_000).unwrap();
            r.notify_stake_changed(5_000).unwrap();
            assert_eq!(r.get_current_reward_rate(), 550);
        }
    }
}
