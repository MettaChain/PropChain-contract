# Clippy & Lint Suppressions Triage Guide

This document triages `#[allow(...)]` suppressions and `clippy::too_many_arguments` usage workspace-wide.

## Policy Guidelines
- **Centralized Suppressions**: Avoid ad-hoc inline `#[allow(...)]` tags where structural refactoring is possible.
- **`too_many_arguments` Reduction**: Refactor multi-parameter functions to pass struct wrappers.
- **Reentrancy Protection**: All liquidation and asset-moving paths must execute behind non-reentrant state locks.
