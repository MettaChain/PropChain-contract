# bridge-message-passing

Arbitrary cross-chain message passing: `send_message` (source chain), validator attestations and threshold
`execute_message` (destination chain), replay-proof ids, a per-message gas limit, and `retry_message` /
`cancel_message` for failures. Closes #1437.

Trust model: a validator's attestation is the call from its account; the `signature` argument is recorded but **not**
cryptographically verified, and the contract cannot prove the message was sent on the source chain. The actual target call
(`dispatch`) needs a real node; unit tests script its outcome.

```bash
cargo test --manifest-path contract-extensions/contracts/bridge-message-passing/Cargo.toml
```
