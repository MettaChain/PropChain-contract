# concentrated-liquidity-dex

Tick-range liquidity positions (0.1% ticks), swaps routed only through ranges that contain the current tick, and
per-position fee accounting. Closes #1435.

This is a **simplified** model, documented in `src/lib.rs`: swaps execute at the current tick's price with no price impact or
tick crossing (the admin moves the tick), balances are internal accounting (no token contract calls), and positions are
non-fungible ids owned by an account (not PSP34 tokens).

```bash
cargo test --manifest-path contract-extensions/contracts/concentrated-liquidity-dex/Cargo.toml
```
