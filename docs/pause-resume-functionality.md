# Contract Pause/Resume Functionality

## Overview

The PropertyRegistry contract now includes comprehensive emergency pause/resume functionality to allow for quick response to security vulnerabilities or critical bugs. This feature provides contract-wide pause capabilities with role-based permissions, multi-signature resume requirements, and automatic resume scheduling.

## Features

### 1. Pause Mechanism

#### Contract-Wide Pause

- **Emergency Stop**: Authorized pausers can immediately halt all critical contract operations
- **Pause State Tracking**: Complete visibility into pause status, timestamp, and who triggered it
- **Pause Counter**: Tracks the number of times the contract has been paused for audit purposes

#### Role-Based Pause Permissions

- **Pauser Role**: Dedicated role for accounts authorized to pause the contract
- **Admin Control**: Only the contract admin can add or remove pausers
- **Initial Setup**: Contract deployer (admin) is automatically assigned as the first pauser

#### Emergency Pause Procedures

```rust
// Pause the contract (requires pauser role)
contract.pause()?;

// Check pause state
let is_paused = contract.is_paused();
let pause_state = contract.get_pause_state();
```

#### Time-Based Automatic Resume

- **Scheduled Resume**: Set a future timestamp for automatic contract resumption
- **Flexible Scheduling**: Can be scheduled, rescheduled, or cancelled as needed
- **Automatic Trigger**: Contract automatically resumes when the scheduled time is reached

```rust
// Schedule auto-resume for 24 hours from now
let resume_time = current_timestamp + (24 * 60 * 60 * 1000);
contract.schedule_auto_resume(resume_time)?;

// Cancel scheduled auto-resume
contract.cancel_auto_resume()?;
```

### 2. Resume Process

#### Multi-Signature Resume Requirement

- **Approval Threshold**: Configurable number of approvals required to resume
- **Resume Approver Role**: Dedicated role for accounts authorized to approve resume
- **Approval Tracking**: System tracks which approvers have approved and total count
- **Idempotent Approvals**: Same approver can't approve multiple times

```rust
// Add resume approvers
contract.add_resume_approver(approver1)?;
contract.add_resume_approver(approver2)?;

// Set threshold (e.g., require 2 approvals)
contract.set_required_approvals(2)?;

// Approvers submit their approvals
contract.approve_resume()?; // Called by approver1
contract.approve_resume()?; // Called by approver2

// Resume once threshold is met
contract.resume()?;
```

#### Security Audit Verification

- **Approval Process**: Multiple approvers ensure thorough review before resuming
- **Audit Trail**: Complete history of all pause/resume events
- **Event Logging**: All actions are logged with timestamps and actor information

#### Admin Override

- **Emergency Resume**: Admin can bypass multi-sig requirements for immediate resume
- **Use Case**: Critical situations where waiting for approvals would cause harm

```rust
// Admin can resume immediately without waiting for approvals
contract.resume()?; // Called by admin
```

#### Gradual Feature Re-enablement

- **Controlled Resume**: Contract resumes all features simultaneously
- **State Reset**: Resume approval counters are automatically reset after successful resume

### 3. Safety Features

#### Pause State Visibility

```rust
// Check if contract is paused
let is_paused = contract.is_paused();

// Get detailed pause state
let pause_state = contract.get_pause_state();
// Returns: PauseState {
//     is_paused: bool,
//     paused_at: Option<u64>,
//     paused_by: Option<AccountId>,
//     pause_reason: String,
//     auto_resume_at: Option<u64>,
//     pause_count: u32,
// }
```

#### Emergency Override Mechanisms

- **Admin Override**: Admin can always resume regardless of approval count
- **Auto-Resume**: Scheduled automatic resume as a failsafe

#### Audit Trail for Pause/Resume Actions

```rust
// Get recent pause events
let events = contract.get_pause_events(limit: 10);

// Get total event count
let total_events = contract.get_pause_event_count();
```

Event types tracked:

- `Paused` - Contract was paused
- `Resumed` - Contract was resumed
- `AutoResumeScheduled` - Auto-resume was scheduled
- `AutoResumeCancelled` - Auto-resume was cancelled
- `PauserAdded` - New pauser role granted
- `PauserRemoved` - Pauser role revoked
- `ApproverAdded` - New resume approver added
- `ApproverRemoved` - Resume approver removed
- `ResumeApproved` - Resume approval submitted
- `EmergencyOverride` - Admin override used

#### User Notification System

All pause/resume actions emit events:

- `ContractPaused` - Emitted when contract is paused
- `ContractResumed` - Emitted when contract is resumed
- `AutoResumeScheduled` - Emitted when auto-resume is scheduled
- `AutoResumeCancelled` - Emitted when auto-resume is cancelled
- `PauserAdded` / `PauserRemoved` - Role management events
- `ResumeApproverAdded` / `ResumeApproverRemoved` - Approver management events
- `ResumeApproved` - Emitted when an approver approves resume
- `RequiredApprovalsChanged` - Emitted when threshold is modified

## Protected Operations

When the contract is paused, the following operations are blocked:

- `register_property` - Cannot register new properties
- `transfer_property` - Cannot transfer property ownership
- `create_escrow` - Cannot create new escrows
- `release_escrow` - Cannot release escrow funds

Read-only operations remain available during pause.

## Role Management

### Pauser Role

```rust
// Add a pauser (admin only)
contract.add_pauser(account)?;

// Remove a pauser (admin only)
contract.remove_pauser(account)?;

// Check if account is a pauser
let is_pauser = contract.is_pauser(account);
```

### Resume Approver Role

```rust
// Add a resume approver (admin only)
contract.add_resume_approver(account)?;

// Remove a resume approver (admin only)
contract.remove_resume_approver(account)?;
```

### Approval Threshold Management

```rust
// Set required approvals (admin only)
contract.set_required_approvals(threshold)?;

// Get current threshold
let threshold = contract.get_required_approvals();

// Get current approval count
let approvals = contract.get_resume_approvals();
```

## Usage Examples

### Example 1: Emergency Pause

```rust
// Security team detects a vulnerability
contract.pause()?;

// Schedule auto-resume for 48 hours (time to fix)
let resume_time = current_time + (48 * 60 * 60 * 1000);
contract.schedule_auto_resume(resume_time)?;

// Notify community via events
```

### Example 2: Multi-Sig Resume After Security Audit

```rust
// Contract was paused due to bug
// Bug has been fixed, now need approval to resume

// Security auditor 1 approves
contract.approve_resume()?; // Called by auditor1

// Security auditor 2 approves
contract.approve_resume()?; // Called by auditor2

// Check if threshold is met
let approvals = contract.get_resume_approvals();
let required = contract.get_required_approvals();

if approvals >= required {
    // Resume the contract
    contract.resume()?;
}
```

### Example 3: Admin Emergency Override

```rust
// Critical situation requires immediate resume
// Admin can bypass multi-sig requirements
contract.resume()?; // Called by admin - works immediately
```

## Error Handling

The pause/resume functionality includes comprehensive error handling:

- `ContractPaused` - Operation attempted while contract is paused
- `ContractNotPaused` - Resume/schedule attempted on active contract
- `NotPauser` - Non-pauser attempted to pause or schedule auto-resume
- `NotResumeApprover` - Non-approver attempted to approve resume
- `InsufficientApprovals` - Resume attempted without enough approvals
- `InvalidAutoResumeTime` - Auto-resume scheduled for past/current time
- `AutoResumeNotScheduled` - Cancel attempted when no auto-resume scheduled
- `InvalidApprovalThreshold` - Threshold set to zero
- `Unauthorized` - Non-admin attempted admin-only operation

## Best Practices

1. **Multiple Pausers**: Assign multiple trusted accounts as pausers for redundancy
2. **Appropriate Threshold**: Set approval threshold based on team size and security needs
3. **Diverse Approvers**: Include different stakeholders (devs, auditors, community) as approvers
4. **Auto-Resume Failsafe**: Always schedule auto-resume when pausing to prevent indefinite pause
5. **Monitor Events**: Subscribe to pause/resume events for real-time notifications
6. **Regular Audits**: Review pause event history periodically
7. **Test Procedures**: Regularly test pause/resume procedures in testnet
8. **Documentation**: Keep pause/resume procedures documented and accessible to team

## Testing

Comprehensive test coverage includes:

- Basic pause/resume functionality
- Role-based access control
- Multi-signature resume process
- Auto-resume scheduling and cancellation
- Pause state checks on critical operations
- Audit trail verification
- Edge cases and error conditions

Run tests with:

```bash
cargo test
```

## Security Considerations

1. **Pauser Selection**: Only assign pauser role to highly trusted accounts
2. **Approver Diversity**: Ensure approvers represent different interests/perspectives
3. **Threshold Balance**: Set threshold high enough for security, low enough for practicality
4. **Auto-Resume Timing**: Set reasonable auto-resume times (not too short, not too long)
5. **Admin Key Security**: Protect admin private key as it can override all restrictions
6. **Event Monitoring**: Monitor pause/resume events for suspicious activity
7. **Regular Reviews**: Periodically review and update pauser/approver lists

## Future Enhancements

Potential future improvements:

- Partial pause (pause specific functions, not all)
- Time-locked resume (minimum pause duration)
- Weighted multi-sig (different approvers have different weights)
- Pause reasons enumeration (categorize pause reasons)
- Integration with external monitoring systems
- Automated pause triggers based on on-chain metrics
