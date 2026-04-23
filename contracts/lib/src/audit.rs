use ink::prelude::vec::Vec;
use ink::primitives::AccountId;
use ink::storage::Mapping;
use propchain_traits::{SecurityEventType, SecuritySeverity};

/// A single tamper-evident audit record in the hash chain.
/// Compact design (~98 bytes) avoids String fields for gas efficiency.
#[derive(Debug, Clone, PartialEq, Eq, scale::Encode, scale::Decode)]
#[cfg_attr(
    feature = "std",
    derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
)]
pub struct AuditRecord {
    /// Sequential record ID (1-indexed)
    pub id: u64,
    /// Account that triggered the operation
    pub actor: AccountId,
    /// Type of security event
    pub event_type: SecurityEventType,
    /// Severity classification
    pub severity: SecuritySeverity,
    /// Resource identifier (property_id, escrow_id, etc.)
    pub resource_id: u64,
    /// Compact additional context (error code, role as u8, etc.)
    pub extra_data: u32,
    /// Block number when recorded
    pub block_number: u32,
    /// Block timestamp when recorded
    pub timestamp: u64,
    /// Blake2x256 hash chained to previous record for tamper evidence
    pub record_hash: [u8; 32],
}

/// Tamper-evident audit trail with hash chain integrity verification.
///
/// Each record's hash incorporates the previous record's hash, forming a chain
/// where modifying any record invalidates all subsequent hashes. This follows
/// the same storage pattern as `AccessControl` in `access_control.rs`.
#[ink::storage_item]
#[derive(Default)]
pub struct AuditTrail {
    /// Sequential audit records indexed by ID
    records: Mapping<u64, AuditRecord>,
    /// Total number of audit records
    record_count: u64,
    /// Hash of the most recent record (chain head)
    latest_hash: [u8; 32],
    /// Secondary index: (actor, actor_record_index) -> global record ID
    actor_index: Mapping<(AccountId, u64), u64>,
    /// Count of records per actor
    actor_record_count: Mapping<AccountId, u64>,
    /// Secondary index: (event_type as u8, type_record_index) -> global record ID
    type_index: Mapping<(u8, u64), u64>,
    /// Count of records per event type
    type_record_count: Mapping<u8, u64>,
}

impl core::fmt::Debug for AuditTrail {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("AuditTrail")
            .field("record_count", &self.record_count)
            .field("latest_hash", &self.latest_hash)
            .finish()
    }
}

impl AuditTrail {
    /// Create a new AuditTrail with genesis hash (all zeros).
    pub fn new() -> Self {
        Self {
            records: Mapping::default(),
            record_count: 0,
            latest_hash: [0u8; 32],
            actor_index: Mapping::default(),
            actor_record_count: Mapping::default(),
            type_index: Mapping::default(),
            type_record_count: Mapping::default(),
        }
    }

    /// Log a security event. Returns the new record ID.
    ///
    /// Computes a Blake2x256 hash that chains to the previous record,
    /// stores the record, and updates secondary indices.
    pub fn log_event(
        &mut self,
        actor: AccountId,
        event_type: SecurityEventType,
        severity: SecuritySeverity,
        resource_id: u64,
        extra_data: u32,
        block_number: u32,
        timestamp: u64,
    ) -> u64 {
        let id = self.record_count.saturating_add(1);

        let record_hash = self.compute_record_hash(
            id,
            &actor,
            event_type,
            severity,
            resource_id,
            extra_data,
            block_number,
            timestamp,
        );

        let record = AuditRecord {
            id,
            actor,
            event_type,
            severity,
            resource_id,
            extra_data,
            block_number,
            timestamp,
            record_hash,
        };

        self.records.insert(id, &record);
        self.record_count = id;
        self.latest_hash = record_hash;

        // Update actor index
        let actor_count = self.actor_record_count.get(actor).unwrap_or(0);
        self.actor_index.insert((actor, actor_count), &id);
        self.actor_record_count
            .insert(actor, &actor_count.saturating_add(1));

        // Update type index
        let type_key = event_type as u8;
        let type_count = self.type_record_count.get(type_key).unwrap_or(0);
        self.type_index.insert((type_key, type_count), &id);
        self.type_record_count
            .insert(type_key, &type_count.saturating_add(1));

        id
    }

    /// Get a specific audit record by ID.
    pub fn get_record(&self, id: u64) -> Option<AuditRecord> {
        self.records.get(id)
    }

    /// Get the total number of audit records.
    pub fn record_count(&self) -> u64 {
        self.record_count
    }

    /// Get the latest hash chain head for off-chain verification.
    pub fn latest_hash(&self) -> [u8; 32] {
        self.latest_hash
    }

    /// Get record IDs for a specific actor (paginated).
    pub fn get_actor_records(&self, actor: AccountId, offset: u64, limit: u64) -> Vec<u64> {
        let count = self.actor_record_count.get(actor).unwrap_or(0);
        let mut result = Vec::new();
        let end = count.min(offset.saturating_add(limit));
        for i in offset..end {
            if let Some(id) = self.actor_index.get((actor, i)) {
                result.push(id);
            }
        }
        result
    }

    /// Get record IDs for a specific event type (paginated).
    pub fn get_type_records(
        &self,
        event_type: SecurityEventType,
        offset: u64,
        limit: u64,
    ) -> Vec<u64> {
        let type_key = event_type as u8;
        let count = self.type_record_count.get(type_key).unwrap_or(0);
        let mut result = Vec::new();
        let end = count.min(offset.saturating_add(limit));
        for i in offset..end {
            if let Some(id) = self.type_index.get((type_key, i)) {
                result.push(id);
            }
        }
        result
    }

    /// Verify integrity of the hash chain between two record IDs (inclusive).
    ///
    /// Recomputes each record's hash and checks it matches the stored hash.
    /// Returns `true` if the chain is intact, `false` if tampered.
    ///
    /// Gas cost is O(to_id - from_id). Use ranges of <= 100 for on-chain calls.
    pub fn verify_integrity(&self, from_id: u64, to_id: u64) -> bool {
        if from_id == 0 || to_id < from_id || to_id > self.record_count {
            return false;
        }

        // Get the hash that should precede from_id
        let mut expected_prev_hash = if from_id == 1 {
            [0u8; 32] // Genesis hash
        } else {
            match self.records.get(from_id - 1) {
                Some(prev) => prev.record_hash,
                None => return false,
            }
        };

        for id in from_id..=to_id {
            let record = match self.records.get(id) {
                Some(r) => r,
                None => return false,
            };

            // Recompute the hash using the previous record's hash
            let data = (
                expected_prev_hash,
                record.id,
                record.actor,
                record.event_type,
                record.severity,
                record.resource_id,
                record.extra_data,
                record.block_number,
                record.timestamp,
            );
            let encoded = scale::Encode::encode(&data);
            let mut computed_hash = [0u8; 32];
            ink::env::hash_bytes::<ink::env::hash::Blake2x256>(&encoded, &mut computed_hash);

            if computed_hash != record.record_hash {
                return false;
            }

            expected_prev_hash = record.record_hash;
        }

        true
    }

    /// Compute Blake2x256 hash for a new record, chaining with the previous hash.
    fn compute_record_hash(
        &self,
        id: u64,
        actor: &AccountId,
        event_type: SecurityEventType,
        severity: SecuritySeverity,
        resource_id: u64,
        extra_data: u32,
        block_number: u32,
        timestamp: u64,
    ) -> [u8; 32] {
        let data = (
            self.latest_hash,
            id,
            actor,
            event_type,
            severity,
            resource_id,
            extra_data,
            block_number,
            timestamp,
        );
        let encoded = scale::Encode::encode(&data);
        let mut output = [0u8; 32];
        ink::env::hash_bytes::<ink::env::hash::Blake2x256>(&encoded, &mut output);
        output
    }
}
