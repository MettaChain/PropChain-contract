# access-control-audit

Reports which `#[ink(message)]` functions have visible access control. Closes #1468.

```bash
cargo run --manifest-path contract-extensions/tools/access-control-audit/Cargo.toml -- \
  [--config FILE] [--json] [--include-views] [--fail-on-unprotected] PATH...
```

Statuses: `Protected` (calls a guard such as `ensure_owner` / `only_admin` / `require_role` / `has_role`,
or reads `caller()` alongside an owner/admin/role-style identifier), `IntentionallyPublic` (a `// public`
comment above the message or after its signature), `Whitelisted` (config `allow = message` or
`allow = Contract::message`), `ReadOnly` (`&self`, excluded from coverage unless `--include-views`),
`Unprotected`. Extra guard names: `guard = NAME` in the config. Output has per-contract and per-module
summaries plus an overall coverage percentage; `--json` is machine-readable.

This is a syntactic heuristic: it shows that a guard is present, not that it is correct.
