# Slashing Conditions — Staking Contract

This document describes all conditions under which staked tokens can be slashed in the PropChain staking contract, the mechanics of slashing, and the impact on validators and delegators.

## Overview

Slashing is a mechanism that penalizes validators who misbehave or fail to meet their obligations. It serves as a deterrent and maintains the integrity of the PropChain network. Slashing can affect both the validator's self-stake and its delegators' delegated stake.

---

## Validator Slashing

### Trigger Conditions

A validator may be slashed under the following conditions:

| Condition | Description | Slash Rate |
|-----------|-------------|------------|
| **Data Discrepancy** | Validator submits conflicting or incorrect price feed data to the oracle | 20% of self-stake |
| **Downtime** | Validator fails to submit updates for an extended period (beyond `max_price_staleness`) | 10% of self-stake |
| **Double Signing** | Validator signs conflicting blocks or attestations | 20% of self-stake |
| **Protocol Violation** | Validator violates protocol rules determined by governance | Configurable via governance (10%–100%) |
| **Oracle Manipulation** | Validator intentionally manipulates price feeds for personal gain | 30% of self-stake + ban |
| **Security Breach** | Validator's keys are compromised, leading to unauthorized actions | 20% of self-stake + temporary suspension |

### Slashing Mechanics

The `slash_validator` function in the staking contract performs the following operations:

1. **Updates rewards**: The validator's reward accumulator is brought up to the current block
2. **Slashes self-stake**: The validator's own staked tokens are reduced by `SLASH_PERCENT` (default 20%)
3. **Slashes delegators**: Each delegator's delegated amount is reduced by the same `SLASH_PERCENT`
4. **Updates totals**: `total_delegated` for the validator and `total_delegated_stake` for the system are reduced
5. **Deactivates if below minimum**: If the remaining self-stake drops below `MIN_VALIDATOR_STAKE` (10,000,000), the validator is automatically deactivated

### Configuration Constants

```
SLASH_PERCENT          = 20    // Default slash rate (20% of stake)
MIN_VALIDATOR_STAKE    = 10_000_000  // Minimum stake to remain active
UNBONDING_PERIOD_BLOCKS = 50_400  // ~3.5 days at 6-second blocks
```

---

## Impact Analysis

### On Validators

- **Self-stake reduction**: Validator's own tokens are reduced by `SLASH_PERCENT`
- **Reputation loss**: The validator's reputation score in the oracle contract is decreased significantly
- **Automatic deactivation**: If self-stake falls below `MIN_VALIDATOR_STAKE`, the validator is deactivated
- **Commission loss**: Accumulated but unclaimed commission is retained, but future commission is affected by reduced total delegated

### On Delegators

- **Proportional reduction**: Each delegator's delegated amount is reduced by `SLASH_PERCENT`
- **No additional penalty**: Delegators are not penalized beyond the proportional reduction
- **Reward impact**: Future rewards are calculated on the reduced delegated amount
- **Unbonding**: Delegators can still initiate unbonding after slashing, subject to `UNBONDING_PERIOD_BLOCKS`

### On the Network

- **Total stake reduction**: The total staked and total delegated amounts decrease
- **Security budget**: Reduced stakeholder commitment may affect network security
- **Governance power**: Slashing reduces governance voting power proportionally

---

## Recovery Paths

### Validator Recovery

1. **Reactivate**: If the validator was deactivated due to slashing, they can:
   - Increase self-stake above `MIN_VALIDATOR_STAKE` (note: adding stake after slashing requires a new stake operation)
   - Call `reactivate_validator()` to become active again
2. **Rebuild reputation**: Participate correctly in oracle updates to gradually restore reputation score
3. **Governance appeal**: Token holders may propose a governance action to reverse or reduce a slashing event

### Delegator Recovery

1. **Claim remaining stake**: Delegators can initiate unbonding and claim their reduced amount after the unbonding period
2. **Redelegate**: Delegators can choose a different, more reliable validator
3. **Monitor validator health**: Use the oracle contract's `get_source_status` query to check validator health before delegating

---

## Prevention Measures

### For Validators

- Run redundant infrastructure to minimize downtime
- Implement monitoring and alerting for oracle submission deadlines
- Regularly update and secure validator keys
- Maintain sufficient self-stake buffer above `MIN_VALIDATOR_STAKE`
- Participate honestly in all protocol operations

### For Delegators

- Diversify delegations across multiple validators
- Monitor validator performance and reputation scores
- Use the oracle's `get_source_status` and `get_slashing_summary` queries to assess risk
- Set up alerts for slashing events via transaction monitoring

---

## Auditing & Transparency

All slashing events emit the following events:

```rust
ValidatorSlashed {
    validator: AccountId,
    slash_amount: u128,          // Amount slashed from self-stake
    delegated_reduction: u128,   // Total reduction across all delegators
}

ValidatorDeactivated {
    validator: AccountId,
    reason: DeactivationReason,  // Slashed or Voluntary
}
```

These events provide a complete audit trail. The event logs can be indexed and monitored via the PropChain indexer.

---

## Governance Override

The admin has the ability to:
- Execute emergency slashing via `slash_validator()`
- Reactivate a validator via `reactivate_validator()`
- The parameter governance system (`propose_param_change`) can adjust slashing parameters through community voting

## Related Documentation

- [Staking Contract Architecture](../contracts/staking/README.md)
- [Oracle Slashing Mechanism](../contracts/oracle/RECOVERY.md)
- [Security Audit Report](../security-audit/SECURITY_AUDIT.md)
- [Error Codes Reference](ERROR_CODES.md)
