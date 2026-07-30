Add fallback-path tests for Oracle source failures
Repo Avatar
MettaChain/PropChain-contract
Problem Statement
Oracle circuit-breaker trips in tests/integration_bridge_oracle.rs:233-249 but no ‘2-of-3’ feed outage test exists.

Why it matters
Reality: feeds go down intermittently; orchestration must degrade gracefully.

Technical Context
Test that fee aggregation falls back to TWAP when sources fail.

Expected Outcome
Aggregation reduces to TWAP and emits OracleFallbackToTwap event.

Acceptance Criteria
tests/oracle_feed_outage.rs covers 2-of-3 outage.
OracleFallbackToTwap event observed.
Kani harness.
Implementation Notes
Add TWAP computation module and tests.

Files or modules likely to be affected
contracts/oracle/src/lib.rs, tests.

Dependencies
#121 (TWAP) for fallback completion.

Difficulty level
MEDIUM.

Estimated effort
S (~1 day).


