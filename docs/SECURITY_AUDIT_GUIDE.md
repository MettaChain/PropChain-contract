# Security Audit Guide

## Overview

PropChain combines automated static analysis (run on every commit) with periodic
third-party audits conducted by independent security firms. This document
describes the cadence, scope, process, and record-keeping for those audits.

## Audit Cadence

| Trigger | Minimum frequency |
|---------|------------------|
| Routine scheduled | Every 6 months |
| Major release (storage layout change, new contract, upgrade) | Before deployment |
| Critical vulnerability patched | Within 30 days of fix |

The `security-audit` CLI enforces the schedule locally:

```bash
# Check whether an audit is overdue
cargo run --bin security-audit -- check-schedule

# Print a blank schedule template
cargo run --bin security-audit -- print-schedule-template > audit-schedule.json
```

Update `audit-schedule.json` at the repository root after each completed audit.
The CI pipeline reads this file and fails the build if the audit is overdue.

## Approved Auditors

Any firm with demonstrated ink!/Substrate smart-contract experience may be
engaged. Past and preferred vendors:

- [Oak Security](https://www.oaksecurity.io/)
- [Trail of Bits](https://www.trailofbits.com/)
- [Halborn](https://halborn.com/)
- [CoinFabrik](https://www.coinfabrik.com/)

Engage at least one firm not previously used for the last audit to ensure
independent perspective.

## Audit Scope

Each third-party audit must cover, at minimum:

1. **Access control** — all roles, permissions, and inheritance (see `docs/ACCESS_CONTROL_AUDIT.md`)
2. **Emergency pause / resume** — `pause_contract`, `emergency_pause`, `force_emergency_stop`, multi-sig resume
3. **Cross-contract calls** — bridge, oracle, compliance registry, fee manager
4. **Arithmetic safety** — overflow/underflow, saturating operations
5. **Reentrancy** — ink! storage locking, cross-contract call ordering
6. **Denial-of-service** — unbounded loops, storage exhaustion, gas limits
7. **Upgrade safety** — storage layout compatibility

Optional (recommended for major releases):

- Formal verification of critical invariants (Kani proofs in `contracts/lib/src/lib.rs`)
- Fuzz testing coverage review (`tests/security_fuzzing_tests.rs`)

## Pre-Audit Checklist

Before handing off to a firm:

- [ ] Run `cargo run --bin security-audit -- audit --report report.json` and review findings
- [ ] Run `cargo clippy --all-targets --all-features` — zero errors required
- [ ] Run `cargo test --all` — all tests passing
- [ ] Ensure `cargo audit` shows no critical advisories
- [ ] Tag the commit to be audited: `git tag audit/YYYY-MM`
- [ ] Share the commit tag, this guide, and `docs/ACCESS_CONTROL_AUDIT.md` with the auditor

## Post-Audit Process

1. Receive the draft report; triage each finding by severity.
2. Create a GitHub issue for each High/Critical finding (link back to the report).
3. Assign and resolve all High/Critical findings before the next release.
4. Obtain a re-test sign-off from the auditor on fixes.
5. Publish the final report to `docs/audits/YYYY-MM-<firm>.pdf`.
6. Update `audit-schedule.json`:
   - Set `last_audit_date` to the re-test completion date.
   - Set `auditor` to the firm name.
   - Set `report_url` to the published PDF path or URL.
   - Set `next_audit_date` to the planned follow-up date.

## CI Integration

`.github/workflows/security.yml` runs `check-schedule` on every push to `main`.
If `audit-schedule.json` reports an overdue audit the job fails with a non-zero
exit code, blocking deployment until the schedule is updated.
