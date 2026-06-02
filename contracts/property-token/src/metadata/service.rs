use soroban_sdk::{Env, Vec, Address, String, Symbol};
use crate::types::MetadataVersion;
use crate::storage::metadata::{self, key, MAX_HISTORY};
use crate::events::metadata_events::emit_metadata_updated;

pub fn update_metadata(
    env: Env,
    property_id: u64,
    updated_by: Address,
    metadata_hash: String,
) -> u32 {
    let storage_key = key(property_id);

    let mut history: Vec<MetadataVersion> =
        env.storage().instance().get(&storage_key).unwrap_or(Vec::new(&env));

    let version_number = (history.len() as u32) + 1;

    let new_version = MetadataVersion {
        version_number,
        updated_by: updated_by.clone(),
        updated_at: env.ledger().timestamp(),
        metadata_hash,
    };

    history.push_back(new_version.clone());

    // enforce cap
    if history.len() > MAX_HISTORY {
        let mut trimmed = Vec::new(&env);
        let start = history.len() - MAX_HISTORY;

        for i in start..history.len() {
            trimmed.push_back(history.get(i).unwrap());
        }

        history = trimmed;
    }

    env.storage().instance().set(&storage_key, &history);

    emit_metadata_updated(&env, property_id, version_number);

    version_number
}