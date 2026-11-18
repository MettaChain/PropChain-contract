# rebalancing-pools

A share pool that keeps target allocation weights across property types and rebalances automatically on every recorded
buy/sell. Closes #1440. Rebalancing is skipped during a cooldown, when reported slippage is above the limit, or when the
deviation is within a tolerance band; `get_deviation()` reports how far the pool is from target.

Holdings are value amounts; the contract produces the sell/buy order list (`get_last_orders`) but does not call a DEX, and
relies on an operator to report observed slippage. See the module docs in `src/lib.rs`.

```bash
cargo test --manifest-path contract-extensions/contracts/rebalancing-pools/Cargo.toml
```
