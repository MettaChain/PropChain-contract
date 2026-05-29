# TODO: Implement Tax Deadline Notifications

## Plan Steps (Approved)

1. [x] Update contracts/tax-compliance/src/tax_engine.rs: Add `days_until_due` helper function.
2. [x] Update contracts/tax-compliance/src/lib.rs: Add events `TaxDeadlineApproaching`, `TaxDeadlineNotification`; emit in `calculate_tax()` and `check_compliance()`.
3. [x] Update contracts/tax-compliance/src/compliance.rs: No changes needed (generate_alerts already supports PaymentDueSoon/TaxOverdue).
4. [x] Update docs/compliance-regulatory-framework.md: Document new features.
5. [x] Update README.md: Add feature mention.
6. [ ] Add/update tests in contracts/tax-compliance/src/lib.rs.
7. [ ] Run `cargo test` and `./scripts/test.sh`.
8. [ ] Complete task.

Progress will be updated after each step.
