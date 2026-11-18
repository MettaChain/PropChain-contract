# reentrancy-scanner

Source-level scan for reentrancy risks in ink! contracts (no compilation of the targets). Closes #1466.

```bash
cargo run --manifest-path contract-extensions/tools/reentrancy-scanner/Cargo.toml -- \
  [--config FILE] [--format json|text] PATH...
```

| Severity | Meaning |
|---|---|
| Critical | state written **after** an external call, no guard in the function |
| Warning | external call, no guard, no later write found |
| Info | external call in a function that uses a guard (`non_reentrant!`, `guard.enter()`, ...) |

External calls: `build_call`, `ink::env::call::*`, `.invoke()`, `.try_invoke()`, `.call_v1()`, `.delegate_call()`,
`self.env().transfer(..)`, plus `external = NAME` from the config. Guards: `non_reentrant!`, `.enter()`/`.exit()` on a
receiver named like a guard/lock, `ensure_not_reentrant`, plus `guard = NAME`. Exit code 1 if any Critical finding.

Calls through generated contract refs have no syntactic marker; list them with `external = NAME`. Statements inside
macros are opaque to `syn`, so `non_reentrant!` blocks are only searched textually for call names.
