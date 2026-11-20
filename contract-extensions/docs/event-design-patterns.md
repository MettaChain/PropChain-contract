# ink! event design patterns

This guide describes event conventions used by PropChain contracts. Source paths are relative to the repository root.

## Event declarations

An event is a SCALE-encoded, non-storage data type declared with `#[ink(event)]`:

```rust
#[ink(event)]
pub struct FundsTransferred {
    #[ink(topic)]
    pub from: AccountId,
    #[ink(topic)]
    pub to: AccountId,
    pub amount: u128,
}
```

The real staking events follow this shape in `contracts/staking/src/lib.rs`. Governance events are in `contracts/governance/src/lib.rs`, and another broad set is in `contracts/dex/src/lib.rs`.

## Indexed fields

Use `#[ink(topic)]` for fields that off-chain systems filter by. Account IDs and IDs are typical topics:

```rust
#[ink(event)]
pub struct Staked {
    #[ink(topic)]
    pub staker: AccountId,
    pub amount: u128,
    pub lock_period: LockPeriod,
    pub lock_until: u64,
}
```

Topics do not encode the complete event. Keep payload fields in the body to reduce topic cost and retain data needed for decoding.

At most two or three stable identities are normally useful. Treat topic changes as an ABI change because indexers build SQL indexes and filter predicates against named topic positions.

## Emitting events

Emit after state is successfully updated:

```rust
self.env().emit_event(FundsTransferred {
    from: self.env().caller(),
    to,
    amount,
});
```

PropChain examples include `Staked`, `Unstaked`, `RewardsClaimed`, and `ParamVoteCast` in `contracts/staking/src/lib.rs`, and `SwapExecuted`, `PoolCreated`, and `LiquidityAdded` in `contracts/dex/src/lib.rs`.

Never emit a success event from a transaction that later reverts; ink! discards events together with reverted state.

## Naming conventions

- Name events with past-tense, descriptive nouns or phrases: `Staked`, `FundsReleased`, `ProposalCreated`.
- Name fields with lower snake_case and exact domain meaning.
- Prefer `Updated`, `Cancelled`, `Claimed`, and `Scheduled` over generic `Changed`.
- Keep event names stable within a contract. Introduce a new versioned event for incompatible payload changes.

## SCALE encoding

Ink! events and their fields use SCALE:

1. The event name/selector identifies the event type.
2. Every field is SCALE-encoded in declaration order.
3. Topic fields are hashed/indexed in addition to appearing in the event body.

Off-chain decoders need the contract metadata JSON (`.contract` bundle), or a matching Rust/TypeScript scale type. Do not infer field offsets; decode from metadata. Unknown future fields should be handled by metadata-aware indexers.

Example conceptual representation:

```text
Event:
  Staked
Data (SCALE):
  staker_topic = hash(SCALE(staker))
  amount       = SCALE(u128)
  lock_period  = SCALE(LockPeriod)
  lock_until   = SCALE(u64)
```

## Indexer and frontend use

The repository indexer is configured in `Dockerfile.indexer` and should:

- Subscribe to contract events by contract address and event signature.
- Decode each event with contract metadata.
- Store topic values and body values separately.
- Keep the block number, block hash, extrinsic index, and event index for replay.
- Make handlers idempotent so resyncing a block cannot duplicate records.
- Use cursor/reorg handling around finalized and best-head subscriptions.

Frontend clients should filter known topics such as `staker`, `proposal_id`, or `contract_type`, then page by block/event index. Do not rely on the contract storage view for immutable audit history.

## Event catalog

| Contract area | Representative events | Source |
|---|---|---|
| Staking | `Staked`, `Unstaked`, `RewardsClaimed`, `ParamVoteCast`, `VestingScheduleCreated` | `contracts/staking/src/lib.rs` |
| Governance | `ProposalCreated`, `Voted`, `ProposalExecuted`, `ProposalCancelled`, `ProposalThresholdUpdated` | `contracts/governance/src/lib.rs` |
| DEX | `PoolCreated`, `LiquidityAdded`, `SwapExecuted`, `TradingCompetitionCreated`, `PriceImpactWarning` | `contracts/dex/src/lib.rs` |
| Crowdfunding | `EscrowCreated`, `FundsDeposited`, `FundsReleased`, `FundsRefunded`, `LargeTransferExecuted` | `contracts/crowdfunding/src/sequence.rs` |
| Lending | `Deposit`, `Withdraw`, `Borrow`, `Repay`, `Liquidation` | `contracts/lending/src/lib.rs` |
| Insurance | `PolicyCreated`, `ClaimSubmitted`, `ClaimApproved`, `ClaimRejected`, `ClaimPaid` | `contracts/insurance/src/lib.rs` |
| Oracle | `PriceUpdated`, `ProviderRegistered`, `ProviderRemoved`, `OracleAggregationUpdated` | `contracts/oracle/src/lib.rs` |
| Property | `PropertyCreated`, `PropertyTokenMinted`, `PropertyUpdated`, `OwnershipTransferred` | `contracts/property-management/src/lib.rs` |
| Factory | `ContractDeployed`, `CodeHashUpdated` | `contracts/factory/src/lib.rs` |
| Proxy | `UpgradeStaged`, `UpgradeConfirmed`, `UpgradeCancelled`, `AdminTransferStarted`, `AdminChanged` | `contracts/proxy/src/lib.rs` |
| Fees | `FeeConfigUpdated`, `PremiumAuctionCreated`, `PremiumAuctionBid`, `RewardsDistributed` | `contracts/fees/src/lib.rs` |
| GDPR | `ConsentGranted`, `ConsentWithdrawn`, `ConsentExpired`, `DataAccessRequested` | `contracts/gdpr/src/lib.rs` |
| Sanctions | `EntitySanctioned`, `EntityRemovedFromSanctions`, `PropertySanctioned`, `ScreeningsPerformed` | `contracts/sanctions/lib.rs` |
| Fractional ownership | issuance, transfer, redemption, and distribution events | `contracts/fractional/src/lib.rs` |
| Identity | verification, credential, recovery, and authorization events | `contracts/identity/lib.rs` |
| Multicall | `CallExecuted`, `BatchExecuted`, `PauseToggled` | `contracts/multicall/src/lib.rs` |

This catalog is representative. Discover the complete set for a contract with:

```bash
rg -n '#\[ink\(event\)\]' contracts/<contract>/src
```

## Versioning and upgrades

For an upgrade:

1. Never remove or reorder existing event fields.
2. Add fields only if indexer decoders ignore unknown fields in the SCALE layout.
3. Prefer a new event name for an incompatible payload, for example `StakedV2`.
4. Include version or migration fields in the new event body.
5. Update metadata and indexer schemas in the same release.
6. Add a migration or replay record if historical data is derived from event history.

The proxy in `contracts/proxy/src/lib.rs` emits upgrade lifecycle events, while staking and governance emit configuration-change events. These patterns allow clients to audit implementation and parameter transitions across upgrades.
