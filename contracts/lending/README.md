# PropChain Lending Platform

Decentralized property-backed lending platform with collateral management, dynamic interest rates, margin trading, and yield farming.

## Features

### Collateral Management
- Property collateral assessment with configurable LTV ratios
- Automated liquidation threshold monitoring
- Real-time collateral valuation tracking

### Lending Pools
- Dynamic interest rates based on pool utilization
- Deposit and borrow operations
- Automated rate adjustments

### Margin Trading
- Long and short position support
- Configurable leverage (up to 10x)
- Real-time PnL calculation

### Loan Underwriting
- Automated credit score evaluation
- LTV ratio validation (max 75%)
- Instant approval/rejection decisions

### Yield Farming
- Stake property tokens to earn rewards
- Per-block reward distribution
- Accumulated rewards tracking

### Governance
- On-chain proposal creation
- Community voting mechanism
- Automated proposal execution

## Usage

### Deploy Contract

```bash
cargo contract build --release
cargo contract instantiate --constructor new --args <ADMIN_ADDRESS>
```

### Assess Collateral

```rust
contract.assess_collateral(property_id, value, ltv_ratio, liquidation_threshold)?;
```

### Create Lending Pool

```rust
let pool_id = contract.create_pool(base_rate)?;
```

### Open Margin Position

```rust
let position_id = contract.open_position(collateral, leverage, is_short, entry_price)?;
```

### Apply for Loan

```rust
let loan_id = contract.apply_for_loan(property_id, amount, collateral_value, credit_score)?;
let approved = contract.underwrite_loan(loan_id)?;
```

### Liquidate Loan

```rust
contract.liquidate_loan(loan_id, vec![(property_id, current_property_value)])?;
```

### Stake for Yield

```rust
contract.stake(amount)?;
let rewards = contract.pending_rewards(owner, current_block);
```

### Governance

```rust
let proposal_id = contract.propose("Lower LTV cap to 70%".into())?;
contract.vote(proposal_id, true)?;
contract.execute_proposal(proposal_id)?;
```

## Testing

```bash
cargo test
```

### Test Module Layout

The lending contract's tests are organised into three distinct layers, each in
its own `mod` block:

| Module | Location | Purpose |
|--------|----------|---------|
| `mod tests` | `contracts/lending/src/lib.rs` | **Core unit tests** — collateral, pools, loan underwriting, servicer integration, restructuring, liquidation, yield farming, governance, credit scoring, and multi-collateral (#588). New feature tests belong here first. |
| `mod lending_admin_rotation_tests` | `contracts/lending/src/lib.rs` | **Admin key-rotation tests** (Issue #496) — verifies the two-step time-locked admin handoff: cooldown enforcement, expiry, and cancellation by either party. |
| `mod storage_derivation_tests` | `contracts/lending/src/lib.rs` | **Compile-time trait assertions** (#589) — confirms every public storage type derives `Encode`, `Decode`, `TypeInfo`, and `StorageLayout`. These fail at `cargo test`, not just at WASM build time. |

There is also a fourth file wired in as a separate module:

| File | Module alias | Purpose |
|------|-------------|---------|
| `contracts/lending/src/test.rs` | `mod lending_regression_test` | **Regression tests** — pins known-good (and known-buggy) behaviours such as the JIT interest accrual ordering. |

When adding new tests, choose the module that best matches the concern:
- Functional behaviour → `mod tests`
- Admin rotation edge cases → `mod lending_admin_rotation_tests`
- New storage types → `mod storage_derivation_tests`
- Regression/known-bug documentation → `src/test.rs`

## Architecture

The lending platform is built as an ink! smart contract with the following components:

- **CollateralRecord**: Tracks property collateral with LTV and liquidation thresholds
- **LendingPool**: Manages deposits, borrows, and dynamic interest rates
- **MarginPosition**: Handles leveraged trading positions
- **LoanApplication**: Processes loan requests with automated underwriting
- **YieldPosition**: Tracks staking and reward accumulation
- **Proposal**: Manages governance proposals and voting

## Security

- Admin-only functions for critical operations
- Automated liquidation monitoring
- Credit score and LTV validation
- Utilization-based rate adjustments

## License

MIT
