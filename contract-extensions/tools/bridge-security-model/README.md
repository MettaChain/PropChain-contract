# bridge-security-model

Economic model of cross-chain bridge security: validator incentives, collusion
cost, attack profitability and compromised-validator stress tests. Closes #1519.

The library is dependency-free and standalone — it is not a member of the root
workspace, so it builds without compiling the ink! contracts.

```bash
cargo test  --manifest-path contract-extensions/tools/bridge-security-model/Cargo.toml
cargo clippy --manifest-path contract-extensions/tools/bridge-security-model/Cargo.toml --all-targets -- -D warnings
```

## Usage

```rust
use bridge_security_model::{model_bridge_security, compare_threshold_schemes};

// 7 validators, 5 signatures required, 10M of stake each.
let report = model_bridge_security(7, 5, 10_000_000.0);

println!("collusion cost: {}", report.collusion.collusion_cost);        // 50M
println!("safe value at risk: {}", report.attack.break_even_value_at_risk);
println!("attack profitable: {}", report.attack.profitable);
println!("recommended set: {:?}", report.recommendation.recommended_validator_count);

// 2-of-3 vs 3-of-5 on identical economics.
for row in compare_threshold_schemes(&report.parameters) {
    println!("{}: {} signatures, collusion cost {}", row.label, row.required_validators, row.collusion_cost);
}
```

For a real deployment, supply the inputs the three-argument entry point cannot
carry:

```rust
use bridge_security_model::{model_bridge_security_with, BridgeParameters, TransferVolume};

let params = BridgeParameters {
    // 250M can leave in a single fraudulent approval.
    volume: TransferVolume {
        daily_transfer_volume: 40_000_000.0,
        average_transfer: 20_000.0,
        max_transfer: 250_000_000.0,
        bridge_tvl: 400_000_000.0,
    },
    detection_probability: 0.45,
    ..BridgeParameters::reference(11, 8, 25_000_000.0)
};
let report = model_bridge_security_with(&params);
```

## What is modelled

| Quantity | Formula |
| --- | --- |
| Collusion cost | `threshold × stake_per_validator` |
| Slash penalty | `collusion_cost × slash_fraction` |
| Rewards forgone | `collusion_cost × reward_apr × horizon_years` |
| Value at risk | `min(max_transfer, bridge_tvl)` |
| Expected attack profit | `(1 − d)·value_at_risk − d·slash_penalty − rewards_forgone` |
| Break-even value at risk | `(rewards_forgone + d·slash_penalty) / (1 − d)` |
| Break-even detection | `(value_at_risk − rewards_forgone) / (value_at_risk + slash_penalty)` |

`d` is the probability the colluding quorum is detected before it withdraws. An
attack is reported as profitable only when the expected profit is strictly
positive; break-even is treated as unprofitable.

Derived outputs:

- **`attack.break_even_value_at_risk`** — the bridge's security budget, i.e. the
  largest value at risk that stays unprofitable to attack.
- **`safety_margin`** — that budget divided by the modelled value at risk. Above
  1 means headroom; below 1 means the modelled attack pays.
- **`incentives`** — honest return (`stake × apr × years`) against a colluder's
  expected return, plus whether honesty is at least as profitable
  (incentive-compatible).
- **`stress`** — outcome if the threshold validators are compromised: breach,
  honest remainder, additional colluders needed, and the slash that would be
  taken.
- **`recommendation`** — smallest validator set (two-thirds threshold) at which
  the attack is unprofitable, the per-validator stake that would deter it at the
  current threshold, and whether the recommendation clears the attack.

`model_bridge_security` uses a labelled **reference scenario** for the inputs its
three-argument signature cannot carry: 10M value at risk, 10 % reward APR, 30 %
detection, one-year horizon. These are named constants (`REFERENCE_*`), not
measurements of a live bridge. Use `BridgeParameters` +
`model_bridge_security_with` for an actual deployment.

## Assumptions and limits

- Quantities are `f64` in one unit of account; there is no FX modelling.
- Rewards and slashing are linear in stake. This mirrors the contracts: the
  bridge slashes `SLASH_PERCENT` (20 %) of current stake
  (`contracts/bridge/src/validator_staking.rs`) and requires
  `MIN_VALIDATOR_STAKE` (10M) for a validator to be active.
- `expected_net_profit` is a one-shot, risk-neutral expectation. Repeated
  attempts, governance capture, key compromise without slashing and MEV are out
  of scope.
- Invalid inputs (zero validators, threshold outside the set, negative or
  non-finite amounts, ratios outside `0..=1`) are clamped before modelling and
  every correction is listed in `SecurityResult::warnings`.

## Tests

21 unit tests and two doctests pin threshold rounding, collusion and slashing
arithmetic, attack profitability and both break-even points, stress behaviour,
the 2-of-3 / 3-of-5 comparison, the recommendation and input normalisation.
