# ADR 5: Lending Liquidation Buffer Design

## Status

Accepted

## Context

The PropChain lending contract allows property owners to borrow stablecoins against tokenized real estate as collateral. If the collateral value falls below the loan value, the protocol must liquidate the position to remain solvent.

Real estate valuations are updated infrequently (hourly to daily via the oracle commit-reveal cycle, per ADR 0002). This creates a window where a sharp drop in property value could leave the protocol undercollateralized before a liquidation can be triggered. Unlike DeFi protocols with liquid, second-by-second price feeds, real estate prices move more slowly but oracle latency can still allow bad debt to accumulate.

Requirements:
- Prevent the protocol from accumulating bad debt when collateral values drop.
- Avoid premature liquidations that harm borrowers when prices are merely volatile.
- Incentivize third-party liquidators to act promptly.

## Decision

We implement a **two-threshold liquidation buffer**:

1. **Warning threshold (LTV = 80%)**: When the loan-to-value ratio reaches 80%, the protocol emits a `LiquidationWarning` event and allows the borrower to add collateral or partially repay without penalty.
2. **Liquidation threshold (LTV = 90%)**: When LTV reaches or exceeds 90%, the position becomes eligible for liquidation. Any external account can call `liquidate(loan_id)`.
3. **Liquidation bonus**: The liquidator receives a 5% bonus on the liquidated collateral value, funded from the protocol fee reserve, as an incentive to act quickly.
4. **Partial liquidation**: Only enough collateral is sold to restore the LTV to 70% (the target ratio), minimizing disruption to the borrower.
5. **Oracle staleness guard**: If the oracle valuation for the collateral property is older than `max_oracle_age` blocks (default: 50 blocks), `borrow` and `liquidate` revert with `InvalidValuation` (code 4003) until a fresh valuation is available. This prevents the system from acting on dangerously stale data.

The thresholds (80%, 90%, 70%) and the liquidation bonus (5%) are configurable via a governance proposal.

## Consequences

**Positive:**
- The two-threshold design gives borrowers a warning before forced liquidation, improving UX for legitimate borrowers.
- Partial liquidation avoids wiping out an entire position over a minor LTV breach.
- The oracle staleness guard prevents the system from executing liquidations based on outdated prices.
- The liquidation bonus creates a competitive market for liquidators, ensuring prompt action.

**Negative:**
- The staleness guard can pause lending and liquidation activity if oracle operators are slow, potentially creating a window of undercollateralization.
- Configuring thresholds via governance means the system cannot respond quickly to extreme market conditions without an emergency governance process.
- The 5% liquidation bonus is a direct cost to the protocol's fee reserve; reserve depletion during a market crash could reduce the incentive.

**Neutral:**
- `LiquidationThresholdNotMet` is returned when `liquidate` is called on a position with LTV below 90%. Liquidator bots should check `get_loan_ltv(loan_id)` before attempting liquidation to avoid wasted gas.
- `InsufficientCollateral` (lending) is returned by `borrow` when the requested loan amount would immediately place the LTV above 80%.
