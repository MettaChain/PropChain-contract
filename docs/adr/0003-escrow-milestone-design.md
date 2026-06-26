# ADR 3: Escrow Milestone-Based Release Design

## Status

Accepted

## Context

PropChain escrows hold funds between a buyer and a seller during a property transaction. A simple all-or-nothing escrow is insufficient for complex transactions that involve staged deliverables—for example, a new development where 30% is paid on signing, 40% on construction completion, and 30% on title transfer.

Requirements:
- Support multiple conditional release tranches tied to verifiable on-chain or off-chain milestones.
- Allow disputes at any milestone without invalidating the entire escrow.
- Ensure funds cannot be locked permanently if a milestone is never reached.

## Decision

The `Escrow` contract implements **milestone-based releases**:

- An escrow is created with an ordered list of `Milestone` structs. Each milestone specifies: `amount`, `description`, `condition_hash` (hash of the off-chain condition document), and an optional `deadline`.
- Funds are deposited in full upfront but released incrementally as each milestone is approved.
- Milestone approval requires either (a) multi-sig sign-off from both buyer and seller, or (b) an arbitrator signature if a dispute is raised.
- If a milestone deadline passes without approval, either party may trigger a dispute. The contract moves to `DisputeActive` state for that milestone only; other milestones are unaffected.
- A global `TimeLock` on the entire escrow is set at creation. If the escrow is still open when the timelock expires, remaining funds are returned to the depositor.

The `EscrowMilestone` storage entry is a `Vec` capped at 20 milestones per escrow to bound gas usage.

## Consequences

**Positive:**
- Supports real-world staged payment structures without requiring multiple separate escrow contracts.
- Disputes are isolated to individual milestones, reducing friction for the unaffected tranches.
- The global timelock prevents funds from being locked indefinitely due to abandoned transactions.

**Negative:**
- More complex state machine than a simple single-release escrow, increasing audit surface.
- The 20-milestone cap may be restrictive for long construction projects; exceeding it requires splitting into multiple escrows.
- Off-chain condition documents must be pinned to IPFS and their CIDs stored as `condition_hash` inputs; there is no on-chain enforcement of the actual conditions.

**Neutral:**
- The `ConditionsNotMet` error (code 2005) is returned when a release is attempted before a milestone is approved. Callers should poll `get_escrow_milestones(escrow_id)` to check approval status.
