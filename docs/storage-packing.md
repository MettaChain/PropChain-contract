# Storage Slot Packing (Issue #515)

## Summary

Related bool fields that were stored separately have been packed into single
`u8` bitfield values to reduce storage reads and on-chain encoded size.

## Changes

### `contracts/insurance/src/types.rs` — `PropertyRiskFactors`

**Before:** three separate `bool` fields (3 bytes SCALE-encoded)

```rust
pub has_security_system: bool,   // 1 byte
pub has_fire_extinguisher: bool, // 1 byte
pub has_alarm_system: bool,      // 1 byte
```

**After:** single `u8 safety_flags` bitfield (1 byte)

```rust
pub safety_flags: u8,  // bits: 0=security_system, 1=fire_extinguisher, 2=alarm_system
```

| bit | flag constant                    | meaning                  |
|-----|----------------------------------|--------------------------|
|  0  | `safety_flag::SECURITY_SYSTEM`   | `has_security_system`    |
|  1  | `safety_flag::FIRE_EXTINGUISHER` | `has_fire_extinguisher`  |
|  2  | `safety_flag::ALARM_SYSTEM`      | `has_alarm_system`       |

**Savings:** 2 bytes per stored `PropertyRiskModel` entry. Since every
`PropertyRiskModel` embeds a `PropertyRiskFactors`, this reduces the storage
footprint of every risk model record and cuts the storage read count for
safety-feature lookups from 3 slot reads to 1.

**Accessor methods** on `PropertyRiskFactors` preserve the old bool semantics:

```rust
factors.has_security_system()    // → bool
factors.has_fire_extinguisher()  // → bool
factors.has_alarm_system()       // → bool
PropertyRiskFactors::encode_safety_flags(sec, fire, alarm)  // → u8
```

## Audit

All existing tests pass unchanged. The public ink! message signatures
(`assess_property_risk_comprehensive`, `update_property_risk_assessment`)
continue to accept three individual `bool` parameters — callers are unaffected.
