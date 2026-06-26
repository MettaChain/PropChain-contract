# Troubleshooting and FAQ

This document provides solutions to common issues and answers to frequently asked questions for developers working with PropChain smart contracts.

## Common Issues

### 1. Compliance Verification Failure
**Issue:** Submitting a transaction returns `ComplianceFailed` or `ComplianceError::NotVerified`.
**Solution:**
- Ensure the account has undergone KYC/AML verification via the `ComplianceRegistry`.
- Check if the verification has expired using `get_compliance_data`.
- Verify that GDPR consent has been granted via `update_consent`.

### 2. Bridge Request Timeout
**Issue:** A bridge request has not been executed and the timeout has passed.
**Solution:**
- Bridge operators may be offline or experiencing high network congestion.
- Use `recover_failed_bridge` with `RecoveryAction::UnlockToken` to retrieve your token on the source chain.
- Try re-initiating the bridge with a higher gas estimate or longer timeout.

### 3. "InsufficientSignatures" Error on Bridge Execution
**Issue:** Calling `execute_bridge` fails with `InsufficientSignatures`.
**Solution:**
- Verify that the required number of bridge operators have signed the request using `monitor_bridge_status`.
- Wait for more signatures if the threshold has not been met.

### 4. IPFS CID Validation Error
**Issue:** Registering metadata or documents fails with `InvalidIpfsCid`.
**Solution:**
- Ensure the CID format is correct (CIDv0 starts with "Qm", CIDv1 starts with "b").
- Check if the CID string is exactly 46 characters for CIDv0.

### 5. Insurance Premium Calculation Discrepancy
**Issue:** The premium calculated off-chain doesn't match the on-chain value.
**Solution:**
- Ensure you are using the exact same risk assessment parameters.
- Check if the `PropertyValuation` has updated since the last calculation.
- On-chain premium calculation factors in real-time pool utilization and reinsurance costs.

## Frequently Asked Questions

### Q: Which standards are PropChain tokens compatible with?
A: PropChain property tokens are natively compatible with both ERC-721 (for unique property ownership) and ERC-1155 (for fractionalized or batch operations).

### Q: How is data privacy handled for compliance?
A: We use a combination of encrypted data hashes in the `ComplianceRegistry` and private Zero-Knowledge proofs in `ZkCompliance` to ensure regulatory requirements are met without exposing sensitive personal information on-chain.

### Q: Can a property be un-tokenized?
A: Yes, if the business logic allows, a token can be burned, and the property status can be updated to "Unlisted" or "Private" in the registry, effectively removing it from the blockchain lifecycle.

### Q: What happens if a bridge operator is malicious?
A: The bridge uses a multi-signature threshold. A single malicious operator cannot compromise a bridge request. Operators are authorized through a governance process and can be removed/slashed for bad behavior.

### Q: How often are property valuations updated?
A: Valuations depend on the `PropertyValuationOracle` configuration. High-volatility markets may have daily updates, while stable assets might be updated monthly or upon request.

---

## Additional Troubleshooting

### 6. `TokenNotFound` When Calling Transfer
**Issue:** `transfer_token` returns `TokenNotFound` even though the token was minted.
**Root Cause:** The token ID passed to `transfer_token` does not match the ID returned by `mint`. Token IDs are assigned sequentially and start at 1.
**Fix:**
- Capture the return value from `mint` to get the exact token ID.
- Use `get_token_info(id)` to confirm the token exists before transferring.

### 7. `InvalidState` on Escrow Release
**Issue:** Calling `release_escrow` returns `InvalidState`.
**Root Cause:** The escrow is not in the `Funded` state. Common causes are releasing before funding completes, or after the escrow has already been released or cancelled.
**Fix:**
- Call `get_escrow_status(escrow_id)` to inspect the current state.
- Ensure all parties have signed and the escrow is fully funded before releasing.

### 8. Gas Estimation Fails for Large Batch Operations
**Issue:** Batch minting or batch transfer calls fail with `OutOfGas` or `GasLimitExceeded`.
**Root Cause:** Each item in a batch consumes incremental storage and execution weight. Large batches can exceed the block gas limit.
**Fix:**
- Split batches into chunks of 10–20 items.
- Use `estimate_weight` in `cargo-contract` to calibrate your batch size before submitting.

### 9. Oracle Returns Stale Valuation
**Issue:** `get_valuation` returns a price that is significantly out of date.
**Root Cause:** Either no oracle source submitted a new value within the staleness window, or the `InsufficientSources` guard prevented aggregation.
**Fix:**
- Call `get_oracle_sources(property_id)` to verify that active, reputable sources exist.
- Trigger `request_valuation` to prompt a fresh off-chain computation cycle.
- Check that oracle sources have not been deregistered via the admin interface.

### 10. `ProhibitedJurisdiction` Error During Compliance Check
**Issue:** Operations fail with error code `6007 ProhibitedJurisdiction`.
**Root Cause:** The user's registered jurisdiction is on the restricted list maintained by the `ComplianceRegistry`.
**Fix:**
- Confirm the jurisdiction stored in `get_compliance_data` is correct—incorrect registration is a common mistake.
- If the jurisdiction is correct and restricted, contact the compliance team; this cannot be bypassed programmatically.

### 11. Cross-Chain Token Does Not Arrive on Destination Chain
**Issue:** `initiate_bridge` completes on the source chain, but no token appears on the destination chain.
**Root Cause:** The relayer network may not have processed the event, or the destination chain's `execute_bridge` call failed silently.
**Fix:**
- Query `monitor_bridge_status(request_id)` on the source chain to check whether enough operators have signed.
- On the destination chain, call `get_pending_bridge_requests()` to see if the request is queued.
- If the TTL has expired, use `recover_failed_bridge` with `RecoveryAction::UnlockToken` to reclaim the token.

### 12. `InsufficientCollateral` in Lending Contract
**Issue:** `borrow` fails with `InsufficientCollateral` even after depositing collateral.
**Root Cause:** The property valuation oracle has not updated the collateral value, so the LTV ratio calculation uses a stale (lower) price.
**Fix:**
- Trigger a fresh valuation via `request_valuation(property_id)` on the oracle contract.
- Wait for oracle confirmation, then retry `borrow`.
- If urgent, add additional collateral to ensure the LTV ratio is met even at the stale price.

### 13. `DuplicateClaim` on Insurance
**Issue:** Filing an insurance claim returns `DuplicateClaim`.
**Root Cause:** A claim for the same event and policy has already been submitted (even if still pending).
**Fix:**
- Call `get_claims_for_policy(policy_id)` to list existing claims and their statuses.
- If the existing claim is stuck in `Pending`, contact the insurer to resolve it rather than filing a new one.

### 14. `VersionConflict` When Updating Property Metadata
**Issue:** `update_metadata` fails with `VersionConflict`.
**Root Cause:** Another transaction updated the metadata between when you read it and when you submitted your update. The contract uses optimistic concurrency—a `version` field must match the on-chain value.
**Fix:**
- Re-fetch the metadata with `get_metadata(property_id)` to obtain the latest `version`.
- Reapply your changes on top of the new version and resubmit.

### 15. Governance Proposal Not Executing After Timelock
**Issue:** Calling `execute_proposal` after the timelock period returns `TimelockActive`.
**Root Cause:** The block timestamp used by the contract is behind wall-clock time, or the timelock duration was configured longer than expected.
**Fix:**
- Check `get_proposal_details(proposal_id)` for the exact `executable_after` block timestamp.
- Wait until the current block timestamp exceeds `executable_after`, then retry.

### 16. `SubscriberCallFailed` in EventBus
**Issue:** Publishing to a topic causes `SubscriberCallFailed` and the transaction reverts.
**Root Cause:** A subscriber contract's `on_event` callback panics or returns an error, which propagates back to the EventBus.
**Fix:**
- Identify the failing subscriber via the event logs or by calling `get_subscribers(topic)`.
- Fix or upgrade the subscriber contract to handle events gracefully (use defensive error handling).
- Temporarily remove the misbehaving subscriber with `unsubscribe(topic, subscriber_address)` until it is fixed.

---

## Frequently Asked Questions (Additional)

### Q: Can I upgrade a deployed contract?
A: Yes, via the `Proxy` contract and a governance proposal. Submit an upgrade proposal with the new code hash, collect the required governor approvals, wait for the timelock, then call `execute_upgrade`. See `docs/adr/0002-oracle-commit-reveal.md` for an example of how upgrade decisions are recorded.

### Q: How do I add a new bridge operator?
A: Bridge operators are managed through the `Governance` contract. Submit a `AddBridgeOperator` proposal, pass the vote, and execute it after the timelock. Individual operators cannot be added without a governance vote.

### Q: What is the minimum stake required to participate in governance voting?
A: The minimum is configured in the `Staking` contract's `min_stake` parameter. Query it with `get_staking_config()`. Typically it is set to 100 tokens on testnet; mainnet values are set by governance.

### Q: Why does my local test pass but the on-chain call fails?
A: The most common reasons are (1) gas limits are tighter on-chain, (2) block timestamps differ from `MockTimestamp` used in unit tests, and (3) the deployed contract version may differ from your local source. Always build with `--release` and use `cargo-contract` dry-run mode to simulate on-chain execution before submitting.

### Q: How do I rotate a compromised signing key?
A: Call `initiate_key_rotation(new_public_key)` on the `AccessControl` contract. After the cooldown period, call `confirm_key_rotation`. This two-phase process prevents an attacker from immediately taking control with a stolen key.

### Q: Is there a way to pause the entire system in an emergency?
A: Yes. Accounts with the `PauseAdmin` role can call `pause()` on individual contracts. For a system-wide pause, the `Governance` contract can issue a multi-contract pause proposal. Monitor the `ContractPaused` events to track state.
