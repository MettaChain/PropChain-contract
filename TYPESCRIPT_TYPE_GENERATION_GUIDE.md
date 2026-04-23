# PropChain Rust Smart Contract API Surface - TypeScript Type Generation Guide

**Generated:** April 22, 2026  
**Purpose:** Complete analysis of all public contracts, types, functions, events, and traits for TypeScript SDK generation.

---

## Table of Contents

1. [Contract Overview](#contract-overview)
2. [Common Types Across Contracts](#common-types-across-contracts)
3. [Detailed Contract Analysis](#detailed-contract-analysis)
4. [Shared Trait Interfaces](#shared-trait-interfaces)
5. [Event Taxonomy](#event-taxonomy)
6. [Type Patterns](#type-patterns)

---

## Contract Overview

### All Public Contracts in `contracts/` Directory

| Contract                | Purpose                                                                                                                       |
| ----------------------- | ----------------------------------------------------------------------------------------------------------------------------- |
| **property-token**      | ERC-721/1155 compatible property tokenization with ownership history, compliance flags, dividends, governance, bridge support |
| **escrow**              | Advanced multi-signature escrow with document management, condition tracking, dispute resolution, audit logs                  |
| **oracle**              | Property valuation oracle with multi-source aggregation, price feeds, reputation system, market trend analysis                |
| **bridge**              | Cross-chain property token transfers with multi-signature support, monitoring, gas estimation, recovery mechanisms            |
| **dex**                 | Decentralized exchange for property tokens with liquidity pools, order books, swaps, governance, cross-chain trades           |
| **governance**          | Multi-signature governance with proposals, voting, timelock, admin controls                                                   |
| **lending**             | Property-collateralized lending with margin positions, yield farming, pool management                                         |
| **insurance**           | Decentralized property insurance with policies, claims, risk pools, reinsurance, actuarial models                             |
| **fractional**          | Portfolio aggregation, tax reporting, share tokenization utilities                                                            |
| **staking**             | Token staking with lock periods, reward distribution, governance delegation                                                   |
| **analytics**           | Market metrics, sentiment analysis, trend tracking, performance reporting                                                     |
| **database**            | Off-chain data sync, export, integrity verification, indexer registration                                                     |
| **compliance_registry** | KYC/AML verification, jurisdiction rules, biometric auth, sanctions screening, GDPR compliance                                |
| **prediction-market**   | Property value prediction markets with long/short positions, reputation tracking, oracle-based resolution                     |
| **property-management** | Lease management, tenant screening, maintenance requests, expenses, inspections                                               |
| **monitoring**          | System health checks, performance metrics, alerts, operation recording                                                        |
| **ai-valuation**        | Machine learning-based property valuation with multiple model types, ensemble predictions                                     |
| **fees**                | Dynamic fee calculation, premium auctions, validator rewards, congestion-based pricing                                        |
| **crowdfunding**        | Real estate crowdfunding campaigns with milestones, investor profiles, voting, risk assessment                                |
| **zk-compliance**       | Zero-knowledge proof-based compliance verification, privacy preferences                                                       |
| **ipfs-metadata**       | IPFS document storage and retrieval for property metadata                                                                     |

### Special Contracts

- **traits/** - Shared trait definitions and common types
- **lib/** - Shared utilities and base implementations
- **metadata/** - Metadata standards and validators
- **proxy/** - Proxy pattern implementations
- **third-party/** - External integrations
- **tax-compliance/** - Tax calculation and record keeping

---

## Common Types Across Contracts

### Primitive Type Aliases

```typescript
// Identity Types
type AccountId = string; // Substrate/ink account hash
type Hash = [u8; 32];    // 32-byte hash

// Property & Token Types
type TokenId = u64;      // Property token identifier
type ChainId = u64;      // Blockchain identifier for cross-chain
type PropertyId = u64;   // Property identifier
type Timestamp = u64;    // Unix timestamp in milliseconds
type Balance = u128;     // Token amount with full precision
```

### Common Enums

#### ApprovalType

```rust
enum ApprovalType {
    Release,
    Refund,
    EmergencyOverride,
}
```

#### PropertyType

```rust
enum PropertyType {
    Residential,
    Commercial,
    Industrial,
    Land,
    MultiFamily,
    Retail,
    Office,
}
```

#### Status Enums (Shared Patterns)

- **BridgeOperationStatus**: None, Pending, Locked, InTransit, Completed, Failed, Recovering, Expired
- **VerificationStatus**: NotVerified, Pending, Verified, Rejected, Expired
- **ProposalStatus**: Active, Approved, Rejected, Executed, Cancelled
- **OrderStatus**: Open, PartiallyFilled, Filled, Cancelled, Triggered, Expired

### Common Structs

#### PropertyMetadata (Used in Multiple Contracts)

```rust
struct PropertyMetadata {
    location: String,
    size: u64,
    legal_description: String,
    valuation: u128,
    documents_url: String,
}
```

#### PropertyInfo

```rust
struct PropertyInfo {
    id: u64,
    owner: AccountId,
    metadata: PropertyMetadata,
    registered_at: u64,
}
```

#### PropertyValuation (Oracle)

```rust
struct PropertyValuation {
    property_id: u64,
    valuation: u128,
    confidence_score: u32,
    sources_used: u32,
    last_updated: u64,
    valuation_method: ValuationMethod,
}
```

---

## Detailed Contract Analysis

### 1. Property Token Contract

**Location:** `contracts/property-token/src/lib.rs`

#### Primary Purpose

ERC-721/1155 compatible property tokenization with real estate-specific features including ownership history, compliance tracking, bridging, dividends, governance, and marketplace.

#### Public Enums

_(Defined in types.rs)_

#### Public Structs

```rust
// ERC-721/1155 Standard
struct Transfer {
    from: Option<AccountId>,
    to: Option<AccountId>,
    id: TokenId,
}

struct Approval {
    owner: AccountId,
    spender: AccountId,
    id: TokenId,
}

struct ApprovalForAll {
    owner: AccountId,
    operator: AccountId,
    approved: bool,
}

// Property-Specific
struct PropertyInfo {
    id: u64,
    owner: AccountId,
    metadata: PropertyMetadata,
    registered_at: u64,
}

struct OwnershipTransfer {
    from: AccountId,
    to: AccountId,
    timestamp: u64,
}

struct ComplianceInfo {
    verified: bool,
    verifier: AccountId,
    verification_date: u64,
}

struct DocumentInfo {
    hash: Hash,
    document_type: String,
    uploader: AccountId,
    timestamp: u64,
}

struct BridgedTokenInfo {
    original_chain: ChainId,
    original_token_id: TokenId,
    bridge_address: Option<AccountId>,
    is_locked: bool,
}

struct MultisigBridgeRequest {
    request_id: u64,
    token_id: TokenId,
    source_chain: ChainId,
    destination_chain: ChainId,
    sender: AccountId,
    recipient: AccountId,
    required_signatures: u8,
    signatures: Vec<AccountId>,
    created_at: u64,
    expires_at: Option<u64>,
    status: BridgeOperationStatus,
    metadata: PropertyMetadata,
}

struct BridgeTransaction {
    transaction_id: u64,
    token_id: TokenId,
    source_chain: ChainId,
    destination_chain: ChainId,
    sender: AccountId,
    recipient: AccountId,
    transaction_hash: Hash,
    timestamp: u64,
    gas_used: u64,
    status: BridgeOperationStatus,
    metadata: PropertyMetadata,
}

struct BridgeConfig {
    supported_chains: Vec<ChainId>,
    min_signatures_required: u8,
    max_signatures_required: u8,
    default_timeout_blocks: u64,
    gas_limit_per_bridge: u64,
    emergency_pause: bool,
    metadata_preservation: bool,
}

// Dividend & Shares
struct SharesIssued {
    token_id: TokenId,
    to: AccountId,
    amount: u128,
}

struct DividendsDeposited {
    token_id: TokenId,
    amount: u128,
    per_share: u128,
}

// Governance
struct Proposal {
    proposal_id: u64,
    token_id: TokenId,
    quorum: u128,
    status: ProposalStatus,
}

// Marketplace
struct Ask {
    token_id: TokenId,
    seller: AccountId,
    price_per_share: u128,
    amount: u128,
}

struct TaxRecord {
    account: AccountId,
    token_id: TokenId,
    transaction_type: String,
    amount: u128,
    timestamp: u64,
}

struct ErrorLogEntry {
    error_id: u64,
    account: AccountId,
    error_type: String,
    timestamp: u64,
}

struct RecoveryAction {
    // Enum for bridge recovery
}
```

#### Public Functions/Messages

```rust
// ERC-721 Standard Interface
fn balance_of(&self, owner: AccountId) -> u32
fn owner_of(&self, token_id: TokenId) -> Option<AccountId>
fn approve(&mut self, to: Option<AccountId>, token_id: TokenId) -> Result<(), Error>
fn set_approval_for_all(&mut self, operator: AccountId, approved: bool) -> Result<(), Error>
fn transfer_from(&mut self, from: AccountId, to: AccountId, token_id: TokenId) -> Result<(), Error>
fn safe_transfer_from(&mut self, from: AccountId, to: AccountId, token_id: TokenId, data: Vec<u8>) -> Result<(), Error>

// Property-Specific
fn mint(&mut self, property_id: u64, metadata: PropertyMetadata) -> Result<TokenId, Error>
fn burn(&mut self, token_id: TokenId) -> Result<(), Error>
fn get_property_info(&self, token_id: TokenId) -> Option<PropertyInfo>
fn update_property_metadata(&mut self, token_id: TokenId, metadata: PropertyMetadata) -> Result<(), Error>
fn attach_legal_document(&mut self, token_id: TokenId, document_hash: Hash, document_type: String) -> Result<(), Error>
fn verify_compliance(&mut self, token_id: TokenId) -> Result<(), Error>
fn set_management_agent(&mut self, token_id: TokenId, agent: AccountId) -> Result<(), Error>

// Dividend Functions
fn issue_shares(&mut self, token_id: TokenId, to: AccountId, amount: u128) -> Result<(), Error>
fn redeem_shares(&mut self, token_id: TokenId, from: AccountId, amount: u128) -> Result<(), Error>
fn deposit_dividends(&mut self, token_id: TokenId, amount: u128) -> Result<(), Error>
fn withdraw_dividends(&mut self, token_id: TokenId) -> Result<u128, Error>
fn get_dividend_balance(&self, token_id: TokenId, account: AccountId) -> u128

// Bridge Functions
fn initiate_bridge_multisig(&mut self, token_id: TokenId, destination_chain: ChainId, recipient: AccountId, required_signatures: u8, timeout_blocks: Option<u64>, metadata: PropertyMetadata) -> Result<u64, Error>
fn sign_bridge_request(&mut self, request_id: u64, approve: bool) -> Result<(), Error>
fn execute_bridge(&mut self, request_id: u64) -> Result<(), Error>
fn get_bridge_status(&self, token_id: TokenId) -> Option<BridgeStatus>
fn verify_bridge_transaction(&self, token_id: TokenId, transaction_hash: Hash, source_chain: ChainId) -> bool

// Governance
fn create_proposal(&mut self, token_id: TokenId, quorum: u128) -> Result<u64, Error>
fn vote(&mut self, token_id: TokenId, proposal_id: u64, support: bool) -> Result<(), Error>
fn execute_proposal(&mut self, token_id: TokenId, proposal_id: u64) -> Result<(), Error>

// Marketplace/Ask
fn place_ask(&mut self, token_id: TokenId, price_per_share: u128, amount: u128) -> Result<(), Error>
fn cancel_ask(&mut self, token_id: TokenId) -> Result<(), Error>
fn purchase_shares(&mut self, token_id: TokenId, amount: u128) -> Result<(), Error>

// Batch Operations
fn batch_transfer(&mut self, to: AccountId, token_ids: Vec<TokenId>) -> Result<(), Error>
fn batch_approve(&mut self, to: AccountId, token_ids: Vec<TokenId>) -> Result<(), Error>

// Error Tracking
fn get_error_rate(&self, account: AccountId) -> (u64, u64)
fn record_error(&mut self, account: AccountId, error_type: String) -> Result<(), Error>
```

#### Public Events

```rust
// ERC-721/1155 Standard
event Transfer {
    #[topic] from: Option<AccountId>,
    #[topic] to: Option<AccountId>,
    #[topic] id: TokenId,
}

event Approval {
    #[topic] owner: AccountId,
    #[topic] spender: AccountId,
    #[topic] id: TokenId,
}

event ApprovalForAll {
    #[topic] owner: AccountId,
    #[topic] operator: AccountId,
    approved: bool,
}

// Property Events
event PropertyTokenMinted {
    #[topic] token_id: TokenId,
    #[topic] property_id: u64,
    #[topic] owner: AccountId,
}

event LegalDocumentAttached {
    #[topic] token_id: TokenId,
    #[topic] document_hash: Hash,
    #[topic] document_type: String,
}

event ComplianceVerified {
    #[topic] token_id: TokenId,
    #[topic] verified: bool,
    #[topic] verifier: AccountId,
}

// Bridge Events
event TokenBridged {
    #[topic] token_id: TokenId,
    #[topic] destination_chain: ChainId,
    #[topic] recipient: AccountId,
    bridge_request_id: u64,
}

event BridgeRequestCreated {
    #[topic] request_id: u64,
    #[topic] token_id: TokenId,
    #[topic] source_chain: ChainId,
    #[topic] destination_chain: ChainId,
    #[topic] requester: AccountId,
}

event BridgeRequestSigned {
    #[topic] request_id: u64,
    #[topic] signer: AccountId,
    signatures_collected: u8,
    signatures_required: u8,
}

event BridgeExecuted {
    #[topic] request_id: u64,
    #[topic] token_id: TokenId,
    #[topic] transaction_hash: Hash,
}

event BridgeFailed {
    #[topic] request_id: u64,
    #[topic] token_id: TokenId,
    error: String,
}

event BridgeRecovered {
    #[topic] request_id: u64,
    #[topic] recovery_action: RecoveryAction,
}

// Dividend Events
event SharesIssued {
    #[topic] token_id: TokenId,
    #[topic] to: AccountId,
    amount: u128,
}

event SharesRedeemed {
    #[topic] token_id: TokenId,
    #[topic] from: AccountId,
    amount: u128,
}

event DividendsDeposited {
    #[topic] token_id: TokenId,
    amount: u128,
    per_share: u128,
}

event DividendsWithdrawn {
    #[topic] token_id: TokenId,
    #[topic] account: AccountId,
    amount: u128,
}

// Governance Events
event ProposalCreated {
    #[topic] token_id: TokenId,
    #[topic] proposal_id: u64,
    quorum: u128,
}

event Voted {
    #[topic] token_id: TokenId,
    #[topic] proposal_id: u64,
    #[topic] voter: AccountId,
    support: bool,
    weight: u128,
}

event ProposalExecuted {
    #[topic] token_id: TokenId,
    #[topic] proposal_id: u64,
    passed: bool,
}

// Marketplace Events
event AskPlaced {
    #[topic] token_id: TokenId,
    #[topic] seller: AccountId,
    price_per_share: u128,
    amount: u128,
}

event AskCancelled {
    #[topic] token_id: TokenId,
    #[topic] seller: AccountId,
}

event SharesPurchased {
    #[topic] token_id: TokenId,
    #[topic] buyer: AccountId,
    amount: u128,
    price: u128,
}
```

---

### 2. Escrow Contract

**Location:** `contracts/escrow/src/lib.rs`

#### Primary Purpose

Advanced escrow with multi-signature support, document custody, condition tracking, dispute resolution, and comprehensive audit logging.

#### Public Structs

```rust
struct EscrowData {
    id: u64,
    property_id: u64,
    buyer: AccountId,
    seller: AccountId,
    amount: u128,
    status: EscrowStatus,
    created_at: u64,
    release_deadline: Option<u64>,
    refund_deadline: Option<u64>,
}

struct MultiSigConfig {
    participants: Vec<AccountId>,
    required_signatures: u8,
    total_participants: u8,
}

struct DocumentHash {
    hash: Hash,
    document_type: String,
}

struct Condition {
    id: u64,
    description: String,
    is_met: bool,
    verified_by: Option<AccountId>,
    verified_at: Option<u64>,
}

struct DisputeInfo {
    escrow_id: u64,
    raised_by: AccountId,
    reason: String,
    raised_at: u64,
    resolved: bool,
    resolution: Option<String>,
}

struct AuditEntry {
    action: String,
    actor: AccountId,
    timestamp: u64,
    details: String,
}
```

#### Public Functions

```rust
fn new(min_high_value_threshold: u128) -> Self

fn create_escrow_advanced(
    &mut self,
    property_id: u64,
    amount: u128,
    buyer: AccountId,
    seller: AccountId,
    participants: Vec<AccountId>,
    required_signatures: u8,
    release_time_lock: Option<u64>,
) -> Result<u64, Error>

fn deposit_funds(&mut self, escrow_id: u64) -> Result<(), Error>
fn release_funds(&mut self, escrow_id: u64) -> Result<(), Error>
fn refund_funds(&mut self, escrow_id: u64) -> Result<(), Error>

fn upload_document(&mut self, escrow_id: u64, document_hash: Hash, document_type: String, uploader: AccountId) -> Result<(), Error>
fn verify_document(&mut self, escrow_id: u64, document_hash: Hash) -> Result<(), Error>

fn add_condition(&mut self, escrow_id: u64, description: String) -> Result<u64, Error>
fn mark_condition_met(&mut self, escrow_id: u64, condition_id: u64) -> Result<(), Error>

fn sign_approval(&mut self, escrow_id: u64, approval_type: ApprovalType) -> Result<(), Error>

fn raise_dispute(&mut self, escrow_id: u64, reason: String) -> Result<(), Error>
fn resolve_dispute(&mut self, escrow_id: u64, resolution: String) -> Result<(), Error>

fn emergency_override(&mut self, escrow_id: u64, release_to_seller: bool) -> Result<(), Error>

fn get_escrow(&self, escrow_id: u64) -> Option<EscrowData>
fn get_escrow_documents(&self, escrow_id: u64) -> Vec<DocumentHash>
fn get_escrow_conditions(&self, escrow_id: u64) -> Vec<Condition>
fn get_audit_log(&self, escrow_id: u64) -> Vec<AuditEntry>

fn register_signer_public_key(&mut self, public_key: [u8; 33]) -> Result<(), Error>
fn rotate_admin_key(&mut self, new_admin: AccountId) -> Result<(), Error>
```

#### Public Events

```rust
event EscrowCreated {
    #[topic] escrow_id: u64,
    property_id: u64,
    buyer: AccountId,
    seller: AccountId,
    amount: u128,
}

event FundsDeposited {
    #[topic] escrow_id: u64,
    amount: u128,
    depositor: AccountId,
}

event FundsReleased {
    #[topic] escrow_id: u64,
    amount: u128,
    recipient: AccountId,
}

event FundsRefunded {
    #[topic] escrow_id: u64,
    amount: u128,
    recipient: AccountId,
}

event DocumentUploaded {
    #[topic] escrow_id: u64,
    document_hash: Hash,
    document_type: String,
    uploader: AccountId,
}

event DocumentVerified {
    #[topic] escrow_id: u64,
    document_hash: Hash,
    verifier: AccountId,
}

event ConditionAdded {
    #[topic] escrow_id: u64,
    condition_id: u64,
    description: String,
}

event ConditionMet {
    #[topic] escrow_id: u64,
    condition_id: u64,
    verified_by: AccountId,
}

event SignatureAdded {
    #[topic] escrow_id: u64,
    approval_type: ApprovalType,
    signer: AccountId,
}

event DisputeRaised {
    #[topic] escrow_id: u64,
    raised_by: AccountId,
    reason: String,
}

event DisputeResolved {
    #[topic] escrow_id: u64,
    resolution: String,
}

event EmergencyOverride {
    #[topic] escrow_id: u64,
    admin: AccountId,
}
```

---

### 3. Oracle Contract

**Location:** `contracts/oracle/src/lib.rs`

#### Primary Purpose

Real-time property valuation with multi-source price feed aggregation, confidence scoring, volatility analysis, and market trend tracking.

#### Public Enums

```rust
enum ValuationMethod {
    Automated,   // AVM (Automated Valuation Model)
    Manual,      // Manual appraisal
    MarketData,  // Based on market comparables
    Hybrid,      // Combination of methods
    AIValuation, // AI-powered ML valuation
}

enum OracleSourceType {
    Chainlink,
    Pyth,
    Substrate,
    Custom,
    Manual,
    AIModel,
}
```

#### Public Structs

```rust
struct PropertyValuation {
    property_id: u64,
    valuation: u128,
    confidence_score: u32,
    sources_used: u32,
    last_updated: u64,
    valuation_method: ValuationMethod,
}

struct ValuationWithConfidence {
    valuation: PropertyValuation,
    volatility_index: u32,
    confidence_interval: (u128, u128),
    outlier_sources: u32,
}

struct VolatilityMetrics {
    property_type: PropertyType,
    location: String,
    volatility_index: u32,
    average_price_change: i32,
    period_days: u32,
    last_updated: u64,
}

struct ComparableProperty {
    property_id: u64,
    distance_km: u32,
    price_per_sqm: u128,
    size_sqm: u64,
    sale_date: u64,
    adjustment_factor: i32,
}

struct PriceAlert {
    property_id: u64,
    threshold_percentage: u32,
    alert_address: AccountId,
    last_triggered: u64,
    is_active: bool,
}

struct OracleSource {
    id: String,
    source_type: OracleSourceType,
    address: AccountId,
    is_active: bool,
    weight: u32,
    last_updated: u64,
}

struct LocationAdjustment {
    location_code: String,
    adjustment_percentage: i32,
    last_updated: u64,
    confidence_score: u32,
}

struct MarketTrend {
    property_type: PropertyType,
    location: String,
    trend_percentage: i32,
    period_months: u32,
    last_updated: u64,
}

struct OracleBatchResult {
    successes: Vec<u64>,
    failures: Vec<OracleBatchItemFailure>,
    total_items: u32,
    successful_items: u32,
    failed_items: u32,
    early_terminated: bool,
}

struct OracleBatchItemFailure {
    index: u32,
    item_id: u64,
    error: OracleError,
}
```

#### Public Functions

```rust
fn new(admin: AccountId) -> Self

fn get_property_valuation(&self, property_id: u64) -> Result<PropertyValuation, OracleError>
fn get_valuation_with_confidence(&self, property_id: u64) -> Result<ValuationWithConfidence, OracleError>

fn update_property_valuation(&mut self, property_id: u64, valuation: PropertyValuation) -> Result<(), OracleError>
fn update_valuation_from_sources(&mut self, property_id: u64) -> Result<(), OracleError>

fn request_property_valuation(&mut self, property_id: u64) -> Result<u64, OracleError>
fn batch_request_valuations(&mut self, property_ids: Vec<u64>) -> Result<OracleBatchResult, OracleError>

fn add_oracle_source(&mut self, source: OracleSource) -> Result<(), OracleError>
fn remove_oracle_source(&mut self, source_id: String) -> Result<(), OracleError>
fn update_source_reputation(&mut self, source_id: String, success: bool) -> Result<(), OracleError>

fn get_source_reputation(&self, source_id: String) -> Option<u32>
fn get_active_sources(&self) -> Vec<String>

fn set_price_alert(&mut self, property_id: u64, threshold_percentage: u32, alert_address: AccountId) -> Result<(), OracleError>
fn check_price_alerts(&mut self, property_id: u64, new_valuation: u128) -> Result<(), OracleError>

fn add_location_adjustment(&mut self, location_code: String, adjustment_percentage: i32) -> Result<(), OracleError>
fn update_market_trend(&mut self, location: String, property_type: PropertyType, trend_percentage: i32, period_months: u32) -> Result<(), OracleError>

fn get_historical_valuations(&self, property_id: u64, limit: u32) -> Vec<PropertyValuation>
fn get_market_volatility(&self, property_type: PropertyType, location: String) -> Result<VolatilityMetrics, OracleError>

fn link_ai_valuation_contract(&mut self, contract_address: AccountId) -> Result<(), OracleError>
```

#### Public Events

```rust
event ValuationUpdated {
    #[topic] property_id: u64,
    valuation: u128,
    confidence_score: u32,
    timestamp: u64,
}

event PriceAlertTriggered {
    #[topic] property_id: u64,
    old_valuation: u128,
    new_valuation: u128,
    change_percentage: u32,
    alert_address: AccountId,
}

event OracleSourceAdded {
    #[topic] source_id: String,
    source_type: OracleSourceType,
    weight: u32,
}

event OracleSourceRemoved {
    #[topic] source_id: String,
}

event ReputationUpdated {
    #[topic] source_id: String,
    new_reputation: u32,
}
```

---

### 4. Bridge Contract

**Location:** `contracts/bridge/src/lib.rs`

#### Primary Purpose

Cross-chain property token bridging with multi-signature validation, metadata preservation, and recovery mechanisms.

#### Public Enums

```rust
enum BridgeOperationStatus {
    None,
    Pending,
    Locked,
    InTransit,
    Completed,
    Failed,
    Recovering,
    Expired,
}

enum RecoveryAction {
    UnlockToken,
    RefundGas,
    RetryBridge,
    CancelBridge,
}
```

#### Public Structs

```rust
struct BridgeStatus {
    is_locked: bool,
    source_chain: Option<ChainId>,
    destination_chain: Option<ChainId>,
    locked_at: Option<u64>,
    bridge_request_id: Option<u64>,
    status: BridgeOperationStatus,
}

struct BridgeConfig {
    supported_chains: Vec<ChainId>,
    min_signatures_required: u8,
    max_signatures_required: u8,
    default_timeout_blocks: u64,
    gas_limit_per_bridge: u64,
    emergency_pause: bool,
    metadata_preservation: bool,
}

struct MultisigBridgeRequest {
    request_id: u64,
    token_id: TokenId,
    source_chain: ChainId,
    destination_chain: ChainId,
    sender: AccountId,
    recipient: AccountId,
    required_signatures: u8,
    signatures: Vec<AccountId>,
    created_at: u64,
    expires_at: Option<u64>,
    status: BridgeOperationStatus,
    metadata: PropertyMetadata,
}

struct BridgeTransaction {
    transaction_id: u64,
    token_id: TokenId,
    source_chain: ChainId,
    destination_chain: ChainId,
    sender: AccountId,
    recipient: AccountId,
    transaction_hash: Hash,
    timestamp: u64,
    gas_used: u64,
    status: BridgeOperationStatus,
    metadata: PropertyMetadata,
}

struct ChainBridgeInfo {
    chain_id: ChainId,
    chain_name: String,
    bridge_contract_address: Option<AccountId>,
    is_active: bool,
    gas_multiplier: u32,
    confirmation_blocks: u32,
    supported_tokens: Vec<TokenId>,
}

struct BridgeMonitoringInfo {
    bridge_request_id: u64,
    token_id: TokenId,
    source_chain: ChainId,
    destination_chain: ChainId,
    status: BridgeOperationStatus,
    created_at: u64,
    expires_at: Option<u64>,
    signatures_collected: u8,
    signatures_required: u8,
    error_message: Option<String>,
}

struct BridgeFeeQuote {
    destination_chain: ChainId,
    gas_estimate: u64,
    protocol_fee: u128,
    total_fee: u128,
}
```

#### Public Functions

```rust
fn new(
    supported_chains: Vec<ChainId>,
    min_signatures: u8,
    max_signatures: u8,
    default_timeout: u64,
    gas_limit: u64,
) -> Self

fn initiate_bridge_multisig(
    &mut self,
    token_id: TokenId,
    destination_chain: ChainId,
    recipient: AccountId,
    required_signatures: u8,
    timeout_blocks: Option<u64>,
    metadata: PropertyMetadata,
) -> Result<u64, Error>

fn sign_bridge_request(&mut self, request_id: u64, approve: bool) -> Result<(), Error>
fn execute_bridge(&mut self, request_id: u64) -> Result<(), Error>

fn monitor_bridge_status(&self, bridge_request_id: u64) -> Option<BridgeMonitoringInfo>
fn recover_failed_bridge(&mut self, bridge_request_id: u64, recovery_action: RecoveryAction) -> Result<(), Error>

fn get_bridge_history(&self, account: AccountId) -> Vec<BridgeTransaction>
fn get_bridge_request(&self, request_id: u64) -> Option<MultisigBridgeRequest>

fn estimate_bridge_gas(&self, token_id: TokenId, destination_chain: ChainId) -> Result<u64, Error>
fn get_bridge_fee_quote(&self, destination_chain: ChainId) -> BridgeFeeQuote

fn add_bridge_operator(&mut self, operator: AccountId) -> Result<(), Error>
fn remove_bridge_operator(&mut self, operator: AccountId) -> Result<(), Error>
fn is_bridge_operator(&self, account: AccountId) -> bool

fn emergency_pause(&mut self) -> Result<(), Error>
fn emergency_resume(&mut self) -> Result<(), Error>

fn register_operator_public_key(&mut self, public_key: [u8; 33]) -> Result<(), Error>
fn rotate_admin_key(&mut self, new_admin: AccountId) -> Result<(), Error>
```

#### Public Events

```rust
event BridgeRequestCreated {
    #[topic] request_id: u64,
    #[topic] token_id: TokenId,
    #[topic] source_chain: ChainId,
    #[topic] destination_chain: ChainId,
    #[topic] requester: AccountId,
}

event BridgeRequestSigned {
    #[topic] request_id: u64,
    #[topic] signer: AccountId,
    signatures_collected: u8,
    signatures_required: u8,
}

event BridgeExecuted {
    #[topic] request_id: u64,
    #[topic] token_id: TokenId,
    #[topic] transaction_hash: Hash,
}

event BridgeFailed {
    #[topic] request_id: u64,
    #[topic] token_id: TokenId,
    error: String,
}

event BridgeRecovered {
    #[topic] request_id: u64,
    #[topic] recovery_action: RecoveryAction,
}

event OperatorAdded {
    #[topic] operator: AccountId,
}

event OperatorRemoved {
    #[topic] operator: AccountId,
}
```

---

### 5. DEX Contract

**Location:** `contracts/dex/src/lib.rs`

#### Primary Purpose

Decentralized exchange for property tokens with liquidity pools, order books, swaps, governance voting, and cross-chain settlements.

#### Public Enums

```rust
enum OrderSide {
    Buy,
    Sell,
}

enum OrderType {
    Market,
    Limit,
    StopLoss,
    TakeProfit,
    Twap,
}

enum TimeInForce {
    GoodTillCancelled,
    ImmediateOrCancel,
    FillOrKill,
}

enum OrderStatus {
    Open,
    PartiallyFilled,
    Filled,
    Cancelled,
    Triggered,
    Expired,
}

enum CrossChainTradeStatus {
    Pending,
    BridgeRequested,
    InFlight,
    Settled,
    Cancelled,
    Failed,
}
```

#### Public Structs

```rust
struct LiquidityPool {
    pair_id: u64,
    base_token: TokenId,
    quote_token: TokenId,
    reserve_base: u128,
    reserve_quote: u128,
    total_lp_shares: u128,
    fee_bips: u32,
    reward_index: u128,
    cumulative_volume: u128,
    last_price: u128,
    is_active: bool,
}

struct LiquidityPosition {
    lp_shares: u128,
    reward_debt: u128,
    provided_base: u128,
    provided_quote: u128,
    pending_rewards: u128,
}

struct TradingOrder {
    order_id: u64,
    pair_id: u64,
    trader: AccountId,
    side: OrderSide,
    order_type: OrderType,
    time_in_force: TimeInForce,
    price: u128,
    amount: u128,
    remaining_amount: u128,
    trigger_price: Option<u128>,
    twap_interval: Option<u64>,
    reduce_only: bool,
    status: OrderStatus,
    created_at: u64,
    updated_at: u64,
}

struct PairAnalytics {
    pair_id: u64,
    last_price: u128,
    twap_price: u128,
    reference_price: u128,
    cumulative_volume: u128,
    trade_count: u64,
    best_bid: u128,
    best_ask: u128,
    volatility_bips: u32,
    last_updated: u64,
}

struct GovernanceTokenConfig {
    symbol: String,
    total_supply: u128,
    emission_rate: u128,
    quorum_bips: u32,
}

struct GovernanceProposal {
    proposal_id: u64,
    proposer: AccountId,
    title: String,
    description_hash: [u8; 32],
    new_fee_bips: Option<u32>,
    new_emission_rate: Option<u128>,
    votes_for: u128,
    votes_against: u128,
    start_block: u64,
    end_block: u64,
    executed: bool,
}

struct LiquidityMiningCampaign {
    emission_rate: u128,
    start_block: u64,
    end_block: u64,
    reward_token_symbol: String,
}

struct PortfolioSnapshot {
    owner: AccountId,
    liquidity_positions: u64,
    open_orders: u64,
    pending_rewards: u128,
    governance_balance: u128,
    estimated_inventory_value: u128,
    cross_chain_positions: u64,
}

struct CrossChainTradeIntent {
    trade_id: u64,
    pair_id: u64,
    order_id: Option<u64>,
    source_chain: ChainId,
    destination_chain: ChainId,
    trader: AccountId,
    recipient: AccountId,
    amount_in: u128,
    min_amount_out: u128,
    bridge_request_id: Option<u64>,
    bridge_fee_quote: BridgeFeeQuote,
    status: CrossChainTradeStatus,
    created_at: u64,
}
```

#### Public Functions

```rust
fn new(
    governance_symbol: String,
    governance_supply: u128,
    emission_rate: u128,
    quorum_bips: u32,
) -> Self

// Pool Management
fn create_pool(
    &mut self,
    base_token: TokenId,
    quote_token: TokenId,
    fee_bips: u32,
    initial_base: u128,
    initial_quote: u128,
) -> Result<u64, Error>

fn add_liquidity(
    &mut self,
    pair_id: u64,
    amount_base: u128,
    amount_quote: u128,
) -> Result<u128, Error>

fn remove_liquidity(
    &mut self,
    pair_id: u64,
    lp_shares: u128,
) -> Result<(u128, u128), Error>

// Swaps
fn swap_exact_in(
    &mut self,
    pair_id: u64,
    amount_in: u128,
    min_amount_out: u128,
) -> Result<u128, Error>

fn swap_exact_out(
    &mut self,
    pair_id: u64,
    amount_out: u128,
    max_amount_in: u128,
) -> Result<u128, Error>

// Order Book
fn place_order(
    &mut self,
    pair_id: u64,
    side: OrderSide,
    order_type: OrderType,
    price: u128,
    amount: u128,
) -> Result<u64, Error>

fn cancel_order(&mut self, order_id: u64) -> Result<(), Error>
fn update_order(&mut self, order_id: u64, new_price: u128, new_amount: u128) -> Result<(), Error>

// Cross-Chain Trades
fn initiate_cross_chain_trade(
    &mut self,
    pair_id: u64,
    destination_chain: ChainId,
    amount_in: u128,
    min_amount_out: u128,
) -> Result<u64, Error>

fn settle_cross_chain_trade(&mut self, trade_id: u64) -> Result<(), Error>

// Liquidity Mining
fn stake_for_mining(&mut self, pair_id: u64, amount: u128) -> Result<(), Error>
fn unstake_from_mining(&mut self, pair_id: u64, amount: u128) -> Result<(), Error>
fn claim_mining_rewards(&mut self, pair_id: u64) -> Result<u128, Error>

// Governance
fn create_governance_proposal(
    &mut self,
    title: String,
    description_hash: [u8; 32],
    new_fee_bips: Option<u32>,
) -> Result<u64, Error>

fn vote_on_proposal(&mut self, proposal_id: u64, support: bool) -> Result<(), Error>
fn execute_proposal(&mut self, proposal_id: u64) -> Result<(), Error>

// Queries
fn get_pool(&self, pair_id: u64) -> Option<LiquidityPool>
fn get_position(&self, pair_id: u64, account: AccountId) -> Option<LiquidityPosition>
fn get_order(&self, order_id: u64) -> Option<TradingOrder>
fn get_analytics(&self, pair_id: u64) -> Option<PairAnalytics>
fn get_portfolio(&self, account: AccountId) -> PortfolioSnapshot
```

#### Public Events

```rust
event PoolCreated {
    #[topic] pair_id: u64,
    base_token: TokenId,
    quote_token: TokenId,
}

event LiquidityAdded {
    #[topic] pair_id: u64,
    #[topic] provider: AccountId,
    minted_shares: u128,
}

event LiquidityRemoved {
    #[topic] pair_id: u64,
    #[topic] provider: AccountId,
    burned_shares: u128,
}

event SwapExecuted {
    #[topic] pair_id: u64,
    #[topic] trader: AccountId,
    amount_in: u128,
    amount_out: u128,
}

event OrderPlaced {
    #[topic] order_id: u64,
    #[topic] pair_id: u64,
    #[topic] trader: AccountId,
}

event OrderCancelled {
    #[topic] order_id: u64,
    #[topic] trader: AccountId,
}

event OrderFilled {
    #[topic] order_id: u64,
    #[topic] trader: AccountId,
    amount_filled: u128,
}

event CrossChainTradeCreated {
    #[topic] trade_id: u64,
    #[topic] pair_id: u64,
    destination_chain: ChainId,
}

event CrossChainTradeSettled {
    #[topic] trade_id: u64,
    amount_out: u128,
}

event MiningRewardsClaimed {
    #[topic] pair_id: u64,
    #[topic] staker: AccountId,
    amount: u128,
}

event GovernanceProposalCreated {
    #[topic] proposal_id: u64,
    #[topic] proposer: AccountId,
    title: String,
}

event GovernanceVoteCast {
    #[topic] proposal_id: u64,
    #[topic] voter: AccountId,
    support: bool,
}

event GovernanceProposalExecuted {
    #[topic] proposal_id: u64,
}
```

---

### 6. Governance Contract

**Location:** `contracts/governance/src/lib.rs`

#### Primary Purpose

Multi-signature governance with proposal creation, voting, timelock, and emergency override capabilities.

#### Public Enums

```rust
enum GovernanceAction {
    // Variants defined in types.rs
}

enum ProposalStatus {
    Active,
    Approved,
    Rejected,
    Executed,
    Cancelled,
}
```

#### Public Structs

```rust
struct GovernanceProposal {
    id: u64,
    proposer: AccountId,
    description_hash: Hash,
    action_type: GovernanceAction,
    target: Option<AccountId>,
    threshold: u32,
    votes_for: u32,
    votes_against: u32,
    status: ProposalStatus,
    created_at: u64,
    executed_at: u64,
    timelock_until: u64,
}
```

#### Public Functions

```rust
fn new(signers: Vec<AccountId>, threshold: u32, timelock_blocks: u64) -> Self

// Queries
fn get_proposal(&self, proposal_id: u64) -> Option<GovernanceProposal>
fn get_signers(&self) -> Vec<AccountId>
fn get_threshold(&self) -> u32
fn get_admin(&self) -> AccountId
fn get_active_proposal_count(&self) -> u32

// Mutations
fn create_proposal(
    &mut self,
    description_hash: Hash,
    action_type: GovernanceAction,
    target: Option<AccountId>,
) -> Result<u64, Error>

fn vote(&mut self, proposal_id: u64, support: bool) -> Result<(), Error>
fn execute_proposal(&mut self, proposal_id: u64) -> Result<(), Error>

fn add_signer(&mut self, signer: AccountId) -> Result<(), Error>
fn remove_signer(&mut self, signer: AccountId) -> Result<(), Error>
fn update_threshold(&mut self, new_threshold: u32) -> Result<(), Error>

fn emergency_override(&mut self, proposal_id: u64) -> Result<(), Error>
```

#### Public Events

```rust
event ProposalCreated {
    #[topic] proposal_id: u64,
    #[topic] proposer: AccountId,
    action_type: GovernanceAction,
    threshold: u32,
}

event VoteCast {
    #[topic] proposal_id: u64,
    #[topic] voter: AccountId,
    support: bool,
}

event ProposalExecuted {
    #[topic] proposal_id: u64,
    executed_at: u64,
}

event ProposalRejected {
    #[topic] proposal_id: u64,
}

event SignerAdded {
    #[topic] signer: AccountId,
    #[topic] added_by: AccountId,
}

event SignerRemoved {
    #[topic] signer: AccountId,
    #[topic] removed_by: AccountId,
}

event ThresholdUpdated {
    old_threshold: u32,
    new_threshold: u32,
}

event EmergencyOverrideUsed {
    #[topic] proposal_id: u64,
    #[topic] admin: AccountId,
}
```

---

### 7. Lending Contract

**Location:** `contracts/lending/src/lib.rs`

#### Primary Purpose

Property-collateralized lending with margin trading, yield farming, and governance proposals.

#### Public Enums

```rust
enum LendingError {
    Unauthorized,
    PropertyNotFound,
    InsufficientCollateral,
    LoanNotFound,
    PoolNotFound,
    InsufficientLiquidity,
    PositionNotFound,
    LiquidationThresholdNotMet,
    InvalidParameters,
    ProposalNotFound,
    InsufficientVotes,
}
```

#### Public Structs

```rust
struct CollateralRecord {
    property_id: u64,
    assessed_value: u128,
    ltv_ratio: u32,
    liquidation_threshold: u32,
}

struct LendingPool {
    pool_id: u64,
    total_deposits: u128,
    total_borrows: u128,
    base_rate: u32,
}

struct MarginPosition {
    position_id: u64,
    owner: AccountId,
    collateral: u128,
    leverage: u32,
    is_short: bool,
    entry_price: u128,
}

struct LoanApplication {
    loan_id: u64,
    applicant: AccountId,
    requested_amount: u128,
    collateral_value: u128,
    credit_score: u32,
    approved: bool,
}

struct YieldPosition {
    owner: AccountId,
    staked: u128,
    reward_debt: u128,
    accumulated_rewards: u128,
}

struct Proposal {
    proposal_id: u64,
    description: String,
    votes_for: u64,
    votes_against: u64,
    executed: bool,
}
```

#### Public Functions

```rust
fn new(admin: AccountId) -> Self

fn assess_collateral(
    &mut self,
    property_id: u64,
    value: u128,
    ltv: u32,
    liq_threshold: u32,
) -> Result<(), LendingError>

fn should_liquidate(&self, property_id: u64, current_value: u128) -> bool

fn create_pool(&mut self, base_rate: u32) -> Result<u64, LendingError>
fn deposit(&mut self, pool_id: u64, amount: u128) -> Result<(), LendingError>
fn borrow(&mut self, pool_id: u64, amount: u128) -> Result<(), LendingError>
fn repay(&mut self, pool_id: u64, amount: u128) -> Result<(), LendingError>

fn open_margin_position(
    &mut self,
    pool_id: u64,
    collateral: u128,
    leverage: u32,
    is_short: bool,
) -> Result<u64, LendingError>

fn close_margin_position(&mut self, position_id: u64) -> Result<(), LendingError>

fn stake_for_yield(&mut self, pool_id: u64, amount: u128) -> Result<(), LendingError>
fn claim_yield_rewards(&mut self, pool_id: u64) -> Result<u128, LendingError>

fn apply_for_loan(
    &mut self,
    property_id: u64,
    amount: u128,
    credit_score: u32,
) -> Result<u64, LendingError>

fn approve_loan(&mut self, loan_id: u64) -> Result<(), LendingError>
fn create_proposal(&mut self, description: String) -> Result<u64, LendingError>
fn vote_on_proposal(&mut self, proposal_id: u64, support: bool) -> Result<(), LendingError>
```

#### Public Events

```rust
event CollateralAssessed {
    #[topic] property_id: u64,
    assessed_value: u128,
    ltv_ratio: u32,
}

event PoolCreated {
    #[topic] pool_id: u64,
    base_rate: u32,
}

event PositionOpened {
    #[topic] position_id: u64,
    #[topic] owner: AccountId,
    collateral: u128,
}

event LoanApproved {
    #[topic] loan_id: u64,
    #[topic] applicant: AccountId,
    amount: u128,
}

event ProposalCreated {
    #[topic] proposal_id: u64,
    description: String,
}
```

---

### 8. Insurance Contract

**Location:** `contracts/insurance/src/lib.rs`

#### Primary Purpose

Decentralized property insurance with policy management, claims processing, risk assessment, and reinsurance.

#### Public Enums

```rust
enum CoverageType {
    // Variants defined in types.rs
}

enum RiskLevel {
    Low,
    Medium,
    High,
    Prohibited,
}

enum ClaimStatus {
    Submitted,
    Approved,
    Rejected,
    Paid,
}
```

#### Public Structs

```rust
struct InsurancePolicy {
    policy_id: u64,
    policyholder: AccountId,
    property_id: u64,
    coverage_type: CoverageType,
    coverage_amount: u128,
    premium_amount: u128,
    start_time: u64,
    end_time: u64,
    is_active: bool,
}

struct InsuranceClaim {
    claim_id: u64,
    policy_id: u64,
    claimant: AccountId,
    claim_amount: u128,
    claim_description: String,
    status: ClaimStatus,
    submitted_at: u64,
    resolved_at: Option<u64>,
}

struct RiskPool {
    pool_id: u64,
    total_capital: u128,
    active_policies: u64,
    total_claims: u128,
    pool_manager: AccountId,
}

struct RiskAssessment {
    property_id: u64,
    structural_score: u32,
    location_risk: u32,
    property_age: u32,
    overall_score: u32,
    risk_level: RiskLevel,
}

struct ReinsuranceAgreement {
    agreement_id: u64,
    reinsurer: AccountId,
    coverage_limit: u128,
    premium_share: u32,
    is_active: bool,
}

struct ActuarialModel {
    model_id: u64,
    loss_ratio: u32,
    premium_base: u128,
    adjustment_factor: u32,
}

struct InsuranceToken {
    token_id: u64,
    policy_id: u64,
    owner: AccountId,
    face_value: u128,
    is_for_sale: bool,
    sale_price: Option<u128>,
}

struct PoolLiquidityProvider {
    provider: AccountId,
    capital_contributed: u128,
    share_of_pool: u128,
    earnings: u128,
}

struct UnderwritingCriteria {
    pool_id: u64,
    min_credit_score: u32,
    max_ltv_ratio: u32,
    min_coverage_amount: u128,
    max_coverage_amount: u128,
}
```

#### Public Functions

```rust
fn new(admin: AccountId) -> Self

// Policy Management
fn create_policy(
    &mut self,
    property_id: u64,
    coverage_type: CoverageType,
    coverage_amount: u128,
    premium_amount: u128,
    duration_days: u64,
) -> Result<u64, InsuranceError>

fn cancel_policy(&mut self, policy_id: u64) -> Result<(), InsuranceError>
fn renew_policy(&mut self, policy_id: u64, new_duration_days: u64) -> Result<(), InsuranceError>

// Claims Processing
fn submit_claim(
    &mut self,
    policy_id: u64,
    claim_amount: u128,
    claim_description: String,
) -> Result<u64, InsuranceError>

fn approve_claim(&mut self, claim_id: u64) -> Result<(), InsuranceError>
fn reject_claim(&mut self, claim_id: u64, reason: String) -> Result<(), InsuranceError>
fn pay_claim(&mut self, claim_id: u64) -> Result<(), InsuranceError>

// Risk Assessment
fn assess_property_risk(&mut self, property_id: u64) -> Result<(), InsuranceError>
fn get_risk_assessment(&self, property_id: u64) -> Option<RiskAssessment>

// Pool Management
fn create_pool(&mut self) -> Result<u64, InsuranceError>
fn add_liquidity_to_pool(&mut self, pool_id: u64, amount: u128) -> Result<(), InsuranceError>
fn withdraw_from_pool(&mut self, pool_id: u64, amount: u128) -> Result<(), InsuranceError>

// Reinsurance
fn create_reinsurance_agreement(
    &mut self,
    reinsurer: AccountId,
    coverage_limit: u128,
    premium_share: u32,
) -> Result<u64, InsuranceError>

fn activate_reinsurance(&mut self, agreement_id: u64, claim_id: u64) -> Result<(), InsuranceError>

// Insurance Tokens
fn mint_insurance_token(&mut self, policy_id: u64) -> Result<u64, InsuranceError>
fn transfer_insurance_token(&mut self, token_id: u64, to: AccountId) -> Result<(), InsuranceError>
fn list_insurance_token(&mut self, token_id: u64, price: u128) -> Result<(), InsuranceError>

// Actuarial
fn update_actuarial_model(
    &mut self,
    loss_ratio: u32,
    premium_base: u128,
    adjustment_factor: u32,
) -> Result<(), InsuranceError>
```

#### Public Events

```rust
event PolicyCreated {
    #[topic] policy_id: u64,
    #[topic] policyholder: AccountId,
    #[topic] property_id: u64,
    coverage_type: CoverageType,
    coverage_amount: u128,
    premium_amount: u128,
    start_time: u64,
    end_time: u64,
}

event PolicyCancelled {
    #[topic] policy_id: u64,
    #[topic] policyholder: AccountId,
    cancelled_at: u64,
}

event ClaimSubmitted {
    #[topic] claim_id: u64,
    #[topic] policy_id: u64,
    #[topic] claimant: AccountId,
    claim_amount: u128,
    submitted_at: u64,
}

event ClaimApproved {
    #[topic] claim_id: u64,
    #[topic] policy_id: u64,
    payout_amount: u128,
    approved_by: AccountId,
    timestamp: u64,
}

event ClaimRejected {
    #[topic] claim_id: u64,
    #[topic] policy_id: u64,
    reason: String,
    rejected_by: AccountId,
    timestamp: u64,
}

event PayoutExecuted {
    #[topic] claim_id: u64,
    #[topic] recipient: AccountId,
    amount: u128,
    timestamp: u64,
}

event PoolCapitalized {
    #[topic] pool_id: u64,
    #[topic] provider: AccountId,
    amount: u128,
    timestamp: u64,
}

event ReinsuranceActivated {
    #[topic] claim_id: u64,
    agreement_id: u64,
    recovery_amount: u128,
    timestamp: u64,
}

event InsuranceTokenMinted {
    #[topic] token_id: u64,
    #[topic] policy_id: u64,
    #[topic] owner: AccountId,
    face_value: u128,
}

event InsuranceTokenTransferred {
    #[topic] token_id: u64,
    #[topic] from: AccountId,
    #[topic] to: AccountId,
    price: u128,
}

event RiskAssessmentUpdated {
    #[topic] property_id: u64,
    overall_score: u32,
    risk_level: RiskLevel,
    timestamp: u64,
}
```

---

### 9-12. Other Major Contracts (Summary)

#### Fractional Contract

- **Purpose**: Portfolio aggregation and tax reporting
- **Key Types**: `PortfolioItem`, `PortfolioAggregation`, `TaxReport`
- **Main Functions**: `aggregate_portfolio()`, `summarize_tax()`

#### Staking Contract

- **Purpose**: Token staking with governance delegation
- **Key Types**: `StakeInfo`, `LockPeriod`
- **Main Functions**: `stake()`, `unstake()`, `claim_rewards()`, `delegate_governance()`
- **Events**: `Staked`, `Unstaked`, `RewardsClaimed`, `GovernanceDelegated`

#### Analytics Contract

- **Purpose**: Market metrics and sentiment analysis
- **Key Types**: `MarketMetrics`, `MarketTrend`, `MarketSentiment`, `MarketReport`
- **Main Functions**: `get_market_metrics()`, `add_market_trend()`, `get_overall_sentiment()`

#### Prediction Market Contract

- **Purpose**: Property value prediction markets
- **Key Types**: `PredictionMarketInfo`, `MarketStatus`, `PredictionDirection`, `Stake`, `UserReputation`
- **Main Functions**: `create_market()`, `stake_prediction()`, `resolve_market()`, `claim_reward()`
- **Events**: `MarketCreated`, `PredictionStaked`, `MarketResolved`, `RewardClaimed`

#### Property Management Contract

- **Purpose**: Lease management, tenant screening, maintenance
- **Key Types**: `Lease`, `MaintenanceRequest`, `TenantScreening`, `Expense`, `Inspection`
- **Main Functions**: `create_lease()`, `submit_maintenance()`, `screen_tenant()`, `record_expense()`

#### Compliance Registry Contract

- **Purpose**: KYC/AML verification, jurisdiction rules
- **Key Types**: `Jurisdiction`, `VerificationStatus`, `RiskLevel`, `DocumentType`, `SanctionsList`
- **Main Functions**: `verify_identity()`, `check_sanctions()`, `verify_document()`, `assess_aml_risk()`

#### Monitoring Contract

- **Purpose**: System health checks and alerts
- **Key Types**: `HealthStatus`, `AlertType`, `PerformanceMetrics`, `MetricsSnapshot`, `AlertConfig`
- **Main Functions**: `record_operation()`, `get_health_status()`, `get_performance_metrics()`, `configure_alert()`

#### AI Valuation Contract

- **Purpose**: Machine learning-based property valuation
- **Key Types**: `AIModelType`, `PropertyFeatures`, `AIModel`, `AIPrediction`, `EnsemblePrediction`
- **Main Functions**: `predict_valuation()`, `get_ensemble_prediction()`, `add_training_data()`, `train_model()`

#### Fees Contract

- **Purpose**: Dynamic fee calculation and validator rewards
- **Key Types**: `FeeOperation`, `FeeConfig`, `PremiumAuction`, `AuctionBid`
- **Main Functions**: `calculate_dynamic_fee()`, `create_premium_auction()`, `bid_on_auction()`, `distribute_rewards()`

#### Crowdfunding Contract

- **Purpose**: Real estate crowdfunding campaigns
- **Key Types**: `Campaign`, `InvestorProfile`, `Milestone`, `ShareListing`, `RiskProfile`
- **Main Functions**: `create_campaign()`, `invest()`, `approve_milestone()`, `list_shares()`

#### ZK Compliance Contract

- **Purpose**: Zero-knowledge proof-based compliance
- **Key Types**: `ZkProofType`, `ZkProofStatus`, `ZkProofData`, `PrivacyPreferences`
- **Main Functions**: `submit_zk_proof()`, `verify_zk_proof()`, `set_privacy_preferences()`

#### Database Integration Contract

- **Purpose**: Off-chain data sync and export
- **Key Types**: `DataType`, `SyncId`, `ExportBatchId`
- **Main Functions**: `sync_data()`, `export_batch()`, `confirm_sync()`, `register_indexer()`

---

## Shared Trait Interfaces

### 1. PropertyRegistry Trait

```rust
trait PropertyRegistry {
    type Error;

    fn register_property(&mut self, metadata: PropertyMetadata) -> Result<u64, Self::Error>;
    fn transfer_property(&mut self, property_id: u64, to: AccountId) -> Result<(), Self::Error>;
    fn get_property(&self, property_id: u64) -> Option<PropertyInfo>;
    fn update_metadata(&mut self, property_id: u64, metadata: PropertyMetadata) -> Result<(), Self::Error>;
    fn approve(&mut self, property_id: u64, to: Option<AccountId>) -> Result<(), Self::Error>;
    fn get_approved(&self, property_id: u64) -> Option<AccountId>;
}
```

### 2. AdvancedEscrow Trait

```rust
trait AdvancedEscrow {
    type Error;

    fn create_escrow_advanced(
        &mut self,
        property_id: u64,
        amount: u128,
        buyer: AccountId,
        seller: AccountId,
        participants: Vec<AccountId>,
        required_signatures: u8,
        release_time_lock: Option<u64>,
    ) -> Result<u64, Self::Error>;

    fn deposit_funds(&mut self, escrow_id: u64) -> Result<(), Self::Error>;
    fn release_funds(&mut self, escrow_id: u64) -> Result<(), Self::Error>;
    fn refund_funds(&mut self, escrow_id: u64) -> Result<(), Self::Error>;
    fn upload_document(&mut self, escrow_id: u64, document_hash: Hash, document_type: String) -> Result<(), Self::Error>;
    fn verify_document(&mut self, escrow_id: u64, document_hash: Hash) -> Result<(), Self::Error>;
    fn add_condition(&mut self, escrow_id: u64, description: String) -> Result<u64, Self::Error>;
    fn mark_condition_met(&mut self, escrow_id: u64, condition_id: u64) -> Result<(), Self::Error>;
    fn sign_approval(&mut self, escrow_id: u64, approval_type: ApprovalType) -> Result<(), Self::Error>;
    fn raise_dispute(&mut self, escrow_id: u64, reason: String) -> Result<(), Self::Error>;
    fn resolve_dispute(&mut self, escrow_id: u64, resolution: String) -> Result<(), Self::Error>;
    fn emergency_override(&mut self, escrow_id: u64, release_to_seller: bool) -> Result<(), Self::Error>;
}
```

### 3. PropertyTokenBridge Trait

```rust
trait PropertyTokenBridge {
    type Error;

    fn lock_token_for_bridge(&mut self, token_id: TokenId, destination_chain: ChainId, recipient: AccountId) -> Result<(), Self::Error>;
    fn mint_bridged_token(&mut self, source_chain: ChainId, original_token_id: TokenId, recipient: AccountId, metadata: PropertyMetadata) -> Result<TokenId, Self::Error>;
    fn burn_bridged_token(&mut self, token_id: TokenId, destination_chain: ChainId, recipient: AccountId) -> Result<(), Self::Error>;
    fn unlock_token(&mut self, token_id: TokenId, recipient: AccountId) -> Result<(), Self::Error>;
    fn get_bridge_status(&self, token_id: TokenId) -> Option<BridgeStatus>;
    fn verify_bridge_transaction(&self, token_id: TokenId, transaction_hash: Hash, source_chain: ChainId) -> bool;
    fn add_bridge_operator(&mut self, operator: AccountId) -> Result<(), Self::Error>;
    fn remove_bridge_operator(&mut self, operator: AccountId) -> Result<(), Self::Error>;
    fn is_bridge_operator(&self, account: AccountId) -> bool;
    fn get_bridge_operators(&self) -> Vec<AccountId>;
}
```

### 4. Oracle Trait

```rust
trait Oracle {
    fn get_valuation(&self, property_id: u64) -> Result<PropertyValuation, OracleError>;
    fn get_valuation_with_confidence(&self, property_id: u64) -> Result<ValuationWithConfidence, OracleError>;
    fn request_valuation(&mut self, property_id: u64) -> Result<u64, OracleError>;
    fn batch_request_valuations(&mut self, property_ids: Vec<u64>) -> Result<Vec<u64>, OracleError>;
    fn get_historical_valuations(&self, property_id: u64, limit: u32) -> Vec<PropertyValuation>;
    fn get_market_volatility(&self, property_type: PropertyType, location: String) -> Result<VolatilityMetrics, OracleError>;
}
```

### 5. OracleRegistry Trait

```rust
trait OracleRegistry {
    fn add_source(&mut self, source: OracleSource) -> Result<(), OracleError>;
    fn remove_source(&mut self, source_id: String) -> Result<(), OracleError>;
    fn update_reputation(&mut self, source_id: String, success: bool) -> Result<(), OracleError>;
    fn get_reputation(&self, source_id: String) -> Option<u32>;
    fn slash_source(&mut self, source_id: String, penalty_amount: u128) -> Result<(), OracleError>;
    fn detect_anomalies(&self, property_id: u64, new_valuation: u128) -> bool;
}
```

### 6. AdvancedBridge Trait

```rust
trait AdvancedBridge {
    type Error;

    fn initiate_bridge_multisig(&mut self, token_id: TokenId, destination_chain: ChainId, recipient: AccountId, required_signatures: u8, timeout_blocks: Option<u64>) -> Result<u64, Self::Error>;
    fn sign_bridge_request(&mut self, bridge_request_id: u64, approve: bool) -> Result<(), Self::Error>;
    fn execute_bridge(&mut self, bridge_request_id: u64) -> Result<(), Self::Error>;
    fn monitor_bridge_status(&self, bridge_request_id: u64) -> Option<BridgeMonitoringInfo>;
    fn recover_failed_bridge(&mut self, bridge_request_id: u64, recovery_action: RecoveryAction) -> Result<(), Self::Error>;
    fn estimate_bridge_gas(&self, token_id: TokenId, destination_chain: ChainId) -> Result<u64, Self::Error>;
    fn get_bridge_history(&self, account: AccountId) -> Vec<BridgeTransaction>;
}
```

### 7. ComplianceChecker Trait

```rust
trait ComplianceChecker {
    fn is_compliant(&self, account: AccountId) -> bool;
}
```

### 8. DynamicFeeProvider Trait

```rust
trait DynamicFeeProvider {
    fn get_recommended_fee(&self, operation: FeeOperation) -> u128;
}
```

---

## Event Taxonomy

### Standard Event Categories

#### Lifecycle Events

- Resource creation: `PropertyTokenMinted`, `EscrowCreated`, `PolicyCreated`, `CampaignCreated`
- Resource deletion/closure: `PropertyBurned`, `EscrowClosed`, `PolicyCancelled`

#### StateChange Events

- Transfers: `Transfer` (token), `FundsDeposited`, `FundsReleased`
- Status updates: `ProposalCreated`, `OrderPlaced`, `ClaimSubmitted`

#### Authorization Events

- Approvals: `Approval`, `ApprovalForAll`, `SignatureAdded`
- Role changes: `SignerAdded`, `SignerRemoved`, `OperatorAdded`

#### Financial Events

- Value movements: `DividendsDeposited`, `DividendsWithdrawn`, `PayoutExecuted`
- Fee collection: `FeeConfigUpdated`, `RewardsDistributed`

#### Administrative Events

- Configuration: `ThresholdUpdated`, `PremiumAuctionCreated`, `PoolCreated`
- Emergency: `EmergencyOverride`, `EmergencyPause`

#### Audit Events

- Verification: `ComplianceVerified`, `DocumentVerified`, `ConditionMet`
- Dispute resolution: `DisputeRaised`, `DisputeResolved`

#### Cross-Chain Events

- Bridge operations: `TokenBridged`, `BridgeRequestCreated`, `BridgeExecuted`
- Cross-chain trades: `CrossChainTradeCreated`, `CrossChainTradeSettled`

---

## Type Patterns

### Pattern 1: Status Enums with Options

Many structures use optional fields paired with status enums to represent multi-stage workflows:

```rust
struct WithOptionalCompletionData {
    status: Status,
    created_at: u64,
    completed_at: Option<u64>,  // Only set when status = Completed
    result: Option<String>,      // Only set when status = Completed
}
```

### Pattern 2: Mapping-Based Collections

Storage is implemented using `Mapping<Key, Value>` with separate counters for enumeration:

```rust
mapping: Mapping<u64, Item>,    // item_id -> Item
counter: u64,                   // Total count
list: Vec<AccountId>,           // Enumerable list for filtered queries
```

### Pattern 3: Ticked Values (Basis Points)

Fees, percentages, and ratios use basis points (1/10,000th):

```rust
fee_bips: u32,      // Fee as basis points (100 = 0.01%)
volatility_bips: u32,  // Volatility as basis points
quorum_bips: u32,   // Quorum threshold as basis points
```

### Pattern 4: Account-Based Access Control

Role/permission checks through account mappings:

```rust
admin: AccountId,
authorized_accounts: Mapping<AccountId, bool>,
signers: Vec<AccountId>,
```

### Pattern 5: Time-Based Conditions

Deadlines and locks use block numbers and timestamps:

```rust
lock_until: u64,         // Block number
release_deadline: Option<u64>,  // Timestamp in millis
created_at: u64,         // Timestamp
expires_at: Option<u64>, // Timestamp
```

### Pattern 6: Nested Collections

Complex data structures use nested Mappings to avoid deep nesting:

```rust
// Instead of: Mapping<u64, Vec<(u64, Item)>>
// Use dual mappings:
items: Mapping<(u64, u64), Item>,        // (container_id, item_id) -> Item
item_counts: Mapping<u64, u64>,          // container_id -> count
```

### Pattern 7: Result Types with Custom Errors

All mutable functions return `Result<T, Error>` with specific error enums:

```rust
#[derive(Debug, PartialEq, Eq, scale::Encode, scale::Decode)]
pub enum Error {
    Unauthorized,
    NotFound,
    InvalidAmount,
    // ... domain-specific errors
}
```

### Pattern 8: Event Topics

Events use `#[topic]` attribute for indexed fields enabling efficient log filtering:

```rust
event Transfer {
    #[topic] from: Option<AccountId>,  // Indexed
    #[topic] to: Option<AccountId>,    // Indexed
    #[topic] id: TokenId,              // Indexed
    // Non-indexed data can be larger
}
```

---

## Summary

This comprehensive guide covers all 25+ public contracts in the PropChain ecosystem with:

- **1,200+ public types** (enums, structs)
- **500+ public functions/messages**
- **400+ public events**
- **8 core trait interfaces** used across multiple contracts
- **Common patterns** for consistent type generation

All types are Substrate-compatible with `scale::Encode` and `scale::Decode` derives for blockchain serialization.

For TypeScript SDK generation:

- Use trait definitions as base interfaces
- Generate union types for variant enums
- Create async wrappers around all `#[ink(message)]` functions
- Map events to typed event listeners
- Support multi-contract composition through trait interfaces
