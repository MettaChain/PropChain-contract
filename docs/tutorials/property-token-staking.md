# Property Token ↔ Staking Integration

This tutorial explains how fractional property shares interact with the staking subsystem inside `contracts/property-token`. You will learn how to:

1. Acquire fractional shares of a property NFT
2. Deposit (stake) those shares as collateral to earn rewards
3. Claim accrued rewards without unstaking
4. Unstake and retrieve the original shares

---

## Background

The `PropertyToken` contract stores fractional ownership via an ERC-1155-style `balances` mapping:

```
balances: Mapping<(AccountId, TokenId), u128>
```

The staking module (`src/staking.rs`, included into the main `impl` block) lets share holders lock their balance into `share_stakes` for a chosen `LockPeriod`. A reward accumulator tracks yield per staked share at every block, so rewards accrue continuously without any storage writes.

---

## Lock Periods and Reward Multipliers

| Variant | Duration | Reward multiplier |
|---------|----------|------------------|
| `LockPeriod::Flexible` | 0 blocks (unlock any time) | 1.0× |
| `LockPeriod::ThirtyDays` | ~30 days in blocks | 1.1× |
| `LockPeriod::NinetyDays` | ~90 days in blocks | 1.25× |
| `LockPeriod::OneYear` | ~365 days in blocks | 1.5× |

Multipliers are applied at claim time so longer lock-ups compound the reward rate.

---

## Step-by-Step: Staking Shares

### 1. Mint or acquire shares

```typescript
// Assumes `api` is a connected PolkadotJS ApiPromise and
// `propertyToken` is the contract instance from @polkadot/api-contract

const tokenId = 1n; // existing property NFT

// Check share balance
const { output: balance } = await propertyToken.query.balancesOf(
  callerAddress,
  { gasLimit: -1 },
  callerAddress,
  tokenId
);
console.log("Share balance:", balance.toHuman());
```

### 2. Stake shares

```typescript
import { BN } from "@polkadot/util";

const amountToStake = new BN("1000000000000000000"); // 1e18 shares
const lockPeriod = { ninetyDays: null }; // LockPeriod::NinetyDays

const { result, output } = await propertyToken.tx
  .stakeShares(
    { gasLimit: api.registry.createType("WeightV2", { refTime: 10_000_000_000n, proofSize: 131072n }) },
    tokenId,
    amountToStake,
    lockPeriod
  )
  .signAndSend(callerKeypair);

console.log("Stake result:", result.toHuman());
```

> **Note:** Only one active stake per `(AccountId, TokenId)` pair is allowed. Call `unstakeShares` before re-staking with different parameters.

### 3. Check pending rewards

```typescript
const { output: pendingRewards } = await propertyToken.query.getPendingStakeRewards(
  callerAddress,
  { gasLimit: -1 },
  callerAddress,  // staker
  tokenId
);
console.log("Pending rewards (planck):", pendingRewards.toHuman());
```

### 4. Claim rewards without unstaking

```typescript
const { result } = await propertyToken.tx
  .claimStakeRewards(
    { gasLimit: api.registry.createType("WeightV2", { refTime: 5_000_000_000n, proofSize: 65536n }) },
    tokenId
  )
  .signAndSend(callerKeypair);

console.log("Claim tx:", result.toHuman());
```

Rewards are sent to the caller's address on-chain; the stake itself remains locked.

### 5. Unstake (auto-claims remaining rewards)

```typescript
const { result } = await propertyToken.tx
  .unstakeShares(
    { gasLimit: api.registry.createType("WeightV2", { refTime: 10_000_000_000n, proofSize: 131072n }) },
    tokenId
  )
  .signAndSend(callerKeypair);

console.log("Unstake tx:", result.toHuman());
```

`unstakeShares` validates that the `lock_until` block has passed; calling it early returns `Error::LockActive`.

---

## Worked Example Transaction Sequence

Below is a complete lifecycle for one staker (Alice) on token `1`:

```
Block 100  Alice calls issue_shares(token_id=1, amount=1_000)
           → balances[(Alice, 1)] = 1_000

Block 101  Admin calls fund_stake_reward_pool(token_id=1)
           with value = 10 UNIT
           Admin calls set_stake_reward_rate(token_id=1, rate_bps=500)
           → 5% annual rate

Block 102  Alice calls stake_shares(token_id=1, amount=500, lock_period=NinetyDays)
           → share_stakes[(Alice, 1)] recorded
           → balances[(Alice, 1)] = 500  (500 locked, 500 free)

Block 2000 Alice calls claim_stake_rewards(token_id=1)
           → pending rewards transferred to Alice
           → stake_stakes[(Alice, 1)].reward_debt updated

Block 8000 (lock_until reached)
           Alice calls unstake_shares(token_id=1)
           → remaining rewards claimed
           → balances[(Alice, 1)] = 1_000  (500 returned)
           → share_stakes[(Alice, 1)] removed
```

---

## Reading the Stake Record

```typescript
const { output: stake } = await propertyToken.query.getShareStake(
  callerAddress,
  { gasLimit: -1 },
  callerAddress, // staker
  tokenId
);

if (stake.isSome) {
  const s = stake.unwrap();
  console.log({
    amount:      s.amount.toHuman(),
    stakedAt:    s.stakedAt.toHuman(),
    lockUntil:   s.lockUntil.toHuman(),
    lockPeriod:  s.lockPeriod.toHuman(),
  });
}
```

---

## Error Reference

| Error | Cause |
|-------|-------|
| `AlreadyStaked` | Tried to stake when an active stake already exists for this `(account, token_id)` |
| `InsufficientBalance` | Not enough free shares to cover the requested stake amount |
| `LockActive` | Attempted to unstake before `lock_until` block |
| `NoRewards` | Claim called but no rewards have accrued yet |
| `InsufficientRewardPool` | Reward pool ran out of funds |
| `InvalidAmount` | `amount = 0` passed to `stake_shares` |

---

## See Also

- [`contracts/property-token/src/staking.rs`](../../contracts/property-token/src/staking.rs) — reward accumulator and governance weight logic
- [`docs/tutorials/basic-property-registration.md`](./basic-property-registration.md) — minting a property NFT and issuing fractional shares
- [`docs/tutorials/property_token_tutorial.md`](./property_token_tutorial.md) — full property token lifecycle
