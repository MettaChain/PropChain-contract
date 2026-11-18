#![cfg_attr(not(feature = "std"), no_std, no_main)]
#![allow(clippy::new_without_default)]

//! # Upgradeable factory (#1445)
//!
//! Deploys [`upgradeable-proxy`](../../upgradeable-proxy) instances pointing at
//! registered implementation code hashes, and controls who may upgrade them.
//!
//! ## Flow
//!
//! 1. The admin registers an implementation with
//!    [`UpgradeableFactory::register_implementation`], declaring its storage
//!    **layout id** (a hash naming the storage family) and **layout revision**.
//! 2. Anyone calls [`UpgradeableFactory::deploy_upgradeable`]; the factory
//!    instantiates a proxy (which delegate-calls the optional initializer) and
//!    records it.
//! 3. The admin or the designated *upgrade manager* calls
//!    [`UpgradeableFactory::upgrade`] to move a proxy to another registered
//!    implementation.
//!
//! ## Storage-layout compatibility
//!
//! The proxy's storage is read and written by the implementation, so a new
//! implementation must extend the old layout. The factory models that as:
//! same `layout_id` and `layout_revision >= current revision` (append-only
//! evolution). A different family, or a lower revision, is rejected with
//! `IncompatibleLayout`. The factory trusts the layout id/revision the admin
//! registered; it cannot inspect the compiled code.
//!
//! ## Testing note
//!
//! Instantiating the proxy and calling `upgrade_to` on it are cross-contract
//! operations the off-chain test engine cannot run. All authorisation, registry
//! and compatibility logic is in `authorize_upgrade` / `record_proxy`, which the
//! unit tests cover; the thin on-chain wrappers need an e2e environment.

#[ink::contract]
pub mod upgradeable_factory {
    use ink::env::call::{build_call, ExecutionInput, Selector};
    use ink::prelude::vec::Vec;
    use ink::storage::Mapping;
    use upgradeable_proxy::upgradeable_proxy::{Error as ProxyError, UpgradeableProxyRef};

    #[derive(Debug, Clone, Copy, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout))]
    pub struct ImplementationInfo {
        pub layout_id: Hash,
        pub layout_revision: u32,
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout))]
    pub struct ProxyRecord {
        pub implementation: Hash,
        pub owner: AccountId,
        pub deployed_at: u32,
        pub upgrades: u32,
    }

    #[derive(Debug, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub enum Error {
        Unauthorized,
        ImplementationNotRegistered,
        ImplementationAlreadyRegistered,
        ProxyNotFound,
        SameImplementation,
        /// The new implementation's layout family differs or its revision is lower.
        IncompatibleLayout,
        /// Instantiating the proxy failed.
        DeploymentFailed,
        /// The call to the proxy's `upgrade_to` failed.
        ProxyUpgradeFailed,
    }

    #[ink(event)]
    pub struct UpgradeableDeployed {
        #[ink(topic)]
        pub proxy: AccountId,
        #[ink(topic)]
        pub implementation: Hash,
        pub owner: AccountId,
    }

    #[ink(event)]
    pub struct ImplementationUpgraded {
        #[ink(topic)]
        pub proxy: AccountId,
        pub old_implementation: Hash,
        pub new_implementation: Hash,
    }

    #[ink(event)]
    pub struct UpgradeAuthorizationChanged {
        pub old_manager: Option<AccountId>,
        pub new_manager: Option<AccountId>,
    }

    #[ink(event)]
    pub struct ImplementationRegistered {
        #[ink(topic)]
        pub code_hash: Hash,
        pub layout_revision: u32,
    }

    #[ink(storage)]
    pub struct UpgradeableFactory {
        admin: AccountId,
        upgrade_manager: Option<AccountId>,
        proxy_code_hash: Hash,
        implementations: Mapping<Hash, ImplementationInfo>,
        proxies: Mapping<AccountId, ProxyRecord>,
        proxy_list: Vec<AccountId>,
    }

    impl UpgradeableFactory {
        /// `proxy_code_hash` is the uploaded code hash of the `upgradeable-proxy` contract.
        #[ink(constructor)]
        pub fn new(proxy_code_hash: Hash) -> Self {
            Self {
                admin: Self::env().caller(),
                upgrade_manager: None,
                proxy_code_hash,
                implementations: Mapping::default(),
                proxies: Mapping::default(),
                proxy_list: Vec::new(),
            }
        }

        fn ensure_admin(&self) -> Result<(), Error> {
            if self.env().caller() != self.admin {
                return Err(Error::Unauthorized);
            }
            Ok(())
        }

        // ── Administration ─────────────────────────────────────────────

        /// Make an implementation deployable and a valid upgrade target. Admin only.
        #[ink(message)]
        pub fn register_implementation(
            &mut self,
            code_hash: Hash,
            layout_id: Hash,
            layout_revision: u32,
        ) -> Result<(), Error> {
            self.ensure_admin()?;
            if self.implementations.contains(code_hash) {
                return Err(Error::ImplementationAlreadyRegistered);
            }
            self.implementations
                .insert(code_hash, &ImplementationInfo { layout_id, layout_revision });
            self.env().emit_event(ImplementationRegistered { code_hash, layout_revision });
            Ok(())
        }

        /// Designate (or clear) the account that may upgrade proxies besides the admin. Admin only.
        #[ink(message)]
        pub fn set_upgrade_manager(&mut self, manager: Option<AccountId>) -> Result<(), Error> {
            self.ensure_admin()?;
            let old = self.upgrade_manager;
            self.upgrade_manager = manager;
            self.env().emit_event(UpgradeAuthorizationChanged { old_manager: old, new_manager: manager });
            Ok(())
        }

        // ── Queries ────────────────────────────────────────────────────

        #[ink(message)]
        pub fn get_implementation(&self, proxy: AccountId) -> Option<Hash> {
            self.proxies.get(proxy).map(|p| p.implementation)
        }

        #[ink(message)]
        pub fn get_proxy_info(&self, proxy: AccountId) -> Option<ProxyRecord> {
            self.proxies.get(proxy)
        }

        #[ink(message)]
        pub fn get_implementation_info(&self, code_hash: Hash) -> Option<ImplementationInfo> {
            self.implementations.get(code_hash)
        }

        #[ink(message)]
        pub fn get_proxies(&self) -> Vec<AccountId> {
            self.proxy_list.clone()
        }

        #[ink(message)]
        pub fn get_upgrade_manager(&self) -> Option<AccountId> {
            self.upgrade_manager
        }

        #[ink(message)]
        pub fn is_upgrade_authorized(&self, account: AccountId) -> bool {
            account == self.admin || self.upgrade_manager == Some(account)
        }

        // ── Deploy and upgrade ─────────────────────────────────────────

        /// Deploy a proxy for a registered implementation. `init_selector` /
        /// `init_data` (pre-encoded arguments) are run through the proxy's
        /// constructor with `delegate_call`. The caller becomes the proxy owner.
        #[ink(message)]
        pub fn deploy_upgradeable(
            &mut self,
            implementation_code_hash: Hash,
            init_selector: Option<[u8; 4]>,
            init_data: Vec<u8>,
            salt: [u8; 4],
        ) -> Result<AccountId, Error> {
            if !self.implementations.contains(implementation_code_hash) {
                return Err(Error::ImplementationNotRegistered);
            }
            let owner = self.env().caller();
            let proxy = UpgradeableProxyRef::new(implementation_code_hash, owner, init_selector, init_data)
                .code_hash(self.proxy_code_hash)
                .endowment(0)
                .salt_bytes(salt)
                .try_instantiate()
                .map_err(|_| Error::DeploymentFailed)?
                .map_err(|_| Error::DeploymentFailed)?;
            let proxy_account = ink::ToAccountId::to_account_id(&proxy);
            self.record_proxy(proxy_account, implementation_code_hash, owner);
            Ok(proxy_account)
        }

        /// Move `proxy` to `new_implementation`. Admin or upgrade manager only.
        #[ink(message)]
        pub fn upgrade(&mut self, proxy: AccountId, new_implementation: Hash) -> Result<(), Error> {
            self.authorize_upgrade(proxy, new_implementation)?;
            build_call::<ink::env::DefaultEnvironment>()
                .call(proxy)
                .exec_input(
                    ExecutionInput::new(Selector::new(ink::selector_bytes!("upgrade_to")))
                        .push_arg(new_implementation),
                )
                .returns::<Result<(), ProxyError>>()
                .try_invoke()
                .map_err(|_| Error::ProxyUpgradeFailed)?
                .map_err(|_| Error::ProxyUpgradeFailed)?
                .map_err(|_| Error::ProxyUpgradeFailed)?;
            Ok(())
        }

        // ── Logic kept free of cross-contract calls (unit-tested) ──────

        fn record_proxy(&mut self, proxy: AccountId, implementation: Hash, owner: AccountId) {
            self.proxies.insert(
                proxy,
                &ProxyRecord {
                    implementation,
                    owner,
                    deployed_at: self.env().block_number(),
                    upgrades: 0,
                },
            );
            self.proxy_list.push(proxy);
            self.env().emit_event(UpgradeableDeployed { proxy, implementation, owner });
        }

        /// Check authorisation and layout compatibility, then update the registry.
        fn authorize_upgrade(&mut self, proxy: AccountId, new_implementation: Hash) -> Result<(), Error> {
            let caller = self.env().caller();
            if !self.is_upgrade_authorized(caller) {
                return Err(Error::Unauthorized);
            }
            let mut record = self.proxies.get(proxy).ok_or(Error::ProxyNotFound)?;
            let new_info = self
                .implementations
                .get(new_implementation)
                .ok_or(Error::ImplementationNotRegistered)?;
            if new_implementation == record.implementation {
                return Err(Error::SameImplementation);
            }
            // The current implementation was registered at deployment, so it is known.
            let current = self
                .implementations
                .get(record.implementation)
                .ok_or(Error::ImplementationNotRegistered)?;
            if new_info.layout_id != current.layout_id || new_info.layout_revision < current.layout_revision {
                return Err(Error::IncompatibleLayout);
            }
            let old = record.implementation;
            record.implementation = new_implementation;
            record.upgrades = record.upgrades.saturating_add(1);
            self.proxies.insert(proxy, &record);
            self.env().emit_event(ImplementationUpgraded {
                proxy,
                old_implementation: old,
                new_implementation,
            });
            Ok(())
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use ink::env::test;

        type Accounts = test::DefaultAccounts<ink::env::DefaultEnvironment>;

        fn hash(byte: u8) -> Hash {
            Hash::from([byte; 32])
        }

        const FAMILY_A: u8 = 0xA0;
        const FAMILY_B: u8 = 0xB0;

        /// Admin = alice. Implementations: v1/v2/v3 in family A (revisions 1/2/3), x1 in family B.
        fn setup() -> (UpgradeableFactory, Accounts) {
            let accounts = test::default_accounts::<ink::env::DefaultEnvironment>();
            test::set_caller::<ink::env::DefaultEnvironment>(accounts.alice);
            let mut f = UpgradeableFactory::new(hash(0xEE));
            f.register_implementation(hash(1), hash(FAMILY_A), 1).unwrap();
            f.register_implementation(hash(2), hash(FAMILY_A), 2).unwrap();
            f.register_implementation(hash(3), hash(FAMILY_A), 3).unwrap();
            f.register_implementation(hash(9), hash(FAMILY_B), 1).unwrap();
            (f, accounts)
        }

        /// Record a proxy at `proxy` running implementation `hash(1)`, owned by bob.
        fn with_proxy() -> (UpgradeableFactory, Accounts, AccountId) {
            let (mut f, accounts) = setup();
            let proxy = accounts.django;
            f.record_proxy(proxy, hash(1), accounts.bob);
            (f, accounts, proxy)
        }

        // ── Implementations ─────────────────────────────────────────────

        #[ink::test]
        fn implementations_are_registered_by_the_admin_once() {
            let (mut f, accounts) = setup();
            assert_eq!(
                f.get_implementation_info(hash(2)),
                Some(ImplementationInfo { layout_id: hash(FAMILY_A), layout_revision: 2 })
            );
            assert_eq!(
                f.register_implementation(hash(2), hash(FAMILY_A), 5),
                Err(Error::ImplementationAlreadyRegistered)
            );
            test::set_caller::<ink::env::DefaultEnvironment>(accounts.bob);
            assert_eq!(
                f.register_implementation(hash(7), hash(FAMILY_A), 1),
                Err(Error::Unauthorized)
            );
            assert!(f.get_implementation_info(hash(7)).is_none());
        }

        // ── Recording a deployment ──────────────────────────────────────

        #[ink::test]
        fn a_deployed_proxy_is_recorded_with_its_implementation_and_owner() {
            let (f, accounts, proxy) = with_proxy();
            assert_eq!(f.get_implementation(proxy), Some(hash(1)));
            let info = f.get_proxy_info(proxy).unwrap();
            assert_eq!(info.owner, accounts.bob);
            assert_eq!(info.upgrades, 0);
            assert_eq!(f.get_proxies(), vec![proxy]);
            assert_eq!(f.get_implementation(accounts.eve), None);
        }

        #[ink::test]
        fn deploying_an_unregistered_implementation_is_rejected_before_any_instantiation() {
            let (mut f, _) = setup();
            assert_eq!(
                f.deploy_upgradeable(hash(0x77), None, Vec::new(), [0; 4]),
                Err(Error::ImplementationNotRegistered)
            );
            assert!(f.get_proxies().is_empty());
        }

        // ── Upgrade authorisation ───────────────────────────────────────

        #[ink::test]
        fn the_admin_can_upgrade_within_a_layout_family() {
            let (mut f, _, proxy) = with_proxy();
            f.authorize_upgrade(proxy, hash(2)).unwrap();
            assert_eq!(f.get_implementation(proxy), Some(hash(2)));
            assert_eq!(f.get_proxy_info(proxy).unwrap().upgrades, 1);
            f.authorize_upgrade(proxy, hash(3)).unwrap();
            assert_eq!(f.get_proxy_info(proxy).unwrap().upgrades, 2);
        }

        #[ink::test]
        fn strangers_and_proxy_owners_cannot_upgrade() {
            let (mut f, accounts, proxy) = with_proxy();
            for who in [accounts.bob, accounts.eve] {
                test::set_caller::<ink::env::DefaultEnvironment>(who);
                assert_eq!(f.authorize_upgrade(proxy, hash(2)), Err(Error::Unauthorized));
            }
            assert_eq!(f.get_implementation(proxy), Some(hash(1)));
        }

        #[ink::test]
        fn the_upgrade_manager_can_upgrade_until_the_role_is_revoked() {
            let (mut f, accounts, proxy) = with_proxy();
            f.set_upgrade_manager(Some(accounts.charlie)).unwrap();
            assert_eq!(f.get_upgrade_manager(), Some(accounts.charlie));
            assert!(f.is_upgrade_authorized(accounts.charlie));

            test::set_caller::<ink::env::DefaultEnvironment>(accounts.charlie);
            f.authorize_upgrade(proxy, hash(2)).unwrap();

            test::set_caller::<ink::env::DefaultEnvironment>(accounts.alice);
            f.set_upgrade_manager(None).unwrap();
            assert!(!f.is_upgrade_authorized(accounts.charlie));
            test::set_caller::<ink::env::DefaultEnvironment>(accounts.charlie);
            assert_eq!(f.authorize_upgrade(proxy, hash(3)), Err(Error::Unauthorized));
        }

        #[ink::test]
        fn only_the_admin_can_designate_the_upgrade_manager() {
            let (mut f, accounts) = setup();
            test::set_caller::<ink::env::DefaultEnvironment>(accounts.bob);
            assert_eq!(f.set_upgrade_manager(Some(accounts.bob)), Err(Error::Unauthorized));
            assert_eq!(f.get_upgrade_manager(), None);
        }

        // ── Layout compatibility ────────────────────────────────────────

        #[ink::test]
        fn an_implementation_from_another_layout_family_is_rejected() {
            let (mut f, _, proxy) = with_proxy();
            assert_eq!(f.authorize_upgrade(proxy, hash(9)), Err(Error::IncompatibleLayout));
            assert_eq!(f.get_implementation(proxy), Some(hash(1)));
        }

        #[ink::test]
        fn a_lower_layout_revision_is_rejected_and_an_equal_one_is_allowed() {
            let (mut f, _, proxy) = with_proxy();
            f.authorize_upgrade(proxy, hash(3)).unwrap(); // revision 3
            assert_eq!(f.authorize_upgrade(proxy, hash(2)), Err(Error::IncompatibleLayout)); // revision 2
            // A same-revision sibling in the same family is compatible.
            let (mut g, _, p2) = with_proxy();
            g.register_implementation(hash(4), hash(FAMILY_A), 1).unwrap();
            assert!(g.authorize_upgrade(p2, hash(4)).is_ok());
        }

        #[ink::test]
        fn upgrade_target_and_proxy_must_exist_and_differ() {
            let (mut f, accounts, proxy) = with_proxy();
            assert_eq!(f.authorize_upgrade(proxy, hash(0x55)), Err(Error::ImplementationNotRegistered));
            assert_eq!(f.authorize_upgrade(accounts.eve, hash(2)), Err(Error::ProxyNotFound));
            assert_eq!(f.authorize_upgrade(proxy, hash(1)), Err(Error::SameImplementation));
        }

        // ── Events ──────────────────────────────────────────────────────

        #[ink::test]
        fn deployment_upgrade_and_authorisation_changes_emit_events() {
            let (mut f, accounts, proxy) = with_proxy();
            let base = test::recorded_events().count();
            f.authorize_upgrade(proxy, hash(2)).unwrap();
            assert_eq!(test::recorded_events().count() - base, 1);
            f.set_upgrade_manager(Some(accounts.charlie)).unwrap();
            assert_eq!(test::recorded_events().count() - base, 2);
        }
    }
}
