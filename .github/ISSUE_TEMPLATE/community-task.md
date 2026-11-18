---
name: Community Task
description: Self-contained task for the contract-extensions folder
labels: ["community-contribution"]
---
## Task Description

<!-- Clear, specific description of what needs to be built -->

## Target Folder

`contract-extensions/`

**Subfolder:** `<!-- e.g., contracts/, tests/, tools/ -->`

## Acceptance Criteria

- [ ] Code is placed in the correct subfolder under `contract-extensions/`
- [ ] No files outside `contract-extensions/` are modified
- [ ] Rust code follows existing ink! patterns and conventions
- [ ] Tests are included (unit and/or integration)
- [ ] `cargo clippy` passes with no warnings
- [ ] `cargo test` passes
- [ ] `cargo build` succeeds

## Technical Context

<!-- Relevant technical details, patterns to follow, dependencies -->

## Prerequisites

<!-- Any dependencies or knowledge needed -->

## Isolation Statement

**All changes must remain within the `contract-extensions/` folder. Do not modify files in `contracts/`, `tests/`, `scripts/`, or any other directory unless this issue explicitly requires it.**
