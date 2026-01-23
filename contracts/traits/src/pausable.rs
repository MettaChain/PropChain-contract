#![cfg_attr(not(feature = "std"), no_std)]

use ink::primitives::AccountId;

#[cfg(not(feature = "std"))]
use scale_info::prelude::{string::String, vec::Vec};

/// Pausable trait for emergency contract pause/resume functionality
pub trait Pausable {
    /// Error type for pausable operations
    type Error;

    /// Pause the contract (emergency stop)
    fn pause(&mut self) -> Result<(), Self::Error>;

    /// Resume the contract after pause
    fn resume(&mut self) -> Result<(), Self::Error>;

    /// Check if contract is paused
    fn is_paused(&self) -> bool;

    /// Add a pauser role to an account
    fn add_pauser(&mut self, account: AccountId) -> Result<(), Self::Error>;

    /// Remove a pauser role from an account
    fn remove_pauser(&mut self, account: AccountId) -> Result<(), Self::Error>;

    /// Check if an account has pauser role
    fn is_pauser(&self, account: AccountId) -> bool;

    /// Schedule automatic resume at a specific timestamp
    fn schedule_auto_resume(&mut self, timestamp: u64) -> Result<(), Self::Error>;

    /// Cancel scheduled automatic resume
    fn cancel_auto_resume(&mut self) -> Result<(), Self::Error>;

    /// Get scheduled auto-resume timestamp
    fn get_auto_resume_time(&self) -> Option<u64>;

    /// Add a resume approver (for multi-sig resume)
    fn add_resume_approver(&mut self, account: AccountId) -> Result<(), Self::Error>;

    /// Remove a resume approver
    fn remove_resume_approver(&mut self, account: AccountId) -> Result<(), Self::Error>;

    /// Approve resume (multi-sig)
    fn approve_resume(&mut self) -> Result<(), Self::Error>;

    /// Get current resume approval count
    fn get_resume_approvals(&self) -> u32;

    /// Get required resume approvals threshold
    fn get_required_approvals(&self) -> u32;

    /// Set required resume approvals threshold
    fn set_required_approvals(&mut self, threshold: u32) -> Result<(), Self::Error>;
}

/// Pause state information
#[derive(Debug, Clone, PartialEq, scale::Encode, scale::Decode, ink::storage::traits::StorageLayout)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub struct PauseState {
    /// Whether the contract is currently paused
    pub is_paused: bool,
    /// Timestamp when the contract was paused
    pub paused_at: Option<u64>,
    /// Account that triggered the pause
    pub paused_by: Option<AccountId>,
    /// Reason for the pause
    pub pause_reason: String,
    /// Scheduled automatic resume timestamp
    pub auto_resume_at: Option<u64>,
    /// Number of times the contract has been paused
    pub pause_count: u32,
}

/// Pause event record for audit trail
#[derive(Debug, Clone, PartialEq, scale::Encode, scale::Decode, ink::storage::traits::StorageLayout)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub struct PauseEvent {
    /// Event type (Pause, Resume, etc.)
    pub event_type: PauseEventType,
    /// Timestamp of the event
    pub timestamp: u64,
    /// Account that triggered the event
    pub triggered_by: AccountId,
    /// Additional details
    pub details: String,
}

/// Types of pause events
#[derive(Debug, Clone, PartialEq, scale::Encode, scale::Decode, ink::storage::traits::StorageLayout)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub enum PauseEventType {
    Paused,
    Resumed,
    AutoResumeScheduled,
    AutoResumeCancelled,
    PauserAdded,
    PauserRemoved,
    ApproverAdded,
    ApproverRemoved,
    ResumeApproved,
    EmergencyOverride,
}

impl Default for PauseState {
    fn default() -> Self {
        Self {
            is_paused: false,
            paused_at: None,
            paused_by: None,
            pause_reason: String::new(),
            auto_resume_at: None,
            pause_count: 0,
        }
    }
}
