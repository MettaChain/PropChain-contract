# Formal Verification, Static Analysis, and Extension Contribution Guide

This lean guide addresses:
- **#1500 Documentation: Formal Verification and Static Analysis Guide**
- **#1499 Documentation: Contract Extension Contribution Guide**

It is scoped to `contract-extensions/` documentation and references repository tooling/config used by PropChain contributors.

## 1. Formal verification and static analysis (#1500)

### Kani formal verification
- Reference: `docs/formal_verification.md` and `.github/workflows/formal-verification.yml`.
- Typical flow: install Kani toolchain, add/maintain proof harnesses for critical invariants, and run Kani in CI via the formal verification workflow.
- When adding proofs for new contracts, focus on state-machine invariants, arithmetic bounds, and access-control pre/post-conditions. Keep harnesses small and deterministic.

### Clippy configuration and checks
- Reference: `clippy.toml`, `scripts/check_clippy_config.sh`, and CI lint jobs.
- Use workspace clippy rules consistently; avoid introducing `allow` attributes without justification.
- Local expectation: `cargo clippy --all-targets --all-features` should pass cleanly for the affected crate(s) (contributors run this before PRs as required).

### cargo-audit and dependency/security policy
- Reference: `.cargo/audit.toml`, `deny.toml`, `deny-new.toml`, and security workflows.
- `cargo audit` checks advisory vulnerabilities against the configured ignore/justification policy.
- Dependency policy is enforced with `cargo-deny` configs; changes to dependencies should be reviewed for license, source, and advisory impact.

### Secrets detection
- Reference: `.secrets.baseline` and `.secrets-audit-justifications.md`.
- Secret scanning baselines must be kept accurate; do not commit credentials or private keys. Any baseline update should include a clear justification.

### security-audit crate and automated auditing
- Reference: `security-audit/` crate.
- This crate supports automated/security-focused checks used in the project’s audit process; integrate findings into fixes and document notable exceptions.

### Mutation testing
- Reference: `.cargo/mutants.toml`.
- `cargo mutants` configuration guides mutation-testing runs for selected targets to assess test effectiveness. Use it to strengthen critical-path tests where practical.

### Writing Kani proofs for new contracts (quick guide)
1. Identify safety-critical invariants (balances, access control, state transitions).
2. Add a proof harness (`#[kani::proof]`) with constrained symbolic inputs.
3. Use assumptions to model valid environments and assertions for invariants.
4. Run Kani locally and ensure CI workflow coverage for the new proof scope.

## 2. Contract extension contribution guide (#1499)

### `contract-extensions/` structure and purpose
- `contract-extensions/contracts/`: extension contract crates.
- `contract-extensions/docs/`: contributor and architecture documentation.
- `contract-extensions/examples/`: runnable/reference examples.
- `contract-extensions/tools/`: helper tools, analyzers, and models.
- `contract-extensions/tests/`: extension-focused test crates.
- `contract-extensions/scripts/`: operational/deployment scripts.

Extensions should be self-contained, composable, and isolated from core `contracts/` unless explicitly required.

### Required files for a new extension module
- `Cargo.toml` with correct crate metadata/dependencies.
- `src/lib.rs` (or binary entry as appropriate) implementing the module.
- Tests (unit/integration as applicable) and `README.md` when behavior or usage needs explanation.

### Workspace membership and conventions
- Add the crate to the relevant workspace members when creating a new buildable package.
- Follow repository conventions from `CONTRIBUTING.md`, `docs/CONTRIBUTING.md`, and existing extension patterns.
- Prefer shared types/patterns from `propchain_traits` where suitable to keep interfaces consistent.

### Isolation principle
- **Allowed:** changes within `contract-extensions/` for extension-scoped issues.
- **Not allowed (unless explicitly required):** modifying `contracts/`, root `tests/`, `scripts/`, or unrelated directories.
- Keep diffs minimal, focused, and reviewable.

### Testing requirements and PR process
- Expected checks for contributors: formatting, linting, and tests for affected crates (e.g. `cargo fmt`, `cargo clippy`, `cargo test`) as applicable to the change.
- Use the PR template from `.github/pull_request_template.md`: summarize changes, link issues, and note validation performed.
- Ensure code/comments/docs are clear and security-relevant behavior is called out.

### PR submission checklist
- [ ] Change is isolated to the intended folder/scope.
- [ ] Required module files (`Cargo.toml`, `src/lib.rs`, tests/docs as needed) are present.
- [ ] Conventions and shared traits/patterns are followed.
- [ ] Lint/format/tests run for affected scope (or limitations documented).
- [ ] PR description includes context, linked issues, and review notes.
