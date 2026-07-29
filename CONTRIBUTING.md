# Contributing to PropChain Smart Contracts

Thank you for helping make PropChain better!  This guide covers the process
for reporting issues, proposing changes, and submitting pull requests.

## Table of Contents

1. [Code of Conduct](#code-of-conduct)
2. [Reporting Issues](#reporting-issues)
3. [Development Workflow](#development-workflow)
4. [Pull Request Guidelines](#pull-request-guidelines)
5. [Code Style](#code-style)
6. [Testing Requirements](#testing-requirements)
7. [Documentation Requirements](#documentation-requirements)

---

## Code of Conduct

All contributors are expected to be respectful and professional.
Harassment or discriminatory behaviour will not be tolerated.

---

## Reporting Issues

1. Search [existing issues](https://github.com/MettaChain/PropChain-contract/issues)
   before opening a new one.
2. Use the appropriate issue template (bug report, feature request, etc.).
3. Include a minimal reproduction case for bugs.
4. Tag issues with the relevant contract name (e.g. `lending`, `insurance`).

---

## Development Workflow

```bash
# 1. Fork the repository and clone your fork
git clone https://github.com/<your-username>/PropChain-contract.git
cd PropChain-contract

# 2. Create a feature branch
git checkout -b feat/my-feature

# 3. Make your changes, then run the full test suite
cargo test --all-features --workspace

# 4. Check formatting and lints
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings

# 5. Commit with a clear message
git commit -m "feat(lending): add amortization schedule support"

# 6. Push and open a pull request against `main`
git push -u origin feat/my-feature
```

For full setup instructions see [DEVELOPMENT.md](./DEVELOPMENT.md).

---

## Pull Request Guidelines

- **One concern per PR** — keep changes focused.
- **Title format**: `type(scope): short description`
  - Types: `feat`, `fix`, `docs`, `chore`, `refactor`, `test`
  - Example: `fix(lending): prevent zero-division in borrow_rate`
- Fill in the [PR template](.github/pull_request_template.md) completely.
- All CI checks must pass before merge:
  - `cargo fmt --check`
  - `cargo clippy -- -D warnings`
  - `cargo test --all-features --workspace`
  - ARCHITECTURE.md link check

### Review Process

1. At least one maintainer approval is required.
2. Address all review comments before requesting re-review.
3. Squash commits before merge (maintainers may squash on merge).

---

## Code Style

### Formatting

The project uses **stable** `rustfmt` for general formatting and **nightly**
`rustfmt` with `format_macro_matchers = true` for ink! macro bodies.

```bash
# Format all code (stable)
cargo fmt

# Format ink! macros (nightly — required for macro-heavy files)
cargo +nightly fmt
```

See `rustfmt.toml` in the repo root for the full configuration.

### Clippy

All Clippy warnings are treated as errors in CI.  Run locally with:

```bash
cargo clippy --all-targets --all-features -- -D warnings
```

### Naming Conventions

| Item | Convention | Example |
|------|-----------|---------|
| Types / Traits | `PascalCase` | `LoanApplication` |
| Functions / Methods | `snake_case` | `apply_for_loan` |
| Constants | `SCREAMING_SNAKE_CASE` | `MAX_LTV_RATIO` |
| Storage fields | `snake_case` | `loan_count` |

---

## Testing Requirements

- **Every new feature** must include unit tests.
- **Every bug fix** must include a regression test.
- Tests live alongside their module (`mod tests` inside `lib.rs`) **or** in
  a dedicated `tests/` directory for larger suites.
- See the [lending test module layout](./contracts/lending/README.md#test-module-layout)
  for the project's three-layer test organisation pattern.

Run the test suite:

```bash
cargo test --all-features --workspace
```

---

## Documentation Requirements

- All public `fn`, `struct`, `enum`, and `mod` items must have rustdoc
  comments (`///`).
- `mod tests` blocks must include a rustdoc header describing the test
  group's purpose and scope.
- Update `ARCHITECTURE.md` if your change affects the high-level design.
- Verify all links in `ARCHITECTURE.md` still resolve after your change:

```bash
python3 scripts/verify_doc_sync.sh  # or the inline CI check
```

---

## Getting Help

- Open a [GitHub Discussion](https://github.com/MettaChain/PropChain-contract/discussions)
  for questions.
- Join the PropChain [Discord](https://discord.gg/propchain) for real-time help.
- Email the maintainers: contracts@propchain.io
