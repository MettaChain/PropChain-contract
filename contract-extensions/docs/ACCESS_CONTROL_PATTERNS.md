# Access Control and Role Management

PropChain uses several authorization shapes. Choose the simplest model that
matches the contract's operational and threat model; do not treat an `admin`
field as equivalent to a full role system.

## Owner/admin checks (Ownable-style)

The cited contracts implement owner/admin checks locally rather than exposing a
single `Ownable` trait in `contracts/traits/`. Staking records its constructor
caller as `admin` and checks that caller before privileged messages. The
compliance registry similarly stores an `owner` and has an `ensure_owner`
guard. The factory exposes `change_admin`, which is a direct one-step transfer.

```rust
// contracts/staking/src/lib.rs
fn ensure_admin(&self) -> Result<(), Error> {
    if self.env().caller() != self.admin {
        return Err(Error::Unauthorized);
    }
    Ok(())
}
```

This pattern is simple and cheap for one trusted operator. It creates a
single-key trust boundary: compromise or loss of that key affects every
operation guarded by it. Validate proposed account IDs and emit an event for
security-relevant transfers in new code.

## Role-based access control (RBAC)

The reusable `propchain_traits::AccessControl` storage item defines a role
hierarchy, role-specific administrators, account/role permissions, key
rotation, and an audit log. Its role assignments use a mapping keyed by
`(AccountId, Role)`. `ensure_has_role` is the guard; a `SuperAdmin` inherits
subordinate roles. The hierarchy, cache invalidation, and audit behavior are
described in [`docs/access_control_audit.md`](../../docs/access_control_audit.md).

```rust
// contracts/traits/src/access_control.rs
self.ensure_has_role(actor, required_admin)?;
self.role_assignments.insert((target, role), &true);
self.invalidate_cache()?;
```

The current `contracts/compliance_registry/lib.rs` code uses an owner field and
`ensure_owner` for its privileged operations; do not describe that implementation
as using the reusable RBAC item. The traits module is the source for role-based
examples and can be embedded by contracts that need the richer model.

## Threshold authorization and governance

`contracts/governance/src/lib.rs` stores signers and a `threshold`. A proposal
becomes approved when its affirmative votes meet that threshold; non-emergency
execution also observes a timelock. Construction clamps invalid thresholds to
the number of unique signers. This is a threshold authorization flow, not
necessarily a cryptographic multisignature transaction: each signer submits a
vote on-chain. The traits module also defines bridge request flows that collect
signatures.

```rust
// contracts/governance/src/lib.rs
if proposal.votes_for >= proposal.threshold {
    let timelock = now.saturating_add(self.timelock_blocks);
    proposal.timelock_until = timelock;
    self.apply_status_transition(&mut proposal, ProposalStatus::Approved);
}
```

## Two-step transfer and pause controls

Prefer a two-step handoff when an accidental transfer could permanently lose
administrative control. The proxy implements `transfer_admin` (nominate) and
`accept_admin` (the nominee confirms); until acceptance, the current admin
retains control. By contrast, the factory's `change_admin` and the multicall
contract's `transfer_admin` take effect immediately.

The `contracts/multicall/src/lib.rs` pause and unpause messages are admin-only,
emit a `PauseToggled` event, and state-changing operations guard against the
paused state. In a role-based system, a limited `PauseGuardian` role can be
safer than giving every emergency operator full administration.

## Decision guide

| Use | When it fits | Trade-off |
|---|---|---|
| Owner/admin | One trusted operator and few privileged actions | Simple, but a single key controls all guarded operations |
| RBAC | Different teams need different permissions or auditable role grants | More storage and policy complexity; define hierarchy and grant/revoke rules carefully |
| Threshold governance | High-impact changes should require approval from multiple signers | More transactions and coordination; thresholds, signer removal, and timelocks need explicit policy |

A common composition is RBAC for routine operations, a threshold/timelock for
upgrades or treasury actions, and a narrowly scoped pause role for emergencies.

## Testing authorization

Test both allowed and denied callers for every privileged message. Set the
caller to a non-admin account and assert the documented unauthorized error;
then set it to an authorized caller and assert the state change. For RBAC, also
test role inheritance, role-admin restrictions, revocation/cache invalidation,
and key-rotation cooldown/expiry. Existing compliance-registry tests include
owner and non-owner cases; access-control unit tests exercise role inheritance
and unauthorized grants.

### Source references

- [`contracts/traits/src/access_control.rs`](../../contracts/traits/src/access_control.rs)
- [`contracts/traits/src/admin_multisig.rs`](../../contracts/traits/src/admin_multisig.rs)
- [`contracts/traits/src/bridge.rs`](../../contracts/traits/src/bridge.rs)
- [`contracts/compliance_registry/lib.rs`](../../contracts/compliance_registry/lib.rs)
- [`contracts/staking/src/lib.rs`](../../contracts/staking/src/lib.rs)
- [`contracts/governance/src/lib.rs`](../../contracts/governance/src/lib.rs)
- [`contracts/factory/src/lib.rs`](../../contracts/factory/src/lib.rs)
- [`contracts/proxy/src/lib.rs`](../../contracts/proxy/src/lib.rs)
- [`contracts/multicall/src/lib.rs`](../../contracts/multicall/src/lib.rs)
- [`docs/access_control_audit.md`](../../docs/access_control_audit.md)
