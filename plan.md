# Plan: Property Token — Add Staking and Governance Utility Features (Issue #197)

## Context

Issue #197 requests token utility features for the property-token contract: staking of fractional shares with lock-period-based rewards, and governance voting weight boosted by staking. The property-token already has a basic per-token proposal/vote system (using raw share balance as voting weight) and a fractional share system. This plan wires in staking so that share holders can lock their shares to earn rewards and gain multiplied governance power.

The standalone `staking` contract is NOT touched — this adds share staking directly into `property-token`, scoped per property token.

---

## Files to Modify

| File | Change |
|---|---|
| `contracts/traits/src/errors.rs` | Add 5 error codes to `property_token_codes` (1026–1030) |
| `contracts/property-token/src/errors.rs` | Add 5 error variants + Display/ContractError impls |
| `contracts/property-token/src/types.rs` | Add `ShareLockPeriod` enum + `ShareStakeInfo` struct |
| `contracts/property-token/src/lib.rs` | Add storage fields, events, public methods, `include!`, modify `vote()` |
| `contracts/property-token/src/staking.rs` | **New file** — pure helper functions |
| `contracts/property-token/src/tests.rs` | 11 new tests |

---

## Step 1 — `contracts/traits/src/errors.rs`

Append to `property_token_codes` module (after `BATCH_SIZE_EXCEEDED = 1025`):

```rust
pub const STAKE_NOT_FOUND: u32 = 1026;
pub const LOCK_ACTIVE: u32 = 1027;
pub const NO_REWARDS: u32 = 1028;
pub const INSUFFICIENT_REWARD_POOL: u32 = 1029;
pub const ALREADY_STAKED: u32 = 1030;
```

---

## Step 2 — `contracts/property-token/src/errors.rs`

Add 5 variants to the `Error` enum (after `BatchSizeExceeded`):

```rust
/// No stake found for this account and token
StakeNotFound,
/// Stake lock period has not yet expired
LockActive,
/// No staking rewards available to claim
NoRewards,
/// Stake reward pool has insufficient funds
InsufficientRewardPool,
/// An active stake already exists for this account and token
AlreadyStaked,
```

Add matching arms to the three `impl` blocks:

- **Display**: `"Stake not found"`, `"Stake lock period is still active"`, `"No staking rewards available"`, `"Insufficient reward pool balance"`, `"An active stake already exists for this token"`
- **error_code**: map to `property_token_codes::STAKE_NOT_FOUND` … `ALREADY_STAKED`
- **error_description**: same content, slightly longer phrasing (see existing pattern)

---

## Step 3 — `contracts/property-token/src/types.rs`

Add after the existing types:

```rust
/// Lock period options for staking fractional shares
#[derive(Debug, Clone, Copy, PartialEq, Eq, scale::Encode, scale::Decode, ink::storage::traits::StorageLayout)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub enum ShareLockPeriod {
    Flexible,
    ThirtyDays,
    NinetyDays,
    OneYear,
}

impl ShareLockPeriod {
    pub fn duration_blocks(&self) -> u64 {
        match self {
            ShareLockPeriod::Flexible    => 0,
            ShareLockPeriod::ThirtyDays  => LOCK_PERIOD_30_DAYS,
            ShareLockPeriod::NinetyDays  => LOCK_PERIOD_90_DAYS,
            ShareLockPeriod::OneYear     => LOCK_PERIOD_1_YEAR,
        }
    }

    /// Returns the reward/governance multiplier in basis points (100 = 1×)
    pub fn multiplier(&self) -> u128 {
        match self {
            ShareLockPeriod::Flexible    => MULTIPLIER_FLEXIBLE,
            ShareLockPeriod::ThirtyDays  => MULTIPLIER_30_DAYS,
            ShareLockPeriod::NinetyDays  => MULTIPLIER_90_DAYS,
            ShareLockPeriod::OneYear     => MULTIPLIER_1_YEAR,
        }
    }
}

/// Per-account, per-token staking record
#[derive(Debug, Clone, PartialEq, scale::Encode, scale::Decode, ink::storage::traits::StorageLayout)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub struct ShareStakeInfo {
    pub staker:      AccountId,
    pub token_id:    TokenId,
    pub amount:      u128,
    pub staked_at:   u64,
    pub lock_until:  u64,
    pub lock_period: ShareLockPeriod,
    pub reward_debt: u128,
}
```

The constants (`LOCK_PERIOD_30_DAYS`, `MULTIPLIER_FLEXIBLE`, etc.) are already in scope via `use propchain_traits::*` at the top of lib.rs, and `types.rs` is `include!`-ed inside the module.

---

## Step 4 — New file `contracts/property-token/src/staking.rs`

This file is included inside `impl PropertyToken { ... }`. It contains three private helpers:

```rust
// Staking helper methods for PropertyToken (Issue #197)
// Included inside `impl PropertyToken` — do not wrap in another impl block.

const STAKE_SCALING: u128 = 1_000_000_000_000;

/// Update the accumulated reward-per-staked-share for a token.
/// Must be called before any stake balance change.
fn update_stake_acc_reward(&mut self, token_id: TokenId) {
    let total = self.share_total_staked.get(token_id).unwrap_or(0);
    if total == 0 { return; }
    let now = self.env().block_number() as u64;
    let last = self.share_last_reward_block.get(token_id).unwrap_or(now);
    let blocks = (now as u128).saturating_sub(last as u128);
    if blocks == 0 { return; }
    let rate = self.share_reward_rate_bps.get(token_id).unwrap_or(0);
    // reward per block per share (scaled)
    let reward = total
        .saturating_mul(rate)
        .saturating_mul(blocks)
        / REWARD_RATE_PRECISION
        / 5_256_000;
    let acc = self.share_acc_reward_per_share.get(token_id).unwrap_or(0);
    self.share_acc_reward_per_share
        .insert(token_id, &acc.saturating_add(reward.saturating_mul(STAKE_SCALING) / total));
    self.share_last_reward_block.insert(token_id, &now);
}

/// Compute pending rewards for a stake (view, no mutation).
fn pending_stake_rewards(&self, stake: &ShareStakeInfo) -> u128 {
    let acc = self.share_acc_reward_per_share.get(stake.token_id).unwrap_or(0);
    let base = stake.amount
        .saturating_mul(acc.saturating_sub(stake.reward_debt))
        / STAKE_SCALING;
    base.saturating_mul(stake.lock_period.multiplier()) / 100
}

/// Return the effective governance voting weight for an account on a token.
/// Stakers receive their staked amount × lock multiplier.
/// Non-stakers receive their raw share balance (1× — backward compatible).
fn governance_weight(&self, voter: AccountId, token_id: TokenId) -> u128 {
    if let Some(stake) = self.share_stakes.get((voter, token_id)) {
        stake.amount.saturating_mul(stake.lock_period.multiplier()) / 100
    } else {
        self.balances.get((voter, token_id)).unwrap_or(0)
    }
}
```

---

## Step 5 — `contracts/property-token/src/lib.rs`

### 5a. Storage fields (add after `management_agent` field, line 80)

```rust
// Share staking (Issue #197)
share_stakes: Mapping<(AccountId, TokenId), ShareStakeInfo>,
share_total_staked: Mapping<TokenId, u128>,
share_reward_pool: Mapping<TokenId, u128>,
share_acc_reward_per_share: Mapping<TokenId, u128>,
share_last_reward_block: Mapping<TokenId, u64>,
share_reward_rate_bps: Mapping<TokenId, u128>,
```

Initialize each as `Mapping::default()` in the `new()` constructor.

### 5b. Include staking helpers (add inside `impl PropertyToken`, before the closing `}` at line 2384)

```rust
include!("staking.rs");
```

### 5c. New events (add after the Management Events section)

```rust
#[ink(event)]
pub struct SharesStaked {
    #[ink(topic)]
    pub token_id: TokenId,
    #[ink(topic)]
    pub staker: AccountId,
    pub amount: u128,
    pub lock_period: ShareLockPeriod,
    pub lock_until: u64,
}

#[ink(event)]
pub struct SharesUnstaked {
    #[ink(topic)]
    pub token_id: TokenId,
    #[ink(topic)]
    pub staker: AccountId,
    pub amount: u128,
}

#[ink(event)]
pub struct StakeRewardsClaimed {
    #[ink(topic)]
    pub token_id: TokenId,
    #[ink(topic)]
    pub staker: AccountId,
    pub amount: u128,
}

#[ink(event)]
pub struct StakeRewardPoolFunded {
    #[ink(topic)]
    pub token_id: TokenId,
    #[ink(topic)]
    pub funder: AccountId,
    pub amount: u128,
}
```

### 5d. New public `#[ink(message)]` methods

**`stake_shares(token_id, amount, lock_period) -> Result<(), Error>`**
1. Validate `amount > 0` → `InvalidAmount`
2. Require no existing stake → `AlreadyStaked`
3. Require `balances[(caller, token_id)] >= amount` → `InsufficientBalance`
4. Call `update_dividend_credit_on_change(caller, token_id)` (preserves dividend accounting)
5. Deduct `amount` from `balances`
6. Call `update_stake_acc_reward(token_id)`
7. Build `ShareStakeInfo` with `staked_at = block_number()`, `lock_until = staked_at + lock_period.duration_blocks()`, `reward_debt = acc_reward_per_share`
8. Insert into `share_stakes`, increment `share_total_staked`
9. Emit `SharesStaked`

**`unstake_shares(token_id) -> Result<(), Error>`**
1. Get stake → `StakeNotFound`
2. Check `block_number() >= lock_until` → `LockActive`
3. Call `update_stake_acc_reward(token_id)`
4. Calculate and claim any pending rewards (auto-claim on unstake)
5. Call `update_dividend_credit_on_change(caller, token_id)` before restoring balance
6. Restore `amount` to `balances`, remove from `share_stakes`, decrement `share_total_staked`
7. Emit `SharesUnstaked` (and `StakeRewardsClaimed` if rewards > 0)

**`claim_stake_rewards(token_id) -> Result<u128, Error>`**
1. Get stake → `StakeNotFound`
2. Call `update_stake_acc_reward(token_id)`
3. `rewards = pending_stake_rewards(&stake)` → 0 means `NoRewards`
4. Check `share_reward_pool[token_id] >= rewards` → `InsufficientRewardPool`
5. Deduct from pool, update `stake.reward_debt = acc_reward_per_share`
6. `env().transfer(caller, rewards)` (best-effort, ignore transfer error for no-op in tests)
7. Emit `StakeRewardsClaimed`, return `Ok(rewards)`

**`fund_stake_reward_pool(token_id) -> Result<(), Error>`** (`#[ink(message, payable)]`)
1. Validate token exists → `TokenNotFound`
2. Get transferred value, validate > 0 → `InvalidAmount`
3. Increment `share_reward_pool[token_id]`
4. Emit `StakeRewardPoolFunded`

**`set_stake_reward_rate(token_id, rate_bps) -> Result<(), Error>`** (admin only)
1. Check caller == admin → `Unauthorized`
2. Flush pending rewards at old rate: `update_stake_acc_reward(token_id)`
3. Insert `share_reward_rate_bps[token_id] = rate_bps`

**Query methods (pure `#[ink(message)]`):**
- `get_share_stake(staker, token_id) -> Option<ShareStakeInfo>`
- `get_pending_stake_rewards(staker, token_id) -> u128` — read-only calculation
- `get_governance_weight(voter, token_id) -> u128` — public wrapper for the private helper

### 5e. Modify `vote()` (line 1053)

Replace:
```rust
let weight = self.balances.get((voter, token_id)).unwrap_or(0);
```
with:
```rust
let weight = self.governance_weight(voter, token_id);
```

Update the doc comment: "Voting weight equals the caller's staked share balance multiplied by the lock-period multiplier, or their raw share balance if not staking."

---

## Step 6 — `contracts/property-token/src/tests.rs`

Add 11 new tests in the existing `tests` module:

| Test | Scenario |
|---|---|
| `test_stake_shares_success` | Issue shares, stake half, verify `share_stakes` record and reduced `balances` |
| `test_stake_zero_amount_fails` | `stake_shares(_, 0, _)` → `InvalidAmount` |
| `test_stake_insufficient_balance_fails` | Stake more than balance → `InsufficientBalance` |
| `test_stake_already_staked_fails` | Second stake on same token → `AlreadyStaked` |
| `test_unstake_lock_active_fails` | Unstake with `ThirtyDays` lock immediately → `LockActive` |
| `test_unstake_flexible_succeeds` | `Flexible` lock unstake immediately → `Ok`, balance restored |
| `test_unstake_not_staked_fails` | Unstake without prior stake → `StakeNotFound` |
| `test_claim_rewards_no_stake_fails` | Claim without stake → `StakeNotFound` |
| `test_governance_weight_non_staker` | Non-staker weight = raw balance (backward compat) |
| `test_governance_weight_staker_boosted` | `OneYear` stake → weight = amount × 3 |
| `test_vote_uses_boosted_weight` | Staker's 3× boost causes proposal to pass quorum it otherwise wouldn't |

---

## Verification

```bash
cargo check -p property-token --all-features
cargo test -p property-token --lib
cargo clippy -p property-token -- -D warnings
cargo fmt -p property-token -- --check
```

All existing tests must remain green. The `vote()` change is backward-compatible: non-stakers get `governance_weight = raw_balance` (unchanged behavior).
