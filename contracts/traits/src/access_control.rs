use ink::prelude::vec::Vec;
use ink::primitives::AccountId;
use ink::storage::Mapping;

#[derive(Debug, Clone, Copy, PartialEq, Eq, scale::Encode, scale::Decode)]
#[cfg_attr(
    feature = "std",
    derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
)]
pub enum Role {
    SuperAdmin,
    Admin,
    OracleAdmin,
    ComplianceAdmin,
    FeeAdmin,
    BridgeOperator,
    Verifier,
    PauseGuardian,
    Manager,
}

#[allow(clippy::cast_possible_truncation)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, scale::Encode, scale::Decode)]
#[cfg_attr(
    feature = "std",
    derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
)]
pub enum Resource {
    Global,
    PropertyRegistry,
    Oracle,
    Bridge,
    Escrow,
    Compliance,
    Metadata,
    Insurance,
    Analytics,
    Fees,
    Property(u64),
    Token(u64),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, scale::Encode, scale::Decode)]
#[cfg_attr(
    feature = "std",
    derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
)]
pub enum Action {
    ManageRoles,
    Configure,
    Update,
    Transfer,
    Pause,
    Verify,
    Mint,
    Burn,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, scale::Encode, scale::Decode)]
#[cfg_attr(
    feature = "std",
    derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
)]
pub struct Permission {
    pub resource: Resource,
    pub action: Action,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, scale::Encode, scale::Decode)]
#[cfg_attr(
    feature = "std",
    derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
)]
pub enum AuditAction {
    RoleGranted,
    RoleRevoked,
    PermissionGrantedToRole,
    PermissionRevokedFromRole,
    PermissionGrantedToAccount,
    PermissionRevokedFromAccount,
    KeyRotationRequested,
    KeyRotationCompleted,
    KeyRotationCancelled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, scale::Encode, scale::Decode)]
#[cfg_attr(
    feature = "std",
    derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
)]
pub struct PermissionAuditEntry {
    pub id: u64,
    pub actor: AccountId,
    pub target: AccountId,
    pub action: AuditAction,
    pub role: Option<Role>,
    pub permission: Option<Permission>,
    pub block_number: u32,
    pub timestamp: u64,
}

#[derive(Debug, PartialEq, Eq, scale::Encode, scale::Decode)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub enum AccessControlError {
    Unauthorized,
    KeyRotationCooldown,
    KeyRotationExpired,
    NoPendingRotation,
    RotationUnauthorized,
}

type PermissionCacheKey = (AccountId, Permission, u64);

#[ink::storage_item]
#[derive(Default)]
pub struct AccessControl {
    role_assignments: Mapping<(AccountId, Role), bool>,
    role_permissions: Mapping<(Role, Permission), bool>,
    account_permissions: Mapping<(AccountId, Permission), bool>,
    permission_cache: Mapping<PermissionCacheKey, bool>,
    audit_log: Mapping<u64, PermissionAuditEntry>,
    audit_count: u64,
    cache_epoch: u64,
    cache_ttl_blocks: u32,
    pending_rotations: Mapping<AccountId, crate::crypto::KeyRotationRequest>,
    rotation_nonce: Mapping<AccountId, u64>,
}

impl core::fmt::Debug for AccessControl {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("AccessControl")
            .field("audit_count", &self.audit_count)
            .field("cache_epoch", &self.cache_epoch)
            .field("cache_ttl_blocks", &self.cache_ttl_blocks)
            .finish()
    }
}

impl AccessControl {
    pub fn new(cache_ttl_blocks: u32) -> Self {
        Self {
            role_assignments: Mapping::default(),
            role_permissions: Mapping::default(),
            account_permissions: Mapping::default(),
            permission_cache: Mapping::default(),
            audit_log: Mapping::default(),
            audit_count: 0,
            cache_epoch: 0,
            cache_ttl_blocks,
            pending_rotations: Mapping::default(),
            rotation_nonce: Mapping::default(),
        }
    }

    pub fn bootstrap(&mut self, admin: AccountId, block_number: u32, timestamp: u64) {
        self.role_assignments
            .insert((admin, Role::SuperAdmin), &true);
        self.role_assignments.insert((admin, Role::Admin), &true);
        self.write_audit(
            admin,
            admin,
            AuditAction::RoleGranted,
            Some(Role::SuperAdmin),
            None,
            block_number,
            timestamp,
        );
        self.write_audit(
            admin,
            admin,
            AuditAction::RoleGranted,
            Some(Role::Admin),
            None,
            block_number,
            timestamp,
        );
    }

    pub fn grant_role(
        &mut self,
        actor: AccountId,
        target: AccountId,
        role: Role,
        block_number: u32,
        timestamp: u64,
    ) -> Result<(), AccessControlError> {
        self.ensure_has_role(actor, Role::Admin)?;
        self.role_assignments.insert((target, role), &true);
        self.invalidate_cache();
        self.write_audit(
            actor,
            target,
            AuditAction::RoleGranted,
            Some(role),
            None,
            block_number,
            timestamp,
        );
        Ok(())
    }

    pub fn revoke_role(
        &mut self,
        actor: AccountId,
        target: AccountId,
        role: Role,
        block_number: u32,
        timestamp: u64,
    ) -> Result<(), AccessControlError> {
        self.ensure_has_role(actor, Role::Admin)?;
        self.role_assignments.remove((target, role));
        self.invalidate_cache();
        self.write_audit(
            actor,
            target,
            AuditAction::RoleRevoked,
            Some(role),
            None,
            block_number,
            timestamp,
        );
        Ok(())
    }

    pub fn grant_permission_to_role(
        &mut self,
        actor: AccountId,
        role: Role,
        permission: Permission,
        block_number: u32,
        timestamp: u64,
    ) -> Result<(), AccessControlError> {
        self.ensure_has_role(actor, Role::Admin)?;
        self.role_permissions.insert((role, permission), &true);
        self.invalidate_cache();
        self.write_audit(
            actor,
            actor,
            AuditAction::PermissionGrantedToRole,
            Some(role),
            Some(permission),
            block_number,
            timestamp,
        );
        Ok(())
    }

    pub fn has_role(&self, account: AccountId, role: Role) -> bool {
        self.role_assignments.get((account, role)).unwrap_or(false)
            || self.ancestor_roles(role).iter().any(|ancestor| {
                self.role_assignments
                    .get((account, *ancestor))
                    .unwrap_or(false)
            })
    }

    pub fn ensure_has_role(
        &self,
        account: AccountId,
        role: Role,
    ) -> Result<(), AccessControlError> {
        if self.has_role(account, role) {
            Ok(())
        } else {
            Err(AccessControlError::Unauthorized)
        }
    }

    pub fn has_permission_cached(
        &mut self,
        account: AccountId,
        permission: Permission,
        current_block: u32,
    ) -> bool {
        if let Some(cached) = self
            .permission_cache
            .get((account, permission, self.cache_epoch))
        {
            return cached;
        }
        let value = self.has_permission(account, permission);
        self.permission_cache
            .insert((account, permission, self.cache_epoch), &value);
        let _ = current_block.saturating_add(self.cache_ttl_blocks);
        value
    }

    pub fn has_permission(&self, account: AccountId, permission: Permission) -> bool {
        if self
            .account_permissions
            .get((account, permission))
            .unwrap_or(false)
        {
            return true;
        }
        self.all_roles()
            .iter()
            .filter(|role| self.has_role(account, **role))
            .any(|role| {
                self.role_permissions
                    .get((*role, permission))
                    .unwrap_or(false)
            })
    }

    pub fn get_audit_entry(&self, id: u64) -> Option<PermissionAuditEntry> {
        self.audit_log.get(id)
    }

    pub fn audit_count(&self) -> u64 {
        self.audit_count
    }

    /// Request a key rotation. Only the account being rotated can initiate.
    /// The rotation enters a cooldown period before it can be confirmed.
    pub fn request_key_rotation(
        &mut self,
        actor: AccountId,
        new_account: AccountId,
        block_number: u32,
        timestamp: u64,
    ) -> Result<(), AccessControlError> {
        // Only the account itself can request rotation of its own keys
        if self.pending_rotations.contains(actor) {
            return Err(AccessControlError::KeyRotationCooldown);
        }

        let effective_at =
            block_number.saturating_add(crate::constants::KEY_ROTATION_COOLDOWN_BLOCKS);

        let request = crate::crypto::KeyRotationRequest {
            old_account: actor,
            new_account,
            requested_at: block_number,
            effective_at,
            confirmed: false,
        };

        self.pending_rotations.insert(actor, &request);

        let nonce = self.rotation_nonce.get(actor).unwrap_or(0);
        self.rotation_nonce.insert(actor, &nonce.saturating_add(1));

        self.write_audit(
            actor,
            new_account,
            AuditAction::KeyRotationRequested,
            None,
            None,
            block_number,
            timestamp,
        );

        Ok(())
    }

    /// Confirm a pending key rotation. Must be called by the new account
    /// after the cooldown period has elapsed. Transfers all roles from old to new.
    pub fn confirm_key_rotation(
        &mut self,
        old_account: AccountId,
        caller: AccountId,
        block_number: u32,
        timestamp: u64,
    ) -> Result<(), AccessControlError> {
        let request = self
            .pending_rotations
            .get(old_account)
            .ok_or(AccessControlError::NoPendingRotation)?;

        // Only the designated new account can confirm
        if request.new_account != caller {
            return Err(AccessControlError::RotationUnauthorized);
        }

        // Check cooldown has elapsed
        if block_number < request.effective_at {
            return Err(AccessControlError::KeyRotationCooldown);
        }

        // Check expiry
        let expiry = request
            .effective_at
            .saturating_add(crate::constants::KEY_ROTATION_EXPIRY_BLOCKS);
        if block_number > expiry {
            self.pending_rotations.remove(old_account);
            return Err(AccessControlError::KeyRotationExpired);
        }

        // Transfer all roles from old_account to new_account
        for role in self.all_roles() {
            if self
                .role_assignments
                .get((old_account, role))
                .unwrap_or(false)
            {
                self.role_assignments.remove((old_account, role));
                self.role_assignments
                    .insert((request.new_account, role), &true);
            }
        }

        self.pending_rotations.remove(old_account);
        self.invalidate_cache();

        self.write_audit(
            caller,
            old_account,
            AuditAction::KeyRotationCompleted,
            None,
            None,
            block_number,
            timestamp,
        );

        Ok(())
    }

    /// Cancel a pending key rotation. Either the old or new account can cancel.
    pub fn cancel_key_rotation(
        &mut self,
        old_account: AccountId,
        caller: AccountId,
        block_number: u32,
        timestamp: u64,
    ) -> Result<(), AccessControlError> {
        let request = self
            .pending_rotations
            .get(old_account)
            .ok_or(AccessControlError::NoPendingRotation)?;

        if caller != request.old_account && caller != request.new_account {
            return Err(AccessControlError::RotationUnauthorized);
        }

        self.pending_rotations.remove(old_account);

        self.write_audit(
            caller,
            old_account,
            AuditAction::KeyRotationCancelled,
            None,
            None,
            block_number,
            timestamp,
        );

        Ok(())
    }

    /// Get the pending key rotation for an account, if any.
    pub fn get_pending_rotation(
        &self,
        account: AccountId,
    ) -> Option<crate::crypto::KeyRotationRequest> {
        self.pending_rotations.get(account)
    }

    fn invalidate_cache(&mut self) {
        self.cache_epoch = self.cache_epoch.saturating_add(1);
    }

    fn ancestor_roles(&self, role: Role) -> Vec<Role> {
        match role {
            Role::SuperAdmin => Vec::new(),
            Role::Admin => vec![Role::SuperAdmin],
            Role::OracleAdmin => vec![Role::Admin, Role::SuperAdmin],
            Role::ComplianceAdmin => vec![Role::Admin, Role::SuperAdmin],
            Role::FeeAdmin => vec![Role::Admin, Role::SuperAdmin],
            Role::BridgeOperator => vec![Role::Admin, Role::SuperAdmin],
            Role::Verifier => vec![Role::Admin, Role::SuperAdmin],
            Role::PauseGuardian => vec![Role::Admin, Role::SuperAdmin],
            Role::Manager => vec![Role::Admin, Role::SuperAdmin],
        }
    }

    fn all_roles(&self) -> [Role; 9] {
        [
            Role::SuperAdmin,
            Role::Admin,
            Role::OracleAdmin,
            Role::ComplianceAdmin,
            Role::FeeAdmin,
            Role::BridgeOperator,
            Role::Verifier,
            Role::PauseGuardian,
            Role::Manager,
        ]
    }

    #[allow(clippy::too_many_arguments)]
    fn write_audit(
        &mut self,
        actor: AccountId,
        target: AccountId,
        action: AuditAction,
        role: Option<Role>,
        permission: Option<Permission>,
        block_number: u32,
        timestamp: u64,
    ) {
        self.audit_count = self.audit_count.saturating_add(1);
        let entry = PermissionAuditEntry {
            id: self.audit_count,
            actor,
            target,
            action,
            role,
            permission,
            block_number,
            timestamp,
        };
        self.audit_log.insert(self.audit_count, &entry);
    }
}
