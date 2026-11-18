# tenant-onboarding

Tenant pipeline: `submit_application` → `approve_application` / `reject_application` → `sign_lease` → `confirm_move_in`,
with `get_application_status`. Closes #1439. Only hashes of off-chain documents are stored. Landlords are registered per
property by the admin; the contract does not call the property-management contract (creating the on-chain lease at move-in
is left to the integration layer, using the `MoveInConfirmed` event).

```bash
cargo test --manifest-path contract-extensions/contracts/tenant-onboarding/Cargo.toml
```
