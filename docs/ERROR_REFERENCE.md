# PropChain Error Code Reference

> Comprehensive reference for all error codes across the PropChain protocol contracts.
> **Issue:** #524 | **Source:** Compiled from all contract trait definitions.

---

## Error Code Index

| Code(s) | Contract / Crate | Error Enum |
|---------|------------------|------------|
| 1–10 | `traits` crate | `CommonError` |
| 1001–1035 | `property-token` | `Error` |
| 2001–2021 | `escrow` | `Error` |
| 3001–3018 | `bridge` | `Error` |
| 4001–4014 | `traits` crate | `OracleError` |
| 5001–5008 | `fees` | `Error` |
| 6001–6014 | `tax-compliance` | `Error` |
| 6001–6014 | `compliance_registry` | `Error` |
| 7001–7019 | `dex` | `Error` |
| 8001–8013 | `governance` | `Error` |
| 9001–9024 | `staking` | `Error` |
| 10001–10005 | `traits` crate | `MonitoringError` |
| 11001–11007 | `traits` crate | `EventBusError` |
| — | Various (no numeric codes) | See §6 |

> **⚠️ Known Bug:** Several contracts reuse the same numeric code for multiple semantically different errors. See [§5 — Known Bugs & Collisions](#5-known-bugs--collisions).

---

## 1. Authorization Errors

Errors raised when a caller lacks the required permissions, is unverified, or fails a compliance check.

| Code | Contract | Error Name | Description | Likely Cause | Resolution |
|------|----------|------------|-------------|--------------|------------|
| 1 | `traits` (CommonError) | `Unauthorized` | The caller is not authorized to perform the action | Caller lacks the required role or permission | Ensure the caller has the necessary role or permissions |
| 1001 | `property-token` | `NotAuthorized` | Caller is not authorized for the token operation | Caller is not the owner, admin, or approved operator | Verify caller identity and authorization before invoking |
| 1016 | `property-token` | `NotTokenOwner` | Only the token owner may perform this action | A non-owner called an owner-restricted function | Use the token owner's account or obtain authorization |
| 1017 | `property-token` | `MarketplaceNotApproved` | The marketplace operator is not approved | The caller's marketplace has not been set as an approved operator | Register the marketplace operator via the approval flow |
| 1023 | `property-token` | `ComplianceCheckFailed` | Token transfer blocked by compliance rules | Investor accreditation, jurisdiction, or KYC check failed | Complete required compliance verification |
| 1024 | `property-token` | `JurisdictionMismatch` | The investor's jurisdiction does not match the property's jurisdiction | Cross-jurisdiction trade attempted without proper exemptions | Verify jurisdiction compatibility or obtain exemption |
| 1025 | `property-token` | `KYCNotCompleted` | KYC/AML verification has not been completed | Investor has not submitted or passed KYC | Complete the KYC/AML process before transacting |
| 1026 | `property-token` | `SENDER_NOT_VERIFIED` | Sender address has not passed verification | Address is not whitelisted or verified on-chain | Complete address verification via the registry |
| 2001 | `escrow` | `Unauthorized` | Caller is not authorized to interact with this escrow | Only the depositor, beneficiary, or arbiter may act | Confirm the caller's role in the escrow agreement |
| 3001 | `bridge` | `Unauthorized` | Caller is not a recognized bridge signer or relayer | The caller is not in the bridge signer set | Only registered signers and relayers may call this function |
| 5001 | `fees` | `Unauthorized` | Caller is not authorized to configure fees | Only the fee admin or contract owner may set fee parameters | Use the admin account |
| 6001 | `tax-compliance` | `UnauthorizedAccess` | Caller does not have permission to access tax-compliance functions | Missing tax admin or compliance officer role | Grant the required role or use an authorized account |
| 6001 | `compliance_registry` | `UnauthorizedAccess` | Caller does not have permission to access registry functions | Missing registry admin role | Grant the required role or use an authorized account |
| 6002 | `tax-compliance` | `ComplianceCheckFailed` | General compliance check failure | Rule evaluation, jurisdiction check, or accreditation check failed | Inspect compliance rule output for specific failure reason |
| 6002 | `compliance_registry` | `ComplianceCheckFailed` | Compliance registry check failed | Registry-level compliance rule rejected the operation | Check associated compliance rules in the registry |
| 6003 | `tax-compliance` | `JurisdictionNotSupported` | The specified jurisdiction is not supported | The jurisdiction has not been configured in the tax module | Add the jurisdiction configuration or choose a supported one |
| 6003 | `compliance_registry` | `JurisdictionNotSupported` | Jurisdiction is not recognized by the registry | The registry does not have rules for this jurisdiction | Configure jurisdiction rules in the registry |
| 6013 | `tax-compliance` | `AdvisorInactive` | The tax advisor is currently inactive | Advisor was removed, suspended, or not yet activated | Activate the advisor or assign a different one |
| 6013 | `compliance_registry` | `AdvisorInactive` | Compliance advisor is not active | Advisor was deregistered or suspended | Activate the advisor record |
| 7001 | `dex` | — | — | — | — |
| 7012 | `dex` | `Unauthorized` | Caller is not authorized for this DEX operation | Pool or order access restricted to specific roles | Verify caller authorization |
| 8001 | `governance` | `NotAuthorized` | Caller is not authorized for this governance action | Only proposers, voters, or admins may perform this action | Check governance role requirements |
| 10005 | `traits` (MonitoringError) | `UnauthorizedAccess` | Caller is not authorized to submit metrics or manage alerts | Missing monitoring role | Grant monitoring access |
| 11006 | `traits` (EventBusError) | `UnauthorizedPublisher` | Publisher is not registered to emit events | The contract or account has not been authorized as a publisher | Register the publisher with the event bus |

---

## 2. Validation Errors

Errors raised when inputs, parameters, or state transitions are invalid.

| Code | Contract | Error Name | Description | Likely Cause | Resolution |
|------|----------|------------|-------------|--------------|------------|
| 4 | `traits` (CommonError) | `InvalidInput` | Provided input is malformed or out of range | Parameter failed basic validation | Check input format and constraints |
| 5 | `traits` (CommonError) | `Overflow` | Arithmetic operation overflowed | Computed value exceeded the maximum representable value | Use safe math or check bounds before computation |
| 6 | `traits` (CommonError) | `Underflow` | Arithmetic operation underflowed | Computed value went below the minimum representable value | Use safe math or check bounds before computation |
| 4005 | `traits` (OracleError) | `InvalidPropertyId` | The property ID does not match any known asset | Malformed or non-existent property identifier | Verify the property ID against on-chain records |
| 4006 | `traits` (OracleError) | `InvalidValuation` | The submitted valuation is outside allowed bounds | Valuation is zero, negative, or exceeds configured limits | Submit a valuation within the permitted range |
| 4014 | `traits` (OracleError) | `InvalidParameters` | Oracle configuration parameters are invalid | Parameters fail semantic validation | Review parameter constraints and retry |
| 1010 | `property-token` | `ZeroAmount` | Amount must be greater than zero | A zero-value transfer, mint, or burn was attempted | Ensure amounts are positive |
| 1029 | `property-token` | `STAKE_BELOW_MINIMUM` | The stake amount is below the required minimum | Attempted to stake less than the protocol minimum | Increase the stake to meet the minimum threshold |
| 1030 | `property-token` | `INVALID_STAKE_AMOUNT` | Stake amount is invalid for the current operation | Amount does not satisfy validation rules | Use a valid stake amount within the configured range |
| 2006 | `escrow` | `InvalidAmount` | Escrow amount is invalid | Amount is zero, negative, or exceeds limits | Provide a valid escrow amount |
| 2007 | `escrow` | `InvalidStateTransition` | The requested state transition is not allowed | Attempting to release, refund, or dispute from the wrong state | Follow the correct escrow lifecycle flow |
| 2017 | `escrow` | `InvalidSignature` | Provided signature does not match the signer or message | Signature verification failed | Use the correct signing key and message format |
| 2018 | `escrow` | `ExpirationTooFar` | Escrow expiration exceeds the maximum allowed duration | Expiration timestamp is too far in the future | Set expiration within the permitted window |
| 2019 | `escrow` | `FeeRateTooHigh` | Fee rate exceeds the maximum allowed percentage | Fee rate configured above the protocol limit | Reduce fee rate to within allowed bounds |
| 2020 | `escrow` | `InvalidFeeAmount` | The computed fee amount is invalid | Fee calculation produced an unexpected result | Review fee parameters and recalculate |
| 2021 | `escrow` | `InsufficientFeePayment` | The fee payment included with the call is insufficient | Attached fee is less than the required fee amount | Include sufficient fee payment |
| 3003 | `bridge` | `InvalidTransaction` | The bridge transaction data is malformed | Transaction payload does not match expected format | Ensure the transaction is properly encoded |
| 3009 | `bridge` | `InvalidSignature` | Bridge transaction signature is invalid | Signature does not correspond to a known signer | Use a valid signer key and correct signature scheme |
| 3013 | `bridge` | `InvalidNonce` | Transaction nonce is invalid or out of order | Nonce does not match the expected sequence for the sender | Use the correct next nonce value |
| 3015 | `bridge` | `FeeTooHigh` | Bridge fee exceeds the maximum allowed | Fee amount is above the protocol limit | Reduce the fee or wait for lower fee period |
| 5004 | `fees` | `InvalidFeeRate` | Fee rate is outside the allowed range | Rate is below minimum or above maximum | Configure a rate within the permitted range |
| 5006 | `fees` | `FeeExceedsLimit` | The computed fee exceeds the maximum allowed limit | Fee amount breaches the configured cap | Lower the transaction amount or fee rate |
| 6002 | `tax-compliance` | `InvalidRate` | Tax rate value is invalid | Rate is negative, zero, or exceeds maximum | Set a valid tax rate within bounds |
| 7006 | `dex` | `InvalidAmount` / `ZeroAmount` | Amount is zero or invalid for the operation | Zero-amount swap, add, or remove attempted | Provide a positive amount |
| 7010 | `dex` | `InvalidFee` | DEX fee configuration is out of range | Fee rate is below minimum or above maximum | Set fee within the allowed range |
| 7013 | `dex` | `InvalidToken` | Token address or ID is not recognized by the DEX | Token is not in the whitelist or does not exist | Use a supported token |
| 7016 | `dex` | `InvalidRequest` | The DEX request is malformed or contains invalid parameters | Request body or parameters fail validation | Check request format and constraints |
| 8008 | `governance` | `InvalidProposalParameters` | Governance proposal parameters are invalid | Voting period, quorum, or threshold values are out of range | Adjust parameters to meet governance rules |
| 9007 | `staking` | `InvalidUnstakeAmount` / `UnstakeAmountExceedsLocked` | Unstake amount is invalid or exceeds the locked amount | Amount is zero, exceeds the staked balance, or exceeds the unlocked portion | Provide a valid unstake amount ≤ unlocked balance |
| 9013 | `staking` | `MinimumDelegationNotMet` | Delegation amount is below the minimum required | Delegation is less than the protocol minimum | Increase delegation to meet the minimum |
| 9014 | `staking` | `InvalidCommissionRate` | Validator commission rate is outside allowed bounds | Rate is below 0% or above 100% (or protocol limit) | Set commission within the permitted range |
| 10003 | `traits` (MonitoringError) | `InvalidAlertThreshold` | Alert threshold value is invalid | Threshold is out of the valid range | Configure a threshold within the permitted bounds |
| 11003 | `traits` (EventBusError) | `InvalidEventType` | The event type is not registered or recognized | Publisher emitted an unregistered event type | Register the event type before publishing |

---

## 3. State Errors

Errors raised when the contract is in an unexpected state — items are missing, already exist, are paused, locked, or in the wrong lifecycle phase.

### 3.1 Not Found

| Code | Contract | Error Name | Description | Likely Cause | Resolution |
|------|----------|------------|-------------|--------------|------------|
| 2 | `traits` (CommonError) | `NotFound` | Requested entity does not exist | Lookup by ID, address, or index failed | Verify the identifier references an existing entity |
| 1002 | `property-token` | `TokenNotFound` | The specified property token does not exist | Token ID is invalid or has been burned | Verify the token ID |
| 1011 | `property-token` | `PropertyNotFound` | The underlying property record does not exist | Property ID does not match any on-chain property | Verify the property ID |
| 1026 | `property-token` | `STAKE_NOT_FOUND` | No stake record found for the caller | Caller has not staked tokens | Call `stake()` before unstaking or claiming rewards |
| 2002 | `escrow` | `EscrowNotFound` | Escrow with the given ID does not exist | Invalid escrow ID or already finalized | Verify the escrow ID |
| 2012 | `escrow` | `DisputeNotFound` | No dispute record found for this escrow | Dispute has not been raised or has been resolved | Raise a dispute before referencing it |
| 2014 | `escrow` | `SignerNotFound` | The specified multi-sig signer is not registered | Signer address was never added | Add the signer to the escrow's signer set |
| 3006 | `bridge` | `TransactionNotFound` | Bridge transaction with the given hash does not exist | Invalid transaction hash or not yet submitted | Verify the transaction hash |
| 3011 | `bridge` | `SignerNotFound` | The signer is not registered on the bridge | Signer address is not in the signer set | Register the signer |
| 4001 | `traits` (OracleError) | `SourceNotFound` | Oracle data source is not registered | Source ID does not match any registered source | Register the data source first |
| 5002 | `fees` | `FeeNotFound` | Fee configuration for the specified scope does not exist | Fee schedule has not been configured | Configure the fee before querying |
| 5007 | `fees` | `ExemptNotFound` | The fee exemption record does not exist | Exemption was never granted or has been revoked | Grant the exemption first |
| 6002 | `tax-compliance` | `RuleNotFound` | Compliance rule not found | Rule ID does not match any configured rule | Verify the rule ID or configure the rule |
| 6002 | `tax-compliance` | `AssessmentNotFound` | Tax assessment record not found | Assessment ID is invalid | Verify the assessment ID |
| 6002 | `tax-compliance` | `RecordNotFound` | Compliance record not found | Record ID does not exist | Verify the record ID |
| 6002 | `tax-compliance` | `TreatyNotFound` | Tax treaty configuration not found | Treaty ID is not recognized | Configure the treaty or use a valid ID |
| 6004 | `tax-compliance` | `DocumentNotFound` | Tax document not found | Document ID or hash is invalid | Verify the document identifier |
| 6006 | `compliance_registry` | `RegistryRuleNotFound` | Rule is not configured in the registry | Rule ID does not exist in the registry | Add the rule to the registry |
| 6007 | `compliance_registry` | `RegistryProfileNotFound` | Profile is not registered in the registry | Profile ID is invalid | Register the profile |
| 6008 | `compliance_registry` | `RegistryAssessmentNotFound` | Assessment record not found in the registry | Assessment ID is invalid | Verify the assessment ID |
| 6009 | `compliance_registry` | `RegistryRecordNotFound` | Compliance record not found in the registry | Record ID is invalid | Verify the record ID |
| 6010 | `tax-compliance` / `compliance_registry` | `ProfileNotFound` | Compliance profile not found | Profile ID does not exist | Verify the profile ID or create a new profile |
| 6011 | `tax-compliance` / `compliance_registry` | `AdvisorNotFound` | Advisor record not found | Advisor ID is invalid or was removed | Verify the advisor ID |
| 7001 | `dex` | `PairNotFound` | Liquidity pair does not exist | Token pair has not been created | Create the pair first |
| 7007 | `dex` | `OrderNotFound` | The specified order does not exist | Order ID is invalid or has been filled/cancelled | Verify the order ID |
| 8002 | `governance` | `ProposalNotFound` | Governance proposal not found | Proposal ID is invalid | Verify the proposal ID |
| 9004 | `staking` | `StakeNotFound` | No stake record exists for the caller | Caller has not staked tokens | Stake tokens before attempting operations |
| 9016 | `staking` | `ValidatorNotFound` | Validator record not found | Validator address is not registered | Register as a validator first |
| 9019/9004 | `staking` | `DelegationNotFound` | Delegation record not found | Delegator has not delegated to the specified validator | Create a delegation first |
| 9024 | `staking` | `VestingScheduleNotFound` | Vesting schedule not found | Schedule ID does not exist | Verify the schedule ID |

### 3.2 Already Exists

| Code | Contract | Error Name | Description | Likely Cause | Resolution |
|------|----------|------------|-------------|--------------|------------|
| 3 | `traits` (CommonError) | `AlreadyExists` | Entity already exists and cannot be duplicated | Attempted to create a duplicate record | Check if the entity already exists before creation |
| 1007 | `property-token` | `AlreadyFractionalized` | Token is already in a fractionalized state | Fractionalization already active | Unfractionalize first or use the existing fractional state |
| 1008 | `property-token` | `AlreadyInitialized` | Contract or token instance is already initialized | Re-initialization attempted | Do not re-initialize |
| 1028 | `property-token` | `STAKE_ALREADY_EXISTS` | A stake record already exists for the caller | Double-staking attempted without unstaking first | Use the existing stake or unstake first |
| 2008 | `escrow` | `AlreadyApproved` | Escrow has already been approved by this party | Duplicate approval call | No action needed — approval is already recorded |
| 2009 | `escrow` | `AlreadyDisputed` | Escrow is already under dispute | Duplicate dispute call | No action needed — dispute is already active |
| 2015 | `escrow` | `SignerAlreadyExists` | Multi-sig signer is already registered | Duplicate signer registration | Signer is already in the set |
| 3010 | `bridge` | `SignerAlreadyExists` | Bridge signer is already registered | Duplicate signer registration | No action needed — signer is already active |
| 4002 | `traits` (OracleError) | `SourceAlreadyRegistered` | Oracle data source is already registered | Duplicate registration attempt | Source is already active |
| 4013 | `traits` (OracleError) | `AlreadyExists` | Entity already exists in the oracle context | Duplicate creation attempt | Check for existing entities before creation |
| 5003 | `fees` | `FeeAlreadyConfigured` | Fee schedule is already configured for this scope | Duplicate fee configuration | Update the existing configuration instead |
| 5008 | `fees` | `ExemptAlreadyExists` | Fee exemption already exists for this address | Duplicate exemption grant | Exemption is already active |
| 6012 | `tax-compliance` / `compliance_registry` | `AdvisorAlreadyRegistered` | Advisor is already registered | Duplicate advisor registration | Advisor is already in the registry |
| 7002 | `dex` | `PairAlreadyExists` | Liquidity pair already exists | Attempted to create a duplicate pair | Use the existing pair |
| 9003 | `staking` | `StakeAlreadyExists` | Stake record already exists for the caller | Double-staking from the same account | Use the existing stake position |
| 9008 | `staking` | `ValidatorAlreadyRegistered` | Validator is already registered | Duplicate validator registration | Validator is already in the set |
| 9012 | `staking` | `DelegationAlreadyExists` / `DelegationExists` | Delegation to this validator already exists | Duplicate delegation attempted | Use the existing delegation |
| 9015 | `staking` | `AlreadyValidator` | Address is already registered as a validator | Duplicate registration | Already a validator |
| 9018 | `staking` | `AlreadyDelegated` | Already delegated to this validator | Duplicate delegation | Use the existing delegation |
| 9023 | `staking` | `VestingScheduleExists` | A vesting schedule already exists for this address | Duplicate vesting schedule creation | Use the existing schedule |

### 3.3 Lifecycle & Activity State

| Code | Contract | Error Name | Description | Likely Cause | Resolution |
|------|----------|------------|-------------|--------------|------------|
| 8 | `traits` (CommonError) | `Paused` | Contract or feature is currently paused | Admin has paused the contract due to maintenance or emergency | Wait for the contract to be unpaused |
| 1003 | `property-token` | `TokenNotTransferable` | Token transfers are currently disabled | Transfer lock or restriction is active | Wait until transfers are re-enabled |
| 1005 | `property-token` | `FractionalizationInProgress` | A fractionalization operation is already in progress | Concurrent fractionalization is not allowed | Wait for the current operation to complete |
| 1006 | `property-token` | `NotFractionalized` | Token is not in a fractionalized state | Operation requires fractionalization | Fractionalize the token first |
| 1012 | `property-token` | `PropertyNotVerified` | The property has not passed verification | Property is pending or failed verification | Complete property verification |
| 1013 | `property-token` | `PropertyNotActive` | The property is not in an active state | Property is in development, paused, or retired | Ensure property status is active |
| 1014 | `property-token` | `PropertySuspended` | The property has been suspended | Regulatory or compliance suspension active | Resolve the suspension reason |
| 1015 | `property-token` | `TokenPaused` | Token operations are paused | Token-level pause activated by admin | Wait for unpause |
| 1018 | `property-token` | `DistributionInProgress` | A distribution (dividend/rental) is already in progress | Concurrent distribution not allowed | Wait for completion |
| 1019 | `property-token` | `RentalIncomeNotAvailable` | No rental income is available for distribution | No rental income has been collected or it has already been distributed | Wait for rental income to accrue |
| 1020 | `property-token` | `DividendNotAvailable` | No dividends are available for distribution | No dividends declared or already distributed | Wait for the next dividend declaration |
| 1021 | `property-token` | `MaintenanceFeeDue` | Maintenance fees are due before this operation | Outstanding maintenance fees must be paid | Pay the maintenance fee |
| 1022 | `property-token` | `InsuranceRequired` | Insurance must be active before this operation | Property insurance has lapsed or is insufficient | Renew or increase insurance coverage |
| 1027 | `property-token` | `STAKE_LOCK_ACTIVE` | Stake is currently locked and cannot be withdrawn | Locking period has not elapsed | Wait for the lock period to expire |
| 2003 | `escrow` | `EscrowNotActive` | Escrow is not in an active state | Escrow has been released, refunded, or cancelled | Check escrow status |
| 2004 | `escrow` | `EscrowExpired` | Escrow has expired | Expiration deadline has passed | The escrow must be refunded or re-initiated |
| 2010 | `escrow` | `NotDisputed` | Escrow is not currently under dispute | Cannot resolve a dispute that does not exist | Raise a dispute first |
| 2011 | `escrow` | `DisputeResolved` | The dispute has already been resolved | Cannot modify a resolved dispute | No action needed — dispute is closed |
| 2013 | `escrow` | `ConditionNotMet` | The release conditions have not been satisfied | Conditions defined in the escrow are not yet fulfilled | Meet all escrow conditions before release |
| 2016 | `escrow` | `SignerLimitExceeded` | Maximum number of multi-sig signers has been reached | Attempted to add a signer beyond the limit | Remove an existing signer or reduce the signer count |
| 3005 | `bridge` | `BridgePaused` | The bridge is currently paused | Admin pause due to maintenance or security incident | Wait for the bridge to be unpaused |
| 3007 | `bridge` | `TransactionAlreadyProcessed` | Transaction has already been processed | Duplicate processing attempt | No action needed — transaction is complete |
| 3008 | `bridge` | `TransactionExpired` | Bridge transaction has expired | Block height or timestamp deadline passed | Submit a new bridge transaction |
| 3017 | `bridge` | `SourceChainNotVerified` | The source chain has not been verified | Chain ID not in the verified chain list | Verify the source chain through governance |
| 3018 | `bridge` | `MaxPendingTransactionsExceeded` | Too many pending transactions from this sender | Rate limit hit — sender has too many unprocessed transactions | Wait for pending transactions to be processed |
| 4007 | `traits` (OracleError) | `SourceNotActive` | Oracle data source is not active | Source is in inactive status | Activate the source |
| 4008 | `traits` (OracleError) | `SourcePenalized` | Oracle data source is penalized | Source submitted bad data or went offline | Wait for penalty period to expire |
| 4009 | `traits` (OracleError) | `SourceBanned` | Oracle data source is permanently banned | Source violated protocol rules | Source cannot be used — register a new source |
| 4012 | `traits` (OracleError) | `CircuitBreakerActive` | Oracle circuit breaker has been triggered | Rapid price deviation or excessive errors detected | Wait for circuit breaker to cool down |
| 4010 | `traits` (OracleError) | `ConsensusNotReached` | Oracle sources did not reach consensus | Disagreement among data sources | Wait for new data round or review source configuration |
| 4011 | `traits` (OracleError) | `QuorumNotMet` | Insufficient oracle sources reported data | Fewer sources responded than required for quorum | Wait for more sources to submit data |
| 6002 | `tax-compliance` | `InactiveRule` | The compliance rule is not active | Rule is disabled or expired | Activate the rule or use an active rule |
| 7008 | `dex` | `OrderNotActive` | The DEX order is not active | Order has been filled, cancelled, or expired | Place a new order |
| 7011 | `dex` | `PoolNotActive` | The liquidity pool is not active | Pool is disabled or paused | Check pool status |
| 7015 | `dex` | `Locked` | The DEX operation is locked | Reentrancy lock or timelock active | Retry after the lock period |
| 8003 | `governance` | `ProposalNotActive` | Proposal is not in an active voting state | Voting period has not started or has ended | Wait for the voting window |
| 8004 | `governance` | `ProposalAlreadyExecuted` | Proposal has already been executed | Duplicate execution attempt | No action needed — proposal is complete |
| 8005 | `governance` | `ProposalExpired` | Proposal has expired without execution | Deadline passed before execution | Create a new proposal |
| 8006 | `governance` | `DuplicateVote` | Address has already voted on this proposal | Duplicate vote attempt | No action needed — vote is already recorded |
| 8007 | `governance` | `InsufficientVotingPower` | Caller's voting power is below the required threshold | Not enough governance tokens staked | Acquire or delegate more voting power |
| 8009 | `governance` | `QuorumNotMet` | Proposal did not reach quorum | Insufficient total votes cast | Encourage more participation |
| 8011 | `governance` | `VotingNotStarted` | Voting has not started for this proposal | Proposal is still in the queue or pending | Wait for the voting period to begin |
| 8012 | `governance` | `GovernancePaused` | Governance is currently paused | Admin paused governance | Wait for unpause |
| 8013 | `governance` | `InvalidDelegation` | Delegation configuration is invalid | Self-delegation or circular delegation detected | Fix the delegation chain |
| 9001 | `staking` | `StakingInactive` | Staking is currently inactive | Staking disabled by admin | Wait for staking to be re-enabled |
| 9005 | `staking` | `StakingLockActive` / `StakingNoRewards` | Stake is locked or no rewards available | Lock period active, or reward pool is empty | Wait for lock to expire or rewards to accumulate |
| 9010 | `staking` | `ValidatorActive` / `ValidatorInactive` | Validator is in an unexpected state | Operation requires the opposite validator state | Toggle validator status as needed |
| 9017 | `staking` | `ValidatorNotActive` | The validator is not currently active | Validator is inactive, jailed, or unbonding | Choose an active validator |
| 9020 | `staking` | `AlreadyUnbonding` | The stake or delegation is already unbonding | Duplicate unbonding request | No action needed — unbonding is in progress |
| 9021 | `staking` | `UnbondingPeriodActive` | Cannot complete operation during unbonding | Unbonding period has not elapsed | Wait for the unbonding period to complete |
| 9022 | `staking` | `InsufficientValidatorStake` | Validator's total stake is below the minimum threshold | Validator self-stake or total delegated stake too low | Increase stake to meet the minimum |

### 3.4 Balance & Capacity Errors

| Code | Contract | Error Name | Description | Likely Cause | Resolution |
|------|----------|------------|-------------|--------------|------------|
| 9 | `traits` (CommonError) | `InsufficientBalance` | Account balance is insufficient | Balance is lower than the required amount | Deposit additional funds |
| 10 | `traits` (CommonError) | `InsufficientAllowance` | Allowance is insufficient for the transfer | Allowance granted to the spender is too low | Increase the allowance |
| 1004 | `property-token` | `InsufficientBalance` | Token balance is insufficient | Caller does not hold enough tokens | Acquire or transfer additional tokens |
| 1009 | `property-token` | `MaxSupplyExceeded` | Maximum token supply would be exceeded | Minting would exceed the supply cap | Reduce the mint amount or increase cap via governance |
| 1027 | `property-token` | `INSUFFICIENT_STAKE` | Staked balance is below the required amount | Insufficient tokens staked for the operation | Stake additional tokens |
| 1031 | `property-token` | `INSUFFICIENT_REWARDS` | Rewards balance is below the requested withdrawal amount | Reward pool is empty or caller has no accrued rewards | Wait for rewards to accrue |
| 2005 | `escrow` | `InsufficientBalance` | Escrow balance is insufficient for the requested transfer | Escrow holds fewer funds than requested | Fund the escrow or request a lower amount |
| 3004 | `bridge` | `InsufficientBalance` | Bridge account balance is insufficient | Source account lacks funds for the bridge transaction | Add funds to the source account |
| 3014 | `bridge` | `AmountExceedsLimit` | Transfer amount exceeds the bridge limit | Per-transaction or daily limit breached | Reduce the amount or wait for limit reset |
| 4003 | `traits` (OracleError) | `InsufficientStake` | Oracle source stake is below the minimum requirement | Source does not have enough stake | Increase the source's staked amount |
| 5005 | `fees` | `InsufficientPayment` | Payment is less than the required fee amount | Underpayment for the fee | Attach the correct fee payment |
| 7003 | `dex` | `InsufficientLiquidity` | Liquidity pool does not have enough reserves | Pool reserves are below the requested output amount | Reduce the swap amount or add liquidity |
| 7004 | `dex` | `InsufficientOutputAmount` | Expected output amount is below the minimum | Slippage or liquidity caused lower output | Increase slippage tolerance or reduce amount |
| 7009 | `dex` | `InsufficientBalance` | DEX account balance is insufficient | Account lacks tokens for the operation | Ensure sufficient token balance |
| 7014 | `dex` | `PriceImpactTooHigh` | Price impact exceeds the configured maximum | Swap size is too large relative to pool depth | Reduce the swap amount |
| 9002 | `staking` | `InsufficientStake` | Staked amount is below the required minimum | Not enough tokens staked | Increase the stake |
| 9005 | `staking` | `StakingNoRewards` | No rewards available to claim | Reward pool is empty | Wait for rewards to be distributed |
| 9006 | `staking` | `StakingRewardsClaimFailed` / `StakingInsufficientRewards` | Reward claim failed or insufficient rewards | Reward calculation error or empty rewards | Check reward accumulation and retry |
| 9011 | `staking` | `InsufficientDelegation` / `DelegationNotFound` | Delegation is insufficient or not found | Delegator has insufficient delegation or none at all | Increase delegation amount |
| 11004 | `traits` (EventBusError) | `EventLimitExceeded` | Maximum number of events has been reached | Publisher exceeded the event emission quota | Reduce event emission frequency |

---

## 4. System Errors

Errors related to infrastructure, cross-cutting concerns, and external integrations.

| Code | Contract | Error Name | Description | Likely Cause | Resolution |
|------|----------|------------|-------------|--------------|------------|
| 7 | `traits` (CommonError) | `ReentrantCall` | Reentrant call detected and rejected | Contract attempted to re-enter during execution | Ensure non-reentrant execution flow |
| 7016 | `dex` | `ReentrantCall` | Reentrancy guard triggered in the DEX | Concurrent or recursive call detected | Retry the operation sequentially |
| 7016 | `dex` | `TimelockRequired` | Timelock delay has not elapsed | Operation attempted before timelock expiry | Wait for the timelock period to complete |
| 7017 | `dex` | `CrossChainTradeFailed` | Cross-chain DEX trade failed | Bridge or destination chain execution error | Check cross-chain infrastructure and retry |
| 8010 | `governance` | `ExecutionFailed` | Proposal execution failed | On-chain execution reverted | Debug the proposal's underlying transactions |
| 10001 | `traits` (MonitoringError) | `MetricsSubmissionFailed` | Metrics data submission failed | Backend storage or network error | Check monitoring infrastructure |
| 10002 | `traits` (MonitoringError) | `AlertDispatchFailed` | Alert notification could not be dispatched | Downstream notification service error | Check alert provider configuration |
| 10004 | `traits` (MonitoringError) | `StorageExhausted` | Monitoring storage capacity exhausted | Metrics database or log storage is full | Increase storage capacity or rotate logs |
| 11001 | `traits` (EventBusError) | `SubscriptionFailed` | Event subscription failed | Subscriber registration or callback error | Check subscriber configuration |
| 11002 | `traits` (EventBusError) | `EventPublishFailed` | Event could not be published | Event bus internal error | Check event bus status and retry |
| 11005 | `traits` (EventBusError) | `HandlerExecutionFailed` | Event handler execution failed | Handler contract or callback reverted | Debug the event handler |
| 11007 | `traits` (EventBusError) | `EventBusFull` | Event bus message queue is full | Backpressure — too many unprocessed events | Wait for events to be consumed |

---

## 5. Known Bugs & Collisions

### 5.1 Same Code, Different Names — Single Contract

| Contract | Code | Colliding Variants | Impact |
|----------|------|--------------------|--------|
| `property-token` | 1026 | `STAKE_NOT_FOUND` / `SENDER_NOT_VERIFIED` | Both map to the same discriminant — a `match` cannot distinguish them at runtime. |
| `property-token` | 1027 | `STAKE_LOCK_ACTIVE` / `INSUFFICIENT_STAKE` | Same discriminant — different semantics conflated. |
| `property-token` | 1028 | `STAKE_ALREADY_EXISTS` / `CANNOT_UNSTAKE_BELOW_MINIMUM` | Same discriminant — conflicting meanings. |
| `staking` | 9005 | `StakingLockActive` / `StakingNoRewards` | Two unrelated states share code 9005. |
| `staking` | 9006 | `StakingRewardsClaimFailed` / `StakingInsufficientRewards` | Reward failure codified twice under one code. |
| `staking` | 9007 | `InvalidUnstakeAmount` / `UnstakeAmountExceedsLocked` | Related but distinct validation errors. |
| `staking` | 9010 | `ValidatorActive` / `ValidatorInactive` | Direct opposites sharing one code. |
| `staking` | 9011 | `InsufficientDelegation` / `DelegationNotFound` | Different scenarios with the same discriminant. |
| `staking` | 9012 | `DelegationAlreadyExists` / `DelegationExists` | Redundant aliases for the same condition. |
| `staking` | 9019 | `DelegationNotFound` (also 9004) | Duplicate variant across two code points. |
| `dex` | 7006 | `InvalidAmount` / `ZeroAmount` | Both amount validation errors but distinct semantics. |
| `dex` | 7016 | `ReentrantCall` / `InvalidRequest` / `TimelockRequired` | Three entirely different errors mapped to 7016. |

### 5.2 Same Code Range, Different Contracts

| Code | Contract A | Contract B | Notes |
|------|------------|------------|-------|
| 6001–6014 | `tax-compliance` | `compliance_registry` | The two contracts use the same code range and many of the same error names. Errors 6005–6009 are unique to `compliance_registry`; the rest overlap. This is intentional — both contracts are part of the compliance subsystem — but callers must check the contract address to disambiguate. |

---

## 6. Contracts Without Numeric `error_code()`

The following contracts define error enums but do **not** implement `error_code()` (or `fn error_code() -> u64`). They cannot be reliably matched by numeric code in cross-contract calls. Adding a numeric code to each variant is recommended for consistency.

| Contract | Enum | Variants | Recommendation |
|----------|------|----------|----------------|
| `insurance` | `InsuranceError` | 39 variants | Assign codes starting at 12001 |
| `crowdfunding` | `CrowdfundingError` | 17 variants | Assign codes starting at 13001 |
| `fractional` | `FractionalError` | 15 variants | Assign codes starting at 14001 |
| `identity` | `IdentityError` | 14 variants | Assign codes starting at 15001 |
| `lending` | `LendingError` | 21 variants | Assign codes starting at 16001 |
| `analytics` | `AnalyticsError` | 7 variants | Assign codes starting at 17001 |
| `ai-valuation` | `AIValuationError` | 12 variants | Assign codes starting at 18001 |
| `factory` | `Error` | 6 variants | Assign codes starting at 19001 |
| `ipfs-metadata` | `Error` | 15 variants | Assign codes starting at 20001 |
| `prediction-market` | `Error` | 12 variants | Assign codes starting at 21001 |
| `property-management` | `Error` | 18 variants | Assign codes starting at 22001 |
| `zk-compliance` | `Error` | 10 variants | Assign codes starting at 23001 |
| `database` | `Error` | 8 variants | Assign codes starting at 24001 |
| `proxy` | `Error` | 18 variants | Assign codes starting at 25001 |
| `metadata` | `Error` | 15 variants | Assign codes starting at 26001 |
| `third-party` | `Error` | 10 variants | Assign codes starting at 27001 |
| `lib/src/lib.rs` | `Error` | 27 variants | Assign codes starting at 28001 |
| `rental_income` (Soroban) | `Error` | 2 variants (`ZeroAmount=1`, `NoIncomeAvailable=2`) | Already has small numeric codes — move to reserved range to avoid collisions |

### 6.1 Total Uncodified Variants

| Metric | Count |
|--------|-------|
| Contracts missing `error_code()` | 18 |
| Total uncodified error variants | ~282 |

> **Action Item:** These enums should implement `error_code()` to enable numeric matching in cross-contract calls and error-propagation logic. See `ADR-007` in `docs/adr/` for the standard.

---

## 7. Reserved Ranges

| Range | Allocation | Status |
|-------|-----------|--------|
| 1–999 | Reserved — trait-level common errors | 1–10 used by `CommonError` |
| 1000–1999 | `property-token` | 1001–1035 used |
| 2000–2999 | `escrow` | 2001–2021 used |
| 3000–3999 | `bridge` | 3001–3018 used |
| 4000–4999 | Reserved — `OracleError` | 4001–4014 used |
| 5000–5999 | `fees` | 5001–5008 used |
| 6000–6999 | Shared — `tax-compliance` / `compliance_registry` | 6001–6014 used |
| 7000–7999 | `dex` | 7001–7019 used |
| 8000–8999 | `governance` | 8001–8013 used |
| 9000–9999 | `staking` | 9001–9024 used |
| 10000–10999 | Reserved — `MonitoringError` | 10001–10005 used |
| 11000–11999 | Reserved — `EventBusError` | 11001–11007 used |
| 12000+ | Available for uncodified contracts (see §6) | Unassigned |

---

*Generated for Issue #524. Update this document whenever new error variants or contracts are added.*
