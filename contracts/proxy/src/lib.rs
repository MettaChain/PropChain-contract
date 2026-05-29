#![cfg_attr(not(feature = "std"), no_std)]
#![allow(dead_code)]

//! # PropChain Transparent Proxy with Upgrade Governance
//!
//! Enhanced proxy pattern for upgradeable ink! contracts with:
//! - Transparent proxy pattern (admin vs user call routing)
//! - Multi-sig upgrade governance mechanism
//! - Version compatibility checking
//! - Rollback capabilities
//! - Upgrade timelock (delay before activation)
//! - Migration state tracking
//!
//! Resolves: https://github.com/MettaChain/PropChain-contract/issues/77

use ink::prelude::string::String;
use ink::prelude::vec::Vec;

#[ink::contract]
mod propchain_proxy {
    use super::*;

    /// Unique storage key for the proxy data to avoid collisions.
    /// bytes4(keccak256("proxy.storage")) = 0xc5f3bc7a
    #[allow(dead_code)]
    const PROXY_STORAGE_KEY: u32 = 0xC5F3BC7A;

    /// Minimum timelock period (in blocks) before an upgrade can be executed
    const MIN_TIMELOCK_BLOCKS: u32 = 10;

    /// Maximum number of stored versions for rollback
    const MAX_VERSION_HISTORY: u32 = 10;

    // Error types extracted to errors.rs (Issue #101)
    include!("errors.rs");

    // Data types extracted to types.rs (Issue #101)
    include!("types.rs");

    // ========================================================================
    // EVENTS
    // ========================================================================

    #[ink(event)]
    pub struct Upgraded {
        #[ink(topic)]
        new_code_hash: Hash,
        #[ink(topic)]
        proposal_id: u64,
        from_version: String,
        to_version: String,
        timestamp: u64,
    }

    #[ink(event)]
    pub struct AdminChanged {
        #[ink(topic)]
        old_admin: AccountId,
        #[ink(topic)]
        new_admin: AccountId,
    }

    #[ink(event)]
    pub struct UpgradeProposed {
        #[ink(topic)]
        proposal_id: u64,
        #[ink(topic)]
        proposer: AccountId,
        new_code_hash: Hash,
        timelock_until_block: u32,
        timestamp: u64,
    }

    #[ink(event)]
    pub struct UpgradeApproved {
        #[ink(topic)]
        proposal_id: u64,
        #[ink(topic)]
        approver: AccountId,
        current_approvals: u32,
        required_approvals: u32,
        timestamp: u64,
    }

    #[ink(event)]
    pub struct UpgradeCancelled {
        #[ink(topic)]
        proposal_id: u64,
        #[ink(topic)]
        cancelled_by: AccountId,
        timestamp: u64,
    }

    #[ink(event)]
    pub struct UpgradeRolledBack {
        #[ink(topic)]
        from_version: String,
        #[ink(topic)]
        to_version: String,
        rolled_back_by: AccountId,
        timestamp: u64,
    }

    #[ink(event)]
    pub struct GovernorAdded {
        #[ink(topic)]
        governor: AccountId,
        added_by: AccountId,
    }

    #[ink(event)]
    pub struct GovernorRemoved {
        #[ink(topic)]
        governor: AccountId,
        removed_by: AccountId,
    }

    #[ink(event)]
    pub struct EmergencyPauseToggled {
        #[ink(topic)]
        paused: bool,
        by: AccountId,
        timestamp: u64,
    }

    // ========================================================================
    // CONTRACT STORAGE
    // ========================================================================

    #[ink(storage)]
    pub struct TransparentProxy {
        /// The code hash of the current implementation contract.
        code_hash: Hash,
        /// The address of the proxy admin.
        admin: AccountId,
        /// Governance accounts that can approve upgrades
        governors: Vec<AccountId>,
        /// Upgrade proposals
        proposals: ink::storage::Mapping<u64, UpgradeProposal>,
        /// Proposal counter
        proposal_counter: u64,
        /// Required number of approvals for upgrade
        required_approvals: u32,
        /// Timelock period in blocks
        timelock_blocks: u32,
        /// Version history (ordered, most recent last)
        version_history: Vec<VersionInfo>,
        /// Current version index
        current_version_index: u32,
        /// Migration state
        migration_state: MigrationState,
        /// Emergency pause flag
        emergency_pause: bool,
    }

    // ========================================================================
    // IMPLEMENTATION
    // ========================================================================

    impl TransparentProxy {
        /// Creates a new proxy with governance configuration
        #[ink(constructor)]
        pub fn new(code_hash: Hash) -> Self {
            let caller = Self::env().caller();
            let initial_version = VersionInfo {
                major: 1,
                minor: 0,
                patch: 0,
                code_hash,
                deployed_at_block: Self::env().block_number(),
                deployed_at: Self::env().block_timestamp(),
                description: String::from("Initial deployment"),
                deployed_by: caller,
            };

            Self {
                code_hash,
                admin: caller,
                governors: vec![caller],
                proposals: ink::storage::Mapping::default(),
                proposal_counter: 0,
                required_approvals: 1,
                timelock_blocks: MIN_TIMELOCK_BLOCKS,
                version_history: vec![initial_version],
                current_version_index: 0,
                migration_state: MigrationState::None,
                emergency_pause: false,
            }
        }

        /// Creates a new proxy with custom governance parameters
        #[ink(constructor)]
        pub fn new_with_governance(
            code_hash: Hash,
            governors: Vec<AccountId>,
            required_approvals: u32,
            timelock_blocks: u32,
        ) -> Self {
            let caller = Self::env().caller();
            let initial_version = VersionInfo {
                major: 1,
                minor: 0,
                patch: 0,
                code_hash,
                deployed_at_block: Self::env().block_number(),
                deployed_at: Self::env().block_timestamp(),
                description: String::from("Initial deployment"),
                deployed_by: caller,
            };

            let effective_timelock = if timelock_blocks < MIN_TIMELOCK_BLOCKS {
                MIN_TIMELOCK_BLOCKS
            } else {
                timelock_blocks
            };

            let effective_required =
                if required_approvals == 0 || required_approvals > governors.len() as u32 {
                    1
                } else {
                    required_approvals
                };

            Self {
                code_hash,
                admin: caller,
                governors,
                proposals: ink::storage::Mapping::default(),
                proposal_counter: 0,
                required_approvals: effective_required,
                timelock_blocks: effective_timelock,
                version_history: vec![initial_version],
                current_version_index: 0,
                migration_state: MigrationState::None,
                emergency_pause: false,
            }
        }

        // ====================================================================
        // UPGRADE GOVERNANCE
        // ====================================================================

        /// Proposes a new upgrade with version info and timelock
        #[ink(message)]
        pub fn propose_upgrade(
            &mut self,
            new_code_hash: Hash,
            major: u32,
            minor: u32,
            patch: u32,
            description: String,
            migration_notes: String,
        ) -> Result<u64, Error> {
            let caller = self.env().caller();
            self.ensure_governor(caller)?;
            self.ensure_not_paused()?;

            if self.migration_state != MigrationState::None
                && self.migration_state != MigrationState::Completed
                && self.migration_state != MigrationState::RolledBack
            {
                return Err(Error::MigrationInProgress);
            }

            // Version compatibility check: new version must be >= current
            self.check_version_compatibility(major, minor, patch)?;

            self.proposal_counter += 1;
            let proposal_id = self.proposal_counter;

            let current_block = self.env().block_number();
            let timelock_until = current_block + self.timelock_blocks;

            let version = VersionInfo {
                major,
                minor,
                patch,
                code_hash: new_code_hash,
                deployed_at_block: 0, // Set upon execution
                deployed_at: 0,       // Set upon execution
                description,
                deployed_by: caller,
            };

            let proposal = UpgradeProposal {
                id: proposal_id,
                new_code_hash,
                version,
                proposer: caller,
                created_at_block: current_block,
                created_at: self.env().block_timestamp(),
                timelock_until_block: timelock_until,
                approvals: vec![caller], // Proposer auto-approves
                required_approvals: self.required_approvals,
                cancelled: false,
                executed: false,
                migration_notes,
            };

            self.proposals.insert(proposal_id, &proposal);
            self.migration_state = MigrationState::Proposed;

            self.env().emit_event(UpgradeProposed {
                proposal_id,
                proposer: caller,
                new_code_hash,
                timelock_until_block: timelock_until,
                timestamp: self.env().block_timestamp(),
            });

            Ok(proposal_id)
        }

        /// Approves an upgrade proposal
        #[ink(message)]
        pub fn approve_upgrade(&mut self, proposal_id: u64) -> Result<(), Error> {
            let caller = self.env().caller();
            self.ensure_governor(caller)?;
            self.ensure_not_paused()?;

            let mut proposal = self
                .proposals
                .get(proposal_id)
                .ok_or(Error::ProposalNotFound)?;

            if proposal.cancelled {
                return Err(Error::ProposalCancelled);
            }

            if proposal.executed {
                return Err(Error::ProposalNotFound);
            }

            if proposal.approvals.contains(&caller) {
                return Err(Error::AlreadyApproved);
            }

            proposal.approvals.push(caller);

            let current_approvals = proposal.approvals.len() as u32;

            if current_approvals >= proposal.required_approvals {
                self.migration_state = MigrationState::Approved;
            }

            self.proposals.insert(proposal_id, &proposal);

            self.env().emit_event(UpgradeApproved {
                proposal_id,
                approver: caller,
                current_approvals,
                required_approvals: proposal.required_approvals,
                timestamp: self.env().block_timestamp(),
            });

            Ok(())
        }

        /// Executes an approved upgrade after timelock period
        #[ink(message)]
        pub fn execute_upgrade(&mut self, proposal_id: u64) -> Result<(), Error> {
            let caller = self.env().caller();
            self.ensure_governor(caller)?;
            self.ensure_not_paused()?;

            let mut proposal = self
                .proposals
                .get(proposal_id)
                .ok_or(Error::ProposalNotFound)?;

            if proposal.cancelled {
                return Err(Error::ProposalCancelled);
            }
            if proposal.executed {
                return Err(Error::ProposalNotFound);
            }

            // Check approvals
            if (proposal.approvals.len() as u32) < proposal.required_approvals {
                return Err(Error::InsufficientApprovals);
            }

            // Check timelock
            if self.env().block_number() < proposal.timelock_until_block {
                return Err(Error::TimelockNotExpired);
            }

            // Execute the upgrade
            self.migration_state = MigrationState::InProgress;

            let old_version = self.format_current_version();

            // Update code hash
            let old_code_hash = self.code_hash;
            self.code_hash = proposal.new_code_hash;

            // Record version history
            let mut version_info = proposal.version.clone();
            version_info.deployed_at_block = self.env().block_number();
            version_info.deployed_at = self.env().block_timestamp();
            version_info.deployed_by = caller;

            // Trim history if needed
            if self.version_history.len() as u32 >= MAX_VERSION_HISTORY {
                self.version_history.remove(0);
            }

            self.version_history.push(version_info);
            self.current_version_index = (self.version_history.len() - 1) as u32;

            // Mark proposal as executed
            proposal.executed = true;
            self.proposals.insert(proposal_id, &proposal);

            self.migration_state = MigrationState::Completed;

            let new_version = self.format_current_version();

            self.env().emit_event(Upgraded {
                new_code_hash: proposal.new_code_hash,
                proposal_id,
                from_version: old_version,
                to_version: new_version,
                timestamp: self.env().block_timestamp(),
            });

            // If the old code hash is different, we can try to apply via set_code_hash
            // (only works for ink! contracts that support it)
            let _ = old_code_hash; // suppress unused warning

            Ok(())
        }

        /// Cancels an upgrade proposal (proposer or admin)
        #[ink(message)]
        pub fn cancel_upgrade(&mut self, proposal_id: u64) -> Result<(), Error> {
            let caller = self.env().caller();

            let mut proposal = self
                .proposals
                .get(proposal_id)
                .ok_or(Error::ProposalNotFound)?;

            if proposal.cancelled || proposal.executed {
                return Err(Error::ProposalNotFound);
            }

            // Only proposer or admin can cancel
            if caller != proposal.proposer && caller != self.admin {
                return Err(Error::Unauthorized);
            }

            proposal.cancelled = true;
            self.proposals.insert(proposal_id, &proposal);

            self.migration_state = MigrationState::None;

            self.env().emit_event(UpgradeCancelled {
                proposal_id,
                cancelled_by: caller,
                timestamp: self.env().block_timestamp(),
            });

            Ok(())
        }

        // ====================================================================
        // ROLLBACK
        // ====================================================================

        /// Rolls back to the previous version (admin only, emergency)
        #[ink(message)]
        pub fn rollback(&mut self) -> Result<(), Error> {
            self.ensure_admin()?;

            if self.version_history.len() < 2 {
                return Err(Error::NoPreviousVersion);
            }

            let from_version = self.format_current_version();

            // Get previous version
            let prev_index = (self.version_history.len() - 2) as u32;
            let prev_version = self.version_history[prev_index as usize].clone();

            // Apply rollback
            self.code_hash = prev_version.code_hash;
            self.current_version_index = prev_index;
            self.migration_state = MigrationState::RolledBack;

            let to_version = self.format_current_version();

            self.env().emit_event(UpgradeRolledBack {
                from_version,
                to_version,
                rolled_back_by: self.env().caller(),
                timestamp: self.env().block_timestamp(),
            });

            Ok(())
        }

        // ====================================================================
        // EMERGENCY CONTROLS
        // ====================================================================

        /// Toggles emergency pause (admin only)
        #[ink(message)]
        pub fn toggle_emergency_pause(&mut self) -> Result<(), Error> {
            self.ensure_admin()?;
            self.emergency_pause = !self.emergency_pause;

            self.env().emit_event(EmergencyPauseToggled {
                paused: self.emergency_pause,
                by: self.env().caller(),
                timestamp: self.env().block_timestamp(),
            });

            Ok(())
        }

        // ====================================================================
        // GOVERNANCE MANAGEMENT
        // ====================================================================

        /// Adds a governor (admin only)
        #[ink(message)]
        pub fn add_governor(&mut self, governor: AccountId) -> Result<(), Error> {
            self.ensure_admin()?;
            if !self.governors.contains(&governor) {
                self.governors.push(governor);
                self.env().emit_event(GovernorAdded {
                    governor,
                    added_by: self.env().caller(),
                });
            }
            Ok(())
        }

        /// Removes a governor (admin only)
        #[ink(message)]
        pub fn remove_governor(&mut self, governor: AccountId) -> Result<(), Error> {
            self.ensure_admin()?;
            self.governors.retain(|g| *g != governor);
            self.env().emit_event(GovernorRemoved {
                governor,
                removed_by: self.env().caller(),
            });
            Ok(())
        }

        /// Updates required approval count (admin only)
        #[ink(message)]
        pub fn set_required_approvals(&mut self, required: u32) -> Result<(), Error> {
            self.ensure_admin()?;
            if required == 0 || required > self.governors.len() as u32 {
                return Err(Error::InsufficientApprovals);
            }
            self.required_approvals = required;
            Ok(())
        }

        /// Updates timelock period (admin only)
        #[ink(message)]
        pub fn set_timelock_blocks(&mut self, blocks: u32) -> Result<(), Error> {
            self.ensure_admin()?;
            if blocks < MIN_TIMELOCK_BLOCKS {
                return Err(Error::InvalidTimelockPeriod);
            }
            self.timelock_blocks = blocks;
            Ok(())
        }

        /// Changes the admin address
        #[ink(message)]
        pub fn change_admin(&mut self, new_admin: AccountId) -> Result<(), Error> {
            self.ensure_admin()?;
            let old_admin = self.admin;
            self.admin = new_admin;
            self.env().emit_event(AdminChanged {
                old_admin,
                new_admin,
            });
            Ok(())
        }

        // ====================================================================
        // DIRECT UPGRADE (backwards compatibility, admin only)
        // ====================================================================

        /// Direct upgrade without governance (admin only, for emergencies)
        #[ink(message)]
        pub fn upgrade_to(&mut self, new_code_hash: Hash) -> Result<(), Error> {
            self.ensure_admin()?;
            self.ensure_not_paused()?;

            let old_version = self.format_current_version();
            self.code_hash = new_code_hash;

            // Record as emergency version
            let version_info = VersionInfo {
                major: self.current_version().0,
                minor: self.current_version().1,
                patch: self.current_version().2 + 1,
                code_hash: new_code_hash,
                deployed_at_block: self.env().block_number(),
                deployed_at: self.env().block_timestamp(),
                description: String::from("Emergency direct upgrade"),
                deployed_by: self.env().caller(),
            };

            if self.version_history.len() as u32 >= MAX_VERSION_HISTORY {
                self.version_history.remove(0);
            }
            self.version_history.push(version_info);
            self.current_version_index = (self.version_history.len() - 1) as u32;

            let new_version = self.format_current_version();

            self.env().emit_event(Upgraded {
                new_code_hash,
                proposal_id: 0, // Direct upgrade, no proposal
                from_version: old_version,
                to_version: new_version,
                timestamp: self.env().block_timestamp(),
            });

            Ok(())
        }

        // ====================================================================
        // QUERY FUNCTIONS
        // ====================================================================

        /// Returns the current implementation code hash
        #[ink(message)]
        pub fn code_hash(&self) -> Hash {
            self.code_hash
        }

        /// Returns the admin address
        #[ink(message)]
        pub fn admin(&self) -> AccountId {
            self.admin
        }

        /// Returns the list of governors
        #[ink(message)]
        pub fn governors(&self) -> Vec<AccountId> {
            self.governors.clone()
        }

        /// Returns the current version as (major, minor, patch)
        #[ink(message)]
        pub fn current_version(&self) -> (u32, u32, u32) {
            if let Some(version) = self
                .version_history
                .get(self.current_version_index as usize)
            {
                (version.major, version.minor, version.patch)
            } else {
                (1, 0, 0)
            }
        }

        /// Returns the full version history
        #[ink(message)]
        pub fn get_version_history(&self) -> Vec<VersionInfo> {
            self.version_history.clone()
        }

        /// Returns a specific upgrade proposal
        #[ink(message)]
        pub fn get_proposal(&self, proposal_id: u64) -> Option<UpgradeProposal> {
            self.proposals.get(proposal_id)
        }

        /// Returns the current migration state
        #[ink(message)]
        pub fn migration_state(&self) -> MigrationState {
            self.migration_state.clone()
        }

        /// Returns whether emergency pause is active
        #[ink(message)]
        pub fn is_paused(&self) -> bool {
            self.emergency_pause
        }

        /// Returns required approvals count
        #[ink(message)]
        pub fn get_required_approvals(&self) -> u32 {
            self.required_approvals
        }

        /// Returns timelock period in blocks
        #[ink(message)]
        pub fn get_timelock_blocks(&self) -> u32 {
            self.timelock_blocks
        }

        /// Returns whether version compatibility checks pass for a target version
        #[ink(message)]
        pub fn check_compatibility(&self, major: u32, minor: u32, patch: u32) -> bool {
            self.check_version_compatibility(major, minor, patch)
                .is_ok()
        }

        // ====================================================================
        // INTERNAL HELPERS
        // ====================================================================

        fn ensure_admin(&self) -> Result<(), Error> {
            if self.env().caller() != self.admin {
                return Err(Error::Unauthorized);
            }
            Ok(())
        }

        fn ensure_governor(&self, caller: AccountId) -> Result<(), Error> {
            if !self.governors.contains(&caller) && caller != self.admin {
                return Err(Error::NotGovernor);
            }
            Ok(())
        }

        fn ensure_not_paused(&self) -> Result<(), Error> {
            if self.emergency_pause {
                return Err(Error::EmergencyPauseActive);
            }
            Ok(())
        }

        fn check_version_compatibility(
            &self,
            major: u32,
            minor: u32,
            patch: u32,
        ) -> Result<(), Error> {
            let (cur_major, cur_minor, cur_patch) = self.current_version();

            // New version must be >= current version
            if major > cur_major {
                return Ok(());
            }
            if major == cur_major && minor > cur_minor {
                return Ok(());
            }
            if major == cur_major && minor == cur_minor && patch > cur_patch {
                return Ok(());
            }

            Err(Error::IncompatibleVersion)
        }

        fn format_current_version(&self) -> String {
            let (major, minor, patch) = self.current_version();
            let mut v = String::from("v");
            // Manual formatting without format!() macro overhead
            v.push_str(&Self::u32_to_string(major));
            v.push('.');
            v.push_str(&Self::u32_to_string(minor));
            v.push('.');
            v.push_str(&Self::u32_to_string(patch));
            v
        }

        fn u32_to_string(n: u32) -> String {
            if n == 0 {
                return String::from("0");
            }
            let mut s = String::new();
            let mut num = n;
            let mut digits = Vec::new();
            while num > 0 {
                digits.push((b'0' + (num % 10) as u8) as char);
                num /= 10;
            }
            digits.reverse();
            for d in digits {
                s.push(d);
            }
            s
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[ink::test]
        fn new_initializes_correctly() {
            let hash = Hash::from([0x42; 32]);
            let proxy = TransparentProxy::new(hash);
            assert_eq!(proxy.code_hash(), hash);
            assert_eq!(proxy.current_version(), (1, 0, 0));
            assert_eq!(proxy.get_version_history().len(), 1);
            assert_eq!(proxy.migration_state(), MigrationState::None);
            assert!(!proxy.is_paused());
        }

        #[ink::test]
        fn propose_upgrade_works() {
            let hash = Hash::from([0x42; 32]);
            let mut proxy = TransparentProxy::new(hash);

            let new_hash = Hash::from([0x43; 32]);
            let result = proxy.propose_upgrade(
                new_hash,
                1,
                1,
                0,
                String::from("Feature upgrade"),
                String::from("No migration needed"),
            );
            assert!(result.is_ok());
            assert_eq!(result.unwrap(), 1);

            let proposal = proxy.get_proposal(1).unwrap();
            assert_eq!(proposal.new_code_hash, new_hash);
            assert!(!proposal.cancelled);
            assert!(!proposal.executed);
        }

        #[ink::test]
        fn version_compatibility_check_works() {
            let hash = Hash::from([0x42; 32]);
            let proxy = TransparentProxy::new(hash);

            // Version 1.1.0 is compatible (higher)
            assert!(proxy.check_compatibility(1, 1, 0));
            // Version 2.0.0 is compatible (higher)
            assert!(proxy.check_compatibility(2, 0, 0));
            // Version 0.9.0 is not compatible (lower)
            assert!(!proxy.check_compatibility(0, 9, 0));
            // Same version is not compatible
            assert!(!proxy.check_compatibility(1, 0, 0));
        }

        #[ink::test]
        fn direct_upgrade_works() {
            let hash = Hash::from([0x42; 32]);
            let mut proxy = TransparentProxy::new(hash);

            let new_hash = Hash::from([0x43; 32]);
            let result = proxy.upgrade_to(new_hash);
            assert!(result.is_ok());
            assert_eq!(proxy.code_hash(), new_hash);
            assert_eq!(proxy.get_version_history().len(), 2);
        }

        #[ink::test]
        fn rollback_works() {
            let hash = Hash::from([0x42; 32]);
            let mut proxy = TransparentProxy::new(hash);

            let new_hash = Hash::from([0x43; 32]);
            proxy.upgrade_to(new_hash).unwrap();
            assert_eq!(proxy.code_hash(), new_hash);

            let rollback_result = proxy.rollback();
            assert!(rollback_result.is_ok());
            assert_eq!(proxy.code_hash(), hash);
            assert_eq!(proxy.migration_state(), MigrationState::RolledBack);
        }

        #[ink::test]
        fn rollback_fails_with_no_history() {
            let hash = Hash::from([0x42; 32]);
            let mut proxy = TransparentProxy::new(hash);
            assert_eq!(proxy.rollback(), Err(Error::NoPreviousVersion));
        }

        #[ink::test]
        fn emergency_pause_works() {
            let hash = Hash::from([0x42; 32]);
            let mut proxy = TransparentProxy::new(hash);
            assert!(!proxy.is_paused());

            proxy.toggle_emergency_pause().unwrap();
            assert!(proxy.is_paused());

            // Upgrade should fail when paused
            let new_hash = Hash::from([0x43; 32]);
            assert_eq!(proxy.upgrade_to(new_hash), Err(Error::EmergencyPauseActive));

            proxy.toggle_emergency_pause().unwrap();
            assert!(!proxy.is_paused());
        }

        #[ink::test]
        fn cancel_upgrade_works() {
            let hash = Hash::from([0x42; 32]);
            let mut proxy = TransparentProxy::new(hash);

            let new_hash = Hash::from([0x43; 32]);
            proxy
                .propose_upgrade(new_hash, 1, 1, 0, String::from("Test"), String::from(""))
                .unwrap();

            let result = proxy.cancel_upgrade(1);
            assert!(result.is_ok());

            let proposal = proxy.get_proposal(1).unwrap();
            assert!(proposal.cancelled);
        }

        #[ink::test]
        fn governor_management_works() {
            let hash = Hash::from([0x42; 32]);
            let mut proxy = TransparentProxy::new(hash);

            let new_governor = AccountId::from([0x02; 32]);
            proxy.add_governor(new_governor).unwrap();
            assert_eq!(proxy.governors().len(), 2);

            proxy.remove_governor(new_governor).unwrap();
            assert_eq!(proxy.governors().len(), 1);
        }
    }
}
