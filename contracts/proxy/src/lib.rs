#![cfg_attr(not(feature = "std"), no_std, no_main)]
#![allow(clippy::new_without_default, clippy::clone_on_copy)]

//! # PropChain Transparent Proxy
//!
//! A transparent proxy contract that routes calls to an implementation contract.
//! Upgrades follow a two-step governance pattern: stage a new implementation
//! address, then confirm after a configurable delay.
//!
//! ## Storage layout
//!
//! The proxy stores only proxy-specific state (implementation address, admin,
//! pending upgrade). The implementation contract runs in its own storage
//! context via cross-contract calls, avoiding storage collisions.
//!
//! ## Upgrade flow
//!
//! 1. Admin calls `set_implementation(new_addr)` to stage the upgrade.
//! 2. After `upgrade_delay_blocks` blocks elapse, admin calls `confirm_implementation()`.
//! 3. The implementation address is updated; all subsequent fallback calls
//!    route to the new implementation.
//!
//! ## Call forwarding
//!
//! The `call_implementation` message forwards a selector + encoded input to the
//! implementation via `call_v1` cross-contract call. The implementation's return
//! data is propagated back to the caller.
//!
//! **Note**: This uses cross-contract calls, so the implementation runs with its
//! own storage (not the proxy's). For true delegatecall semantics (shared
//! storage), a low-level ink! storage overlay would be required — this is out
//! of scope for the initial implementation.

#[ink::contract]
pub mod propchain_proxy {
    use ink::env::call::build_call;
    use ink::prelude::vec::Vec;

    /// Errors that the proxy itself can return.
    #[derive(Debug, Clone, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub enum ProxyError {
        /// Caller is not the admin.
        Unauthorized,
        /// No upgrade has been staged.
        NoPendingUpgrade,
        /// The delay period has not yet elapsed.
        DelayNotElapsed,
        /// The provided address is the zero address.
        InvalidImplementation,
        /// The implementation call reverted.
        ImplementationCallFailed,
    }

    impl From<ink::env::Error> for ProxyError {
        fn from(_: ink::env::Error) -> Self {
            ProxyError::ImplementationCallFailed
        }
    }

    /// Transparent proxy with two-step upgrade governance.
    #[ink(storage)]
    pub struct TransparentProxy {
        /// AccountId of the current implementation contract.
        implementation: AccountId,
        /// Staged new implementation, or zero address if none pending.
        pending_implementation: AccountId,
        /// Block number at which the pending upgrade becomes confirmable.
        upgrade_effective_at: u64,
        /// Account authorized to stage and confirm upgrades (DAO timelock).
        admin: AccountId,
        /// Number of blocks that must pass between staging and confirmation.
        upgrade_delay_blocks: u64,
    }

    /// Emitted when a new implementation is staged.
    #[ink(event)]
    pub struct UpgradeStaged {
        #[ink(topic)]
        new_implementation: AccountId,
        effective_at: u64,
    }

    /// Emitted when a staged upgrade is confirmed and takes effect.
    #[ink(event)]
    pub struct UpgradeConfirmed {
        #[ink(topic)]
        old_implementation: AccountId,
        #[ink(topic)]
        new_implementation: AccountId,
    }

    impl TransparentProxy {
        /// Deploy the proxy with an initial implementation.
        ///
        /// The deployer becomes the admin. The upgrade delay defaults to 100
        /// blocks (~10 minutes on a 6-second chain).
        #[ink(constructor)]
        pub fn new(implementation: AccountId, admin: AccountId) -> Self {
            Self {
                implementation,
                pending_implementation: AccountId::from([0u8; 32]),
                upgrade_effective_at: 0,
                admin,
                upgrade_delay_blocks: 100,
            }
        }

        // ── Admin-only messages ───────────────────────────────────────────

        /// Stage a new implementation address (admin only).
        ///
        /// The upgrade will not be confirmable until `upgrade_delay_blocks`
        /// blocks have passed. Calling this again before confirmation replaces
        /// the staged address and restarts the timer.
        #[ink(message)]
        pub fn set_implementation(
            &mut self,
            new_implementation: AccountId,
        ) -> Result<(), ProxyError> {
            self.ensure_admin()?;
            if new_implementation == AccountId::from([0u8; 32]) {
                return Err(ProxyError::InvalidImplementation);
            }
            let current_block = self.env().block_number() as u64;
            self.pending_implementation = new_implementation;
            self.upgrade_effective_at = current_block + self.upgrade_delay_blocks;

            self.env().emit_event(UpgradeStaged {
                new_implementation,
                effective_at: self.upgrade_effective_at,
            });
            Ok(())
        }

        /// Confirm a pending upgrade (admin only).
        ///
        /// Succeeds only if the delay period has elapsed since `set_implementation`.
        #[ink(message)]
        pub fn confirm_implementation(&mut self) -> Result<(), ProxyError> {
            self.ensure_admin()?;
            if self.pending_implementation == AccountId::from([0u8; 32]) {
                return Err(ProxyError::NoPendingUpgrade);
            }
            let current_block = self.env().block_number() as u64;
            if current_block < self.upgrade_effective_at {
                return Err(ProxyError::DelayNotElapsed);
            }

            let old = self.implementation;
            self.implementation = self.pending_implementation;
            self.pending_implementation = AccountId::from([0u8; 32]);
            self.upgrade_effective_at = 0;

            self.env().emit_event(UpgradeConfirmed {
                old_implementation: old,
                new_implementation: self.implementation,
            });
            Ok(())
        }

        /// Update the upgrade delay (admin only).
        ///
        /// Only affects future `set_implementation` calls; a pending upgrade's
        /// `effective_at` is not retroactively changed.
        #[ink(message)]
        pub fn set_upgrade_delay_blocks(&mut self, new_delay: u64) -> Result<(), ProxyError> {
            self.ensure_admin()?;
            self.upgrade_delay_blocks = new_delay;
            Ok(())
        }

        // ── Public read-only messages ─────────────────────────────────────

        /// Return the current implementation address.
        #[ink(message)]
        pub fn implementation(&self) -> AccountId {
            self.implementation
        }

        /// Return the pending implementation address, or the zero address if none.
        #[ink(message)]
        pub fn pending_implementation(&self) -> AccountId {
            self.pending_implementation
        }

        /// Return the block number at which the pending upgrade becomes
        /// confirmable. Zero if no upgrade is pending.
        #[ink(message)]
        pub fn upgrade_effective_at(&self) -> u64 {
            self.upgrade_effective_at
        }

        /// Return the admin address.
        #[ink(message)]
        pub fn admin(&self) -> AccountId {
            self.admin
        }

        /// Return the current upgrade delay in blocks.
        #[ink(message)]
        pub fn upgrade_delay_blocks(&self) -> u64 {
            self.upgrade_delay_blocks
        }

        // ── Fallback / forwarding ─────────────────────────────────────────

        /// Forward a call to the implementation contract.
        ///
        /// The 4-byte `selector` and SCALE-encoded `input` are forwarded
        /// via a cross-contract call. The implementation's return value
        /// replaces this call's return value.
        ///
        /// If the implementation call reverts, the error is propagated.
        #[ink(message)]
        pub fn call_implementation(
            &self,
            selector: [u8; 4],
            input: Vec<u8>,
        ) -> Result<Vec<u8>, ProxyError> {
            let call_result = build_call::<ink::env::DefaultEnvironment>()
                .call_v1(self.implementation)
                .exec_input(
                    ink::env::call::ExecutionInput::new(ink::env::call::Selector::new(selector))
                        .push_arg(&input),
                )
                .returns::<Vec<u8>>()
                .try_invoke();

            match call_result {
                Ok(Ok(bytes)) => Ok(bytes),
                Ok(Err(_lang_err)) => Err(ProxyError::ImplementationCallFailed),
                Err(_env_err) => Err(ProxyError::ImplementationCallFailed),
            }
        }

        // ── Internal ──────────────────────────────────────────────────────

        fn ensure_admin(&self) -> Result<(), ProxyError> {
            if self.env().caller() != self.admin {
                return Err(ProxyError::Unauthorized);
            }
            Ok(())
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        fn accounts() -> ink::env::test::DefaultAccounts<ink::env::DefaultEnvironment> {
            ink::env::test::default_accounts::<ink::env::DefaultEnvironment>()
        }

        #[ink::test]
        fn constructor_sets_fields() {
            let accounts = accounts();
            let proxy = TransparentProxy::new(accounts.bob, accounts.alice);
            assert_eq!(proxy.implementation(), accounts.bob);
            assert_eq!(proxy.admin(), accounts.alice);
            assert_eq!(proxy.upgrade_delay_blocks(), 100);
        }

        #[ink::test]
        fn set_implementation_stages_upgrade() {
            let accounts = accounts();
            let mut proxy = TransparentProxy::new(accounts.bob, accounts.alice);

            ink::env::test::set_caller::<ink::env::DefaultEnvironment>(accounts.alice);
            assert!(proxy.set_implementation(accounts.charlie).is_ok());
            assert_eq!(proxy.pending_implementation(), accounts.charlie);
            assert!(proxy.upgrade_effective_at() > 0);
        }

        #[ink::test]
        fn non_admin_cannot_stage() {
            let accounts = accounts();
            let mut proxy = TransparentProxy::new(accounts.bob, accounts.alice);

            ink::env::test::set_caller::<ink::env::DefaultEnvironment>(accounts.bob);
            assert_eq!(
                proxy.set_implementation(accounts.charlie),
                Err(ProxyError::Unauthorized)
            );
        }

        #[ink::test]
        fn confirm_requires_delay() {
            let accounts = accounts();
            let mut proxy = TransparentProxy::new(accounts.bob, accounts.alice);

            ink::env::test::set_caller::<ink::env::DefaultEnvironment>(accounts.alice);
            proxy.set_implementation(accounts.charlie).unwrap();

            assert_eq!(
                proxy.confirm_implementation(),
                Err(ProxyError::DelayNotElapsed)
            );
        }

        #[ink::test]
        fn confirm_succeeds_after_delay() {
            let accounts = accounts();
            let mut proxy = TransparentProxy::new(accounts.bob, accounts.alice);

            ink::env::test::set_caller::<ink::env::DefaultEnvironment>(accounts.alice);
            proxy.set_implementation(accounts.charlie).unwrap();

            let current = ink::env::block_number::<ink::env::DefaultEnvironment>();
            ink::env::test::set_block_number::<ink::env::DefaultEnvironment>(
                current + proxy.upgrade_delay_blocks() as u32,
            );

            assert!(proxy.confirm_implementation().is_ok());
            assert_eq!(proxy.implementation(), accounts.charlie);
            assert_eq!(proxy.pending_implementation(), AccountId::from([0u8; 32]));
        }

        #[ink::test]
        fn confirm_without_pending_fails() {
            let accounts = accounts();
            let mut proxy = TransparentProxy::new(accounts.bob, accounts.alice);

            ink::env::test::set_caller::<ink::env::DefaultEnvironment>(accounts.alice);
            assert_eq!(
                proxy.confirm_implementation(),
                Err(ProxyError::NoPendingUpgrade)
            );
        }

        #[ink::test]
        fn set_delay_works() {
            let accounts = accounts();
            let mut proxy = TransparentProxy::new(accounts.bob, accounts.alice);

            ink::env::test::set_caller::<ink::env::DefaultEnvironment>(accounts.alice);
            proxy.set_upgrade_delay_blocks(50).unwrap();
            assert_eq!(proxy.upgrade_delay_blocks(), 50);
        }

        #[ink::test]
        fn zero_address_rejected() {
            let accounts = accounts();
            let mut proxy = TransparentProxy::new(accounts.bob, accounts.alice);

            ink::env::test::set_caller::<ink::env::DefaultEnvironment>(accounts.alice);
            assert_eq!(
                proxy.set_implementation(AccountId::from([0u8; 32])),
                Err(ProxyError::InvalidImplementation)
            );
        }
    }
}
