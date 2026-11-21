# PropChain economic simulators

This dependency-free Rust library contains four seeded, standalone economic
models corresponding to issues #1515-#1518. It lives under
`contract-extensions/tools/` and does not change or execute contract code.

## Modules

- `governance`: Monte Carlo participation, support, lock-weighted voting power,
  direct delegation, quorum sensitivity, and single-voter concentration.
- `insurance`: frequency/severity claim scenarios, expected-loss premiums,
  reserve recommendations, all-policy stress losses, and a comparator for the
  contract's documented dynamic-premium arithmetic.
- `campaign`: early/late contribution patterns, goal momentum, aggregate
  withdrawal events, milestone releases, goal/deadline sensitivity, and
  comparison against caller-supplied historical campaign observations.
- `prediction_market`: two-outcome LMSR pricing, simulated informed/uninformed
  trade flow, price impact/depth, expected liquidity-provider returns, and
  comparisons across `b` values.

Monte Carlo entry points expose explicit iteration counts and seeds for
reproducible analysis. Convenience functions use fixed defaults. Probability
fields are fractions from 0 to 1; input fields suffixed `_bps` use basis points
(10,000 = 100%). These outputs are estimates, not financial advice or guarantees
about on-chain outcomes.

## Run locally

```bash
cargo test --manifest-path contract-extensions/tools/economic-simulators/Cargo.toml
```
