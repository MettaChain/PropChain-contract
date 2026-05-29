# Architecture Documentation Quick Reference

## 🎯 Find What You Need Fast

### I want to...

#### Understand the System
- **New to PropChain?** → Start with [Architecture Index](./ARCHITECTURE_INDEX.md) → [System Overview](./SYSTEM_ARCHITECTURE_OVERVIEW.md)
- **Need high-level view?** → [System Overview Section 1-3](./SYSTEM_ARCHITECTURE_OVERVIEW.md#high-level-architecture)
- **Want component details?** → [System Overview Section 4](./SYSTEM_ARCHITECTURE_OVERVIEW.md#core-component-architecture)
- **Curious about design choices?** → [Architectural Principles](./ARCHITECTURAL_PRINCIPLES.md)

#### Build Something
- **Integrating with PropChain?** → [Integration Guide](./integration.md) ← linked from Index
- **Need API details?** → [Contract API](./contracts.md)
- **Want to see interaction flows?** → [Component Diagrams](./COMPONENT_INTERACTION_DIAGRAMS.md)
- **Looking for examples?** → [Tutorials](./tutorials/)

#### Solve a Problem
- **Debugging an issue?** → [Error Handling Scenarios](./COMPONENT_INTERACTION_DIAGRAMS.md#error-handling--edge-cases)
- **Understanding a failure?** → [Failed Transaction Flow](./COMPONENT_INTERACTION_DIAGRAMS.md#16-failed-transaction-rollback)
- **Gas optimization needed?** → [Performance Section](./SYSTEM_ARCHITECTURE_OVERVIEW.md#performance-architecture)
- **Security concern?** → [Security Architecture](./SYSTEM_ARCHITECTURE_OVERVIEW.md#security-architecture)

#### Make Decisions
- **Evaluating trade-offs?** → [Trade-off Analysis](./ARCHITECTURAL_PRINCIPLES.md#tradeoff-analysis)
- **Making design choices?** → [Decision Framework](./ARCHITECTURAL_PRINCIPLES.md#technical-decision-framework)
- **Need precedent?** → [Architecture Decision Records](./adr/)
- **Questioning principles?** → [Core Principles](./ARCHITECTURAL_PRINCIPLES.md#core-architectural-principles)

#### Contribute
- **Want to contribute?** → [Contributing Guide](../CONTRIBUTING.md)
- **Updating documentation?** → [Maintenance Guide](./ARCHITECTURE_DOCUMENTATION_MAINTENANCE.md)
- **Reporting issues?** → [Issue Template](./ARCHITECTURE_INDEX.md#reporting-issues)
- **Suggesting improvements?** → [Discussion Guidelines](./ARCHITECTURE_INDEX.md#getting-help)

---

## 📊 Document Selector

```
What's your role?
│
├─ New Team Member ────────────────┐
│  1. Architecture Index           │
│  2. System Overview              │ FASTEST PATH
│  3. Basic Tutorial               │ TO PRODUCTIVITY
│                                  │
├─ Developer ──────────────────────┤
│  1. Contract API                 │ DAILY REFERENCE
│  2. Component Diagrams           │ FOR BUILDING
│  3. Error Handling Guide         │
│                                  │
├─ Architect ──────────────────────┤
│  1. Architectural Principles     │ STRATEGIC
│  2. ADR Collection               │ DECISION-MAKING
│  3. Trade-off Analyses           │
│                                  │
├─ Integrator ─────────────────────┤
│  1. Integration Guide            │ CONNECTION
│  2. Interaction Diagrams         │ PATTERNS
│  3. SDK Docs                     │
│                                  │
└─ Auditor/Researcher ─────────────┘
   1. Security Docs
   2. Architecture Overview        VERIFICATION
   3. Compliance Integration       & ANALYSIS
```

---

## 🔍 Common Questions - Fast Answers

### Technical Questions

| Question | Answer Location | Time to Find |
|----------|-----------------|--------------|
| How do I register a property? | [Component Diagrams §1](./COMPONENT_INTERACTION_DIAGRAMS.md#1-property-registration-sequence) | 2 min |
| What happens during escrow? | [Component Diagrams §3-4](./COMPONENT_INTERACTION_DIAGRAMS.md#3-escrow-creation--funding) | 5 min |
| How does cross-chain bridge work? | [Component Diagrams §8](./COMPONENT_INTERACTION_DIAGRAMS.md#8-bridge-token-transfer-source-chain) | 7 min |
| Why use Ink! instead of Solidity? | [ADR-001](./ARCHITECTURAL_PRINCIPLES.md#adr-001-ink-smart-contract-framework) | 3 min |
| What are the security guarantees? | [System Overview §7](./SYSTEM_ARCHITECTURE_OVERVIEW.md#security-architecture) | 5 min |
| How is gas optimized? | [System Overview §8.3](./SYSTEM_ARCHITECTURE_OVERVIEW.md#gas-optimization-techniques) | 4 min |

### Design Questions

| Question | Answer Location | Time to Find |
|----------|-----------------|--------------|
| Why modular architecture? | [ADR-002](./ARCHITECTURAL_PRINCIPLES.md#adr-002-modular-contract-architecture) | 3 min |
| Why proxy pattern? | [ADR-003](./ARCHITECTURAL_PRINCIPLES.md#adr-003-proxy-pattern-for-upgradability) | 3 min |
| Trade-offs of compliance approach? | [ADR-005](./ARCHITECTURAL_PRINCIPLES.md#adr-005-on-chain-compliance-registry) | 4 min |
| Privacy vs transparency balance? | [Trade-off Analysis](./ARCHITECTURAL_PRINCIPLES.md#privacy-vs-transparency) | 5 min |

### Operational Questions

| Question | Answer Location | Time to Find |
|----------|-----------------|--------------|
| How to deploy to production? | [Deployment Guide](./deployment.md) | 10 min |
| What monitoring exists? | [System Overview §9](./SYSTEM_ARCHITECTURE_OVERVIEW.md#monitoring--observability) | 5 min |
| Emergency procedures? | [Disaster Recovery](./DISASTER_RECOVERY.md) | 7 min |
| How to upgrade contracts? | [System Overview §10](./SYSTEM_ARCHITECTURE_OVERVIEW.md#upgrade-mechanism) | 4 min |

---

## 📱 One-Page Cheat Sheet

### System at a Glance

```
┌─────────────────────────────────────────────────────┐
│              PROPCHAIN ARCHITECTURE                 │
├─────────────────────────────────────────────────────┤
│                                                     │
│  Users → Gateway → Smart Contracts → Data Layer    │
│                                                     │
│  Core Components:                                   │
│  • Property Registry (ownership records)           │
│  • Escrow (secure transfers)                       │
│  • Compliance (KYC/AML)                            │
│  • Bridge (cross-chain)                            │
│  • Insurance (risk pools)                          │
│  • Oracle (valuations)                             │
│                                                     │
│  Key Features:                                      │
│  ✓ NFT-based property tokens                       │
│  ✓ Multi-sig security                              │
│  ✓ Regulatory compliance built-in                  │
│  ✓ Cross-chain compatible                          │
│  ✓ Upgradeable via proxy                           │
│  ✓ Gas optimized                                   │
│                                                     │
└─────────────────────────────────────────────────────┘
```

### Quick Stats

| Metric | Value |
|--------|-------|
| Total Properties Registered | Check on-chain |
| Active Escrows | Check on-chain |
| Supported Jurisdictions | See compliance docs |
| Average Gas Cost | See benchmarks |
| Security Audit Status | See SECURITY.md |

---

## 🎓 Learning Pathways

### 15-Minute Crash Course

**Goal**: Understand basics fast

```
Minute 0-5:   Read README overview
Minute 5-10:  Skim System Architecture diagrams
Minute 10-15: Review one component flow
```

**Resources**:
- [README](../README.md) - Project overview
- [System Overview §1-3](./SYSTEM_ARCHITECTURE_OVERVIEW.md) - Architecture basics
- [One Component Diagram](./COMPONENT_INTERACTION_DIAGRAMS.md) - Pick relevant flow

**Outcome**: Can discuss system at high level

---

### 1-Hour Deep Dive

**Goal**: Working understanding for development

```
Minute 0-15:  Architecture Index + System Overview
Minute 15-30: Component interactions (relevant section)
Minute 30-45: Contract API (key methods)
Minute 45-60: One tutorial (hands-on)
```

**Resources**:
- [Architecture Index](./ARCHITECTURE_INDEX.md) - Navigation
- [Relevant Component Diagram](./COMPONENT_INTERACTION_DIAGRAMS.md) - Your use case
- [Contract API](./contracts.md) - Method signatures
- [Tutorials](./tutorials/) - Practical example

**Outcome**: Ready to start basic integration

---

### Full-Day Mastery

**Goal**: Comprehensive understanding for core contribution

```
Hour 0-1:   Complete System Overview
Hour 1-2:   All relevant Component Diagrams
Hour 2-3:   Architectural Principles
Hour 3-4:   Key ADRs (001, 002, 003)
Hour 4-5:   Security Architecture
Hour 5-6:   Hands-on implementation
Hour 6-7:   Code review with architect
Hour 7-8:   Q&A and gap filling
```

**Resources**:
- All core architecture documents
- Contract source code
- Development environment setup
- Mentor/architect availability

**Outcome**: Prepared to make core contributions

---

## 🔗 Essential Links

### Core Documents (Must Know)
1. ⭐ [Architecture Index](./ARCHITECTURE_INDEX.md) - Master navigation
2. 📋 [System Overview](./SYSTEM_ARCHITECTURE_OVERVIEW.md) - Big picture
3. 🔗 [Component Diagrams](./COMPONENT_INTERACTION_DIAGRAMS.md) - Detailed flows
4. 📐 [Principles](./ARCHITECTURAL_PRINCIPLES.md) - Design rationale

### Daily Reference (Frequent Use)
- [Contract API](./contracts.md) - Method documentation
- [Integration Guide](./integration.md) - Connection patterns
- [Error Handling](./error-handling.md) - Troubleshooting
- [Best Practices](./best-practices.md) - Coding standards

### Occasional Reference (As Needed)
- [Deployment Guide](./deployment.md) - Production deployment
- [Testing Guide](./testing-guide.md) - Testing strategies
- [Troubleshooting FAQ](./troubleshooting-faq.md) - Common issues
- [Health Checks](./health-checks.md) - Monitoring

### Strategic Reading (Important Context)
- [ADR Collection](./adr/) - Decision history
- [Security Pipeline](./security_pipeline.md) - Security approach
- [Compliance Integration](./compliance-integration.md) - Regulatory
- [Performance Issues](./performance-issue-lazy-loading.md) - Optimization

---

## 🚨 Emergency Quick Access

### Critical Issues

**Security Incident**:
1. [Security Policy](../SECURITY.md) - Immediate steps
2. [Emergency Pause Flow](./COMPONENT_INTERACTION_DIAGRAMS.md#15-emergency-pause-mechanism) - How it works
3. [Disaster Recovery](./DISASTER_RECOVERY.md) - Recovery procedures
4. Contact: security@propchain.io

**System Outage**:
1. [Health Checks](./health-checks.md) - Diagnostic steps
2. [Monitoring Section](./SYSTEM_ARCHITECTURE_OVERVIEW.md#monitoring--observability) - Metrics
3. [Error Scenarios](./COMPONENT_INTERACTION_DIAGRAMS.md#error-handling--edge-cases) - Known issues

**Critical Bug**:
1. [Error Handling](./error-handling.md) - Error taxonomy
2. [Failed Transaction Flow](./COMPONENT_INTERACTION_DIAGRAMS.md#16-failed-transaction-rollback) - Rollback process
3. Create GitHub issue with [BUG] tag

---

## 📞 Getting Help

### Self-Service (Fastest)
1. Search this documentation index
2. Check troubleshooting FAQ
3. Review similar issues on GitHub
4. Read relevant tutorial

### Community Support
- **GitHub Discussions**: General questions
- **Discord**: Real-time chat (PropChain server)
- **Stack Overflow**: Technical Q&A (tag: propchain)

### Direct Support
- **Office Hours**: Weekly architect Q&A (see Discord)
- **1:1 Sessions**: For enterprise partners (email support@propchain.io)
- **Workshops**: Monthly deep-dive sessions (announced in Discord)

---

## ✅ Checklist: Did You Check Documentation?

Before asking for help, verify:

- [ ] Searched Architecture Index
- [ ] Reviewed relevant System Overview section
- [ ] Checked Component Diagrams for your flow
- [ ] Read Contract API documentation
- [ ] Searched existing GitHub issues
- [ ] Checked Troubleshooting FAQ
- [ ] Reviewed tutorials for similar examples

If all checked and still stuck → Ask in Discord or create GitHub issue

---

## 🎯 Success Criteria

You've found what you need when:

✅ Can explain the concept to someone else  
✅ Have working code/example  
✅ Understand trade-offs involved  
✅ Know where to find more details  
✅ Confident in implementation approach  

Still uncertain? → Revisit [Architecture Index](./ARCHITECTURE_INDEX.md) starting point

---

**Quick Reference Version**: 1.0.0  
**Last Updated**: March 27, 2026  
**Maintained By**: PropChain Architecture Team  
**Feedback Welcome**: Create GitHub issue or ask in Discord
