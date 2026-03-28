# PropChain Crowdfunding Platform

Decentralized real estate crowdfunding platform enabling multiple investors to pool resources for property acquisitions.

## Features

### Campaign Management
- Create and activate funding campaigns
- Track funding progress and investor participation
- Automatic status transitions (Draft → Active → Funded)

### Investor Compliance
- KYC/AML onboarding
- Jurisdiction-based restrictions
- Accredited investor verification

### Milestone-Based Fund Release
- Create project milestones with release amounts
- Approval workflow (Pending → Approved → Released)
- Transparent fund disbursement tracking

### Profit Sharing
- Proportional dividend distribution
- Automated payout calculations based on investment share

### Governance
- Investor voting on proposals
- Weighted voting based on investment amount
- Proposal lifecycle (Active → Passed/Rejected)

### Secondary Market
- List crowdfunding shares for sale
- Peer-to-peer share transfers
- Price discovery mechanism

### Risk Assessment
- LTV ratio analysis
- Developer score evaluation
- Market volatility tracking
- Automated risk rating (Low/Medium/High)

### Analytics
- Campaign funding percentage
- Investor count tracking
- Investment amount monitoring

## Usage

### Deploy Contract

```bash
cargo contract build --release
cargo contract instantiate --constructor new --args <ADMIN_ADDRESS>
```

### Create Campaign

```rust
let campaign_id = contract.create_campaign("Downtown Lofts".into(), 1_000_000)?;
contract.activate_campaign(campaign_id)?;
```

### Investor Onboarding

```rust
contract.onboard_investor("US".into(), true)?;
contract.invest(campaign_id, 250_000)?;
```

### Milestone Management

```rust
let milestone_id = contract.add_milestone(campaign_id, "Foundation Complete".into(), 200_000)?;
contract.approve_milestone(milestone_id)?;
contract.release_milestone(milestone_id)?;
```

### Profit Distribution

```rust
let payout = contract.distribute_profit(campaign_id, 50_000, investor_address);
```

### Governance

```rust
let proposal_id = contract.create_proposal(campaign_id, "Release milestone funds".into())?;
contract.vote(proposal_id, true)?;
let status = contract.finalize_proposal(proposal_id)?;
```

### Secondary Market

```rust
let listing_id = contract.list_shares(campaign_id, 100, 1_000)?;
let cost = contract.buy_shares(listing_id)?;
```

### Risk Assessment

```rust
contract.assess_risk(campaign_id, 60, 75, 15)?;
let profile = contract.get_risk_profile(campaign_id);
```

## Testing

```bash
cargo test
```

## Architecture

Built as an ink! smart contract with:

- **Campaign**: Project creation and funding tracking
- **InvestorProfile**: KYC/AML compliance data
- **Milestone**: Fund release management
- **Proposal**: Governance voting
- **ShareListing**: Secondary market trading
- **RiskProfile**: Risk assessment data

## Security

- Admin-only functions for critical operations
- Compliance checks before investment
- Jurisdiction-based restrictions
- Milestone approval workflow
- Voting weight validation

## License

MIT
