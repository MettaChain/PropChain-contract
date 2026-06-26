# ADR 4: Bridge Multi-Signature Authorization

## Status

Accepted

## Context

The PropChain cross-chain bridge moves property token representations between Substrate-based chains. A bridge controlled by a single relayer is a single point of failure and a high-value target: compromise of the relayer key allows arbitrary token minting on the destination chain.

Requirements:
- No single party should be able to authorize a bridge transfer unilaterally.
- The system must remain operational even if some operators are temporarily offline.
- Rogue or compromised operators must not be able to forge transfers even if they collude in small numbers.

## Decision

The bridge uses a **k-of-n threshold multi-signature** scheme:

- A set of `n` authorized bridge operators is maintained on-chain. Adding or removing operators requires a governance proposal.
- Each bridge request must collect signatures from at least `k` distinct operators before `execute_bridge` can be called. The default threshold is `k = ceil(2n/3)` (two-thirds supermajority).
- Operators sign the canonical request hash `H(request_id ‖ token_id ‖ source_chain ‖ dest_chain ‖ recipient ‖ nonce)` off-chain and submit their signatures on-chain via `sign_bridge_request`.
- The contract verifies each signature against the registered operator public key using `CryptoError::InvalidSignature` / `InvalidPublicKey` guards.
- Bridge requests expire after a configurable TTL (default: 100 blocks). Expired requests cannot be executed; tokens are unlocked via `recover_failed_bridge`.
- A `RateLimitExceeded` guard (code 3013) caps the number of bridge operations per rolling 24-hour window per token to prevent flooding.

## Consequences

**Positive:**
- Compromise of fewer than `k` operators cannot produce a valid bridge execution.
- Liveness is maintained as long as at least `k` operators are online; no single operator is a bottleneck.
- The TTL and recovery mechanism ensure tokens are never permanently locked by a stalled bridge request.

**Negative:**
- If more than `n - k` operators are simultaneously offline, the bridge halts until enough come back online.
- Adding or removing operators requires a governance cycle, making rapid operator rotation slow.
- Multi-signature collection happens off-chain (operators communicate via a relayer network), introducing an off-chain coordination layer that is not directly observable on-chain until signatures are submitted.

**Neutral:**
- `InsufficientSignatures` (code 1010 / 3005) is the primary error callers will see when the threshold has not been reached. Clients should poll `monitor_bridge_status(request_id)` and display a "waiting for operator consensus" state to users.
