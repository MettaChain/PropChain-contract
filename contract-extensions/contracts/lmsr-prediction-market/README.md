# lmsr-prediction-market

Prediction market maker using the **logarithmic market scoring rule** (`C(q) = b·ln Σ exp(q_i/b)`) with 2–4 outcomes.
Closes #1442. `b` is derived from the funding (`b = funding / ln n`, so the worst-case loss is the funding), buys are solved
in closed form for a budget, sells pay the cost difference, winners redeem 1 unit per share, and providers withdraw the residual.

`no_std` has no `exp`/`ln`, so both use documented fixed-point approximations (Taylor with range reduction; `atanh` series with
power-of-two normalisation; scale 10^9, ~1e-8 relative error). Liquidity can only be added before the first trade. See `src/lib.rs`.

```bash
cargo test --manifest-path contract-extensions/contracts/lmsr-prediction-market/Cargo.toml
```
