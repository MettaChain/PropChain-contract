# Security Policy & Bug Bounty Program

## Supported Versions

| Version | Supported |
|---------|-----------|
| main (latest) | Yes |
| older branches | No |

Only the latest revision on `main` receives security fixes.

## Reporting a Vulnerability

**Do not open a public GitHub issue for security vulnerabilities.**

Report privately via one of these channels:

- **Email**: security@propchain.io
- **GitHub Private Advisory**: Use the "Report a vulnerability" button on the [Security tab](https://github.com/MettaChain/PropChain-contract/security/advisories/new).

Include as much detail as possible:

- A concise description of the vulnerability
- Steps to reproduce or a proof-of-concept
- Affected contract(s) / function(s) and Rust file paths
- Estimated impact (funds at risk, access control bypass, DoS, etc.)
- Any suggested fix or mitigation

We aim to acknowledge your report within **48 hours** and provide a remediation timeline within **7 business days**.

## Bug Bounty Program

PropChain operates a community bug bounty program. Rewards are paid in USDC on Polkadot.

### Scope

In scope:

- All ink! smart contracts under `contracts/`
- The `security-audit` CLI tool
- Cross-contract interactions (bridge, oracle, escrow, compliance)
- Access control and role management (`contracts/traits/src/access_control.rs`)
- Emergency pause / resume mechanism (`contracts/lib/src/lib.rs`)

Out of scope:

- Off-chain indexer or SDK code unless the vulnerability impacts on-chain state
- Third-party dependencies (report to upstream maintainers instead)
- Issues already reported or duplicates of known issues
- Social engineering or phishing attacks

### Reward Tiers

| Severity | Description | Reward (USDC) |
|----------|-------------|---------------|
| Critical | Arbitrary fund loss, complete access control bypass, permanent DoS | $5,000 – $20,000 |
| High | Partial fund loss, privilege escalation, multi-sig bypass | $1,000 – $5,000 |
| Medium | Temporary DoS, incorrect state transitions, event manipulation | $250 – $1,000 |
| Low | Incorrect error codes, missing events, minor logic flaws | $50 – $250 |
| Informational | Best-practice improvements, gas optimisations | Recognition only |

Severity follows the [CVSS v3.1](https://www.first.org/cvss/calculator/3-1) base score. Final reward is at the discretion of the PropChain security team.

### Rules

1. Test only on testnet (`Rococo` / `Shibuya`). Never attack mainnet contracts.
2. Do not access or exfiltrate user data beyond what is necessary to demonstrate the vulnerability.
3. Avoid actions that degrade system availability for other users.
4. Give us a reasonable time to remediate before public disclosure (coordinated disclosure).
5. One reward per unique vulnerability; duplicates receive no reward.

### Disclosure Timeline

1. **Day 0** – Researcher submits report privately.
2. **Day 1–2** – PropChain confirms receipt and assigns severity.
3. **Day 7** – Remediation plan communicated to researcher.
4. **Day 30–90** – Fix developed, audited, and deployed (depends on severity).
5. **Post-fix** – Public advisory published; researcher credited (if desired).

## Responsible Disclosure Policy

We follow a coordinated disclosure model. We will not pursue legal action against researchers who:

- Act in good faith and report through the channels above
- Do not exploit vulnerabilities beyond a proof-of-concept
- Give us adequate time to remediate before public disclosure

## Contact

security@propchain.io — PGP key available on request.
