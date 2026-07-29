# PropChain Smart Contract Architecture

> One-stop overview of the contract system.  
> For deep-dives see the per-contract READMEs under `contracts/` and the
> supplementary docs under `docs/`.

---

## Table of Contents

1. [System Overview](#1-system-overview)
2. [Contract Map](#2-contract-map)
3. [Component Interaction Diagrams](#3-component-interaction-diagrams)
   - [Property Purchase Flow](#31-property-purchase-flow)
   - [Lending & Liquidation Flow](#32-lending--liquidation-flow)
   - [Insurance Premium & Claims Flow](#33-insurance-premium--claims-flow)
   - [Cross-Contract Data Flow](#34-cross-contract-data-flow)
4. [Layer Breakdown](#4-layer-breakdown)
5. [Key Design Principles](#5-key-design-principles)
6. [Security Model](#6-security-model)
7. [Further Reading](#7-further-reading)

---

## 1. System Overview

PropChain is a modular set of ink! (Substrate/Polkadot) smart contracts that
together implement decentralised real-estate infrastructure: tokenisation,
lending, insurance, governance, compliance, and cross-chain bridging.

```mermaid
graph TD
    subgraph Clients
        dApp["dApp / SDK"]
        Admin["Admin EOA"]
        Oracle["Price Oracle"]
    end

    subgraph Core["Core Layer"]
        Registry["Property Registry\n(identity)"]
        Token["Property Token\n(metadata / fractional)"]
        Escrow["Factory / Escrow"]
    end

    subgraph Finance["Finance Layer"]
        Lending["Lending\n(PropertyLending)"]
        Insurance["Insurance\n(PropchainInsurance)"]
        Staking["Staking"]
        DEX["DEX"]
    end

    subgraph Infra["Infrastructure Layer"]
        Traits["Shared Traits\n(propchain-traits)"]
        Oracle2["Oracle / Mock-Oracle"]
        Bridge["Cross-Chain Bridge"]
        Governance["Governance"]
        Compliance["Compliance Registry"]
        GDPR["GDPR Module"]
    end

    dApp -->|ink! messages| Core
    dApp -->|ink! messages| Finance
    Admin -->|admin messages| Core
    Admin -->|admin messages| Finance
    Oracle -->|price feed| Oracle2

    Core --> Traits
    Finance --> Traits
    Finance --> Oracle2
    Lending -->|reads collateral| Registry
    Insurance -->|reads property| Registry
    Bridge -->|lock/unlock| Token
    Governance -->|proposals| Finance
    Compliance --> Registry
    GDPR --> Registry
```

---

## 2. Contract Map

| Contract | Crate | Primary Responsibility |
|----------|-------|----------------------|
| `identity` | `propchain-identity` | On-chain KYC / DID registry |
| `metadata` | `propchain-metadata` | Property metadata & IPFS pinning |
| `fractional` | `propchain-fractional` | Fractional ownership tokens |
| `ipfs-metadata` | `propchain-ipfs-metadata` | IPFS CID storage helpers |
| `lending` | `propchain-lending` | Collateral, pools, loans, liquidations, yield |
| `insurance` | `propchain-insurance` | Dynamic premiums, risk pools, claims |
| `staking` | `propchain-staking` | Token staking & rewards |
| `dex` | `propchain-dex` | On-chain swap / AMM |
| `oracle` | `propchain-oracle` | Price feed aggregation |
| `mock-oracle` | `propchain-mock-oracle` | Deterministic test oracle |
| `bridge` | `propchain-bridge` | Cross-chain asset bridge |
| `governance` | `propchain-governance` | Proposals & voting |
| `compliance_registry` | `propchain-compliance-registry` | AML/KYC compliance records |
| `gdpr` | `propchain-gdpr` | GDPR data erasure requests |
| `sanctions` | `propchain-sanctions` | OFAC/sanctions screening |
| `factory` | `propchain-factory` | Contract deployment factory |
| `proxy` | `propchain-proxy` | Upgradeable proxy pattern |
| `analytics` | `propchain-analytics` | On-chain metrics accumulation |
| `monitoring` | `propchain-monitoring` | Health-check aggregation |
| `version-registry` | `propchain-version-registry` | Contract version tracking |
| `database` | `propchain-database` | Generic key-value store |
| `multicall` | `propchain-multicall` | Batch message dispatcher |
| `prediction-market` | `propchain-prediction-market` | Property price prediction markets |
| `crowdfunding` | `propchain-crowdfunding` | Property crowdfunding campaigns |
| `fees` | `propchain-fees` | Platform fee configuration |
| `property-management` | `propchain-property-management` | Ongoing property management |
| `traits` | `propchain-traits` | Shared types, errors, macros |
| `lib` | `propchain-lib` | Shared utilities & Kani proofs |

---

## 3. Component Interaction Diagrams

### 3.1 Property Purchase Flow

```mermaid
sequenceDiagram
    participant Buyer
    participant Factory
    participant Identity
    participant Token
    participant Escrow

    Buyer->>Identity: verify_identity()
    Identity-->>Buyer: ✓ KYC approved

    Buyer->>Factory: deploy_property_contract(params)
    Factory->>Token: mint(property_id, buyer)
    Token-->>Factory: token_id

    Buyer->>Escrow: create_escrow(token_id, price)
    Escrow-->>Buyer: escrow_id

    note over Escrow: Funds locked on-chain

    Buyer->>Escrow: release_escrow(escrow_id)
    Escrow->>Token: transfer(buyer → seller)
    Escrow-->>Buyer: ✓ Transfer complete
```

### 3.2 Lending & Liquidation Flow

```mermaid
sequenceDiagram
    participant Borrower
    participant Lending
    participant Oracle
    participant Admin

    Borrower->>Lending: apply_for_property_backed_loan(property_id, amount)
    Lending->>Oracle: get_price(property_id)
    Oracle-->>Lending: current_value

    Lending-->>Borrower: loan_id (pending)

    Admin->>Lending: underwrite_loan(loan_id)
    note over Lending: Checks credit score ≥ 600\nand LTV ≤ 75%
    Lending-->>Admin: approved = true

    note over Lending: Time passes, value drops

    Admin->>Lending: liquidate_loan(loan_id, current_values)
    note over Lending: Checks LTV > liquidation_threshold\nor fixed-rate term expired
    Lending-->>Admin: ✓ LoanStatus::Liquidated
```

### 3.3 Insurance Premium & Claims Flow

```mermaid
sequenceDiagram
    participant Policyholder
    participant Insurance
    participant RiskPool
    participant Admin

    Admin->>Insurance: assess_risk(property_id, scores)
    Insurance-->>Admin: risk_assessment_id

    Policyholder->>Insurance: calculate_premium_with_modifiers(property_id, coverage, modifiers)
    note over Insurance: Dynamic pricing:\nbase_rate × risk × pool_utilisation\n× time × discounts
    Insurance-->>Policyholder: PremiumCalculation

    Policyholder->>Insurance: create_policy(property_id, coverage_type, pool_id)
    Insurance->>RiskPool: deduct premium from available capital
    Insurance-->>Policyholder: policy_id

    Policyholder->>Insurance: submit_claim(policy_id, amount, evidence_url)
    Admin->>Insurance: approve_claim(claim_id)
    Insurance->>RiskPool: pay_out(claim_amount - deductible)
    Insurance-->>Policyholder: ✓ Claim paid
```

### 3.4 Cross-Contract Data Flow

```mermaid
graph LR
    subgraph "propchain-traits"
        T1["ReentrancyGuard"]
        T2["non_reentrant! macro"]
        T3["KeyRotationRequest"]
        T4["HealthReport"]
        T5["Shared Error Types"]
    end

    Lending["Lending"] -->|uses| T1
    Lending -->|uses| T2
    Lending -->|uses| T3
    Insurance["Insurance"] -->|uses| T1
    Insurance -->|uses| T2
    Insurance -->|uses| T3

    subgraph "propchain-lib"
        L1["balance_proofs (Kani)"]
        L2["access_control_proofs (Kani)"]
        L3["oracle_proofs (Kani)"]
    end

    CI["formal-verification.yml"] -->|kani harnesses| L1
    CI -->|kani harnesses| L2
    CI -->|kani harnesses| L3

    Governance["Governance"] -->|cross-call| Lending
    Governance -->|cross-call| Insurance
    Oracle["Oracle"] -->|price feed| Lending
    Oracle -->|price feed| Insurance
    Bridge["Bridge"] -->|lock/unlock| Token["Property Token"]
```

---

## 4. Layer Breakdown

```mermaid
graph TB
    subgraph "User-Facing Layer"
        UI["dApp / SDK\n(TypeScript)"]
    end

    subgraph "Application Layer"
        A1["Factory — deployment orchestration"]
        A2["Multicall — batch operations"]
        A3["Proxy — upgrade management"]
    end

    subgraph "Business Logic Layer"
        B1["Lending — loans, collateral, yield"]
        B2["Insurance — premiums, pools, claims"]
        B3["Governance — proposals, voting"]
        B4["DEX — swaps, AMM"]
        B5["Crowdfunding — campaigns"]
        B6["Staking — rewards"]
    end

    subgraph "Registry / Identity Layer"
        R1["Identity — KYC / DID"]
        R2["Compliance Registry — AML"]
        R3["Sanctions — screening"]
        R4["GDPR — erasure requests"]
        R5["Version Registry — upgrades"]
    end

    subgraph "Data / Infra Layer"
        D1["Oracle — price feeds"]
        D2["Bridge — cross-chain"]
        D3["Database — generic KV"]
        D4["Analytics — metrics"]
        D5["Monitoring — health"]
    end

    subgraph "Shared Foundation"
        F1["propchain-traits\n(types, macros, errors)"]
        F2["propchain-lib\n(utils, Kani proofs)"]
    end

    UI --> A1
    UI --> B1
    UI --> B2
    A1 --> B1
    A1 --> B2
    B1 --> R1
    B2 --> R1
    B1 --> D1
    B2 --> D1
    B1 --> F1
    B2 --> F1
    B3 --> B1
    B3 --> B2
    D2 --> B1
    R2 --> R1
    R3 --> R1
    D4 --> B1
    D5 --> B1
    F1 --> F2
```

---

## 5. Key Design Principles

**Reentrancy protection** — every state-mutating message that calls back
into unknown code is wrapped with the `propchain_traits::non_reentrant!`
macro, which checks and sets a `ReentrancyGuard` flag stored in contract
storage.

**Two-step admin rotation** — admin transfers use a time-locked two-step
flow (`request_admin_rotation` / `confirm_admin_rotation`) with a
configurable cooldown and expiry window, preventing key-compromise attacks.

**Saturating arithmetic** — all numeric operations use Rust's
`saturating_add` / `saturating_sub` / `saturating_mul` to prevent overflow
panics, which would abort the entire extrinsic on-chain.

**Basis-point precision** — rates, multipliers, and fees are stored and
computed in basis points (1 bp = 0.01 %) using integer arithmetic.
This avoids floating-point non-determinism while providing 0.01 % precision.

**Formal verification** — balance conservation, access control, and oracle
freshness are proved by Kani model-checker harnesses in `contracts/lib`
and run on every PR via `.github/workflows/formal-verification.yml`.

**Modular crates** — each contract is its own Cargo workspace member.
Shared types and macros live in `contracts/traits` and `contracts/lib`,
which are pure-Rust crates (no `#[ink::contract]`) for easy unit-testing
and inclusion in Kani proofs.

---

## 6. Security Model

| Threat | Mitigation |
|--------|-----------|
| Reentrancy | `non_reentrant!` macro in lending, insurance, bridge |
| Admin key compromise | Two-step time-locked rotation with expiry |
| Integer overflow | Saturating arithmetic throughout |
| Oracle manipulation | Staleness bound proved by Kani; mock oracle in tests |
| Unauthorised calls | `caller != self.admin` guards on all privileged messages |
| Dependency vulnerabilities | `cargo-deny` + `cargo-audit` in nightly CI |
| Logic mutations | `cargo-mutants` gate in nightly CI for lending, bridge, oracle |
| Formal invariants | Kani balance/access/oracle proofs on every PR |

See [`SECURITY.md`](./SECURITY.md) and [`security-audit/`](./security-audit/)
for the full threat model and third-party audit reports.

---

## 7. Further Reading

| Resource | Path |
|----------|------|
| Lending contract | [`contracts/lending/README.md`](./contracts/lending/README.md) |
| Insurance premium calculation | [`contracts/insurance/DYNAMIC_PREMIUM_CALCULATION.md`](./contracts/insurance/DYNAMIC_PREMIUM_CALCULATION.md) |
| Insurance implementation summary | [`contracts/insurance/IMPLEMENTATION_SUMMARY.md`](./contracts/insurance/IMPLEMENTATION_SUMMARY.md) |
| Insurance penalty drift resolution | [`docs/penalty_drift.md`](./docs/penalty_drift.md) |
| Bridge liquidity pools | [`contracts/bridge/LIQUIDITY_POOLS.md`](./contracts/bridge/LIQUIDITY_POOLS.md) |
| Factory deployment guide | [`contracts/factory/DEPLOYMENT_GUIDE.md`](./contracts/factory/DEPLOYMENT_GUIDE.md) |
| Oracle encryption | [`contracts/oracle/ENCRYPTION.md`](./contracts/oracle/ENCRYPTION.md) |
| Proxy upgrade governance | [`docs/proxy_upgrade_governance.md`](./docs/proxy_upgrade_governance.md) |
| Dependency unification | [`docs/dependency_and_prelude_unification.md`](./docs/dependency_and_prelude_unification.md) |
| Clippy triage guide | [`docs/clippy_triage_guide.md`](./docs/clippy_triage_guide.md) |
| Development setup | [`DEVELOPMENT.md`](./DEVELOPMENT.md) |
| Contributing guide | [`CONTRIBUTING.md`](./CONTRIBUTING.md) |
| Security policy | [`SECURITY.md`](./SECURITY.md) |
| Audit log | [`AUDIT_LOG.md`](./AUDIT_LOG.md) |
