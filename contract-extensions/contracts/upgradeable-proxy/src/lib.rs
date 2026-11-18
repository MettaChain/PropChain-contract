#![cfg_attr(not(feature = "std"), no_std, no_main)]

//! # Upgradeable proxy (part of #1445)
//!
//! A minimal delegating proxy. All state lives in **this** contract; any message
//! it does not define itself is forwarded with `delegate_call` to the code hash
//! stored as the current implementation, so the implementation's logic runs
//! against the proxy's storage and the proxy keeps one stable address.
//!
//! * The deployer (the factory) is the only account that may change the
//!   implementation, via [`UpgradeableProxy::upgrade_to`].
//! * The optional initializer is executed through `delegate_call` in the
//!   constructor, once.
//!
//! The forwarding and initializer paths are exercised only on a real node (the
//! off-chain test engine cannot run `delegate_call`); the unit tests cover
//! storage, authorisation and the getter surface.
//!
//! **Storage layout.** Because the implementation reads and writes the proxy's
//! storage, an upgrade is only safe if the new implementation's layout extends
//! the old one (append-only). The factory enforces this with a layout id and
//! revision; this contract trusts its factory.

#[ink::contract]
pub mod upgradeable_proxy {
    use ink::env::call::{build_call, ExecutionInput, Selector};
    use ink::prelude::vec::Vec;

    #[derive(Debug, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub enum Error {
        /// Only the factory that deployed this proxy may upgrade it.
        Unauthorized,
        /// The implementation hash is the one already in use.
        SameImplementation,
    }

    /// Encodes as its raw bytes, with no length prefix, so pre-encoded initializer
    /// arguments can be passed through unchanged.
    struct RawArgs(Vec<u8>);

    impl scale::Encode for RawArgs {
        fn encode_to<T: scale::Output + ?Sized>(&self, dest: &mut T) {
            dest.write(&self.0);
        }
    }

    #[ink(event)]
    pub struct ImplementationChanged {
        #[ink(topic)]
        pub old_implementation: Hash,
        #[ink(topic)]
        pub new_implementation: Hash,
    }

    #[ink(storage)]
    pub struct UpgradeableProxy {
        factory: AccountId,
        owner: AccountId,
        implementation: Hash,
    }

    impl UpgradeableProxy {
        /// `init_selector` + `init_data` (pre-encoded arguments) are delegate-called
        /// on `implementation` when `init_selector` is `Some`.
        #[ink(constructor)]
        pub fn new(
            implementation: Hash,
            owner: AccountId,
            init_selector: Option<[u8; 4]>,
            init_data: Vec<u8>,
        ) -> Self {
            if let Some(selector) = init_selector {
                build_call::<ink::env::DefaultEnvironment>()
                    .delegate(implementation)
                    .exec_input(ExecutionInput::new(Selector::new(selector)).push_arg(RawArgs(init_data)))
                    .returns::<()>()
                    .invoke();
            }
            Self { factory: Self::env().caller(), owner, implementation }
        }

        #[ink(message)]
        pub fn implementation(&self) -> Hash {
            self.implementation
        }

        #[ink(message)]
        pub fn factory(&self) -> AccountId {
            self.factory
        }

        #[ink(message)]
        pub fn owner(&self) -> AccountId {
            self.owner
        }

        /// Point the proxy at a new implementation. Factory only.
        #[ink(message)]
        pub fn upgrade_to(&mut self, new_implementation: Hash) -> Result<(), Error> {
            if self.env().caller() != self.factory {
                return Err(Error::Unauthorized);
            }
            if new_implementation == self.implementation {
                return Err(Error::SameImplementation);
            }
            let old = self.implementation;
            self.implementation = new_implementation;
            self.env().emit_event(ImplementationChanged {
                old_implementation: old,
                new_implementation,
            });
            Ok(())
        }

        /// Forward every other message to the implementation, with this contract's storage.
        #[ink(message, payable, selector = _)]
        pub fn forward(&self) -> u32 {
            build_call::<ink::env::DefaultEnvironment>()
                .delegate(self.implementation)
                .call_flags(
                    ink::env::CallFlags::default()
                        .set_forward_input(true)
                        .set_tail_call(true),
                )
                .try_invoke()
                .unwrap_or_else(|err| panic!("delegate call to {:?} failed due to {:?}", self.implementation, err));
            unreachable!("the forwarded call never returns because `tail_call` is set");
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use ink::env::test;

        fn hash(byte: u8) -> Hash {
            Hash::from([byte; 32])
        }

        fn deploy() -> (UpgradeableProxy, test::DefaultAccounts<ink::env::DefaultEnvironment>) {
            let accounts = test::default_accounts::<ink::env::DefaultEnvironment>();
            // The deployer (alice) plays the factory.
            test::set_caller::<ink::env::DefaultEnvironment>(accounts.alice);
            (UpgradeableProxy::new(hash(1), accounts.bob, None, Vec::new()), accounts)
        }

        #[ink::test]
        fn the_constructor_records_the_factory_owner_and_implementation() {
            let (p, accounts) = deploy();
            assert_eq!(p.factory(), accounts.alice);
            assert_eq!(p.owner(), accounts.bob);
            assert_eq!(p.implementation(), hash(1));
        }

        #[ink::test]
        fn only_the_factory_can_upgrade() {
            let (mut p, accounts) = deploy();
            test::set_caller::<ink::env::DefaultEnvironment>(accounts.bob);
            assert_eq!(p.upgrade_to(hash(2)), Err(Error::Unauthorized));
            assert_eq!(p.implementation(), hash(1));

            test::set_caller::<ink::env::DefaultEnvironment>(accounts.alice);
            p.upgrade_to(hash(2)).unwrap();
            assert_eq!(p.implementation(), hash(2));
        }

        #[ink::test]
        fn upgrading_to_the_current_implementation_is_rejected() {
            let (mut p, _) = deploy();
            assert_eq!(p.upgrade_to(hash(1)), Err(Error::SameImplementation));
            p.upgrade_to(hash(2)).unwrap();
            assert_eq!(p.upgrade_to(hash(2)), Err(Error::SameImplementation));
            // Going back to an earlier implementation is allowed.
            p.upgrade_to(hash(1)).unwrap();
            assert_eq!(p.implementation(), hash(1));
        }

        #[ink::test]
        fn an_upgrade_emits_one_event() {
            let (mut p, _) = deploy();
            p.upgrade_to(hash(2)).unwrap();
            assert_eq!(test::recorded_events().count(), 1);
        }

        #[test]
        fn raw_args_encode_without_a_length_prefix() {
            use scale::Encode;
            assert_eq!(RawArgs(vec![1, 2, 3]).encode(), vec![1, 2, 3]);
            assert!(RawArgs(Vec::new()).encode().is_empty());
        }
    }
}
