# volume-fee-tiers

Tiered fees by trading volume over a rolling ~24 hour window. Closes #1444.

Default schedule: 0–9 999 → 30 bps, 10 000–49 999 → 25, 50 000–99 999 → 20, 100 000+ → 15 (lower bound inclusive).
`register_volume_tracker`, `record_volume` (admin or authorised recorder, e.g. the DEX), `get_fee_rate`, `set_fee_tiers`,
plus `set_window_blocks` and `set_recorder`. Events: `VolumeRecorded`, `FeeTierChanged`, `FeeTierUpdated`.

The rolling window uses two buckets per account (current + previous) and linear decay, so storage is O(1) per account;
it is an **estimate** of a true rolling sum, documented in `src/lib.rs`. This contract does not call the `fees` contract
or the DEX on-chain; the DEX calls `record_volume`.

```bash
cargo test --manifest-path contract-extensions/contracts/volume-fee-tiers/Cargo.toml
```
