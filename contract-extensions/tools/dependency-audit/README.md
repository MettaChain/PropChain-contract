# dependency-audit

Audits every `Cargo.toml` under a folder (issue #1472): known vulnerabilities, license
compliance, version pinning and duplicate versions.

```bash
cargo run --release -- --root ../.. --run-audit --run-metadata
# or from saved reports
cargo audit --json > audit.json && cargo metadata --format-version 1 > metadata.json
cargo run --release -- --root ../.. --audit audit.json --metadata metadata.json --format json
```

Run it on the extensions folder with `--root contract-extensions` from the repository root.

| Check | Source | Fails the run |
|---|---|---|
| vulnerable dependency | `cargo audit --json` | yes |
| license not allowed / missing | `cargo metadata` | yes |
| wildcard (`*`, `1.*`) or unpinned (`1`, `1.2`, `>=1`) requirement, git dep without `rev`/`tag` | manifests | no (reported) |
| different requirements for one crate across manifests | manifests | no (reported) |

Allowed licenses default to the `deny.toml` policy (MIT, Apache-2.0, BSD-3-Clause) plus other
permissive licenses (see `DEFAULT_ALLOWED_LICENSES`); add more with `--allow-license`.
A check whose data was not supplied is printed as `NOT CHECKED`, never as a pass.

Exit codes: `0` clean, `1` vulnerability or license issue, `2` usage, I/O or parse error.
