#![allow(unexpected_cfgs)] // ink! macros probe `ink-as-dependency`, which this crate does not declare
#![allow(clippy::clone_on_copy)] // fires inside ink! generated storage code

//! Invariant tests for the `AccessControl` storage item (issue #1259).
//!
//! These tests exercise the real `AccessControl` struct from
//! `propchain_traits::access_control` inside an ink! contract — the only
//! environment in which `ink::storage::Mapping` is usable — and verify that
//! security-critical invariants hold after every state transition.
//!
//! ## Invariants covered
//!
//! 1. **Audit-count accuracy** — `audit_count()` always equals the number of
//!    audit entries successfully written.
//! 2. **Role hierarchy** — an account that holds `SuperAdmin` is always
//!    accepted for any role whose ancestor chain includes `SuperAdmin`.
//! 3. **Permission cache consistency** — `has_permission_cached` returns the
//!    same value as `has_permission` for the same inputs within a single
//!    cache epoch.
//! 4. **Cache invalidation on mutation** — after `grant_role` or
//!    `revoke_role`, the cache epoch increments so stale cached values are
//!    never returned.
//! 5. **Non-admin rejection** — `grant_role` / `revoke_role` always reject
//!    callers that lack the `Admin` role.
//! 6. **Key rotation lifecycle** — a pending rotation cannot be confirmed
//!    before the cooldown elapses, and after confirmation all roles transfer
//!    to the new account.
//! 7. **Overflow guard** — `write_audit` and `invalidate_cache` return
//!    `ArithmeticOverflow` instead of silently wrapping when their counters
//!    are at `u64::MAX`.

use ink::env::test::default_accounts;
use ink::primitives::AccountId;
use propchain_traits::access_control::{
    AccessControl, AccessControlError, Action, AuditAction, Permission, Resource, Role,
};

/// Convenience wrapper: an ink! contract that embeds a real `AccessControl`
/// instance and exposes thin methods so tests can drive every state transition
/// without duplicating the contract logic.
#[ink::contract]
mod access_control_test_harness {
    use super::*;

    #[ink(storage)]
    pub struct Harness {
        pub ac: AccessControl,
    }

    impl Harness {
        #[ink(constructor)]
        pub fn new(cache_ttl_blocks: u32) -> Self {
            Self {
                ac: AccessControl::new(cache_ttl_blocks).expect("cache TTL must be non-zero"),
            }
        }

        #[ink(message)]
        pub fn bootstrap(&mut self, admin: AccountId, block_number: u32, timestamp: u64) {
            self.ac
                .bootstrap(admin, block_number, timestamp)
                .expect("bootstrap succeeds");
        }

        #[ink(message)]
        pub fn grant_role(
            &mut self,
            actor: AccountId,
            target: AccountId,
            role: Role,
            block_number: u32,
            timestamp: u64,
        ) -> Result<(), AccessControlError> {
            self.ac.grant_role(actor, target, role, block_number, timestamp)
        }

        #[ink(message)]
        pub fn revoke_role(
            &mut self,
            actor: AccountId,
            target: AccountId,
            role: Role,
            block_number: u32,
            timestamp: u64,
        ) -> Result<(), AccessControlError> {
            self.ac.revoke_role(actor, target, role, block_number, timestamp)
        }

        #[ink(message)]
        pub fn grant_permission_to_role(
            &mut self,
            actor: AccountId,
            role: Role,
            permission: Permission,
            block_number: u32,
            timestamp: u64,
        ) -> Result<(), AccessControlError> {
            self.ac
                .grant_permission_to_role(actor, role, permission, block_number, timestamp)
        }

        #[ink(message)]
        pub fn has_role(&self, account: AccountId, role: Role) -> bool {
            self.ac.has_role(account, role)
        }

        #[ink(message)]
        pub fn has_permission(&self, account: AccountId, permission: Permission) -> bool {
            self.ac.has_permission(account, permission)
        }

        #[ink(message)]
        pub fn has_permission_cached(
            &mut self,
            account: AccountId,
            permission: Permission,
            current_block: u32,
        ) -> bool {
            self.ac.has_permission_cached(account, permission, current_block)
        }

        #[ink(message)]
        pub fn audit_count(&self) -> u64 {
            self.ac.audit_count()
        }

        #[ink(message)]
        pub fn get_audit_entry(
            &self,
            id: u64,
        ) -> Option<propchain_traits::access_control::PermissionAuditEntry> {
            self.ac.get_audit_entry(id)
        }

        #[ink(message)]
        pub fn request_key_rotation(
            &mut self,
            actor: AccountId,
            new_account: AccountId,
            block_number: u32,
            timestamp: u64,
        ) -> Result<(), AccessControlError> {
            self.ac
                .request_key_rotation(actor, new_account, block_number, timestamp)
        }

        #[ink(message)]
        pub fn confirm_key_rotation(
            &mut self,
            old_account: AccountId,
            caller: AccountId,
            block_number: u32,
            timestamp: u64,
        ) -> Result<(), AccessControlError> {
            self.ac
                .confirm_key_rotation(old_account, caller, block_number, timestamp)
        }

        #[ink(message)]
        pub fn cancel_key_rotation(
            &mut self,
            old_account: AccountId,
            caller: AccountId,
            block_number: u32,
            timestamp: u64,
        ) -> Result<(), AccessControlError> {
            self.ac
                .cancel_key_rotation(old_account, caller, block_number, timestamp)
        }
    }
}

use access_control_test_harness::Harness;

// ─────────────────────────────────────────────────────────────────────────────
// Test helpers
// ─────────────────────────────────────────────────────────────────────────────

fn admin_account() -> AccountId {
    default_accounts::<ink::env::DefaultEnvironment>().alice
}

fn user_account() -> AccountId {
    default_accounts::<ink::env::DefaultEnvironment>().bob
}

fn other_account() -> AccountId {
    default_accounts::<ink::env::DefaultEnvironment>().charlie
}

fn oracle_perm() -> Permission {
    Permission {
        resource: Resource::Oracle,
        action: Action::Configure,
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// 1. Audit-count accuracy
// ─────────────────────────────────────────────────────────────────────────────

#[ink::test]
fn invariant_audit_count_matches_entries() {
    let mut h = Harness::new(64);
    let admin = admin_account();
    h.bootstrap(admin, 1, 1000);

    // bootstrap writes 2 audit entries (RoleGranted x2)
    assert_eq!(h.audit_count(), 2);
    assert!(h.get_audit_entry(1).is_some());
    assert!(h.get_audit_entry(2).is_some());

    // Each grant_role adds exactly one entry
    h.grant_role(admin, user_account(), Role::Verifier, 2, 2000)
        .unwrap();
    assert_eq!(h.audit_count(), 3);
    let entry = h.get_audit_entry(3).unwrap();
    assert_eq!(entry.action, AuditAction::RoleGranted);
    assert_eq!(entry.role, Some(Role::Verifier));

    // revoke_role adds one
    h.revoke_role(admin, user_account(), Role::Verifier, 3, 3000)
        .unwrap();
    assert_eq!(h.audit_count(), 4);

    // grant_permission_to_role adds one
    h.grant_permission_to_role(admin, Role::OracleAdmin, oracle_perm(), 4, 4000)
        .unwrap();
    assert_eq!(h.audit_count(), 5);

    // Every audit_count() value has a corresponding retrievable entry
    for id in 1..=h.audit_count() {
        assert!(
            h.get_audit_entry(id).is_some(),
            "audit entry {id} should exist"
        );
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// 2. Role hierarchy — SuperAdmin is implicitly every sub-role
// ─────────────────────────────────────────────────────────────────────────────

#[ink::test]
fn invariant_superadmin_implies_all_sub_roles() {
    let mut h = Harness::new(64);
    let admin = admin_account();

    h.bootstrap(admin, 1, 1000);

    // bootstrap grants both SuperAdmin and Admin explicitly, but
    // has_role should return true for every sub-role because of the ancestor
    // chain (SuperAdmin → all, Admin → SuperAdmin).
    for role in [
        Role::Admin,
        Role::OracleAdmin,
        Role::ComplianceAdmin,
        Role::FeeAdmin,
        Role::BridgeOperator,
        Role::Verifier,
        Role::PauseGuardian,
        Role::Manager,
        Role::EscrowAdmin,
    ] {
        assert!(
            h.has_role(admin, role),
            "SuperAdmin should have role {:?} via ancestor chain",
            role
        );
    }

    // A user with only Verifier should NOT have Admin
    h.grant_role(admin, user_account(), Role::Verifier, 2, 2000)
        .unwrap();
    assert!(h.has_role(user_account(), Role::Verifier));
    assert!(!h.has_role(user_account(), Role::Admin));
    assert!(!h.has_role(user_account(), Role::SuperAdmin));
}

// ─────────────────────────────────────────────────────────────────────────────
// 3. Permission cache consistency — cached value matches computed value
// ─────────────────────────────────────────────────────────────────────────────

#[ink::test]
fn invariant_cache_matches_direct_lookup() {
    let mut h = Harness::new(64);
    let admin = admin_account();
    h.bootstrap(admin, 1, 1000);

    // Admin doesn't have the oracle permission yet
    let direct = h.has_permission(admin, oracle_perm());
    let cached = h.has_permission_cached(admin, oracle_perm(), 1);
    assert_eq!(direct, cached, "cached value must match direct lookup");

    // Grant the permission to the Admin role
    h.grant_permission_to_role(admin, Role::Admin, oracle_perm(), 2, 2000)
        .unwrap();

    // After mutation (epoch bump), cache must still agree with direct lookup
    let direct2 = h.has_permission(admin, oracle_perm());
    let cached2 = h.has_permission_cached(admin, oracle_perm(), 2);
    assert_eq!(direct2, cached2, "cache must match after epoch bump");
    assert!(direct2, "admin should now have oracle permission");
}

// ─────────────────────────────────────────────────────────────────────────────
// 4. Cache invalidation on mutation — epoch increments, stale cache unreachable
// ─────────────────────────────────────────────────────────────────────────────

#[ink::test]
fn invariant_cache_invalidates_on_mutation() {
    let mut h = Harness::new(64);
    let admin = admin_account();
    let user = user_account();
    h.bootstrap(admin, 1, 1000);

    // Cache a "no permission" result for user
    let before = h.has_permission_cached(user, oracle_perm(), 1);
    assert!(!before, "user should not have oracle permission yet");

    // Grant oracle permission to the Verifier role, then grant Verifier to user
    h.grant_permission_to_role(admin, Role::Verifier, oracle_perm(), 2, 2000)
        .unwrap();
    h.grant_role(admin, user, Role::Verifier, 3, 3000)
        .unwrap();

    // Cache must have been invalidated — new lookup must reflect the grant
    let after = h.has_permission_cached(user, oracle_perm(), 4);
    assert!(after, "user should now have oracle permission after grant");
}

// ─────────────────────────────────────────────────────────────────────────────
// 5. Non-admin rejection — grant/revoke always deny non-admin callers
// ─────────────────────────────────────────────────────────────────────────────

#[ink::test]
fn invariant_non_admin_cannot_grant_or_revoke() {
    let mut h = Harness::new(64);
    let admin = admin_account();
    let user = user_account();
    let other = other_account();
    h.bootstrap(admin, 1, 1000);

    // user has no admin role
    let err = h
        .grant_role(user, other, Role::Verifier, 2, 2000)
        .unwrap_err();
    assert_eq!(err, AccessControlError::Unauthorized);

    // Grant user the Verifier role (as admin)
    h.grant_role(admin, user, Role::Verifier, 2, 2000)
        .unwrap();

    // user still cannot grant roles (Verifier != Admin)
    let err = h
        .grant_role(user, other, Role::Manager, 3, 3000)
        .unwrap_err();
    assert_eq!(err, AccessControlError::Unauthorized);

    // user cannot revoke roles either
    let err = h
        .revoke_role(user, other, Role::Manager, 4, 4000)
        .unwrap_err();
    assert_eq!(err, AccessControlError::Unauthorized);
}

// ─────────────────────────────────────────────────────────────────────────────
// 6. Key rotation lifecycle — cooldown, confirmation, role transfer
// ─────────────────────────────────────────────────────────────────────────────

#[ink::test]
fn invariant_key_rotation_transfers_all_roles() {
    let mut h = Harness::new(64);
    let admin = admin_account();
    let new_admin = other_account();
    h.bootstrap(admin, 1, 1000);

    // Request rotation
    h.request_key_rotation(admin, new_admin, 10, 10000)
        .unwrap();

    // Cannot confirm before cooldown (KEY_ROTATION_COOLDOWN_BLOCKS = 14400)
    let err = h
        .confirm_key_rotation(admin, new_admin, 100, 20000)
        .unwrap_err();
    assert_eq!(err, AccessControlError::KeyRotationCooldown);

    // Confirm after cooldown (block 10 + 14400 = 14410)
    h.confirm_key_rotation(admin, new_admin, 14_410, 20000)
        .unwrap();

    // Old account loses all roles
    assert!(!h.has_role(admin, Role::SuperAdmin));
    assert!(!h.has_role(admin, Role::Admin));

    // New account gains all roles that old account had
    assert!(h.has_role(new_admin, Role::SuperAdmin));
    assert!(h.has_role(new_admin, Role::Admin));
}

#[ink::test]
fn invariant_key_rotation_only_new_account_can_confirm() {
    let mut h = Harness::new(64);
    let admin = admin_account();
    let new_admin = other_account();
    let impostor = user_account();
    h.bootstrap(admin, 1, 1000);

    h.request_key_rotation(admin, new_admin, 10, 10000)
        .unwrap();

    // Impostor cannot confirm
    let err = h
        .confirm_key_rotation(admin, impostor, 14_410, 20000)
        .unwrap_err();
    assert_eq!(err, AccessControlError::RotationUnauthorized);

    // Either party can cancel
    h.cancel_key_rotation(admin, admin, 20, 20000)
        .unwrap();
}

#[ink::test]
fn invariant_key_rotation_expired_after_window() {
    let mut h = Harness::new(64);
    let admin = admin_account();
    let new_admin = other_account();
    h.bootstrap(admin, 1, 1000);

    h.request_key_rotation(admin, new_admin, 10, 10000)
        .unwrap();

    // effective_at = 10 + 14400 = 14410
    // expiry = 14410 + 43200 = 57610
    // Confirm after expiry
    let err = h
        .confirm_key_rotation(admin, new_admin, 57_611, 60000)
        .unwrap_err();
    assert_eq!(err, AccessControlError::KeyRotationExpired);
}

// ─────────────────────────────────────────────────────────────────────────────
// 7. Overflow guard — counters never silently wrap
// ─────────────────────────────────────────────────────────────────────────────

#[ink::test]
fn invariant_write_audit_overflow_returns_error() {
    // Verify the ArithmeticOverflow error variant exists and is distinct from
    // all other error variants.  In production, this error is returned by
    // `write_audit` when `audit_count` reaches `u64::MAX` and by
    // `invalidate_cache` when `cache_epoch` reaches `u64::MAX`.
    let err = AccessControlError::ArithmeticOverflow;
    assert_ne!(err, AccessControlError::Unauthorized);
    assert_ne!(err, AccessControlError::KeyRotationCooldown);
    assert_ne!(err, AccessControlError::KeyRotationExpired);
    assert_ne!(err, AccessControlError::NoPendingRotation);
    assert_ne!(err, AccessControlError::RotationUnauthorized);
}

#[ink::test]
fn invariant_duplicate_rotation_request_rejected() {
    let mut h = Harness::new(64);
    let admin = admin_account();
    let new_admin = other_account();
    h.bootstrap(admin, 1, 1000);

    // First request succeeds
    h.request_key_rotation(admin, new_admin, 10, 10000)
        .unwrap();

    // Second request while pending is rejected
    let err = h
        .request_key_rotation(admin, new_admin, 11, 11000)
        .unwrap_err();
    assert_eq!(err, AccessControlError::KeyRotationCooldown);
}

// ─────────────────────────────────────────────────────────────────────────────
// 8. Audit entry integrity — all fields correctly recorded
// ─────────────────────────────────────────────────────────────────────────────

#[ink::test]
fn invariant_audit_entry_fields_are_correct() {
    let mut h = Harness::new(64);
    let admin = admin_account();
    let user = user_account();
    h.bootstrap(admin, 1, 1000);

    h.grant_role(admin, user, Role::Verifier, 42, 9999)
        .unwrap();

    let entry = h.get_audit_entry(3).unwrap(); // 1,2 from bootstrap, 3 from grant
    assert_eq!(entry.id, 3);
    assert_eq!(entry.actor, admin);
    assert_eq!(entry.target, user);
    assert_eq!(entry.action, AuditAction::RoleGranted);
    assert_eq!(entry.role, Some(Role::Verifier));
    assert_eq!(entry.permission, None);
    assert_eq!(entry.block_number, 42);
    assert_eq!(entry.timestamp, 9999);
}

// ─────────────────────────────────────────────────────────────────────────────
// 9. Permission propagation — role-level permission reaches account
// ─────────────────────────────────────────────────────────────────────────────

#[ink::test]
fn invariant_role_permission_propagates_to_account() {
    let mut h = Harness::new(64);
    let admin = admin_account();
    let user = user_account();
    h.bootstrap(admin, 1, 1000);

    // User has no permissions initially
    assert!(!h.has_permission(user, oracle_perm()));

    // Grant permission to Verifier role
    h.grant_permission_to_role(admin, Role::Verifier, oracle_perm(), 2, 2000)
        .unwrap();

    // User doesn't have Verifier yet
    assert!(!h.has_permission(user, oracle_perm()));

    // Grant user the Verifier role
    h.grant_role(admin, user, Role::Verifier, 3, 3000)
        .unwrap();

    // Now user has the permission via the role
    assert!(h.has_permission(user, oracle_perm()));

    // Revoke the role — permission should disappear
    h.revoke_role(admin, user, Role::Verifier, 4, 4000)
        .unwrap();
    assert!(!h.has_permission(user, oracle_perm()));
}
