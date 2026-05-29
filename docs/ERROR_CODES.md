# PropChain Error Codes Reference

All PropChain contracts use a unified numeric error code system. Codes are globally unique across the entire system and are grouped by contract domain.

## Code Ranges

| Range | Domain | Contract |
|-------|--------|----------|
| 1–10 | Common | All contracts |
| 1001–1025 | PropertyToken | `property-token` |
| 2001–2013 | Escrow | `escrow` |
| 3001–3013 | Bridge | `bridge` |
| 4001–4012 | Oracle | `oracle` |
| 5001–5008 | Fees | `fees` |
| 6001–6013 | Compliance | `compliance_registry`, `tax-compliance` |
| 7001–7015 | DEX | `dex` |
| 8001–8013 | Governance | `governance` |
| 9001–9010 | Staking | `staking` |
| 10001–10005 | Monitoring | `monitoring` |
| 11001–11006 | EventBus | `event_bus` |
| — | (no numeric code) | `insurance`, `proxy`, `database`, `metadata`, `third-party`, `lending`, `crowdfunding`, `identity`, `prediction-market`, `zk-compliance`, `ai-valuation`, `property-management`, `ipfs-metadata`, `access_control`, `crypto`, `di` |

Errors without a numeric code are returned as typed Rust enums and are identified by variant name in client SDKs.

---

## Common Errors (1–10)

Shared across all contracts via `CommonError`.

| Code | Variant | Meaning | Recovery |
|------|---------|---------|----------|
| 1 | `Unauthorized` | Caller lacks required permissions | Check your role/account; request access from admin |
| 2 | `InvalidParameters` | One or more function parameters are invalid | Review parameter constraints and resubmit |
| 3 | `NotFound` | Requested resource does not exist | Verify the ID/key exists before calling |
| 4 | `InsufficientFunds` | Account balance too low for the operation | Top up balance and retry |
| 5 | `InvalidState` | Operation not allowed in the current contract state | Wait for state to change or check preconditions |
| 6 | `InternalError` | Unexpected internal contract error | Report to contract admin; may require upgrade |
| 7 | `CodecError` | SCALE encode/decode failure | Ensure data is correctly serialized |
| 8 | `NotImplemented` | Feature not yet available | Check roadmap; use an alternative path |
| 9 | `Timeout` | Operation exceeded its time limit | Retry; check network/block conditions |
| 10 | `Duplicate` | Resource or operation already exists | Check for existing record before creating |

---

## PropertyToken Errors (1001–1025)

Contract: `contracts/property-token`

| Code | Variant | Meaning | Recovery |
|------|---------|---------|----------|
| 1001 | `TokenNotFound` | Token ID does not exist | Verify token was minted; check token ID |
| 1002 | `Unauthorized` | Caller is not the token owner or approved operator | Use the owner account or obtain approval |
| 1003 | `PropertyNotFound` | Property record does not exist | Register the property first |
| 1004 | `InvalidMetadata` | Metadata is malformed or missing required fields | Validate all metadata fields before submitting |
| 1005 | `DocumentNotFound` | Referenced document does not exist | Upload the document before referencing it |
| 1006 | `ComplianceFailed` | Compliance check rejected the operation | Ensure KYC/AML verification is current |
| 1007 | `BridgeNotSupported` | This token cannot be bridged | Check bridge eligibility for the token type |
| 1008 | `InvalidChain` | Destination chain ID is not recognized | Use a supported chain ID |
| 1009 | `BridgeLocked` | Token is locked in an active bridge operation | Wait for the bridge operation to complete or expire |
| 1010 | `InsufficientSignatures` | Not enough multi-sig approvals collected | Gather required signatures from authorized signers |
| 1011 | `RequestExpired` | Bridge request TTL has elapsed | Create a new bridge request |
| 1012 | `InvalidRequest` | Bridge request is malformed | Check request parameters and resubmit |
| 1013 | `BridgePaused` | Bridge is temporarily suspended | Wait for bridge to resume; monitor governance |
| 1014 | `GasLimitExceeded` | Operation exceeded the gas budget | Reduce batch size or simplify the call |
| 1015 | `MetadataCorruption` | Token metadata integrity check failed | Contact admin; metadata may need re-anchoring |
| 1016 | `InvalidBridgeOperator` | Signer is not a registered bridge operator | Use an authorized bridge operator account |
| 1017 | `DuplicateBridgeRequest` | Identical bridge request already exists | Check pending requests before submitting |
| 1018 | `BridgeTimeout` | Bridge operation timed out | Retry; check relayer status |
| 1019 | `AlreadySigned` | Caller already signed this bridge request | No action needed; wait for other signers |
| 1020 | `InsufficientBalance` | Account balance too low | Add funds and retry |
| 1021 | `InvalidAmount` | Amount is zero, negative, or out of range | Provide a valid positive amount |
| 1022 | `ProposalNotFound` | Governance proposal does not exist | Verify proposal ID |
| 1023 | `ProposalClosed` | Proposal is no longer accepting votes | Check proposal status before voting |
| 1024 | `AskNotFound` | Sell ask does not exist | Verify ask ID on the marketplace |
| 1025 | `BatchSizeExceeded` | Input array exceeds the maximum batch size | Split into smaller batches |

> `LengthMismatch` (token IDs and amounts arrays differ in length) maps to code 1025 as a secondary variant.

---

## Escrow Errors (2001–2013)

Contract: `contracts/escrow`

| Code | Variant | Meaning | Recovery |
|------|---------|---------|----------|
| 2001 | `EscrowNotFound` | Escrow ID does not exist | Verify escrow was created; check ID |
| 2002 | `Unauthorized` | Caller is not a participant or admin | Use an authorized participant account |
| 2003 | `InvalidStatus` | Escrow is not in the required state for this operation | Check escrow status before calling |
| 2004 | `InsufficientFunds` | Escrow balance is too low | Fund the escrow to the required amount |
| 2005 | `ConditionsNotMet` | Release conditions have not been satisfied | Fulfill all conditions before releasing |
| 2006 | `SignatureThresholdNotMet` | Not enough participants have signed | Collect required signatures |
| 2007 | `AlreadySigned` | Caller already signed this escrow action | Wait for other participants |
| 2008 | `DocumentNotFound` | Required document is missing from escrow | Upload the document before proceeding |
| 2009 | `DisputeActive` | An open dispute blocks this operation | Resolve the dispute first |
| 2010 | `TimeLockActive` | Time lock period has not yet expired | Wait until the lock period ends |
| 2011 | `InvalidConfiguration` | Escrow configuration parameters are invalid | Review and correct configuration |
| 2012 | `EscrowAlreadyFunded` | Escrow has already received funds | Do not double-fund; check escrow state |
| 2013 | `ParticipantNotFound` | Specified account is not a registered participant | Add the participant before referencing them |

---

## Bridge Errors (3001–3013)

Contract: `contracts/bridge`

| Code | Variant | Meaning | Recovery |
|------|---------|---------|----------|
| 3001 | `Unauthorized` | Caller is not an authorized bridge operator | Use a registered bridge operator account |
| 3002 | `TokenNotFound` | Token does not exist on this chain | Verify token ID and chain |
| 3003 | `InvalidChain` | Destination chain is not supported | Use a supported chain ID |
| 3004 | `BridgeNotSupported` | Token type cannot be bridged | Check bridge eligibility |
| 3005 | `InsufficientSignatures` | Multi-sig threshold not reached | Collect required operator signatures |
| 3006 | `RequestExpired` | Bridge request has passed its deadline | Submit a new request |
| 3007 | `AlreadySigned` | Operator already signed this request | No action; wait for remaining signers |
| 3008 | `InvalidRequest` | Request payload is malformed | Validate request fields and resubmit |
| 3009 | `BridgePaused` | Bridge operations are suspended | Monitor governance for resume announcement |
| 3010 | `InvalidMetadata` | Token metadata is invalid for bridging | Fix metadata before initiating bridge |
| 3011 | `DuplicateRequest` | Identical request already pending | Check pending requests; do not duplicate |
| 3012 | `GasLimitExceeded` | Operation exceeded gas budget | Reduce payload size |
| 3013 | `RateLimitExceeded` | Daily bridge rate limit reached | Wait for the rate limit window to reset |

---

## Oracle Errors (4001–4012)

Contract: `contracts/oracle`

| Code | Variant | Meaning | Recovery |
|------|---------|---------|----------|
| 4001 | `PropertyNotFound` | Property has no oracle record | Register the property with the oracle first |
| 4002 | `InsufficientSources` | Not enough oracle sources to produce a valuation | Add more oracle sources or wait for existing ones |
| 4003 | `InvalidValuation` | Valuation data is out of acceptable range | Check source data quality |
| 4004 | `Unauthorized` | Caller is not an authorized oracle operator | Use a registered oracle account |
| 4005 | `OracleSourceNotFound` | Referenced oracle source does not exist | Register the source before using it |
| 4006 | `InvalidParameters` | Request parameters are invalid | Review parameter constraints |
| 4007 | `PriceFeedError` | External price feed returned an error | Check feed availability; retry later |
| 4008 | `AlertNotFound` | Price alert does not exist | Verify alert ID |
| 4009 | `InsufficientReputation` | Oracle source reputation score too low | Improve source reputation before submitting |
| 4010 | `SourceAlreadyExists` | Oracle source is already registered | Do not re-register; update existing source |
| 4011 | `RequestPending` | A valuation request is already in progress | Wait for the pending request to resolve |
| 4012 | `BatchSizeExceeded` | Batch request exceeds the configured maximum | Split into smaller batches |

---

## Fee Errors (5001–5008)

Contract: `contracts/fees`

| Code | Variant | Meaning | Recovery |
|------|---------|---------|----------|
| 5001 | `Unauthorized` | Caller lacks fee admin permissions | Use an authorized admin account |
| 5002 | `AuctionNotFound` | Fee auction does not exist | Verify auction ID |
| 5003 | `AuctionEnded` | Auction has already closed | No further bids accepted |
| 5004 | `AuctionNotEnded` | Auction is still active | Wait for auction end time before settling |
| 5005 | `BidTooLow` | Bid is below the current minimum | Increase bid amount above current highest |
| 5006 | `AlreadySettled` | Auction has already been settled | No further action needed |
| 5007 | `InvalidConfig` | Fee configuration parameters are invalid | Review and correct fee config |
| 5008 | `InvalidProperty` | Property ID is invalid or unregistered | Verify property exists before creating auction |

---

## Compliance Errors (6001–6013)

Contracts: `contracts/compliance_registry`, `contracts/tax-compliance`

| Code | Variant | Meaning | Recovery |
|------|---------|---------|----------|
| 6001 | `Unauthorized` | Caller lacks compliance admin role | Use an authorized compliance officer account |
| 6002 | `NotVerified` | User has not completed KYC/AML verification | Complete identity verification process |
| 6003 | `CheckFailed` / `RuleNotFound` / `AssessmentNotFound` / `RecordNotFound` / `InactiveRule` | Compliance check failed or record missing | Review compliance status; re-submit required documents |
| 6004 | `DocumentMissing` | Required compliance document not uploaded | Upload all required documents |
| 6005 | `Expired` / `VerificationExpired` | Compliance verification has expired | Renew KYC/AML verification |
| 6006 | `HighRisk` | User risk score exceeds allowed threshold | Reduce risk profile or contact compliance team |
| 6007 | `ProhibitedJurisdiction` | User's jurisdiction is not permitted | Operations not available in this jurisdiction |
| 6008 | `AlreadyVerified` | User is already verified | No action needed |
| 6009 | `ConsentNotGiven` | Required data processing consent not recorded | Provide consent before proceeding |
| 6010 | `InvalidRiskScore` | Risk score value is out of valid range | Provide a score within the accepted range |
| 6011 | `JurisdictionNotSupported` | Jurisdiction is not in the supported list | Check supported jurisdictions list |
| 6012 | `InvalidDocumentType` | Document type is not accepted | Use a supported document type |
| 6013 | `DataRetentionExpired` | Stored compliance data has passed retention period | Re-submit compliance data |

---

## DEX Errors (7001–7015)

Contract: `contracts/dex`

| Code | Variant | Meaning | Recovery |
|------|---------|---------|----------|
| 7001 | `Unauthorized` | Caller lacks DEX operator permissions | Use an authorized account |
| 7002 | `InvalidPair` | Trading pair is invalid or inactive | Check supported trading pairs |
| 7003 | `PoolNotFound` | Liquidity pool does not exist | Create the pool or use an existing one |
| 7004 | `InsufficientLiquidity` | Pool does not have enough liquidity | Add liquidity or reduce trade size |
| 7005 | `SlippageExceeded` | Trade output is below the slippage tolerance | Increase slippage tolerance or reduce trade size |
| 7006 | `OrderNotFound` | Order ID does not exist | Verify order ID |
| 7007 | `InvalidOrder` | Order parameters are invalid | Review order constraints and resubmit |
| 7008 | `OrderNotExecutable` | Order conditions are not currently satisfied | Wait for market conditions to match |
| 7009 | `RewardUnavailable` | No rewards available to claim | Check reward accrual period |
| 7010 | `ProposalNotFound` | DEX governance proposal does not exist | Verify proposal ID |
| 7011 | `ProposalClosed` | Proposal is no longer accepting changes | Check proposal status |
| 7012 | `AlreadyVoted` | Caller already voted on this proposal | No further action needed |
| 7013 | `InvalidBridgeRoute` | Cross-chain route is not supported | Use a supported bridge route |
| 7014 | `CrossChainTradeNotFound` | Cross-chain trade record does not exist | Verify trade ID |
| 7015 | `InsufficientGovernanceBalance` | Caller does not hold enough governance tokens | Acquire governance tokens before voting |

---

## Governance Errors (8001–8013)

Contract: `contracts/governance`

| Code | Variant | Meaning | Recovery |
|------|---------|---------|----------|
| 8001 | `Unauthorized` | Caller lacks governance permissions | Use a registered signer account |
| 8002 | `ProposalNotFound` | Proposal ID does not exist | Verify proposal ID |
| 8003 | `AlreadyVoted` | Caller already voted on this proposal | No further action needed |
| 8004 | `ProposalClosed` | Proposal is no longer accepting votes | Check proposal status |
| 8005 | `ThresholdNotMet` | Approval threshold not reached | Gather more votes |
| 8006 | `TimelockActive` | Timelock period has not elapsed | Wait for timelock to expire |
| 8007 | `InvalidThreshold` | Threshold value is out of valid range | Set threshold between 1 and signer count |
| 8008 | `SignerExists` | Account is already a registered signer | Do not re-add existing signers |
| 8009 | `SignerNotFound` | Account is not a registered signer | Register the account as a signer first |
| 8010 | `MinSigners` | Removing signer would go below minimum | Add another signer before removing |
| 8011 | `MaxProposals` | Active proposal limit reached | Wait for existing proposals to close |
| 8012 | `NotASigner` | Only signers can perform this action | Use a registered signer account |
| 8013 | `ProposalExpired` | Proposal voting period has ended | Create a new proposal |

---

## Staking Errors (9001–9010)

Contract: `contracts/staking`

| Code | Variant | Meaning | Recovery |
|------|---------|---------|----------|
| 9001 | `Unauthorized` | Caller lacks staking permissions | Use an authorized account |
| 9002 | `InsufficientAmount` | Stake amount is below the minimum | Increase stake to meet the minimum threshold |
| 9003 | `StakeNotFound` | No active stake found for this account | Stake tokens before calling stake operations |
| 9004 | `LockActive` | Lock period has not expired | Wait until the lock period ends |
| 9005 | `NoRewards` | No pending rewards to claim | Wait for rewards to accrue |
| 9006 | `InsufficientPool` | Reward pool has insufficient funds | Contact admin; pool may need replenishment |
| 9007 | `InvalidConfig` | Staking configuration parameters are invalid | Review and correct configuration |
| 9008 | `AlreadyStaked` | Account already has an active stake | Unstake first or use a different account |
| 9009 | `InvalidDelegate` | Delegation target address is invalid | Use a valid account for delegation |
| 9010 | `ZeroAmount` | Amount must be greater than zero | Provide a positive non-zero amount |

---

## Monitoring Errors (10001–10005)

Contract: `contracts/monitoring`

| Code | Variant | Meaning | Recovery |
|------|---------|---------|----------|
| 10001 | `Unauthorized` | Caller is not an authorized reporter or admin | Use a registered reporter account |
| 10002 | `ContractPaused` | Monitoring contract is paused | Wait for admin to resume the contract |
| 10003 | `InvalidThreshold` | Alert threshold value is out of valid range | Provide a threshold within the accepted range |
| 10004 | `SubscriberLimitReached` | Maximum number of alert subscribers reached | Remove an existing subscriber before adding |
| 10005 | `SubscriberNotFound` | Subscriber account is not registered | Register the subscriber before managing it |

---

## EventBus Errors (11001–11006)

Contract: `contracts/event_bus`

| Code | Variant | Meaning | Recovery |
|------|---------|---------|----------|
| 11001 | `Unauthorized` | Caller is not authorized to publish or manage topics | Use an authorized publisher account |
| 11002 | `TopicNotFound` | Topic does not exist | Register the topic before subscribing or publishing |
| 11003 | `AlreadySubscribed` | Caller is already subscribed to this topic | No action needed |
| 11004 | `NotSubscribed` | Caller is not subscribed to this topic | Subscribe before attempting to unsubscribe |
| 11005 | `MaxSubscribersReached` | Topic has reached its subscriber limit | Remove an existing subscriber first |
| 11006 | `SubscriberCallFailed` | Callback to a subscriber contract failed | Check subscriber contract implementation |

---

## Insurance Errors (no numeric code)

Contract: `contracts/insurance` — `InsuranceError`

| Variant | Meaning | Recovery |
|---------|---------|----------|
| `Unauthorized` | Caller lacks insurance admin or policyholder permissions | Use an authorized account |
| `PolicyNotFound` | Policy ID does not exist | Verify policy was created |
| `ClaimNotFound` | Claim ID does not exist | Verify claim was submitted |
| `PoolNotFound` | Insurance pool does not exist | Verify pool ID |
| `PolicyAlreadyActive` | Policy is already in active state | No action needed |
| `PolicyExpired` | Policy coverage period has ended | Renew the policy |
| `PolicyInactive` | Policy is not active | Activate the policy before filing claims |
| `InsufficientPremium` | Premium payment is below the required amount | Pay the full premium amount |
| `InsufficientPoolFunds` | Pool does not have enough funds to pay the claim | Contact admin; pool may need replenishment |
| `ClaimAlreadyProcessed` | Claim has already been settled | No further action needed |
| `ClaimExceedsCoverage` | Claim amount exceeds the policy coverage limit | Claim only up to the coverage limit |
| `InvalidParameters` | Request parameters are invalid | Review and correct parameters |
| `OracleVerificationFailed` | Oracle could not verify the claim event | Provide verifiable on-chain evidence |
| `ReinsuranceCapacityExceeded` | Reinsurance capacity limit reached | Contact admin |
| `TokenNotFound` | Referenced token does not exist | Verify token ID |
| `TransferFailed` | Premium or payout transfer failed | Check balances and retry |
| `CooldownPeriodActive` | Action blocked by cooldown period | Wait for cooldown to expire |
| `PropertyNotInsurable` | Property does not meet insurability criteria | Review property eligibility requirements |
| `DuplicateClaim` | Identical claim already submitted | Check existing claims before submitting |

---

## Proxy Errors (no numeric code)

Contract: `contracts/proxy` — `Error`

| Variant | Meaning | Recovery |
|---------|---------|----------|
| `Unauthorized` | Caller lacks upgrade or governance permissions | Use an authorized governor account |
| `UpgradeFailed` | Contract upgrade execution failed | Check new implementation compatibility |
| `ProposalNotFound` | Upgrade proposal does not exist | Verify proposal ID |
| `ProposalAlreadyExists` | Identical upgrade proposal already pending | Check existing proposals |
| `TimelockNotExpired` | Timelock period has not elapsed | Wait for timelock to expire |
| `InsufficientApprovals` | Not enough governor approvals collected | Gather required approvals |
| `AlreadyApproved` | Caller already approved this proposal | No further action needed |
| `NoPreviousVersion` | No previous version available to roll back to | Cannot roll back on first deployment |
| `IncompatibleVersion` | New implementation is incompatible with current storage | Ensure storage layout compatibility |
| `MigrationInProgress` | A migration is currently running | Wait for migration to complete |
| `NotGovernor` | Caller is not a registered governor | Use a registered governor account |
| `ProposalCancelled` | Proposal has been cancelled | Create a new proposal |
| `EmergencyPauseActive` | Emergency pause is blocking upgrades | Resolve the emergency before upgrading |
| `InvalidTimelockPeriod` | Timelock duration is out of valid range | Set a valid timelock duration |

---

## Database Errors (no numeric code)

Contract: `contracts/database` — `Error`

| Variant | Meaning | Recovery |
|---------|---------|----------|
| `Unauthorized` | Caller lacks database admin permissions | Use an authorized admin account |
| `SyncNotFound` | Sync job does not exist | Verify sync job ID |
| `ExportNotFound` | Export job does not exist | Verify export job ID |
| `InvalidDataRange` | Requested data range is invalid | Provide a valid block/time range |
| `IndexerNotFound` | Indexer is not registered | Register the indexer before using it |
| `IndexerAlreadyRegistered` | Indexer is already registered | Do not re-register existing indexers |
| `InvalidChecksum` | Data checksum verification failed | Re-export or re-sync the data |
| `SnapshotNotFound` | Snapshot does not exist | Verify snapshot ID |

---

## Metadata Errors (no numeric code)

Contract: `contracts/metadata` — `Error`

| Variant | Meaning | Recovery |
|---------|---------|----------|
| `PropertyNotFound` | Property does not exist | Register the property first |
| `Unauthorized` | Caller lacks metadata write permissions | Use an authorized account |
| `InvalidMetadata` | Metadata is malformed or fails validation | Fix all required fields and resubmit |
| `MetadataAlreadyFinalized` | Metadata has been locked and cannot be changed | No further edits allowed after finalization |
| `InvalidIpfsCid` | IPFS CID format is invalid | Provide a valid CIDv0 or CIDv1 string |
| `DocumentNotFound` | Document does not exist | Verify document ID |
| `DocumentAlreadyExists` | Document with this ID already exists | Use a unique document ID |
| `VersionConflict` | Submitted version conflicts with current version | Fetch latest version and rebase changes |
| `RequiredFieldMissing` | A mandatory metadata field is absent | Provide all required fields |
| `SizeLimitExceeded` | Metadata payload exceeds the size limit | Reduce payload size |
| `InvalidContentHash` | Content hash does not match stored data | Re-upload the document |
| `SearchQueryTooLong` | Search query string exceeds maximum length | Shorten the query |

---

## Third-Party Errors (no numeric code)

Contract: `contracts/third-party` — `Error`

| Variant | Meaning | Recovery |
|---------|---------|----------|
| `Unauthorized` | Caller lacks third-party admin permissions | Use an authorized account |
| `ServiceNotFound` | Third-party service does not exist | Register the service before using it |
| `ServiceInactive` | Service is registered but not active | Activate the service first |
| `RequestNotFound` | Service request does not exist | Verify request ID |
| `InvalidStatusTransition` | Requested status change is not allowed | Check valid status transitions |
| `InvalidFeePercentage` | Fee percentage is out of valid range | Provide a value between 0 and 100 |
| `KycExpired` | Third-party KYC verification has expired | Renew KYC verification |
| `PaymentProcessingFailed` | Payment to third-party service failed | Check balances and retry |

---

## Lending Errors (no numeric code)

Contract: `contracts/lending` — `LendingError`

| Variant | Meaning | Recovery |
|---------|---------|----------|
| `Unauthorized` | Caller lacks lending permissions | Use an authorized account |
| `PropertyNotFound` | Property used as collateral does not exist | Register the property first |
| `InsufficientCollateral` | Collateral value is below the required LTV ratio | Add more collateral or reduce loan amount |
| `LoanNotFound` | Loan ID does not exist | Verify loan ID |
| `PoolNotFound` | Lending pool does not exist | Verify pool ID |
| `InsufficientLiquidity` | Pool does not have enough funds | Reduce borrow amount or wait for liquidity |
| `PositionNotFound` | Margin position does not exist | Verify position ID |
| `LiquidationThresholdNotMet` | Position is not yet eligible for liquidation | Wait until threshold is breached |
| `InvalidParameters` | Request parameters are invalid | Review and correct parameters |
| `ProposalNotFound` | Governance proposal does not exist | Verify proposal ID |
| `InsufficientVotes` | Not enough votes to execute proposal | Gather more votes |

---

## Crowdfunding Errors (no numeric code)

Contract: `contracts/crowdfunding` — `CrowdfundingError`

| Variant | Meaning | Recovery |
|---------|---------|----------|
| `Unauthorized` | Caller lacks campaign admin permissions | Use an authorized account |
| `CampaignNotFound` | Campaign does not exist | Verify campaign ID |
| `CampaignNotActive` | Campaign is not in active state | Check campaign status |
| `InsufficientFunds` | Investment amount is below the minimum | Increase investment amount |
| `MilestoneNotFound` | Milestone does not exist | Verify milestone ID |
| `MilestoneNotApproved` | Milestone has not been approved for release | Get milestone approved first |
| `InvestorNotCompliant` | Investor has not passed compliance checks | Complete KYC/AML verification |
| `InsufficientShares` | Not enough shares available | Reduce share request or wait for availability |
| `ListingNotFound` | Secondary market listing does not exist | Verify listing ID |
| `ProposalNotFound` | Governance proposal does not exist | Verify proposal ID |
| `ProposalNotActive` | Proposal is not in active voting state | Check proposal status |
| `InvalidParameters` | Request parameters are invalid | Review and correct parameters |
| `AlreadyVoted` | Caller already voted on this proposal | No further action needed |

---

## Identity Errors (no numeric code)

Contract: `contracts/identity` — `IdentityError`

| Variant | Meaning | Recovery |
|---------|---------|----------|
| `IdentityNotFound` | DID/identity record does not exist | Register identity before using it |
| `Unauthorized` | Caller lacks identity management permissions | Use the identity owner account |
| `InvalidSignature` | Cryptographic signature verification failed | Re-sign with the correct private key |
| `VerificationFailed` | Identity verification process failed | Re-submit verification with valid credentials |
| `InsufficientReputation` | Reputation score is below the required threshold | Build reputation through verified transactions |
| `RecoveryInProgress` | A social recovery process is already active | Wait for current recovery to complete or cancel it |
| `RecoveryNotActive` | No active recovery process exists | Initiate recovery before managing it |
| `InvalidRecoveryParams` | Recovery parameters are invalid | Review recovery configuration |
| `IdentityAlreadyExists` | Identity is already registered for this account | Use the existing identity |
| `InvalidDid` | DID string format is invalid | Provide a valid DID format |
| `RecoveryThresholdNotMet` | Not enough guardians approved the recovery | Gather required guardian approvals |
| `PrivacyVerificationFailed` | Zero-knowledge privacy proof failed | Re-generate and submit a valid proof |
| `UnsupportedChain` | Target chain is not supported for cross-chain identity | Use a supported chain |
| `CrossChainVerificationFailed` | Cross-chain identity verification failed | Check cross-chain bridge status and retry |

---

## Prediction Market Errors (no numeric code)

Contract: `contracts/prediction-market` — `Error`

| Variant | Meaning | Recovery |
|---------|---------|----------|
| `Unauthorized` | Caller lacks market admin permissions | Use an authorized account |
| `MarketNotFound` | Prediction market does not exist | Verify market ID |
| `MarketNotActive` | Market is not in active state | Check market status |
| `MarketNotReadyForResolution` | Resolution conditions not yet met | Wait for resolution time or oracle data |
| `MarketAlreadyResolved` | Market has already been resolved | No further action needed |
| `StakeNotFound` | Stake record does not exist | Verify stake was placed |
| `RewardAlreadyClaimed` | Reward has already been claimed | No further action needed |
| `InvalidAmount` | Stake amount is invalid | Provide a valid positive amount |
| `OracleNotSet` | Oracle contract address has not been configured | Admin must configure the oracle address |
| `TransferFailed` | Token transfer failed | Check balances and retry |
| `LoserCannotClaim` | Caller backed the losing outcome | Only winners can claim rewards |

---

## ZK Compliance Errors (no numeric code)

Contract: `contracts/zk-compliance` — `Error`

| Variant | Meaning | Recovery |
|---------|---------|----------|
| `NotAuthorized` | Caller lacks ZK compliance permissions | Use an authorized account |
| `ProofNotFound` | ZK proof record does not exist | Submit a proof before referencing it |
| `InvalidProof` | Proof data is malformed or fails verification | Re-generate a valid ZK proof |
| `VerificationFailed` | ZK proof verification failed | Check proof inputs and re-generate |
| `ExpiredProof` | Proof has passed its validity window | Submit a fresh proof |
| `AlreadyVerified` | Account is already ZK-verified | No further action needed |
| `InvalidInputs` | Proof inputs are invalid | Review circuit inputs and resubmit |
| `PrivacyControlsViolation` | Operation would violate privacy settings | Adjust privacy level or use a different operation |
| `StatsNotAvailable` | Aggregate statistics are not yet available | Wait for sufficient data to accumulate |
| `InvalidPrivacyLevel` | Privacy level value is out of valid range | Use a supported privacy level |

---

## AI Valuation Errors (no numeric code)

Contract: `contracts/ai-valuation` — `AIValuationError`

| Variant | Meaning | Recovery |
|---------|---------|----------|
| `Unauthorized` | Caller lacks AI valuation admin permissions | Use an authorized account |
| `ModelNotFound` | ML model does not exist | Register the model before using it |
| `PropertyNotFound` | Property has no feature data | Extract features for the property first |
| `InvalidModel` | Model configuration is invalid | Review model parameters |
| `InsufficientData` | Not enough training data for the model | Provide more training samples |
| `LowConfidence` | Prediction confidence is below the minimum threshold | Use more data or a better-trained model |
| `BiasDetected` | Model bias exceeds the configured threshold | Retrain model with balanced data |
| `ContractPaused` | AI valuation contract is paused | Wait for admin to resume |
| `OracleNotSet` | Oracle contract address not configured | Admin must set the oracle address |
| `PropertyRegistryNotSet` | Property registry address not configured | Admin must set the registry address |
| `FeatureExtractionFailed` | Could not extract features from property data | Check property data completeness |
| `PredictionFailed` | Model inference failed | Check model health and retry |
| `InvalidParameters` | Request parameters are invalid | Review and correct parameters |

### AI Valuation — Rate Limit Error

| Variant | Meaning | Recovery |
|---------|---------|----------|
| `RateLimitExceeded` | Per-user or global request rate limit reached | Wait for the rate limit window to reset |

### AI Valuation — Reentrancy Error

| Variant | Meaning | Recovery |
|---------|---------|----------|
| `ReentrantCall` | Reentrant call detected and blocked | Do not call this function recursively |

---

## Property Management Errors (no numeric code)

Contract: `contracts/property-management` — `Error`

| Variant | Meaning | Recovery |
|---------|---------|----------|
| `Unauthorized` | Caller lacks property management permissions | Use an authorized landlord or manager account |
| `NotFound` | Requested record does not exist | Verify the ID |
| `InvalidAmount` | Amount is invalid | Provide a valid positive amount |
| `LeaseNotActive` | Lease is not in active state | Check lease status |
| `NotTenant` | Caller is not the tenant on this lease | Use the tenant account |
| `NotLandlordOrManager` | Caller is not the landlord or property manager | Use an authorized account |
| `InvalidFee` | Fee amount or configuration is invalid | Review fee parameters |
| `ScreeningNotFound` | Tenant screening record does not exist | Verify screening ID |
| `MaintenanceNotFound` | Maintenance request does not exist | Verify request ID |
| `ExpenseNotFound` | Expense record does not exist | Verify expense ID |
| `DisputeNotFound` | Dispute record does not exist | Verify dispute ID |
| `InvalidStatus` | Status transition is not allowed | Check valid status transitions |
| `ComplianceViolation` | Operation violates a compliance rule | Review compliance requirements |
| `NotCompliant` | Property or party is not compliant | Complete compliance requirements |
| `InspectionNotFound` | Inspection record does not exist | Verify inspection ID |
| `TransferFailed` | Rent or fee transfer failed | Check balances and retry |
| `RespondentMismatch` | Dispute respondent does not match | Verify dispute parties |

---

## Property Registry / Lib Errors (no numeric code)

Contract: `contracts/lib` — `Error`

| Variant | Meaning | Recovery |
|---------|---------|----------|
| `PropertyNotFound` | Property does not exist in the registry | Register the property first |
| `Unauthorized` | Caller lacks registry permissions | Use an authorized account |
| `InvalidMetadata` | Property metadata is invalid | Fix metadata fields and resubmit |
| `NotCompliant` | Recipient has not passed compliance checks | Complete KYC/AML verification |
| `ComplianceCheckFailed` | Call to compliance registry failed | Check compliance registry contract status |
| `EscrowNotFound` | Escrow does not exist | Verify escrow ID |
| `EscrowAlreadyReleased` | Escrow has already been released | No further action needed |
| `BadgeNotFound` | Compliance badge does not exist | Verify badge ID |
| `InvalidBadgeType` | Badge type is not recognized | Use a supported badge type |
| `BadgeAlreadyIssued` | Badge already issued to this property | No further action needed |
| `NotVerifier` | Caller is not an authorized verifier | Request verifier role from admin |
| `AppealNotFound` | Appeal record does not exist | Verify appeal ID |
| `InvalidAppealStatus` | Appeal status does not allow this operation | Check appeal status before acting |
| `ComplianceRegistryNotSet` | Compliance registry address not configured | Admin must configure the registry address |
| `OracleError` | Oracle contract returned an error | Check oracle contract status |
| `ContractPaused` | Contract is currently paused | Wait for admin to resume |
| `AlreadyPaused` | Contract is already paused | No action needed |
| `NotPaused` | Contract is not paused | Only valid when contract is paused |
| `ResumeRequestAlreadyActive` | A resume request is already pending | Wait for existing request to resolve |
| `ResumeRequestNotFound` | No active resume request exists | Create a resume request first |

---

## IPFS Metadata Errors (no numeric code)

Contract: `contracts/ipfs-metadata` — `Error`

| Variant | Meaning | Recovery |
|---------|---------|----------|
| `PropertyNotFound` | Property does not exist | Register the property first |
| `Unauthorized` | Caller lacks IPFS metadata permissions | Use an authorized account |
| `InvalidMetadata` | Metadata structure is invalid | Fix metadata and resubmit |
| `RequiredFieldMissing` | A required metadata field is absent | Provide all required fields |
| `DataTypeMismatch` | Field value type does not match schema | Use the correct data type |
| `SizeLimitExceeded` | Metadata payload exceeds size limit | Reduce payload size |
| `InvalidIpfsCid` | IPFS CID format is invalid | Provide a valid CIDv0 or CIDv1 |
| `IpfsNetworkFailure` | IPFS network is unreachable | Retry; check IPFS node availability |
| `ContentHashMismatch` | Content hash does not match stored CID | Re-pin the correct content |
| `MaliciousFileDetected` | File failed security scan | Do not upload malicious content |
| `FileTypeNotAllowed` | File MIME type is not permitted | Use an allowed file type |
| `EncryptionRequired` | Sensitive content must be encrypted | Encrypt before uploading |
| `PinLimitExceeded` | Maximum pinned files limit reached | Unpin unused files first |
| `DocumentNotFound` | Document does not exist | Verify document ID |
| `DocumentAlreadyExists` | Document with this ID already exists | Use a unique document ID |

---

## Access Control Errors (no numeric code)

Trait: `contracts/traits/src/access_control.rs` — `AccessControlError`

| Variant | Meaning | Recovery |
|---------|---------|----------|
| `Unauthorized` | Caller lacks the required role or permission | Request role assignment from admin |
| `KeyRotationCooldown` | Key rotation is still in cooldown period | Wait for cooldown to expire |
| `KeyRotationExpired` | Key rotation request has expired | Submit a new rotation request |
| `NoPendingRotation` | No pending key rotation for this account | Initiate a rotation request first |
| `RotationUnauthorized` | Caller is not authorized for this key rotation | Use the account owner or authorized guardian |

---

## Crypto Errors (no numeric code)

Trait: `contracts/traits/src/crypto.rs` — `CryptoError`

| Variant | Meaning | Recovery |
|---------|---------|----------|
| `InvalidSignature` | ECDSA signature recovery failed | Re-sign the message with the correct key |
| `InvalidPublicKey` | Recovered key does not match the registered key | Ensure the correct signing key is used |
| `HashError` | Hash computation failed | Retry; check input data integrity |
| `KeyRotationCooldown` | Key rotation is in cooldown period | Wait for cooldown to expire |
| `KeyRotationExpired` | Key rotation request has expired | Submit a new rotation request |
| `NoPendingRotation` | No pending rotation for this account | Initiate a rotation request first |
| `RotationUnauthorized` | Caller is not authorized for this rotation | Use the account owner or authorized guardian |
| `InvalidRandomnessPhase` | Randomness round is not in the expected phase | Wait for the correct phase |
| `CommitMismatch` | Revealed secret does not match the commit | Re-commit with the correct secret |
| `InsufficientReveals` | Not enough participants revealed their secrets | Wait for more participants to reveal |

---

## Dependency Injection Errors (no numeric code)

Trait: `contracts/traits/src/di.rs` — `DependencyError`

| Variant | Meaning | Recovery |
|---------|---------|----------|
| `ServiceNotRegistered` | Requested service has not been registered | Admin must register the service address |
| `Unauthorized` | Caller is not authorized to modify the registry | Use the admin account |
| `InvalidAddress` | Provided address is the zero address | Supply a valid non-zero account address |

---

## EventBus Subscriber Errors (no numeric code)

Trait: `contracts/traits/src/event_bus.rs` — `EventSubscriberError`

| Variant | Meaning | Recovery |
|---------|---------|----------|
| `UnauthorizedSender` | Caller is not the authorized EventBus contract | Only the EventBus may call subscriber callbacks |
| `ProcessingFailed` | Subscriber contract failed to process the event | Check subscriber implementation and fix the handler |

---

## Quick Reference: Numeric Code Lookup

| Code | Variant | Domain |
|------|---------|--------|
| 1 | `CommonError::Unauthorized` | Common |
| 2 | `CommonError::InvalidParameters` | Common |
| 3 | `CommonError::NotFound` | Common |
| 4 | `CommonError::InsufficientFunds` | Common |
| 5 | `CommonError::InvalidState` | Common |
| 6 | `CommonError::InternalError` | Common |
| 7 | `CommonError::CodecError` | Common |
| 8 | `CommonError::NotImplemented` | Common |
| 9 | `CommonError::Timeout` | Common |
| 10 | `CommonError::Duplicate` | Common |
| 1001–1025 | PropertyToken variants | PropertyToken |
| 2001–2013 | Escrow variants | Escrow |
| 3001–3013 | Bridge variants | Bridge |
| 4001–4012 | Oracle variants | Oracle |
| 5001–5008 | Fee variants | Fees |
| 6001–6013 | Compliance variants | Compliance |
| 7001–7015 | DEX variants | DEX |
| 8001–8013 | Governance variants | Governance |
| 9001–9010 | Staking variants | Staking |
| 10001–10005 | Monitoring variants | Monitoring |
| 11001–11006 | EventBus variants | EventBus |

---

*Source of truth: `contracts/traits/src/errors.rs` for all numeric codes. Contract-specific error enums are in each contract's `src/errors.rs` or `lib.rs`.*
