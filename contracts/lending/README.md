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
contract.liquidate_loan(loan_id, current_property_value)?;
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
