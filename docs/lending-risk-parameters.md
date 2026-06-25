# Lending Risk Parameters Governance Guide

This document outlines the recommended governance parameters for the PropChain Lending Protocol to ensure capital efficiency while maintaining robust risk management against market volatility and potential defaults.

## Reserve Factor

The **Reserve Factor** dictates the portion of borrower interest that is diverted to the protocol's treasury as a reserve against bad debt. 

- **Stablecoins (USDC/USDT):** 10% - 15%
- **Major Crypto (BTC/ETH):** 20%
- **Tokenized Real Estate / Prop Tokens:** 25% - 30%

*Recommendation:* Higher reserve factors should be applied to less liquid assets (like tokenized real estate) to build a larger safety buffer against potential shortfall events during liquidations.

## Loan-to-Value (LTV) Ratios

The **LTV Ratio** defines the maximum amount that can be borrowed against a specific collateral type.

- **Stablecoins:** 80% - 85%
- **Major Crypto (BTC/ETH):** 70% - 75%
- **Tokenized Real Estate (Prime Residential):** 60%
- **Tokenized Real Estate (Commercial/Development):** 40% - 50%

*Recommendation:* LTV ratios must reflect the historical volatility and liquidity of the underlying asset. Real estate tokens, while relatively stable, suffer from low secondary market liquidity, necessitating lower LTV ceilings compared to major cryptocurrencies.

## Liquidation Thresholds

The **Liquidation Threshold** is the LTV point at which a position becomes eligible for liquidation (health factor drops below 1.0).

- **Stablecoins:** 85% - 90%
- **Major Crypto (BTC/ETH):** 80%
- **Tokenized Real Estate:** 70% - 75%

*Recommendation:* The liquidation threshold should be set higher than the LTV ratio to provide a "safety cushion" for borrowers to top up their collateral or repay debt before facing liquidation. For real estate, a wider spread (e.g., 60% LTV, 75% Liquidation Threshold) is recommended due to slower price discovery and auction clearing times.

## Oracle Freshness Windows

The **Oracle Freshness Window** defines the maximum acceptable age of a price feed update before the protocol halts borrowing/liquidation actions to prevent stale-price exploits.

- **Stablecoins:** 24 hours (or upon 0.5% deviation)
- **Major Crypto (BTC/ETH):** 1 hour (or upon 1% deviation)
- **Tokenized Real Estate:** 7 days - 30 days (depending on independent appraisal frequency)

*Recommendation:* Real estate appraisals are inherently slow; therefore, oracle updates for tokenized properties can have significantly longer freshness windows. However, during times of broad macroeconomic distress, governance should manually trigger off-cycle property reappraisals.
