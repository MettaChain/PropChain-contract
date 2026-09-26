# Resolutions: issues #1195, #1196, #1197, #1198

What each issue reported, what the code actually did, and what changed.

Test totals for the three touched crates: **144 passing** (analytics 43,
crowdfunding 40, monitoring 61), up from 98. `cargo fmt` and `cargo clippy` are
clean on the changed code.

| Issue | Subject | Outcome |
| --- | --- | --- |
| [#1198](#1198) | Crowdfunding success metrics blended placeholder analytics | Placeholders replaced with values derived from recorded state; absent data returns `None`; duplicate `line.rs` deleted |
| [#1196](#1196) | `quorum_guard` history grew without bound | History capped to a rolling window; reads bounded by a constant |
| [#1197](#1197) | Alerts were on-chain records with no delivery or retry | Bounded alert log, batch read, self-contained payload, acknowledgement, delivery contract documented |
| [#1195](#1195) | No test pinned recomputed metrics to the reported ones | Integrity checksum, provenance and override tracing, consistency tests |

---

## #1198 — Crowdfunding success metrics blended placeholder analytics

### Reported

`get_campaign_success_metrics` reported fabricated coefficients as measured
outcomes: a hardcoded 80% retention rate, an invented jurisdiction split and an
invented investment-size distribution, duplicated in a `line.rs` copy of the
contract.

### What the code actually did

The function named in the report is `get_campaign_analytics`. Every reported
placeholder was real:

| Location | Placeholder |
| --- | --- |
| `get_campaign_analytics` | `investor_retention_rate = 8_000` — a literal `80%` comment |
| `get_investor_demographics` | `accredited_investors = total_investors * 7 / 10` — "assume 70% accredited" |
| `get_investor_demographics` | jurisdiction split hardcoded to US 60% / CA 20% / EU 10% / Other 10% |
| `get_investor_demographics` | distribution hardcoded to 30% / 40% / 20% / 10% across four buckets |
| `get_investor_demographics` | `max_investment = 0`, so `top_investor_amount` was always zero |
| `get_funding_timeline` | 30 points on a straight `target_amount / 30` ramp |

`get_investor_demographics` compounded this: it iterated *every campaign id* to
find one, and then reported figures for that campaign that had nothing to do with
its investors.

### Fix

Every figure is now derived from storage the contract already maintains:

- **`investor_retention_rate`** — the share of recorded investors who have not
  claimed a refund, from `campaign_investors` and `refunds_issued`. The
  denominator is the stored investor list rather than `campaign.investor_count`,
  which can drift from it. Type changed `u32` → `Option<u32>`: `None` for a
  campaign with no recorded investors, because retention over an empty cohort is
  undefined and reporting `8_000` there was the bug.
- **`accredited_investors`** — counted from `InvestorProfile.accredited`.
- **`jurisdictions`** — grouped from `InvestorProfile.jurisdiction`, ordered by
  descending count then name for a stable result. Type changed
  `Vec<(String, u32)>` → `Option<Vec<(String, u32)>>`, returning `None` when not
  one investor has a stored profile. Investors with no profile are grouped under
  `"Unknown"` so the split still accounts for the whole cohort.
- **`investment_distribution`** — a histogram over the recorded `investments`
  amounts (bounds 1k / 10k / 100k / 1M), with empty buckets dropped.
- **`top_investor_amount`** — the real maximum, from the same scan that already
  computed `largest_investment`.
- **`get_funding_timeline`** — now always returns `None`, and says why in its
  docs. A cumulative curve is not reconstructible: `investments` is keyed by
  `(campaign_id, investor)` and holds a running total with no per-investment
  timestamp, and `campaign_investors` is an unordered `Vec` rather than a
  chronological log. Producing a curve from that would be a different
  fabrication. The docs describe the state change that would make it real.

`campaign_investors` is now read once per analytics call and reused by the
largest-investment scan and the retention calculation.

`contracts/crowdfunding/src/line.rs` is deleted. It was 1 835 lines, an older
snapshot of the whole contract: not declared as a module in `lib.rs` and
therefore **never compiled**, and referenced nowhere in the repository (a grep
for `line::` hits only the word "linear" in an unrelated crate's comment). All
45 of its public function names are duplicates of methods that already exist in
`lib.rs`, so it contributed no function the compiled contract did not already
have. It was a live copy of every placeholder above.

### Verification

`contracts/crowdfunding/src/lib.rs` also had a pre-existing
`investor_retention_rate` consumer in `test_investor_demographics`, which
asserted only `!demographics.jurisdictions.is_empty()`. It now asserts the exact
derived values.

New tests cover: retention absent for an empty cohort; full retention with no
refunds; retention falling to 6 666 bps after one of three refunds; a case that
returns 5 000 bps where the old constant could only ever return 8 000;
jurisdictions absent with no surviving profile; unprofiled investors grouped
under `"Unknown"`; accredited count read from profiles; real histogram bucketing
with one investor per bucket; an empty distribution for an empty cohort; and
`get_funding_timeline` returning `None` for a funded campaign and for an unknown
one.

Two of these need a state the public API cannot reach: `invest` requires an
onboarded, KYC-approved, **accredited** profile, so a campaign built through the
public API always has exactly one profile per recorded investor. Two
`#[cfg(test)]`-gated helpers (`erase_investor_profile`, `set_investor_accredited`)
let the tests reach a missing or revoked profile. They are compiled out of
non-test builds and are not part of the contract interface.

### Note on the manifest

`contracts/crowdfunding/Cargo.toml` declared `ink_e2e` as a dev-dependency while
having no `tests/` directory and no reference to it. That unused dependency
pulls in the whole substrate stack, and `trie-db` 0.28.0 does not compile on the
nightly toolchain this repo pins — so `cargo test -p propchain-crowdfunding`
failed before running a single test, on `main` as well as on this branch. The
unused dev-dependency is removed, which is what makes the tests above runnable.

**Worth a separate look:** eight crates declare `ink_e2e`, but only `contracts/lib`
actually uses it (in `src/e2e_tests.rs`) and no `tests/` directory references it
at all. Seven unused declarations therefore remain after this change, so their
`cargo test` is broken for the same reason. That is outside this PR's scope.

---

## #1196 — `quorum_guard` participation history grew without bound

### Reported

`QuorumGuard` stored `history: Vec<ProposalParticipation>` with no cap, appending
one entry per proposal forever, so both storage and the participation read grew
with the lifetime proposal count.

### Fix

`contracts/monitoring/src/quorum_guard.rs` now keeps a rolling window bounded by
the new `MONITORING_MAX_QUORUM_HISTORY` (100), evicting the oldest record when
full. The window is a `Vec`, matching the rest of the crate, so eviction shifts
within a compile-time-constant number of elements.

The read API is built for the window:

- `capacity()` — the bound.
- `participation_bps(proposal_id)` — `None` for an evicted proposal rather than a
  stale or invented value.
- `average_participation_bps()` — `None` for an empty window.
- `history_page(offset, limit)` — paginated, with `limit` clamped to what remains.
- `window()` — the retained records, oldest first.
- `is_saturated()`, `total_recorded()`, `evicted_count()`.

`total_recorded` is a separate lifetime counter that survives eviction, so the
number of proposals observed is never lost — only the per-proposal detail of old
ones.

This mirrors the circular buffer the crate already uses for metrics snapshots
(`MONITORING_MAX_SNAPSHOTS`).

### Verification

Eleven new tests. The load test records 50 000 proposals and asserts
`history_len() == capacity()`, `total_recorded() == 50_000` and
`evicted_count() == 50_000 - capacity()`, which is the case the unbounded `Vec`
could not survive. The others assert: the window caps at capacity after 4×
capacity records and reports `is_saturated()`; eviction keeps the newest and
reports `None` for the oldest; the window holds exactly the last `capacity`
records in order; a lookup for a long-evicted proposal returns `None` while the
average still spans the window rather than the lifetime history; the average is
`None` until something is recorded and then reflects only retained records;
`history_page` clamps a limit past the end of the window; a retained proposal is
still found and an unknown one is `None`; and the participation warning, which
compares a proposal against the previous record in the window, still fires after
the window has rolled over 10 000 times.

---

## #1197 — Alerts had no out-of-band delivery or retry

### Reported

`AlertTriggered` events were the only alert record. Nothing delivered them, there
was no retry, and an operator whose indexer was down during an incident missed
the signal entirely.

### Fix

Alerts are now also appended to a bounded ring buffer
(`MONITORING_MAX_ALERT_LOG` = 256) alongside the event, which stays
authoritative. Four messages were added to the `MonitoringSystem` trait:

- `alert_payload(alert_id) -> Option<AlertPayload>` — one alert as a
  self-contained blob: structured fields, the alert type's stable name, a
  severity ranking, and canonical JSON. A consumer needs no follow-up calls and
  no SCALE enum decoding to route or render it.
- `get_recent_alerts(since_alert_id, limit) -> Vec<AlertRecord>` — gap-free batch
  read, with the limit clamped to the window. A cursor older than the window is
  clamped *forward* so a worker that was offline resumes instead of silently
  receiving nothing.
- `acknowledge_alert(alert_id)` — admin only, idempotent, clears the alert from
  the retry set. New `MonitoringError::AlertNotFound` (code 10007).
- `pending_alert_count() -> u32` — size of the retry set; a value that is not
  falling means delivery is stuck.

`alertId` is the delivery key, so a consumer that deduplicates on it is safe
against redelivery after a crash between POST and acknowledge.

Two bugs found while implementing this, both in the ring-buffer readers I wrote
and both caught by the eviction tests: the batch cursor treated `since_alert_id
= 0` as "skip alert 0" rather than as the "handled nothing" sentinel, and
`pending_alert_count` read `alert_log.get(id)` without the ring-buffer modulo,
silently missing every id at or above the cap. Both readers now go through one
`retained_alert` helper that applies the modulo and rejects a recycled slot
whose `alert_id` no longer matches.

**`docs/alert_delivery.md`** specifies the delivery contract: retention and
window sizing, the read and acknowledgement semantics, a recommended worker loop
and the three properties it depends on, and an explicit list of what the
contract does not do.

### Verification

Twenty new tests: payload is `None` before any alert fires; a fired alert is
recorded; the payload is self-contained; the JSON matches its canonical form
exactly; `SystemDegraded` carries its own name and higher severity; batching is
oldest-first; the cursor is exclusive; limits are honoured and clamped; an unset
limit returns everything; a stale cursor resumes without gaps; acknowledgement
marks, is idempotent, and rejects an unknown id; `pending_alert_count` tracks
the retry set; the log evicts beyond the cap; a recycled slot never leaks an
older alert; and pending count, batch read and slot lookup all stay bounded and
correct after eviction.

---

## #1195 — No consistency test tied reported metrics to stored ones

### Reported

`get_market_metrics` and the admin override paths were not pinned together, so
overrides and recomputed values could diverge silently.

### A correction to the premise

`get_market_metrics` does not build `MarketMetrics` from contract views. It
returns `self.current_metrics.clone()` — a single stored value. There is no
independent derivation path in this contract, so there are no "two views" to
pin together.

More importantly, **there is nothing to recompute from.** The contract stores no
property valuations, no listing set and no trade tape; `historical_trends`,
`property_sentiments` and `portfolio_positions` cannot yield an average price,
a total volume or a listing count. `average_price` and `total_volume` are
prices and amounts, and no aggregation of this contract's own state produces
them.

So an `oracle + staking` snapshot, as the issue proposes, would mean adding an
oracle integration that does not exist here. Writing one would be a large
feature well beyond a consistency-test issue, and inventing a derivation that
returns plausible-looking numbers would reproduce the exact failure mode of
#1198 in a different contract. That path is left as an explicit follow-up and is
documented on `update_market_metrics` rather than faked.

What *is* achievable, and what the acceptance criteria ask for, is the part that
does not need a second source: make the reported value's provenance explicit and
auditable, and make a silent divergence impossible to miss.

### Fix

- **Integrity checksum.** An FNV-1a digest over the three metric fields is
  written alongside the metrics on every update.
  `verify_market_metrics_integrity()` recomputes it and compares, so a partial or
  corrupted write is reported rather than served as fact. This is the on-chain
  expression of the "recompute == stored" invariant. It is an integrity check,
  not a commitment — it does not constrain a malicious writer, who recomputes it
  anyway.
- **Provenance.** `get_metrics_provenance()` returns the live metrics plus
  `updated_at`, `updated_by`, `update_count`, `is_override` and `is_intact`. A
  consumer can now tell an admin-supplied figure from a contract-derived one
  instead of having to trust it.
- **Override tracing.** Every write emits `MarketMetricsOverridden` with the
  previous and new values, the writer and the timestamp. A lifetime
  `update_count` plus the last writer means an unexplained change in the
  reported number is always attributable to a specific admin write.
- **Single write path.** `set_market_metrics` is the only writer, so the
  checksum, provenance and event cannot be left out of an update path.
- **Documented override semantics.** On `update_market_metrics`: the admin value
  is authoritative until rewritten, `is_override` stays `true`, nothing
  recomputes these numbers on chain, and a consumer needing independent
  verification must recompute off chain from the same source.

### Two further bugs found here

Both are silent-divergence bugs of exactly the kind this issue is about, and
both are fixed:

1. **`batch_update_metrics` discarded all but the last entry.** It looped over
   `updates` assigning `self.current_metrics` each time, so only the final
   `MetricUpdate` survived — while `BatchMetricsUpdated` reported `count` as
   though all of them had been applied. The entries are now **combined**:
   volumes and listing counts sum, and `average_price` is the volume-weighted
   mean, falling back to the unweighted mean when the entries carry no volume.
   `BatchMetricsUpdated` now also carries the resulting metrics, so the event
   and the stored value cannot disagree.
2. **`batch_add_trends` emitted `BatchMetricsUpdated`.** Adding a *trend*
   emitted the *market-metrics* event, so a consumer tailing that event for
   metric changes got a spurious one per trend with no way to tell them apart.
   It now emits a new `BatchTrendsAdded { count }`.

The aggregation semantics in (1) are a behaviour change: a caller that passed
several entries and expected last-write-wins now gets a combined view. The
previous behaviour was data loss, so this needs a maintainer's acknowledgement
rather than silent acceptance.

### Verification

Thirteen new tests: reported metrics equal stored metrics; integrity holds on a
fresh deploy and after every update path; a deliberately mismatched checksum is
reported; the checksum covers each of the three fields independently;
`is_override` is false before any write and true after; every write is counted
and timestamped; provenance reports the live value; batch combination sums
volumes and counts and produces a 250 volume-weighted mean from `(100 @ 1 000)`
and `(300 @ 3 000)`; a single entry is unchanged; the no-volume fallback yields a
plain mean of 200; an empty batch yields zeroed metrics; oversized batches are
still rejected; and a non-admin caller cannot write metrics or move the stored
value.
