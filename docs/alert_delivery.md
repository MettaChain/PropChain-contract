# Alert Delivery Contract

How an off-chain operator gets alerted by the `propchain-monitoring` contract,
and what the contract guarantees to them.

## The problem this solves

Alerts are raised by `record_operation`, which evaluates the configured
thresholds and emits an `AlertTriggered` event when one is breached. An event is
only *evidence that something happened*. It is not delivery: nothing on chain
can reach a pager.

That leaves an indexer or webhook worker as the only delivery path, and it
leaves two failure modes that the event stream alone cannot address:

- **The worker's indexer is down during the incident.** The event is emitted
  once and never re-read. The alert is lost, and the operator finds out from the
  outage rather than from the alert.
- **The alert is more critical than the batch it arrived in.** During a burst,
  one RPC round trip per alert is both slow and expensive, so the worker either
  falls behind or drops alerts.

`AlertTriggered` remains the authoritative record. This document describes the
*additional* surface that makes retry and batching possible.

## Retention

Every alert is also appended to a bounded ring buffer in contract storage:

| Constant | Value | Meaning |
| --- | --- | --- |
| `MONITORING_MAX_ALERT_LOG` | 256 | Alerts retained on chain |
| `MONITORING_ALERT_COOLDOWN_MS` | 300 000 | Minimum gap between repeats of one alert type |

The buffer is a ring: once it is full, the oldest alert is overwritten. The
window is deliberately much larger than the five-minute cooldown, so a worker
that is offline for a couple of hours still finds every alert it missed on its
next poll. Beyond that window, alerts are genuinely gone and the worker must
fall back to reading live state via `health_check()`.

## Reading alerts

### `get_recent_alerts(since_alert_id, limit) -> Vec<AlertRecord>`

The batch read. `since_alert_id` is the **last id the caller already handled**,
and the result starts strictly after it, so consecutive calls tile the window
with no gaps and no duplicates.

Alert ids start at `0`, which leaves no value meaning "handled nothing yet", so
`0` is used as that sentinel and starts the batch at the oldest retained alert.

Two details matter for a worker that has been away:

- A cursor **older than the window** is clamped forward to the oldest retained
  alert. The worker resumes from the oldest alert it can still see rather than
  silently receiving nothing and assuming it is up to date.
- `limit` is clamped to `MONITORING_MAX_ALERT_LOG`, so one call cannot exceed
  the retained window. `limit = 0` is treated as "everything retained" rather
  than "nothing", because an unset limit that returned an empty batch would look
  identical to a healthy, alert-free system.

### `alert_payload(alert_id) -> Option<AlertPayload>`

A single alert rendered as a self-contained blob: the structured fields, the
alert type's stable name, a severity ranking, and a canonical JSON object.

```json
{
  "alertId": 0,
  "alertType": "HighErrorRate",
  "severity": 1,
  "currentValue": 10000,
  "threshold": 500,
  "triggeredAt": 1700000,
  "acknowledged": false
}
```

`alertType` is a name rather than a SCALE enum discriminant so a consumer does
not have to decode the enum to route on it. `severity` is `1` for
`HighErrorRate` and `2` for `SystemDegraded`, so a batch can be triaged without
a second round of calls.

Returns `None` for an id that was never recorded or has been evicted. It does
not return a stale value from a recycled slot.

`alertId` is the delivery key. A consumer that deduplicates on it is safe
against redelivery, which matters because `acknowledge_alert` is idempotent and
a worker that crashes between POSTing and acknowledging will re-send.

### `acknowledge_alert(alert_id)` — admin only

Marks an alert delivered. Idempotent: acknowledging an already-acknowledged
alert succeeds without changing state, so a redelivery that races a previous
ack does not error. Returns `MonitoringError::AlertNotFound` for an unknown or
evicted id.

### `pending_alert_count() -> u32`

Number of retained alerts that are **not** acknowledged. This is the retry set
size, and the signal to monitor for stuck delivery: a value that is not falling
means the worker is not making progress, even while the contract is perfectly
healthy.

## Recommended worker loop

```
cursor = 0
loop:
    for record in get_recent_alerts(cursor, 50):
        deliver(record)                  # POST alert_payload(record.alert_id).json
        acknowledge_alert(record.alert_id)
        cursor = record.alert_id        # advance only after a successful ack
    sleep(poll_interval)
```

Three properties this loop depends on:

1. **Advance the cursor only after acknowledging.** If the POST fails, leaving
   the cursor put means the same alert is returned on the next poll, which is
   the retry.
2. **`get_recent_alerts` is gap-free.** Advancing to the last handled id cannot
   skip an alert that was written between calls.
3. **Alert ids are monotonic across eviction.** An id is never reused for a
   different alert, so a cursor that has fallen behind the window is clamped
   rather than silently aliased onto newer alerts.

## What this contract does not do

- It does not send anything. Delivery is off chain by design; a contract cannot
  make an outbound request.
- It does not guarantee delivery. A worker that stays away longer than the
  256-alert window will miss alerts, and only live state via `health_check()`
  remains. The on-chain ring buffer bounds *how long* a missed alert stays
  recoverable; it does not make recovery unbounded.
- It does not replace the event stream. `AlertTriggered` is still the
  authoritative record and is what a chain reindex should be built from. The
  ring buffer is the operational retry surface.
