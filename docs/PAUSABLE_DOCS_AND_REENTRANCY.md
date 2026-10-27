# Pausable: documentation and reentrancy review

Pausable in this repository is the pause logic of the `PropertyRegistry` ink! contract (`contracts/lib/src/lib.rs`): `pause_contract`, `emergency_pause`, `request_resume`, `approve_resume`, `ensure_not_paused` and `get_pause_state`. There is no separate Solidity `Pausable` contract.

## Documentation (#1298)

| Message | Caller | Effect |
|---------|--------|--------|
| `pause_contract(reason, duration)` | admin or pause guardian | Pauses; optional auto-resume after 60 s .. 30 days |
| `emergency_pause(reason)` | admin or pause guardian | Logs an emergency audit event, then pauses with no auto-resume |
| `request_resume()` | admin or pause guardian | Opens a resume request |
| `approve_resume()` | pause guardian | Adds an approval; resumes once the required number is reached |
| `get_pause_state()` | anyone | Returns the current `PauseInfo` |
| `ensure_not_paused()` | anyone | `Err(ContractPaused)` while paused (honours auto-resume) |

## Reentrancy review (#1299)

The pause and resume messages only read the caller, write `pause_info`, emit events and append audit entries. They make no cross-contract calls and transfer no value, so there is no external call through which the contract could be re-entered mid-update. State is written before events are emitted.

## Tests (#1296, #1297)

`contracts/lib/tests/pausable_unit_fuzz.rs` holds unit tests plus a sweep over many unprivileged callers.
# Pausable: documentation and reentrancy review

Pausable in this repository is the pause logic of the `PropertyRegistry` ink! contract (`contracts/lib/src/lib.rs`): `pause_contract`, `emergency_pause`, `request_resume`, `approve_resume`, `ensure_not_paused` and `get_pause_state`. There is no separate Solidity `Pausable` contract.

## Documentation (#1298)

| Message | Caller | Effect |
|---------|--------|--------|
| `pause_contract(reason, duration)` | admin or pause guardian | Pauses; optional auto-resume after 60 s .. 30 days |
| `emergency_pause(reason)` | admin or pause guardian | Logs an emergency audit event, then pauses with no auto-resume |
| `request_resume()` | admin or pause guardian | Opens a resume request |
| `approve_resume()` | pause guardian | Adds an approval; resumes once the required number is reached |
| `get_pause_state()` | anyone | Returns the current `PauseInfo` |
| `ensure_not_paused()` | anyone | `Err(ContractPaused)` while paused (honours auto-resume) |

## Reentrancy review (#1299)

The pause and resume messages only read the caller, write `pause_info`, emit events and append audit entries. They make no cross-contract calls and transfer no value, so there is no external call through which the contract could be re-entered mid-update. State is written before events are emitted.

## Tests (#1296, #1297)

`contracts/lib/tests/pausable_unit_fuzz.rs` holds unit tests plus a sweep over many unprivileged callers.
