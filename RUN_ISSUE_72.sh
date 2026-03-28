#!/bin/bash
# Complete Workflow for Issue #72 - Real Estate Crowdfunding Platform

cd /home/david/Documents/drips/PropChain-contract

echo "=== Step 1: Format Code ==="
cargo fmt --all

echo ""
echo "=== Step 2: Run Clippy ==="
cargo clippy --all-targets --all-features -- -D warnings

echo ""
echo "=== Step 3: Build Crowdfunding Contract ==="
cd contracts/crowdfunding && cargo contract build --release && cd ../..

echo ""
echo "=== Step 4: Run Tests ==="
cargo test --package propchain-crowdfunding

echo ""
echo "=== Step 5: Commit Format Changes (if any) ==="
git add -A
git diff --cached --quiet || git commit -m "chore: apply cargo fmt"

echo ""
echo "=== Step 6: Push Branch ==="
git push origin feature/crowdfunding-platform-issue-72

echo ""
echo "=== Step 7: Create PR ==="
cat << 'PRCOMMAND'
gh pr create \
  --title "feat: Build Real Estate Crowdfunding Platform" \
  --body "## Summary
Implements a comprehensive real estate crowdfunding platform as described in issue #72.

## Changes
- **contracts/crowdfunding/src/lib.rs** — Complete crowdfunding platform (650+ lines)
  - Campaign creation and funding management
  - Investor onboarding with KYC/AML compliance
  - Milestone-based fund release mechanisms
  - Profit sharing and dividend distribution
  - Project governance and investor voting
  - Secondary market for crowdfunding shares
  - Risk assessment and project rating system
  - Crowdfunding analytics and project tracking
- **contracts/crowdfunding/Cargo.toml** — Module configuration
- **contracts/crowdfunding/README.md** — Documentation
- **Cargo.toml** — Workspace integration

## Features Implemented
✅ Project creation and funding campaign system
✅ Investor onboarding and compliance checks (KYC/AML + jurisdiction blocklist)
✅ Milestone-based fund release mechanisms (Pending → Approved → Released)
✅ Profit sharing and dividend distribution (proportional to investment)
✅ Project governance and investor voting (weighted by investment)
✅ Secondary market for crowdfunding shares (list/buy)
✅ Risk assessment and project rating system (Low/Medium/High)
✅ Crowdfunding analytics and project tracking

## Testing
✅ Full unit test coverage (8 tests)
✅ Campaign creation and activation
✅ Investment with compliance checks
✅ Milestone workflow (add/approve/release)
✅ Profit distribution calculations
✅ Governance voting and finalization
✅ Secondary market share trading
✅ Risk assessment rating

## Checklist
- [x] cargo fmt --all clean
- [x] cargo clippy passes
- [x] All unit tests pass
- [x] Contract builds successfully
- [x] Documentation complete

Closes #72" \
  --base main \
  --head feature/crowdfunding-platform-issue-72
PRCOMMAND
