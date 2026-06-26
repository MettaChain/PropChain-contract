# ADR 2: Oracle Commit-Reveal Design

## Status

Accepted

## Context

The PropChain oracle aggregates property valuations from multiple off-chain sources. A naive design where operators submit values directly in a single transaction is vulnerable to **last-look manipulation**: later operators can observe earlier submissions and bias the aggregate toward a desired outcome. This is especially problematic in a real-estate context where valuations directly determine collateral ratios in the lending contract and trigger insurance pay-outs.

Requirements:
- Prevent oracle operators from copying or adjusting submissions based on others' values.
- Produce a single aggregated valuation that is resistant to outlier manipulation.
- Tolerate partial operator participation (some operators may be offline).

## Decision

We adopt a **two-phase commit-reveal scheme**:

1. **Commit phase** — Each operator submits `H(value ‖ nonce)` (a hash commitment) within a configurable window (default: 10 blocks). No value is visible on-chain during this phase.
2. **Reveal phase** — After the commit window closes, operators submit their plaintext `(value, nonce)`. The contract verifies each reveal against the stored commitment.
3. **Aggregation** — Once the reveal window closes (default: 10 blocks) or the minimum reveal threshold is met, the contract discards the top and bottom outliers (trimmed mean) and publishes the aggregated valuation.

Operators who commit but do not reveal within the reveal window are penalized via a reputation score reduction. Operators who reveal without a prior commit are rejected.

## Consequences

**Positive:**
- Eliminates last-look bias; no operator can see others' values before committing.
- Trimmed-mean aggregation reduces the impact of compromised or misconfigured oracle sources.
- On-chain verifiability: any observer can confirm that the published value matches the revealed inputs.

**Negative:**
- Adds latency: a full valuation cycle takes at least two block windows (~20 blocks) rather than one.
- Operators must manage nonce state and ensure they reveal in time or face reputation penalties.
- Increased gas cost: two transactions per operator per valuation round instead of one.

**Neutral:**
- The `InsufficientReveals` error (`CryptoError`) is returned if fewer than the minimum number of operators reveal, requiring a new round to be initiated.
