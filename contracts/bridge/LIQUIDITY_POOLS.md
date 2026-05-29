# Bridge Liquidity Pool Management

This document describes the liquidity pool management interface for the
`PropertyBridge` contract.

## Pool lifecycle

### Create pool
```rust
fn create_liquidity_pool(
    pool_id: u64,
    token: AccountId,
    initial_deposit: u128,
) -> Result<(), Error>
```
Admin-only. Registers a new pool and transfers `initial_deposit` from the caller.

### Add liquidity
```rust
fn add_liquidity(pool_id: u64, amount: u128) -> Result<u128, Error>
```
Any caller may deposit tokens. Returns LP shares minted.

### Remove liquidity
```rust
fn remove_liquidity(pool_id: u64, shares: u128) -> Result<u128, Error>
```
Burns LP shares and returns the proportional token amount.

### Rebalance
```rust
fn rebalance_pool(pool_id: u64, target_chain: ChainId) -> Result<(), Error>
```
Operator-only. Moves excess liquidity to an under-funded destination chain.

## Fee model

| Action | Fee |
|---|---|
| Cross-chain swap | 0.1 % of transfer amount |
| Liquidity removal | 0.05 % of withdrawn amount |

Fees accumulate in the pool and are distributed pro-rata to LP share holders.

## Error variants

`PoolNotFound`, `InsufficientPoolFunds`, `Unauthorized`, `ReentrantCall`