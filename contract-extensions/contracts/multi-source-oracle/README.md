# multi-source-oracle

Registered sources submit prices per asset; the contract aggregates the fresh, non-outlier ones into a median with a
confidence score. Closes #1436. Rules (stale filter, 2σ outlier cut, `confidence = 10_000 − σ/mean`) are in `src/lib.rs`.

Source weights feed the mean / standard deviation behind the confidence score and outlier cut-off; the reported price is the
plain median. The submitting account is the source, so `submit_price` takes no separate source address.

```bash
cargo test --manifest-path contract-extensions/contracts/multi-source-oracle/Cargo.toml
```
