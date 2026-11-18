# compliance-transaction-monitor

Real-time transaction monitoring rules (velocity limit, round-trip detection, structuring detection,
high-value flag) with per-sender risk accumulation. Closes #1443.

See the module docs in `src/lib.rs` for the rule semantics. Notes:

- A reporter (admin or `set_reporter` account) calls `record_transaction`; `evaluate_transaction` is the read-only preview.
- Windows are tumbling and stored as one `(window_start, count)` pair per account, not a transaction list.
- Self-contained: it does not call `contracts/compliance_registry` or a sanctions list on-chain.

```bash
cargo test --manifest-path contract-extensions/contracts/compliance-transaction-monitor/Cargo.toml
```
