# Access Control Audit

**Scope**: All ink! smart contracts under `contracts/`  
**Primary file**: `contracts/traits/src/access_control.rs`  
**Contract entry point**: `contracts/lib/src/lib.rs` (`PropertyRegistry`)

---

## 1. Architecture Overview

PropChain uses a two-layer access control model:

| Layer | Location | Description |
|-------|----------|-------------|
| RBAC (Role-Based) | `AccessControl` storage item | Roles assigned to accounts; roles map to permissions via `role_permissions` |
| Direct permissions | `AccessControl.account_permissions` | Per-account overrides independent of role |
| Legacy guardian mapping | `PropertyRegistry.pause_guardians` | `Mapping<AccountId, bool>` — kept for backwards-compatibility; superseded by `Role::PauseGuardian` |

All access control state mutations are recorded in two audit systems:
- **Permission audit log** (`AccessControl.audit_log`) — RBAC changes only
- **Security audit trail** (`AuditTrail`) — all security events with a Blake2x256 hash chain for tamper evidence

---

## 2. Roles

Defined in `contracts/traits/src/access_control.rs`:

| Role | Inherits from | Description |
|------|--------------|-------------|
| `SuperAdmin` | — | Highest privilege; required for `force_emergency_stop` and key rotation bootstrap |
| `Admin` | `SuperAdmin` | Can grant/revoke roles, configure all sub-systems, call `ensure_admin_rbac`-guarded functions |
| `OracleAdmin` | `Admin`, `SuperAdmin` | Manages oracle configuration |
| `ComplianceAdmin` | `Admin`, `SuperAdmin` | Manages compliance registry |
| `FeeAdmin` | `Admin`, `SuperAdmin` | Manages fee configuration |
| `BridgeOperator` | `Admin`, `SuperAdmin` | Manages bridge operations |
| `Verifier` | `Admin`, `SuperAdmin` | Issues and revokes property badges |
| `PauseGuardian` | `Admin`, `SuperAdmin` | Can pause/resume the contract; granted at bootstrap to the deployer |
| `Manager` | `Admin`, `SuperAdmin` | General management operations |

**Role inheritance rule**: A check for role `R` passes if the account holds `R` _or_ any ancestor of `R`. Ancestors are defined in `ancestor_roles()` in `access_control.rs`.

---

## 3. Resources and Actions

| Resource | Description |
|----------|-------------|
| `Global` | Contract-wide settings |
| `PropertyRegistry` | Property CRUD and configuration |
| `Oracle` | Price feed configuration |
| `Bridge` | Cross-chain bridge operations |
| `Escrow` | Escrow lifecycle |
| `Compliance` | Compliance registry integration |
| `Metadata` | Property metadata |
| `Insurance` | Insurance module |
| `Analytics` | Analytics module |
| `Fees` | Fee management |
| `Property(u64)` | Specific property by ID |
| `Token(u64)` | Specific token by ID |

| Action | Description |
|--------|-------------|
| `ManageRoles` | Grant / revoke roles |
| `Configure` | Update contract configuration (used by `ensure_admin_rbac`) |
| `Update` | Update entity data |
| `Transfer` | Transfer ownership |
| `Pause` | Pause / resume the contract |
| `Verify` | Issue / revoke badges |
| `Mint` | Mint tokens |
| `Burn` | Burn tokens |

---

## 4. Protected Functions and Required Authorization

### 4.1 Admin / RBAC-guarded (`ensure_admin_rbac`)

`ensure_admin_rbac()` passes if the caller holds:
- `Permission { resource: PropertyRegistry, action: Configure }` (direct or via role), **or**
- `Role::Admin` (or any ancestor)

| Function | File:Line |
|----------|-----------|
| `set_compliance_registry` | `lib.rs:1280` |
| `set_oracle` | `lib.rs:1314` |
| `change_admin` | `lib.rs:1384` |
| `set_fee_manager` | `lib.rs:1439` |
| `set_identity_registry` | `lib.rs:1469` |
| `set_batch_config` | `lib.rs:1485` |
| `set_pause_guardian` | `lib.rs:1841` |
| `grant_role` | `lib.rs:1876` |
| `revoke_role` | `lib.rs:1908` |
| `set_min_reputation_threshold` | `lib.rs:3183` |
| `update_deps` | `lib.rs:3610` |
| `add_badge_verifier` / `remove_badge_verifier` | `lib.rs:4109`, `lib.rs:4139` |

### 4.2 Pause / Emergency Controls

| Function | Required Authorization | Notes |
|----------|----------------------|-------|
| `pause_contract` | `Role::Admin` **or** `pause_guardians[caller]` **or** `Role::PauseGuardian` | Standard pause with optional duration |
| `emergency_pause` | Same as `pause_contract` | Logs `EmergencyAction` before pausing; no auto-resume |
| `force_emergency_stop` | `Role::SuperAdmin` only | Overrides already-paused state; clears auto-resume and pending approvals |
| `try_auto_resume` | None (public) | Only succeeds if `auto_resume_at` has elapsed |
| `request_resume` | `Role::Admin` **or** `pause_guardians[caller]` **or** `Role::PauseGuardian` | Starts multi-sig resume flow |
| `approve_resume` | Same as `request_resume` | Adds approval; executes resume when threshold met |

### 4.3 Key Rotation (RBAC module)

| Function | Authorization | Cooldown |
|----------|--------------|----------|
| `request_key_rotation` | Account itself (no external auth) | Blocked if rotation already pending |
| `confirm_key_rotation` | Designated new account only | Must wait `KEY_ROTATION_COOLDOWN_BLOCKS` |
| `cancel_key_rotation` | Old **or** new account | None |

Transfers all roles from the old account to the new account atomically.

---

## 5. Audit Logging

### 5.1 Permission Audit Log (`AccessControl.audit_log`)

Every RBAC mutation emits an `AuditAction` entry containing:

| Field | Description |
|-------|-------------|
| `id` | Sequential 1-indexed record ID |
| `actor` | Account initiating the change |
| `target` | Account affected |
| `action` | `RoleGranted`, `RoleRevoked`, `PermissionGrantedToRole`, etc. |
| `role` | Role involved (optional) |
| `permission` | Permission involved (optional) |
| `block_number` | On-chain block number |
| `timestamp` | Block timestamp |

Queried via `get_permission_audit_entry(id)` and `audit_count()`.

### 5.2 Security Audit Trail (`AuditTrail`)

All security-relevant operations (including all role changes, pauses, admin changes, and access violations) are logged in a tamper-evident hash chain:

- Each record's `record_hash` is Blake2x256 over `(prev_hash, id, actor, event_type, severity, resource_id, extra_data, block_number, timestamp)`.
- Chain integrity can be verified on-chain via `verify_audit_integrity(from_id, to_id)` (range ≤ 100 for gas safety).
- Secondary indices by `actor` and `event_type` enable efficient historical queries.

Security event severities for access-control events:

| Event | Severity |
|-------|----------|
| `AdminChanged` | Critical |
| `RoleGranted` | Critical |
| `RoleRevoked` | Critical |
| `ContractPaused` | Critical |
| `ContractResumed` | Critical |
| `EmergencyAction` | Critical |
| `PauseGuardianUpdated` | High |
| `UnauthorizedAccess` | Critical |

---

## 6. Bootstrap Sequence

On contract deployment (`new()`):

1. The deployer becomes `admin` (storage field) and is granted `Role::SuperAdmin` and `Role::Admin` via `AccessControl::bootstrap`.
2. The deployer is also granted `Role::Verifier` and `Role::PauseGuardian` via `grant_role`.
3. `PauseInfo.required_approvals` is set from the constructor parameter (recommended: ≥ 2 for production).

---

## 7. Known Gaps and Recommendations

| # | Finding | Recommendation |
|---|---------|---------------|
| 1 | `pause_guardians` mapping and `Role::PauseGuardian` were previously inconsistent — the mapping was checked but the role was not. **Fixed** in `feat/security-audit-and-emergency-controls`. | Deprecate the `pause_guardians` mapping in a future release; migrate all callers to `Role::PauseGuardian`. |
| 2 | `ancestor_roles()` is hand-coded; adding a new role requires updating the list in two places (`ancestor_roles` and `all_roles`). | Consider a declarative role DAG to reduce maintenance surface. |
| 3 | `ensure_admin_rbac` returns `bool`; callers must remember to emit an audit event on failure. | Refactor to a `Result<(), Error>` that auto-logs the violation. |
| 4 | `change_admin` updates both the `admin` storage field and the RBAC role but the two can drift if one reverts. | Merge the two into a single authoritative source. |

---

## 8. Testing Coverage

| Test file | Coverage area |
|-----------|--------------|
| `tests/security_access_control_tests.rs` | Role grant/revoke, permission checks, unauthorized access |
| `tests/security_fuzzing_tests.rs` | Fuzz access control inputs |
| `tests/integration_tests.rs` | End-to-end admin flows |

Run the access-control tests:

```bash
cargo test --test security_access_control_tests
```
