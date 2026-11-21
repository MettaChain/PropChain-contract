# protocol-revenue-model

Projects protocol fee revenue, fee distribution and treasury growth under volume
scenarios. Closes #1520.

The library is dependency-free and standalone — it is not a member of the root
workspace, so it builds without compiling the ink! contracts.

```bash
cargo test  --manifest-path contract-extensions/tools/protocol-revenue-model/Cargo.toml
cargo clippy --manifest-path contract-extensions/tools/protocol-revenue-model/Cargo.toml --all-targets -- -D warnings
```

## Usage

```rust
use protocol_revenue_model::project_fee_revenue;

// 1M/day of volume at 30 bps, flat, for 30 days.
let projection = project_fee_revenue(1_000_000.0, 30.0, 0.0, 30);

assert_eq!(projection.total_fee_revenue, 90_000.0);
assert_eq!(projection.daily_fee_revenue.len(), 30);
```

For a real fee mix, build a `RevenueModel` with several `FeeStream`s, a
`FeeDistribution` and a `TreasuryPolicy`:

```rust
use protocol_revenue_model::{RevenueModel, FeeStream, FeeDistribution, TreasuryPolicy};

let model = RevenueModel::new(
    vec![
        FeeStream::dex_swaps(1_000_000.0, 30.0, 0.0),
        FeeStream::lending_interest(500_000.0, 50.0, 0.0),
    ],
    FeeDistribution::default(),
    TreasuryPolicy::default(),
    30,
);
let projection = model.project();
```

## What is modelled

- Volume grows by a daily compounded rate `g`: `volume(day) = daily_volume * (1 + g)^day`.
- Revenue: `revenue(day) = volume(day) * fee_bps / 10_000`.
- Fees are split by basis points between treasury, liquidity providers, stakers
  and a burn.
- Treasury: `balance(day) = balance(day - 1) + balance(day - 1) * reserve_apr / 365`
  (when positive) + `revenue(day) * treasury_share` - `operating_cost_per_day`.
- Volume scenarios: conservative (flat), moderate (0.2 %/day), aggressive
  (0.5 %/day).
- Runway: linear extrapolation from a constant monthly burn and revenue, plus
  the day the treasury first goes negative under the projected path.
- Fee adjustment: `new_volume = base_volume * (new_fee / base_fee)^(-elasticity)`.
- Comparison with published DeFi protocol revenue benchmarks.

Inputs are normalised: negative or non-finite amounts become zero, fee rates are
clamped to `0..=10 000` bps, and a distribution that does not sum to 10 000 bps
is rescaled. Every correction is recorded in `RevenueProjection::warnings`.

## Tests

21 unit tests and one doctest cover flat and growing revenue, monthly buckets,
zero-day projections, distribution rescaling, treasury compounding and
depletion, runway arithmetic, fee adjustment and elasticity, volume scenarios,
multi-stream totals and input normalisation.
