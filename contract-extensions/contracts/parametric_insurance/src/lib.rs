#![cfg_attr(not(feature = "std"), no_std, no_main)]

#[ink::contract]
mod parametric_insurance {
    use ink::prelude::string::String;
    use ink::storage::Mapping;

    #[derive(Debug, Clone, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout))]
    pub struct ParametricParameter {
        pub parameter_key: String,
        pub threshold: i128,
    }

    #[derive(Debug, Clone, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout))]
    pub struct ParametricPolicy {
        pub trigger_oracle: AccountId,
        pub parameters: Vec<ParametricParameter>,
        pub payout_amount: Balance,
        pub start_block: BlockNumber,
        pub end_block: BlockNumber,
        pub payout_executed: bool,
        pub holder: AccountId,
    }

    #[derive(Debug, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub enum Error {
        Unauthorized, NotFound, AlreadyPaid, DurationExpired, ReentrantCall, TransferFailed, Overflow, InvalidPolicy,
    }

    #[ink(storage)]
    pub struct ParametricInsurance {
        admin: AccountId,
        next_policy_id: u64,
        policies: Mapping<u64, ParametricPolicy>,
        entered: bool,
    }

    #[ink(event)] pub struct ParametricPolicyCreated { #[ink(topic)] policy_id: u64, #[ink(topic)] holder: AccountId, payout_amount: Balance, duration: BlockNumber }
    #[ink(event)] pub struct ParametricTriggerMet { #[ink(topic)] policy_id: u64, parameter_key: String, observed_value: i128 }
    #[ink(event)] pub struct ParametricPayoutExecuted { #[ink(topic)] policy_id: u64, amount: Balance }

    impl ParametricInsurance {
        #[ink(constructor)]
        pub fn new() -> Self {
            Self { admin: Self::env().caller(), next_policy_id: 1, policies: Mapping::default(), entered: false }
        }

        fn guard(&mut self) -> Result<(), Error> { if self.entered { return Err(Error::ReentrantCall); } self.entered = true; Ok(()) }
        fn unguard(&mut self) { self.entered = false; }

        #[ink(message, payable)]
        pub fn create_parametric_policy(&mut self, trigger_oracle: AccountId, parameters: Vec<ParametricParameter>, payout_amount: Balance, duration: BlockNumber) -> Result<u64, Error> {
            if parameters.is_empty() { return Err(Error::InvalidPolicy); }
            let holder = self.env().caller();
            let now = self.env().block_number();
            let id = self.next_policy_id;
            self.next_policy_id = id.saturating_add(1);
            self.policies.insert(id, &ParametricPolicy {
                trigger_oracle, parameters, payout_amount, start_block: now, end_block: now.saturating_add(duration), payout_executed: false, holder,
            });
            self.env().emit_event(ParametricPolicyCreated { policy_id: id, holder, payout_amount, duration });
            Ok(id)
        }

        /// Reads the parameter value from the oracle via cross-contract call.
        fn read_oracle_parameter(&self, oracle: AccountId, key: &str) -> Option<i128> {
            use ink::env::call::{build_call, ExecutionInput, Selector};
            use ink::env::DefaultEnvironment;
            build_call::<DefaultEnvironment>()
                .call(oracle)
                .exec_input(ExecutionInput::new(Selector::new(ink::selector_bytes!("get_parameter"))).push_arg(key.to_string()))
                .returns::<i128>()
                .invoke()
                .ok()
        }

        #[ink(message)]
        pub fn check_and_payout(&mut self, policy_id: u64) -> Result<bool, Error> {
            self.guard()?;
            let res = self.check_payout_inner(policy_id);
            self.unguard();
            res
        }

        fn check_payout_inner(&mut self, policy_id: u64) -> Result<bool, Error> {
            let mut policy = self.policies.get(policy_id).ok_or(Error::NotFound)?;
            if policy.payout_executed { return Ok(true); }
            let now = self.env().block_number();
            if now < policy.start_block || now > policy.end_block { return Err(Error::DurationExpired); }
            let mut met = false;
            for param in policy.parameters.iter() {
                if let Some(value) = self.read_oracle_parameter(policy.trigger_oracle, &param.parameter_key) {
                    if value >= param.threshold {
                        met = true;
                        self.env().emit_event(ParametricTriggerMet { policy_id, parameter_key: param.parameter_key.clone(), observed_value: value });
                        break;
                    }
                }
            }
            if !met { return Ok(false); }
            policy.payout_executed = true;
            self.policies.insert(policy_id, &policy);
            self.env().transfer(policy.holder, policy.payout_amount.min(self.env().balance())).map_err(|_| Error::TransferFailed)?;
            self.env().emit_event(ParametricPayoutExecuted { policy_id, amount: policy.payout_amount });
            Ok(true)
        }

        #[ink(message)] pub fn get_policy(&self, policy_id: u64) -> Option<ParametricPolicy> { self.policies.get(policy_id) }
        #[ink(message)] pub fn is_active(&self, policy_id: u64) -> bool {
            match self.policies.get(policy_id) {
                Some(p) => {
                    let now = self.env().block_number();
                    now >= p.start_block && now <= p.end_block && !p.payout_executed
                }
                None => false,
            }
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
        fn create_and_expire() {
            let a = accounts(); set_caller(a.alice); set_block(1);
            let mut p = ParametricInsurance::new();
            set_value(1000);
            let id = p.create_parametric_policy(a.django, vec![ParametricParameter { parameter_key: "quake".into(), threshold: 5 }], 1000, 100).unwrap();
            assert!(p.is_active(id));
            set_block(200);
            assert!(!p.is_active(id));
        }
    }
}
