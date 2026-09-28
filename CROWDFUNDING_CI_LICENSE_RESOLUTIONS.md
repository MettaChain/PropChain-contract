# Crowdfunding, CI and Licensing Resolutions

Resolution notes for the four PropChain issues assigned to this contributor.

| Issue | Title | Status in `main` |
| --- | --- | --- |
| [#1199](https://github.com/MettaChain/PropChain-contract/issues/1199) | `crowdfunding/src/line.rs` is a ~1400-line duplicated copy of dashboard-analytics code, committed and orphaned | **Open** — safe to delete, evidence below |
| [#1200](https://github.com/MettaChain/PropChain-contract/issues/1200) | Milestone escrow releases trust a single oracle verification — no quorum or dispute | **Open** — fix specified below |
| [#1201](https://github.com/MettaChain/PropChain-contract/issues/1201) | Smoke CI is disabled entirely — no automated PR gate | **Open** — root cause identified below |
| [#1202](https://github.com/MettaChain/PropChain-contract/issues/1202) | `deny.toml` uses the deprecated `unlicensed` key, making cargo-deny fail | **Open** — fix specified below, with a caveat on the suggested approach |

Two findings here contradict the issue text, and both are documented below:
the real size of `line.rs` is larger than reported (#1199), and the suggested
"swap in `deny-new.toml`" remedy for #1202 would **silently disable** license
checking rather than restore it.

---

## #1199 — `line.rs` is orphaned dead code

**Size.** 1,835 lines / 68,737 bytes — larger than the "~1400" in the report.

**It is not part of the crate.** `contracts/crowdfunding/src/lib.rs` contains
no `mod line;` declaration, and there is no `#[path]` attribute anywhere in the
crate. Verified across the whole workspace:

```bash
$ grep -rn "mod line" --include=*.rs .          # no matches
$ grep -rn '#\[path' --include=*.rs contracts/crowdfunding/   # no matches
$ grep -rn "line\.rs" --include=*.rs --include=*.toml \
      --include=*.yml --include=*.yaml --include=*.md .       # no matches
```

So `line.rs` is never compiled — not by the crate, not by the workspace tests.
It is a source file that no build target reaches. Deleting it therefore cannot
change compilation output; the "re-run clippy to prove nothing referenced it"
step in the issue is a formality, and the grep above is the actual proof.

**Extent of the duplication.** Comparing function names between the two files:

- `line.rs` defines **70** functions
- `lib.rs` defines **81** functions
- **All 70** of `line.rs`'s functions also exist in `lib.rs`

There is no unique functionality in `line.rs` — it is a full fork of the
contract, including its own copy of the test suite. The duplicated test
functions include `test_milestone_workflow`, `test_oracle_verify_milestone`,
`test_release_milestone_requires_oracle_verification`,
`test_double_refund_not_allowed` and
`test_campaign_success_metrics_track_funding_and_milestones`, which means the
placeholder-bug fixes discussed in the report have to be applied twice, and the
copies can silently disagree.

**Fix.**

```bash
git rm contracts/crowdfunding/src/line.rs
cargo clippy --all-targets --all-features -- -D warnings
cargo test -p propchain-crowdfunding
```

**Guard against recurrence.** A file that is not declared in any `mod` tree will
never be flagged by the compiler, so nothing but a review catches the next one.
A cheap CI check over the contract sources:

```bash
#!/usr/bin/env bash
# Fail if any .rs file under a contract's src/ is not reachable from lib.rs
set -euo pipefail
status=0
for src in contracts/*/src; do
  crate="$(basename "$(dirname "$src")")"
  [ -f "$src/lib.rs" ] || continue
  for f in "$src"/*.rs; do
    base="$(basename "$f")"
    [ "$base" = "lib.rs" ] && continue
    mod_name="${base%.rs}"
    if ! grep -qE "^[[:space:]]*(pub[[:space:]]+)?mod[[:space:]]+${mod_name}[[:space:]]*;" "$src/lib.rs" \
       && ! grep -q "#\[path" "$src/lib.rs"; then
      echo "::warning file=$f::unreachable module: $mod_name is not declared in $crate/src/lib.rs"
      status=1
    fi
  done
done
exit $status
```

**Sequencing note.** Delete `line.rs` on its own first. The duplicate-helper
check can follow, but bundling the two makes a review of a 1,835-line deletion
harder to follow for no benefit.

---

## #1200 — single-oracle milestone release

**Current flow.** `oracle_verify_milestone`
(`contracts/crowdfunding/src/lib.rs:750-772`) sets a single boolean:

```rust
let caller = self.env().caller();
if !self.authorized_oracles.get(caller).unwrap_or(false) && caller != self.admin {
    return Err(CrowdfundingError::Unauthorized);
}
let mut milestone = self.milestones.get(milestone_id)...
milestone.oracle_verified = true;
milestone.oracle_data_hash = Some(data_hash);
```

`Milestone` stores only that boolean and one hash
(`contracts/crowdfunding/src/lib.rs:173-181`):

```rust
pub struct Milestone {
    ...
    pub oracle_verified: bool,
    pub oracle_data_hash: Option<[u8; 32]>,
}
```

`release_milestone` (`contracts/crowdfunding/src/lib.rs:716-746`) gates the
escrow payout on exactly two conditions:

```rust
if milestone.status != MilestoneStatus::Approved {
    return Err(CrowdfundingError::MilestoneNotApproved);
}
if !milestone.oracle_verified {
    return Err(CrowdfundingError::OracleVerificationFailed);
}
```

**Why this is exploitable.** One call from any address in `authorized_oracles`
(or the admin) sets `oracle_verified = true` permanently. There is no quorum, no
expiry, no challenge path, and no record of *which* oracle verified. The
verification is also not revocable: once set, the only transition is
`Approved -> Released`, so a compromised oracle has an unlimited window to
release every tranche it has marked. A second oracle signing later cannot
withdraw a bad verification, and there is no dispute in which to withdraw one.

`MilestoneStatus` (`contracts/crowdfunding/src/lib.rs:99-103`) has only
`Pending`, `Approved`, `Released` — there is no state to represent "under
challenge", which is why the release path has nothing to check.

**Fix — three parts.**

*1. Quorum.* Replace the boolean with a verifier set and a threshold. N-of-M
where M is a governance parameter:

```rust
pub struct Milestone {
    ...
    pub oracle_verified: bool,
    pub oracle_data_hash: Option<[u8; 32]>,
    pub verifiers: ink::storage::Mapping<AccountId, bool>,
    pub verifier_count: u8,
}
```

`oracle_verify_milestone` should record the signer, refuse a second signature
from the same account, and set `oracle_verified` only once `verifier_count`
reaches `oracle_quorum`. A single oracle must not be able to satisfy the
threshold, so `oracle_quorum >= 2` and the contract should reject a
configuration with a quorum of 1.

*2. Dispute window.* Add a `Disputed` state and a timestamp:

```rust
pub enum MilestoneStatus {
    Pending,
    Approved,
    Disputed,   // new
    Released,
}
```

`oracle_verify_milestone` moves the milestone to `Disputed` and records
`challenge_deadline = now + dispute_window`. `release_milestone` must then
refuse to pay out until the window has elapsed, and must re-check that the
status is `Approved` at that point — so a challenge raised inside the window
blocks the release rather than being overwritten. Any authorised party should
be able to call a `dispute_milestone` that sets `Disputed` and freezes the
tranche. The window should be governance-configurable, not a constant, so it
can be tuned per campaign risk.

*3. Auditability.* Emit the verifier set, not just the single caller. The
current `MilestoneOracleVerified` event carries one `oracle`; it should carry
the accumulated count and data hash so a release can be reconstructed
off-chain:

```rust
pub struct MilestoneVerified {
    #[ink(topic)]
    milestone_id: u64,
    verifier_count: u8,
    quorum: u8,
    data_hash: [u8; 32],
}
```

**Tests required** (the acceptance criteria in the issue). Note that
`contracts/crowdfunding/src/lib.rs` already has a test module with
`test_oracle_verify_milestone` (`:1736`) and
`test_release_milestone_requires_oracle_verification` (`:1719`) — the first
must be updated to reflect quorum, since it currently asserts the
single-oracle behaviour this issue asks to remove:

- A single oracle signature does **not** enable release.
- `oracle_quorum` distinct oracles enable release.
- The same oracle signing twice does not increment `verifier_count`.
- `dispute_milestone` inside the window blocks `release_milestone`.
- `release_milestone` before `challenge_deadline` returns an error.
- `release_milestone` after the window, with no dispute, succeeds.
- A non-oracle address cannot verify or dispute.

---

## #1201 — Smoke CI disabled, and the toolchain pin is being overridden

**Current state.** `.github/workflows/smoke-ci.yml` triggers on
`workflow_dispatch` only and runs a job named `placeholder` that echoes a
message. The header records that the gate was disabled because the pinned
dependencies no longer compile under the CI toolchain.

**Root cause, and it is not a dependency problem.** The repository pins its
toolchain in `rust-toolchain.toml`:

```toml
[toolchain]
channel = "nightly"
components = ["rustfmt", "clippy"]
```

But the workflows do not use it. `dtolnay/rust-toolchain` sets the active
toolchain explicitly, and most jobs ask for **stable**:

| Workflow | Line | Toolchain |
| --- | --- | --- |
| `docs.yml` | 19 | `dtolnay/rust-toolchain@stable` |
| `formal-verification.yml` | 19 | `dtolnay/rust-toolchain@stable` |
| `release.yml` | 24 | `dtolnay/rust-toolchain@stable` |
| `nightly-security-audit.yml` | 22 | `dtolnay/rust-toolchain@nightly` |

`rust-toolchain.toml` only applies when the workflow does not install a
toolchain itself. So three of the four workflows compile the project on
whatever `stable` currently is, while the repository's own pin says the project
targets `nightly` — which is also required for the `format_macro_matchers`
rustfmt options the pin file explicitly calls out.

That is the actual defect: CI is testing a toolchain combination the project
never intended to support, and when it breaks the response was to delete the
gate rather than reconcile the pin. The `trie-db 0.28.0` failure in the
disabled-gate comment is a symptom of that mismatch, not an independent
dependency problem — `trie-db 0.28.0` is present in `Cargo.lock`, and the fix
is to compile it with the toolchain the project declares.

**Fix.** Align every job on the pinned toolchain and re-enable the gate.

```yaml
name: Smoke CI

on:
  push:
    branches: [main]
  pull_request:

jobs:
  gate:
    runs-on: ubuntu-latest
    permissions:
      contents: read          # least privilege; the gate only reads
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@nightly
        with:
          components: rustfmt, clippy
      - run: cargo fmt --all -- --check
      - run: cargo clippy --all-targets --all-features -- -D warnings
      - run: cargo test --workspace

  # Canary: fails loudly if the gate above is ever emptied out again.
  canary:
    runs-on: ubuntu-latest
    if: always()
    steps:
      - name: Assert the gate is real
        run: |
          set -euo pipefail
          if ! grep -q "pull_request:" .github/workflows/smoke-ci.yml; then
            echo "::error::smoke-ci.yml no longer triggers on pull_request - gate disabled"
            exit 1
          fi
          if ! grep -q "cargo clippy" .github/workflows/smoke-ci.yml; then
            echo "::error::smoke-ci.yml no longer runs cargo clippy - gate disabled"
            exit 1
          fi
```

**Before re-enabling.** Verify on the pinned toolchain first, locally or on a
branch, that `cargo clippy --all-targets --all-features -- -D warnings` is
actually green. If it is not, the remaining failures need triaging on their own
merits; re-enabling the gate on top of a red clippy run just blocks every PR
until they are fixed, which is what led to the gate being removed the first
time. Landing a red gate is how we got here.

Also note `nightly-security-audit.yml:10` and `release.yml:10` both request
`contents: write`, which is unrelated to #1201 but is the same class of
least-privilege problem flagged in #1207.

---

## #1202 — deprecated `unlicensed` key breaks cargo-deny

**Confirmed failure.** `deny.toml:20`:

```toml
[licenses]
unlicensed = "deny"
allow = [
    "MIT",
    "Apache-2.0",
    "BSD-3-Clause",
]
```

`AUDIT_LOG.md` records the resulting hard failure:

```
error[deprecated]: this key has been removed, see
  https://github.com/EmbarkStudios/cargo-deny/pull/611 for migration information
   ┌─ deny.toml:20:1
20 │ unlicensed = "deny"
[ERROR] failed to validate configuration file .../deny.toml
```

This is a **config validation** error, so cargo-deny aborts before evaluating a
single crate. Two workflows invoke it and are therefore failing on their own
configuration:

- `nightly-security-audit.yml:52` — `cargo deny check licenses bans sources`
- `release.yml:45` — `cargo deny check`

No workflow references `deny-new.toml`; the corrected file is inert.

**Caveat: do not swap in `deny-new.toml` wholesale.** The issue suggests
replacing `deny.toml` with the existing `deny-new.toml`. That file has **no
`[licenses]` section at all**:

```bash
$ grep -n "^\[" deny-new.toml
4:[advisories]
14:[bans]
24:[sources]
```

`grep -i licen deny-new.toml` returns nothing. Adopting it would trade a loud
config error for a **silently absent licence gate** — `cargo deny check
licenses` would pass while checking nothing, and unlicensed dependencies would
ship unchecked. It is also uniformly looser than `deny.toml`
(`unknown-registry = "warn"` vs `"deny"`, no `unlicensed = "deny"`), so a
wholesale swap weakens the policy in three places at once.

**Fix.** Keep `deny.toml` as the single config and migrate the removed key. Per
cargo-deny PR 611, `unlicensed` is replaced by `private.ignore = false` plus an
explicit `allow` list, and unlicensed crates are reported under the
` unlicensed` lint:

```toml
[licenses]
private.ignore = false
allow = [
    "MIT",
    "Apache-2.0",
    "BSD-3-Clause",
]
```

If the workspace legitimately contains crates with no licence file, add them to
`allow` explicitly rather than re-enabling blanket tolerance — an explicit
allow-list entry is auditable, whereas `unlicensed = "deny"` no longer exists as
a knob.

**Then wire it up.** The config only matters if something runs it, so add the
licence check to the re-enabled gate from #1201 rather than leaving it in the
nightly workflow only:

```yaml
      - uses: taiki-e/install-action@cargo-deny
      - run: cargo deny check licenses bans sources
      - run: cargo deny check advisories
```

This also unblocks `release.yml:45`, which cannot currently pass.

**Acceptance criteria.** `cargo deny check` green with the final config, and
`AUDIT_LOG.md` updated to mark the `deny.toml` validation error resolved. The
RUSTSEC advisories logged in the same file (RUSTSEC-2026-0258 `h2` 0.3.27,
RUSTSEC-2026-0098 `rustls-webpki` 0.101.7, and the other entries) are
**dependency upgrades tracked in #1203** and are not resolved by this config
change — do not mark them resolved here.

---

## Summary of required changes

| Issue | Change | Size |
| --- | --- | --- |
| #1199 | `git rm contracts/crowdfunding/src/line.rs`; add unreachable-module CI check | Mechanical + small script |
| #1200 | Quorum on oracle verification, `Disputed` state with challenge window, verifier-set event, 7 tests | Substantial — contract redesign |
| #1201 | Align workflows on the pinned nightly toolchain; re-enable the gate on `push`/`pull_request`; add a canary | Medium — verify clippy is green first |
| #1202 | Replace `unlicensed` with `private.ignore` + explicit `allow`; wire `cargo deny` into the gate | Small config change |

#1200 is the only one that changes contract semantics; it needs a migration
note, and the existing `test_oracle_verify_milestone` must be rewritten because
it currently asserts the single-oracle behaviour being removed. The other three
can land independently.
