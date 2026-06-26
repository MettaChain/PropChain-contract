# Governance Constitution

## Quorum & Voting Period

| Parameter          | Value                              |
|--------------------|------------------------------------|
| Quorum             | 10% of total voting power          |
| Voting Period      | 28,800 blocks (~2 days at 6s/block)|
| Approval Threshold | >50% of votes cast                 |

## Timelocks

| Proposal Type    | Minimum Timelock |
|------------------|-----------------|
| Parameter Change | 1 day           |
| Fund Release     | 2 days          |
| Emergency Pause  | 0 (immediate)   |

## Proposal Lifecycle

1. **Creation** — Staker with ≥1% of supply submits proposal; voting power snapshotted at this block.
2. **Voting** — Open for the voting period; only snapshot-block balances count.
3. **Quorum Check** — Participation must reach 10% before voting closes.
4. **Timelock** — Approved proposals wait the configured timelock before execution.
5. **Execution** — Anyone may trigger execution after timelock expires.
6. **Rejection** — Proposals failing quorum or threshold are rejected and cannot be re-submitted for 24 hours.

## Emergency Pause

Admin may invoke `emergency_override` to pause immediately. Resumption requires a governance vote with ≥60% approval.

## Edge Case Recovery

- **Stuck proposals**: Admin may cancel proposals open > 30 days with zero participation.
- **Delegate unavailability**: Delegators may reclaim voting power at any block before voting closes.
- **Quorum failure**: Proposal may be resubmitted after a 24-hour cooldown.