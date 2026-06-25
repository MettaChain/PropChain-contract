
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
    use ink::env::call::{build_call, ExecutionInput, Selector};

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

    #[ink(event)]
    pub struct DiamondCut {
        #[ink(topic)]
        from_version: String,
        #[ink(topic)]
        to_version: String,
        rolled_back_by: AccountId,
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
        /// Facet addresses
        facet_addresses: Vec<AccountId>,
        /// Selector to facet mapping
        selector_to_facet: ink::storage::Mapping<[u8; 4], AccountId>,
        /// Facet to selectors mapping
        facet_selectors: ink::storage::Mapping<AccountId, Vec<[u8; 4]>>,
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
                facet_addresses: Vec::new(),
                selector_to_facet: ink::storage::Mapping::default(),
                facet_selectors: ink::storage::Mapping::default(),
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
                facet_addresses: Vec::new(),
                selector_to_facet: ink::storage::Mapping::default(),
                facet_selectors: ink::storage::Mapping::default(),
            }
        }

        // ====================================================================
        // DIAMOND STANDARD (EIP-2535)
        // ====================================================================

        /// Add, replace, or remove facets and functions from the diamond
        #[ink(message)]
        pub fn diamond_cut(&mut self, cuts: Vec<FacetCut>) -> Result<(), Error> {
            self.ensure_admin()?;
            self.ensure_not_paused()?;

            for cut in cuts {
                match cut.action {
                    FacetCutAction::Add => self.add_facet(cut.facet_address, cut.selectors)?,
                    FacetCutAction::Replace => self.replace_facet(cut.facet_address, cut.selectors)?,
                    FacetCutAction::Remove => self.remove_facet(cut.facet_address, cut.selectors)?,
                }
            }

            Ok(())
        }

        /// The proxy's fallback function
        ///
        /// Delegates calls to the appropriate facet if the selector is registered.
        /// Otherwise, it forwards the call to the main implementation contract.
        #[ink(message, payable, selector = "_")]
        pub fn _fallback(&mut self) {
            let selector = self.env().transferred_value();
            let facet = self.selector_to_facet.get(&selector.to_be_bytes());

            match facet {
                Some(facet_address) => {
                    let _ = build_call::<ink::env::DefaultEnvironment>()
                        .call(facet_address)
                        .transferred_value(self.env().transferred_value())
                        .exec_input(ExecutionInput::new(Selector::new(selector.to_be_bytes())))
                        .returns::<()>()
                        .try_invoke();
                }
                None => {
                    let _ = build_call::<ink::env::DefaultEnvironment>()
                        .call(self.code_hash)
                        .transferred_value(self.env().transferred_value())
                        .exec_input(ExecutionInput::new(Selector::new(selector.to_be_bytes())))
                        .returns::<()>()
                        .try_invoke();
                }
            }
        }

        /// Helper to add a new facet
        fn add_facet(&mut self, facet_address: AccountId, selectors: Vec<[u8; 4]>) -> Result<(), Error> {
            if facet_address == AccountId::from([0; 32]) {
                return Err(Error::InvalidFacetAddress);
            }

            if self.facet_addresses.contains(&facet_address) {
                return Err(Error::FacetAlreadyExists);
            }

            for selector in &selectors {
                if self.selector_to_facet.get(selector).is_some() {
                    return Err(Error::SelectorAlreadyExists);
                }
            }

            self.facet_addresses.push(facet_address);
            self.facet_selectors.insert(facet_address, &selectors);

            for selector in selectors {
                self.selector_to_facet.insert(selector, &facet_address);
            }

            Ok(())
        }

        /// Helper to replace selectors of an existing facet
        fn replace_facet(&mut self, facet_address: AccountId, selectors: Vec<[u8; 4]>) -> Result<(), Error> {
            if !self.facet_addresses.contains(&facet_address) {
                return Err(Error::FacetNotFound);
            }

            // Remove old selectors
            if let Some(old_selectors) = self.facet_selectors.get(&facet_address) {
                for selector in old_selectors {
                    self.selector_to_facet.remove(&selector);
                }
            }

            // Add new selectors
            for selector in &selectors {
                if let Some(owner) = self.selector_to_facet.get(selector) {
                    if owner != facet_address {
                        return Err(Error::SelectorAlreadyExists);
                    }
                }
            }

            self.facet_selectors.insert(facet_address, &selectors);
            for selector in selectors {
                self.selector_to_facet.insert(selector, &facet_address);
            }

            Ok(())
        }

        /// Helper to remove a facet or specific functions from it
        fn remove_facet(&mut self, facet_address: AccountId, selectors_to_remove: Vec<[u8; 4]>) -> Result<(), Error> {
            if !self.facet_addresses.contains(&facet_address) {
                return Err(Error::FacetNotFound);
            }

            let mut current_selectors = self.facet_selectors.get(&facet_address).unwrap_or_default();

            for selector in &selectors_to_remove {
                if !current_selectors.contains(selector) {
                    return Err(Error::SelectorNotFound);
                }
                self.selector_to_facet.remove(selector);
            }

            current_selectors.retain(|s| !selectors_to_remove.contains(s));

            if current_selectors.is_empty() {
                // Remove the facet completely if no selectors are left
                self.facet_addresses.retain(|&f| f != facet_address);
                self.facet_selectors.remove(&facet_address);
            } else {
                self.facet_selectors.insert(facet_address, &current_selectors);
            }

            Ok(())
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
            self.code_hash = new_code_hash;
            Ok(())
        }

        // ====================================================================
        // VIEW FUNCTIONS
        // ====================================================================

        /// Returns the current admin address
        #[ink(message)]
        pub fn get_admin(&self) -> AccountId {
            self.admin
        }

        /// Returns the list of governors
        #[ink(message)]
        pub fn get_governors(&self) -> Vec<AccountId> {
            self.governors.clone()
        }

        /// Returns the required number of approvals
        #[ink(message)]
        pub fn get_required_approvals(&self) -> u32 {
            self.required_approvals
        }

        /// Returns the timelock period in blocks
        #[ink(message)]
        pub fn get_timelock_blocks(&self) -> u32 {
            self.timelock_blocks
        }

        /// Returns the current version info
        #[ink(message)]
        pub fn get_current_version(&self) -> VersionInfo {
            self.version_history[self.current_version_index as usize].clone()
        }

        /// Returns the full version history
        #[ink(message)]
        pub fn get_version_history(&self) -> Vec<VersionInfo> {
            self.version_history.clone()
        }

        /// Returns the details of a specific proposal
        #[ink(message)]
        pub fn get_proposal(&self, proposal_id: u64) -> Option<UpgradeProposal> {
            self.proposals.get(proposal_id)
        }

        /// Returns the current migration state
        #[ink(message)]
        pub fn get_migration_state(&self) -> MigrationState {
            self.migration_state
        }

        /// Returns the emergency pause status
        #[ink(message)]
        pub fn is_paused(&self) -> bool {
            self.emergency_pause
        }

        // ====================================================================
        // INTERNAL HELPERS
        // ====================================================================

        /// Ensures the caller is the admin
        fn ensure_admin(&self) -> Result<(), Error> {
            if self.env().caller() != self.admin {
                Err(Error::Unauthorized)
            } else {
                Ok(())
            }
        }

        /// Ensures the caller is a governor
        fn ensure_governor(&self, account: AccountId) -> Result<(), Error> {
            if !self.governors.contains(&account) {
                Err(Error::Unauthorized)
            } else {
                Ok(())
            }
        }

        /// Ensures the contract is not paused
        fn ensure_not_paused(&self) -> Result<(), Error> {
            if self.emergency_pause {
                Err(Error::Paused)
            } else {
                Ok(())
            }
        }

        /// Formats the current version as a string "vX.Y.Z"
        fn format_current_version(&self) -> String {
            let version = self.get_current_version();
            let mut s = String::new();
            s.push_str("v");
            s.push_str(&version.major.to_string());
            s.push_str(".");
            s.push_str(&version.minor.to_string());
            s.push_str(".");
            s.push_str(&version.patch.to_string());
            s
        }

        /// Checks if the new version is compatible (>= current)
        fn check_version_compatibility(&self, major: u32, minor: u32, patch: u32) -> Result<(), Error> {
            let current = self.get_current_version();
            if major < current.major {
                return Err(Error::VersionIncompatible);
            }
            if major == current.major && minor < current.minor {
                return Err(Error::VersionIncompatible);
            }
            if major == current.major && minor == current.minor && patch < current.patch {
                return Err(Error::VersionIncompatible);
            }
            Ok(())
        }

        /// Adds a new facet and its functions to the diamond
        fn add_facet(&mut self, facet_address: AccountId, selectors: Vec<[u8; 4]>) -> Result<(), Error> {
            if facet_address == AccountId::from([0x0; 32]) {
                return Err(Error::InvalidFacetAddress);
            }
            if self.facet_addresses.contains(&facet_address) {
                return Err(Error::FacetAlreadyExists);
            }

            for selector in &selectors {
                if self.selector_to_facet.get(selector).is_some() {
                    return Err(Error::SelectorAlreadyExists);
                }
            }

            for selector in &selectors {
                self.selector_to_facet.insert(selector, &facet_address);
            }

            self.facet_addresses.push(facet_address);
            self.facet_selectors.insert(facet_address, &selectors);

            Ok(())
        }

        /// Replaces an existing facet with a new one
        fn replace_facet(&mut self, facet_address: AccountId, selectors: Vec<[u8; 4]>) -> Result<(), Error> {
            if facet_address == AccountId::from([0x0; 32]) {
                return Err(Error::InvalidFacetAddress);
            }
            if !self.facet_addresses.contains(&facet_address) {
                return Err(Error::FacetNotFound);
            }

            for selector in &selectors {
                if let Some(owner) = self.selector_to_facet.get(selector) {
                    if owner != facet_address {
                        return Err(Error::SelectorAlreadyExists);
                    }
                }
            }

            let old_selectors = self.facet_selectors.get(&facet_address).unwrap_or_default();
            for selector in &old_selectors {
                self.selector_to_facet.remove(selector);
            }

            for selector in &selectors {
                self.selector_to_facet.insert(selector, &facet_address);
            }

            self.facet_selectors.insert(facet_address, &selectors);

            Ok(())
        }

        /// Removes a facet and its functions from the diamond
        fn remove_facet(&mut self, facet_address: AccountId, selectors: Vec<[u8; 4]>) -> Result<(), Error> {
            if facet_address == AccountId::from([0x0; 32]) {
                return Err(Error::InvalidFacetAddress);
            }
            if !self.facet_addresses.contains(&facet_address) {
                return Err(Error::FacetNotFound);
            }

            let registered_selectors = self.facet_selectors.get(&facet_address).unwrap_or_default();
            for selector in &selectors {
                if !registered_selectors.contains(selector) {
                    return Err(Error::SelectorNotFound);
                }
            }

            for selector in &selectors {
                self.selector_to_facet.remove(selector);
            }

            self.facet_addresses.retain(|&addr| addr != facet_address);
            self.facet_selectors.remove(&facet_address);

            Ok(())
        }
    }
}