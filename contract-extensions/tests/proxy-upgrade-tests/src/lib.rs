
//! Unit tests for the transparent proxy contract (#1462).
//!
//! The issue describes a `delegatecall` proxy with shared storage. This
//! implementation is a transparent forwarding proxy that stores an
//! implementation address and forwards calls using `call_v1`.
//!
//! Consequently, there is no delegatecall or shared storage to assert, and
//! the off-chain test engine cannot execute forwarded cross-contract calls.
//! These tests cover the implemented functionality:
//! - Constructor validation and initialization.
//! - Timelocked implementation upgrades.
//! - Upgrade delay configuration and boundary conditions.
//! - Upgrade cancellation and restaging.
//! - Admin authorization and two-step ownership transfer.

#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(test)]
mod tests {
    use ink::env::{test, DefaultEnvironment};
    use ink::primitives::AccountId;

    use propchain_proxy::propchain_proxy::{
        ProxyError, TransparentProxy, DEFAULT_UPGRADE_DELAY_BLOCKS,
        MAX_UPGRADE_DELAY_BLOCKS, MIN_UPGRADE_DELAY_BLOCKS,
    };

    type Accounts = test::DefaultAccounts<DefaultEnvironment>;

    // ---------------------------------------------------------------------
    // Test helpers
    // ---------------------------------------------------------------------

    /// Returns the zero account identifier.
    fn zero() -> AccountId {
        AccountId::from([0u8; 32])
    }

    /// Sets up a proxy with Alice as admin and Charlie as implementation.
    fn setup() -> (TransparentProxy, Accounts) {
        let accounts = test::default_accounts::<DefaultEnvironment>();

        test::set_caller::<DefaultEnvironment>(accounts.alice);

        let proxy = TransparentProxy::new(accounts.charlie, accounts.alice)
            .expect("proxy initialization should succeed");

        (proxy, accounts)
    }

    /// Advances the test environment by the specified number of blocks.
    fn advance(blocks: u64) {
        for _ in 0..blocks {
            test::advance_block::<DefaultEnvironment>();
        }
    }

    // ---------------------------------------------------------------------
    // Initialization
    // ---------------------------------------------------------------------

    #[ink::test]
    fn initializes_with_the_given_implementation_and_admin() {
        let (proxy, accounts) = setup();

        assert_eq!(proxy.implementation(), accounts.charlie);
        assert_eq!(proxy.admin(), accounts.alice);
        assert_eq!(proxy.pending_implementation(), zero());
        assert_eq!(proxy.pending_admin(), zero());
        assert_eq!(proxy.upgrade_effective_at(), 0);
        assert_eq!(
            proxy.upgrade_delay_blocks(),
            DEFAULT_UPGRADE_DELAY_BLOCKS
        );
    }

    #[ink::test]
    fn the_zero_address_is_rejected_at_construction() {
        let accounts = test::default_accounts::<DefaultEnvironment>();

        assert_eq!(
            TransparentProxy::new(zero(), accounts.alice).err(),
            Some(ProxyError::InvalidImplementation)
        );

        assert_eq!(
            TransparentProxy::new(accounts.charlie, zero()).err(),
            Some(ProxyError::InvalidAdmin)
        );
    }

    // ---------------------------------------------------------------------
    // Implementation upgrade lifecycle
    // ---------------------------------------------------------------------

    #[ink::test]
    fn staging_an_upgrade_records_the_pending_implementation_and_effective_block() {
        let (mut proxy, accounts) = setup();

        advance(5);
        proxy.set_implementation(accounts.django).unwrap();

        assert_eq!(proxy.pending_implementation(), accounts.django);
        assert_eq!(
            proxy.upgrade_effective_at(),
            5 + DEFAULT_UPGRADE_DELAY_BLOCKS
        );

        // The active implementation remains unchanged until confirmation.
        assert_eq!(proxy.implementation(), accounts.charlie);
    }

    #[ink::test]
    fn an_upgrade_cannot_be_confirmed_before_the_delay_and_can_at_the_boundary() {
        let (mut proxy, accounts) = setup();

        proxy.set_implementation(accounts.django).unwrap();

        let effective_block = proxy.upgrade_effective_at();

        advance(effective_block - 1);

        assert_eq!(
            proxy.confirm_implementation(),
            Err(ProxyError::DelayNotElapsed)
        );
        assert_eq!(proxy.implementation(), accounts.charlie);

        // Confirmation becomes valid exactly at the effective block.
        advance(1);
        proxy.confirm_implementation().unwrap();

        assert_eq!(proxy.implementation(), accounts.django);
        assert_eq!(proxy.pending_implementation(), zero());
        assert_eq!(proxy.upgrade_effective_at(), 0);
    }

    #[ink::test]
    fn confirming_without_a_staged_upgrade_fails() {
        let (mut proxy, _) = setup();

        assert_eq!(
            proxy.confirm_implementation(),
            Err(ProxyError::NoPendingUpgrade)
        );
    }

    #[ink::test]
    fn a_staged_upgrade_can_be_cancelled_and_cannot_then_be_confirmed() {
        let (mut proxy, accounts) = setup();

        proxy.set_implementation(accounts.django).unwrap();
        proxy.cancel_implementation().unwrap();

        assert_eq!(proxy.pending_implementation(), zero());
        assert_eq!(proxy.upgrade_effective_at(), 0);

        advance(DEFAULT_UPGRADE_DELAY_BLOCKS + 1);

        assert_eq!(
            proxy.confirm_implementation(),
            Err(ProxyError::NoPendingUpgrade)
        );
        assert_eq!(
            proxy.cancel_implementation(),
            Err(ProxyError::NoPendingUpgrade)
        );
    }

    #[ink::test]
    fn restaging_replaces_the_pending_implementation_and_restarts_the_timer() {
        let (mut proxy, accounts) = setup();

        proxy.set_implementation(accounts.django).unwrap();

        advance(50);
        proxy.set_implementation(accounts.eve).unwrap();

        assert_eq!(proxy.pending_implementation(), accounts.eve);
        assert_eq!(
            proxy.upgrade_effective_at(),
            50 + DEFAULT_UPGRADE_DELAY_BLOCKS
        );
    }

    #[ink::test]
    fn staging_the_zero_address_or_the_live_implementation_is_rejected() {
        let (mut proxy, accounts) = setup();

        assert_eq!(
            proxy.set_implementation(zero()),
            Err(ProxyError::InvalidImplementation)
        );

        assert_eq!(
            proxy.set_implementation(accounts.charlie),
            Err(ProxyError::AlreadyImplemented)
        );

        assert_eq!(proxy.pending_implementation(), zero());
    }

    #[ink::test]
    fn only_the_admin_can_stage_confirm_or_cancel() {
        let (mut proxy, accounts) = setup();

        proxy.set_implementation(accounts.django).unwrap();

        test::set_caller::<DefaultEnvironment>(accounts.bob);

        assert_eq!(
            proxy.set_implementation(accounts.eve),
            Err(ProxyError::Unauthorized)
        );
        assert_eq!(
            proxy.confirm_implementation(),
            Err(ProxyError::Unauthorized)
        );
        assert_eq!(
            proxy.cancel_implementation(),
            Err(ProxyError::Unauthorized)
        );

        // Unauthorized operations must not modify the pending upgrade.
        assert_eq!(proxy.pending_implementation(), accounts.django);
    }

    // ---------------------------------------------------------------------
    // Upgrade delay configuration
    // ---------------------------------------------------------------------

    #[ink::test]
    fn the_delay_must_stay_within_its_bounds() {
        let (mut proxy, accounts) = setup();

        assert_eq!(
            proxy.upgrade_delay_bounds(),
            (MIN_UPGRADE_DELAY_BLOCKS, MAX_UPGRADE_DELAY_BLOCKS)
        );

        assert_eq!(
            proxy.set_upgrade_delay_blocks(MIN_UPGRADE_DELAY_BLOCKS - 1),
            Err(ProxyError::InvalidUpgradeDelay)
        );

        assert_eq!(
            proxy.set_upgrade_delay_blocks(MAX_UPGRADE_DELAY_BLOCKS + 1),
            Err(ProxyError::InvalidUpgradeDelay)
        );

        // Both boundary values are valid.
        proxy
            .set_upgrade_delay_blocks(MIN_UPGRADE_DELAY_BLOCKS)
            .unwrap();

        proxy
            .set_upgrade_delay_blocks(MAX_UPGRADE_DELAY_BLOCKS)
            .unwrap();

        // Only the admin can change the upgrade delay.
        test::set_caller::<DefaultEnvironment>(accounts.bob);

        assert_eq!(
            proxy.set_upgrade_delay_blocks(50),
            Err(ProxyError::Unauthorized)
        );
    }

    #[ink::test]
    fn a_new_delay_applies_to_later_stagings_only() {
        let (mut proxy, accounts) = setup();

        proxy.set_implementation(accounts.django).unwrap();

        let original_deadline = proxy.upgrade_effective_at();

        proxy.set_upgrade_delay_blocks(10).unwrap();

        // Changing the delay does not affect an existing staged upgrade.
        assert_eq!(proxy.upgrade_effective_at(), original_deadline);

        // Subsequent upgrades use the newly configured delay.
        proxy.set_implementation(accounts.eve).unwrap();

        assert_eq!(proxy.upgrade_effective_at(), 10);
    }

    // ---------------------------------------------------------------------
    // Two-step admin transfer
    // ---------------------------------------------------------------------

    #[ink::test]
    fn admin_transfer_needs_the_new_admin_to_accept() {
        let (mut proxy, accounts) = setup();

        proxy.transfer_admin(accounts.bob).unwrap();

        assert_eq!(proxy.pending_admin(), accounts.bob);
        assert_eq!(proxy.admin(), accounts.alice);

        // The proposed admin must explicitly accept the transfer.
        test::set_caller::<DefaultEnvironment>(accounts.bob);

        proxy.accept_admin().unwrap();

        assert_eq!(proxy.admin(), accounts.bob);
        assert_eq!(proxy.pending_admin(), zero());

        // The previous admin must lose administrative privileges.
        test::set_caller::<DefaultEnvironment>(accounts.alice);

        assert_eq!(
            proxy.set_implementation(accounts.django),
            Err(ProxyError::Unauthorized)
        );
    }

    #[ink::test]
    fn only_the_pending_admin_can_accept() {
        let (mut proxy, accounts) = setup();

        proxy.transfer_admin(accounts.bob).unwrap();

        test::set_caller::<DefaultEnvironment>(accounts.charlie);

        assert_eq!(
            proxy.accept_admin(),
            Err(ProxyError::Unauthorized)
        );

        assert_eq!(proxy.admin(), accounts.alice);
    }

    #[ink::test]
    fn accepting_without_a_pending_transfer_or_transferring_to_zero_fails() {
        let (mut proxy, accounts) = setup();

        test::set_caller::<DefaultEnvironment>(accounts.bob);

        assert_eq!(
            proxy.accept_admin(),
            Err(ProxyError::NoPendingAdminTransfer)
        );

        assert_eq!(
            proxy.transfer_admin(accounts.bob),
            Err(ProxyError::Unauthorized)
        );

        test::set_caller::<DefaultEnvironment>(accounts.alice);

        assert_eq!(
            proxy.transfer_admin(zero()),
            Err(ProxyError::InvalidAdmin)
        );
    }

    #[ink::test]
    fn a_pending_admin_transfer_does_not_affect_a_staged_upgrade() {
        let (mut proxy, accounts) = setup();

        proxy.set_implementation(accounts.django).unwrap();
        proxy.transfer_admin(accounts.bob).unwrap();

        assert_eq!(proxy.pending_implementation(), accounts.django);

        // The current admin retains control until the transfer is accepted.
        advance(DEFAULT_UPGRADE_DELAY_BLOCKS);

        assert!(proxy.confirm_implementation().is_ok());
    }
}
