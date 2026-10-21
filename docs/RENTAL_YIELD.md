# RentalYield

`contracts/rental-yield` distributes rental income from a tokenized property to
its shareholders proportionally. It is an ink! 5 contract and is part of the
workspace (`cargo test -p rental-yield`).

## Model

- The contract administrator configures a property with a list of `Share`
  entries. Each share is an account plus a basis-point percentage.
- Every property split must add up to exactly `10_000` bps (100%).
- Rental income is credited to shareholders' unclaimed balances. Shareholders
  withdraw independently with `claim_rental_income`.
- Changing a split affects future distributions only. Income already accrued is
  never erased.

## Messages

| Message | Caller | Purpose |
| --- | --- | --- |
| `new()` | anyone | Deploy; the caller becomes administrator. |
| `set_shares(property_id, shares)` | admin | Replace a property's shareholder split. |
| `distribute_rental_income(property_id, amount)` | admin | Credit `amount` proportionally. |
| `claim_rental_income(property_id)` | shareholder | Transfer the caller's accrued balance. |
| `get_unclaimed_rental_income(property_id, owner)` | anyone | Read an accrued balance. |
| `get_shares(property_id)` | anyone | Read the configured split. |
| `get_share(property_id, owner)` | anyone | Read one owner's share in bps. |
| `get_total_distributed(property_id)` | anyone | Read lifetime income distributed. |
| `admin()` | anyone | Read the administrator. |

## Issues addressed

### #1364 — Optimize gas usage

- A property's shareholder list is stored as a single `Vec<Share>` mapping value,
  so a distribution pays for one list read rather than a separate lookup per
  shareholder.
- Zero-value payouts are skipped; they would cost a storage write without
  creating claimable income.
- One aggregate `RentalIncomeDistributed` event is emitted per distribution
  instead of one event per recipient, so event cost does not grow with the
  number of shareholders.
- Error variants are field-less and the recipient count is tracked locally
  rather than derived from additional storage reads.

### #1365 — Add events for state changes

Every state-changing message emits an event:

| Event | Emitted by | Topics |
| --- | --- | --- |
| `SharesUpdated` | `set_shares` | `property_id` |
| `RentalIncomeDistributed` | `distribute_rental_income` | `property_id` |
| `RentalIncomeClaimed` | `claim_rental_income` | `property_id`, `owner` |

Events are emitted after the corresponding storage write.

### #1366 — Add input validation

- `property_id` must be non-zero.
- Distribution amount must be non-zero.
- A split must be non-empty and contain at most `MAX_SHAREHOLDERS` (32) entries.
- Every shareholder must be non-zero and have a non-zero percentage.
- Duplicate shareholders are rejected.
- The sum of all percentages must be exactly `10_000` bps.
- All validation runs before storage is written, so a rejected update cannot
  leave a partial split behind.

### #1367 — Use custom errors

Every fallible message returns `Result<_, Error>`. The `Error` enum has a
specific variant for each failure mode: `Unauthorized`, `InvalidPropertyId`,
`InvalidAmount`, `PropertyNotFound`, `EmptyShares`, `ZeroShare`,
`ZeroShareholder`, `DuplicateShareholder`, `InvalidTotalShares`,
`TooManyShareholders`, `NoIncomeAvailable`, `ArithmeticOverflow`,
`TransferFailed`, and `ReentrantCall`.

## Safety notes

- `claim_rental_income` is wrapped in the shared `ReentrancyGuard`. The native
  transfer runs before the accrued balance is cleared, so a failed transfer
  leaves the claim intact (returning `Err` does not roll storage back in ink!).
- Payout multiplication and all balance additions use checked arithmetic.
- Rounding dust from integer division remains in the contract; it is not
  credited to any shareholder.
