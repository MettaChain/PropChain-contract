# Assigned Issue Resolutions

Analysis of the four issues currently assigned to this contributor, with root causes
confirmed against the source at commit `d4d28a17` (`main`).

| Issue | Subject | Area | Fix |
| --- | --- | --- | --- |
| [#1198](https://github.com/MettaChain/PropChain-contract/issues/1198) | `get_campaign_success_metrics` blends placeholder analytics | `contracts/crowdfunding` | [PR #1241](https://github.com/MettaChain/PropChain-contract/pull/1241) |
| [#1196](https://github.com/MettaChain/PropChain-contract/issues/1196) | `quorum_guard` participation history is unbounded | `contracts/monitoring` | [PR #1241](https://github.com/MettaChain/PropChain-contract/pull/1241) |
| [#1197](https://github.com/MettaChain/PropChain-contract/issues/1197) | Alerts have no delivery hook or retry | `contracts/monitoring` | [PR #1241](https://github.com/MettaChain/PropChain-contract/pull/1241) |
| [#1195](https://github.com/MettaChain/PropChain-contract/issues/1195) | No test pins reported metrics to stored ones | `contracts/analytics` | [PR #1241](https://github.com/MettaChain/PropChain-contract/pull/1241) |

Detailed per-issue write-ups, including the parts deliberately left as follow-ups, are
in [`MONITORING_ANALYTICS_RESOLUTIONS.md`](https://github.com/MettaChain/PropChain-contract/pull/1241)
and [`docs/alert_delivery.md`](https://github.com/MettaChain/PropChain-contract/pull/1241),
both added by PR #1241. This document is the index, and it also records three
corrections to the issues' stated premises that affect how the fixes should be reviewed.

All four are fixed in a single PR because they share a root cause: **a value the code
cannot honestly compute was replaced with a plausible constant.** The instances are in
three different crates, but the failure mode is one design decision repeated.

---

## #1198 — Placeholder analytics in crowdfunding success metrics

### Reported

`get_campaign_success_metrics` computes `investor_retention_rate = 8000 // 80% placeholder`
alongside fabricated jurisdiction and investment-distribution splits, and reports them
through an API downstream dashboards read as measured outcomes.

### Confirmed

`contracts/crowdfunding/src/lib.rs:1425`:

```rust
let investor_retention_rate = 8_000; // 80% placeholder
```

The same function set continues into `get_investor_demographics`
(`contracts/crowdfunding/src/lib.rs:1461-1517`), which hardcodes a US 60 / CA 20 /
EU 10 / Other 10 jurisdiction split, a 30/40/20/10 investment distribution, and derives
`accredited_investors` as `total_investors * 7 / 10` under the comment "Assume 70%"
(`:1481`). `get_funding_timeline` (`:1578-1596`) emits 30 points on a straight
`target_amount / 30` ramp, so the cumulative curve is a line by construction.

Two further findings in the same area:

- **`get_investor_demographics` scans every campaign id** to find the requested one
  rather than reading `campaign_id` directly, then reports figures that are not
  scoped to that campaign's investors. It compounds the placeholder problem with a
  correctness problem.
- **`top_investor_amount` is wired to `max_investment = 0`** (`:1514`), so it is
  structurally always zero regardless of any investment recorded.

### `line.rs` is a 1 835-line orphaned copy of the same code

`contracts/crowdfunding/src/line.rs` is **not declared as a module** in
`contracts/crowdfunding/src/lib.rs`, so it is never compiled — `grep` for `mod line`
across `lib.rs` and `Cargo.toml` returns nothing. It is referenced nowhere in the
repository, and all 45 of its public function names duplicate methods that already
exist in `lib.rs`. Every placeholder above is reproduced in it.

It also carries an unused `ink_e2e` dev-dependency via `contracts/crowdfunding/Cargo.toml`.
There is no `tests/` directory and nothing references one, but `trie-db` 0.28.0 does
not compile on the nightly this repository pins — so `cargo test -p propchain-crowdfunding`
failed before running a single test on `main` as well. The orphaned file was actively
preventing the crate's tests from running at all.

### Resolution

PR #1241 replaces each placeholder with a value derived from recorded state, and makes
absent data return `None` rather than a plausible number. Two quantities become
`Option` because the honest answer is "undefined", not zero: retention over an empty
cohort, and a jurisdiction split when no investor profile is stored. `line.rs` and the
unused dev-dependency are deleted.

**`get_funding_timeline` returning `None` is deliberate, not an oversight.** `investments`
is keyed by `(campaign_id, investor)` and holds a running total with no per-investment
timestamp; `campaign_investors` is an unordered `Vec`, not a chronological log. No
cumulative curve is reconstructible from that state, and producing one anyway would
reproduce the exact fabrication the issue reports. The docs describe the state change
that would make it real.

---

## #1196 — Unbounded participation history in `quorum_guard`

### Reported

`quorum_guard` stores `history: Vec<ProposalParticipation>` with no cap, and "the guard
counts participation by iterating it", so the participation check is O(n) over all time
and storage grows without bound.

### Confirmed, but the premise is wrong in a way that strengthens the fix

`contracts/monitoring/src/quorum_guard.rs` is 39 lines of logic. `record` is:

```rust
pub fn record(&mut self, proposal_id: u64, participation_bps: u32) -> bool {
    let warned = self
        .history
        .last()                                    // O(1)
        .map(|prev| {
            prev.participation_bps >= self.warning_threshold_bps
                && participation_bps < self.warning_threshold_bps
        })
        .unwrap_or(false);
    self.history.push(ProposalParticipation { proposal_id, participation_bps });
    warned
}
```

**There is no iteration over `history`.** The only read is `.last()`, which is O(1), and
the only other accessor is `history_len()`. So the O(n) participation scan the issue
describes does not exist in this code.

That does not make the issue a non-issue — it makes it worse in one respect and better
in another:

- **Worse:** `history` is unbounded and *never read except its last element*. Every
  entry before the final one is pure storage cost with no consumer. The `Vec` is
  accumulating data that nothing will ever query.
- **Better:** because nothing reads it, **capping it costs nothing functionally.** No
  participation behaviour changes. This removes the main risk objection to the fix.

The correct framing is therefore *"unbounded write-only storage"*, not *"unbounded
read"*. A reviewer checking the issue's stated acceptance criterion — "participation
query bounded by a constant after compaction" — would find there is no participation
query to bound. The cap satisfies the intent; the literal criterion is not measurable
against this implementation.

One existing test needs updating: `history_accumulates` asserts
`history_len() == 2` and will need to assert against the cap instead.

### Resolution

PR #1241 bounds `history` to a rolling window of `MONITORING_MAX_QUORUM_HISTORY`,
mirroring the snapshot buffer the crate already uses
(`contracts/traits/src/constants.rs:170`, `MONITORING_MAX_SNAPSHOTS = 100`). A separate
lifetime `total_recorded` counter survives eviction, so the number of proposals observed
is never lost — only per-proposal detail of old ones. `participation_bps` returns `None`
for an evicted proposal rather than a stale value.

---

## #1197 — Alerts are records with no delivery or retry

### Reported

`contracts/monitoring/src/lib.rs` manages alert configs and subscribers and emits
`AlertTriggered` on chain, but there is no delivery mechanism and no retry or batching,
so an operator whose indexer is down during an outage misses the signal.

### Confirmed

`check_and_trigger_alerts` (`contracts/monitoring/src/lib.rs:519`) emits
`AlertTriggered` (`:543`, `:568`) and returns. There is no on-chain alert log at all —
no storage, no retrieval message, no acknowledgement. An indexer that is not polling at
the moment an alert fires has no way to learn it happened except by scanning the full
event history of the contract, which is unbounded work.

So the gap is wider than the issue states: this is not "a record with no delivery", it
is *no* durable record. Anything that wants recent alerts has to go to the chain
indexer, and there is no cursor to resume from.

### Resolution

PR #1241 keeps `AlertTriggered` authoritative — the event remains the source of truth —
and adds a bounded ring buffer as the operational retry surface, capped at
`MONITORING_MAX_ALERT_LOG`. Four additions:

- `get_recent_alerts(since_alert_id, limit)` — gap-free batch read. A cursor **older
  than the window is clamped forward**, so a worker that was offline resumes rather than
  silently receiving nothing and concluding it is up to date. `limit = 0` means
  "everything retained" rather than "nothing", since an unset limit returning empty is
  indistinguishable from a healthy system.
- `alert_payload(alert_id)` — self-contained blob carrying the alert type's stable name,
  a severity ranking, and canonical JSON, so a consumer needs no follow-up calls and no
  SCALE enum decoding.
- `acknowledge_alert(alert_id)` — idempotent, so a crash between POST and ack is safe to
  redeliver.
- `pending_alert_count()` — the retry-set size. A value that is not falling means delivery
  is stuck even while the contract is healthy.

`docs/alert_delivery.md` specifies the delivery contract, including a recommended worker
loop and an explicit list of what the contract does **not** do.

Two bugs in the ring-buffer readers were caught by the eviction tests during this work
and are worth knowing about if the code is extended: the batch cursor treated
`since_alert_id = 0` as "skip alert 0" instead of the "handled nothing" sentinel, and
`pending_alert_count` read without the buffer modulo, silently missing every id at or
above the cap. Both are the failure mode you get from hand-rolled modular arithmetic on
a zero-based cursor, and both produced correct-looking output on small inputs.

---

## #1195 — No consistency test ties reported metrics to stored ones

### Reported

`get_market_metrics` (`contracts/analytics/src/lib.rs:223`) builds `MarketMetrics` from
contract views, while `update_market_metrics` / `batch_update_metrics` allow admin
overrides. Nothing pins the two views together, so overrides and recomputed values can
diverge silently.

### The issue's proposed test is not possible as written

The issue asks for a test asserting that offline-recomputed `MarketMetrics` match the
stored ones. Two independent reasons block it:

1. **`get_market_metrics` returns a single stored value.** `contracts/analytics/src/lib.rs:223-225`
   is `self.current_metrics.clone()`. There is no second derivation path to pin against.
2. **There is nothing to recompute from.** The contract stores no property valuations, no
   listing set and no trade tape. `historical_trends`, `property_sentiments` and
   `portfolio_positions` cannot yield an average price, a total volume or a listing count.
   `average_price` and `total_volume` are prices and amounts; no aggregation of this
   contract's own state produces them.

The issue's suggested `oracle + staking` snapshot would mean building an oracle
integration that does not exist in this crate — a feature well beyond a consistency-test
issue. Inventing a derivation that returns plausible-looking numbers would reproduce
#1198 in a different contract, which is the specific failure mode this issue is about.

So: left as an explicit follow-up and documented on `update_market_metrics`, not faked.

### What is achievable, and is what the acceptance criteria actually ask for

"Recompute equals stored" cannot be made into a real invariant without a second source
of truth. What can be done is make divergence impossible to miss:

- **Integrity checksum.** An FNV-1a digest over the three metric fields is written on
  every update; `verify_market_metrics_integrity()` recomputes and compares. This is the
  on-chain expression of the recompute-equals-stored idea. It is an integrity check, not
  a cryptographic commitment — it does not constrain a malicious writer, who recomputes
  it anyway.
- **Provenance.** `get_metrics_provenance()` returns the live metrics plus writer,
  timestamp, lifetime `update_count`, `is_override` and `is_intact`, so a consumer can
  distinguish an admin-supplied figure from a contract-derived one.
- **Override tracing.** Every write emits `MarketMetricsOverridden` with previous and
  new values, writer and timestamp.
- **Single write path.** `set_market_metrics` is the only writer, so the checksum,
  provenance and event cannot be bypassed by an update path that forgets them.

### Two silent-divergence bugs found while verifying the above

Both are the same failure mode #1195 reports, so both are fixed in PR #1241.

**1. `batch_update_metrics` discarded all but the last entry.**
(`contracts/analytics/src/lib.rs:245-263`)

```rust
for upd in updates.iter() {
    self.current_metrics = MarketMetrics { ... };   // assignment, not accumulation
}
self.env().emit_event(BatchMetricsUpdated { count: updates.len() as u64 });
```

The loop assigns rather than combines, so with N updates only the last survives — while
`BatchMetricsUpdated` reports `count: N` as though all had been applied. A consumer
trusting the event count would believe N updates landed when 1 did.

The fix combines entries — volumes and counts sum, `average_price` becomes the
volume-weighted mean — and emits the resulting value in the event so it cannot disagree
with storage.

> **This is a behaviour change and needs a maintainer's acknowledgement.** A caller that
> passed several entries expecting last-write-wins now gets a combined view. The previous
> behaviour was data loss, so it did not seem right to accept it silently.

**2. `batch_add_trends` emitted the wrong event.** (`contracts/analytics/src/lib.rs:268-281`)

Adding a *trend* emitted `BatchMetricsUpdated`. A consumer tailing the market-metrics
event got a spurious one per trend batch, with nothing in the payload to distinguish the
two. It now emits `BatchTrendsAdded`.

Both defects are invisible to a test suite that only exercises single-entry calls, which
is consistent with #1195's report that nothing pins the reported numbers to anything.

---

## Known remaining gap

Eight crates declare `ink_e2e`, but only `contracts/lib` uses it, and no `tests/`
directory anywhere in the repository references it. Seven unused declarations remain
after PR #1241, so those crates' `cargo test` is likely still broken for the same
`trie-db` / nightly reason that `propchain-crowdfunding` had. Out of scope for #1195–#1198,
but it means a green `cargo test` in this repository is not yet evidence of anything, and
it deserves its own PR.

## Verification

`cargo test -p propchain-crowdfunding -p propchain-monitoring -p propchain-analytics --lib`

144 tests pass across the three touched crates (analytics 43, crowdfunding 40,
monitoring 61), up from 98. `cargo fmt` and `cargo clippy` are clean on the changed code.
All new tests are listed in `MONITORING_ANALYTICS_RESOLUTIONS.md`.

Two #1198 tests need a state the public API cannot reach: `invest` requires an onboarded,
KYC-approved, **accredited** profile, so a campaign built through the public API always
has exactly one profile per investor. Two `#[cfg(test)]`-gated helpers
(`erase_investor_profile`, `set_investor_accredited`) reach a missing or revoked profile.
They are compiled out of non-test builds and are not part of the contract interface.

Not re-run for this documentation-only change; the figures above are from PR #1241.
