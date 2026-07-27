# Dependency Audit & Prelude Unification Specification

This document details the Soroban SDK dependency cleanup and vector prelude unification.

## Cleanup Strategy
- **Soroban SDK**: Workspace dependencies audited and isolated.
- **Prelude Unification**: Standardized on canonical `ink::prelude::vec::Vec` across `contracts/traits`.
- **Monolith Decomposition**: Decomposed `bridge` and `insurance` contracts into focused sub-modules.
