# PropChain System Architecture Overview

## Executive Summary

PropChain is a decentralized real estate tokenization platform built on the Substrate blockchain using ink! smart contracts. This document provides a high-level overview of the system architecture, component interactions, and design principles.

## System Vision

PropChain transforms physical real estate properties into tradable digital assets through a modular, secure, and compliant smart contract ecosystem. The system enables:

- **Property Tokenization**: NFT-based representation of real estate assets
- **Secure Transfers**: Escrow-protected ownership transfers
- **Fractional Ownership**: Division of property ownership into shares
- **Cross-Chain Compatibility**: Multi-chain asset transfers via bridges
- **Regulatory Compliance**: Built-in KYC/AML and jurisdiction-specific compliance
- **Decentralized Governance**: Community-driven protocol management

---

## High-Level Architecture

```
┌─────────────────────────────────────────────────────────────────────┐
│                         PRESENTATION LAYER                          │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐              │
│  │   Web dApp   │  │  Mobile App  │  │   Admin UI   │              │
│  │  (React/     │  │ (Flutter/    │  │  Dashboard   │              │
│  │   Next.js)   │  │  React Native)│  │              │              │
│  └──────────────┘  └──────────────┘  └──────────────┘              │
└─────────────────────────────────────────────────────────────────────┘
                                │
                                ▼
┌─────────────────────────────────────────────────────────────────────┐
│                          GATEWAY LAYER                              │
│  ┌─────────────────────────────────────────────────────────────┐   │
│  │                    API Gateway / RPC                         │   │
│  │         (Polkadot.js API, Substrate RPC Nodes)              │   │
│  └─────────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────────┘
                                │
                                ▼
┌─────────────────────────────────────────────────────────────────────┐
│                       SMART CONTRACT LAYER                          │
│  ┌─────────────────────────────────────────────────────────────┐   │
│  │                  Core Contracts (Ink!)                       │   │
│  │  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐      │   │
│  │  │  Property    │  │    Escrow    │  │  Compliance  │      │   │
│  │  │   Registry   │  │   Contract   │  │  Registry    │      │   │
│  │  └──────────────┘  └──────────────┘  └──────────────┘      │   │
│  │  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐      │   │
│  │  │    Bridge    │  │  Insurance   │  │   Valuation  │      │   │
│  │  │   Contract   │  │   Contract   │  │    Oracle    │      │   │
│  │  └──────────────┘  └──────────────┘  └──────────────┘      │   │
│  └─────────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────────┘
                                │
                                ▼
┌─────────────────────────────────────────────────────────────────────┐
│                        DATA LAYER                                   │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐              │
│  │  On-Chain    │  │  IPFS/       │  │  Off-Chain   │              │
│  │   Storage    │  │  Arweave     │  │  Database    │              │
│  │  (Substrate) │  │  (Documents) │  │  (Indexer)   │              │
│  └──────────────┘  └──────────────┘  └──────────────┘              │
└─────────────────────────────────────────────────────────────────────┘
                                │
                                ▼
┌─────────────────────────────────────────────────────────────────────┐
│                     EXTERNAL INTEGRATIONS                           │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐              │
│  │    KYC/AML   │  │   Price      │  │  Payment     │              │
│  │   Providers  │  │   Oracles    │  │   Gateways   │              │
│  └──────────────┘  └──────────────┘  └──────────────┘              │
└─────────────────────────────────────────────────────────────────────┘
```

---

## Core Component Architecture

### 1. Property Registry Component

**Purpose**: Central system of record for all tokenized properties

**Responsibilities**:
- Property registration and metadata management
- Ownership tracking and verification
- Property lifecycle management
- Integration with compliance systems

**Key Data Structures**:
```rust
pub struct PropertyInfo {
    pub id: u64,
    pub owner: AccountId,
    pub metadata: PropertyMetadata,
    pub registered_at: u64,
}

pub struct PropertyMetadata {
    pub location: String,
    pub size: u64,
    pub legal_description: String,
    pub valuation: u128,
    pub documents_url: String,
}
```

**Interactions**:
- ← Receives: Property registration requests from users
- → Calls: Compliance Registry for ownership verification
- → Calls: Valuation Oracle for pricing updates
- → Emits: PropertyRegistered, OwnershipTransferred events

---

### 2. Escrow Component

**Purpose**: Secure, trustless property transfer mechanism

**Responsibilities**:
- Multi-signature fund locks
- Conditional release mechanisms
- Dispute resolution support
- Time-based escrow management

**Key Data Structures**:
```rust
pub struct EscrowInfo {
    pub id: u64,
    pub property_id: u64,
    pub buyer: AccountId,
    pub seller: AccountId,
    pub amount: u128,
    pub released: bool,
}
```

**State Machine**:
```
Created → Funded → InDispute → Resolved → Released
                ↓
            Cancelled
```

---

### 3. Compliance Registry Component

**Purpose**: Regulatory compliance and identity verification

**Responsibilities**:
- KYC/AML verification tracking
- Jurisdiction-specific compliance rules
- Sanctions screening
- GDPR consent management
- Risk assessment

**Key Data Structures**:
```rust
pub struct ComplianceData {
    pub status: VerificationStatus,
    pub jurisdiction: Jurisdiction,
    pub risk_level: RiskLevel,
    pub kyc_hash: [u8; 32],
    pub aml_checked: bool,
    pub sanctions_checked: bool,
    pub consent_status: ConsentStatus,
}
```

**Compliance Flow**:
```
User Registration → KYC Submission → AML Check → Sanctions Screen 
→ Risk Assessment → Compliance Status Update → Ongoing Monitoring
```

---

### 4. Property Bridge Component

**Purpose**: Cross-chain asset transfer infrastructure

**Responsibilities**:
- Multi-signature bridge operations
- Chain abstraction and routing
- Asset locking and minting
- Validator coordination

**Key Data Structures**:
```rust
pub struct BridgeRequest {
    pub id: u64,
    pub token_id: TokenId,
    pub source_chain: ChainId,
    pub destination_chain: ChainId,
    pub recipient: AccountId,
    pub required_signatures: u8,
    pub current_signatures: Vec<Signature>,
    pub status: BridgeStatus,
}
```

**Bridge Process**:
```
Initiate → Lock Asset → Collect Signatures → Verify Threshold 
→ Execute Transfer → Mint/Burn on Destination
```

---

### 5. Insurance Component

**Purpose**: Decentralized property insurance marketplace

**Responsibilities**:
- Risk pool management
- Premium calculation
- Policy issuance
- Claims processing
- Reinsurance coordination

**Key Data Structures**:
```rust
pub struct InsurancePolicy {
    pub policy_id: u64,
    pub property_id: u64,
    pub coverage_type: CoverageType,
    pub coverage_amount: u128,
    pub premium_amount: u128,
    pub start_time: u64,
    pub end_time: u64,
    pub status: PolicyStatus,
}

pub struct RiskPool {
    pub pool_id: u64,
    pub total_liquidity: u128,
    pub contributors: Vec<(AccountId, u128)>,
    pub active_policies: u64,
}
```

---

### 6. Valuation Oracle Component

**Purpose**: Real-time property valuation from multiple sources

**Responsibilities**:
- Price feed aggregation
- Outlier detection
- Confidence scoring
- Historical data tracking

**Key Data Structures**:
```rust
pub struct PropertyValuation {
    pub property_id: u64,
    pub valuation: u128,
    pub confidence_score: u32,
    pub sources_used: u32,
    pub last_updated: u64,
    pub valuation_method: ValuationMethod,
}
```

**Valuation Process**:
```
Query Multiple Sources → Filter Outliers → Weighted Average 
→ Confidence Calculation → Update On-Chain
```

---

## Component Interaction Matrix

| Component | Registry | Escrow | Compliance | Bridge | Insurance | Oracle |
|-----------|----------|--------|------------|--------|-----------|--------|
| **Registry** | — | Creates escrows for transfers | Verifies ownership compliance | Initiates cross-chain transfers | Registers insured properties | Requests valuations |
| **Escrow** | Reads property info | — | Checks buyer/seller compliance | Handles bridge escrows | Manages claim escrows | Uses valuation for pricing |
| **Compliance** | Updates ownership records | Monitors escrow parties | — | Validates bridge recipients | Checks policyholder eligibility | N/A |
| **Bridge** | Locks/unlocks property tokens | Secures bridge transfers | Ensures cross-chain compliance | — | N/A | N/A |
| **Insurance** | Links policies to properties | Manages claim payouts | Verifies insurable interest | N/A | — | Uses oracle for risk assessment |
| **Oracle** | Provides property valuations | Supplies pricing data | N/A | N/A | Provides risk data | — |

---

## Data Flow Architecture

### Property Registration Flow

```
┌──────────┐     ┌──────────────┐     ┌──────────────┐     ┌──────────┐
│  Owner   │────▶│  Property    │────▶│  Compliance  │────▶│  IPFS    │
│          │     │  Registry    │     │  Registry    │     │ Storage  │
└──────────┘     └──────────────┘     └──────────────┘     └──────────┘
     │                   │                    │                   │
     │  1. Submit        │  2. Validate       │  3. Verify KYC    │
     │     Metadata      │     Metadata       │     Owner         │
     │                   │                    │                   │
     │                   │  4. Register       │                   │
     │◀──────────────────┼──── Property ID    │                   │
     │                   │                    │                   │
     │                   │  5. Store Metadata │                   │
     │                   ├───────────────────▶│                   │
     │                   │                    │                   │
     │  6. Return        │                    │                   │
     │◀──────────────────┤                    │                   │
     │                   │                    │                   │
```

### Property Transfer Flow

```
┌────────┐   ┌────────┐   ┌──────────┐   ┌────────┐   ┌──────────┐
│ Buyer  │   │ Seller │   │  Escrow  │   │Registry│   │Compliance│
└───┬────┘   └───┬────┘   └────┬─────┘   └───┬────┘   └────┬─────┘
    │            │             │             │              │
    │  1. Agree  │             │             │              │
    │◀──────────▶│             │             │              │
    │            │             │             │              │
    │            │  2. Create  │             │              │
    │            │──Escrow────▶│             │              │
    │            │             │             │              │
    │  3. Verify │             │             │              │
    │◀───────────────────────────────────────┼──────────────┤
    │            │             │             │              │
    │  4. Deposit Funds        │             │              │
    │───────────▶│             │             │              │
    │            │             │             │              │
    │            │  5. Transfer Property     │              │
    │            │────────────▶│────────────▶│              │
    │            │             │             │              │
    │            │  6. Release Funds         │              │
    │            │◀────────────┤             │              │
    │            │             │             │              │
    │  7. Confirm Transfer     │             │              │
    │◀─────────────────────────┼─────────────┤              │
    │            │             │             │              │
```

### Cross-Chain Bridge Flow

```
Source Chain                              Destination Chain
┌──────────────┐                          ┌──────────────┐
│   User       │                          │   Recipient  │
└──────┬───────┘                          └──────┬───────┘
       │                                         │
       │ 1. Initiate Bridge                      │
       ├────────────────────────────────────────▶│
       │                                         │
       │ 2. Lock Asset                           │
       ▼                                         │
┌──────────────┐                                 │
│ Bridge Lock  │                                 │
│   Contract   │                                 │
└──────┬───────┘                                 │
       │                                         │
       │ 3. Collect Signatures                   │
       ├────────────────────────────────────────▶│
       │                                         │
       │ 4. Verify Threshold                     │
       │◀────────────────────────────────────────┤
       │                                         │
       │ 5. Execute & Mint                       │
       ├────────────────────────────────────────▶│
       │                                         ▼
       │                                ┌──────────────┐
       │                                │ Bridge Mint  │
       │                                │   Contract   │
       │                                └──────────────┘
       │                                         │
       │ 6. Complete                             │
       ◀─────────────────────────────────────────┤
```

---

## Technology Stack

### Blockchain Layer
- **Framework**: Substrate 2.0+
- **Smart Contracts**: ink! 5.0
- **Runtime**: Wasm (WebAssembly)
- **Consensus**: NPoS/GRANDPA (Polkadot)
- **Network**: Polkadot, Kusama, Parachains

### Smart Contract Dependencies
```toml
ink = "5.0.0"
parity-scale-codec = "3.6.9"
scale-info = "2.10.0"
```

### External Integrations
- **Identity**: KYC/AML providers (Jumio, Onfido)
- **Storage**: IPFS, Arweave
- **Oracles**: Chainlink, custom price feeds
- **Compliance**: Sanctions lists (OFAC, UN), PEP databases
- **Payments**: Fiat on-ramps, stablecoin gateways

### Development Tools
- **Build**: Cargo, wasm32-unknown-unknown target
- **Testing**: ink! testing framework, E2E tests
- **Deployment**: polkadot.js/api, subxt
- **Monitoring**: Substrate telemetry, custom dashboards

---

## Deployment Architecture

### Network Topology

```
┌─────────────────────────────────────────────────────────────┐
│                    Production Environment                    │
│                                                              │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐      │
│  │  Polkadot    │  │   Kusama     │  │  Parachain   │      │
│  │  Mainnet     │  │  (Canary)    │  │  (Specialized)│      │
│  └──────────────┘  └──────────────┘  └──────────────┘      │
│                                                              │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐      │
│  │   Westend    │  │   Local      │  │    Test      │      │
│  │  (Testnet)   │  │  Dev Node    │  │  Networks    │      │
│  └──────────────┘  └──────────────┘  └──────────────┘      │
└─────────────────────────────────────────────────────────────┘
```

### Contract Deployment Strategy

1. **Development**: Local Substrate node with instant finality
2. **Testing**: Westend testnet for public testing
3. **Staging**: Canary deployment on Kusama
4. **Production**: Polkadot mainnet with upgrade governance

### Upgrade Mechanism

```
Proposal → Governance Vote → Timelock → Proxy Upgrade → Migration
```

---

## Security Architecture

### Defense in Depth

**Layer 1: Code Level**
- Formal verification of critical functions
- Comprehensive test coverage (>90%)
- Static analysis (Clippy, cargo-audit)
- Manual code audits

**Layer 2: Runtime Protection**
- Reentrancy guards
- Access control (RBAC)
- Rate limiting
- Circuit breakers (pause mechanism)

**Layer 3: Operational Security**
- Multi-signature admin controls
- Time-locked upgrades
- Emergency response procedures
- Bug bounty program

### Access Control Model

```
┌─────────────────────────────────────────┐
│           Role Hierarchy                │
├─────────────────────────────────────────┤
│  Admin (Superuser)                      │
│    └─> Pause Guardian                   │
│         └─> Agent                       │
│              └─> Verified User          │
│                   └─> Public (Read-only)│
└─────────────────────────────────────────┘
```

### Security Patterns

1. **Checks-Effects-Interactions**: Prevent reentrancy
2. **Pull over Push Payments**: Avoid gas issues
3. **Circuit Breaker**: Emergency pause
4. **Rate Limiting**: Prevent abuse
5. **Multi-sig**: Distributed trust

---

## Performance Architecture

### Scalability Strategies

**Horizontal Scaling**:
- Sharding via parachains
- State channels for micro-transactions
- Layer 2 rollups for batch operations

**Vertical Optimization**:
- Efficient storage (Mapping vs Vec)
- Lazy evaluation
- Batch operations
- Gas optimization

### Caching Strategy

```
┌─────────────────────────────────────────┐
│         Caching Layers                  │
├─────────────────────────────────────────┤
│  L1: On-chain State (Hot)              │
│  L2: Indexer Cache (Warm)              │
│  L3: CDN/Edge Cache (Cool)             │
│  L4: IPFS/Arweave (Cold)               │
└─────────────────────────────────────────┘
```

### Gas Optimization Techniques

1. **Storage Optimization**
   - Use `Mapping` instead of `Vec` for large datasets
   - Pack structs to minimize storage slots
   - Remove unnecessary state variables

2. **Computation Optimization**
   - Batch multiple operations
   - Lazy evaluation of expensive computations
   - Event emission instead of storage writes

3. **Memory Management**
   - Minimize allocations
   - Use references over clones
   - Early returns to avoid unnecessary work

---

## Monitoring & Observability

### Metrics Collection

**On-Chain Metrics**:
- Contract events (PropertyRegistered, TransferCompleted)
- Gas usage per operation
- State changes
- Error rates

**Off-Chain Metrics**:
- API response times
- Frontend performance
- User adoption metrics
- Transaction success rates

### Health Check System

```rust
pub struct HealthStatus {
    pub is_healthy: bool,
    pub is_paused: bool,
    pub contract_version: u32,
    pub property_count: u64,
    pub escrow_count: u64,
    pub has_oracle: bool,
    pub has_compliance_registry: bool,
    pub has_fee_manager: bool,
    pub block_number: u32,
    pub timestamp: u64,
}
```

### Alerting Framework

**Alert Levels**:
- **Critical**: Contract paused, security breach
- **High**: Compliance failures, oracle manipulation
- **Medium**: Performance degradation, high error rates
- **Low**: Non-critical errors, warnings

---

## Disaster Recovery

### Backup Strategy

1. **On-Chain Data**: Inherently replicated across nodes
2. **IPFS Content**: Pin across multiple nodes
3. **Off-Chain Databases**: Regular snapshots + WAL archiving
4. **Contract State**: Periodic state exports

### Recovery Procedures

**Scenario 1: Contract Bug**
1. Pause contract immediately
2. Deploy fixed implementation
3. Migrate state via proxy
4. Resume operations

**Scenario 2: Data Corruption**
1. Identify corruption point
2. Restore from last known good snapshot
3. Replay valid transactions
4. Verify state integrity

**Scenario 3: Oracle Manipulation**
1. Halt valuation-dependent operations
2. Switch to backup oracle sources
3. Investigate and filter bad actors
4. Resume with enhanced validation

---

## Future Architecture Considerations

### Planned Enhancements

1. **AI-Powered Valuation**
   - Machine learning models for property pricing
   - Predictive analytics for market trends
   - Automated comparative market analysis

2. **DeFi Integration**
   - Property-backed lending protocols
   - Liquidity pools for property tokens
   - Yield farming opportunities

3. **DAO Governance**
   - Community-driven protocol upgrades
   - Treasury management
   - Parameter adjustment via governance

4. **Privacy Features**
   - Zero-knowledge compliance proofs
   - Private transactions (optional)
   - Selective disclosure mechanisms

### Emerging Technology Integration

- **zk-Rollups**: Scale transaction throughput
- **Account Abstraction**: Improve UX with smart wallets
- **Cross-Chain Messaging**: Native interoperability (XCM)
- **NFT Fractionalization**: Increased liquidity

---

## Conclusion

The PropChain architecture provides a robust, scalable foundation for real estate tokenization. Its modular design allows for incremental upgrades while maintaining security and compliance. The system balances decentralization with practical regulatory requirements, creating a production-ready platform for blockchain-based property transactions.

For detailed implementation specifics, refer to:
- [Contract API Documentation](./contracts.md)
- [Deployment Guide](./deployment.md)
- [Security Best Practices](./best-practices.md)
- [Integration Guide](./integration.md)
